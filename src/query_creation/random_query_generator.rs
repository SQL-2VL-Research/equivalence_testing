#[macro_use]
mod query_info;

use smol_str::SmolStr;
use sqlparser::ast::{
    Expr, Ident, Query, Select, SetExpr, TableAlias, TableFactor,
    TableWithJoins, Value, BinaryOperator, UnaryOperator, TrimWhereField, Array, SelectItem,
};

use self::query_info::{TypesSelectedType, RelationManager};

use super::state_generators::{MarkovChainGenerator, FunctionInputsType, NodeParams, DynamicModel};

pub struct QueryStats {
    /// Remember to increase this value before
    /// and decrease after generating a subquery, to
    /// control the maximum level of nesting
    /// allowed
    #[allow(dead_code)]
    current_nest_level: u32,
    /// current state number
    current_state_num: u32,
}

impl QueryStats {
    fn new() -> Self {
        Self {
            current_nest_level: 0,
            current_state_num: 0
        }
    }
}

struct LiteralSelector {
    /// used to store running query statistics, such as
    /// the current level of nesting
    pub stats: QueryStats,
}

impl LiteralSelector {
    fn new() -> Self {
        Self {
            stats: QueryStats::new()
        }
    }
}

impl DynamicModel for LiteralSelector {
    fn assign_probabilities(&mut self, node_outgoing: Vec<(f64, NodeParams)>) -> Vec::<(f64, NodeParams)> {
        let any_literal = node_outgoing.iter().any(|el| el.1.literal );
        node_outgoing.into_iter().map(|el| {
            let m = el.0 * f64::sqrt(self.stats.current_state_num as f64);
            (if any_literal {if el.1.literal { el.0 * m } else { el.0 / m }} else { el.0 }, el.1)
        }).collect()
    }
}

pub struct QueryGenerator {
    state_generator: MarkovChainGenerator,
    dynamic_model: LiteralSelector,
    current_query_rm: RelationManager,
}

macro_rules! unwrap_variant {
    ($target: expr, $pat: path) => { {
        if let $pat(a) = $target {
            a
        } else {
            panic!("Failed to unwrap variant: {} to {}", stringify!($target), stringify!($pat));
        }
    } };
}

impl QueryGenerator {
    pub fn from_state_generator(state_generator: MarkovChainGenerator) -> Self {
        QueryGenerator {
            state_generator,
            dynamic_model: LiteralSelector::new(),
            current_query_rm: RelationManager::new()
        }
    }

    fn next_state_opt(&mut self) -> Option<SmolStr> {
        self.dynamic_model.stats.current_state_num += 1;
        self.state_generator.next(&mut self.dynamic_model)
    }

    fn next_state(&mut self) -> SmolStr {
        self.next_state_opt().unwrap()
    }

    fn panic_unexpected(&mut self, state: &str) -> ! {
        self.state_generator.print_stack();
        panic!("Unexpected state: {state}");
    }

    fn expect_state(&mut self, state: &str) {
        let new_state = self.next_state();
        if new_state.as_str() != state {
            self.state_generator.print_stack();
            panic!("Expected {state}, got {new_state}")
        }
    }

    fn expect_compat(&self, target: &TypesSelectedType, compat_with: &TypesSelectedType) {
        if !target.is_compat_with(&compat_with) {
            self.state_generator.print_stack();
            panic!("Incompatible types: expected compatible with {:?}, got {:?}", compat_with, target);
        }
    }

    fn expect_type(&self, target: &TypesSelectedType, expect: &TypesSelectedType) {
        if target != expect {
            self.state_generator.print_stack();
            panic!("Unexpected type: expected {:?}, got {:?}", expect, target);
        }
    }

    /// subgraph def_Query
    fn handle_query(&mut self) -> Query {
        self.expect_state("Query");
        let mut select_limit = Option::<Expr>::None;
        if let Some(mods) = self.state_generator.get_modifiers() {
            if mods.contains(&SmolStr::new("single value")) {
                select_limit = Some(Expr::Identifier(Ident::new("1")));
                // TODO: Not only limits can enforce this
            } else {
                panic!("Unexpected mods (Query): {:?}", mods);
            }
        }
        if let FunctionInputsType::TypeName(type_name) = self.state_generator.get_inputs() {
            println!("TODO: Enforce single column & column type (Query): {type_name}")
        }
        let mut select_body = Select {
            distinct: false,
            top: None,
            projection: vec![],
            into: None,
            from: vec![],
            lateral_views: vec![],
            selection: None,
            group_by: vec![],
            cluster_by: vec![],
            distribute_by: vec![],
            sort_by: vec![],
            having: None,
            qualify: None,
        };
        self.expect_state("FROM");

        loop {
            select_body.from.push(TableWithJoins { relation: match self.next_state().as_str() {
                "Table" => TableFactor::Table {
                    name: self.current_query_rm.new_relation().gen_object_name(),
                    alias: None,
                    args: None,
                    with_hints: vec![]
                },
                "call0_Query" => TableFactor::Derived {
                    lateral: false,
                    subquery: Box::new(self.handle_query()),
                    alias: Some(TableAlias {
                        name: self.current_query_rm.new_ident(),
                        columns: vec![],
                    })
                },
                "EXIT_FROM" => break,
                any => self.panic_unexpected(any)
            }, joins: vec![] });
            self.expect_state("FROM_multiple_relations");
        }

        match self.next_state().as_str() {
            "WHERE" => {
                self.expect_state("call0_VAL_3");
                select_body.selection = Some(self.handle_val_3());
                self.expect_state("EXIT_WHERE");
            },
            "EXIT_WHERE" => {},
            any => self.panic_unexpected(any)
        }

        // CONTINUE FROM HERE
        // PLASEHOLDER CODE BEGIN
        select_body.projection.push(SelectItem::Wildcard);
        // PLASEHOLDER CODE END

        self.expect_state("EXIT_Query");
        Query {
            with: None,
            body: SetExpr::Select(Box::new(select_body)),
            order_by: vec![],
            limit: select_limit,
            offset: None,
            fetch: None,
            lock: None,
        }
    }

    /// subgraph def_VAL_3
    fn handle_val_3(&mut self) -> Expr {
        self.expect_state("VAL_3");
        let val3 = match self.next_state().as_str() {
            "IsNull" => {
                let is_null_not_flag = match self.next_state().as_str() {
                    "IsNull_not" => {
                        self.expect_state("call0_types_all");
                        true
                    }
                    "call0_types_all" => false,
                    any => self.panic_unexpected(any)
                };
                let types_value = Box::new(self.handle_types_all().1);
                if is_null_not_flag {
                    Expr::IsNotNull(types_value)
                } else {
                    Expr::IsNull(types_value)
                }
            },
            "IsDistinctFrom" => {
                self.expect_state("call1_types_all");
                let (types_selected_type, types_value_1) = self.handle_types_all();
                self.state_generator.push_compatible(types_selected_type.get_compat_types());
                let is_distinct_not_flag = match self.next_state().as_str() {
                    "IsDistinctNOT" => {
                        self.expect_state("DISTINCT");
                        true
                    }
                    "DISTINCT" => false,
                    any => self.panic_unexpected(any)
                };
                self.expect_state("call21_types");
                let types_value_2 = self.handle_types(None, Some(types_selected_type)).1;
                if is_distinct_not_flag {
                    Expr::IsNotDistinctFrom(Box::new(types_value_1), Box::new(types_value_2))
                } else {
                    Expr::IsDistinctFrom(Box::new(types_value_1), Box::new(types_value_2))
                }
            },
            "Exists" => {
                let exists_not_flag = match self.next_state().as_str() {
                    "Exists_not" => {
                        self.expect_state("call2_Query");
                        true
                    },
                    "call2_Query" => false,
                    any => self.panic_unexpected(any)
                };
                Expr::Exists {
                    subquery: Box::new(self.handle_query()),
                    negated: exists_not_flag
                }
            },
            "InList" => {
                self.expect_state("call2_types_all");
                let (types_selected_type, types_value) = self.handle_types_all();
                self.state_generator.push_compatible(types_selected_type.get_compat_types());
                let in_list_not_flag = match self.next_state().as_str() {
                    "InListNot" => {
                        self.expect_state("InListIn");
                        true
                    },
                    "InListIn" => false,
                    any => self.panic_unexpected(any)
                };
                self.expect_state("call1_list_expr");
                Expr::InList {
                    expr: Box::new(types_value),
                    list: unwrap_variant!(self.handle_list_expr(), Expr::Tuple),
                    negated: in_list_not_flag
                }
            },
            "InSubquery" => {
                self.expect_state("call3_types_all");
                let (types_selected_type, types_value) = self.handle_types_all();
                self.state_generator.push_compatible(types_selected_type.get_compat_types());
                let in_subquery_not_flag = match self.next_state().as_str() {
                    "InSubqueryNot" => {
                        self.expect_state("InSubqueryIn");
                        true
                    },
                    "InSubqueryIn" => false,
                    any => self.panic_unexpected(any)
                };
                self.expect_state("call3_Query");
                let query = self.handle_query();
                Expr::InSubquery {
                    expr: Box::new(types_value),
                    subquery: Box::new(query),
                    negated: in_subquery_not_flag
                }
            },
            "Between" => {
                self.expect_state("call4_types_all");
                let (types_selected_type, types_value_1) = self.handle_types_all();
                let type_names = types_selected_type.get_compat_types();
                let between_not_flag = match self.next_state().as_str() {
                    "BetweenBetweenNot" => {
                        self.expect_state("BetweenBetween");
                        true
                    },
                    "BetweenBetween" => false,
                    any => self.panic_unexpected(any)
                };
                self.state_generator.push_compatible(type_names.clone());
                self.expect_state("call22_types");
                let types_value_2 = self.handle_types(None, Some(types_selected_type.clone())).1;
                self.expect_state("BetweenBetweenAnd");
                self.state_generator.push_compatible(type_names);
                self.expect_state("call23_types");
                let types_value_3 = self.handle_types(None, Some(types_selected_type)).1;
                Expr::Between {
                    expr: Box::new(types_value_1),
                    negated: between_not_flag,
                    low: Box::new(types_value_2),
                    high: Box::new(types_value_3)
                }
            },
            "BinaryComp" => {
                self.expect_state("call5_types_all");
                let (types_selected_type, types_value_1) = self.handle_types_all();
                self.state_generator.push_compatible(types_selected_type.get_compat_types());
                let binary_comp_op = match self.next_state().as_str() {
                    "BinaryCompEqual" => BinaryOperator::Eq,
                    "BinaryCompLess" => BinaryOperator::Lt,
                    "BinaryCompLessEqual" => BinaryOperator::LtEq,
                    "BinaryCompUnEqual" => BinaryOperator::NotEq,
                    any => self.panic_unexpected(any)
                };
                self.expect_state("call24_types");
                let types_value_2 = self.handle_types(None, Some(types_selected_type)).1;
                Expr::BinaryOp {
                    left: Box::new(types_value_1),
                    op: binary_comp_op,
                    right: Box::new(types_value_2)
                }
            },
            "AnyAll" => {
                self.expect_state("call6_types_all");
                let (types_selected_type, types_value) = self.handle_types_all();
                self.expect_state("AnyAllSelectOp");
                let any_all_op = match self.next_state().as_str() {
                    "AnyAllEqual" => BinaryOperator::Eq,
                    "AnyAllLess" => BinaryOperator::Lt,
                    "AnyAllLessEqual" => BinaryOperator::LtEq,
                    "AnyAllUnEqual" => BinaryOperator::NotEq,
                    any => self.panic_unexpected(any)
                };
                self.expect_state("AnyAllSelectIter");
                self.state_generator.push_compatible(types_selected_type.get_compat_types());
                let iterable = Box::new(match self.next_state().as_str() {
                    "call4_Query" => Expr::Subquery(Box::new(self.handle_query())),
                    "call1_array" => self.handle_array(),
                    any => self.panic_unexpected(any)
                });
                self.expect_state("AnyAllAnyAll");
                let iterable = Box::new(match self.next_state().as_str() {
                    "AnyAllAnyAllAll" => Expr::AllOp(iterable),
                    "AnyAllAnyAllAny" => Expr::AnyOp(iterable),
                    any => self.panic_unexpected(any),
                });
                Expr::BinaryOp {
                    left: Box::new(types_value),
                    op: any_all_op,
                    right: iterable,
                }
            },
            "BinaryStringLike" => {
                self.expect_state("call25_types");
                let types_value_1 = self.handle_types(Some(TypesSelectedType::String), None).1;
                let string_like_not_flag = match self.next_state().as_str() {
                    "BinaryStringLikeNot" => {
                        self.expect_state("BinaryStringLikeIn");
                        true
                    }
                    "BinaryStringLikeIn" => false,
                    any => self.panic_unexpected(any)
                };
                self.expect_state("call26_types");
                let types_value_2 = self.handle_types(Some(TypesSelectedType::String), None).1;
                Expr::BinaryOp {
                left: Box::new(types_value_1),
                    op: if string_like_not_flag { BinaryOperator::NotLike } else { BinaryOperator::Like },
                    right: Box::new(types_value_2)
                }
            },
            "BinaryBooleanOpV3" => {
                self.expect_state("call27_types");
                let types_value_1 = self.handle_types(Some(TypesSelectedType::Val3), None).1;
                let binary_bool_op = match self.next_state().as_str() {
                    "BinaryBooleanOpV3AND" => BinaryOperator::And,
                    "BinaryBooleanOpV3OR" => BinaryOperator::Or,
                    "BinaryBooleanOpV3XOR" => BinaryOperator::Xor,
                    any => self.panic_unexpected(any)
                };
                self.expect_state("call28_types");
                let types_value_2 = self.handle_types(Some(TypesSelectedType::Val3), None).1;
                Expr::BinaryOp {
                    left: Box::new(types_value_1),
                    op: binary_bool_op,
                    right: Box::new(types_value_2)
                }
            },
            "true" => Expr::Value(Value::Boolean(true)),
            "false" => Expr::Value(Value::Boolean(false)),
            "Nested_VAL_3" => {
                self.expect_state("call29_types");
                Expr::Nested(Box::new(self.handle_types(Some(TypesSelectedType::Val3), None).1))
            },
            "UnaryNot_VAL_3" => {
                self.expect_state("call30_types");
                Expr::UnaryOp { op: UnaryOperator::Not, expr: Box::new( self.handle_types(
                    Some(TypesSelectedType::Val3), None
                ).1) }
            },
            any => self.panic_unexpected(any)
        };
        self.expect_state("EXIT_VAL_3");
        val3
    }

    /// subgraph def_numeric
    fn handle_numeric(&mut self) -> Expr {
        self.expect_state("numeric");
        let numeric = match self.next_state().as_str() {
            "numeric_literal" => {
                Expr::Value(Value::Number(match self.next_state().as_str() {
                    "numeric_literal_float" => {
                        "3.1415"  // TODO: hardcode
                    },
                    "numeric_literal_int" => {
                        "3"       // TODO: hardcode
                    },
                    any => self.panic_unexpected(any)
                }.to_string(), false))
            },
            "BinaryNumericOp" => {
                self.expect_state("call48_types");
                let types_value_1 = self.handle_types(Some(TypesSelectedType::Numeric), None).1;
                let numeric_binary_op = match self.next_state().as_str() {
                    "binary_numeric_bin_and" => BinaryOperator::BitwiseAnd,
                    "binary_numeric_bin_or" => BinaryOperator::BitwiseOr,
                    "binary_numeric_bin_xor" => BinaryOperator::BitwiseXor,
                    "binary_numeric_div" => BinaryOperator::Divide,
                    "binary_numeric_minus" => BinaryOperator::Minus,
                    "binary_numeric_mul" => BinaryOperator::Multiply,
                    "binary_numeric_plus" => BinaryOperator::Plus,
                    any => self.panic_unexpected(any),
                };
                self.expect_state("call47_types");
                let types_value_2 = self.handle_types(Some(TypesSelectedType::Numeric), None).1;
                Expr::BinaryOp {
                    left: Box::new(types_value_1),
                    op: numeric_binary_op,
                    right: Box::new(types_value_2)
                }
            },
            "UnaryNumericOp" => {
                let numeric_unary_op = match self.next_state().as_str() {
                    "unary_numeric_abs" => UnaryOperator::PGAbs,
                    "unary_numeric_bin_not" => UnaryOperator::PGBitwiseNot,
                    "unary_numeric_cub_root" => UnaryOperator::PGCubeRoot,
                    "unary_numeric_minus" => UnaryOperator::Minus,
                    "unary_numeric_plus" => UnaryOperator::Plus,
                    "unary_numeric_postfix_fact" => UnaryOperator::PGPostfixFactorial,
                    "unary_numeric_prefix_fact" => UnaryOperator::PGPrefixFactorial,
                    "unary_numeric_sq_root" => UnaryOperator::PGSquareRoot,
                    any => self.panic_unexpected(any),
                };
                self.expect_state("call1_types");
                let types_value = self.handle_types(Some(TypesSelectedType::Numeric), None).1;
                Expr::UnaryOp {
                    op: numeric_unary_op,
                    expr: Box::new(types_value)
                }
            },
            "numeric_string_Position" => {
                self.expect_state("call2_types");
                let types_value_1 = self.handle_types(Some(TypesSelectedType::String), None).1;
                self.expect_state("string_position_in");
                self.expect_state("call3_types");
                let types_value_2 = self.handle_types(Some(TypesSelectedType::String), None).1;
                Expr::Position {
                    expr: Box::new(types_value_1),
                    r#in: Box::new(types_value_2)
                }
            },
            "Nested_numeric" => {
                self.expect_state("call4_types");
                let types_value = self.handle_types(Some(TypesSelectedType::Numeric), None).1;
                Expr::Nested(Box::new(types_value))
            },
            any => self.panic_unexpected(any)
        };
        self.expect_state("EXIT_numeric");
        numeric
    }

    /// subgraph def_string
    fn handle_string(&mut self) -> Expr {
        self.expect_state("string");
        let string = match self.next_state().as_str() {
            "string_literal" => Expr::Value(Value::SingleQuotedString("HJeihfbwei".to_string())),  // TODO: hardcoded
            "string_trim" => {
                let trim_where = match self.next_state().as_str() {
                    "call6_types" => {
                        let types_value = self.handle_types(Some(TypesSelectedType::String), None).1;
                        let spec_mode = match self.next_state().as_str() {
                            "BOTH" => TrimWhereField::Both,
                            "LEADING" => TrimWhereField::Leading,
                            "TRAILING" => TrimWhereField::Trailing,
                            any => self.panic_unexpected(any)
                        };
                        self.expect_state("call5_types");
                        Some((spec_mode, Box::new(types_value)))
                    },
                    "call5_types" => None,
                    any => self.panic_unexpected(any)
                };
                let types_value = self.handle_types(Some(TypesSelectedType::String), None).1;
                Expr::Trim {
                    expr: Box::new(types_value), trim_where
                }
            },
            "string_concat" => {
                self.expect_state("call7_types");
                let types_value_1 = self.handle_types(Some(TypesSelectedType::String), None).1;
                self.expect_state("string_concat_concat");
                self.expect_state("call8_types");
                let types_value_2 = self.handle_types(Some(TypesSelectedType::String), None).1;
                Expr::BinaryOp {
                    left: Box::new(types_value_1),
                    op: BinaryOperator::StringConcat,
                    right: Box::new(types_value_2)
                }
            },
            any => self.panic_unexpected(any)
        };
        self.expect_state("EXIT_string");
        string
    }

    /// subgraph def_types
    fn handle_types(
        &mut self, equal_to: Option<TypesSelectedType>,
        compatible_with: Option<TypesSelectedType>
    ) -> (TypesSelectedType, Expr) {
        self.expect_state("types");
        let (types_selected_type, types_value) = match self.next_state().as_str() {
            "types_select_type" => {
                let types_selected_type = match self.next_state().as_str() {
                    "types_select_type_3vl" => TypesSelectedType::Val3,
                    "types_select_type_array" => TypesSelectedType::Array,
                    "types_select_type_list_expr" => TypesSelectedType::ListExpr,
                    "types_select_type_numeric" => TypesSelectedType::Numeric,
                    "types_select_type_string" => TypesSelectedType::String,
                    any => self.panic_unexpected(any)
                };
                self.state_generator.push_known(types_selected_type.get_types());
                self.expect_state("types_select_type_end");
                (types_selected_type, match self.next_state().as_str() {
                    "call0_column_spec" => self.handle_column_spec(),
                    "call1_Query" => Expr::Subquery(Box::new(self.handle_query())),
                    any => self.panic_unexpected(any)
                })
            },
            "types_null" => (TypesSelectedType::Any, Expr::Value(Value::Null)),
            "call0_numeric" => (TypesSelectedType::Numeric, self.handle_numeric()),
            "call1_VAL_3" => (TypesSelectedType::Val3, self.handle_val_3()),
            "call0_string" => (TypesSelectedType::String, self.handle_string()),
            "call0_list_expr" => {
                self.state_generator.push_known(match self.state_generator.get_inputs() {
                    FunctionInputsType::TypeNameList(list) => list,
                    any => panic!("Couldn't pass {:?} to subgraph def_list_expr", any)
                });
                (TypesSelectedType::ListExpr, self.handle_list_expr())
            },
            "call0_array" => {
                self.state_generator.push_known(match self.state_generator.get_inputs() {
                    FunctionInputsType::TypeNameList(list) => list,
                    any => panic!("Couldn't pass {:?} to subgraph def_array", any)
                });
                (TypesSelectedType::Array, self.handle_array())
            },
            any => self.panic_unexpected(any)
        };
        self.expect_state("EXIT_types");
        if let Some(to) = equal_to {
            self.expect_type(&types_selected_type, &to);
        }
        if let Some(with) = compatible_with {
            self.expect_compat(&types_selected_type, &with);
        }
        (types_selected_type, types_value)
    }

    /// subgraph def_types_all
    fn handle_types_all(&mut self) -> (TypesSelectedType, Expr) {
        self.expect_state("types_all");
        self.expect_state("call0_types");
        let ret = self.handle_types(None, None);
        self.expect_state("EXIT_types_all");
        ret
    }

    /// subgraph def_column_spec
    fn handle_column_spec(&mut self) -> Expr {
        self.expect_state("column_spec");
        let ret = match self.next_state().as_str() {
            "qualified_name" => Expr::Identifier(self.current_query_rm.new_relation().gen_column_ident()),
            "unqualified_name" => Expr::CompoundIdentifier(vec![
                self.current_query_rm.new_relation().gen_column_ident(),
                self.current_query_rm.new_relation().gen_column_ident()
            ]),
            any => self.panic_unexpected(any)
        };
        self.expect_state("EXIT_column_spec");
        ret
    }

    /// subgraph def_array
    fn handle_array(&mut self) -> Expr {
        self.expect_state("array");
        let array_compat_type = match self.next_state().as_str() {
            "call12_types" => TypesSelectedType::Numeric,
            "call13_types" => TypesSelectedType::Val3,
            "call31_types" => TypesSelectedType::String,
            "call51_types" => TypesSelectedType::ListExpr,
            "call14_types" => TypesSelectedType::Array,
            any => self.panic_unexpected(any)
        };
        let types_value = self.handle_types(Some(array_compat_type.clone()), None).1;
        let mut array: Vec<Expr> = vec![types_value];
        loop {
            match self.next_state().as_str() {
                "call50_types" => {
                    self.state_generator.push_compatible(array_compat_type.get_compat_types());
                    let types_value = self.handle_types(None, Some(array_compat_type.clone())).1;
                    array.push(types_value);
                },
                "EXIT_array" => break,
                any => self.panic_unexpected(any)
            }
        }
        Expr::Array(Array {
            elem: array,
            named: false
        })
    }

    /// subgraph def_list_expr
    fn handle_list_expr(&mut self) -> Expr {
        self.expect_state("list_expr");
        let list_compat_type = match self.next_state().as_str() {
            "call16_types" => TypesSelectedType::Numeric,
            "call17_types" => TypesSelectedType::Val3,
            "call18_types" => TypesSelectedType::String,
            "call19_types" => TypesSelectedType::ListExpr,
            "call20_types" => TypesSelectedType::Array,
            any => self.panic_unexpected(any)
        };
        let types_value = self.handle_types(Some(list_compat_type.clone()), None).1;
        let mut list_expr: Vec<Expr> = vec![types_value];
        loop {
            match self.next_state().as_str() {
                "call49_types" => {
                    self.state_generator.push_compatible(list_compat_type.get_compat_types());
                    let types_value = self.handle_types(None, Some(list_compat_type.clone())).1;
                    list_expr.push(types_value);
                },
                "EXIT_list_expr" => break,
                any => self.panic_unexpected(any)
            }
        }
        Expr::Tuple(list_expr)
    }

    /// starting point; calls handle_query for the first time
    fn generate(&mut self) -> Query {
        let query = self.handle_query();
        // reset the generator
        if let Some(state) = self.next_state_opt() {
            panic!("Couldn't reset state_generator: Received {state}");
        }
        self.current_query_rm = RelationManager::new();
        self.dynamic_model = LiteralSelector::new();
        query
    }
}

impl Iterator for QueryGenerator {
    type Item = Query;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.generate())
    }
}