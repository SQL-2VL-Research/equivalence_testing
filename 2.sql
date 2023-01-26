Query {
    with: None,
    body: Select(
        Select {
            distinct: false,
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
                TableWithJoins {
                    relation: Table {
                        name: ObjectName(
                            [
                                Ident {
                                    value: "T2",
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
                TableWithJoins {
                    relation: Table {
                        name: ObjectName(
                            [
                                Ident {
                                    value: "T3",
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
                TableWithJoins {
                    relation: Table {
                        name: ObjectName(
                            [
                                Ident {
                                    value: "T4",
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
                TableWithJoins {
                    relation: Table {
                        name: ObjectName(
                            [
                                Ident {
                                    value: "T5",
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
                TableWithJoins {
                    relation: Table {
                        name: ObjectName(
                            [
                                Ident {
                                    value: "T6",
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
                TableWithJoins {
                    relation: Table {
                        name: ObjectName(
                            [
                                Ident {
                                    value: "T7",
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
                Between {
                    expr: Value(
                        SingleQuotedString(
                            "HJeihfbwei",
                        ),
                    ),
                    negated: false,
                    low: Subquery(
                        Query {
                            with: None,
                            body: Select(
                                Select {
                                    distinct: false,
                                    top: None,
                                    projection: [
                                        Wildcard,
                                        QualifiedWildcard(
                                            ObjectName(
                                                [
                                                    Ident {
                                                        value: "T4",
                                                        quote_style: None,
                                                    },
                                                ],
                                            ),
                                        ),
                                        Wildcard,
                                    ],
                                    into: None,
                                    from: [
                                        TableWithJoins {
                                            relation: Table {
                                                name: ObjectName(
                                                    [
                                                        Ident {
                                                            value: "T8",
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
                                    selection: None,
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
                                    Number(
                                        "1",
                                        false,
                                    ),
                                ),
                            ),
                            offset: None,
                            fetch: None,
                            lock: None,
                        },
                    ),
                    high: Identifier(
                        Ident {
                            value: "C1",
                            quote_style: None,
                        },
                    ),
                },
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
    limit: None,
    offset: None,
    fetch: None,
    lock: None,
}
