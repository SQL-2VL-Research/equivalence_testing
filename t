Query {
    with: None,
    body: Select(
        Select {
            distinct: true,
            top: None,
            projection: [
                Wildcard,
            ],
            into: None,
            from: [
                TableWithJoins {
                    relation: Table {
                        name: ObjectName(
                            [
                                Ident {
                                    value: "T1",
                                    quote_style: None,
                                },
                            ],
                        ),
                        alias: None,
                        args: None,
                        with_hints: [],
                    },
                    joins: [],
                },
            ],
            lateral_views: [],
            selection: Some(
                IsNotNull(
                    BinaryOp {
                        left: Value(
                            Boolean(
                                true,
                            ),
                        ),
                        op: Lt,
                        right: Value(
                            Null,
                        ),
                    },
                ),
            ),
            group_by: [],
            cluster_by: [],
            distribute_by: [],
            sort_by: [],
            having: None,
            qualify: None,
        },
    ),
    order_by: [],
    limit: Some(
        Value(
            Null,
        ),
    ),
    offset: None,
    fetch: None,
    lock: None,
}
