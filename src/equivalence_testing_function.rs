use sqlparser::dialect::GenericDialect;
use sqlparser::keywords::{IGNORE, NEW, NULL};
use sqlparser::parser::Parser;

use sqlparser::ast::{
    Expr, Ident, Query, Select, SelectItem, SetExpr, TableAlias, TableFactor,
    TableWithJoins, Value, BinaryOperator, UnaryOperator, TrimWhereField, Array, Statement, ListAgg,
};
use sqlparser::dialect::{PostgreSqlDialect, Dialect, MySqlDialect};


fn string_to_query(input: &str) -> Box<Query> {
    let dialect = MySqlDialect {};
    let ast = Parser::parse_sql(&dialect, input).unwrap();
    match ast.into_iter().next().expect("No single query in sql file") {
        sqlparser::ast::Statement::Query(query) => query,
        _ => panic!("Query present in file is not a SELECT query.")
    }
}

//returns "true" if query is equivalent under both logics
pub fn equivalence_tester (sql: &str) -> bool {
    let query = string_to_query(sql);
    check_query(*query)
}

fn check_query (query: Query) -> bool {
    let _body = query.body;
    let _select = match _body {
        SetExpr::Select(select) => select,
        _ => panic!("query is not a SELECT query.")
    };
    
    let _where = _select.selection;
    return check_where(_where);
}

fn check_where(_where: Option<Expr>) -> bool {
    if _where == None {
        return true;
    }
    else {
        let where_content = match _where {
            Some(x) => x,
            _ => panic!("Wrong WHERE statement!"),
        };
        check_expr(where_content)
    }
}
    
fn check_expr (expr: Expr) -> bool {
    match expr {
        // Identifier e.g. table name or column name
        Expr::Identifier(Ident) => {
            return true;
        },
        
        // Multi-part identifier, e.g. `table_alias.column` or `schema.table.col`
        Expr::CompoundIdentifier(vec) => {
            return true;
        },
        
        // CompositeAccess (postgres) eg: SELECT (information_schema._pg_expandarray(array['i','i'])).n    
        Expr::CompositeAccess{expr, key} => {
            return check_expr(*expr);
        },
        
        ///"IS FALSE", "IS TRUE", "IS NULL", "IS NOT NULL" return boolean value and are not nullable
        // `IS FALSE` operator
        Expr::IsFalse(expr) => {
            return true;
        },
        
        // `IS TRUE` operator
        Expr::IsTrue(expr) => {
            return true;
        },
        
        // `IS NULL` operator
        Expr::IsNull(expr) => {
                return true;
        },
        
        // `IS NOT NULL` operator
        Expr::IsNotNull(expr) => {
            return true;
        },
        
        ///"IS DISTINCT FROM" return boolean value and is not nullable
        // `IS DISTINCT FROM` operator
        Expr::IsDistinctFrom(expr_first, expr_second) => {
            return true;
        },

        ///"IS NOT DISTINCT FROM" return boolean value and is not nullable
        // `IS NOT DISTINCT FROM` operator
        Expr::IsNotDistinctFrom(expr, negated) => {
            return true;
        },
        
        // `[ NOT ] IN (val1, val2, ...)`
        Expr::InList{expr, list, negated} => {
            if negated {
                if check_expr(*expr) == false {   
                    return false;
                }
                for el in list {
                    if check_expr(el) == false {
                        return false;
                    }
                }
            }
        },


        // `[ NOT ] IN (SELECT ...)`
        Expr::InSubquery {
            expr, subquery, negated
        } => {
            return !negated;
        },
        
        // `[ NOT ] IN UNNEST(array_expression)`
        Expr::InUnnest {  
            expr, array_expr, negated,
        } => {
            if negated {
                return (check_expr(*expr) == true) && (check_expr(*array_expr) == true);
            }
        },

        ///BETWEEN is treated as comparison operation
        // `<expr> [ NOT ] BETWEEN <low> AND <high>`
        Expr::Between {
            expr, negated, low, high,
        } => {
            if negated {
                return check_expr(*low) && check_expr(*high) && check_expr(*expr);
            }
        },

        // Binary operation e.g. `1 + 1` or `foo > bar`
        Expr::BinaryOp {
            left, op, right,
        } => {
                return check_expr(*left) && check_expr(*right);           
        },

        // Any operation e.g. `1 ANY (1)` or `foo > ANY(bar)`, It will be wrapped in the right side of BinaryExpr
        Expr::AnyOp(_box) => {
            return check_expr(*_box);
        },
        
        // ALL operation e.g. `1 ALL (1)` or `foo > ALL(bar)`, It will be wrapped in the right side of BinaryExpr
        Expr::AllOp(_box) => {
            return check_expr(*_box);
        },

        // Unary operation e.g. `NOT foo`
        Expr::UnaryOp { op, expr} => {
            if op == sqlparser::ast::UnaryOperator::Not {
                return check_expr(*expr);
            }
            else {
                return true;
            }
        },

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

        // Nested expression e.g. `(foo > bar)` or `(1)`
        Expr::Nested(_box) => {
            return check_expr(*_box);
        },
        
        // A literal value, such as string, number, date or NULL
        Expr::Value(Value) => {
            if Value == sqlparser::ast::Value::Null {
                return false;
            }
            else{
                return true;
            }
        },
        
        /// An exists expression `[ NOT ] EXISTS(SELECT ...)`, used in expressions like
        /// `WHERE [ NOT ] EXISTS (SELECT ...)`.
        Expr::Exists {subquery, negated} => {
            if negated {
                return check_query(*subquery);
            }
        },
        
        /// A parenthesized subquery `(SELECT ...)`, used in expression like
        /// `SELECT (subquery) AS x` or `WHERE (subquery) = x`
        Expr::Subquery(subquery) => {
            return check_query(*subquery);
        },
        
        // An array expression e.g. `ARRAY[1, 2]`
        Expr::Array(Array) => {
            for el in Array.elem {
                if (!check_expr(el)) {
                    return false;
                }
            }
        }, 

         // The `LISTAGG` function `SELECT LISTAGG(...) WITHIN GROUP (ORDER BY ...)`
         Expr::ListAgg(_ListAgg) => {
            let condition_1 = check_expr(*_ListAgg.expr);

            let condition_2 = match _ListAgg.separator {
                Some(x) => check_expr(*x),
                None => true,
            };

            return condition_1 && condition_2;

        },

        // The `GROUPING SETS` expr.
        Expr::GroupingSets(_Vec) => {
            for _vec in _Vec {
                for el in _vec {
                    if !check_expr(el) {
                        return false;
                    }
                }
            }
            return true;
        },

        // The `CUBE` expr.
        Expr::Cube(_Vec) => {
            for _vec in _Vec {
                for el in _vec {
                    if !check_expr(el) {
                        return false;
                    }
                }
            }
            return true;
        },

        // The `ROLLUP` expr.
        Expr::Rollup(_Vec) => {
            for _vec in _Vec {
                for el in _vec {
                    if !check_expr(el) {
                        return false;
                    }
                }
            }
            return true;
        },

        // ROW / TUPLE a single value, such as `SELECT (1, 2)`
        Expr::Tuple(_Vec) => {
            for el in _Vec {
                if !check_expr(el) {
                    return false;
                }
            }
            return true;
        },

        // An array index expression e.g. `(ARRAY[1, 2])[1]` or `(current_schemas(FALSE))[1]`
        Expr::ArrayIndex { 
            obj,
            indexes, } => {
                if !check_expr(*obj) {
                    return false;
                }

                for el in indexes {
                    if !check_expr(el) {
                        return false;
                    }
                }
                return true;
            }, 

            _ => panic!("These WHERE cases not ready yet!"),
    }

    true
}
