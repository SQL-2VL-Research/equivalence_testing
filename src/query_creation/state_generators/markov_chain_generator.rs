mod dot_parser;
mod error;
use std::{collections::{HashMap, BinaryHeap}, fs::File, io::Read, path::Path, cmp::Ordering};
use rand::{Rng, SeedableRng};

use regex::Regex;
use smol_str::SmolStr;
use rand_chacha::ChaCha8Rng;

use self::{
    dot_parser::DotTokenizer, error::SyntaxError,
};

pub use dot_parser::FunctionInputsType;

/// this structure contains all the parsed graph functions
#[derive(Clone, Debug)]
struct MarkovChain {
    functions: HashMap<SmolStr, Function>,
}

/// this structure represents a single node with all of
/// its possible properties and modifiers
#[derive(Clone, Debug)]
pub struct NodeParams {
    pub name: SmolStr,
    pub call_params: Option<CallParams>,
    pub option_name: Option<SmolStr>,
    pub literal: bool,
    pub min_calls_until_function_exit: usize
}

/// represents the call parameters passed to a function
/// called with a call node
#[derive(Clone, Debug)]
pub struct CallParams {
    /// which function this call node calls
    func_name: SmolStr,
    /// the output types the called function
    /// should be restricted to generate
    inputs: FunctionInputsType,
    /// the special modifiers passed to the
    /// function called, which affect function
    /// behaviour
    modifiers: Option<Vec<SmolStr>>,
}

/// represents a functional subgraph
#[derive(Clone, Debug)]
struct Function {
    /// this is the source node of a function
    /// the function execution starts here
    source_node_name: SmolStr,
    /// this is the function exit node. When this node
    /// is reached, the function exits and the parent
    /// function continues execution
    exit_node_name: SmolStr,
    /// the output types the function supports to
    /// be restricted to generate, affecting optional
    /// nodes
    input_type: FunctionInputsType,
    /// a vector of special function modifiers. Affects
    /// the function behaviour, but not the nodes
    modifiers: Option<Vec<SmolStr>>,
    /// the chain of the function, contains all of the
    /// function nodes and connectiona between them.
    /// We suppose that it's cheaper by time to clone
    /// NodeParams than to use a separate name -> NodeParams
    /// hashmap, which is why this map stores the
    /// NodeParams directly.
    chain: HashMap<SmolStr, Vec<(f64, NodeParams)>>,
}

impl Function {
    /// create Function struct from its parsed parameters
    fn new(definition: dot_parser::Function) -> Self {
        Function {
            source_node_name: definition.source_node_name,
            exit_node_name: definition.exit_node_name,
            input_type: definition.input_type,
            modifiers: definition.modifiers,
            chain: HashMap::<_, _>::new(),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
struct BinaryHeapNode {
    calls_from_src: usize,
    node_name: SmolStr
}

impl Ord for BinaryHeapNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.calls_from_src.cmp(&self.calls_from_src).then_with(
            || self.node_name.cmp(&other.node_name)
        )
    }
}

impl PartialOrd for BinaryHeapNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl MarkovChain {
    /// parse the chain from a dot file; Initialise all the weights uniformly
    fn parse_dot<P: AsRef<Path>>(source_path: P) -> Result<Self, SyntaxError> {
        let mut file = File::open(source_path).unwrap();
        let mut source = String::new();
        file.read_to_string(&mut source).unwrap();
        let source = MarkovChain::remove_fake_edges(source);
        let (
            mut functions,
            node_params
        ) = MarkovChain::parse_functions_and_params(&source)?;
        MarkovChain::perform_type_checks(&functions, &node_params)?;
        MarkovChain::fill_probs_uniform(&mut functions);
        MarkovChain::fill_paths_to_exit_with_call_nums(&mut functions, &node_params);
        Ok(MarkovChain { functions })
    }

    /// manually removes all the [color=none] edges. Works breaking the lexer paradigm because of a bug.
    /// See https://github.com/maciejhirsz/logos/issues/258 for details.
    fn remove_fake_edges(source: String) -> String {
        let fake_edge_regex = Regex::new(r"[^\r\n]+\[[\s]*color[\s]*=[\s]*none[\s]*\]").unwrap();
        fake_edge_regex.replace_all(&source, "").into_owned()
    }

    /// parse all the functions from the code using the lexer; perform all the nessesary type checks
    fn parse_functions_and_params(source: &str) -> Result<(HashMap<SmolStr, Function>, HashMap<SmolStr, NodeParams>), SyntaxError> {
        let mut functions = HashMap::<_, _>::new();
        // stores node parameters map, needed for faster graph creation & type checks
        let mut node_params = HashMap::<SmolStr, NodeParams>::new();
        let mut current_function = Option::<Function>::None;
        for token in DotTokenizer::from_str(&source) {
            match token? {
                dot_parser::CodeUnit::Function(definition) => {
                    current_function = Some(Function::new(definition.clone()));
                    define_node(&mut current_function, &mut node_params, definition.source_node_name, None, None, false)?;
                    define_node(&mut current_function, &mut node_params, definition.exit_node_name, None, None, false)?;
                }
                dot_parser::CodeUnit::CloseDeclaration => {
                    if let Some(function) = current_function {
                        functions.insert(function.source_node_name.clone(), function);
                        current_function = None;
                    } else {
                        return Err(SyntaxError::new(format!("Unexpected CloseDeclaration")));
                    }
                }
                dot_parser::CodeUnit::NodeDef { name, option_name, literal } => {
                    define_node(&mut current_function, &mut node_params, name, None, option_name, literal)?;
                }
                dot_parser::CodeUnit::Call {
                    node_name,
                    func_name,
                    inputs,
                    modifiers,
                    option_name,
                } => {
                    define_node(&mut current_function, &mut node_params, node_name, Some(CallParams {
                        func_name, inputs, modifiers
                    }), option_name, false)?;
                }
                dot_parser::CodeUnit::Edge {
                    node_name_from,
                    node_name_to,
                } => {
                    if let Some(ref mut function) = current_function {
                        if !function.chain.contains_key(&node_name_to) {
                            return Err(SyntaxError::new(format!(
                                "Cannot build edge: destination node does not exist: {}",
                                node_name_to
                            )));
                        }
                        if let Some(dest_list) = function.chain.get_mut(&node_name_from) {
                            dest_list.push((0f64, node_params.get(&node_name_to).unwrap().clone()));
                        } else {
                            return Err(SyntaxError::new(format!(
                                "Cannot build edge: source node does not exist: {}",
                                node_name_from
                            )));
                        }
                    } else {
                        return Err(SyntaxError::new(format!(
                            "Unexpected Edge: {} -> {}",
                            node_name_from, node_name_to
                        )));
                    }
                }
            };
        }
        Ok((functions, node_params))
    }

    /// performs all the type checks to ensure that the .dot code is correct
    fn perform_type_checks(functions: &HashMap<SmolStr, Function>, node_params: &HashMap<SmolStr, NodeParams>) -> Result<(), SyntaxError> {
        if functions.get("Query").is_none() {
            return Err(SyntaxError::new(format!(
                "Query is the entry point and must be defined: subgraph def_Query {{...}}"
            )));
        }
        for (node_name, params) in node_params {
            if let Some(ref call_params) = params.call_params {
                if let Some(function) = functions.get(&call_params.func_name) {
                    let gen_type_error = |error_msg: String| {
                        format!(
                            "Call node {node_name}: Invalid function arguments; {error_msg} (function source node {})",
                            function.source_node_name
                        )
                    };

                    match call_params.inputs.clone() {
                        FunctionInputsType::TypeName(name) => {
                            if let FunctionInputsType::TypeNameVariants(variants) = &function.input_type {
                                if !variants.contains(&name) {
                                    return Err(SyntaxError::new(gen_type_error(format!(
                                        "Expected one of {:?}, got {:?}", function.input_type, call_params.inputs
                                    ))));
                                }
                            } else {
                                return Err(SyntaxError::new(gen_type_error(format!(
                                    "Got {:?}, but the function type is {:?}", call_params.inputs, function.input_type,
                                ))));
                            }
                        },
                        FunctionInputsType::TypeNameList(types_list) => {
                            if let FunctionInputsType::TypeNameList(func_types_list) = &function.input_type {
                                for c_type in types_list {
                                    if !func_types_list.contains(&c_type) {
                                        return Err(SyntaxError::new(gen_type_error(format!(
                                            "type {} is not in {:?}", c_type, function.input_type
                                        ))));
                                    }
                                }
                            } else {
                                return Err(SyntaxError::new(gen_type_error(format!(
                                    "Got {:?}, but the function type is {:?}", call_params.inputs, function.input_type,
                                ))));
                            }
                        },
                        _ => {}
                    };

                    if let Some(ref modifiers) = call_params.modifiers {
                        if let Some(ref func_modifiers) = function.modifiers {
                            for c_mod in modifiers {
                                if !func_modifiers.contains(&c_mod) {
                                    return Err(SyntaxError::new(gen_type_error(format!(
                                        "modifier {} is not in {:?}", c_mod, function.input_type
                                    ))));
                                }
                            }
                        } else {
                            return Err(SyntaxError::new(gen_type_error(format!(
                                "Call has modifiers {:?} which are not present in the declaration", modifiers
                            ))));
                        }
                    }
                } else {
                    return Err(SyntaxError::new(format!(
                        "Function is not defined: {node_name} (function {})",
                        params.name
                    )));
                }
            }
        }
        Ok(())
    }

    /// fill Markov chain probabilities uniformely.
    fn fill_probs_uniform(functions: &mut HashMap<SmolStr, Function>) {
        for (_, function) in functions {
            for (_, out) in function.chain.iter_mut() {
                let fill_with = 1f64 / (out.len() as f64);
                for (weight, _) in out {
                    *weight = fill_with;
                }
            }
        }
    }

    fn fill_paths_to_exit_with_call_nums(functions: &mut HashMap<SmolStr, Function>, node_params: &HashMap<SmolStr, NodeParams>) {
        let mut min_calls_till_exit = HashMap::<SmolStr, usize>::new();
        for (_, function) in functions.iter() {
            min_calls_till_exit.insert(function.exit_node_name.clone(), 0);
            for (start_node_name, start_node_props) in node_params {
                if !function.chain.contains_key(start_node_name) {
                    continue;
                }
                if min_calls_till_exit.contains_key(start_node_name) {
                    continue;
                }
                min_calls_till_exit.insert(start_node_name.clone(), usize::MAX);

                let mut priority_queue = BinaryHeap::<BinaryHeapNode>::new();
                priority_queue.push(BinaryHeapNode {
                    calls_from_src: 0, node_name: start_node_name.clone(),
                });
                let mut min_calls_from_src_map = HashMap::<SmolStr, usize>::new();
                min_calls_from_src_map.insert(start_node_name.clone(), 0);
                while let Some(BinaryHeapNode { calls_from_src, node_name }) = priority_queue.pop() {
                    if node_name == function.exit_node_name {
                        min_calls_till_exit.insert(start_node_name.clone(),
                            calls_from_src + (start_node_props.call_params.is_some() as usize)
                        );
                        break;
                    }
                    if let Some(min_calls_from_src_map_value) = min_calls_from_src_map.get(&node_name) {
                        if *min_calls_from_src_map_value < calls_from_src {
                            continue;
                        }
                    }
                    for out_node_name in function.chain.get_key_value(&node_name).unwrap().1 {
                        // out_node_name.1.name
                        let node_calls_from_src = calls_from_src + (out_node_name.1.call_params.is_some() as usize);
                        let current_min_calls_from_src = min_calls_from_src_map.entry(
                            out_node_name.1.name.clone()
                        ).or_insert(usize::MAX);
                        if node_calls_from_src < *current_min_calls_from_src {
                            *current_min_calls_from_src = node_calls_from_src;
                            priority_queue.push(BinaryHeapNode {
                                calls_from_src: node_calls_from_src,
                                node_name: out_node_name.1.name.clone(),
                            });
                        }
                    }
                }
            }
        }
        for (_, function) in functions {
            for (_, out_nodes) in function.chain.iter_mut() {
                for out_node in out_nodes {
                    if let Some(min_calls) = min_calls_till_exit.get_key_value(&out_node.1.name) {
                        if *min_calls.1 == usize::MAX {
                            panic!("Node {} does not reach function exit", out_node.1.name);
                        }
                        out_node.1.min_calls_until_function_exit = *min_calls.1;
                    } else {
                        panic!("Node {} was not marked with minimum distance.", out_node.1.name);
                    }
                }
            }
        }
    }
}

/// add a node to the current function graph & node_params map; Perform syntax checks for optional nodes
fn define_node(
        current_function: &mut Option<Function>, node_params: &mut HashMap<SmolStr, NodeParams>,
        node_name: SmolStr, call_params: Option<CallParams>, option_name: Option<SmolStr>,
        literal: bool
    ) -> Result<(), SyntaxError> {
    if let Some(ref mut function) = current_function {
        check_option(&function, &node_name, &option_name)?;
        function.chain.insert(node_name.clone(), Vec::<_>::new());
        node_params.insert(node_name.clone(), NodeParams {
            name: node_name, call_params, option_name,
            literal, min_calls_until_function_exit: 0,
        });
    } else {
        return Err(SyntaxError::new(format!(
            "Unexpected node definition: {node_name} [...]"
        )));
    }
    Ok(())
}

/// Perform syntax checks for optional nodes
fn check_option(current_function: &Function, node_name: &SmolStr, option_name: &Option<SmolStr>) -> Result<(), SyntaxError> {
    if let Some(option_name) = option_name {
        if !(match &current_function.input_type {
            FunctionInputsType::TypeNameList(list) => list,
            FunctionInputsType::TypeNameVariants(list) => list,
            _ => return Err(SyntaxError::new(format!(
                "Unexpected optional node: {node_name}. Function does not acccept arguments"
            )))
        }.contains(option_name)) {
            return Err(SyntaxError::new(format!(
                "Unexpected option: {option_name} Expected one of: {:?}",
                current_function.input_type
            )))
        }
    }
    Ok(())
}

/// The markov chain generator. Runs the functional
/// subgraphs parsed from the .dot file. Manages the
/// probabilities, outputs states, disables nodes if
/// needed.
#[derive(Clone, Debug)]
pub struct MarkovChainGenerator {
    /// this stack contains information about the known type names
    /// inferred when [known] is used in [TYPES]
    known_type_name_stack: Vec<Vec<SmolStr>>,
    /// this stack contains information about the compatible type names
    /// inferred when [compatible] is used in [TYPES]
    compatible_type_name_stack: Vec<Vec<SmolStr>>,
    markov_chain: MarkovChain,
    call_stack: Vec<StackItem>,
    pending_call: Option<CallParams>,
    rng: ChaCha8Rng,
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
            }
        }
    }
}

impl MarkovChainGenerator {
    pub fn parse_graph_from_file<P: AsRef<Path>>(source_path: P) -> Result<Self, SyntaxError> {
        let chain = MarkovChain::parse_dot(source_path)?;
        let mut self_ = MarkovChainGenerator {
            markov_chain: chain,
            call_stack: vec![],
            pending_call: None,
            known_type_name_stack: vec![],
            compatible_type_name_stack: vec![],
            rng: ChaCha8Rng::seed_from_u64(1),
        };
        self_.reset();
        Ok(self_)
    }

    /// used to print the call stack of the markov chain functions
    pub fn print_stack(&self) {
        println!("Call stack:");
        for stack_item in &self.call_stack {
            println!("{} ({}):", stack_item.current_function.func_name, stack_item.current_node_name)
        }
    }

    /// used when the markov chain reaches end, for the object to be iterable multiple times
    pub fn reset(&mut self) {
        self.call_stack = vec![StackItem::from_call_params(CallParams {
            func_name: SmolStr::new("Query"),
            inputs: FunctionInputsType::None,
            modifiers: None
        })];
        self.known_type_name_stack = vec![vec![SmolStr::new("")]];
        self.compatible_type_name_stack = vec![vec![SmolStr::new("")]];
    }

    /// get crrent function inputs list
    pub fn get_inputs(&self) -> FunctionInputsType {
        self.call_stack.last().unwrap().current_function.inputs.clone()
    }

    /// get crrent function modifiers list
    pub fn get_modifiers(&self) -> Option<Vec<SmolStr>> {
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

fn check_node_off(function_inputs_conv: &FunctionInputsType, option_name: &Option<SmolStr>) -> bool {
    if let Some(option_name) = option_name {
        match function_inputs_conv {
            FunctionInputsType::None => return true,
            FunctionInputsType::TypeName(t_name) => if t_name != option_name { return true },
            FunctionInputsType::TypeNameList(t_name_list) => if !t_name_list.contains(&option_name) { return true },
            _ => panic!("Must be None, TypeName or a TypeNameList")
        }
    }
    return false;
}

pub trait DynamicModel {
    /// assigns the (unnormalized) probabilities to a dynamic model. Receives probabilities recorded in graph,
    /// with zeroes in place for the deselected nodes. The probabilities on input are thus unnormalized, too
    fn assign_probabilities(&mut self, node_outgoing: Vec<(f64, NodeParams)>) -> Vec::<(f64, NodeParams)>;
}

pub struct DefaultModel { }

impl DefaultModel {
    pub fn new() -> Self {
        Self {}
    }
}

impl DynamicModel for DefaultModel {
    fn assign_probabilities(&mut self, node_outgoing: Vec<(f64, NodeParams)>) -> Vec::<(f64, NodeParams)> {
        node_outgoing
    }
}

impl MarkovChainGenerator {
    pub fn next(&mut self, dyn_model: &mut impl DynamicModel) -> Option<<Self as Iterator>::Item> {
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

        let function = self.markov_chain.functions.get(&stack_item.current_function.func_name).unwrap();
        if current_node.name == function.exit_node_name {
            self.call_stack.pop();
            return Some(current_node.name)
        }

        let cur_node_outgoing = function.chain.get(&current_node.name).unwrap();

        let cur_node_outgoing: Vec<(f64, NodeParams)> = {
            let cur_node_outgoing = cur_node_outgoing.iter().map(|el| {
                (if check_node_off(&stack_item.current_function.inputs, &el.1.option_name) {
                    0f64
                } else {
                    el.0
                }, el.1.clone())
            }).collect::<Vec<_>>();
            let cur_node_outgoing = dyn_model.assign_probabilities(cur_node_outgoing);
            let max_level: f64 = cur_node_outgoing.iter().map(|el| { el.0 }).sum();
            cur_node_outgoing.into_iter().map(|el| { (el.0 / max_level, el.1) }).collect()
        };
        let level: f64 = self.rng.gen::<f64>();
        let mut cumulative_prob = 0f64;
        let mut destination = Option::<NodeParams>::None;
        for (prob, dest) in cur_node_outgoing {
            cumulative_prob += prob;
            if level < cumulative_prob {
                destination = Some(dest);
                break;
            }
        }
        if destination.is_none() {
            panic!("No destination found for {}. stack_item.current_function.inputs={:?}", current_node.name, stack_item.current_function.inputs);
        }
        stack_item.current_node_name = current_node.name.clone();
        stack_item.next_node = destination.unwrap().clone();

        if let Some(call_params) = &current_node.call_params {
            self.pending_call = Some(call_params.clone());
        }

        // println!("{}", current_node.name);

        Some(current_node.name)
    }
}

impl Iterator for MarkovChainGenerator {
    type Item = SmolStr;

    fn next(&mut self) -> Option<Self::Item> {
        self.next(&mut DefaultModel::new())
    }
}