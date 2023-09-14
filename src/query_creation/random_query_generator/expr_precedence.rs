use sqlparser::ast::{Expr, UnaryOperator, BinaryOperator};

pub(crate) trait ExpressionNesting {
    /// nest l-value if needed
    fn p_nest_l(self, parent_priority: i32) -> Self;
    /// nest r-value if needed
    fn p_nest_r(self, parent_priority: i32) -> Self;
}

pub(crate) trait ExpressionPriority: ExpressionNesting {
    fn get_priority(&self) -> i32;
    fn nest_children_if_needed(self) -> Expr;
}

impl ExpressionNesting for Vec<Expr> {
    fn p_nest_l(self, parent_priority: i32) -> Vec<Expr> {
        self.into_iter().map(|expr| expr.p_nest_l(parent_priority)).collect()
    }

    fn p_nest_r(self, parent_priority: i32) -> Vec<Expr> {
        self.into_iter().map(|expr| expr.p_nest_r(parent_priority)).collect()
    }
}

impl ExpressionNesting for Vec<Vec<Expr>> {
    fn p_nest_l(self, parent_priority: i32) -> Vec<Vec<Expr>> {
        self.into_iter().map(|expr| expr.p_nest_l(parent_priority)).collect()
    }

    fn p_nest_r(self, parent_priority: i32) -> Vec<Vec<Expr>> {
        self.into_iter().map(|expr| expr.p_nest_r(parent_priority)).collect()
    }
}

impl ExpressionNesting for Box<Expr> {
    fn p_nest_l(self, parent_priority: i32) -> Box<Expr> {
        Box::new((*self).p_nest_l(parent_priority))
    }

    fn p_nest_r(self, parent_priority: i32) -> Box<Expr> {
        Box::new((*self).p_nest_r(parent_priority))
    }
}

impl ExpressionNesting for Option<Box<Expr>> {
    fn p_nest_l(self, parent_priority: i32) -> Option<Box<Expr>> {
        self.map(|expr| expr.p_nest_l(parent_priority))
    }

    fn p_nest_r(self, parent_priority: i32) -> Option<Box<Expr>> {
        self.map(|expr| expr.p_nest_r(parent_priority))
    }
}

impl ExpressionNesting for Expr {
    fn p_nest_l(self, parent_priority: i32) -> Expr {
        if self.get_priority() > parent_priority {
            Expr::Nested(Box::new(self))
        } else {
            self
        }
    }

    fn p_nest_r(self, parent_priority: i32) -> Expr {
        if self.get_priority() >= parent_priority {
            Expr::Nested(Box::new(self))
        } else {
            self
        }
    }
}

fn nest_rightwards_if_is_a_text_binary_op(expr: Box<Expr>, parent_priority: i32) -> Box<Expr> {
    let is_a_text_binary_op = match &*expr {
        Expr::Like { .. } |
        Expr::ILike { .. } |
        Expr::Between { .. } |
        Expr::InList { .. } |
        Expr::InSubquery { .. } |
        Expr::InUnnest { .. } |
        Expr::SimilarTo { .. } |
        Expr::IsDistinctFrom(..) |
        Expr::IsNotDistinctFrom(..) => true,
        _ => false,
    };
    if is_a_text_binary_op {
        expr.p_nest_r(parent_priority)
    } else {
        expr.p_nest_l(parent_priority)
    }
}

impl ExpressionPriority for Expr {
    fn get_priority(&self) -> i32 {
        match self {
            // no nesting, not of children nor of ourselves is needed
            Expr::Function(..) => -1,
            Expr::Nested(..) => -1,
            Expr::Value(..) => -1,
            Expr::Identifier(..) => -1,
            Expr::AnyOp(..) => -1,
            Expr::AllOp(..) => -1,
            Expr::Extract { .. } => -1,
            Expr::Position { .. } => -1,
            Expr::Substring { .. } => -1,
            Expr::Trim { .. } => -1,
            Expr::TryCast { .. } => -1,
            Expr::SafeCast { .. } => -1,
            Expr::Subquery(..) => -1,
            Expr::ListAgg(..) => -1,
            Expr::Tuple(..) => -1,
            Expr::Array(..) => -1,
            Expr::Ceil { .. } => -1,
            Expr::Floor { .. } => -1,
            Expr::Overlay { .. } => -1,
            Expr::ArrayAgg(..) => -1,
            Expr::ArraySubquery(..) => -1,
            Expr::MatchAgainst { .. } => -1,
            Expr::Case { .. } => -1,
            Expr::GroupingSets(..) => -1,
            Expr::Cube(..) => -1,
            Expr::Rollup(..) => -1,
            Expr::AggregateExpressionWithFilter { .. } => -1,

            // normal operations
            Expr::CompoundIdentifier(..) => 0,
            Expr::CompositeAccess { .. } => 0,
            Expr::Cast { .. } => 1,  // can be a ::
            Expr::ArrayIndex { .. } => 2,
            Expr::MapAccess { .. } => 2,
            Expr::UnaryOp { op, .. } => {
                match op {
                    UnaryOperator::Plus => 3,
                    UnaryOperator::Minus => 3,
                    UnaryOperator::PGBitwiseNot => 7,
                    UnaryOperator::PGSquareRoot => 7,
                    UnaryOperator::PGCubeRoot => 7,
                    UnaryOperator::PGPostfixFactorial => 7,
                    UnaryOperator::PGPrefixFactorial => 7,
                    UnaryOperator::PGAbs => 7,
                    UnaryOperator::Not => 11,
                }
            },
            Expr::BinaryOp { op, .. } => {
                match op {
                    BinaryOperator::PGExp => 4,
                    BinaryOperator::Multiply => 5,
                    BinaryOperator::Divide => 5,
                    BinaryOperator::Modulo => 5,
                    BinaryOperator::Plus => 6,
                    BinaryOperator::Minus => 6,
                    BinaryOperator::StringConcat => 7,
                    BinaryOperator::Spaceship => 7,
                    BinaryOperator::Xor => 7,
                    BinaryOperator::BitwiseOr => 7,
                    BinaryOperator::BitwiseAnd => 7,
                    BinaryOperator::BitwiseXor => 7,
                    BinaryOperator::PGBitwiseXor => 7,
                    BinaryOperator::PGBitwiseShiftLeft => 7,
                    BinaryOperator::PGBitwiseShiftRight => 7,
                    BinaryOperator::PGRegexMatch => 7,
                    BinaryOperator::PGRegexIMatch => 7,
                    BinaryOperator::PGRegexNotMatch => 7,
                    BinaryOperator::PGRegexNotIMatch => 7,
                    BinaryOperator::PGCustomBinaryOperator(..) => 7,
                    BinaryOperator::Gt => 9, // all of those have p_nest_r on the left! (because double comparison not possible)
                    BinaryOperator::Lt => 9,  // ...
                    BinaryOperator::GtEq => 9,  // ...
                    BinaryOperator::LtEq => 9,  // ...
                    BinaryOperator::Eq => 9,  // ...
                    BinaryOperator::NotEq => 9, // all of those have p_nest_r on the left! (because double comparison not possible)
                    BinaryOperator::And => 12,
                    BinaryOperator::Or => 13,
                }
            },
            Expr::Like { .. } => 8,
            Expr::ILike { .. } => 8,
            Expr::Between { .. } => 8,
            Expr::InList { .. } => 8,
            Expr::InSubquery { .. } => 8,
            Expr::InUnnest { .. } => 8,
            Expr::SimilarTo { .. } => 8,
            Expr::IsFalse(..) => 10,
            Expr::IsTrue(..) => 10,
            Expr::IsNull(..) => 10,
            Expr::IsNotNull(..) => 10,
            Expr::IsDistinctFrom(..) => 10,
            Expr::IsNotDistinctFrom(..) => 10,
            Expr::IsNotFalse(..) => 10,
            Expr::IsNotTrue(..) => 10,
            Expr::IsUnknown(..) => 10,
            Expr::IsNotUnknown(..) => 10,
            // EXISTS needs nesting possibly because of NOT, thus inherits its priority
            Expr::Exists { negated, .. } => if *negated {11} else {-1},
            Expr::JsonAccess { .. } => 7,
            Expr::Collate { .. } => 7,
            Expr::TypedString { .. } => 7,
            Expr::AtTimeZone { .. } => 7,
            Expr::IntroducedString { .. } => 7,
            Expr::Interval { .. } => 7,
        }
    }

    /// adds nesting to child if needed
    fn nest_children_if_needed(self) -> Expr {
        let parent_priority = self.get_priority();
        if parent_priority == -1 {
            return self
        }
        match self {
            Expr::CompoundIdentifier(ident_vec) => Expr::CompoundIdentifier(ident_vec),
            Expr::CompositeAccess { expr, key } => Expr::CompositeAccess { expr: expr.p_nest_l(parent_priority), key },
            Expr::Cast { expr, data_type} => Expr::Cast { expr: expr.p_nest_l(parent_priority), data_type},
            Expr::ArrayIndex { obj, indexes } => Expr::ArrayIndex { obj: obj.p_nest_l(parent_priority), indexes },
            Expr::MapAccess { column, keys } => Expr::MapAccess { column: column.p_nest_l(parent_priority), keys },
            Expr::UnaryOp { op, expr } => {
                if op == UnaryOperator::Not {
                    if let Expr::Exists { subquery: _, negated: _ } = *expr {
                        // exists will just be negated by the parser otherwise
                        return Expr::UnaryOp { op, expr: Box::new(Expr::Nested(expr)) }
                    }
                }
                if op == UnaryOperator::Minus {
                    if let Expr::UnaryOp { op: op2, expr: _ } = *expr {
                        if op2 == UnaryOperator::Minus {
                            // -- is a comment in SQL.
                            return Expr::UnaryOp { op, expr: Box::new(Expr::Nested(expr)) }
                        }
                    }
                }
                Expr::UnaryOp { op, expr: expr.p_nest_r(parent_priority) }  // p_nest_r is because unary operations are prefix ones.
            },
            Expr::BinaryOp { left, op, right } => {
                let is_cmp = match &op {
                    BinaryOperator::Gt |
                    BinaryOperator::Lt |
                    BinaryOperator::GtEq |
                    BinaryOperator::LtEq |
                    BinaryOperator::Eq |
                    BinaryOperator::NotEq => true,
                    _ => false
                };
                if is_cmp {
                    Expr::BinaryOp { left: left.p_nest_r(parent_priority), op, right: right.p_nest_r(parent_priority) }
                } else {
                    Expr::BinaryOp { left: left.p_nest_l(parent_priority), op, right: right.p_nest_r(parent_priority) }
                }
            },
            Expr::Like { negated, expr, pattern, escape_char } => Expr::Like { negated, expr: nest_rightwards_if_is_a_text_binary_op(expr, parent_priority), pattern: pattern.p_nest_r(parent_priority), escape_char },
            Expr::ILike { negated, expr, pattern, escape_char } => Expr::ILike { negated, expr: nest_rightwards_if_is_a_text_binary_op(expr, parent_priority), pattern: pattern.p_nest_r(parent_priority), escape_char },
            Expr::Between { expr, negated, low, high } => Expr::Between { expr: nest_rightwards_if_is_a_text_binary_op(expr, parent_priority), negated, low: low.p_nest_r(parent_priority), high: high.p_nest_r(parent_priority) },
            Expr::InList { expr, list, negated } => Expr::InList { expr: nest_rightwards_if_is_a_text_binary_op(expr, parent_priority), list, negated },
            Expr::InSubquery { expr, subquery, negated } => Expr::InSubquery { expr: nest_rightwards_if_is_a_text_binary_op(expr, parent_priority), subquery, negated },
            Expr::InUnnest { expr, array_expr, negated } => Expr::InUnnest { expr: nest_rightwards_if_is_a_text_binary_op(expr, parent_priority), array_expr: array_expr, negated },
            Expr::SimilarTo { negated, expr, pattern, escape_char } => Expr::SimilarTo { negated, expr: nest_rightwards_if_is_a_text_binary_op(expr, parent_priority), pattern: pattern.p_nest_r(parent_priority), escape_char },
            Expr::IsFalse(expr) => Expr::IsFalse(nest_rightwards_if_is_a_text_binary_op(expr, parent_priority)),
            Expr::IsTrue(expr) => Expr::IsTrue(nest_rightwards_if_is_a_text_binary_op(expr, parent_priority)),
            Expr::IsNull(expr) => Expr::IsNull(nest_rightwards_if_is_a_text_binary_op(expr, parent_priority)),
            Expr::IsNotNull(expr) => Expr::IsNotNull(nest_rightwards_if_is_a_text_binary_op(expr, parent_priority)),
            Expr::IsDistinctFrom(expr1, expr2) => Expr::IsDistinctFrom(nest_rightwards_if_is_a_text_binary_op(expr1, parent_priority), expr2.p_nest_r(parent_priority)),
            Expr::IsNotDistinctFrom(expr1, expr2) => Expr::IsNotDistinctFrom(nest_rightwards_if_is_a_text_binary_op(expr1, parent_priority), expr2.p_nest_r(parent_priority)),
            Expr::IsNotFalse(expr) => Expr::IsNotFalse(nest_rightwards_if_is_a_text_binary_op(expr, parent_priority)),
            Expr::IsNotTrue(expr) => Expr::IsNotTrue(nest_rightwards_if_is_a_text_binary_op(expr, parent_priority)),
            Expr::IsUnknown(expr) => Expr::IsUnknown(nest_rightwards_if_is_a_text_binary_op(expr, parent_priority)),
            Expr::IsNotUnknown(expr) => Expr::IsNotUnknown(nest_rightwards_if_is_a_text_binary_op(expr, parent_priority)),
            Expr::JsonAccess { left, operator, right } => Expr::JsonAccess { left: left.p_nest_l(parent_priority), operator, right: right.p_nest_r(parent_priority) },
            Expr::Collate { expr, collation } => Expr::Collate { expr: nest_rightwards_if_is_a_text_binary_op(expr, parent_priority), collation },
            Expr::TypedString { data_type, value } => Expr::TypedString { data_type, value },
            Expr::AtTimeZone { timestamp, time_zone } => Expr::AtTimeZone { timestamp: nest_rightwards_if_is_a_text_binary_op(timestamp, parent_priority), time_zone },
            Expr::IntroducedString { introducer, value } => Expr::IntroducedString { introducer, value },
            Expr::Interval { value, leading_field, leading_precision, last_field, fractional_seconds_precision } => Expr::Interval { value: value.p_nest_l(parent_priority), leading_field, leading_precision, last_field, fractional_seconds_precision },
            any => any
        }
    }
}
