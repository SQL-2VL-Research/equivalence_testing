use equivalence_testing::query_creation::random_query_generator::{
    QueryGenerator, 
};


mod equivalence_testing_function;
use equivalence_testing_function::equivalence_tester;



use sqlparser::ast::{BinaryOperator, Expr, SelectItem};

use equivalence_testing::query_creation::helpers::{
    create_compound_identifier, create_select, create_table,
};
fn main() {
 
    let query0 = "select * from R;";
    println!("Query 0: {:#?}", query0.to_string());
    println!("Equivalent? {:#?}\n", equivalence_tester(query0));
    
    
    let query1 = "select R.A from R where R.A not in ( select S.A from S );";
    println!("Query 1: {:#?}", query1.to_string());
    println!("Equivalent? {:#?}\n", equivalence_tester(query1));

    let query2 = "select R.A from R where NOT EXISTS ( select S.A from S where S.A=R.A );";
    println!("Query 2: {:#?}", query2.to_string());
    println!("Equivalent? {:#?}\n", equivalence_tester(query2));

    let query3 = "select distinct X.A from R X, R Y where X.A=Y.A;";
    println!("Query 3: {:#?}", query3.to_string());
    println!("Equivalent? {:#?}\n", equivalence_tester(query3));

    let query4 = "select distinct R.A from R;";
    println!("Query 4: {:#?}", query4.to_string());
    println!("Equivalent? {:#?}\n", equivalence_tester(query4));

    let query = 
"SELECT 
    p_brand,
    p_type,
    p_size,
    COUNT(DISTINCT ps_suppkey) AS supplier_cnt
FROM
    partsupp,
    part
WHERE
    p_partkey = ps_partkey
    AND p_brand <> 'Brand#45'
    AND p_type NOT LIKE 'MEDIUM POLISHED%'
    AND p_size IN (49, 14, 23, 45, 19, 3, 36, 9)
    AND ps_suppkey NOT IN (
       SELECT
           s_suppkey
       FROM
           supplier
       WHERE
           s_comment LIKE '%Customer%Complaints%'
    )
GROUP BY
    p_brand,
    p_type,
    p_size
ORDER BY
    supplier_cnt DESC,
    p_brand,
    p_type,
    p_size";
    println!("Query: {:#?}", query.to_string());
    println!("Equivalent? {:#?}\n", equivalence_tester(query));
    
    let query = 
    "SELECT p_brand,
    p_type,
    p_size,
    COUNT(distinct ps_suppkey) AS supplier_cnt
    FROM tpc.partsupp
    INNER JOIN tpc.part ON p_partkey = ps_partkey
    WHERE p_brand <> 'Brand#45' AND p_type NOT LIKE 'MEDIUM POLISHED%'
    AND p_size IN (49, 14, 23, 45, 19, 3, 36, 9)
    AND ps_suppkey NOT IN (SELECT s_suppkey FROM tpc.supplier WHERE s_comment like '%Customer%Complaints%')
    GROUP BY p_brand, p_type, p_size
    ORDER BY supplier_cnt desc, p_brand, p_type, p_size";
    println!("Query: {:#?}", query.to_string());
    println!("Equivalent? {:#?}\n", equivalence_tester(query));
    
    let query = 
    "SELECT `brand`,
    `product_type`,
    `size`,
    COUNT(distinct `supplied_by[supplier].supplier_key`) AS supplier_cnt
    FROM `dtimbr`.`product`
    WHERE `brand` <> 'Brand#45'
    AND `product_type` NOT LIKE 'MEDIUM POLISHED%'
    AND `size` IN (49, 14, 23, 45, 19, 3, 36, 9)
    AND `supplied_by[supplier].supplier_comment` NOT LIKE '%Customer%Complaints%'
    GROUP BY `brand`, `product_type`, `size`
    ORDER BY `supplier_cnt` DESC, `brand` ,`product_type` ,`size`";
    println!("Query: {:#?}", query.to_string());
    println!("Equivalent? {:#?}\n", equivalence_tester(query));
    
    
    
    /*
    let mut generator = QueryGenerator::new(QueryGeneratorParams::default());
    for _ in 0..20 {
        let query = generator.generate();
        println!("Generated query: {:#?}", query.to_string());
    }
    */
}
