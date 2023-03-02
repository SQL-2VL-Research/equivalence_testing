use core::fmt::Debug;
use rand::{Rng, SeedableRng};

use smol_str::SmolStr;
use sqlparser::ast::{Query};
use rand_chacha::ChaCha8Rng;

use super::markov_chain::NodeParams;

pub trait StateChooser: Debug + Clone {
    fn new() -> Self where Self: Sized;
    fn choose_destination(&mut self, cur_node_outgoing: Vec<(bool, f64, NodeParams)>) -> Option<NodeParams>;
}

#[derive(Debug, Clone)]
pub struct ProbabilisticStateChooser {
    rng: ChaCha8Rng,
}

impl StateChooser for ProbabilisticStateChooser {
    fn new() -> Self {
        Self { rng: ChaCha8Rng::seed_from_u64(1), }
    }

    fn choose_destination(&mut self, cur_node_outgoing: Vec<(bool, f64, NodeParams)>) -> Option<NodeParams> {
        let cur_node_outgoing: Vec<(f64, NodeParams)> = {
            let cur_node_outgoing = cur_node_outgoing.iter().map(|el| {
                (if el.0 { 0f64 } else { el.1 }, el.2.clone())
            }).collect::<Vec<_>>();
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
        destination
    }
}

#[derive(Debug, Clone)]
pub struct DeterministicStateChooser {
    state_list: Vec<SmolStr>,
    state_index: usize,
}

impl DeterministicStateChooser {
    fn _from_query(_query: Query) -> Self where Self: Sized {
        let _self = Self {
            state_list: vec![],
            state_index: 0
        };
        // _self.process_query(query);
        _self
    }

    fn _push_state(&mut self, state_name: &str) {
        self.state_list.push(SmolStr::new(state_name));
    }

    // /// subgraph def_Query
    // fn process_query(&mut self, query: Query) {
    //     self.push_state("Query");
    //     let mut select_limit = Option::<Expr>::None;
    //     if let Some(mods) = self.state_generator.get_modifiers() {
    //         if mods.contains(&SmolStr::new("single value")) {
    //             select_limit = Some(Expr::Value(Value::Number("1".to_string(), false)));
    //             // TODO: Not only limits can enforce this
    //         } else {
    //             panic!("Unexpected mods (Query): {:?}", mods);
    //         }
    //     }
    //     if let Some(Expr::Value(Value::Number(select_limit, _))) = query.limit {
            
    //     }
    //     if let FunctionInputsType::TypeName(type_name) = self.state_generator.get_inputs() {
    //         println!("TODO: Enforce single column & column type (Query): {type_name}")
    //     }
    //     let mut select_body = Select {
    //         distinct: false,
    //         top: None,
    //         projection: vec![],
    //         into: None,
    //         from: vec![],
    //         lateral_views: vec![],
    //         selection: None,
    //         group_by: vec![],
    //         cluster_by: vec![],
    //         distribute_by: vec![],
    //         sort_by: vec![],
    //         having: None,
    //         qualify: None,
    //     };
    //     self.push_state("FROM");

    //     loop {
    //         select_body.from.push(TableWithJoins { relation: match self.next_state().as_str() {
    //             "Table" => TableFactor::Table {
    //                 name: self.current_query_rm.new_relation().gen_object_name(),
    //                 alias: None,
    //                 args: None,
    //                 with_hints: vec![]
    //             },
    //             "call0_Query" => TableFactor::Derived {
    //                 lateral: false,
    //                 subquery: Box::new(self.process_query()),
    //                 alias: Some(TableAlias {
    //                     name: self.current_query_rm.new_ident(),
    //                     columns: vec![],
    //                 })
    //             },
    //             "EXIT_FROM" => break,
    //             any => self.panic_unexpected(any)
    //         }, joins: vec![] });
    //         self.push_state("FROM_multiple_relations");
    //     }

    //     match self.next_state().as_str() {
    //         "WHERE" => {
    //             self.push_state("call0_VAL_3");
    //             select_body.selection = Some(self.process_val_3());
    //             self.push_state("EXIT_WHERE");
    //         },
    //         "EXIT_WHERE" => {},
    //         any => self.panic_unexpected(any)
    //     }

    //     self.push_state("SELECT");
    //     select_body.distinct = match self.next_state().as_str() {
    //         "SELECT_DISTINCT" => {
    //             self.push_state("SELECT_distinct_end");
    //             true
    //         },
    //         "SELECT_distinct_end" => false,
    //         any => self.panic_unexpected(any)
    //     };

    //     self.push_state("SELECT_projection");
    //     while match self.next_state().as_str() {
    //         "SELECT_list" => true,
    //         "EXIT_SELECT" => false,
    //         any => self.panic_unexpected(any)
    //     } {
    //         match self.next_state().as_str() {
    //             "SELECT_wildcard" => select_body.projection.push(SelectItem::Wildcard),
    //             "SELECT_qualified_wildcard" => {
    //                 select_body.projection.push(SelectItem::QualifiedWildcard(ObjectName(vec![
    //                     self.current_query_rm.get_random_relation().gen_ident()
    //                 ])));
    //             },
    //             arm @ ("SELECT_unnamed_expr" | "SELECT_expr_with_alias") => {
    //                 self.push_state("call7_types_all");
    //                 let expr = self.process_types_all().1;
    //                 select_body.projection.push(match arm {
    //                     "SELECT_unnamed_expr" => SelectItem::UnnamedExpr(expr),
    //                     "SELECT_expr_with_alias" => SelectItem::ExprWithAlias {
    //                         expr, alias: self.gen_select_alias(),
    //                     },
    //                     any => self.panic_unexpected(any)
    //                 });
    //             },
    //             any => self.panic_unexpected(any)
    //         };
    //         self.push_state("SELECT_list_multiple_values");
    //     }

    //     self.push_state("EXIT_Query");
    //     self.dynamic_model.notify_subquery_creation_end();
    //     Query {
    //         with: None,
    //         body: SetExpr::Select(Box::new(select_body)),
    //         order_by: vec![],
    //         limit: select_limit,
    //         offset: None,
    //         fetch: None,
    //         lock: None,
    //     }
    // }

    // /// subgraph def_VAL_3
    // fn process_val_3(&mut self) -> Expr {
    //     self.push_state("VAL_3");
    //     let val3 = match self.next_state().as_str() {
    //         "IsNull" => {
    //             let is_null_not_flag = match self.next_state().as_str() {
    //                 "IsNull_not" => {
    //                     self.push_state("call0_types_all");
    //                     true
    //                 }
    //                 "call0_types_all" => false,
    //                 any => self.panic_unexpected(any)
    //             };
    //             let types_value = Box::new(self.process_types_all().1);
    //             if is_null_not_flag {
    //                 Expr::IsNotNull(types_value)
    //             } else {
    //                 Expr::IsNull(types_value)
    //             }
    //         },
    //         "IsDistinctFrom" => {
    //             self.push_state("call1_types_all");
    //             let (types_selected_type, types_value_1) = self.process_types_all();
    //             self.state_generator.push_compatible(types_selected_type.get_compat_types());
    //             let is_distinct_not_flag = match self.next_state().as_str() {
    //                 "IsDistinctNOT" => {
    //                     self.push_state("DISTINCT");
    //                     true
    //                 }
    //                 "DISTINCT" => false,
    //                 any => self.panic_unexpected(any)
    //             };
    //             self.push_state("call21_types");
    //             let types_value_2 = self.process_types(None, Some(types_selected_type)).1;
    //             if is_distinct_not_flag {
    //                 Expr::IsNotDistinctFrom(Box::new(types_value_1), Box::new(types_value_2))
    //             } else {
    //                 Expr::IsDistinctFrom(Box::new(types_value_1), Box::new(types_value_2))
    //             }
    //         },
    //         "Exists" => {
    //             let exists_not_flag = match self.next_state().as_str() {
    //                 "Exists_not" => {
    //                     self.push_state("call2_Query");
    //                     true
    //                 },
    //                 "call2_Query" => false,
    //                 any => self.panic_unexpected(any)
    //             };
    //             Expr::Exists {
    //                 subquery: Box::new(self.process_query()),
    //                 negated: exists_not_flag
    //             }
    //         },
    //         "InList" => {
    //             self.push_state("call2_types_all");
    //             let (types_selected_type, types_value) = self.process_types_all();
    //             self.state_generator.push_compatible(types_selected_type.get_compat_types());
    //             let in_list_not_flag = match self.next_state().as_str() {
    //                 "InListNot" => {
    //                     self.push_state("InListIn");
    //                     true
    //                 },
    //                 "InListIn" => false,
    //                 any => self.panic_unexpected(any)
    //             };
    //             self.push_state("call1_list_expr");
    //             Expr::InList {
    //                 expr: Box::new(types_value),
    //                 list: unwrap_variant!(self.process_list_expr(), Expr::Tuple),
    //                 negated: in_list_not_flag
    //             }
    //         },
    //         "InSubquery" => {
    //             self.push_state("call3_types_all");
    //             let (types_selected_type, types_value) = self.process_types_all();
    //             self.state_generator.push_compatible(types_selected_type.get_compat_types());
    //             let in_subquery_not_flag = match self.next_state().as_str() {
    //                 "InSubqueryNot" => {
    //                     self.push_state("InSubqueryIn");
    //                     true
    //                 },
    //                 "InSubqueryIn" => false,
    //                 any => self.panic_unexpected(any)
    //             };
    //             self.push_state("call3_Query");
    //             let query = self.process_query();
    //             Expr::InSubquery {
    //                 expr: Box::new(types_value),
    //                 subquery: Box::new(query),
    //                 negated: in_subquery_not_flag
    //             }
    //         },
    //         "Between" => {
    //             self.push_state("call4_types_all");
    //             let (types_selected_type, types_value_1) = self.process_types_all();
    //             let type_names = types_selected_type.get_compat_types();
    //             let between_not_flag = match self.next_state().as_str() {
    //                 "BetweenBetweenNot" => {
    //                     self.push_state("BetweenBetween");
    //                     true
    //                 },
    //                 "BetweenBetween" => false,
    //                 any => self.panic_unexpected(any)
    //             };
    //             self.state_generator.push_compatible(type_names.clone());
    //             self.push_state("call22_types");
    //             let types_value_2 = self.process_types(None, Some(types_selected_type.clone())).1;
    //             self.push_state("BetweenBetweenAnd");
    //             self.state_generator.push_compatible(type_names);
    //             self.push_state("call23_types");
    //             let types_value_3 = self.process_types(None, Some(types_selected_type)).1;
    //             Expr::Between {
    //                 expr: Box::new(types_value_1),
    //                 negated: between_not_flag,
    //                 low: Box::new(types_value_2),
    //                 high: Box::new(types_value_3)
    //             }
    //         },
    //         "BinaryComp" => {
    //             self.push_state("call5_types_all");
    //             let (types_selected_type, types_value_1) = self.process_types_all();
    //             self.state_generator.push_compatible(types_selected_type.get_compat_types());
    //             let binary_comp_op = match self.next_state().as_str() {
    //                 "BinaryCompEqual" => BinaryOperator::Eq,
    //                 "BinaryCompLess" => BinaryOperator::Lt,
    //                 "BinaryCompLessEqual" => BinaryOperator::LtEq,
    //                 "BinaryCompUnEqual" => BinaryOperator::NotEq,
    //                 any => self.panic_unexpected(any)
    //             };
    //             self.push_state("call24_types");
    //             let types_value_2 = self.process_types(None, Some(types_selected_type)).1;
    //             Expr::BinaryOp {
    //                 left: Box::new(types_value_1),
    //                 op: binary_comp_op,
    //                 right: Box::new(types_value_2)
    //             }
    //         },
    //         "AnyAll" => {
    //             self.push_state("call6_types_all");
    //             let (types_selected_type, types_value) = self.process_types_all();
    //             self.push_state("AnyAllSelectOp");
    //             let any_all_op = match self.next_state().as_str() {
    //                 "AnyAllEqual" => BinaryOperator::Eq,
    //                 "AnyAllLess" => BinaryOperator::Lt,
    //                 "AnyAllLessEqual" => BinaryOperator::LtEq,
    //                 "AnyAllUnEqual" => BinaryOperator::NotEq,
    //                 any => self.panic_unexpected(any)
    //             };
    //             self.push_state("AnyAllSelectIter");
    //             self.state_generator.push_compatible(types_selected_type.get_compat_types());
    //             let iterable = Box::new(match self.next_state().as_str() {
    //                 "call4_Query" => Expr::Subquery(Box::new(self.process_query())),
    //                 "call1_array" => self.process_array(),
    //                 any => self.panic_unexpected(any)
    //             });
    //             self.push_state("AnyAllAnyAll");
    //             let iterable = Box::new(match self.next_state().as_str() {
    //                 "AnyAllAnyAllAll" => Expr::AllOp(iterable),
    //                 "AnyAllAnyAllAny" => Expr::AnyOp(iterable),
    //                 any => self.panic_unexpected(any),
    //             });
    //             Expr::BinaryOp {
    //                 left: Box::new(types_value),
    //                 op: any_all_op,
    //                 right: iterable,
    //             }
    //         },
    //         "BinaryStringLike" => {
    //             self.push_state("call25_types");
    //             let types_value_1 = self.process_types(Some(TypesSelectedType::String), None).1;
    //             let string_like_not_flag = match self.next_state().as_str() {
    //                 "BinaryStringLikeNot" => {
    //                     self.push_state("BinaryStringLikeIn");
    //                     true
    //                 }
    //                 "BinaryStringLikeIn" => false,
    //                 any => self.panic_unexpected(any)
    //             };
    //             self.push_state("call26_types");
    //             let types_value_2 = self.process_types(Some(TypesSelectedType::String), None).1;
    //             Expr::BinaryOp {
    //             left: Box::new(types_value_1),
    //                 op: if string_like_not_flag { BinaryOperator::NotLike } else { BinaryOperator::Like },
    //                 right: Box::new(types_value_2)
    //             }
    //         },
    //         "BinaryBooleanOpV3" => {
    //             self.push_state("call27_types");
    //             let types_value_1 = self.process_types(Some(TypesSelectedType::Val3), None).1;
    //             let binary_bool_op = match self.next_state().as_str() {
    //                 "BinaryBooleanOpV3AND" => BinaryOperator::And,
    //                 "BinaryBooleanOpV3OR" => BinaryOperator::Or,
    //                 "BinaryBooleanOpV3XOR" => BinaryOperator::Xor,
    //                 any => self.panic_unexpected(any)
    //             };
    //             self.push_state("call28_types");
    //             let types_value_2 = self.process_types(Some(TypesSelectedType::Val3), None).1;
    //             Expr::BinaryOp {
    //                 left: Box::new(types_value_1),
    //                 op: binary_bool_op,
    //                 right: Box::new(types_value_2)
    //             }
    //         },
    //         "true" => Expr::Value(Value::Boolean(true)),
    //         "false" => Expr::Value(Value::Boolean(false)),
    //         "Nested_VAL_3" => {
    //             self.push_state("call29_types");
    //             Expr::Nested(Box::new(self.process_types(Some(TypesSelectedType::Val3), None).1))
    //         },
    //         "UnaryNot_VAL_3" => {
    //             self.push_state("call30_types");
    //             Expr::UnaryOp { op: UnaryOperator::Not, expr: Box::new( self.process_types(
    //                 Some(TypesSelectedType::Val3), None
    //             ).1) }
    //         },
    //         any => self.panic_unexpected(any)
    //     };
    //     self.push_state("EXIT_VAL_3");
    //     val3
    // }

    // /// subgraph def_numeric
    // fn process_numeric(&mut self) -> Expr {
    //     self.push_state("numeric");
    //     let numeric = match self.next_state().as_str() {
    //         "numeric_literal" => {
    //             Expr::Value(Value::Number(match self.next_state().as_str() {
    //                 "numeric_literal_float" => {
    //                     "3.1415"  // TODO: hardcode
    //                 },
    //                 "numeric_literal_int" => {
    //                     "3"       // TODO: hardcode
    //                 },
    //                 any => self.panic_unexpected(any)
    //             }.to_string(), false))
    //         },
    //         "BinaryNumericOp" => {
    //             self.push_state("call48_types");
    //             let types_value_1 = self.process_types(Some(TypesSelectedType::Numeric), None).1;
    //             let numeric_binary_op = match self.next_state().as_str() {
    //                 "binary_numeric_bin_and" => BinaryOperator::BitwiseAnd,
    //                 "binary_numeric_bin_or" => BinaryOperator::BitwiseOr,
    //                 "binary_numeric_bin_xor" => BinaryOperator::BitwiseXor,
    //                 "binary_numeric_div" => BinaryOperator::Divide,
    //                 "binary_numeric_minus" => BinaryOperator::Minus,
    //                 "binary_numeric_mul" => BinaryOperator::Multiply,
    //                 "binary_numeric_plus" => BinaryOperator::Plus,
    //                 any => self.panic_unexpected(any),
    //             };
    //             self.push_state("call47_types");
    //             let types_value_2 = self.process_types(Some(TypesSelectedType::Numeric), None).1;
    //             Expr::BinaryOp {
    //                 left: Box::new(types_value_1),
    //                 op: numeric_binary_op,
    //                 right: Box::new(types_value_2)
    //             }
    //         },
    //         "UnaryNumericOp" => {
    //             let numeric_unary_op = match self.next_state().as_str() {
    //                 "unary_numeric_abs" => UnaryOperator::PGAbs,
    //                 "unary_numeric_bin_not" => UnaryOperator::PGBitwiseNot,
    //                 "unary_numeric_cub_root" => UnaryOperator::PGCubeRoot,
    //                 "unary_numeric_minus" => UnaryOperator::Minus,
    //                 "unary_numeric_plus" => UnaryOperator::Plus,
    //                 "unary_numeric_postfix_fact" => UnaryOperator::PGPostfixFactorial,
    //                 "unary_numeric_prefix_fact" => UnaryOperator::PGPrefixFactorial,
    //                 "unary_numeric_sq_root" => UnaryOperator::PGSquareRoot,
    //                 any => self.panic_unexpected(any),
    //             };
    //             self.push_state("call1_types");
    //             let types_value = self.process_types(Some(TypesSelectedType::Numeric), None).1;
    //             Expr::UnaryOp {
    //                 op: numeric_unary_op,
    //                 expr: Box::new(types_value)
    //             }
    //         },
    //         "numeric_string_Position" => {
    //             self.push_state("call2_types");
    //             let types_value_1 = self.process_types(Some(TypesSelectedType::String), None).1;
    //             self.push_state("string_position_in");
    //             self.push_state("call3_types");
    //             let types_value_2 = self.process_types(Some(TypesSelectedType::String), None).1;
    //             Expr::Position {
    //                 expr: Box::new(types_value_1),
    //                 r#in: Box::new(types_value_2)
    //             }
    //         },
    //         "Nested_numeric" => {
    //             self.push_state("call4_types");
    //             let types_value = self.process_types(Some(TypesSelectedType::Numeric), None).1;
    //             Expr::Nested(Box::new(types_value))
    //         },
    //         any => self.panic_unexpected(any)
    //     };
    //     self.push_state("EXIT_numeric");
    //     numeric
    // }

    // /// subgraph def_string
    // fn process_string(&mut self) -> Expr {
    //     self.push_state("string");
    //     let string = match self.next_state().as_str() {
    //         "string_literal" => Expr::Value(Value::SingleQuotedString("HJeihfbwei".to_string())),  // TODO: hardcoded
    //         "string_trim" => {
    //             let trim_where = match self.next_state().as_str() {
    //                 "call6_types" => {
    //                     let types_value = self.process_types(Some(TypesSelectedType::String), None).1;
    //                     let spec_mode = match self.next_state().as_str() {
    //                         "BOTH" => TrimWhereField::Both,
    //                         "LEADING" => TrimWhereField::Leading,
    //                         "TRAILING" => TrimWhereField::Trailing,
    //                         any => self.panic_unexpected(any)
    //                     };
    //                     self.push_state("call5_types");
    //                     Some((spec_mode, Box::new(types_value)))
    //                 },
    //                 "call5_types" => None,
    //                 any => self.panic_unexpected(any)
    //             };
    //             let types_value = self.process_types(Some(TypesSelectedType::String), None).1;
    //             Expr::Trim {
    //                 expr: Box::new(types_value), trim_where
    //             }
    //         },
    //         "string_concat" => {
    //             self.push_state("call7_types");
    //             let types_value_1 = self.process_types(Some(TypesSelectedType::String), None).1;
    //             self.push_state("string_concat_concat");
    //             self.push_state("call8_types");
    //             let types_value_2 = self.process_types(Some(TypesSelectedType::String), None).1;
    //             Expr::BinaryOp {
    //                 left: Box::new(types_value_1),
    //                 op: BinaryOperator::StringConcat,
    //                 right: Box::new(types_value_2)
    //             }
    //         },
    //         any => self.panic_unexpected(any)
    //     };
    //     self.push_state("EXIT_string");
    //     string
    // }

    // /// subgraph def_types
    // fn process_types(
    //     &mut self, equal_to: Option<TypesSelectedType>,
    //     compatible_with: Option<TypesSelectedType>
    // ) -> (TypesSelectedType, Expr) {
    //     self.push_state("types");
    //     let (types_selected_type, types_value) = match self.next_state().as_str() {
    //         "types_select_type" => {
    //             let types_selected_type = match self.next_state().as_str() {
    //                 "types_select_type_3vl" => TypesSelectedType::Val3,
    //                 "types_select_type_array" => TypesSelectedType::Array,
    //                 "types_select_type_list_expr" => TypesSelectedType::ListExpr,
    //                 "types_select_type_numeric" => TypesSelectedType::Numeric,
    //                 "types_select_type_string" => TypesSelectedType::String,
    //                 any => self.panic_unexpected(any)
    //             };
    //             self.state_generator.push_known(types_selected_type.get_types());
    //             self.push_state("types_select_type_end");
    //             (types_selected_type, match self.next_state().as_str() {
    //                 "call0_column_spec" => self.process_column_spec(),
    //                 "call1_Query" => Expr::Subquery(Box::new(self.process_query())),
    //                 any => self.panic_unexpected(any)
    //             })
    //         },
    //         "types_null" => (TypesSelectedType::Any, Expr::Value(Value::Null)),
    //         "call0_numeric" => (TypesSelectedType::Numeric, self.process_numeric()),
    //         "call1_VAL_3" => (TypesSelectedType::Val3, self.process_val_3()),
    //         "call0_string" => (TypesSelectedType::String, self.process_string()),
    //         "call0_list_expr" => {
    //             self.state_generator.push_known(match self.state_generator.get_inputs() {
    //                 FunctionInputsType::TypeNameList(list) => list,
    //                 any => panic!("Couldn't pass {:?} to subgraph def_list_expr", any)
    //             });
    //             (TypesSelectedType::ListExpr, self.process_list_expr())
    //         },
    //         "call0_array" => {
    //             self.state_generator.push_known(match self.state_generator.get_inputs() {
    //                 FunctionInputsType::TypeNameList(list) => list,
    //                 any => panic!("Couldn't pass {:?} to subgraph def_array", any)
    //             });
    //             (TypesSelectedType::Array, self.process_array())
    //         },
    //         any => self.panic_unexpected(any)
    //     };
    //     self.push_state("EXIT_types");
    //     if let Some(to) = equal_to {
    //         self.expect_type(&types_selected_type, &to);
    //     }
    //     if let Some(with) = compatible_with {
    //         self.expect_compat(&types_selected_type, &with);
    //     }
    //     (types_selected_type, types_value)
    // }

    // /// subgraph def_types_all
    // fn process_types_all(&mut self) -> (TypesSelectedType, Expr) {
    //     self.push_state("types_all");
    //     self.push_state("call0_types");
    //     let ret = self.process_types(None, None);
    //     self.push_state("EXIT_types_all");
    //     ret
    // }

    // /// subgraph def_column_spec
    // fn process_column_spec(&mut self) -> Expr {
    //     self.push_state("column_spec");
    //     let next_state = self.next_state();
    //     let relation = self.current_query_rm.get_random_relation();
    //     let ret = match next_state.as_str() {
    //         "unqualified_name" => Expr::Identifier(relation.gen_column_ident()),
    //         "qualified_name" => Expr::CompoundIdentifier(vec![
    //             relation.gen_ident(), relation.gen_column_ident()
    //         ]),
    //         any => self.panic_unexpected(any)
    //     };
    //     self.push_state("EXIT_column_spec");
    //     ret
    // }

    // /// subgraph def_array
    // fn process_array(&mut self) -> Expr {
    //     self.push_state("array");
    //     let array_compat_type = match self.next_state().as_str() {
    //         "call12_types" => TypesSelectedType::Numeric,
    //         "call13_types" => TypesSelectedType::Val3,
    //         "call31_types" => TypesSelectedType::String,
    //         "call51_types" => TypesSelectedType::ListExpr,
    //         "call14_types" => TypesSelectedType::Array,
    //         any => self.panic_unexpected(any)
    //     };
    //     let types_value = self.process_types(Some(array_compat_type.clone()), None).1;
    //     let mut array: Vec<Expr> = vec![types_value];
    //     loop {
    //         match self.next_state().as_str() {
    //             "call50_types" => {
    //                 self.state_generator.push_compatible(array_compat_type.get_compat_types());
    //                 let types_value = self.process_types(None, Some(array_compat_type.clone())).1;
    //                 array.push(types_value);
    //             },
    //             "EXIT_array" => break,
    //             any => self.panic_unexpected(any)
    //         }
    //     }
    //     Expr::Array(Array {
    //         elem: array,
    //         named: false
    //     })
    // }

    // /// subgraph def_list_expr
    // fn process_list_expr(&mut self) -> Expr {
    //     self.push_state("list_expr");
    //     let list_compat_type = match self.next_state().as_str() {
    //         "call16_types" => TypesSelectedType::Numeric,
    //         "call17_types" => TypesSelectedType::Val3,
    //         "call18_types" => TypesSelectedType::String,
    //         "call19_types" => TypesSelectedType::ListExpr,
    //         "call20_types" => TypesSelectedType::Array,
    //         any => self.panic_unexpected(any)
    //     };
    //     let types_value = self.process_types(Some(list_compat_type.clone()), None).1;
    //     let mut list_expr: Vec<Expr> = vec![types_value];
    //     loop {
    //         match self.next_state().as_str() {
    //             "call49_types" => {
    //                 self.state_generator.push_compatible(list_compat_type.get_compat_types());
    //                 let types_value = self.process_types(None, Some(list_compat_type.clone())).1;
    //                 list_expr.push(types_value);
    //             },
    //             "EXIT_list_expr" => break,
    //             any => self.panic_unexpected(any)
    //         }
    //     }
    //     Expr::Tuple(list_expr)
    // }
}

impl StateChooser for DeterministicStateChooser {
    fn new() -> Self where Self: Sized {
        Self {
            state_list: vec![],
            state_index: 0
        }
    }

    fn choose_destination(&mut self, cur_node_outgoing: Vec<(bool, f64, NodeParams)>) -> Option<NodeParams> {
        if cur_node_outgoing.len() == 0 {
            println!("List of outgoing nodes is empty!");
            return None
        }
        if cur_node_outgoing.len() == 1 { return Some(cur_node_outgoing[0].2.clone()) }
        let node_name = self.state_list[self.state_index].clone();
        self.state_index += 1;
        match cur_node_outgoing.iter().find(|node| node.2.name == node_name) {
            Some(node) => Some(node.2.clone()),
            None => {
                println!("None of {:?} matches \"{node_name}\" inferred from query", cur_node_outgoing);
                None
            },
        }
    }
}