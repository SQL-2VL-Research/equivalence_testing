use rand::{prelude::ThreadRng, thread_rng, Rng};
use sqlparser::ast::{
    Expr, Ident, ObjectName, Query, Select, SelectItem, SetExpr, TableAlias, TableFactor,
    TableWithJoins, Value,
};

use super::state_generators::MarkovChainGenerator;

pub struct QueryGeneratorParams {
    /// state generator. Default: MarkovChainGenerator
    pub state_generator: MarkovChainGenerator,
    /// max number of tables (counting repetitions) mentioned
    /// in a well-defined SELECT-FROM-WHERE block, including
    /// nested subqueries
    pub max_total_tables: u32,
    /// max level of nested queries in FROM and WHERE
    pub max_nest_level: u32,
    /// max number of attributes in a SELECT clause
    pub max_select_attrs: u32,
    /// max number of atomic conditions in WHERE
    pub max_where_conds: u32,
    /// probability of generating the (n+1)th table after nth
    /// table in the FROM clause
    pub from_next_table_prob: f64,
    /// probability of generating the same table when
    /// generating a new table in FROM
    pub from_same_table_prob: f64,
    /// probability of nesting a query in the FROM clause.
    pub from_nest_prob: f64,
    /// probability of DISTINCT appearing in a SELECT clause
    pub distinct_prob: f64,
    /// probability of generating * in SELECT
    pub select_asterisk_prob: f64,
    /// probability of generating expression with alias in SELECT
    pub select_expr_with_alias_prob: f64,
    /// probability of generating qualified wildcard in SELECT
    /// under condition that wildcard in not generated
    pub select_expr_qualified_wildcard_prob: f64,
    /// probability of generating (n+1)th attribute after
    /// generating the nth one in SELECT
    pub select_next_attr_prob: f64,
    /// probability of generating column with existing name
    pub select_same_column_prob: f64,
    /// probability of generating a subcondition after
    /// generating a condition on WHERE
    pub where_subcond_prob: f64,
    /// probability of nesting a subquery instead of an atomic
    /// condition when generating a subcondition in WHERE
    pub where_nest_prob: f64,
    /// probability of generating an ISNULL around a condition
    /// in WHERE
    pub where_isull_prob: f64,
    /// probability of negating a condition in WHERE
    pub where_negation_prob: f64,
}

impl QueryGeneratorParams {
    pub fn from_generator(generator: MarkovChainGenerator) -> Self {
        Self {
            state_generator: generator,
            max_total_tables: 6,
            max_nest_level: 3,
            max_select_attrs: 3,
            max_where_conds: 8,
            from_next_table_prob: 0.1,
            from_same_table_prob: 0.1,
            from_nest_prob: 0.5,
            distinct_prob: 0.4,
            select_asterisk_prob: 0.1,
            select_expr_qualified_wildcard_prob: 0.3,
            select_expr_with_alias_prob: 0.4,
            select_next_attr_prob: 0.7,
            select_same_column_prob: 0.1,
            where_subcond_prob: 0.3,
            where_nest_prob: 0.5,
            where_isull_prob: 0.2,
            where_negation_prob: 0.5,
        }
    }
}

pub struct QueryGenerator {
    params: QueryGeneratorParams,
    rng: ThreadRng,
}

struct QueryInfo {
    /// global number of table mentions
    /// including repetitions
    table_mentions_num: u32,
    /// Remember to increase this value before
    /// and decrease after generating a subquery, to
    /// control the maximum level of nesting
    /// allowed
    current_nest_level: u32,
    /// tables used in from
    from_tables: Vec<String>,
    /// list of aliased table names used in FROM
    from_aliased_table_names: Vec<String>,
    /// used for table alias name generation in FROM
    from_free_alias_index: u32,
    /// Increased when new attribute in SELECT is added
    select_attrs_num: u32,
    /// list of column names in SELECT
    select_column_names: Vec<String>,
    /// used for table alias name generation in SELECT
    select_free_alias_index: u32,
}

impl QueryInfo {
    fn new() -> QueryInfo {
        QueryInfo {
            table_mentions_num: 0,
            current_nest_level: 1,
            from_tables: Vec::<_>::new(),
            from_aliased_table_names: Vec::<_>::new(),
            from_free_alias_index: 0,
            select_attrs_num: 0,
            select_free_alias_index: 0,
            select_column_names: Vec::<_>::new(),
        }
    }
}

impl QueryGenerator {
    pub fn new(params: QueryGeneratorParams) -> Self {
        QueryGenerator {
            params,
            rng: thread_rng(),
        }
    }

    pub fn generate(&mut self) -> Query {
        self.generate_query(&mut QueryInfo::new())
    }

    fn generate_query(&mut self, query_info: &mut QueryInfo) -> Query {
        let from = self.generate_from(query_info);
        let projection = self.generate_projection(query_info);
        let distinct = self.rng.gen_bool(self.params.distinct_prob);
        Query {
            with: None,
            body: SetExpr::Select(Box::new(Select {
                distinct: distinct,
                top: None,
                projection: projection,
                into: None,
                from: from,
                lateral_views: Vec::<_>::new(),
                selection: Some(Expr::Value(Value::Boolean(true))), // WHERE TRUE;
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

    fn generate_from(&mut self, query_info: &mut QueryInfo) -> Vec<TableWithJoins> {
        let mut from_tables = Vec::<_>::new();

        from_tables.push(self.generate_relation_with_joins(query_info));

        while query_info.table_mentions_num < self.params.max_total_tables
            && self.rng.gen_bool(self.params.from_next_table_prob)
        {
            from_tables.push(self.generate_relation_with_joins(query_info));
        }

        from_tables
    }

    fn generate_relation_with_joins(&mut self, query_info: &mut QueryInfo) -> TableWithJoins {
        TableWithJoins {
            relation: if self.rng.gen_bool(self.params.from_nest_prob)
                && query_info.current_nest_level < self.params.max_nest_level
            {
                self.generate_subquery(query_info)
            } else {
                self.generate_table(query_info)
            },
            joins: Vec::<_>::new(),
        }
    }

    fn generate_subquery(&mut self, query_info: &mut QueryInfo) -> TableFactor {
        query_info.current_nest_level += 1;
        let subquery = self.generate_query(query_info);
        query_info.current_nest_level -= 1;
        TableFactor::Derived {
            lateral: false,
            subquery: Box::new(subquery),
            alias: Some(self.generate_table_alias(query_info)),
        }
    }

    fn generate_table(&mut self, query_info: &mut QueryInfo) -> TableFactor {
        let (name, alias) = self.generate_table_name_and_alias(query_info);
        query_info.table_mentions_num += 1;
        TableFactor::Table {
            name: name,
            alias: alias,
            args: None,
            with_hints: Vec::<_>::new(),
        }
    }

    fn generate_table_name_and_alias(
        &mut self,
        query_info: &mut QueryInfo,
    ) -> (ObjectName, Option<TableAlias>) {
        let tables_num = query_info.from_tables.len();
        let (name, alias) =
            if tables_num != 0 && self.rng.gen_bool(self.params.from_same_table_prob) {
                let table_name = query_info.from_tables[self.rng.gen_range(0..tables_num)].clone();
                (table_name, Some(self.generate_table_alias(query_info)))
            } else {
                let new_name = format!("R{tables_num}");
                query_info.from_tables.push(new_name);
                (format!("R{tables_num}"), None)
            };

        query_info.from_aliased_table_names.push(name.clone());

        (ObjectName(vec![Ident::new(name)]), alias)
    }

    fn generate_table_alias(&mut self, query_info: &mut QueryInfo) -> TableAlias {
        let alias = format!("A{}", query_info.from_free_alias_index);
        query_info.from_free_alias_index += 1;
        TableAlias {
            name: Ident {
                value: alias,
                quote_style: None,
            },
            columns: Vec::<_>::new(),
        }
    }

    fn generate_projection(&mut self, query_info: &mut QueryInfo) -> Vec<SelectItem> {
        if self.rng.gen_bool(self.params.select_asterisk_prob) {
            vec![SelectItem::Wildcard]
        } else {
            let mut select_attrs = Vec::<_>::new();
            select_attrs.push(self.generate_select_item(query_info));
            while self.rng.gen_bool(self.params.select_next_attr_prob)
                && query_info.select_attrs_num < self.params.max_select_attrs
            {
                query_info.select_attrs_num += 1;
                select_attrs.push(self.generate_select_item(query_info));
            }
            select_attrs
        }
    }

    fn generate_select_item(&mut self, query_info: &mut QueryInfo) -> SelectItem {
        if self
            .rng
            .gen_bool(self.params.select_expr_qualified_wildcard_prob)
        {
            let name = ObjectName(vec![Ident::new(
                query_info.from_aliased_table_names[self
                    .rng
                    .gen_range(0..query_info.from_aliased_table_names.len())]
                .clone(),
            )]);
            SelectItem::QualifiedWildcard(name)
        } else {
            let col_ident = Expr::Identifier(Ident {
                value: self.generate_column_name(query_info).to_string(),
                quote_style: None,
            });
            if self.rng.gen_bool(self.params.select_expr_with_alias_prob) {
                SelectItem::ExprWithAlias {
                    expr: col_ident,
                    alias: Ident {
                        value: self.generate_select_alias(query_info).to_string(),
                        quote_style: None,
                    },
                }
            } else {
                SelectItem::UnnamedExpr(col_ident)
            }
        }
    }

    fn generate_column_name(&mut self, query_info: &mut QueryInfo) -> String {
        let columns_num = query_info.select_column_names.len();
        let name = if columns_num != 0 && self.rng.gen_bool(self.params.select_same_column_prob) {
            let column_name =
                query_info.select_column_names[self.rng.gen_range(0..columns_num)].clone();
            column_name
        } else {
            let new_name = format!("AT{columns_num}");
            query_info.select_column_names.push(new_name.clone());
            new_name
        };
        name.to_string()
    }

    fn generate_select_alias(&mut self, query_info: &mut QueryInfo) -> String {
        let name = {
            let new_name = format!("A{}", query_info.select_free_alias_index);
            query_info.select_free_alias_index += 1;
            new_name
        };
        name.to_string()
    }
}
