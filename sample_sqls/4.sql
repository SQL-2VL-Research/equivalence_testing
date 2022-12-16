SELECT o_orderpriority, COUNT(*) AS order_count
  FROM orders
 WHERE o_orderdate >= MDY (7, 1, 1993)
   AND o_orderdate < MDY (7, 1, 1993) + 3 -- UNITS MONTH
   AND EXISTS (
      SELECT *
        FROM lineitem
       WHERE l_orderkey = o_orderkey
         AND l_commitdate < l_receiptdate
     )
GROUP BY o_orderpriority
ORDER BY o_orderpriority;