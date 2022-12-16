SELECT
 	s_name,
     s_address
FROM
     supplier,
     nation
WHERE
     s_suppkey IN (
		SELECT
			ps_suppkey
		FROM
			partsupp
		WHERE
			ps_partkey IN (
				SELECT
					p_partkey
				FROM
					part
				WHERE
					p_name LIKE 'forest%'
			)
			AND ps_availqty > (
				SELECT
					0.5 * SUM(l_quantity)
				FROM
					lineitem
				WHERE
					l_partkey = ps_partkey
					AND l_suppkey = ps_suppkey
					AND l_shipdate >= MDY(1,1,1994)
					AND l_shipdate < MDY(1,1,1994) + 1 -- UNITS YEAR
			)
     )
     AND s_nationkey = n_nationkey
     AND n_name = 'CANADA'
ORDER BY
     s_name;