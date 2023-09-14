use std::{collections::{HashMap, BinaryHeap}, path::Path, cmp::Ordering};

use regex::Regex;
use core::fmt::Debug;
use smol_str::SmolStr;

use super::{
    dot_parser, dot_parser::{DotTokenizer, FunctionInputsType, CodeUnit, NodeCommon, FunctionDeclaration, TypeWithFields}, error::SyntaxError, subgraph_type::SubgraphType,
};

/// this structure contains all the parsed graph functions
#[derive(Clone, Debug)]
pub struct MarkovChain {
    pub functions: HashMap<SmolStr, Function>,
}

/// this structure represents a single node with all of
/// its possible properties and modifiers
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeParams {
    pub node_common: NodeCommon,
    pub call_params: Option<CallParams>,
    pub literal: bool,
    pub min_calls_until_function_exit: usize,
}

/// represents modifiers passed in call parameters.
/// Can be either None, list of static modifiers, or a
/// pass-through (pass current funciton's arguments)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CallModifiers {
    /// none of the modifiers are activated
    None,
    /// Used when we want to pass the function's modifiers further
    /// Optionally, adds new modifiers
    StaticListWithParentMods(Vec<SmolStr>),
    /// USed to specify a static list of modifiers
    StaticList(Vec<SmolStr>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CallTypes {
    /// Сhoose none of the types out of the allowed type list
    None,
    /// Used to be able to set the type later in the handler code..
    /// Is transformed into a TypeList(..).
    KnownList,
    /// Used to be able to set the type later. Similar to known, but indicates type compatibility.
    /// Will be extended in the future to link to the types call node which would supply the type.
    /// Is later transformed into a TypeList(..).
    Compatible,
    /// Used when we want to pass the function's type constraints further
    PassThrough,
    /// Select multiple types among possible type variants.
    TypeList(Vec<SubgraphType>),
    /// Select multiple types among possible type variants.
    TypeListWithFields(Vec<TypeWithFields>),
}

impl CallTypes {
    fn from_function_inputs_type(node_name: &SmolStr, accepted_types: &FunctionTypes, input: FunctionInputsType) -> CallTypes {
        match input {
            FunctionInputsType::Any => {
                match accepted_types {
                    FunctionTypes::None => panic!("Incorrect input type for node {node_name}: Any. Function does not accept arguments."),
                    FunctionTypes::TypeList(list) => CallTypes::TypeList(list.clone()),
                }
            },
            FunctionInputsType::None => CallTypes::None,
            FunctionInputsType::KnownList => CallTypes::KnownList,
            FunctionInputsType::Compatible => CallTypes::Compatible,
            FunctionInputsType::PassThrough => CallTypes::PassThrough,
            FunctionInputsType::TypeListWithFields(tp_list) => CallTypes::TypeListWithFields(tp_list),
        }
    }
}

/// represents the call parameters passed to a function
/// called with a call node
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
}

impl FunctionTypes {
    fn from_function_inputs_type(source_node_name: &SmolStr, input: FunctionInputsType) -> FunctionTypes {
        match input {
            FunctionInputsType::None => FunctionTypes::None,
            FunctionInputsType::TypeListWithFields(list) => {
                FunctionTypes::TypeList(list.into_iter().map(|TypeWithFields::Type(x)| x).collect())
            },
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
    pub accepted_modifiers: Option<Vec<SmolStr>>,
    /// the chain of the function, contains all of the
    /// function nodes and connectiona between them.
    /// We suppose that it's cheaper by time to clone
    /// NodeParams than to use a separate name -> NodeParams
    /// hashmap, which is why this map stores the
    /// NodeParams directly.
    pub chain: HashMap<SmolStr, Vec<(f64, NodeParams)>>,
    /// function node params
    pub node_params: HashMap<SmolStr, NodeParams>,
}

impl Function {
    /// create Function struct from its parsed parameters
    fn new(declaration: FunctionDeclaration) -> Self {
        Function {
            accepted_types: FunctionTypes::from_function_inputs_type(&declaration.source_node_name, declaration.input_type),
            accepted_modifiers: declaration.modifiers,
            source_node_name: declaration.source_node_name,
            exit_node_name: declaration.exit_node_name,
            chain: HashMap::<_, _>::new(),
            node_params: HashMap::new(),
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
        let source = std::fs::read_to_string(source_path).unwrap();
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
                dot_parser::CodeUnit::FunctionDeclaration(declaration) => {
                    let mut function = Some(Function::new(declaration.clone()));
                    define_node(&mut function, &mut node_params, NodeCommon::with_name(declaration.source_node_name.clone()), None, false)?;
                    define_node(&mut function, &mut node_params, NodeCommon::with_name(declaration.exit_node_name.clone()), None, false)?;
                    let function = function.unwrap();
                    functions.insert(function.source_node_name.clone(), function);
                },
                _ => {}
            }
        }
        // Fill in function bodies
        for token in tokens {
            match token {
                dot_parser::CodeUnit::FunctionDeclaration(declaration) => {
                    current_function = Some(functions.remove(&declaration.source_node_name).unwrap());
                }
                dot_parser::CodeUnit::CloseDeclaration => {
                    if let Some(mut function) = current_function.take() {
                        function.node_params = function.chain.keys().map(|node_name| (
                            node_name.clone(), node_params.get(node_name).unwrap().clone()
                        )).collect();

                        functions.insert(function.source_node_name.clone(), function);
                    } else {
                        return Err(SyntaxError::new(format!("Unexpected CloseDeclaration")));
                    }
                }
                dot_parser::CodeUnit::RegularNode { node_common, literal} => {
                    define_node(&mut current_function, &mut node_params, node_common, None, literal)?;
                }
                dot_parser::CodeUnit::CallNode {
                    node_common,
                    func_name,
                    inputs,
                    modifiers,
                } => {
                    let modifiers = match modifiers {
                        Some(mut modifiers) => {
                            if modifiers.contains(&SmolStr::new("...")) {
                                modifiers.retain(|x| x != "...");
                                CallModifiers::StaticListWithParentMods(modifiers)
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
                                    "Call node {} calls non-existing function: {func_name}", node_common.name
                                )))?.accepted_types
                            }
                        } else {
                            return Err(SyntaxError::new(format!(
                                "Unexpected node definition: {} [...]", node_common.name
                            )));
                        };
                        CallTypes::from_function_inputs_type(
                            &node_common.name, accepted_types, inputs
                        )
                    };
                    define_node(&mut current_function, &mut node_params, node_common, Some(
                        CallParams { func_name, selected_types, modifiers }
                    ), false)?;
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
                            CallTypes::TypeList(types_list),
                            FunctionTypes::TypeList(func_types_list)
                        ) if types_list.iter().all(|t| func_types_list.contains(t)) => {},
                        (
                            CallTypes::TypeListWithFields(types_list),
                            FunctionTypes::TypeList(func_types_list)
                        ) if types_list.iter().all(|t| func_types_list.contains(t.inner_ref())) => {},
                        (CallTypes::None, FunctionTypes::TypeList(..) | FunctionTypes::None) => {},
                        (
                            CallTypes::PassThrough, FunctionTypes::TypeList(..)
                        ) => {},
                        (CallTypes::Compatible, FunctionTypes::TypeList(..)) => {},
                        (CallTypes::KnownList, FunctionTypes::TypeList(..)) => {},
                        _ => return Err(SyntaxError::new(gen_type_error(format!(
                            "Got {:?}, but the function type is {:?}", call_params.selected_types, function.accepted_types,
                        ))))
                    }

                    if call_params.modifiers != CallModifiers::None {
                        if let Some(ref func_modifiers) = function.accepted_modifiers {
                            match call_params.modifiers {
                                CallModifiers::None => {},
                                CallModifiers::StaticListWithParentMods(ref modifiers)
                                | CallModifiers::StaticList(ref modifiers) => {
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
                        params.node_common.name
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
                        // out_node_name.1.node_common.name
                        let node_calls_from_src = calls_from_src + (out_node_name.1.call_params.is_some() as usize);
                        let current_min_calls_from_src = min_calls_from_src_map.entry(
                            out_node_name.1.node_common.name.clone()
                        ).or_insert(usize::MAX);
                        if node_calls_from_src < *current_min_calls_from_src {
                            *current_min_calls_from_src = node_calls_from_src;
                            priority_queue.push(BinaryHeapNode {
                                calls_from_src: node_calls_from_src,
                                node_name: out_node_name.1.node_common.name.clone(),
                            });
                        }
                    }
                }
            }
        }
        for (_, function) in functions {
            for (_, out_nodes) in function.chain.iter_mut() {
                for (_, out_node) in out_nodes {
                    if let Some(min_calls) = min_calls_till_exit.get(&out_node.node_common.name) {
                        if *min_calls == usize::MAX {
                            panic!("Node {} does not reach function exit", out_node.node_common.name);
                        }
                        out_node.min_calls_until_function_exit = *min_calls;
                    } else {
                        panic!("Node {} was not marked with minimum distance.", out_node.node_common.name);
                    }
                }
            }
            for (_, node) in function.node_params.iter_mut() {
                if let Some(min_calls) = min_calls_till_exit.get(&node.node_common.name) {
                    node.min_calls_until_function_exit = *min_calls;
                }
            }
        }
    }
}

/// add a node to the current function graph & node_params map; Perform syntax checks for optional nodes
fn define_node(
        current_function: &mut Option<Function>, node_params: &mut HashMap<SmolStr, NodeParams>,
        node_common: NodeCommon, call_params: Option<CallParams>, literal: bool
    ) -> Result<(), SyntaxError> {
    if let Some(function) = current_function {
        check_type(&function, &node_common.name, &node_common.type_name)?;
        check_modifier(&function, &node_common.name, &node_common.modifier)?;
        function.chain.insert(node_common.name.clone(), Vec::<_>::new());
        node_params.insert(node_common.name.clone(), NodeParams {
            node_common: node_common, call_params,
            literal, min_calls_until_function_exit: 0,
        });
    } else {
        return Err(SyntaxError::new(format!(
            "Unexpected node definition: {} [...]", node_common.name
        )));
    }
    Ok(())
}

/// Perform checks for typed nodes
fn check_type(current_function: &Function, node_name: &SmolStr, type_name_opt: &Option<SubgraphType>) -> Result<(), SyntaxError> {
    if let Some(type_name) = type_name_opt {
        if !(match &current_function.accepted_types {
            FunctionTypes::TypeList(list) => list,
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

/// Perform syntax checks for modifier nodes
fn check_modifier(current_function: &Function, node_name: &SmolStr, modifier: &Option<(SmolStr, bool)>) -> Result<(), SyntaxError> {
    if let Some((modifier_name, _)) = modifier {
        if !(match &current_function.accepted_modifiers {
            Some(list) => list,
            None => return Err(SyntaxError::new(format!(
                "Unexpected modifier node: {node_name}. Function does not acccept modifiers"
            )))
        }.contains(modifier_name)) {
            return Err(SyntaxError::new(format!(
                "Unexpected modifier: {modifier_name} Expected one of: {:?}",
                current_function.accepted_modifiers
            )))
        }
    }
    Ok(())
}