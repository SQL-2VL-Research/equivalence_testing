use sqlparser::{ast::{Query, Select, SetExpr, TableWithJoins, TableFactor, Ident, ObjectName, SelectItem, Expr}};

pub fn create_table(name: &str) -> TableWithJoins {
    let name = name.to_string();
    TableWithJoins {
        relation: TableFactor::Table {
            name: ObjectName(vec![Ident::new(name)]),
            alias: None,
            args: None,
            with_hints: Vec::<_>::new()
        },
        joins: Vec::<_>::new()
    }
}

#[allow(dead_code)]
pub fn create_identifier(name: &str) -> Expr {
    let name = name.to_string();
    Expr::Identifier(Ident {
        value: name.to_string(),
        quote_style: None
    })
}

pub fn create_compound_identifier(name_with_dots: &str) -> Expr {
    let name_with_dots = name_with_dots.to_string();
    Expr::CompoundIdentifier(name_with_dots.split('.').map(|name| Ident {
        value: name.to_string(),
        quote_style: None
    }).collect::<Vec<_>>())
}

pub fn create_select(projection: Vec<SelectItem>, from: Vec<TableWithJoins>, selection: Option<Expr>) -> Query {
    Query {
        with: None,
        body: SetExpr::Select(Box::new(Select {
            distinct: false,
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
        })),
        order_by: Vec::<_>::new(),
        limit: None,
        offset: None,
        fetch: None,
        lock: None,
    }
}