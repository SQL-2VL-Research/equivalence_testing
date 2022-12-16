SELECT
     SUM(l_extendedprice * l_discount) AS revenue
FROM
     lineitem
WHERE
     l_shipdate >= MDY(1,1,1994)
     AND l_shipdate < MDY(1,1,1994) + 1 -- UNITS YEAR
     AND l_discount BETWEEN .06 - 0.01 AND .06 + 0.010001
     AND l_quantity < 24;