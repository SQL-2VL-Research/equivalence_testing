use sqlparser::parser::Parser;

use sqlparser::ast::{
    Expr, Query, SetExpr, ObjectName, Ident,
};
use sqlparser::dialect::PostgreSqlDialect;

pub fn string_to_query(input: &str) -> Box<Query> {
    let dialect = PostgreSqlDialect {};
    let ast = match Parser::parse_sql(&dialect, input) {
        Ok(_ast) => _ast,
        Err(err) => {
            println!("Query: {}", input);
            println!("Parsing error! {}", err);
            panic!();
        },
    };
    match ast.into_iter().next().expect("No single query in sql file") {
        sqlparser::ast::Statement::Query(query) => query,
        _ => panic!("Query present in file is not a SELECT query.")
    }
}

pub fn check_query (query: Box<Query>) -> bool {
    let body = query.body;
    let select = match *body {
        SetExpr::Select(select) => select,
        _ => panic!("query is not a SELECT query.")
    };
    let where_ = select.selection;
    return check_where(where_);
}

fn check_where(where_opt: Option<Expr>) -> bool {
    if let Some(where_) = where_opt {
        check_expr(where_)
    } else { true }
}

fn check_expr(expr: Expr) -> bool {
    match expr {
        // Identifier e.g. table name or column name
        Expr::Identifier(_ident) => false,
        // Multi-part identifier, e.g. `table_alias.column` or `schema.table.col`
        Expr::CompoundIdentifier(_vec) => false,
        // CompositeAccess (postgres) eg: SELECT (information_schema._pg_expandarray(array['i','i'])).n    
        Expr::CompositeAccess{expr, key: _} => check_expr(*expr),
        // "IS FALSE", "IS TRUE", "IS NULL", "IS NOT NULL" return boolean value and are not nullable
        // `IS FALSE` operator
        Expr::IsFalse(_expr) => true,
        Expr::IsNotFalse(_) => true,
        // `IS UNKNOWN` operator
        Expr::IsUnknown(_) => true,
        Expr::IsNotUnknown(_) => true,
        // `IS TRUE` operator
        Expr::IsTrue(_expr) => true,
        Expr::IsNotTrue(_) => true,
        // `IS NULL` operator
        Expr::IsNull(_expr) => true,
        // `IS NOT NULL` operator
        Expr::IsNotNull(_expr) => true,
        // "IS DISTINCT FROM" return boolean value and is not nullable
        // `IS DISTINCT FROM` operator
        Expr::IsDistinctFrom(_expr_first, _expr_second) => true,
        // "IS NOT DISTINCT FROM" return boolean value and is not nullable
        // `IS NOT DISTINCT FROM` operator
        Expr::IsNotDistinctFrom(_expr, _negated) => true,
        // `[ NOT ] IN (val1, val2, ...)`
        Expr::InList{expr, list, negated} => {
            if negated {
                check_expr(*expr) && list.into_iter().all(check_expr)
            } else { true }
        },
        // `[ NOT ] IN (SELECT ...)`
        Expr::InSubquery {expr, subquery, negated} => {
            if negated {
                return check_expr(*expr) && check_query(subquery);
            } 
            else { true }
        },  
        // `[ NOT ] IN UNNEST(array_expression)`
        Expr::InUnnest {  
            expr, array_expr, negated,
        } => {
            if negated {
                check_expr(*expr) && check_expr(*array_expr)
            } else { true }
        },
        // A literal value, such as string, number, date or NULL
        Expr::Value(_val) => true,
        // BETWEEN is treated as comparison operation
        // `<expr> [ NOT ] BETWEEN <low> AND <high>`
        Expr::Between {
            expr, negated, low, high,
        } => {
            if negated {
                check_expr(*low) && check_expr(*high) && check_expr(*expr)
            } else { true }
        },
        // Binary operation e.g. `1 + 1` or `foo > bar`
        Expr::BinaryOp {
            left, op: _, right,
        } => check_expr(*left) && check_expr(*right),
        /// LIKE
        Expr::Like {
            negated,
            expr,
            pattern,
            escape_char,
        } => check_expr(*expr) && check_expr(*pattern) && negated,
        /// ILIKE (case-insensitive LIKE)
        Expr::ILike {
            negated,
            expr,
            pattern,
            escape_char,
        } => check_expr(*expr) && check_expr(*pattern) && negated,
        /// SIMILAR TO regex
        Expr::SimilarTo {
            negated,
            expr,
            pattern,
            escape_char,
        } => check_expr(*expr) && check_expr(*pattern),
        // Any operation e.g. `1 ANY (1)` or `foo > ANY(bar)`, It will be wrapped in the right side of BinaryExpr
        Expr::AnyOp(expr) => check_expr(*expr),
        // ALL operation e.g. `1 ALL (1)` or `foo > ALL(bar)`, It will be wrapped in the right side of BinaryExpr
        Expr::AllOp(expr) => check_expr(*expr),
        // Unary operation e.g. `NOT foo`
        Expr::UnaryOp { op, expr} => {
            if op == sqlparser::ast::UnaryOperator::Not {
                check_expr(*expr)
            } else { true }
        },
        /// CAST an expression to a different data type e.g. `CAST(foo AS VARCHAR(123))`
        Expr::Cast {
            expr,
            data_type,
        } => check_expr(*expr),
        /// TRY_CAST an expression to a different data type e.g. `TRY_CAST(foo AS VARCHAR(123))`
        //  this differs from CAST in the choice of how to implement invalid conversions
        Expr::TryCast {
            expr,
            data_type,
        } => check_expr(*expr),
        /// SAFE_CAST an expression to a different data type e.g. `SAFE_CAST(foo AS FLOAT64)`
        //  only available for BigQuery: https://cloud.google.com/bigquery/docs/reference/standard-sql/functions-and-operators#safe_casting
        //  this works the same as `TRY_CAST`
        Expr::SafeCast {
            expr,
            data_type,
        } => check_expr(*expr),
        /// AT a timestamp to a different timezone e.g. `FROM_UNIXTIME(0) AT TIME ZONE 'UTC-06:00'`
        Expr::AtTimeZone {
            timestamp,
            time_zone,
        } => check_expr(*timestamp),
        /// EXTRACT(DateTimeField FROM <expr>)
        Expr::Extract {
            field,
            expr,
        } => check_expr(*expr),
        /// CEIL(<expr> [TO DateTimeField])
        Expr::Ceil {
            expr,
            field,
        } => check_expr(*expr),
        /// FLOOR(<expr> [TO DateTimeField])
        Expr::Floor {
            expr,
            field,
        } => check_expr(*expr),
        /// POSITION(<expr> in <expr>)
        Expr::Position { 
            expr, 
            r#in,
        } => check_expr(*expr) && check_expr(*r#in),
        // SUBSTRING(<expr> [FROM <expr>] [FOR <expr>])
        Expr::Substring {
            expr, substring_from, substring_for
        } => {
            let condition_1 = check_expr(*expr);
            let condition_2 = match substring_from {
                Some(x) => check_expr(*x),
                None => true,
            };
            let condition_3 = match substring_for {
                Some(x) => check_expr(*x),
                None => true,
            };
            return condition_1 && condition_2 && condition_3;
        },
        Expr::Trim {
            expr,
            // ([BOTH | LEADING | TRAILING]
            trim_where,
            trim_what,
        } => {
            match trim_what {
                Some(x) => check_expr(*x),
                None => true,
            }
        },
        /// OVERLAY(<expr> PLACING <expr> FROM <expr>[ FOR <expr> ]
        Expr::Overlay {
            expr,
            overlay_what,
            overlay_from,
            overlay_for,
        } => {
            let cond = match overlay_for {
                Some(x) => check_expr(*x),
                None => true,
            };
            check_expr(*expr) && cond && check_expr(*overlay_from) && check_expr(*overlay_what)
        },        
        /// `expr COLLATE collation`
        Expr::Collate {
            expr,
            collation,
        } => check_expr(*expr),
        Expr::IntroducedString { introducer, value} => true,
        /// A constant of form `<data_type> 'value'`.
        /// This can represent ANSI SQL `DATE`, `TIME`, and `TIMESTAMP` literals (such as `DATE '2020-01-01'`),
        /// as well as constants of other types (a non-standard PostgreSQL extension).
        Expr::TypedString { data_type: DataType, value: String } => true,
        /// Access a map-like object by field (e.g. `column['field']` or `column[4]`        
        Expr::MapAccess { 
            column, 
            keys
        } => keys.into_iter().all(check_expr) && check_expr(*column),
        /// Scalar function call e.g. `LEFT(foo, 5)`
        Expr::Function(Function) => {
            if (Function.name == ObjectName(vec![Ident{value: "COUNT".to_string(), quote_style: (None)}])) {
                true
            }
            else {
                false
            }
        },
        /// Aggregate function with filter
        Expr::AggregateExpressionWithFilter { 
            expr, 
            filter
        } => check_expr(*expr) && check_expr(*filter),
        /// `CASE [<operand>] WHEN <condition> THEN <result> ... [ELSE <result>] END`
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            let mut res = true;
            res = match operand {
                Some(x) => check_expr(*x),
                None => res,
            };
            match else_result {
                Some(x) => {
                    if !check_expr(*x) { 
                        res = false;
                    }
                }
                None => {},
            };
            return res && conditions.into_iter().all(check_expr) && results.into_iter().all(check_expr)
        },
        // Nested expression e.g. `(foo > bar)` or `(1)`
        Expr::Nested(expr) => check_expr(*expr),
        // `WHERE [ NOT ] EXISTS (SELECT ...)`.
        Expr::Exists {subquery, negated} => {
            if negated {
                return check_query(subquery);
            } else { true }
        },
        // A parenthesized subquery `(SELECT ...)`, used in expression like
        // `SELECT (subquery) AS x` or `WHERE (subquery) = x`
        Expr::Subquery(subquery) => check_query(subquery),
        // An array expression e.g. `ARRAY[1, 2]`
        Expr::Array(array) => array.elem.into_iter().all(check_expr), 
        // The `LISTAGG` function `SELECT LISTAGG(...) WITHIN GROUP (ORDER BY ...)`
        Expr::ListAgg(list_agg) => {
            let condition_1 = check_expr(*list_agg.expr);
            let condition_2 = match list_agg.separator {
                Some(x) => check_expr(*x),
                None => true,
            };
            condition_1 && condition_2
        },
        // The `GROUPING SETS` expr.
        Expr::GroupingSets(vec_vec) => {
            vec_vec.into_iter().all(|vec| vec.into_iter().all(check_expr))
        },
        // The `CUBE` expr.
        Expr::Cube(vec_vec) => {
            vec_vec.into_iter().all(|vec| vec.into_iter().all(check_expr))
        },
        // The `ROLLUP` expr.
        Expr::Rollup(vec_vec) => {
            vec_vec.into_iter().all(|vec| vec.into_iter().all(check_expr))
        },
        // ROW / TUPLE a single value, such as `SELECT (1, 2)`
        Expr::Tuple(vec) => {
            vec.into_iter().all(check_expr)
        },
        // An array index expression e.g. `(ARRAY[1, 2])[1]` or `(current_schemas(FALSE))[1]`
        Expr::ArrayIndex { 
            obj,
            indexes,
        } => {
            check_expr(*obj) && indexes.into_iter().all(check_expr)
        },
        /// INTERVAL literals, roughly in the following format:
        /// `INTERVAL '<value>' [ <leading_field> [ (<leading_precision>) ] ]
        /// [ TO <last_field> [ (<fractional_seconds_precision>) ] ]`,
        Expr::Interval {
            value,
            leading_field,
            leading_precision,
            last_field,
            /// The seconds precision can be specified in SQL source as
            /// `INTERVAL '__' SECOND(_, x)` (in which case the `leading_field`
            /// will be `Second` and the `last_field` will be `None`),
            /// or as `__ TO SECOND(x)`.
            fractional_seconds_precision,
        } => check_expr(*value),
        Expr::JsonAccess { left, operator: _, right } => {
            check_expr(*left) && check_expr(*right)
        },
        Expr::MatchAgainst {
            /// `(<col>, <col>, ...)`.
            columns,
            /// `<expr>`.
            match_value,
            /// `<search modifier>`
            opt_search_modifier,
        } => true,
        _ => panic!("unexpected agrument"),
    }
}