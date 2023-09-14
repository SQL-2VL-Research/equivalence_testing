pub mod state_choosers;
pub mod dynamic_models;
pub mod markov_chain;
pub mod subgraph_type;
pub mod dot_parser;
mod error;

use std::{path::PathBuf, collections::{HashMap, HashSet, VecDeque, BTreeMap}, sync::{Arc, Mutex}};

use core::fmt::Debug;
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;
use smol_str::SmolStr;
use take_until::TakeUntilExt;

use crate::{unwrap_variant, query_creation::{state_generators::markov_chain_generator::markov_chain::FunctionTypes, random_query_generator::{query_info::ClauseContext, call_modifiers::{StatelessCallModifier, StatefulCallModifier, IsColumnTypeAvailableModifier, TypesTypeValueSetter, ValueSetter, NamedValue}}}, config::TomlReadable};

use self::{
    markov_chain::{
        MarkovChain, NodeParams, CallParams, CallModifiers, Function
    }, error::SyntaxError, dot_parser::{NodeCommon, TypeWithFields}, subgraph_type::SubgraphType
};

use state_choosers::StateChooser;
use dynamic_models::{MarkovModel, DynamicModel};

pub use self::markov_chain::CallTypes;

#[derive(Clone)]
pub struct StateGeneratorConfig {
    pub graph_file_path: PathBuf,
}

impl TomlReadable for StateGeneratorConfig {
    fn from_toml(toml_config: &toml::Value) -> Self {
        let section = &toml_config["state_generator"];
        Self {
            graph_file_path: PathBuf::from(section["graph_file_path"].as_str().unwrap()),
        }
    }
}

#[derive(Debug)]
struct StatefulCallModifierCreator(fn() -> Box<dyn StatefulCallModifier>);

/// The markov chain generator. Runs the functional
/// subgraphs parsed from the .dot file. Manages the
/// probabilities, outputs states, disables nodes if
/// needed.
#[derive(Debug)]
pub struct MarkovChainGenerator<StC: StateChooser> {
    markov_chain: MarkovChain,
    call_stack: Vec<StackFrame>,
    pending_call: Option<CallParams>,
    state_chooser: Box<StC>,
    value_setters: HashMap<SmolStr, Box<dyn ValueSetter>>,
    stateless_call_modifiers: HashMap<SmolStr, Box<dyn StatelessCallModifier>>,
    stateful_call_modifier_creators: HashMap<SmolStr, StatefulCallModifierCreator>,
    function_modifier_info: HashMap<SmolStr, FunctionModifierInfo>,
    /// if a dead_end_info was collected for the specific function once,
    /// with set arguments (types & modifiers), and all call modifier
    /// states, depending on which affectors affected what and how,
    /// we can re-use that information.
    dead_end_infos: HashMap<(CallParams, BTreeMap<SmolStr, BTreeMap<SmolStr, bool>>), Arc<Mutex<HashMap<SmolStr, bool>>>>,
}

#[derive(Debug)]
pub struct StackFrame {
    /// function context, like the current node, and the
    /// current function arguments (call params)
    pub function_context: FunctionContext,
    /// various call modifier info
    call_modifier_info: CallModifierInfo,
    /// Cached version of all the dead ends
    /// in the current function
    dead_end_info: Arc<Mutex<HashMap<SmolStr, bool>>>,
    /// this contains information about the known type name list
    /// inferred when [known] is used in TYPES
    known_type_list: Option<Vec<SubgraphType>>,
    /// this contains information about the compatible type name list
    /// inferred when [compatible] is used in [TYPES]
    compatible_type_list: Option<Vec<SubgraphType>>,
}

impl StackFrame {
    fn new(
        function_context: FunctionContext,
        call_modifier_info: CallModifierInfo,
        dead_end_info: Arc<Mutex<HashMap<SmolStr, bool>>>
    ) -> Self {
        Self {
            function_context,
            call_modifier_info,
            dead_end_info,
            known_type_list: None,
            compatible_type_list: None,
        }
    }
}

#[derive(Debug)]
pub struct FunctionModifierInfo {
    /// names of all stateful modifiers present in function
    stateful_modifier_names: HashSet<SmolStr>,
    /// every node that affects a modifier, with the affected modifier names and vector of
    /// corresponding affected nodes
    affector_nodes_and_affected_nodes: HashMap<NodeParams, HashMap<SmolStr, Vec<NodeParams>>>,
}

impl FunctionModifierInfo {
    fn new(
        function_node_params: &HashMap<SmolStr, NodeParams>,
        stateless_call_modifiers: &HashMap<SmolStr, Box<dyn StatelessCallModifier>>,
        stateful_call_modifier_creators: &HashMap<SmolStr, StatefulCallModifierCreator>,
    ) -> Self {
        // modifier name -> modified nodes
        let modified_nodes_map = function_node_params.values()
            .filter_map(|node| {
                node.node_common.call_modifier_name.clone().map(|modifier_name| (
                    modifier_name, node.clone()
                ))
            })
            .fold(HashMap::new(), |mut acc, (key, value)| {
                acc.entry(key).or_insert_with(Vec::new).push(value);
                acc
            });

        let (stateful_modifier_names, stateless_modifier_names): (_, HashSet<_>) = modified_nodes_map.keys().cloned().partition(
            |modifier_name| stateful_call_modifier_creators.contains_key(modifier_name)
        );

        // modifier name -> associated value
        let associated_values_map: HashMap<_, _> = stateless_modifier_names.iter().map(
            |modifier_name| (
                modifier_name,
                stateless_call_modifiers.get(modifier_name).unwrap_or_else(|| panic!(
                    "Stateless modifier {modifier_name} wasn't registered"
                )).get_associated_value_name()
            )
        ).collect();

        let affector_nodes_and_affected_nodes = function_node_params.values()
            .map(|node| {
                (
                    node.node_common.affects_call_modifier_name.as_ref(),
                    node.node_common.sets_value_name.as_ref(),
                    node
                )
            })
            .filter(|(mod_name, v_name, _)| mod_name.is_some() || v_name.is_some())
            .map(|(affects_modifier_name, sets_value_name, affector_node)| {
                let mut affected = HashMap::new();

                if let Some(modifier_name) = affects_modifier_name {
                    affected.extend(modified_nodes_map.iter()
                        .filter(|(affected_modifier_name, _)| *affected_modifier_name == modifier_name)
                        .map(|x| (x.0.clone(), x.1.clone()))
                    );
                }

                if let Some(value_name) = sets_value_name {
                    affected.extend(modified_nodes_map.iter()
                        .filter(|(affected_modifier_name, _)| associated_values_map
                            .get(affected_modifier_name)
                            .map_or(false,|accosiated_value| accosiated_value == value_name)
                        )
                        .map(|x| (x.0.clone(), x.1.clone()))
                    );
                }

                (
                    affector_node.clone(),
                    affected
                )
            })
            .collect();

        Self {
            stateful_modifier_names,
            affector_nodes_and_affected_nodes,
        }
    }

    /// return all the stateful modifiers of the current function in
    /// their initial state
    fn create_stateful_modifiers(
        &self, stateful_call_modifier_creators: &HashMap<SmolStr, StatefulCallModifierCreator>
    ) -> HashMap<SmolStr, Box<dyn StatefulCallModifier>> {
        self.stateful_modifier_names.iter()
            .filter_map(|modifier_name| stateful_call_modifier_creators.get(modifier_name).map(
                |x| (modifier_name.clone(), x.0())
            )).collect()
    }

    /// Returns how running of each value setter affects each stateless-modified node
    fn get_stateless_node_relations(
        &self,
        function_context: &FunctionContext,
        clause_context: &ClauseContext,
        value_setters: &HashMap<SmolStr, Box<dyn ValueSetter>>,
        stateless_call_modifiers: &HashMap<SmolStr, Box<dyn StatelessCallModifier>>,
    ) -> BTreeMap<SmolStr, BTreeMap<SmolStr, bool>> {
        let stateless_affectors_iter = self.affector_nodes_and_affected_nodes.iter()
            .filter(|(affector_node, _)| {
                affector_node.node_common.sets_value_name.as_ref().map_or(false, |value_name| {
                    stateless_call_modifiers.iter().any(
                        |(_, modifier)| &modifier.get_associated_value_name() == value_name
                    )
                })
            });

        stateless_affectors_iter
            .map(|(affector_node, affected_mods)|
                (affector_node.node_common.name.clone(), affected_mods.into_iter().flat_map(|(modifier_name, affected_nodes)| {
                    let stateless_modifier = stateless_call_modifiers.get(modifier_name).unwrap();
                    let value_setter = value_setters.get(affector_node.node_common.sets_value_name.as_ref().unwrap()).unwrap();
                    let new_state = value_setter.get_value(
                        clause_context, &function_context.with_node(affector_node.clone())
                    );
                    affected_nodes.into_iter().map(|affected_node| (affected_node.node_common.name.clone(), stateless_modifier.run(
                        clause_context,
                        &function_context.with_node(affected_node.clone()),
                        &new_state,
                    ))).collect::<Vec<_>>().into_iter()
                }).collect())
            ).collect()
    }

    /// returns all modifier states in an uninitialised form
    fn get_initial_affected_node_states(&self) -> HashMap<SmolStr, Option<bool>> {
        self.affector_nodes_and_affected_nodes.iter()
            .flat_map(|(_, affected_modifiers)| affected_modifiers.values())
            .flat_map(|x| x.iter())
            .map(|x| (x.node_common.name.clone(), None))
            .collect()
    }

    /// checks whether stateful affectors are present
    fn has_stateful_affectors(&self) -> bool {
        !self.stateful_modifier_names.is_empty()
    }

    /// checks whether node is a stateful affector
    fn is_a_stateful_affector(&self, node: &NodeParams) -> bool {
        if !self.affector_nodes_and_affected_nodes.contains_key(node) {
            false
        } else {
            if let Some(ref call_modifier_name) = node.node_common.affects_call_modifier_name {
                self.stateful_modifier_names.contains(call_modifier_name)
            } else {
                false
            }
        }
    }

    /// checks whether node is a stateful affector
    fn get_affected_nodes(&self, node: &NodeParams) -> &HashMap<SmolStr, Vec<NodeParams>> {
        self.affector_nodes_and_affected_nodes.get(node).unwrap()
    }
}

/// function context: the current node, and the
/// current function arguments (call params)
/// NOTE: Whatever data is added here in the future
/// should be PATH INDEPENDANT
#[derive(Debug, Clone)]
pub struct FunctionContext {
    /// the call params (arguments) of the current function
    pub call_params: CallParams,
    /// the last node that was outputted by the state generator
    pub current_node: NodeParams,
}

impl FunctionContext {
    fn new(call_params: CallParams) -> Self {
        let func_name = call_params.func_name.clone();
        Self {
            call_params,
            current_node: NodeParams {
                node_common: NodeCommon::with_name(func_name),
                call_params: None,
                literal: false,
                min_calls_until_function_exit: 0,
            },
        }
    }

    /// create a new FunctionContext with the provided node_params as the current_node
    fn with_node(&self, node_params: NodeParams) -> Self {
        Self {
            call_params: self.call_params.clone(),
            current_node: node_params,
        }
    }
}

/// This structure stores all the info about
/// call modifiers in the current function
#[derive(Debug)]
struct CallModifierInfo {
    call_modifier_states: CallModifierStates,
    /// the call modifier inner state memory
    values: HashMap<SmolStr, Box<dyn std::any::Any>>,
    /// call modifier configuration: How each STATELESS call modifier
    /// affector node affects every node with a call modifier (STATELESS)
    stateless_node_relations: BTreeMap<SmolStr, BTreeMap<SmolStr, bool>>,
}

impl CallModifierInfo {
    fn new(
        function_modifier_info: &FunctionModifierInfo,
        stateful_call_modifier_creators: &HashMap<SmolStr, StatefulCallModifierCreator>,
        stateless_node_relations: BTreeMap<SmolStr, BTreeMap<SmolStr, bool>>,
    ) -> Self {
        Self {
            call_modifier_states: CallModifierStates::new(function_modifier_info, stateful_call_modifier_creators),
            values: HashMap::new(),
            stateless_node_relations
        }
    }

    fn update_modifier_states(
        &mut self,
        function_context: &FunctionContext,
        function_modifier_info: &FunctionModifierInfo,
        clause_context: &ClauseContext,
        value_setters: &HashMap<SmolStr, Box<dyn ValueSetter>>,
    ) {
        self.call_modifier_states.update_modifier_states(
            function_context,
            function_modifier_info,
            &self.stateless_node_relations,
            clause_context
        );
        if let Some(ref value_name) = function_context.current_node.node_common.sets_value_name {
            self.values.insert(
                value_name.clone(),
                value_setters.get(value_name).unwrap().get_value(clause_context, &function_context)
            );
        }
    }
}

/// Stires the states of all affected nodes and all stateful call modifiers
#[derive(Debug)]
struct CallModifierStates {
    /// stores current node states that are affected by call modifiers
    affected_node_states: HashMap<SmolStr, Option<bool>>,
    /// stores all the stateful modifiers of the current function in
    /// their current state
    stateful_modifiers: HashMap<SmolStr, Box<dyn StatefulCallModifier>>,
}

trait DynClone {
    fn dyn_clone(&self) -> Self;
}

impl DynClone for HashMap<SmolStr, Box<dyn StatefulCallModifier>> {
    fn dyn_clone(&self) -> Self {
        self.iter().map(|x| (x.0.clone(), x.1.dyn_box_clone())).collect()
    }
}

impl DynClone for CallModifierStates {
    fn dyn_clone(&self) -> Self {
        Self {
            affected_node_states: self.affected_node_states.clone(),
            stateful_modifiers: self.stateful_modifiers.dyn_clone(),
        }
    }
}

impl CallModifierStates {
    fn new(
        function_modifier_info: &FunctionModifierInfo,
        stateful_call_modifier_creators: &HashMap<SmolStr, StatefulCallModifierCreator>,
    ) -> Self {
        Self {
            affected_node_states: function_modifier_info.get_initial_affected_node_states(),
            stateful_modifiers: function_modifier_info.create_stateful_modifiers(stateful_call_modifier_creators)
        }
    }

    fn update_modifier_states(
        &mut self,
        function_context: &FunctionContext,
        function_modifier_info: &FunctionModifierInfo,
        stateless_node_relations: &BTreeMap<SmolStr, BTreeMap<SmolStr, bool>>,
        clause_context: &ClauseContext,
    ) {
        let node = &function_context.current_node;
        // update with a cached influence result
        if let Some(affected) = stateless_node_relations.get(&node.node_common.name) {
            self.affected_node_states.extend(
                affected.clone().into_iter().map(|(affected_node_name, how)| (affected_node_name, Some(how)))
            );
        }
        // update stateful affector's states
        if function_modifier_info.is_a_stateful_affector(node) {
            let affected_modifiers = function_modifier_info.get_affected_nodes(node);
            for (modifier_name, affected_nodes) in affected_modifiers.iter() {
                if let Some(modifier) = self.stateful_modifiers.get_mut(modifier_name) {
                    modifier.update_state(clause_context, &function_context.with_node(node.clone()));
                    self.affected_node_states.extend(affected_nodes.into_iter().map(|affected_node| {
                        (affected_node.node_common.name.clone(), Some(modifier.run(
                            clause_context, &function_context.with_node(affected_node.clone())
                        )))
                    }));
                }
            }
        }
    }

    fn has_path_been_affected(&self) -> bool {
        self.affected_node_states.iter().any(|(_, state)| state.is_some())
    }
}

impl<StC: StateChooser> MarkovChainGenerator<StC> {
    pub fn with_config(config: StateGeneratorConfig) -> Result<Self, SyntaxError> {
        let chain = MarkovChain::parse_dot(config.graph_file_path)?;
        let mut _self = MarkovChainGenerator::<StC> {
            markov_chain: chain,
            call_stack: vec![],
            pending_call: None,
            state_chooser: Box::new(StC::new()),
            value_setters: HashMap::new(),
            stateless_call_modifiers: HashMap::new(),
            stateful_call_modifier_creators: HashMap::new(),
            function_modifier_info: HashMap::new(),
            dead_end_infos: HashMap::new(),
        };
        _self.register_value_setter(TypesTypeValueSetter {});
        _self.register_stateless_call_modifier(IsColumnTypeAvailableModifier {});
        _self.fill_function_modifier_info();
        _self.reset();
        Ok(_self)
    }

    fn register_value_setter<T: ValueSetter + 'static>(&mut self, value_setter: T) {
        self.value_setters.insert(value_setter.get_value_name(), Box::new(value_setter));
    }

    fn register_stateless_call_modifier<T: StatelessCallModifier + 'static>(&mut self, modifier: T) {
        self.stateless_call_modifiers.insert(modifier.get_name(), Box::new(modifier));
    }

    fn _register_stateful_call_modifier<T: StatefulCallModifier + 'static>(&mut self) {
        self.stateful_call_modifier_creators.insert(T::new().get_name(), StatefulCallModifierCreator(
            T::new
        ));
    }

    fn fill_function_modifier_info(&mut self) {
        self.function_modifier_info = self.markov_chain.functions.iter().map(|(function_name, function)| {
            (function_name.clone(), FunctionModifierInfo::new(
                &function.node_params,
                &self.stateless_call_modifiers,
                &self.stateful_call_modifier_creators
            ))
        }).collect();
    }

    pub fn get_named_value<T: NamedValue + 'static>(&self) -> Option<&T> {
        self.call_stack.last().unwrap().call_modifier_info.values.get(&T::name()).map(
            |named_value| named_value.downcast_ref::<T>().unwrap()
        )
    }

    /// used to print the call stack of the markov chain functions
    pub fn print_stack(&self) {
        println!("Call stack:");
        for stack_item in &self.call_stack {
            println!("{} ({:?} | {:?}) [{}]:", stack_item.function_context.call_params.func_name, stack_item.function_context.call_params.selected_types, stack_item.function_context.call_params.modifiers, stack_item.function_context.current_node.node_common.name)
        }
    }

    /// used when the markov chain reaches end, for the object to be iterable multiple times
    /// resets the stack to only have the entry function, query, as its pending call.
    pub fn reset(&mut self) {
        let accepted_types = unwrap_variant!(self.markov_chain.functions.get("Query").expect(
            "Graph should have an entry function named Query, with TYPES=[...]"
        ).accepted_types.clone(), FunctionTypes::TypeList);

        self.pending_call = Some(CallParams {
            func_name: SmolStr::new("Query"),
            selected_types: CallTypes::TypeList(accepted_types),  // CallTypes::TypeList(vec![SubgraphType::Numeric]),  // 
            modifiers: CallModifiers::None  // CallModifiers::StaticList(vec![SmolStr::new("single value")])  //
        });
    }

    /// push all the known data to fields of call params that
    /// were previously left unfilled
    fn fill_call_params_fields(&self, mut call_params: CallParams) -> CallParams {
        let called_function = self.markov_chain.functions.get(&call_params.func_name).unwrap();

        call_params.selected_types = match call_params.selected_types {
            CallTypes::KnownList => CallTypes::TypeList(self.get_known_list()),
            CallTypes::Compatible => CallTypes::TypeList(self.get_compatible_list()),
            CallTypes::PassThrough => self.get_fn_selected_types_unwrapped().clone(),
            CallTypes::TypeListWithFields(type_list) => CallTypes::TypeList({
                type_list.into_iter().map(|TypeWithFields::Type(tp)| tp).collect()
            }),
            any => any,
        };

        // if Undetermined is in the parameter list, replace the list with all acceptable arguments.
        match &call_params.selected_types {
            CallTypes::TypeList(type_list) if type_list.contains(&SubgraphType::Undetermined) => {
                call_params.selected_types = CallTypes::TypeList(unwrap_variant!(called_function.accepted_types.clone(), FunctionTypes::TypeList));
            },
            _ => {},
        };

        // finally, deal with modifiers
        call_params.modifiers = match call_params.modifiers {
            CallModifiers::StaticListWithParentMods(modifiers) => {
                CallModifiers::StaticList([
                    &{ match self.get_fn_modifiers().clone() {
                        CallModifiers::None => vec![],
                        CallModifiers::StaticListWithParentMods(_) => panic!(
                            "CallModifiers::StaticListWithParentMods was not substituted with StaticList for parent function"
                        ),
                        CallModifiers::StaticList(list) => list,
                    }}[..],
                    &modifiers[..]
                ].concat())
            },
            any => any,
        };

        call_params
    }

    /// update stack, preparing the stack frame by filling it out with all the
    /// cached data
    fn update_stack(&mut self, call_params: CallParams, clause_context: &ClauseContext) {
        let function_modifier_info = self.function_modifier_info.get(&call_params.func_name).unwrap();
        let function_context = FunctionContext::new(call_params);

        let stateless_node_relations = function_modifier_info.get_stateless_node_relations(
            &function_context, clause_context, &self.value_setters, &self.stateless_call_modifiers
        );

        let dead_end_info = self.dead_end_infos.entry(
            (function_context.call_params.to_owned(), stateless_node_relations.clone())
        ).or_insert(Arc::new(Mutex::new(HashMap::new()))).to_owned();

        self.call_stack.push(StackFrame::new(
            function_context,
            CallModifierInfo::new(
                function_modifier_info,
                &self.stateful_call_modifier_creators, 
                stateless_node_relations
            ),
            dead_end_info,
        ));
    }

    /// update stack and return function's first state
    fn start_function(&mut self, call_params: CallParams, clause_context: &ClauseContext) -> SmolStr {
        let call_params = self.fill_call_params_fields(call_params);

        self.update_stack(call_params, clause_context);

        self.call_stack.last().unwrap().function_context.current_node.node_common.name.clone()
    }

    /// choose a new node among the available destibation nodes with the fynamic model
    fn update_current_node(&mut self, rng: &mut ChaCha8Rng, clause_context: &ClauseContext, dynamic_model: &mut (impl DynamicModel + ?Sized)) {
        dynamic_model.notify_call_stack_length(self.call_stack.len());

        let stack_frame = self.call_stack.last_mut().unwrap();
        // last_node guaranteed not to be an exit node
        let last_node = stack_frame.function_context.current_node.clone();
        let function = self.markov_chain.functions.get(&stack_frame.function_context.call_params.func_name).unwrap();
        let function_modifier_info = self.function_modifier_info.get(
            &stack_frame.function_context.call_params.func_name
        ).unwrap();

        let last_node_outgoing = function.chain.get(&last_node.node_common.name).unwrap();

        let last_node_outgoing = last_node_outgoing.iter().map(|el| {
            (check_node_off_dfs(rng, function, function_modifier_info, clause_context, stack_frame, &el.1), el.0, el.1.clone())
        }).collect::<Vec<_>>();

        dynamic_model.update_current_state(&last_node.node_common.name);
        let last_node_outgoing = dynamic_model.assign_log_probabilities(last_node_outgoing);

        let destination = self.state_chooser.choose_destination(rng, last_node_outgoing);

        if let Some(destination) = destination {
            if check_node_off_dfs(rng, function, function_modifier_info, clause_context, stack_frame, &destination) {
                panic!("Chosen node is off: {} (after {})", destination.node_common.name, last_node.node_common.name);
            }
            stack_frame.function_context.current_node = destination.clone();
        } else {
            let function_context = stack_frame.function_context.clone();
            self.print_stack();
            panic!("No destination found for {} in {:#?}.", last_node.node_common.name, function_context);
        }
    }

    /// get current function inputs list
    pub fn get_fn_selected_types_unwrapped(&self) -> CallTypes {
        let function_context = &self.call_stack.last().unwrap().function_context;
        let selected_types = function_context.call_params.selected_types.clone();
        selected_types
    }

    pub fn get_current_state_type_name_unwrapped(&self) -> SubgraphType {
        let function_context = &self.call_stack.last().unwrap().function_context;
        let call_node_type_name = function_context.current_node.node_common.type_name.as_ref().unwrap().clone();
        call_node_type_name
    }

    /// get crrent function modifiers list
    pub fn get_fn_modifiers(&self) -> &CallModifiers {
       &self.call_stack.last().unwrap().function_context.call_params.modifiers
    }

    pub fn get_pending_call_accepted_types(&self) -> FunctionTypes {
        self.markov_chain.functions.get(&self.pending_call.as_ref().unwrap().func_name).unwrap().accepted_types.clone()
    }

    pub fn get_pending_call_accepted_modifiers(&self) -> Option<Vec<SmolStr>> {
        self.markov_chain.functions.get(&self.pending_call.as_ref().unwrap().func_name).unwrap().accepted_modifiers.clone()
    }

    /// push the known type list for the next node that will use the types=[known]
    /// these will be wrapped automatically if uses_wrapped_types=true
    pub fn set_known_list(&mut self, type_list: Vec<SubgraphType>) {
        self.call_stack.last_mut().unwrap().known_type_list = Some(type_list);
    }

    /// push the compatible type list for the next node that will use the type=[compatible]
    /// these will be wrapped automatically if uses_wrapped_types=true
    pub fn set_compatible_list(&mut self, type_list: Vec<SubgraphType>) {
        self.call_stack.last_mut().unwrap().compatible_type_list = Some(type_list);
    }

    /// pop the known types for the current node which will uses type=[known]
    fn get_known_list(&self) -> Vec<SubgraphType> {
        self.call_stack.last().unwrap().known_type_list.clone().unwrap_or_else(|| {
            self.print_stack();
            panic!("No known type list found!")
        })
    }

    /// pop the compatible types for the current node which will uses type=[compatible]
    fn get_compatible_list(&self) -> Vec<SubgraphType> {
        self.call_stack.last().unwrap().compatible_type_list.clone().unwrap_or_else(|| {
            self.print_stack();
            panic!("No compatible type list found!")
        })
    }
}

impl<StC: StateChooser> MarkovChainGenerator<StC> {
    pub fn next(&mut self, rng: &mut ChaCha8Rng, clause_context: &ClauseContext, dynamic_model: &mut (impl DynamicModel + ?Sized)) -> Option<<Self as Iterator>::Item> {
        if let Some(call_params) = self.pending_call.take() {
            return Some(self.start_function(call_params, clause_context));
        }

        if self.call_stack.is_empty() {
            self.reset();
            return None
        }

        let (is_an_exit, new_node_name) = {
            self.update_current_node(rng, clause_context, dynamic_model);

            let stack_frame = self.call_stack.last_mut().unwrap();

            let function_modifier_info = self.function_modifier_info.get(&stack_frame.function_context.call_params.func_name).unwrap();
            stack_frame.call_modifier_info.update_modifier_states(
                &stack_frame.function_context, function_modifier_info, clause_context, &self.value_setters
            );

            self.pending_call = stack_frame.function_context.current_node.call_params.clone();

            let new_node_name = stack_frame.function_context.current_node.node_common.name.clone();
            let function = self.markov_chain.functions.get(&stack_frame.function_context.call_params.func_name).unwrap();
            (new_node_name == function.exit_node_name, new_node_name)
        };

        if is_an_exit {
            self.call_stack.pop();
        }

        Some(new_node_name)
    }
}

fn check_node_off_dfs(
        rng: &mut ChaCha8Rng,
        function: &Function,
        function_modifier_info: &FunctionModifierInfo,
        clause_context: &ClauseContext,
        stack_frame: &StackFrame,
        node_params: &NodeParams
    ) -> bool {
    if check_node_off(&stack_frame.function_context.call_params, &stack_frame.call_modifier_info.call_modifier_states.affected_node_states, &node_params.node_common) {
        return true
    }

    let mut dead_end_info = stack_frame.dead_end_info.lock().unwrap();
    if let Some(is_dead_end) = dead_end_info.get(&node_params.node_common.name) {
        return *is_dead_end;
    }

    // only update dead_end_info for paths that were not affected yet
    let path_not_yet_affected = !stack_frame.call_modifier_info.call_modifier_states.has_path_been_affected();
    let cycles_can_unlock_paths = function_modifier_info.has_stateful_affectors();

    let mut visited = HashSet::new();
    let mut cacheable_as_dead_ends = HashSet::new();
    let mut stack = VecDeque::new();
    stack.push_back((
        vec![node_params.clone()],
        stack_frame.call_modifier_info.call_modifier_states.dyn_clone(),
    ));

    while let Some((path, mut call_modifier_states)) = stack.pop_back() {
        let current_node = path.last().unwrap().clone();
        let current_node_name = current_node.node_common.name.clone();

        if cycles_can_unlock_paths || visited.insert(current_node_name.clone()) {
            let cacheable_path_part: Vec<SmolStr> = if path_not_yet_affected && path.iter().all(
                    |x| !function_modifier_info.is_a_stateful_affector(x)
                ) {
                path.iter().take_until(
                    |&x| stack_frame.call_modifier_info.stateless_node_relations.contains_key(&x.node_common.name)
                ).map(|x| x.node_common.name.clone()).collect()
            } else {
                vec![]
            };

            if let Some(is_dead_end) = dead_end_info.get(&current_node_name) {
                if !*is_dead_end {
                    dead_end_info.extend(cacheable_path_part.into_iter().map(|x| (x, false)));
                    return false;
                }
            }

            if current_node_name == function.exit_node_name {
                dead_end_info.extend(cacheable_path_part.into_iter().map(|x| (x, false)));
                return false;
            }

            call_modifier_states.update_modifier_states(
                &stack_frame.function_context.with_node(current_node.clone()),
                function_modifier_info,
                &stack_frame.call_modifier_info.stateless_node_relations,
                clause_context
            );
    
            cacheable_as_dead_ends.extend(cacheable_path_part.into_iter());

            if let Some(node_outgoing) = function.chain.get(&current_node_name) {
                let mut outgoing: Vec<_> = node_outgoing.iter()
                    .map(|(_, node)| node.clone())
                    .filter(|x| !check_node_off(
                        &stack_frame.function_context.call_params, &call_modifier_states.affected_node_states, &x.node_common
                    )).collect();
                if cycles_can_unlock_paths {
                    outgoing.shuffle(rng);
                } // prevent DFS looping
                stack.extend(outgoing.into_iter()
                    .map(|x| (
                        [&path[..], &[x]].concat(),
                        call_modifier_states.dyn_clone(),
                    ))
                );
            }
        }
    }
    dead_end_info.extend(cacheable_as_dead_ends.into_iter().map(|x| (x, true)));
    true
}

fn check_node_off(
        call_params: &CallParams,
        affected_node_states: &HashMap<SmolStr, Option<bool>>,
        node_common: &NodeCommon
    ) -> bool {
    let mut off = false;
    if let Some(ref type_name) = node_common.type_name {
        off = match call_params.selected_types {
            CallTypes::None => true,
            CallTypes::TypeList(ref t_name_list) => !t_name_list.iter()
                .any(|x| x.is_same_or_more_determined_or_undetermined(type_name)),
            _ => panic!("Expected None or TypeNameList for function selected types")
        };
    }
    if let Some((ref modifier_name, ref modifier_on)) = node_common.modifier {
        off = off || match call_params.modifiers {
            CallModifiers::None => *modifier_on,
            CallModifiers::StaticListWithParentMods(..) => panic!("CallModifiers::StaticListWithParentArgs was not substituted for CallModifiers::StaticList!"),
            CallModifiers::StaticList(ref modifiers) => {
                if modifiers.contains(&modifier_name) { !modifier_on } else { *modifier_on }
            },
        };
    }
    if let Some(ref modifier_name) = node_common.call_modifier_name {
        off = off || !affected_node_states.get(&node_common.name).unwrap().unwrap_or_else(
            || panic!("State of call modifier {modifier_name} was not set (node {})", node_common.name)
        )
    }
    return off;
}

impl<StC: StateChooser> Iterator for MarkovChainGenerator<StC> {
    type Item = SmolStr;

    fn next(&mut self) -> Option<Self::Item> {
        self.next(&mut ChaCha8Rng::seed_from_u64(1), &mut ClauseContext::new(), &mut MarkovModel::new())
    }
}