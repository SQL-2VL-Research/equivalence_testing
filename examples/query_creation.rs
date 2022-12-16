use sqlparser::ast::{BinaryOperator, Expr, SelectItem};

use equivalence_testing::query_creation::helpers::{
    create_compound_identifier, create_select, create_table,
};

fn main() {
    // SELECT R.A FROM R WHERE R.A NOT IN (SELECT S.A FROM S)
    let q1 = create_select(
        vec![SelectItem::UnnamedExpr(create_compound_identifier("R.A"))],
        vec![create_table("R", None)],
        Some(Expr::InSubquery {
            expr: Box::new(create_compound_identifier("R.A")),
            subquery: Box::new(create_select(
                vec![SelectItem::UnnamedExpr(create_compound_identifier("S.A"))],
                vec![create_table("S", None)],
                None,
                false,
            )),
            negated: true,
        }),
        false,
    );
    println!("Query 1: {:#?}", q1.to_string());

    // SELECT R.A FROM R WHERE NOT EXISTS (SELECT S.A FROM S WHERE S.A = R.A)
    let q2 = create_select(
        vec![SelectItem::UnnamedExpr(create_compound_identifier("R.A"))],
        vec![create_table("R", None)],
        Some(Expr::Exists {
            subquery: Box::new(create_select(
                vec![SelectItem::UnnamedExpr(create_compound_identifier("S.A"))],
                vec![create_table("S", None)],
                Some(Expr::BinaryOp {
                    left: Box::new(create_compound_identifier("S.A")),
                    op: BinaryOperator::Eq,
                    right: Box::new(create_compound_identifier("R.A")),
                }),
                false,
            )),
            negated: true,
        }),
        false,
    );
    println!("Query 2: {:#?}", q2.to_string());

    // SELECT DISTINT X.A FROM R X, R Y WHERE X.A=Y.A
    let q3 = create_select(
        vec![SelectItem::UnnamedExpr(create_compound_identifier("X.A"))],
        vec![create_table("R", Some("X")), create_table("R", Some("Y"))],
        Some(Expr::BinaryOp {
            left: Box::new(create_compound_identifier("X.A")),
            op: BinaryOperator::Eq,
            right: Box::new(create_compound_identifier("Y.A")),
        }),
        true,
    );
    println!("Query 3: {:#?}", q3.to_string());

    // SELECT DISTINT R.A FROM R
    let q4 = create_select(
        vec![SelectItem::UnnamedExpr(create_compound_identifier("R.A"))],
        vec![create_table("R", None)],
        None,
        true,
    );
    println!("Query 4: {:#?}", q4.to_string());
}
