to_comment = """
FROM_cartesian_product -> FROM_table
FROM_cartesian_product -> call0_Query

limit_num [label="limit [num]\\nmod: 'single row' -> OFF", modifier="single row", modifier_mode="off"]
LIMIT -> limit_num
call52_types [TYPES="[integer, numeric, bigint]", MODS="[no column spec, no aggregate]", label="TYPES: [integer, numeric, bigint]\\nMODS: [no column spec, no aggregate]", shape=rectangle, style=filled, color=lightblue]
limit_num -> call52_types
call52_types -> EXIT_LIMIT

types_return_typed_null [label="Typed null\\nmod.: 'no typed nulls' -> OFF", modifier="no typed nulls", modifier_mode="off"]
types_select_type_bigint -> types_return_typed_null
types_select_type_integer -> types_return_typed_null
types_select_type_numeric -> types_return_typed_null
types_select_type_3vl -> types_return_typed_null
types_select_type_text -> types_return_typed_null
types_select_type_date -> types_return_typed_null
types_select_type_interval -> types_return_typed_null
types_return_typed_null -> EXIT_types

set_list_empty_allowed -> set_list

call69_types -> set_list
"""

graph = open("graph.dot", 'r').read()

for line in to_comment.split('\n'):
    if line != '':
        new_line = "// " + line
        if graph.find(line) == -1:
            print(f"Failed to find:\n{line}")
        graph = graph.replace(line, new_line)

open("performance_untrained_graph.dot", 'w').write(graph)

