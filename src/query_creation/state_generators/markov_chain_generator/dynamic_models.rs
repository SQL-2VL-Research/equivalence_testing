use smol_str::SmolStr;

use super::markov_chain::NodeParams;


/// Dynamic model for assigning probabilities when using ProbabilisticModel 
pub trait DynamicModel {
    fn new() -> Self;
    /// assigns the (unnormalized) probabilities to the outgoing nodes.
    /// Receives probabilities recorded in graph
    fn assign_probabilities(&mut self, node_outgoing: Vec<(bool, f64, NodeParams)>) -> Vec::<(bool, f64, NodeParams)>;
    /// is called at the beginning of each subquery creation
    fn notify_subquery_creation_begin(&mut self) {}
    /// is called at the end of each subquery creation
    fn notify_subquery_creation_end(&mut self) {}
    /// is called when a new state is reached
    fn update_current_state(&mut self, _node_name: &SmolStr) {}
    /// is called to set a new stack length
    fn notify_call_stack_length(&mut self, _stack_len: usize) {}
}

pub struct MarkovModel { }

impl DynamicModel for MarkovModel {
    fn new() -> Self {
        Self {}
    }
    fn assign_probabilities(&mut self, node_outgoing: Vec<(bool, f64, NodeParams)>) -> Vec::<(bool, f64, NodeParams)> {
        node_outgoing
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
            current_stack_length: 0
        }
    }
}

pub struct AntiCallModel {
    /// used to store running query statistics, such as
    /// the current level of nesting
    pub stats: QueryStats,
}

impl DynamicModel for AntiCallModel {
    fn new() -> Self {
        Self { stats: QueryStats::new() }
    }

    fn assign_probabilities(&mut self, node_outgoing: Vec<(bool, f64, NodeParams)>) -> Vec::<(bool, f64, NodeParams)> {
        let prob_multiplier = if self.stats.current_stack_length > 3 {
            self.stats.current_state_num as f64
        } else { 1f64 };
        node_outgoing.into_iter().map(|el| {(
            el.0,
            if ![  // TODO: the following is for DEBUG purposes
                "SELECT_wildcard", "SELECT_qualified_wildcard", "types_null",
                "call0_list_expr", "call0_array",  // "call1_Query", "call0_numeric", "call1_VAL_3", "call0_string",
            ].contains(&el.2.name.as_str()) || self.stats.current_stack_length > 3 {
                el.1 / f64::powf(prob_multiplier, el.2.min_calls_until_function_exit as f64)
            } else { 0f64 },
            el.2
        )}).collect()
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