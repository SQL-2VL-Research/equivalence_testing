use core::fmt::Debug;
use logaddexp::LogSumExp;
use rand::Rng;

use smol_str::SmolStr;
use rand_chacha::ChaCha8Rng;
use sqlparser::ast::Query; // , Expr, Value, SetExpr, TableFactor, SelectItem, BinaryOperator, UnaryOperator, TrimWhereField};

// use super::super::super::random_query_generator::TypesSelectedType;
use super::markov_chain::NodeParams;

pub trait StateChooser: Debug + Clone {
    fn new() -> Self where Self: Sized;
    fn choose_destination(&mut self, rng: &mut ChaCha8Rng, outgoing_states: Vec<(bool, f64, NodeParams)>) -> Option<NodeParams>;
}

#[derive(Debug, Clone)]
pub struct ProbabilisticStateChooser { }

impl StateChooser for ProbabilisticStateChooser {
    fn new() -> Self {
        Self { }
    }

    fn choose_destination(&mut self, rng: &mut ChaCha8Rng, outgoing_states: Vec<(bool, f64, NodeParams)>) -> Option<NodeParams> {
        let cur_node_outgoing: Vec<(f64, NodeParams)> = {
            let cur_node_outgoing = outgoing_states.iter().map(|el| {
                (if el.0 { f64::NEG_INFINITY } else { el.1 }, el.2.clone())
            }).collect::<Vec<_>>();
            // Rebalancing on the current level only as a temporary work-around
            let max_level: f64 = cur_node_outgoing.iter().map(|el| { el.0 }).ln_sum_exp();
            cur_node_outgoing.into_iter().map(|el| { (f64::exp(el.0 - max_level), el.1) }).collect()
        };
        let level: f64 = rng.gen::<f64>();
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

// macro_rules! panic_unexpected_struct {
//     ($el: expr) => {{
//         panic!("AST parsing error: Unexpected element: {:?}", $el);
//     }};
// }

impl DeterministicStateChooser {
    fn _from_query(_query: Query) -> Self where Self: Sized {
        let mut _self = Self {
            state_list: vec![],
            state_index: 0
        };
        // _self.process_query(query);
        _self
    }

//     fn panic_unexpected(self, el_name: String) -> ! {
//         panic!("AST parsing error: Unexpected element: {el_name}");
//     }

//     fn push_state(&mut self, state_name: &str) {
//         // add check for graph state availability
//         self.state_list.push(SmolStr::new(state_name));
//     }

//     fn push_states(&mut self, state_names: Vec<&str>) {
//         for state_name in state_names {
//             self.push_state(state_name);
//         }
//     }

//     /// subgraph def_Query
//     fn process_query(&mut self, query: Query) {
//         self.push_state("Query");

//         match query.limit {
//             Some(limit) => {
//                 if limit == Expr::Value(Value::Number("1".to_string(), false)) {
//                     self.push_state("single_value_true");
//                 } else {
//                     self.push_states(vec!["single_value_false", "call52_types"]);
//                     self.process_types(Some(TypesSelectedType::Numeric), limit);
//                 }
//             },
//             None => self.push_state("single_value_false"),
//         };
//         self.push_state("FROM");

//         let select_body = match *query.body {
//             SetExpr::Select(expr) => *expr,
//             any => panic_unexpected_struct!(any),
//         };

//         for (i, table_with_joins) in select_body.from.iter().enumerate() {
//             match table_with_joins.relation {
//                 TableFactor::Table { .. } => self.push_state("Table"),
//                 TableFactor::Derived { subquery, .. } => {
//                     self.push_state("call0_Query");
//                     self.process_query(*subquery);
//                 },
//                 any => panic_unexpected_struct!(any),
//             }
//             self.push_state("FROM_multiple_relations");
//         }
//         self.push_state("EXIT_FROM");

//         if let Some(where_stmt) = select_body.selection {
//             self.push_states(vec!["WHERE", "call53_types"]);
//             self.process_types(Some(TypesSelectedType::Val3), where_stmt);
//         }
//         self.push_state("EXIT_WHERE");

//         self.push_state("SELECT");
//         if select_body.distinct {
//             self.push_state("SELECT_DISTINCT");
//         }
//         self.push_state("SELECT_distinct_end");

//         self.push_state("SELECT_projection");

//         for (i, select_item) in select_body.projection.into_iter().enumerate() {
//             self.push_state("SELECT_list");
//             match select_item {
//                 SelectItem::Wildcard(..) => self.push_state("SELECT_wildcard"),
//                 SelectItem::QualifiedWildcard(..) => self.push_state("SELECT_qualified_wildcard"),
//                 arm @ (SelectItem::UnnamedExpr(..) | SelectItem::ExprWithAlias { .. }) => {
//                     let (state_name, expr) = match arm {
//                         SelectItem::UnnamedExpr(expr) => ("SELECT_unnamed_expr", expr),
//                         SelectItem::ExprWithAlias{ expr, .. } => ("SELECT_expr_with_alias", expr),
//                         any => panic_unexpected_struct!(any),
//                     };
//                     self.push_state(state_name);
//                     self.push_state("call7_types_all");
//                     self.process_types_all(expr);
//                 },
//                 any => panic_unexpected_struct!(any),
//             }
//             self.push_state("SELECT_list_multiple_values");
//         }
//         self.push_state("EXIT_SELECT");

//         self.push_state("EXIT_Query");
//     }

// // TODO: process_...() returns whether the type was actually the specified one.
//     /// subgraph def_VAL_3
//     fn process_val_3(&mut self, expr: Expr) {
//         self.push_state("VAL_3");
//         match expr {
//             arm @ (Expr::IsNull(expr) | Expr::IsNotNull(expr)) => {
//                 self.push_state("IsNull");
//                 if let Expr::IsNotNull(..) = arm {
//                     self.push_state("IsNull_not");
//                 }
//                 self.push_state("call0_types_all");
//                 self.process_types_all(*expr);
//             },
//             arm @ (Expr::IsDistinctFrom(expr_1, expr_2) | Expr::IsNotDistinctFrom(expr_1, expr_2)) => {
//                 self.push_state("IsDistinctFrom");
//                 self.push_state("call1_types_all");
//                 let selected_type = self.process_types_all(*expr_1);
//                 if let Expr::IsNotDistinctFrom(..) = arm {
//                     self.push_state("IsDistinctNOT");
//                 }
//                 self.push_state("DISTINCT");
//                 self.push_state("call21_types");
//                 self.process_types(Some(selected_type), *expr_2);
//             },
//             Expr::Exists { subquery, negated } => {
//                 self.push_state("Exists");
//                 if negated {
//                     self.push_state("Exists_not");
//                 }
//                 self.push_state("call2_Query");
//                 self.process_query(*subquery);
//             },
//             Expr::InList { expr, list, negated } => {
//                 self.push_states(vec!["InList", "call2_types_all"]);
//                 let selected_type = self.process_types_all(*expr);
//                 if negated {
//                     self.push_state("InListNot");
//                 }
//                 self.push_states(vec!["InListIn", "call1_list_expr"]);
//                 self.process_list_expr(Some(selected_type), list);
//             },
//             Expr::InSubquery { expr, subquery, negated } => {
//                 self.push_states(vec!["InSubquery", "call3_types_all"]);
//                 let _selected_type = self.process_types_all(*expr);
//                 if negated {
//                     self.push_state("InSubqueryNot");
//                 }
//                 self.push_states(vec!["InSubqueryIn", "call3_Query"]);
//                 self.process_query(*subquery);
//             },
//             Expr::Between { expr, negated, low, high  } => {
//                 self.push_states(vec!["Between", "call4_types_all"]);
//                 let selected_type = self.process_types_all(*expr);
//                 if negated {
//                     self.push_state("BetweenBetweenNot");
//                 }
//                 self.push_states(vec!["BetweenBetween", "call22_types"]);
//                 self.process_types(Some(selected_type), *low);
//                 self.push_states(vec!["BetweenBetweenAnd", "call23_types"]);
//                 self.process_types(Some(selected_type), *high);
//             },
//             Expr::BinaryOp {
//                 left, op, right
//             } if matches!(*right, Expr::AllOp(..) | Expr::AnyOp(..)) => {
//                 self.push_states(vec!["AnyAll", "call6_types_all"]);
//                 let selected_type = self.process_types_all(*left);
//                 self.push_states(vec!["AnyAllSelectOp", match op {
//                     BinaryOperator::Eq => "AnyAllEqual",
//                     BinaryOperator::Lt => "AnyAllLess",
//                     BinaryOperator::LtEq => "AnyAllLessEqual",
//                     BinaryOperator::NotEq => "AnyAllUnEqual",
//                     any => panic_unexpected_struct!(any),
//                 }]);
//                 let (any_all_val, iterable) = match *right {
//                     Expr::AllOp(iterable) => ("AnyAllAnyAllAll", iterable),
//                     Expr::AnyOp(iterable) => ("AnyAllAnyAllAny", iterable),
//                     any => panic_unexpected_struct!(any),
//                 };
//                 self.push_state("AnyAllSelectIter");
//                 match *iterable {
//                     Expr::Subquery(subquery) => {
//                         self.push_state("call4_Query");
//                         self.process_query(*subquery);
//                     },
//                     any => panic_unexpected_struct!(any),
//                 }
//                 self.push_states(vec!["AnyAllAnyAll", any_all_val]);
//             },
//             Expr::BinaryOp { left, op, right } => {
//                 let (boolean, op_state) = match op {
//                     BinaryOperator::Eq => (false, "BinaryCompEqual"),
//                     BinaryOperator::Lt => (false, "BinaryCompLess"),
//                     BinaryOperator::LtEq => (false, "BinaryCompLessEqual"),
//                     BinaryOperator::NotEq => (false, "BinaryCompUnEqual"),
//                     BinaryOperator::And => (true, "BinaryBooleanOpV3AND"),
//                     BinaryOperator::Or => (true, "BinaryBooleanOpV3OR"),
//                     BinaryOperator::Xor => (true, "BinaryBooleanOpV3XOR"),
//                     any => panic_unexpected_struct!(any),
//                 };
//                 if boolean {
//                     self.push_states(vec!["BinaryBooleanOpV3", "call27_types"]);
//                     self.process_types(Some(TypesSelectedType::Val3), *left);
//                     self.push_state(op_state);
//                     self.push_state("call28_types");
//                     self.process_types(Some(TypesSelectedType::Val3), *right);
//                 } else {
//                     self.push_states(vec!["BinaryComp", "call5_types_all"]);
//                     let selected_type = self.process_types_all(*left);
//                     self.push_state(op_state);
//                     self.push_state("call24_types");
//                     self.process_types(Some(selected_type), *right);
//                 }
//             },
//             Expr::Like { negated, expr, pattern, escape_char } => {
//                 self.push_states(vec!["BinaryStringLike", "call25_types"]);
//                 self.process_types(Some(TypesSelectedType::String), *expr);
//                 if negated {
//                     self.push_state("BinaryStringLikeNot");
//                 }
//                 self.push_states(vec!["BinaryStringLikeIn", "call26_types"]);
//                 self.process_types(Some(TypesSelectedType::String), *pattern);
//             },
//             Expr::Value(Value::Boolean(bool_val)) => {
//                 if bool_val {
//                     self.push_state("true");
//                 } else {
//                     self.push_state("false");
//                 }
//             },
//             Expr::Nested(val3) => {
//                 self.push_states(vec!["Nested_VAL_3", "call29_types"]);
//                 self.process_types(Some(TypesSelectedType::Val3), *val3);
//             },
//             Expr::UnaryOp { op, expr } if op == UnaryOperator::Not => {
//                 self.push_states(vec!["UnaryNot_VAL_3", "call30_types"]);
//                 self.process_types(Some(TypesSelectedType::Val3), *expr);
//             },
//             any => panic_unexpected_struct!(any),
//         };
//         self.push_state("EXIT_VAL_3");
//     }

//     /// subgraph def_numeric
//     fn process_numeric(&mut self, expr: Expr) {
//         self.push_state("numeric");
//         match expr {
//             Expr::Value(Value::Number(literal, _)) => {
//                 self.push_state("numeric_literal");
//                 if let Ok(..) = literal.parse::<i64>() {
//                     self.push_state("numeric_literal_int");
//                 } else if let Ok(..) = literal.parse::<f64>() {
//                     self.push_state("numeric_literal_float");
//                 } else {
//                     panic_unexpected_struct!(literal);
//                 }
//             },
//             Expr::BinaryOp { left, op, right } => {
//                 self.push_states(vec!["BinaryNumericOp", "call48_types"]);
//                 self.process_types(Some(TypesSelectedType::Numeric), *left);
//                 self.push_state(match op {
//                     BinaryOperator::BitwiseAnd => "binary_numeric_bin_and",
//                     BinaryOperator::BitwiseOr => "binary_numeric_bin_or",
//                     BinaryOperator::PGBitwiseXor => "binary_numeric_bin_xor",  // BitwiseXor is exponentiation
//                     BinaryOperator::Divide => "binary_numeric_div",
//                     BinaryOperator::Minus => "binary_numeric_minus",
//                     BinaryOperator::Multiply => "binary_numeric_mul",
//                     BinaryOperator::Plus => "binary_numeric_plus",
//                     any => panic_unexpected_struct!(any),
//                 });
//                 self.push_state("call47_types");
//                 self.process_types(Some(TypesSelectedType::Numeric), *right);
//             },
//             Expr::UnaryOp { op, expr } => {
//                 self.push_states(vec!["UnaryNumericOp", match op {
//                     UnaryOperator::PGAbs => "unary_numeric_abs",
//                     UnaryOperator::PGBitwiseNot => "unary_numeric_bin_not",
//                     UnaryOperator::PGCubeRoot => "unary_numeric_cub_root",
//                     UnaryOperator::Minus => "unary_numeric_minus",
//                     UnaryOperator::Plus => "unary_numeric_plus",
//                     UnaryOperator::PGSquareRoot => "unary_numeric_sq_root",
//                     any => panic_unexpected_struct!(any),
//                 }, "call1_types"]);
//                 self.process_types(Some(TypesSelectedType::Numeric), *expr);
//             },
//             Expr::Position { expr, r#in } => {
//                 self.push_states(vec!["numeric_string_Position", "call2_types"]);
//                 self.process_types(Some(TypesSelectedType::String), *expr);
//                 self.push_states(vec!["string_position_in", "call3_types"]);
//                 self.process_types(Some(TypesSelectedType::String), *r#in);
//             },
//             Expr::Nested(expr) => {
//                 self.push_states(vec!["Nested_numeric", "call4_types"]);
//                 self.process_types(Some(TypesSelectedType::Numeric), *expr);
//             },
//             any => panic_unexpected_struct!(any),
//         };
//         self.push_state("EXIT_numeric");
//     }

//     /// subgraph def_string
//     fn process_string(&mut self, expr: Expr) {
//         self.push_state("string");
//         match expr {
//             Expr::Value(Value::SingleQuotedString(literal)) => {
//                 self.push_state("text_literal");
//             },
//             Expr::Trim { expr, trim_where, trim_what } => {
//                 self.push_state("text_trim");
//                 match (trim_where, trim_what) {
//                     (Some(trim_where), Some(trim_what)) => {
//                         self.push_state("call6_types");
//                         self.process_types(Some(TypesSelectedType::String), *trim_what);
//                         self.push_state(match trim_where {
//                             TrimWhereField::Both => "BOTH",
//                             TrimWhereField::Leading => "LEADING",
//                             TrimWhereField::Trailing => "TRAILING",
//                             any => panic_unexpected_struct!(any),
//                         });
//                     },
//                     (None, None) => {},
//                     any => panic_unexpected_struct!(any),
//                 };
//                 self.push_state("call5_types");
//                 self.process_types(Some(TypesSelectedType::String), *expr);
//             },
//             Expr::BinaryOp { left, op, right } => {
//                 self.push_states(vec!["text_concat", "call7_types"]);
//                 self.process_types(Some(TypesSelectedType::String), *left);
//                 self.push_states(vec!["text_concat_concat", "call8_types"]);
//                 self.process_types(Some(TypesSelectedType::String), *right);
//             },
//             any => panic_unexpected_struct!(any),
//         };
//         self.push_state("EXIT_string");
//     }

//     /// subgraph def_types
//     fn process_types(&mut self, selected_type: Option<TypesSelectedType>, expr: Expr) -> TypesSelectedType {
//         self.push_state("types");
//         let (types_selected_type, types_value) = match self.next_state().as_str() {
//             "types_select_type" => {
//                 let types_selected_type = match self.next_state().as_str() {
//                     "types_select_type_3vl" => TypesSelectedType::Val3,
//                     "types_select_type_numeric" => TypesSelectedType::Numeric,
//                     "types_select_type_text" => TypesSelectedType::String,
//                     any => self.panic_unexpected(any)
//                 };
//                 self.state_generator.push_known(types_selected_type.get_types());
//                 self.push_state("types_select_type_end");
//                 (types_selected_type, match self.next_state().as_str() {
//                     "call0_column_spec" => self.process_column_spec(),
//                     "call1_Query" => Expr::Subquery(Box::new(self.process_query())),
//                     any => self.panic_unexpected(any)
//                 })
//             },
//             "types_null" => (TypesSelectedType::Any, Expr::Value(Value::Null)),
//             "call0_numeric" => (TypesSelectedType::Numeric, self.process_numeric()),
//             "call1_VAL_3" => (TypesSelectedType::Val3, self.process_val_3()),
//             "call0_string" => (TypesSelectedType::String, self.process_string()),
//             any => self.panic_unexpected(any)
//         };
//         self.push_state("EXIT_types");
//         if let Some(to) = equal_to {
//             self.expect_type(&types_selected_type, &to);
//         }
//         if let Some(with) = compatible_with {
//             self.expect_compat(&types_selected_type, &with);
//         }
//         (types_selected_type, types_value.nest_children_if_needed())
//     }

//     /// subgraph def_types_all
//     fn process_types_all(&mut self, expr: Expr) -> TypesSelectedType {
//         self.push_state("types_all");
//         self.push_state("call0_types");
//         let ret = self.process_types(None, None);
//         self.push_state("EXIT_types_all");
//         ret
//     }

//     /// subgraph def_column_spec
//     fn process_column_spec(&mut self, expr: Expr) {
//         self.push_state("column_spec");
//         let next_state = self.next_state();
//         let relation = self.current_query_rm.get_random_relation();
//         let ret = match next_state.as_str() {
//             "unqualified_name" => Expr::Identifier(relation.gen_column_ident()),
//             "qualified_name" => Expr::CompoundIdentifier(vec![
//                 relation.gen_ident(), relation.gen_column_ident()
//             ]),
//             any => self.panic_unexpected(any)
//         };
//         self.push_state("EXIT_column_spec");
//         ret
//     }

//     /// subgraph def_list_expr
//     fn process_list_expr(&mut self, selected_type: Option<TypesSelectedType>, list_expr: Vec<Expr>) {
    //     self.push_state("list_expr");
    //     let list_compat_type = match self.next_state().as_str() {
    //         "call16_types" => TypesSelectedType::Numeric,
    //         "call17_types" => TypesSelectedType::Val3,
    //         "call18_types" => TypesSelectedType::String,
    //         "call19_types" => TypesSelectedType::ListExpr,
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

    fn choose_destination(&mut self, _rng: &mut ChaCha8Rng, outgoing_states: Vec<(bool, f64, NodeParams)>) -> Option<NodeParams> {
        if outgoing_states.len() == 0 {
            println!("List of outgoing nodes is empty!");
            return None
        }
        if outgoing_states.len() == 1 { return Some(outgoing_states[0].2.clone()) }
        let node_name = self.state_list[self.state_index].clone();
        self.state_index += 1;
        match outgoing_states.iter().find(|node| node.2.node_common.name == node_name) {
            Some(node) => Some(node.2.clone()),
            None => {
                println!("None of {:?} matches \"{node_name}\" inferred from query", outgoing_states);
                None
            },
        }
    }
}