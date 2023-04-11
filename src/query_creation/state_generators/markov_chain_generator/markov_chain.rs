use std::{collections::{HashMap, BinaryHeap}, fs::File, io::Read, path::Path, cmp::Ordering};

use regex::Regex;
use core::fmt::Debug;
use smol_str::SmolStr;

use super::{
    dot_parser, dot_parser::{DotTokenizer, FunctionInputsType, SubgraphType, CodeUnit}, error::SyntaxError,
};

/// this structure contains all the parsed graph functions
#[derive(Clone, Debug)]
pub struct MarkovChain {
    pub functions: HashMap<SmolStr, Function>,
}

/// this structure represents a single node with all of
/// its possible properties and modifiers
#[derive(Clone, Debug)]
pub struct NodeParams {
    pub name: SmolStr,
    pub call_params: Option<CallParams>,
    pub type_name: Option<SubgraphType>,
    pub literal: bool,
    pub min_calls_until_function_exit: usize,
    pub trigger: Option<(SmolStr, bool)>
}

/// represents modifiers passed in call parameters.
/// Can be either None, list of static modifiers, or a
/// pass-through (pass current funciton's arguments)
#[derive(Clone, Debug, PartialEq)]
pub enum CallModifiers {
    None,
    PassThrough,
    StaticList(Vec<SmolStr>),
}

#[derive(Clone, Debug)]
pub enum CallTypes {
    /// Сhoose none of the types out of the allowed type list
    None,
    /// Used to be able to set the type later in the handler code.
    /// Is transformed into a Type(..).
    Known,
    /// Used to be able to set the type later in the handler code..
    /// Is transformed into a TypeList(..).
    KnownList,
    /// Used to be able to set the type later. Similar to known, but indicates type compatibility.
    /// Will be extended in the future to link to the types call node which would supply the type.
    /// Is later transformed into a TypeList(..).
    Compatible,
    /// Select a type among type variants.
    Type(SubgraphType),
    /// Select multiple types among possible type variants.
    TypeList(Vec<SubgraphType>),
}

impl CallTypes {
    fn from_function_inputs_type(node_name: &SmolStr, accepted_types: &FunctionTypes, input: FunctionInputsType) -> CallTypes {
        match input {
            FunctionInputsType::Any => {
                match accepted_types {
                    FunctionTypes::None => panic!("Incorrect input type for node {node_name}: Any. Function does not accept arguments."),
                    FunctionTypes::TypeList(list) => CallTypes::TypeList(list.clone()),
                    FunctionTypes::TypeVariants(_) => panic!("Incorrect input type for node {node_name}: Any. Function only accepts type variants."),
                }
            },
            FunctionInputsType::None => CallTypes::None,
            FunctionInputsType::Known => CallTypes::Known,
            FunctionInputsType::KnownList => CallTypes::KnownList,
            FunctionInputsType::Compatible => CallTypes::Compatible,
            FunctionInputsType::TypeName(tp) => CallTypes::Type(tp),
            FunctionInputsType::TypeNameList(tp_list) => CallTypes::TypeList(tp_list),
            any => panic!("Incorrect input type for node {node_name}: {:?}", any),
        }
    }
}

/// represents the call parameters passed to a function
/// called with a call node
#[derive(Clone, Debug)]
pub struct CallParams {
    /// which function this call node calls
    pub func_name: SmolStr,
    /// the output types the called function
    /// should be restricted to generate
    pub selected_types: CallTypes,
    /// the special modifiers passed to the
    /// function called, which affect function
    /// behaviour
    pub modifiers: CallModifiers,
}

#[derive(Clone, Debug)]
pub enum FunctionTypes {
    /// Indicate that no types are accepted.
    None,
    /// Specify the allowed type list, of which multiple can be selected.
    TypeList(Vec<SubgraphType>),
    /// Specify possible type variants, of which only one can be selected.
    TypeVariants(Vec<SubgraphType>),
}

impl FunctionTypes {
    fn from_function_inputs_type(source_node_name: SmolStr, input: FunctionInputsType) -> FunctionTypes {
        match input {
            FunctionInputsType::None => FunctionTypes::None,
            FunctionInputsType::TypeNameList(list) => FunctionTypes::TypeList(list),
            FunctionInputsType::TypeNameVariants(variants) => FunctionTypes::TypeVariants(variants),
            any => panic!("Incorrect input type for function {}: {:?}", source_node_name, any),
        }
    }
}

/// represents a functional subgraph
#[derive(Clone, Debug)]
pub struct Function {
    /// this is the source node of a function
    /// the function execution starts here
    pub source_node_name: SmolStr,
    /// this is the function exit node. When this node
    /// is reached, the function exits and the parent
    /// function continues execution
    pub exit_node_name: SmolStr,
    /// the output types the function supports to
    /// be restricted to generate, affecting optional
    /// nodes
    pub accepted_types: FunctionTypes,
    /// a vector of special function modifiers. Affects
    /// the function behaviour, but not the nodes
    pub modifiers: Option<Vec<SmolStr>>,
    /// the chain of the function, contains all of the
    /// function nodes and connectiona between them.
    /// We suppose that it's cheaper by time to clone
    /// NodeParams than to use a separate name -> NodeParams
    /// hashmap, which is why this map stores the
    /// NodeParams directly.
    pub chain: HashMap<SmolStr, Vec<(f64, NodeParams)>>,
}

impl Function {
    /// create Function struct from its parsed parameters
    fn new(definition: dot_parser::Function) -> Self {
        Function {
            source_node_name: definition.source_node_name.clone(),
            exit_node_name: definition.exit_node_name,
            accepted_types: FunctionTypes::from_function_inputs_type(definition.source_node_name, definition.input_type),
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
    pub fn parse_dot<P: AsRef<Path>>(source_path: P) -> Result<Self, SyntaxError> {
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
        let tokens = DotTokenizer::from_str(&source).collect::<Result<Vec<CodeUnit>, SyntaxError>>()?;
        // Run through all function definitions
        for token in &tokens {
            match token {
                dot_parser::CodeUnit::Function(definition) => {
                    let mut function = Some(Function::new(definition.clone()));
                    define_node(&mut function, &mut node_params, definition.source_node_name.clone(), None, None, false, None)?;
                    define_node(&mut function, &mut node_params, definition.exit_node_name.clone(), None, None, false, None)?;
                    let function = function.unwrap();
                    functions.insert(function.source_node_name.clone(), function);
                },
                _ => {}
            }
        }
        // Fill in function bodies
        for token in tokens {
            match token {
                dot_parser::CodeUnit::Function(definition) => {
                    current_function = Some(functions.remove(&definition.source_node_name).unwrap());
                }
                dot_parser::CodeUnit::CloseDeclaration => {
                    if let Some(function) = current_function.take() {
                        functions.insert(function.source_node_name.clone(), function);
                    } else {
                        return Err(SyntaxError::new(format!("Unexpected CloseDeclaration")));
                    }
                }
                dot_parser::CodeUnit::NodeDef { name, type_name: option_name, literal, trigger } => {
                    define_node(&mut current_function, &mut node_params, name, None, option_name, literal, trigger)?;
                }
                dot_parser::CodeUnit::Call {
                    node_name,
                    func_name,
                    inputs,
                    modifiers,
                    type_name,
                    trigger,
                } => {
                    let modifiers = match modifiers {
                        Some(modifiers) => {
                            if modifiers.contains(&SmolStr::new("...")) {
                                if modifiers.len() != 1 {
                                    return Err(SyntaxError::new(format!(
                                        "{node_name}: when modifier '...' is present, it should be the only modifier, but got {:?}", modifiers
                                    )));
                                }
                                CallModifiers::PassThrough
                            } else {
                                CallModifiers::StaticList(modifiers)
                            }
                        },
                        None => CallModifiers::None,
                    };
                    let selected_types = {
                        let accepted_types = if let Some(ref function) = current_function {
                            if function.source_node_name == func_name {
                                &function.accepted_types
                            } else {
                                &functions.get(&func_name).ok_or_else(|| SyntaxError::new(format!(
                                    "Call node {node_name} calls non-existing function: {func_name}"
                                )))?.accepted_types
                            }
                        } else {
                            return Err(SyntaxError::new(format!(
                                "Unexpected node definition: {node_name} [...]"
                            )));
                        };
                        CallTypes::from_function_inputs_type(
                            &node_name, accepted_types, inputs
                        )
                    };
                    define_node(&mut current_function, &mut node_params, node_name, Some(
                        CallParams { func_name, selected_types, modifiers }
                    ), type_name, false, trigger)?;
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

                    match (&call_params.selected_types, &function.accepted_types) {
                        (
                            CallTypes::Type(name),
                            FunctionTypes::TypeVariants(variants)
                        ) if variants.contains(name) => {},
                        (
                            CallTypes::TypeList(types_list),
                            FunctionTypes::TypeList(func_types_list)
                        ) if types_list.iter().all(|t| func_types_list.contains(t)) => {},
                        (CallTypes::None, FunctionTypes::TypeList(..) | FunctionTypes::None) => {},
                        (
                            CallTypes::Compatible, FunctionTypes::TypeList(..) | FunctionTypes::TypeVariants(..)
                        ) => {},
                        (CallTypes::Known, FunctionTypes::TypeVariants(..)) => {},
                        (CallTypes::KnownList, FunctionTypes::TypeList(..)) => {},
                        _ => return Err(SyntaxError::new(gen_type_error(format!(
                            "Got {:?}, but the function type is {:?}", call_params.selected_types, function.accepted_types,
                        ))))
                    }

                    if call_params.modifiers != CallModifiers::None {
                        if let Some(ref func_modifiers) = function.modifiers {
                            match call_params.modifiers {
                                CallModifiers::None => {},
                                CallModifiers::PassThrough => {},
                                CallModifiers::StaticList(ref modifiers) => {
                                    for c_mod in modifiers {
                                        if !func_modifiers.contains(&c_mod) {
                                            return Err(SyntaxError::new(gen_type_error(format!(
                                                "Modifier {} is not in {:?}", c_mod, func_modifiers
                                            ))));
                                        }
                                    }
                                },
                            }
                        } else {
                            return Err(SyntaxError::new(gen_type_error(format!(
                                "Call has modifiers {:?} but no modifiers are present in the function declaration", call_params.modifiers
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
        node_name: SmolStr, call_params: Option<CallParams>, type_name_opt: Option<SubgraphType>,
        literal: bool, trigger: Option<(SmolStr, bool)>
    ) -> Result<(), SyntaxError> {
    if let Some(function) = current_function {
        check_type(&function, &node_name, &type_name_opt)?;
        check_trigger(&function, &node_name, &trigger)?;
        function.chain.insert(node_name.clone(), Vec::<_>::new());
        node_params.insert(node_name.clone(), NodeParams {
            name: node_name, call_params, type_name: type_name_opt,
            literal, min_calls_until_function_exit: 0,
            trigger
        });
    } else {
        return Err(SyntaxError::new(format!(
            "Unexpected node definition: {node_name} [...]"
        )));
    }
    Ok(())
}

/// Perform checks for typed nodes
fn check_type(current_function: &Function, node_name: &SmolStr, type_name_opt: &Option<SubgraphType>) -> Result<(), SyntaxError> {
    if let Some(type_name) = type_name_opt {
        if !(match &current_function.accepted_types {
            FunctionTypes::TypeList(list) => list,
            FunctionTypes::TypeVariants(list) => list,
            _ => return Err(SyntaxError::new(format!(
                "Unexpected typed node: {node_name}. Function does not acccept arguments"
            )))
        }.contains(type_name)) {
            return Err(SyntaxError::new(format!(
                "Unexpected option: {type_name} Expected one of: {:?}",
                current_function.accepted_types
            )))
        }
    }
    Ok(())
}

/// Perform syntax checks for trigger nodes
fn check_trigger(current_function: &Function, node_name: &SmolStr, trigger: &Option<(SmolStr, bool)>) -> Result<(), SyntaxError> {
    if let Some((trigger_name, _)) = trigger {
        if !(match &current_function.modifiers {
            Some(list) => list,
            None => return Err(SyntaxError::new(format!(
                "Unexpected trigger node: {node_name}. Function does not acccept modifiers"
            )))
        }.contains(trigger_name)) {
            return Err(SyntaxError::new(format!(
                "Unexpected trigger: {trigger_name} Expected one of: {:?}",
                current_function.modifiers
            )))
        }
    }
    Ok(())
}