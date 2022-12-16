# equivalence_testing

Run `cargo run graph.dot 2` to generate 2 queries using a graph from `graph.dot`.\
\
**Warning: the generation currently stack overflows.**\
This effect was mitigated with the `LiteralSelector`, but not altogether removed. The `DynamicManager` implementation that would make the geneartion work is a `work-in-progress`. The effect would not be present after we train the chain, which is in our to-do list. Thanks!

## Query generator graph 
![Query generator graph](query-generator-graph/graph%2025_09.svg)
