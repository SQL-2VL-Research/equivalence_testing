#[macro_use]
mod query_info;

use smol_str::SmolStr;
use sqlparser::ast::{
    Expr, Ident, Query, Select, SelectItem, SetExpr, TableAlias, TableFactor,
    TableWithJoins, Value, BinaryOperator, UnaryOperator, TrimWhereField, Array,
};

use self::query_info::{QueryInfo, Variable, TypesSelectedType};

use super::state_generators::{MarkovChainGenerator, FunctionInputsType};

pub struct QueryGenerator {
    state_generator: MarkovChainGenerator,
}

/// query_info.rs
pub enum Variable {
    SelectLimit(Option<Expr>),
    SelectBody(Select),
    Query(Query),
    LastRelation(TableFactor),
    Val3(Expr),
    IsNullNotFlag(bool),
    TypesValue(Expr),
    IsDistinctNotFlag(bool),
    TypesSelectedType(TypesSelectedType),
    ExistsNotFlag(bool),
    InListNotFlag(bool),
    InSubqueryNotFlag(bool),
    BetweenNotFlag(bool),
    BinaryCompOp(BinaryOperator),
    AnyAllOp(BinaryOperator),
    Array(Expr),
    StringLikeNotFlag(bool),
    BinaryBoolOp(BinaryOperator),
    Numeric(Expr),
    NumericBinaryOp(BinaryOperator),
    NumericUnaryOp(UnaryOperator),
    String(Expr),
    TrimSpecFlag(bool),
    TrimSpecValue(TrimWhereField),
    ColumnSpec(Expr),
    ListExpr(Expr),
}

/// query_info.rs
pub struct QueryInfo {
    /// used to generate and store info about
    /// all available relations
    pub relation_generator: RelationGenerator,
}

/// query_info.rs
impl QueryInfo {
    pub fn new() -> QueryInfo {
        QueryInfo {
            relation_generator: RelationGenerator::new(),
        }
    }
}

/// query_info.rs
pub struct QueryInfo {
    /// used to generate and store info about
    /// all available relations
    pub relation_generator: RelationGenerator,
    /// used to store running query statistics, such as
    /// the current level of nesting
    pub stats: QueryStats,
    /// variables needed during query generation
    pub named_var_stack: HashMap<String, Vec<Variable>>,
}

/// query_info.rs
/// push value into its stack
macro_rules! push_var {
    ($info: expr, $variant: ident, $value: expr) => {
        if let Some(stack) = $info.named_var_stack.get_mut(stringify!($variant)) {
            stack.push(Variable::$variant($value));
        } else {
            $info.named_var_stack.insert(stringify!($variant).to_string(), vec![Variable::$variant($value)]);
        }
    };
}

/// query_info.rs
/// pop value from its stack
macro_rules! pop_var {
    ($info: expr, $variant: ident) => {
        if let Some(stack) = $info.named_var_stack.get_mut(stringify!($variant)) {
            if let Some(Variable::$variant(value)) = stack.pop() {
                value
            } else {
                panic!("Failed to pop variant: {}", stringify!($variant));
            }
        } else {
            panic!("Failed to find variant stack: {}", stringify!($variant));
        }
    };
}

/// query_info.rs
/// return mutable reference to the last value in its stack
macro_rules! get_mut_var {
    ($info: expr, $variant: ident) => {
        if let Some(stack) = $info.named_var_stack.get_mut(stringify!($variant)) {
            if let Some(Variable::$variant(value)) = stack.last_mut() {
                value
            } else {
                panic!("Failed to get last mut variant: {}", stringify!($variant));
            }
        } else {
            panic!("Failed to find variant stack: {}", stringify!($variant));
        }
    };
}

impl QueryGenerator {
    pub fn from_state_generator(state_generator: MarkovChainGenerator) -> Self {
        QueryGenerator {
            state_generator,
        }
    }

    fn next_state(&mut self) -> SmolStr {
        self.state_generator.next().unwrap()
    }

    fn panic_unexpected(&mut self, state: &str, after: &str) {
        panic!("Unexpected {state} after {after}");
    }

    fn expect_state(&mut self, state: &str, after: &str) {
        let new_state = self.next_state();
        if new_state.as_str() != state {
            panic!("Expected {state} after {after}. Got {}", new_state)
        }
    }

    // ======================================================== subgraph Query

    fn handle_query(&mut self, info: &mut QueryInfo) {
        let mut limit = Option::<Expr>::None;
        if let Some(mods) = self.state_generator.get_modifiers() {
            if mods.contains(&SmolStr::new("single value")) {
                limit = Some(Expr::Identifier(Ident::new("1")));
                // TODO: Not only limits can enforce this
            } else {
                panic!("Unexpected mods (Query): {:?}", mods);
            }
        }
        if let FunctionInputsType::TypeName(type_name) = self.state_generator.get_inputs() {
            println!("TODO: Enforce single column & column type (Query): {type_name}")
        }

        push_var!(info, SelectLimit, limit);
        push_var!(info, SelectBody, Select {
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
        });

        self.expect_state("FROM", "Query");
    }

    fn handle_query_exit(&mut self, info: &mut QueryInfo) {
        let query = Query {
            with: None,
            body: SetExpr::Select(Box::new(pop_var!(info, SelectBody))),
            order_by: vec![],
            limit: pop_var!(info, SelectLimit),
            offset: None,
            fetch: None,
            lock: None,
        };
        push_var!(info, Query, query);
    }

    fn handle_from_table(&mut self, info: &mut QueryInfo) {
        let last_relation = TableFactor::Table {
            name: info.relation_generator.new_relation().gen_object_name(),
            alias: None,
            args: None,
            with_hints: vec![]
        };
        push_var!(info, LastRelation, last_relation);
    }

    fn handle_from_subquery_r(&mut self, info: &mut QueryInfo) {
        let last_relation = TableFactor::Derived {
            lateral: false,
            subquery: Box::new(pop_var!(info, Query)),
            alias: Some(TableAlias {
                name: info.relation_generator.new_ident(),
                columns: vec![],
            })
        };
        push_var!(info, LastRelation, last_relation);
    }

    fn handle_multiple_relations(&mut self, info: &mut QueryInfo) {
        let relation = pop_var!(info, LastRelation);
        let body = get_mut_var!(info, SelectBody);
        body.from.push(TableWithJoins { relation, joins: vec![] });
    }

    fn handle_select_all(&mut self, info: &mut QueryInfo) {
        let body = get_mut_var!(info, SelectBody);
        body.projection.push(SelectItem::Wildcard);
    }

    fn handle_where_val_3_ret(&mut self, info: &mut QueryInfo) {
        let val_3 = pop_var!(info, Val3);
        let body = get_mut_var!(info, SelectBody);
        body.selection = Some(val_3);
    }

    // ======================================================== subgraph def_VAL_3

    fn handle_val_3(&mut self, info: &mut QueryInfo) {
        match self.next_state().as_str() {
            "IsNull" => self.handle_is_null(info),
            "IsDistinctFrom" => self.handle_is_distinct_from(info),
            "Exists" => self.handle_exists(info),
            "InList" => self.handle_in_list(info),
            "InSubquery" => self.handle_in_subquery(info),
            "Between" => self.handle_between(info),
            "BinaryComp" => self.handle_binary_comp(info),
            "AnyAll" => self.handle_any_all(info),
            "BinaryStringLike" => self.handle_string_like(info),
            "BinaryBooleanOpV3" => self.handle_binary_bool_op(info),
            "true" => push_var!(info, Val3, Expr::Value(Value::Boolean(true))),
            "false" => push_var!(info, Val3, Expr::Value(Value::Boolean(false))),
            "Nested_VAL_3" => self.handle_nested_val(info),
            "UnaryNot_VAL_3" => self.expect_state("call30_types", "UnaryNot_VAL_3"),
            any => self.panic_unexpected(any, "VAL_3")
        }
    }

    fn handle_is_null(&mut self, info: &mut QueryInfo) {
        match self.next_state().as_str() {
            "IsNull_not" => {
                push_var!(info, IsNullNotFlag, true);
                self.expect_state("call0_types_all", "IsNull_not");
            }
            "call0_types_all" => {
                push_var!(info, IsNullNotFlag, false);
            },
            any => self.panic_unexpected(any, "IsNull")
        }
    }

    fn handle_is_null_value(&mut self, info: &mut QueryInfo) {
        let types_all_ret = pop_var!(info, TypesValue);
        let is_not_null = pop_var!(info, IsNullNotFlag);
        let val_3 = if is_not_null {
            Expr::IsNotNull(Box::new(types_all_ret))
        } else {
            Expr::IsNull(Box::new(types_all_ret))
        };
        push_var!(info, Val3, val_3);
    }

    fn handle_is_distinct_from(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call1_types_all", "IsDistinctFrom");
    }

    fn handle_is_distinct_from_1st_val(&mut self, info: &mut QueryInfo) {
        self.state_generator.compatible_type_name_stack.push(
            pop_var!(info, TypesSelectedType).get_types()
        );
        match self.next_state().as_str() {
            "IsDistinctNOT" => {
                push_var!(info, IsDistinctNotFlag, true);
                self.expect_state("DISTINCT", "IsDistinctNOT");
            }
            "DISTINCT" => {
                push_var!(info, IsDistinctNotFlag, false);
            },
            any => self.panic_unexpected(any, "call1_types_all")
        }
        self.expect_state("call21_types", "DISTINCT");
    }

    fn handle_is_distinct_from_2nd_val(&mut self, info: &mut QueryInfo) {
        let val2 = Box::new(pop_var!(info, TypesValue));
        let val1 = Box::new(pop_var!(info, TypesValue));
        let is_not_distinct = pop_var!(info, IsDistinctNotFlag);

        let val_3 = if is_not_distinct {
            Expr::IsNotDistinctFrom(val1, val2)
        } else {
            Expr::IsDistinctFrom(val1, val2)
        };

        push_var!(info, Val3, val_3);
    }

    fn handle_exists(&mut self, info: &mut QueryInfo) {
        match self.next_state().as_str() {
            "Exists_not" => {
                push_var!(info, ExistsNotFlag, true);
                self.expect_state("call2_Query", "Exists_not");
            },
            "call2_Query" => {
                push_var!(info, ExistsNotFlag, false);
            },
            any => self.panic_unexpected(any, "Exists")
        }
    }

    fn handle_exists_val(&mut self, info: &mut QueryInfo) {
        let val_3 = Expr::Exists {
            subquery: Box::new(pop_var!(info, Query)),
            negated: pop_var!(info, ExistsNotFlag)
        };

        push_var!(info, Val3, val_3);
    }

    fn handle_in_list(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call2_types_all", "InList");
    }

    fn handle_in_list_1st_val(&mut self, info: &mut QueryInfo) {
        self.state_generator.compatible_type_name_stack.push(
            pop_var!(info, TypesSelectedType).get_types()
        );
        match self.next_state().as_str() {
            "InListNot" => {
                push_var!(info, InListNotFlag, true);
                self.expect_state("InListIn", "InListNot");
                self.expect_state("call1_list_expr", "InListIn");
            },
            "InListIn" => {
                push_var!(info, InListNotFlag, false);
                self.expect_state("call1_list_expr", "InListIn");
            },
            any => self.panic_unexpected(any, "call2_types_all")
        }
    }

    fn handle_in_list_2nd_val(&mut self, info: &mut QueryInfo) {
        let val_3 = Expr::InList {
            expr: Box::new(pop_var!(info, TypesValue)),
            list: vec![
                Expr::Value(Value::SingleQuotedString("5".to_string())),
                Expr::Value(Value::SingleQuotedString("erf".to_string())),
                Expr::Value(Value::SingleQuotedString("vvv".to_string())),
            ], // TODO: HARDCODE 
            negated: pop_var!(info, InListNotFlag)
        };

        push_var!(info, Val3, val_3);
    }

    fn handle_in_subquery(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call3_types_all", "InSubquery");
    }

    fn handle_in_subquery_1st_val(&mut self, info: &mut QueryInfo) {
        self.state_generator.compatible_type_name_stack.push(
            pop_var!(info, TypesSelectedType).get_types()
        );
        match self.next_state().as_str() {
            "InSubqueryNot" => {
                push_var!(info, InSubqueryNotFlag, true);
                self.expect_state("InSubqueryIn", "InSubqueryNot");
            },
            "InSubqueryIn" => {
                push_var!(info, InSubqueryNotFlag, false);
            },
            any => self.panic_unexpected(any, "call2_types_all")
        }
        self.expect_state("call3_Query", "InSubqueryIn");
    }

    fn handle_in_subquery_2nd_val(&mut self, info: &mut QueryInfo) {
        let val_3 = Expr::InSubquery {
            expr: Box::new(pop_var!(info, TypesValue)),
            subquery: Box::new(pop_var!(info, Query)),
            negated: pop_var!(info, InSubqueryNotFlag)
        };

        push_var!(info, Val3, val_3);
    }

    fn handle_between(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call4_types_all", "Between");
    }

    fn handle_between_1st_val(&mut self, info: &mut QueryInfo) {
        let types_name = pop_var!(info, TypesSelectedType).get_types();
        self.state_generator.compatible_type_name_stack.push(types_name.clone());
        self.state_generator.compatible_type_name_stack.push(types_name);
        match self.next_state().as_str() {
            "BetweenBetweenNot" => {
                push_var!(info, BetweenNotFlag, true);
                self.expect_state("BetweenBetween", "BetweenBetweenNot");
            },
            "BetweenBetween" => {
                push_var!(info, BetweenNotFlag, false);
            },
            any => self.panic_unexpected(any, "call4_types_all")
        }
        self.expect_state("call22_types", "BetweenBetween");
    }

    fn handle_between_2nd_val(&mut self, _info: &mut QueryInfo) {
        self.expect_state("BetweenBetweenAnd", "call22_types");
        self.expect_state("call23_types", "BetweenBetweenAnd");
    }

    fn handle_between_3rd_val(&mut self, info: &mut QueryInfo) {
        let high = Box::new(pop_var!(info, TypesValue));
        let low = Box::new(pop_var!(info, TypesValue));
        let expr = Box::new(pop_var!(info, TypesValue));
        let negated = pop_var!(info, BetweenNotFlag);
        let val_3 = Expr::Between { expr, negated, low, high };

        push_var!(info, Val3, val_3);
    }

    fn handle_binary_comp(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call5_types_all", "BinaryComp");
    }

    fn handle_binary_comp_1st_val(&mut self, info: &mut QueryInfo) {
        self.state_generator.compatible_type_name_stack.push(
            pop_var!(info, TypesSelectedType).get_types()
        );
        match self.next_state().as_str() {
            "BinaryCompEqual" => {
                push_var!(info, BinaryCompOp, BinaryOperator::Eq);
                self.expect_state("call24_types", "BinaryCompEqual");
            },
            "BinaryCompLess" => {
                push_var!(info, BinaryCompOp, BinaryOperator::Lt);
                self.expect_state("call24_types", "BinaryCompLess");
            },
            "BinaryCompLessEqual" => {
                push_var!(info, BinaryCompOp, BinaryOperator::LtEq);
                self.expect_state("call24_types", "BinaryCompLessEqual");
            },
            "BinaryCompUnEqual" => {
                push_var!(info, BinaryCompOp, BinaryOperator::NotEq);
                self.expect_state("call24_types", "BinaryCompUnEqual");
            },
            any => self.panic_unexpected(any, "call5_types_all")
        }
    }

    fn handle_binary_comp_2nd_val(&mut self, info: &mut QueryInfo) {
        let right = Box::new(pop_var!(info, TypesValue));
        let op = pop_var!(info, BinaryCompOp);
        let left = Box::new(pop_var!(info, TypesValue));

        push_var!(info, Val3, Expr::BinaryOp { left, op, right });
    }

    fn handle_any_all(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call6_types_all", "AnyAll");
    }

    fn handle_any_all_1st_val(&mut self, info: &mut QueryInfo) {
        self.state_generator.compatible_type_name_stack.push(
            pop_var!(info, TypesSelectedType).get_types()
        );
        self.expect_state("AnyAllSelectOp", "call6_types_all");

        match self.next_state().as_str() {
            "AnyAllEqual" => {
                push_var!(info, AnyAllOp, BinaryOperator::Eq);
                self.expect_state("AnyAllSelectIter", "AnyAllEqual");
            },
            "AnyAllLess" => {
                push_var!(info, AnyAllOp, BinaryOperator::Lt);
                self.expect_state("AnyAllSelectIter", "AnyAllLess");
            },
            "AnyAllLessEqual" => {
                push_var!(info, AnyAllOp, BinaryOperator::LtEq);
                self.expect_state("AnyAllSelectIter", "AnyAllLessEqual");
            },
            "AnyAllUnEqual" => {
                push_var!(info, AnyAllOp, BinaryOperator::NotEq);
                self.expect_state("AnyAllSelectIter", "AnyAllUnEqual");
            },
            any => self.panic_unexpected(any, "call6_types_all")
        }

        match self.next_state().as_str() {
            "call4_Query" => {},
            "call1_array" => {},
            any => self.panic_unexpected(any, "AnyAllSelectIter")
        }
    }

    fn handle_any_all_2nd_val(&mut self, info: &mut QueryInfo, array: bool) {
        self.expect_state("AnyAllAnyAll", if array { "call1_array" } else { "call4_Query" });

        let right = Box::new(if array {
            pop_var!(info, Array)
        } else {
            Expr::Subquery(Box::new(pop_var!(info, Query)))
        });
        let right = Box::new(match self.next_state().as_str() {
            "AnyAllAnyAllAll" => Expr::AllOp(right),
            "AnyAllAnyAllAny" => Expr::AnyOp(right),
            any => {
                self.panic_unexpected(any, "AnyAllAnyAll");
                panic!()  // dumb compiler errors :(
            }
        });

        let op = pop_var!(info, AnyAllOp);
        let left = Box::new(pop_var!(info, TypesValue));

        push_var!(info, Val3, Expr::BinaryOp { left, op, right });
    }

    fn handle_string_like(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call25_types", "BinaryStringLike");
    }

    fn handle_string_like_1st_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::String {
            panic!("String LIKE expected String as 1st argument")
        }
        match self.next_state().as_str() {
            "BinaryStringLikeNot" => {
                push_var!(info, StringLikeNotFlag, true);
                self.expect_state("BinaryStringLikeIn", "BinaryStringLikeNot");
            }
            "BinaryStringLikeIn" => {
                push_var!(info, StringLikeNotFlag, false);
            },
            any => self.panic_unexpected(any, "call25_types")
        }
        self.expect_state("call26_types", "BinaryStringLikeIn");
    }

    fn handle_string_like_2nd_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::String {
            panic!("String LIKE expected String as 2nd argument")
        }
        let right = Box::new(pop_var!(info, TypesValue));
        let left = Box::new(pop_var!(info, TypesValue));
        let op = if pop_var!(info, StringLikeNotFlag) { BinaryOperator::NotLike } else { BinaryOperator::Like };

        push_var!(info, Val3, Expr::BinaryOp { left, op, right });
    }

    fn handle_binary_bool_op(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call27_types", "BinaryBooleanOpV3");
    }

    fn handle_binary_bool_op_1st_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::Val3 {
            panic!("Boolean op expected TypesSelectedType::Val3 as 1st argument")
        }
        match self.next_state().as_str() {
            "BinaryBooleanOpV3AND" => {
                push_var!(info, BinaryBoolOp, BinaryOperator::And);
                self.expect_state("call28_types", "BinaryBooleanOpV3AND");
            },
            "BinaryBooleanOpV3OR" => {
                push_var!(info, BinaryBoolOp, BinaryOperator::Or);
                self.expect_state("call28_types", "BinaryBooleanOpV3OR");
            },
            "BinaryBooleanOpV3XOR" => {
                push_var!(info, BinaryBoolOp, BinaryOperator::Xor);
                self.expect_state("call28_types", "BinaryBooleanOpV3XOR");
            },
            any => self.panic_unexpected(any, "call27_types")
        }
    }

    fn handle_binary_bool_op_2nd_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::Val3 {
            panic!("Boolean op expected TypesSelectedType::Val3 as 2nd argument")
        }
        let right = Box::new(pop_var!(info, TypesValue));
        let op = pop_var!(info, BinaryBoolOp);
        let left = Box::new(pop_var!(info, TypesValue));

        push_var!(info, Val3, Expr::BinaryOp { left, op, right });
    }

    fn handle_nested_val(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call29_types", "Nested_VAL_3");
    }

    fn handle_nested_val_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::Val3 {
            panic!("Nested_VAL_3 expected TypesSelectedType::Val3 as argument")
        }
        let nested = Box::new(pop_var!(info, TypesValue));
        push_var!(info, Val3, Expr::Nested(nested));
    }

    fn handle_unary_not_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::Val3 {
            panic!("Nested_VAL_3 expected TypesSelectedType::Val3 as argument")
        }
        let expr = Box::new(pop_var!(info, TypesValue));
        push_var!(info, Val3, Expr::UnaryOp { op: UnaryOperator::Not, expr });
    }

    // ======================================================== subgraph def_numeric

    fn handle_numeric(&mut self, info: &mut QueryInfo) {
        match self.next_state().as_str() {
            "numeric_literal" => self.handle_numeric_literal(info),
            "BinaryNumericOp" => self.handle_numeric_binary(info),
            "UnaryNumericOp" => self.handle_numeric_unary(info),
            "numeric_string_Position" => self.handle_string_position(info),
            "Nested_numeric" => self.handle_numeric_nested_val(info),
            any => self.panic_unexpected(any, "numeric")
        }
    }

    fn handle_numeric_literal(&mut self, info: &mut QueryInfo) {
        let num_str = match self.next_state().as_str() {
            "numeric_literal_float" => {
                "3.1415"  // TODO: hardcode
            },
            "numeric_literal_int" => {
                "3"       // TODO: hardcode
            },
            any => {
                self.panic_unexpected(any, "numeric_literal");
                panic!() // compiler dumb
            }
        }.to_string();
        push_var!(info, Numeric, Expr::Value(Value::Number(num_str, false)));
    }

    fn handle_numeric_binary(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call33_types", "BinaryNumericOp");
    }

    fn handle_numeric_binary_1st_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::Numeric {
            panic!("BinaryNumericOp expected TypesSelectedType::Numeric as argument")
        }
        match self.next_state().as_str() {
            "binary_numeric_bin_and" => {
                push_var!(info, NumericBinaryOp, BinaryOperator::BitwiseAnd);
                self.expect_state("call32_types", "binary_numeric_bin_and")
            },
            "binary_numeric_bin_or" => {
                push_var!(info, NumericBinaryOp, BinaryOperator::BitwiseOr);
                self.expect_state("call32_types", "binary_numeric_bin_or")
            },
            "binary_numeric_bin_xor" => {
                push_var!(info, NumericBinaryOp, BinaryOperator::BitwiseXor);
                self.expect_state("call32_types", "binary_numeric_bin_xor")
            },
            "binary_numeric_div" => {
                push_var!(info, NumericBinaryOp, BinaryOperator::Divide);
                self.expect_state("call32_types", "binary_numeric_div")
            },
            "binary_numeric_minus" => {
                push_var!(info, NumericBinaryOp, BinaryOperator::Minus);
                self.expect_state("call32_types", "binary_numeric_minus")
            },
            "binary_numeric_mul" => {
                push_var!(info, NumericBinaryOp, BinaryOperator::Multiply);
                self.expect_state("call32_types", "binary_numeric_mul")
            },
            "binary_numeric_plus" => {
                push_var!(info, NumericBinaryOp, BinaryOperator::Plus);
                self.expect_state("call32_types", "binary_numeric_plus")
            },
            any => self.panic_unexpected(any, "call33_types"),
        }
    }

    fn handle_numeric_binary_2nd_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::Numeric {
            panic!("BinaryNumericOp expected TypesSelectedType::Numeric as argument")
        }
        let right = Box::new(pop_var!(info, TypesValue));
        let op = pop_var!(info, NumericBinaryOp);
        let left = Box::new(pop_var!(info, TypesValue));

        push_var!(info, Numeric, Expr::BinaryOp { left, op, right });
    }

    fn handle_numeric_unary(&mut self, info: &mut QueryInfo) {
        match self.next_state().as_str() {
            "unary_numeric_abs" => {
                push_var!(info, NumericUnaryOp, UnaryOperator::PGAbs);
                self.expect_state("call1_types", "unary_numeric_abs");
            },
            "unary_numeric_bin_not" => {
                push_var!(info, NumericUnaryOp, UnaryOperator::PGBitwiseNot);
                self.expect_state("call1_types", "unary_numeric_bin_not");
            },
            "unary_numeric_cub_root" => {
                push_var!(info, NumericUnaryOp, UnaryOperator::PGCubeRoot);
                self.expect_state("call1_types", "unary_numeric_cub_root");
            },
            "unary_numeric_minus" => {
                push_var!(info, NumericUnaryOp, UnaryOperator::Minus);
                self.expect_state("call1_types", "unary_numeric_minus");
            },
            "unary_numeric_plus" => {
                push_var!(info, NumericUnaryOp, UnaryOperator::Plus);
                self.expect_state("call1_types", "unary_numeric_plus");
            },
            "unary_numeric_postfix_fact" => {
                push_var!(info, NumericUnaryOp, UnaryOperator::PGPostfixFactorial);
                self.expect_state("call1_types", "unary_numeric_postfix_fact");
            },
            "unary_numeric_prefix_fact" => {
                push_var!(info, NumericUnaryOp, UnaryOperator::PGPrefixFactorial);
                self.expect_state("call1_types", "unary_numeric_prefix_fact");
            },
            "unary_numeric_sq_root" => {
                push_var!(info, NumericUnaryOp, UnaryOperator::PGSquareRoot);
                self.expect_state("call1_types", "unary_numeric_sq_root");
            },
            any => self.panic_unexpected(any, "call33_types"),
        }
    }

    fn handle_numeric_unary_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::Numeric {
            panic!("UnaryNumericOp expected TypesSelectedType::Numeric as argument")
        }
        let op = pop_var!(info, NumericUnaryOp);
        let expr = Box::new(pop_var!(info, TypesValue));
        push_var!(info, Numeric, Expr::UnaryOp { op, expr });
    }

    fn handle_string_position(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call2_types", "numeric_string_Position");
    }

    fn handle_string_position_1st_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::String {
            panic!("numeric_string_Position expected TypesSelectedType::String as 1st argument")
        }
        self.expect_state("string_position_in", "call2_types");
        self.expect_state("call3_types", "string_position_in");
    }

    fn handle_string_position_2nd_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::String {
            panic!("numeric_string_Position expected TypesSelectedType::String as 2nd argument")
        }
        let right = Box::new(pop_var!(info, TypesValue));
        let left = Box::new(pop_var!(info, TypesValue));
        push_var!(info, Numeric, Expr::Position { expr: left, r#in: right });
    }

    fn handle_numeric_nested_val(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call4_types", "Nested_numeric");
    }

    fn handle_numeric_nested_val_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::Numeric {
            panic!("Nested_numeric expected TypesSelectedType::Numeric as argument")
        }
        let nested = Box::new(pop_var!(info, TypesValue));
        push_var!(info, Numeric, Expr::Nested(nested));
    }

    // ======================================================== subgraph def_string

    fn handle_string(&mut self, info: &mut QueryInfo) {
        match self.next_state().as_str() {
            "string_literal" => push_var!(info, String, Expr::Value(Value::SingleQuotedString("HJeihfbwei".to_string()))),  // TODO: hardcoded
            "string_trim" => self.handle_trim(info),
            "string_concat" => self.handle_concat(info),
            any => self.panic_unexpected(any, "string")
        }
    }

    fn handle_trim(&mut self, info: &mut QueryInfo) {
        match self.next_state().as_str() {
            "call6_types" => { push_var!(info, TrimSpecFlag, true); },
            "call5_types" => { push_var!(info, TrimSpecFlag, false); },
            any => self.panic_unexpected(any, "string_trim")
        }
    }

    fn handle_trim_spec(&mut self, info: &mut QueryInfo) {
        match self.next_state().as_str() {
            "BOTH" => { 
                push_var!(info, TrimSpecValue, TrimWhereField::Both);
                self.expect_state("call5_types", "BOTH");
            },
            "LEADING" => {
                push_var!(info, TrimSpecValue, TrimWhereField::Leading);
                self.expect_state("call5_types", "LEADING");
            },
            "TRAILING" => {
                push_var!(info, TrimSpecValue, TrimWhereField::Trailing);
                self.expect_state("call5_types", "TRAILING");
            },
            any => self.panic_unexpected(any, "string_trim")
        }
    }

    fn handle_trim_val(&mut self, info: &mut QueryInfo) {
        let is_spec = pop_var!(info, TrimSpecFlag);

        if pop_var!(info, TypesSelectedType) != TypesSelectedType::String {
            panic!("trim expected TypesSelectedType::String as expr")
        }
        let expr = Box::new(pop_var!(info, TypesValue));

        let trim_where = if is_spec {
            if pop_var!(info, TypesSelectedType) != TypesSelectedType::String {
                panic!("trim expected TypesSelectedType::String as spec")
            }
            let field = pop_var!(info, TrimSpecValue);
            let spec = Box::new(pop_var!(info, TypesValue));
            Some((field, spec))
        } else {
            None
        };
        push_var!(info, String, Expr::Trim {
            expr, trim_where
        })
    }

    fn handle_concat(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call7_types", "string_concat");
    }

    fn handle_concat_1st_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::String {
            panic!("string_concat expected TypesSelectedType::String as 1st value")
        }
        self.expect_state("string_concat_concat", "call7_types");
        self.expect_state("call8_types", "string_concat_concat");
    }

    fn handle_concat_2nd_val(&mut self, info: &mut QueryInfo) {
        if pop_var!(info, TypesSelectedType) != TypesSelectedType::String {
            panic!("string_concat expected TypesSelectedType::String as 2nd value")
        }
        let right = Box::new(pop_var!(info, TypesValue));
        let left = Box::new(pop_var!(info, TypesValue));
        push_var!(info, String, Expr::BinaryOp { left, op: BinaryOperator::StringConcat, right }  )
    }

    // ======================================================== subgraph def_types

    fn handle_types(&mut self, info: &mut QueryInfo) {
        match self.next_state().as_str() {
            "types_select_type" => self.handle_types_select_type(info),
            "types_null" => {
                push_var!(info, TypesSelectedType, TypesSelectedType::Any);
                push_var!(info, TypesValue, Expr::Value(Value::Null));
            },
            "call0_numeric" => {},
            "call1_VAL_3" => {},
            "call0_string" => {},
            "call0_list_expr" => {
                self.state_generator.known_type_name_stack.push(match self.state_generator.get_inputs() {
                    FunctionInputsType::TypeNameList(list) => list,
                    _ => panic!("Couldn't replicate inputs")
                })
            },
            "call0_array" => {
                self.state_generator.known_type_name_stack.push(match self.state_generator.get_inputs() {
                    FunctionInputsType::TypeNameList(list) => list,
                    _ => panic!("Couldn't replicate inputs")
                })
            },
            any => self.panic_unexpected(any, "types")
        }
    }

    fn handle_types_select_type(&mut self, info: &mut QueryInfo) {
        match self.next_state().as_str() {
            "types_select_type_3vl" => {
                push_var!(info, TypesSelectedType, TypesSelectedType::Val3);
                self.state_generator.known_type_name_stack.push((TypesSelectedType::Val3).get_types());
                self.expect_state("types_select_type_end", "types_select_type_3vl");
            },
            "types_select_type_array" => {
                push_var!(info, TypesSelectedType, TypesSelectedType::Array);
                self.state_generator.known_type_name_stack.push((TypesSelectedType::Array).get_types());
                self.expect_state("types_select_type_end", "types_select_type_array");
            },
            "types_select_type_list_expr" => {
                push_var!(info, TypesSelectedType, TypesSelectedType::ListExpr);
                self.state_generator.known_type_name_stack.push((TypesSelectedType::ListExpr).get_types());
                self.expect_state("types_select_type_end", "types_select_type_list_expr");
            },
            "types_select_type_numeric" => {
                push_var!(info, TypesSelectedType, TypesSelectedType::Numeric);
                self.state_generator.known_type_name_stack.push((TypesSelectedType::Numeric).get_types());
                self.expect_state("types_select_type_end", "types_select_type_numeric");
            },
            "types_select_type_string" => {
                push_var!(info, TypesSelectedType, TypesSelectedType::String);
                self.state_generator.known_type_name_stack.push((TypesSelectedType::String).get_types());
                self.expect_state("types_select_type_end", "types_select_type_string");
            },
            any => self.panic_unexpected(any, "types_select_type")
        }

        match self.next_state().as_str() {
            "call0_column_spec" => {},
            "call1_Query" => {},
            any => self.panic_unexpected(any, "types_select_type_end")
        }
    }

    fn handle_types_select_type_value(&mut self, info: &mut QueryInfo, query: bool) {
        let value = if query {
            Expr::Subquery(Box::new(pop_var!(info, Query)))
        } else {
            pop_var!(info, ColumnSpec)
        };
        push_var!(info, TypesValue, value);
    }

    fn handle_types_numeric(&mut self, info: &mut QueryInfo) {
        push_var!(info, TypesSelectedType, TypesSelectedType::Numeric);
        let val = pop_var!(info, Numeric);
        push_var!(info, TypesValue, val);
    }

    fn handle_types_val3(&mut self, info: &mut QueryInfo) {
        push_var!(info, TypesSelectedType, TypesSelectedType::Val3);
        let val = pop_var!(info, Val3);
        push_var!(info, TypesValue, val);
    }

    fn handle_types_string(&mut self, info: &mut QueryInfo) {
        push_var!(info, TypesSelectedType, TypesSelectedType::String);
        let val = pop_var!(info, String);
        push_var!(info, TypesValue, val);
    }

    fn handle_types_list_expr(&mut self, info: &mut QueryInfo) {
        push_var!(info, TypesSelectedType, TypesSelectedType::ListExpr);
        let val = pop_var!(info, ListExpr);
        push_var!(info, TypesValue, val);
    }

    fn handle_types_array(&mut self, info: &mut QueryInfo) {
        push_var!(info, TypesSelectedType, TypesSelectedType::Array);
        let val = pop_var!(info, Array);
        push_var!(info, TypesValue, val);
    }

    // ======================================================== subgraph def_types_all

    fn handle_types_all(&mut self, _info: &mut QueryInfo) {
        self.expect_state("call0_types", "types_all");
    }

    // ======================================================== subgraph def_column_spec

    fn handle_column_spec(&mut self, info: &mut QueryInfo) {
        match self.next_state().as_str() {
            "qualified_name" => {
                push_var!(info, ColumnSpec, Expr::Identifier(info.relation_generator.new_relation().gen_column_ident()));
            },
            "unqualified_name" => {
                push_var!(info, ColumnSpec, Expr::CompoundIdentifier(vec![
                    info.relation_generator.new_relation().gen_column_ident(),
                    info.relation_generator.new_relation().gen_column_ident()
                ]));
            },
            any => self.panic_unexpected(any, "types_select_type")
        }
    }

    // ======================================================== subgraph def_array

    fn handle_array(&mut self, info: &mut QueryInfo) {
        // TODO: HARDCODE !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
        match self.next_state().as_str() {
            "call12_types" => {
                push_var!(info, Array, Expr::Array(Array { elem: vec![
                    Expr::Value(Value::Number("5".to_string(), false)),
                    Expr::Value(Value::Number("64".to_string(), false)),
                    Expr::Value(Value::Number("2".to_string(), false))
                ], named: false }));
            },
            "call13_types" => {
                push_var!(info, Array, Expr::Array(Array { elem: vec![
                    Expr::Value(Value::Boolean(false)),
                    Expr::Value(Value::Boolean(false)),
                    Expr::Value(Value::Boolean(true))
                ], named: false }));
            },
            "call31_types" => {
                push_var!(info, Array, Expr::Array(Array { elem: vec![
                    Expr::Value(Value::SingleQuotedString("5".to_string())),
                    Expr::Value(Value::SingleQuotedString("erf".to_string())),
                    Expr::Value(Value::SingleQuotedString("vvv".to_string())),
                ], named: false }));
            },
            "call14_types" => {
                push_var!(info, Array, Expr::Array(Array { elem: vec![
                    Expr::Array(Array { elem: vec![
                        Expr::Value(Value::SingleQuotedString("5".to_string())),
                        Expr::Value(Value::SingleQuotedString("erf".to_string())),
                        Expr::Value(Value::SingleQuotedString("vvv".to_string())),
                    ], named: false }),
                    Expr::Array(Array { elem: vec![
                        Expr::Value(Value::SingleQuotedString("5".to_string())),
                        Expr::Value(Value::SingleQuotedString("erf".to_string())),
                        Expr::Value(Value::SingleQuotedString("vvv".to_string())),
                    ], named: false }),
                    Expr::Array(Array { elem: vec![
                        Expr::Value(Value::SingleQuotedString("5".to_string())),
                        Expr::Value(Value::SingleQuotedString("erf".to_string())),
                        Expr::Value(Value::SingleQuotedString("vvv".to_string())),
                    ], named: false }),
                ], named: false }));
            },
            any => self.panic_unexpected(any, "types_select_type")
        }
    }

    fn generate(&mut self, info: &mut QueryInfo) -> Query {
        while let Some(state) = self.state_generator.next() {
            match state.as_str() {
                // ===== subgraph Query
                "Query" => self.handle_query(info),
                "EXIT_Query" => self.handle_query_exit(info),
                "Table" => self.handle_from_table(info),
                "call0_Query" => {},
                "Rcall0_Query" => self.handle_from_subquery_r(info),
                "FROM_multiple_relations" => self.handle_multiple_relations(info),
                "EXIT_FROM" => {},
                "WHERE" => {},
                "call0_VAL_3" => {},
                "Rcall0_VAL_3" => self.handle_where_val_3_ret(info),
                "SELECT" => {},
                "select_all" => self.handle_select_all(info),

                // ===== subgraph VAL_3
                "VAL_3" => self.handle_val_3(info),
                "EXIT_VAL_3" => {},
                "Rcall0_types_all" => self.handle_is_null_value(info),
                "Rcall1_types_all" => self.handle_is_distinct_from_1st_val(info),
                "Rcall21_types" => self.handle_is_distinct_from_2nd_val(info),
                "Rcall2_Query" => self.handle_exists_val(info),
                "Rcall2_types_all" => self.handle_in_list_1st_val(info),
                "Rcall1_list_expr" => self.handle_in_list_2nd_val(info),
                "Rcall3_types_all" => self.handle_in_subquery_1st_val(info),
                "Rcall3_Query" => self.handle_in_subquery_2nd_val(info),
                "Rcall4_types_all" => self.handle_between_1st_val(info),
                "Rcall22_types" => self.handle_between_2nd_val(info),
                "Rcall23_types" => self.handle_between_3rd_val(info),
                "Rcall5_types_all" => self.handle_binary_comp_1st_val(info),
                "Rcall24_types" => self.handle_binary_comp_2nd_val(info),
                "Rcall6_types_all" => self.handle_any_all_1st_val(info),
                "Rcall4_Query" => self.handle_any_all_2nd_val(info, false),
                "Rcall1_array" => self.handle_any_all_2nd_val(info, true),
                "Rcall25_types" => self.handle_string_like_1st_val(info),
                "Rcall26_types" => self.handle_string_like_2nd_val(info),
                "Rcall27_types" => self.handle_binary_bool_op_1st_val(info),
                "Rcall28_types" => self.handle_binary_bool_op_2nd_val(info),
                "Rcall29_types" => self.handle_nested_val_val(info),
                "Rcall30_types" => self.handle_unary_not_val(info),

                // ===== subgraph def_numeric
                "numeric" => self.handle_numeric(info),
                "EXIT_numeric" => {},
                "Rcall33_types" => self.handle_numeric_binary_1st_val(info),
                "Rcall32_types" => self.handle_numeric_binary_2nd_val(info),
                "Rcall1_types" => self.handle_numeric_unary_val(info),
                "Rcall2_types" => self.handle_string_position_1st_val(info),
                "Rcall3_types" => self.handle_string_position_2nd_val(info),
                "Rcall4_types" => self.handle_numeric_nested_val_val(info),

                // ===== subgraph def_string
                "string" => self.handle_string(info),
                "EXIT_string" => {},
                "Rcall6_types" => self.handle_trim_spec(info),
                "Rcall5_types" => self.handle_trim_val(info),
                "Rcall7_types" => self.handle_concat_1st_val(info),
                "Rcall8_types" => self.handle_concat_2nd_val(info),

                // ===== subgraph def_types
                "types" => self.handle_types(info),
                "EXIT_types" => {},
                "Rcall0_column_spec" => self.handle_types_select_type_value(info, false),
                "Rcall1_Query" => self.handle_types_select_type_value(info, true),
                "call0_numeric" => self.handle_types_numeric(info),
                "call1_VAL_3" => self.handle_types_val3(info),
                "call0_string" => self.handle_types_string(info),
                "call0_list_expr" => self.handle_types_list_expr(info),
                "call0_array" => self.handle_types_array(info),

                // ===== subgraph def_types_all
                "types_all" => self.handle_types_all(info),
                "EXIT_types_all" => {},

                // ===== subgraph def_column_spec
                "column_spec" => self.handle_column_spec(info),
                "EXIT_column_spec" => {},

                // ===== subgraph def_array
                "array" => self.handle_array(info),
                "EXIT_array" => {},
                "call12_types" => {},
                "call13_types" => {},  // this is hardcoded (no time!)
                "call31_types" => {},
                "call14_types" => {},
                "Rcall12_types" => {},
                "Rcall13_types" => {},
                "Rcall31_types" => {},
                "Rcall14_types" => {},

                // ===== subgraph def_list_expr
                "list_expr" => {
                    // TODO : HARDCODE
                    push_var!(info, ListExpr, Expr::Tuple(vec![
                        Expr::Value(Value::SingleQuotedString("5".to_string())),
                        Expr::Value(Value::SingleQuotedString("erf".to_string())),
                        Expr::Value(Value::SingleQuotedString("vvv".to_string())),
                    ]));
                }
                "EXIT_list_expr" => {},
                "list_expr_element" => {},
                "list_expr_element_exit" => {},
                "call16_types" => {},
                "call17_types" => {},
                "call18_types" => {},
                "call19_types" => {},
                "call20_types" => {},

                _ => panic!("Received unknown state: {state}")
            }
        }
        pop_var!(info, Query)
    }
}

/*
"call10_types" => {},
"call11_types" => {},
"call2_list_expr" => {},

"numericRight" => {},
"string_substr_numeric_from_from" => {},
"string_substr_numeric_len_len" => {},
"string_substring" => {},
*/

impl Iterator for QueryGenerator {
    type Item = Query;

    fn next(&mut self) -> Option<Self::Item> {

        // async self.generate():
        //   await self.handle_query(info)

        // async self.handle_query(info):
        //   self.expect_state("Query");
        //   let mut limit = Option::<Expr>::None;
        //   if let Some(mods) = self.state_generator.get_modifiers() {
        //       if mods.contains(&SmolStr::new("single value")) {
        //           limit = Some(Expr::Identifier(Ident::new("1")));
        //           // TODO: Not only limits can enforce this
        //       } else {
        //           panic!("Unexpected mods (Query): {:?}", mods);
        //       }
        //   }
        //   if let FunctionInputsType::TypeName(type_name) = self.state_generator.get_inputs() {
        //       println!("TODO: Enforce single column & column type (Query): {type_name}")
        //   }
        //   let select_body = Select {
        //       distinct: false,
        //       top: None,
        //       projection: vec![],
        //       into: None,
        //       from: vec![],
        //       lateral_views: vec![],
        //       selection: None,
        //       group_by: vec![],
        //       cluster_by: vec![],
        //       distribute_by: vec![],
        //       sort_by: vec![],
        //       having: None,
        //       qualify: None,
        //   };
        //   await self.handle_from(select_body, info);
        //   await self.handle_where(select_body, info);
        //   let query = Query {
        //       with: None,
        //       body: SetExpr::Select(Box::new(select_body)),
        //       order_by: vec![],
        //       limit: limit,
        //       offset: None,
        //       fetch: None,
        //       lock: None,
        //   };

        // async self.handle_from(select_body, info):
        //   self.expect_state("FROM");

        Some(self.generate(&mut QueryInfo::new()))
    }
}