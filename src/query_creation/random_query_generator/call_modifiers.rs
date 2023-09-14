use smol_str::SmolStr;

use core::fmt::Debug;

use crate::{query_creation::state_generators::{CallTypes, markov_chain_generator::{FunctionContext, subgraph_type::SubgraphType}}, unwrap_variant};

use super::query_info::ClauseContext;

pub trait ValueSetter: Debug {
    /// returns value name
    fn get_value_name(&self) -> SmolStr;

    /// get the value in given context
    fn get_value(&self, clause_context: &ClauseContext, function_context: &FunctionContext) -> Box<dyn std::any::Any>;
}

pub trait NamedValue: Debug {
    fn name() -> SmolStr where Self : Sized;
}

#[derive(Debug, Clone)]
pub struct TypesTypeValueSetter { }

#[derive(Debug, Clone)]
pub struct TypesTypeValue {
    pub selected_types: Vec<SubgraphType>,
}

impl NamedValue for TypesTypeValue {
    fn name() -> SmolStr {
        SmolStr::new("types_type")
    }
}

impl ValueSetter for TypesTypeValueSetter {
    fn get_value_name(&self) -> SmolStr {
        TypesTypeValue::name()
    }

    fn get_value(&self, _clause_context: &ClauseContext, function_context: &FunctionContext) -> Box<dyn std::any::Any> {
        let selected_type = match function_context.current_node.node_common.name.as_str() {
            "types_select_type_3vl" => SubgraphType::Val3,
            "types_select_type_numeric" => SubgraphType::Numeric,
            "types_select_type_text" => SubgraphType::Text,
            "types_select_type_date" => SubgraphType::Date,
            any => panic!("{any} unexpectedly triggered the is_column_type_available call modifier affector"),
        };
        let allowed_type_list = match selected_type {
            with_inner @ SubgraphType::ListExpr(..) => {
                let argument_selected_types = unwrap_variant!(function_context.call_params.selected_types.clone(), CallTypes::TypeList);
                argument_selected_types
                    .iter()
                    .map(|x| x.to_owned())
                    .filter(|x| std::mem::discriminant(x) == std::mem::discriminant(&with_inner))
                    .collect::<Vec<_>>()
            }
            any => vec![any]
        };

        Box::new(TypesTypeValue {
            selected_types: allowed_type_list
        })
    }
}

/// Call modifier that relies on external (static in function scope) values
pub trait StatelessCallModifier: Debug {
    /// returns call modifier name
    fn get_name(&self) -> SmolStr;

    /// returns associated value, which sets the state of this modifier
    fn get_associated_value_name(&self) -> SmolStr;

    /// Runs the modifier value based on the current value
    fn run(&self, clause_context: &ClauseContext, function_context: &FunctionContext, modifier_state: &Box<dyn std::any::Any>) -> bool;
}

pub trait StatefulCallModifier: Debug {
    fn new() -> Box<dyn StatefulCallModifier> where Self : Sized;

    fn dyn_box_clone(&self) -> Box<dyn StatefulCallModifier>;

    fn get_name(&self) -> SmolStr;

    fn update_state(&mut self, clause_context: &ClauseContext, function_context: &FunctionContext);

    fn run(&self, clause_context: &ClauseContext, function_context: &FunctionContext) -> bool;
}

#[derive(Debug, Clone)]
pub struct IsColumnTypeAvailableModifier {}

impl StatelessCallModifier for IsColumnTypeAvailableModifier {
    fn get_name(&self) -> SmolStr {
        SmolStr::new("is_column_type_available")
    }

    fn get_associated_value_name(&self) -> SmolStr {
        TypesTypeValue::name()
    }

    fn run(&self, clause_context: &ClauseContext, function_context: &FunctionContext, modifier_state: &Box<dyn std::any::Any>) -> bool {
        let check_group_by = match function_context.current_node.node_common.name.as_str() {
            "call0_column_spec" => false,
            "call1_column_spec" => true,
            any => panic!("is_column_type_available call trigger unexpectedly called by {any}"),
        };
        modifier_state.downcast_ref::<TypesTypeValue>().unwrap().selected_types.iter()
            .any(|x|
                if check_group_by {
                    clause_context
                        .group_by()
                        .is_type_available(x)
                } else {
                    clause_context
                        .from()
                        .is_type_available(x)
                }
            )
    }
}
