#[macro_use]
pub mod query_info;
pub mod call_modifiers;
pub mod expr_precedence;
pub mod aggregate_function_settings;

use std::{path::PathBuf, vec};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use smol_str::SmolStr;
use sqlparser::ast::{
    Expr, Ident, Query, Select, SetExpr, TableFactor,
    TableWithJoins, Value, BinaryOperator, UnaryOperator, TrimWhereField, SelectItem, WildcardAdditionalOptions, ObjectName, DataType, FunctionArg,
};
use crate::config::TomlReadable;

use super::{super::{unwrap_variant, unwrap_variant_or_else}, state_generators::{CallTypes, markov_chain_generator::subgraph_type::SubgraphType}};
use self::{query_info::{DatabaseSchema, ClauseContext}, call_modifiers::TypesTypeValue, aggregate_function_settings::AggregateFunctionDistribution, expr_precedence::ExpressionPriority};

use super::state_generators::{MarkovChainGenerator, dynamic_models::DynamicModel, state_choosers::StateChooser};

pub struct QueryGeneratorConfig {
    pub print_queries: bool,
    pub print_schema: bool,
    pub table_schema_path: PathBuf,
    pub dynamic_model_name: String,
    pub aggregate_functions_distribution: AggregateFunctionDistribution,
}

impl TomlReadable for QueryGeneratorConfig {
    fn from_toml(toml_config: &toml::Value) -> Self {
        let section = &toml_config["query_generator"];
        Self {
            print_queries: section["print_queries"].as_bool().unwrap(),
            print_schema: section["print_schema"].as_bool().unwrap(),
            table_schema_path: PathBuf::from(section["table_schema_path"].as_str().unwrap()),
            dynamic_model_name: section["dynamic_model"].as_str().unwrap().to_string(),
            aggregate_functions_distribution: AggregateFunctionDistribution::parse_file(
                PathBuf::from(section["aggregate_functions_distribution_map_file"].as_str().unwrap()),
            ),
        }
    }
}

pub struct QueryGenerator<DynMod: DynamicModel, StC: StateChooser> {
    config: QueryGeneratorConfig,
    state_generator: MarkovChainGenerator<StC>,
    dynamic_model: Box<DynMod>,
    database_schema: DatabaseSchema,
    clause_context: ClauseContext,
    free_projection_alias_index: u32,
    rng: ChaCha8Rng,
}

impl<DynMod: DynamicModel, StC: StateChooser> QueryGenerator<DynMod, StC> {
    pub fn from_state_generator_and_schema(state_generator: MarkovChainGenerator<StC>, config: QueryGeneratorConfig) -> Self {
        let mut _self = QueryGenerator::<DynMod, StC> {
            state_generator,
            dynamic_model: Box::new(DynMod::new()),
            database_schema: DatabaseSchema::parse_schema(&config.table_schema_path),
            config,
            clause_context: ClauseContext::new(),
            free_projection_alias_index: 1,
            rng: ChaCha8Rng::seed_from_u64(1),
        };

        if _self.config.print_schema {
            println!("Relations:\n{}", _self.database_schema);
        }

        _self
    }

    fn next_state_opt(&mut self) -> Option<SmolStr> {
        self.state_generator.next(&mut self.rng, &mut self.clause_context, &mut *self.dynamic_model)
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

    fn expect_compat(&self, target: &SubgraphType, compat_with: &SubgraphType) {
        if !target.is_compat_with(compat_with) {
            self.state_generator.print_stack();
            panic!("Incompatible types: expected compatible with {:?}, got {:?}", compat_with, target);
        }
    }

    pub fn gen_select_alias(&mut self) -> ObjectName {
        let name = format!("C{}", self.free_projection_alias_index);
        self.free_projection_alias_index += 1;
        ObjectName(vec![Ident { value: name.clone(), quote_style: None }])
    }

    /// subgraph def_Query
    fn handle_query(&mut self) -> (Query, Vec<(Option<ObjectName>, SubgraphType)>) {
        self.dynamic_model.notify_subquery_creation_begin();
        self.clause_context.on_query_begin();
        self.expect_state("Query");

        let select_limit = match self.next_state().as_str() {
            "single_value_true" => {
                self.expect_state("FROM");
                Some(Expr::Value(Value::Number("1".to_string(), false)))
            },
            "single_value_false" => {
                match self.next_state().as_str() {
                    "limit" => {
                        self.expect_state("call52_types");
                        let num = self.handle_types(Some(SubgraphType::Numeric), None).1;
                        self.expect_state("FROM");
                        Some(num)
                    },
                    "FROM" => None,
                    any => self.panic_unexpected(any)
                }
            },
            any => self.panic_unexpected(any)
        };

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

        loop {
            select_body.from.push(TableWithJoins { relation: match self.next_state().as_str() {
                "Table" => {
                    let create_table_st = self.database_schema.get_random_table_def(&mut self.rng);
                    let alias = self.clause_context.from_mut().append_table(create_table_st);
                    TableFactor::Table {
                        name: create_table_st.name.clone(),
                        alias: Some(alias),
                        args: None,
                        with_hints: vec![],
                        columns_definition: None,
                    }
                },
                "call0_Query" => {
                    let (query, column_idents_and_graph_types) = self.handle_query();
                    let alias = self.clause_context.from_mut().append_query(column_idents_and_graph_types);
                    TableFactor::Derived {
                        lateral: false,
                        subquery: Box::new(query),
                        alias: Some(alias)
                    }
                },
                "EXIT_FROM" => break,
                any => self.panic_unexpected(any)
            }, joins: vec![] });
            self.expect_state("FROM_multiple_relations");
        }

        match self.next_state().as_str() {
            "WHERE" => {
                self.expect_state("call53_types");
                select_body.selection = Some(self.handle_types(Some(SubgraphType::Val3), None).1);
                self.expect_state("EXIT_WHERE");
            },
            "EXIT_WHERE" => {},
            any => self.panic_unexpected(any)
        }

        self.expect_state("SELECT");
        select_body.distinct = match self.next_state().as_str() {
            "SELECT_DISTINCT" => {
                self.expect_state("SELECT_distinct_end");
                true
            },
            "SELECT_distinct_end" => false,
            any => self.panic_unexpected(any)
        };

        let mut column_idents_and_graph_types = vec![];

        self.expect_state("SELECT_projection");
        while match self.next_state().as_str() {
            "SELECT_list" => true,
            "call0_aggregate_function" => {
                select_body.projection.push(
                    SelectItem::UnnamedExpr(self.handle_aggregate_function(None, None)));
                self.expect_state("EXIT_SELECT");
                false
            },
            "SELECT_list_multiple_values_single_column_false" => {
                self.expect_state("SELECT_list");
                true
            },
            "EXIT_SELECT" => false,
            any => self.panic_unexpected(any)
        } {
            match self.next_state().as_str() {
                "SELECT_wildcard" => {
                    column_idents_and_graph_types = [
                        column_idents_and_graph_types,
                        self.clause_context
                            .from()
                            .get_wildcard_columns()
                    ].concat();
                    select_body.projection.push(SelectItem::Wildcard(WildcardAdditionalOptions {
                        opt_exclude: None, opt_except: None, opt_rename: None,
                    }));
                    continue;
                },
                "SELECT_qualified_wildcard" => {
                    let from_contents = self.clause_context.from();
                    let (alias, relation) = from_contents.get_random_relation(&mut self.rng);
                    column_idents_and_graph_types = [
                        column_idents_and_graph_types,
                        relation.get_columns_with_types()
                    ].concat();
                    select_body.projection.push(SelectItem::QualifiedWildcard(
                        alias.to_owned(),
                        WildcardAdditionalOptions {
                            opt_exclude: None, opt_except: None, opt_rename: None,
                        }
                    ));
                },
                arm @ ("SELECT_unnamed_expr" | "SELECT_expr_with_alias") => {
                    self.expect_state("call54_types");
                    let (subgraph_type, expr) = self.handle_types(None, None);
                    let (alias, select_item) = match arm {
                        "SELECT_unnamed_expr" => (
                            None, SelectItem::UnnamedExpr(expr)
                        ),
                        "SELECT_expr_with_alias" => {
                            let select_alias = self.gen_select_alias();
                            (Some(select_alias.clone()), SelectItem::ExprWithAlias {
                                expr, alias: select_alias.0[0].clone(),
                            })
                        },
                        any => self.panic_unexpected(any)
                    };
                    select_body.projection.push(select_item);
                    column_idents_and_graph_types.push((alias, subgraph_type));
                },
                any => self.panic_unexpected(any)
            };
            self.expect_state("SELECT_list_multiple_values");
        }

        self.expect_state("EXIT_Query");
        self.dynamic_model.notify_subquery_creation_end();
        self.clause_context.on_query_end();
        (Query {
            with: None,
            body: Box::new(SetExpr::Select(Box::new(select_body))),
            order_by: vec![],
            limit: select_limit,
            offset: None,
            fetch: None,
            locks: vec![],
        }, column_idents_and_graph_types)
    }

    /// subgraph def_VAL_3
    fn handle_val_3(&mut self) -> (SubgraphType, Expr) {
        self.expect_state("VAL_3");
        let val3 = match self.next_state().as_str() {
            "IsNull" => {
                let is_null_not_flag = match self.next_state().as_str() {
                    "IsNull_not" => {
                        self.expect_state("call55_types");
                        true
                    }
                    "call55_types" => false,
                    any => self.panic_unexpected(any)
                };
                let types_value = Box::new(self.handle_types(None, None).1);
                if is_null_not_flag {
                    Expr::IsNotNull(types_value)
                } else {
                    Expr::IsNull(types_value)
                }
            },
            "IsDistinctFrom" => {
                self.expect_state("call56_types");
                let (types_selected_type, types_value_1) = self.handle_types(None, None);
                self.state_generator.set_compatible_list(types_selected_type.get_compat_types());
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
                    subquery: Box::new(self.handle_query().0),
                    negated: exists_not_flag
                }
            },
            "InList" => {
                self.expect_state("call57_types");
                let (types_selected_type, types_value) = self.handle_types(None, None);
                self.state_generator.set_compatible_list(types_selected_type.get_compat_types());
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
                    list: unwrap_variant!(self.handle_list_expr().1, Expr::Tuple),
                    negated: in_list_not_flag
                }
            },
            "InSubquery" => {
                self.expect_state("call58_types");
                let (types_selected_type, types_value) = self.handle_types(None, None);
                self.state_generator.set_compatible_list(types_selected_type.get_compat_types());
                let in_subquery_not_flag = match self.next_state().as_str() {
                    "InSubqueryNot" => {
                        self.expect_state("InSubqueryIn");
                        true
                    },
                    "InSubqueryIn" => false,
                    any => self.panic_unexpected(any)
                };
                self.expect_state("call3_Query");
                let query = self.handle_query().0;
                Expr::InSubquery {
                    expr: Box::new(types_value),
                    subquery: Box::new(query),
                    negated: in_subquery_not_flag
                }
            },
            "Between" => {
                self.expect_state("call59_types");
                let (types_selected_type, types_value_1) = self.handle_types(None, None);
                self.state_generator.set_compatible_list(types_selected_type.get_compat_types());
                let between_not_flag = match self.next_state().as_str() {
                    "BetweenBetweenNot" => {
                        self.expect_state("BetweenBetween");
                        true
                    },
                    "BetweenBetween" => false,
                    any => self.panic_unexpected(any)
                };
                self.expect_state("call22_types");
                let types_value_2 = self.handle_types(None, Some(types_selected_type.clone())).1;
                self.expect_state("BetweenBetweenAnd");
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
                self.expect_state("call60_types");
                let (types_selected_type, types_value_1) = self.handle_types(None, None);
                self.state_generator.set_compatible_list(types_selected_type.get_compat_types());
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
                self.expect_state("call61_types");
                let (types_selected_type, types_value) = self.handle_types(None, None);
                self.state_generator.set_compatible_list(types_selected_type.get_compat_types());
                self.expect_state("AnyAllSelectOp");
                let any_all_op = match self.next_state().as_str() {
                    "AnyAllEqual" => BinaryOperator::Eq,
                    "AnyAllLess" => BinaryOperator::Lt,
                    "AnyAllLessEqual" => BinaryOperator::LtEq,
                    "AnyAllUnEqual" => BinaryOperator::NotEq,
                    any => self.panic_unexpected(any)
                };
                self.expect_state("AnyAllSelectIter");
                let iterable = Box::new(match self.next_state().as_str() {
                    "call4_Query" => Expr::Subquery(Box::new(self.handle_query().0)),
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
                let types_value_1 = self.handle_types(Some(SubgraphType::Text), None).1;
                let string_like_not_flag = match self.next_state().as_str() {
                    "BinaryStringLikeNot" => {
                        self.expect_state("BinaryStringLikeIn");
                        true
                    }
                    "BinaryStringLikeIn" => false,
                    any => self.panic_unexpected(any)
                };
                self.expect_state("call26_types");
                let types_value_2 = self.handle_types(Some(SubgraphType::Text), None).1;
                Expr::Like {
                    negated: string_like_not_flag,
                    expr: Box::new(types_value_1),
                    pattern: Box::new(types_value_2),
                    escape_char: None
                }
            },
            "BinaryBooleanOpV3" => {
                self.expect_state("call27_types");
                let types_value_1 = self.handle_types(Some(SubgraphType::Val3), None).1;
                let binary_bool_op = match self.next_state().as_str() {
                    "BinaryBooleanOpV3AND" => BinaryOperator::And,
                    "BinaryBooleanOpV3OR" => BinaryOperator::Or,
                    "BinaryBooleanOpV3XOR" => BinaryOperator::Xor,
                    any => self.panic_unexpected(any)
                };
                self.expect_state("call28_types");
                let types_value_2 = self.handle_types(Some(SubgraphType::Val3), None).1;
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
                Expr::Nested(Box::new(self.handle_types(Some(SubgraphType::Val3), None).1))
            },
            "UnaryNot_VAL_3" => {
                self.expect_state("call30_types");
                Expr::UnaryOp { op: UnaryOperator::Not, expr: Box::new( self.handle_types(
                    Some(SubgraphType::Val3), None
                ).1) }
            },
            any => self.panic_unexpected(any)
        };
        self.expect_state("EXIT_VAL_3");
        (SubgraphType::Val3, val3)
    }

    /// subgraph def_numeric
    fn handle_numeric(&mut self) -> (SubgraphType, Expr) {
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
                let types_value_1 = self.handle_types(Some(SubgraphType::Numeric), None).1;
                let numeric_binary_op = match self.next_state().as_str() {
                    "binary_numeric_bin_and" => BinaryOperator::BitwiseAnd,
                    "binary_numeric_bin_or" => BinaryOperator::BitwiseOr,
                    "binary_numeric_bin_xor" => BinaryOperator::PGBitwiseXor,  // BitwiseXor is exponentiation
                    "binary_numeric_div" => BinaryOperator::Divide,
                    "binary_numeric_minus" => BinaryOperator::Minus,
                    "binary_numeric_mul" => BinaryOperator::Multiply,
                    "binary_numeric_plus" => BinaryOperator::Plus,
                    any => self.panic_unexpected(any),
                };
                self.expect_state("call47_types");
                let types_value_2 = self.handle_types(Some(SubgraphType::Numeric), None).1;
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
                    // "unary_numeric_postfix_fact" => UnaryOperator::PGPostfixFactorial,
                    // "unary_numeric_prefix_fact" => UnaryOperator::PGPrefixFactorial,  // THESE 2 WERE REMOVED FROM POSTGRESQL
                    "unary_numeric_sq_root" => UnaryOperator::PGSquareRoot,
                    any => self.panic_unexpected(any),
                };
                self.expect_state("call1_types");
                let types_value = self.handle_types(Some(SubgraphType::Numeric), None).1;
                Expr::UnaryOp {
                    op: numeric_unary_op,
                    expr: Box::new(types_value)
                }
            },
            "numeric_string_Position" => {
                self.expect_state("call2_types");
                let types_value_1 = self.handle_types(Some(SubgraphType::Text), None).1;
                self.expect_state("string_position_in");
                self.expect_state("call3_types");
                let types_value_2 = self.handle_types(Some(SubgraphType::Text), None).1;
                Expr::Position {
                    expr: Box::new(types_value_1),
                    r#in: Box::new(types_value_2)
                }
            },
            "Nested_numeric" => {
                self.expect_state("call4_types");
                let types_value = self.handle_types(Some(SubgraphType::Numeric), None).1;
                Expr::Nested(Box::new(types_value))
            },
            any => self.panic_unexpected(any)
        };
        self.expect_state("EXIT_numeric");
        (SubgraphType::Numeric, numeric)
    }

    /// subgraph def_text
    fn handle_text(&mut self) -> (SubgraphType, Expr) {
        self.expect_state("text");
        let string = match self.next_state().as_str() {
            "text_literal" => Expr::Value(Value::SingleQuotedString("HJeihfbwei".to_string())),  // TODO: hardcoded
            "text_trim" => {
                let (trim_where, trim_what) = match self.next_state().as_str() {
                    "call6_types" => {
                        let types_value = self.handle_types(Some(SubgraphType::Text), None).1;
                        let spec_mode = match self.next_state().as_str() {
                            "BOTH" => TrimWhereField::Both,
                            "LEADING" => TrimWhereField::Leading,
                            "TRAILING" => TrimWhereField::Trailing,
                            any => self.panic_unexpected(any)
                        };
                        self.expect_state("call5_types");
                        (Some(spec_mode), Some(Box::new(types_value)))
                    },
                    "call5_types" => (None, None),
                    any => self.panic_unexpected(any)
                };
                let types_value = self.handle_types(Some(SubgraphType::Text), None).1;
                Expr::Trim {
                    expr: Box::new(types_value), trim_where, trim_what
                }
            },
            "text_concat" => {
                self.expect_state("call7_types");
                let types_value_1 = self.handle_types(Some(SubgraphType::Text), None).1;
                self.expect_state("text_concat_concat");
                self.expect_state("call8_types");
                let types_value_2 = self.handle_types(Some(SubgraphType::Text), None).1;
                Expr::BinaryOp {
                    left: Box::new(types_value_1),
                    op: BinaryOperator::StringConcat,
                    right: Box::new(types_value_2)
                }
            },
            any => self.panic_unexpected(any)
        };
        self.expect_state("EXIT_text");
        (SubgraphType::Text, string)
    }

    /// subgarph def_date
    fn handle_date(&mut self) -> (SubgraphType, Expr) {
        self.expect_state("date");
        self.expect_state("date_literal");
        let date = Expr::TypedString {
            data_type: DataType::Date,
            value: "2023-08-27".to_string(),
        };
        self.expect_state("EXIT_date");
        (SubgraphType::Date, date)
    }

    /// subgraph def_types
    fn handle_types(
        &mut self, check_generated_by: Option<SubgraphType>, check_compatible_with: Option<SubgraphType>
    ) -> (SubgraphType, Expr) {
        self.expect_state("types");
        match self.next_state().as_str() {
            "types_select_type_3vl" |
            "types_select_type_numeric" |
            "types_select_type_text" |
            "types_select_type_date" => {},
            "types_null" => {
                return (SubgraphType::Undetermined, Expr::Value(Value::Null));
            },
            any => self.panic_unexpected(any),
        };

        let allowed_type_list = self.state_generator
            .get_named_value::<TypesTypeValue>().unwrap().selected_types.clone();

        let (selected_type, types_value) = match self.next_state().as_str() {
            "types_return_typed_null" => {
                let null_type = {
                    let allowed_type_list = allowed_type_list.into_iter()
                        .filter(|x| !x.has_inner()).collect::<Vec<_>>();
                    match allowed_type_list.as_slice() {
                        [tp] => tp.clone(),
                        any => panic!("allowed_type_list must have single element here (got {:?})", any)
                    }
                };
                (null_type.clone(), Expr::Cast {
                    expr: Box::new(Expr::Value(Value::Null)),
                    data_type: null_type.to_data_type(),
                })
            }
            "types_select_special_expression" => {
                match self.next_state().as_str() {
                    "types_column_spec" => {
                        match self.next_state().as_str() {
                            "call0_column_spec" | "call1_column_spec" => {
                                self.state_generator.set_known_list(allowed_type_list);
                                self.handle_column_spec()
                            },
                            any => self.panic_unexpected(any)
                        }
                    }
                    "call1_Query" => {
                        self.state_generator.set_known_list(allowed_type_list);
                        let (subquery, column_types) = self.handle_query();
                        let selected_type = match column_types.len() {
                            1 => column_types[0].1.clone(),
                            any => panic!("Subquery should have selected a single column, but selected {any}"),
                        };
                        (selected_type, Expr::Subquery(Box::new(subquery)))
                    },
                    any => self.panic_unexpected(any)
                }
            },
            "call0_numeric" => self.handle_numeric(),
            "call1_VAL_3" => self.handle_val_3(),
            "call0_text" => self.handle_text(),
            "call0_date" => self.handle_date(),
            any => self.panic_unexpected(any)
        };
        self.expect_state("EXIT_types");

        if let Some(as_what) = check_generated_by {
            if !selected_type.is_same_or_more_determined_or_undetermined(&as_what) {
                self.state_generator.print_stack();
                panic!("Unexpected type: expected {:?}, got {:?}", as_what, selected_type);
            }
        }
        if let Some(with) = check_compatible_with {
            self.expect_compat(&selected_type, &with);
        }
        (selected_type, types_value.nest_children_if_needed())
    }

    /// subgraph def_column_spec
    fn handle_column_spec(&mut self) -> (SubgraphType, Expr) {
        self.expect_state("column_spec");
        self.expect_state("typed_column_name");
        let (selected_type, ident_components) = {
            let column_types = unwrap_variant_or_else!(
                self.state_generator.get_fn_selected_types_unwrapped(), CallTypes::TypeList, || self.state_generator.print_stack()
            );
            self.clause_context.from().get_random_column_with_type_of(&mut self.rng, &column_types)
        };
        self.expect_state("EXIT_column_spec");
        (selected_type, Expr::CompoundIdentifier(ident_components))
    }

    /// subgraph def_list_expr
    fn handle_list_expr(&mut self) -> (SubgraphType, Expr) {
        self.expect_state("list_expr");
        let inner_type = match self.next_state().as_str() {
            "call16_types" => SubgraphType::Numeric,
            "call17_types" => SubgraphType::Val3,
            "call18_types" => SubgraphType::Text,
            "call62_types" => SubgraphType::Date,
            any => self.panic_unexpected(any)
        };
        let (inner_type, types_value) = self.handle_types(Some(inner_type.clone()), None);
        match self.next_state().as_str() {
            "list_expr_multiple_values" => {
                self.state_generator.set_compatible_list(inner_type.get_compat_types());
                let mut list_expr: Vec<Expr> = vec![types_value];
                loop {
                    match self.next_state().as_str() {
                        "call49_types" => {
                            list_expr.push(self.handle_types(None, Some(inner_type.clone())).1);
                        },
                        "EXIT_list_expr" => break,
                        any => self.panic_unexpected(any)
                    }
                }
                (SubgraphType::ListExpr(Box::new(inner_type)), Expr::Tuple(list_expr))
            }
            any => self.panic_unexpected(any)
        }
    }

    /// subgraph adef_aggregate_function
    // TODO: add alias and DISTINCT
    fn handle_aggregate_function(
        &mut self,
        equal_to: Option<SubgraphType>,
        compatible_with: Option<SubgraphType>,
    ) -> Expr {
        self.expect_state("aggregate_function");
        self.expect_state("aggregate_select_return_type");
        let result;
        match self.next_state().as_str() {
            "aggregate_select_type_numeric" => match self.next_state().as_str() {
                arm @ "COUNT" => match self.next_state().as_str() {
                    "COUNT_wildcard" => {
                        match self.next_state().as_str() {
                            arm_2 @ ("EXIT_aggregate" | "COUNT_distinct_wildcard")=> {
                                result = Expr::Function(sqlparser::ast::Function {
                                    name: ObjectName(vec![Ident {
                                        value: arm.to_string(),
                                        quote_style: (None),
                                    }]),
                                    args: vec![FunctionArg::Unnamed(
                                        sqlparser::ast::FunctionArgExpr::Wildcard,
                                    )],
                                    over: None,
                                    distinct: match arm_2 {
                                        "EXIT_aggregate" => false,
                                        "COUNT_distinct_wildcard" => true,
                                        any => self.panic_unexpected(any),
                                    },
                                    special: false,
                                });
                            }
                            any => self.panic_unexpected(any),
                        }
                        //return result;
                    }
                    arm @ ("COUNT_distinct" | "call65_types") => {
                        let count_distinct = match arm {
                            "COUNT_distinct" => {
                                self.expect_state("call65_types");
                                true
                            }
                            "call65_types" => false,
                            any => self.panic_unexpected(any),
                        };
                        result = Expr::Function(sqlparser::ast::Function {
                            name: ObjectName(vec![Ident {
                                value: "COUNT".to_string(),
                                quote_style: (None),
                            }]),
                            args: vec![FunctionArg::Unnamed(
                                sqlparser::ast::FunctionArgExpr::Expr(self.handle_types(None, None).1),
                            )],
                            over: None,
                            distinct: count_distinct,
                            special: false,
                        });
                    }
                    any => self.panic_unexpected(any),
                },
                "arg_double_numeric" => {
                    let chosen_type = &(
                        vec![SubgraphType::Numeric, SubgraphType::Numeric],
                        vec![SubgraphType::Numeric],
                    );
                    let fun_name = self.config.aggregate_functions_distribution.get_fun_name(chosen_type.0.to_owned(), chosen_type.1.to_owned());
                    self.expect_state("call68_types");
                    self.expect_state("call52_types");
                    result = Expr::Function(sqlparser::ast::Function {
                        name: fun_name,
                        args: vec![
                            FunctionArg::Unnamed(sqlparser::ast::FunctionArgExpr::Expr(
                                self.handle_types(Some(SubgraphType::Numeric), None).1,
                            )),
                            FunctionArg::Unnamed(sqlparser::ast::FunctionArgExpr::Expr(
                                self.handle_types(Some(SubgraphType::Numeric), None).1,
                            )),
                        ],
                        over: None,
                        distinct: false,
                        special: false,
                    });
                }
                "arg_single_numeric" => {
                    let chosen_type = &(
                        vec![SubgraphType::Numeric],
                        vec![SubgraphType::Numeric],
                    );
                    let fun_name = self.config.aggregate_functions_distribution.get_fun_name(chosen_type.0.to_owned(), chosen_type.1.to_owned());
                    self.expect_state("call52_types");
                    result = Expr::Function(sqlparser::ast::Function {
                        name: fun_name,
                        args: vec![
                            FunctionArg::Unnamed(sqlparser::ast::FunctionArgExpr::Expr(
                                self.handle_types(Some(SubgraphType::Numeric), None).1,
                            )),
                        ],
                        over: None,
                        distinct: false,
                        special: false,
                    });
                },
                any => self.panic_unexpected(any),
            },
            arm @ ("aggregate_select_type_string" | "aggregate_select_type_bool") => {
                self.expect_state(match arm {
                    "aggregate_select_type_string" => "arg_string", 
                    "aggregate_select_type_bool" => "arg_single_vl3", 
                    any => self.panic_unexpected(any),
                });
                let chosen_type = &(
                    vec![
                        match arm {
                            "aggregate_select_type_string" => SubgraphType::Text, 
                            "aggregate_select_type_bool" => SubgraphType::Val3, 
                            any => self.panic_unexpected(any),
                        }],
                        vec![
                            match arm {
                                "aggregate_select_type_string" => SubgraphType::Text, 
                                "aggregate_select_type_bool" => SubgraphType::Val3, 
                                any => self.panic_unexpected(any),
                            }],
                );
                let fun_name = self.config.aggregate_functions_distribution.get_fun_name(chosen_type.0.to_owned(), chosen_type.1.to_owned());
                self.expect_state(match arm {
                    "aggregate_select_type_string" => "call63_types", 
                    "aggregate_select_type_bool" => "call64_types", 
                    any => self.panic_unexpected(any),
                });
                result = Expr::Function(sqlparser::ast::Function {
                    name: fun_name,
                    args: vec![
                        FunctionArg::Unnamed(sqlparser::ast::FunctionArgExpr::Expr(
                            self.handle_types(Some(
                                match arm {
                                    "aggregate_select_type_string" => SubgraphType::Text, 
                                    "aggregate_select_type_bool" => SubgraphType::Val3, 
                                    any => self.panic_unexpected(any),
                            }), None).1,
                        )),
                    ],
                    over: None,
                    distinct: false,
                    special: false,
                });
            },
            "aggregate_select_type_array" => {
                self.expect_state("array_arg");
                let fun_name = ObjectName(vec![Ident {
                    value: "ARRAY_AGG".to_string(),
                    quote_style: None,
                }]);
                self.expect_state("call50_types");
                result = Expr::Function(sqlparser::ast::Function {
                    name: fun_name,
                    args: vec![
                        FunctionArg::Unnamed(sqlparser::ast::FunctionArgExpr::Expr(
                            self.handle_types(None, None).1,
                        )),
                    ],
                    over: None,
                    distinct: false,
                    special: false,
                });
            },
            any => self.panic_unexpected(any),
        }
        self.expect_state("EXIT_aggregate_function");
        result
    }

    /// subgraph def_group_by
    /// TODO add grouping function
    fn handle_group_by(&mut self) -> Expr {
        self.expect_state("group_by");
        let result;
        let mut arg: Vec<Vec<Expr>> = Vec::new();
        match self.next_state().as_str() {
            "grouping_relations_list" => {
                let mut list : Vec<Expr> = Vec::new();
                loop {
                    match self.next_state().as_str() {
                        "list_of_relations" => {

                        },
                        "call2_column_spec" => {
                            list.push(self.handle_column_spec().1);
                        },
                        "EXIT_group_by" => {
                            result = Expr::Tuple(list);
                            return result;
                        },
                        any => self.panic_unexpected(any),
                    }
                }
            },
            arm @ ("grouping_set" | "grouping_rollup" | "grouping_cube") => {
                //let mut arg: Vec<Vec<Expr>> = Vec::new();
                let mut current_set : Vec<Expr> = Vec::new();
                loop {
                    match self.next_state().as_str() {
                        "set_list" => {
                            if current_set.is_empty() == false {
                                arg.push(current_set);
                            }
                            current_set = Vec::new();
                        },
                        "new_set" => {

                        },
                        "call3_column_spec" => {
                            current_set.push(self.handle_column_spec().1);
                        },
                        "EXIT_group_by" => {
                            result = match arm {
                                "grouping_set" => Expr::GroupingSets(arg),
                                "grouping_rollup" => Expr::Rollup(arg),
                                "grouping_cube" => Expr::Cube(arg),
                                any => self.panic_unexpected(any),
                            };
                            return result;
                        }
                        any => self.panic_unexpected(any),
                    }
                }
            },
            any => self.panic_unexpected(any)
        }
        self.expect_state("EXIT_group_by");
        result
    }
    


    /// starting point; calls handle_query for the first time
    fn generate(&mut self) -> Query {
        let query = self.handle_query().0;
        if self.config.print_queries {
            println!("\n{};\n", query);
        }
        self.free_projection_alias_index = 1;
        // reset the generator
        if let Some(state) = self.next_state_opt() {
            panic!("Couldn't reset state_generator: Received {state}");
        }
        self.dynamic_model = Box::new(DynMod::new());
        query
    }
}

impl<DynMod: DynamicModel, StC: StateChooser> Iterator for QueryGenerator<DynMod, StC> {
    type Item = Query;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.generate())
    }
}
