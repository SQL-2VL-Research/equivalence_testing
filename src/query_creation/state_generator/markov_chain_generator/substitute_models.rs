use smol_str::SmolStr;

use crate::training::ast_to_path::PathNode;

use super::{markov_chain::NodeParams, StateGenerationError};


/// Dynamic model for assigning probabilities when using ProbabilisticModel 
pub trait SubstituteModel {
    fn new() -> Self;
    /// assigns the (unnormalized) log-probabilities to the outgoing nodes.
    /// Receives log-probability distruibution recorded in graph
    fn trasform_log_probabilities(&mut self, node_outgoing: Vec<(f64, NodeParams)>) -> Result<Vec::<(f64, NodeParams)>, StateGenerationError>;
    /// is called at the beginning of each subquery creation
    fn notify_subquery_creation_begin(&mut self) {}
    /// is called at the end of each subquery creation
    fn notify_subquery_creation_end(&mut self) {}
    /// is called when a new state is reached
    fn update_current_state(&mut self, _node_name: &SmolStr) {}
    /// is called to set a new stack length
    fn notify_call_stack_length(&mut self, _stack_len: usize) {}
}

/// preserves the probabilities
pub struct MarkovModel { }

impl SubstituteModel for MarkovModel {
    fn new() -> Self {
        Self {}
    }
    fn trasform_log_probabilities(&mut self, node_outgoing: Vec<(f64, NodeParams)>) -> Result<Vec::<(f64, NodeParams)>, StateGenerationError> {
        Ok(node_outgoing)
    }
}

/// uses a predetermined path to generate probabilities
#[derive(Debug, Clone)]
pub struct PathModel {
    path: Vec<SmolStr>,
    index: usize,
}

impl PathModel {
    pub fn from_path_nodes(path: &Vec<PathNode>) -> Self {
        Self {
            path: path.iter().cloned().filter_map(
                |x| if let PathNode::State(state) = x { Some(state) } else { None }
            ).collect(),
            index: 0,
        }
    }
}

impl SubstituteModel for PathModel {
    fn new() -> Self {
        Self {
            path: vec![],
            index: 0,
        }
    }

    fn trasform_log_probabilities(&mut self, node_outgoing: Vec<(f64, NodeParams)>) -> Result<Vec::<(f64, NodeParams)>, StateGenerationError> {
        let node_name = &self.path[self.index];
        self.index += 1;
        if node_outgoing.iter().find(|(.., node)| node.node_common.name == *node_name).is_none() {
            Err(StateGenerationError::new(format!(
                "Did not find {node_name} among {:?}", node_outgoing.iter().map(
                    |(.., node)| node.node_common.name.to_string()
                ).collect::<Vec<_>>()
            )))
        } else {
            Ok(node_outgoing.into_iter().map(
                |(_, node)| (
                    if node.node_common.name == *node_name {
                        1f64.ln()
                    } else {
                        f64::NEG_INFINITY  // 0f64.ln()
                    },
                    node,
                )
            ).collect())
        }
    }
}

/// uses the current state to generate the next probability set
#[derive(Debug, Clone)]
pub struct DeterministicModel {
    state_to_choose: Option<SmolStr>,
}

impl DeterministicModel {
    pub fn set_state(&mut self, state: SmolStr) {
        self.state_to_choose = Some(state);
    }
}

impl SubstituteModel for DeterministicModel {
    fn new() -> Self where Self: Sized {
        Self {
            state_to_choose: None,
        }
    }

    fn trasform_log_probabilities(&mut self, node_outgoing: Vec<(f64, NodeParams)>) -> Result<Vec::<(f64, NodeParams)>, StateGenerationError> {
        let node_name = self.state_to_choose.take().unwrap();
        if node_outgoing.iter().find(|(.., node)| node.node_common.name == node_name).is_none() {
            Err(StateGenerationError::new(format!(
                "Did not find {node_name} among {:?}", node_outgoing.iter().map(
                    |(.., node)| node.node_common.name.to_string()
                ).collect::<Vec<_>>()
            )))
        } else {
            Ok(node_outgoing.into_iter().map(
                |(_, node)| (
                    if node.node_common.name == node_name {
                        1f64.ln()
                    } else {
                        f64::NEG_INFINITY  // 0f64.ln()
                    },
                    node,
                )
            ).collect())
        }
    }
}

pub struct QueryStats {
    /// Remember to increase this value before
    /// and decrease after generating a subquery, to
    /// control the maximum level of nesting
    /// allowed
    #[allow(dead_code)]
    current_nest_level: u32,
    /// current state number in the global path
    current_state_num: u32,
    /// used to store current functional graph stack length
    current_stack_length: usize,
}

impl QueryStats {
    fn new() -> Self {
        Self {
            current_nest_level: 0,
            current_state_num: 0,
            current_stack_length: 0,
        }
    }
}

/// this model alters the probabilies in a way that tries to omit the call nodes
/// once certain stack length is reached, making the probabilities lower for
/// nodes that are further from exit in terms of call nodes
pub struct AntiCallModel {
    /// used to store running query statistics, such as
    /// the current level of nesting
    pub stats: QueryStats,
}

impl SubstituteModel for AntiCallModel {
    fn new() -> Self {
        Self { stats: QueryStats::new() }
    }

    fn trasform_log_probabilities(&mut self, node_outgoing: Vec<(f64, NodeParams)>) -> Result<Vec::<(f64, NodeParams)>, StateGenerationError> {
        let prob_multiplier = if self.stats.current_stack_length > 10 { 100f64 } else { 0f64 };
        Ok(node_outgoing.into_iter().map(|(l_p, node)| (
            l_p - (node.min_calls_until_function_exit as f64) * prob_multiplier,
            node
        )).collect::<Vec<_>>())
    }

    fn notify_subquery_creation_begin(&mut self) {
        self.stats.current_nest_level += 1;
    }

    fn notify_subquery_creation_end(&mut self) {
        self.stats.current_nest_level -= 1;
    }

    fn update_current_state(&mut self, _node_name: &SmolStr) {
        self.stats.current_state_num += 1;
    }

    fn notify_call_stack_length(&mut self, stack_len: usize) {
        self.stats.current_stack_length = stack_len;
    }
}