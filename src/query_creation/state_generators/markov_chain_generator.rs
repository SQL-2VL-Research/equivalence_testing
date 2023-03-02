pub mod state_choosers;
pub mod dynamic_models;
mod markov_chain;
mod dot_parser;
mod error;

use std::path::Path;

use core::fmt::Debug;
use smol_str::SmolStr;

use self::{
    markov_chain::{
        MarkovChain, NodeParams, CallParams, CallModifiers
    }, error::SyntaxError,
};

use state_choosers::StateChooser;
use dynamic_models::{MarkovModel, DynamicModel};

pub use dot_parser::FunctionInputsType;

/// The markov chain generator. Runs the functional
/// subgraphs parsed from the .dot file. Manages the
/// probabilities, outputs states, disables nodes if
/// needed.
#[derive(Debug, Clone)]
pub struct MarkovChainGenerator<StC: StateChooser> {
    /// this stack contains information about the known type names
    /// inferred when [known] is used in [TYPES]
    known_type_name_stack: Vec<Vec<SmolStr>>,
    /// this stack contains information about the compatible type names
    /// inferred when [compatible] is used in [TYPES]
    compatible_type_name_stack: Vec<Vec<SmolStr>>,
    markov_chain: MarkovChain,
    call_stack: Vec<StackItem>,
    pending_call: Option<CallParams>,
    state_chooser: Box<StC>,
}

#[derive(Clone, Debug)]
struct StackItem {
    current_function: CallParams,
    current_node_name: SmolStr,
    next_node: NodeParams,
}

impl StackItem {
    fn from_call_params(call_params: CallParams) -> Self {
        let func_name = call_params.func_name.clone();
        Self {
            current_function: call_params,
            current_node_name: SmolStr::new("-"),
            next_node: NodeParams {
                name: func_name,
                call_params: None, option_name: None,
                literal: false, min_calls_until_function_exit: 0,
                trigger: None
            }
        }
    }
}

impl<StC: StateChooser> MarkovChainGenerator<StC> {
    pub fn parse_graph_from_file<P: AsRef<Path>>(source_path: P) -> Result<Self, SyntaxError> {
        let chain = MarkovChain::parse_dot(source_path)?;
        let mut self_ = MarkovChainGenerator::<StC> {
            markov_chain: chain,
            call_stack: vec![],
            pending_call: None,
            known_type_name_stack: vec![],
            compatible_type_name_stack: vec![],
            state_chooser: Box::new(StC::new())
        };
        self_.reset();
        Ok(self_)
    }

    /// used to print the call stack of the markov chain functions
    pub fn print_stack(&self) {
        println!("Call stack:");
        for stack_item in &self.call_stack {
            println!("{} ({:?}) [{}]:", stack_item.current_function.func_name, stack_item.current_function.inputs, stack_item.current_node_name)
        }
    }

    /// used when the markov chain reaches end, for the object to be iterable multiple times
    pub fn reset(&mut self) {
        self.call_stack = vec![StackItem::from_call_params(CallParams {
            func_name: SmolStr::new("Query"),
            inputs: FunctionInputsType::None,
            modifiers: CallModifiers::None
        })];
        self.known_type_name_stack = vec![vec![SmolStr::new("")]];
        self.compatible_type_name_stack = vec![vec![SmolStr::new("")]];
    }

    /// get crrent function inputs list
    pub fn get_inputs(&self) -> FunctionInputsType {
        self.call_stack.last().unwrap().current_function.inputs.clone()
    }

    /// get crrent function modifiers list
    pub fn get_modifiers(&self) -> CallModifiers {
        self.call_stack.last().unwrap().current_function.modifiers.clone()
    }

    /// push the known type list for the next node that will use the type=[known]
    pub fn push_known(&mut self, type_list: Vec<SmolStr>) {
        self.known_type_name_stack.push(type_list);
    }

    /// push the compatible type list for the next node that will use the type=[compatible]
    pub fn push_compatible(&mut self, type_list: Vec<SmolStr>) {
        self.compatible_type_name_stack.push(type_list);
    }

    /// pop the known type for the current node which will uses type=[known]
    fn pop_known(&mut self) -> Vec<SmolStr> {
        match self.known_type_name_stack.pop() {
            Some(item) => item,
            None => {
                self.print_stack();
                panic!("No known type name found!")
            },
        }
    }

    /// pop the compatible type for the current node which will uses type=[compatible]
    fn pop_compatible(&mut self) -> Vec<SmolStr> {
        match self.compatible_type_name_stack.pop() {
            Some(item) => item,
            None => {
                self.print_stack();
                panic!("No known type name found!")
            },
        }
    }
}

fn check_node_off(call_params: &CallParams, node_params: &NodeParams) -> bool {
    let mut off = false;
    if let Some(ref option_name) = node_params.option_name {
        off = match call_params.inputs {
            FunctionInputsType::None => true,
            FunctionInputsType::TypeName(ref t_name) => if t_name != option_name { true } else { false },
            FunctionInputsType::TypeNameList(ref t_name_list) => if !t_name_list.contains(&option_name) { true } else { false },
            _ => panic!("Expected None, TypeName or a TypeNameList for function inputs")
        };
    }
    if let Some((ref trigger_name, ref trigger_on)) = node_params.trigger {
        off = match call_params.modifiers {
            CallModifiers::None => *trigger_on,
            CallModifiers::PassThrough => panic!("CallModifiers::PassThrough was not substituted for CallModifiers::StaticList!"),
            CallModifiers::StaticList(ref modifiers) => {
                if modifiers.contains(&trigger_name) { !trigger_on } else { *trigger_on }
            },
        };
    }
    return off;
}

impl<StC: StateChooser> MarkovChainGenerator<StC> {
    pub fn next(&mut self, dyn_model: &mut (impl DynamicModel + ?Sized)) -> Option<<Self as Iterator>::Item> {
        if let Some(call_params) = self.pending_call.take() {
            let inputs = match &call_params.inputs {
                FunctionInputsType::TypeName(t_name) => FunctionInputsType::TypeName(t_name.clone()),
                FunctionInputsType::Known => FunctionInputsType::TypeNameList(self.pop_known()),
                FunctionInputsType::Compatible => FunctionInputsType::TypeNameList(self.pop_compatible()),
                FunctionInputsType::TypeNameList(t_name_list) => FunctionInputsType::TypeNameList(t_name_list.clone()),
                FunctionInputsType::Any => {
                    let function = self.markov_chain.functions.get(&call_params.func_name).unwrap();
                    function.input_type.clone()
                },
                _ => FunctionInputsType::None
            };
            self.call_stack.push(StackItem::from_call_params(CallParams {
                func_name: call_params.func_name.clone(),
                inputs: inputs,
                modifiers: call_params.modifiers.clone(),
            }));
        }

        let stack_item = match self.call_stack.last_mut() {
            Some(stack_item) => stack_item,
            None => {
                self.reset();
                return None;
            },
        };

        let current_node = stack_item.next_node.clone();

        dyn_model.update_current_state(&current_node.name);

        let function = self.markov_chain.functions.get(&stack_item.current_function.func_name).unwrap();
        if current_node.name == function.exit_node_name {
            self.call_stack.pop();
            return Some(current_node.name)
        }

        let cur_node_outgoing = function.chain.get(&current_node.name).unwrap();

        let cur_node_outgoing = cur_node_outgoing.iter().map(|el| {
            (check_node_off(&stack_item.current_function, &el.1), el.0, el.1.clone())
        }).collect::<Vec<_>>();

        let cur_node_outgoing = dyn_model.assign_probabilities(cur_node_outgoing);

        let destination = self.state_chooser.choose_destination(cur_node_outgoing);

        if let Some(destination) = destination {
            if check_node_off(&stack_item.current_function, &destination) {
                panic!("Chosen node is off: {} (after {})", destination.name, current_node.name);
            }
            stack_item.current_node_name = current_node.name.clone();
            stack_item.next_node = destination.clone();
        } else {
            self.print_stack();
            panic!("No destination found for {}.", current_node.name);
        }

        if let Some(call_params) = &current_node.call_params {
            let mut prepared_call_params = call_params.clone();
            prepared_call_params.modifiers = match prepared_call_params.modifiers {
                CallModifiers::PassThrough => stack_item.current_function.modifiers.clone(),
                any => any,
            };
            self.pending_call = Some(prepared_call_params);
        }

        Some(current_node.name)
    }
}

impl<StC: StateChooser> Iterator for MarkovChainGenerator<StC> {
    type Item = SmolStr;

    fn next(&mut self) -> Option<Self::Item> {
        self.next(&mut MarkovModel::new())
    }
}