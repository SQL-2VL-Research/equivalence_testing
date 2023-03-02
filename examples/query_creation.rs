use sqlparser::ast::{
    BinaryOperator, Expr, SelectItem, Ident, ObjectName, Query,
    Select, SetExpr, TableAlias, TableFactor, TableWithJoins,
};

pub fn create_table(name: &str, alias_name: Option<&str>) -> TableWithJoins {
    let name = name.to_string();
    TableWithJoins {
        relation: TableFactor::Table {
            name: ObjectName(vec![Ident::new(name)]),
            alias: alias_name.map(|name| TableAlias {
                name: Ident {
                    value: name.to_string(),
                    quote_style: None,
                },
                columns: Vec::<_>::new(),
            }),
            args: None,
            with_hints: Vec::<_>::new(),
            columns_definition: None,
        },
        joins: Vec::<_>::new(),
    }
}

#[allow(dead_code)]
pub fn create_identifier(name: &str) -> Expr {
    let name = name.to_string();
    Expr::Identifier(Ident {
        value: name.to_string(),
        quote_style: None,
    })
}

pub fn create_compound_identifier(name_with_dots: &str) -> Expr {
    let name_with_dots = name_with_dots.to_string();
    Expr::CompoundIdentifier(
        name_with_dots
            .split('.')
            .map(|name| Ident {
                value: name.to_string(),
                quote_style: None,
            })
            .collect::<Vec<_>>(),
    )
}

pub fn create_select(
    projection: Vec<SelectItem>,
    from: Vec<TableWithJoins>,
    selection: Option<Expr>,
    distinct: bool,
) -> Query {
    Query {
        with: None,
        body: Box::new(SetExpr::Select(Box::new(Select {
            distinct: distinct,
            top: None,
            projection: projection,
            into: None,
            from: from,
            lateral_views: Vec::<_>::new(),
            selection: selection,
            group_by: Vec::<_>::new(),
            cluster_by: Vec::<_>::new(),
            distribute_by: Vec::<_>::new(),
            sort_by: Vec::<_>::new(),
            having: None,
            qualify: None,
        }))),
        order_by: Vec::<_>::new(),
        limit: None,
        offset: None,
        fetch: None,
        locks: vec![],
    }
}

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
