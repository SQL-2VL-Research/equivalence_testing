INSERT INTO PART VALUES (1, 'Part 1', 'Manufacturer 1', 'Brand 1', 'Type 1', 100, 'Ctr 1', 200.00, 'Comment 1');
INSERT INTO SUPPLIER VALUES (1, 'Supplier 1', 'Address 1', 1, '123-456-7890', 1000.00, 'Comment 2');

INSERT INTO PART (P_PARTKEY, P_NAME, P_MFGR, P_BRAND, P_TYPE, P_SIZE, P_CONTAINER, P_RETAILPRICE, P_COMMENT) VALUES 
(1, 'Part1', 'Manufacturer1', 'Brand1', 'Type1', 10, 'Container1', 100.00, 'Part1 Comment'),
(2, 'Part2', 'Manufacturer2', 'Brand2', 'Type2', 20, 'Container2', 200.00, 'Part2 Comment'),
(3, 'Part3', 'Manufacturer3', 'Brand3', 'Type3', 30, 'Container3', 300.00, 'Part3 Comment'),
(4, 'Part4', 'Manufacturer4', 'Brand4', 'Type4', 40, 'Container4', 400.00, 'Part4 Comment');

INSERT INTO SUPPLIER (S_SUPPKEY, S_NAME, S_ADDRESS, S_NATIONKEY, S_PHONE, S_ACCTBAL, S_COMMENT) VALUES 
(1, 'Supplier1', 'Address1', 1, '111-111-1111', 1000.00, 'Supplier1 Comment'),
(2, 'Supplier2', 'Address2', 2, '222-222-2222', 2000.00, 'Supplier2 Comment'),
(3, 'Supplier3', 'Address3', 3, '333-333-3333', 3000.00, 'Supplier3 Comment'),
(4, 'Supplier4', 'Address4', 4, '444-444-4444', 4000.00, 'Supplier4 Comment');

INSERT INTO PARTSUPP VALUES (1, 1, 50, 500.00, 'Comment 3');
INSERT INTO CUSTOMER VALUES (1, 'Customer 1', 'Address 2', 1, '098-765-4321', 1500.00, 'Segment 1', 'Comment 4');

INSERT INTO PARTSUPP (PS_PARTKEY, PS_SUPPKEY, PS_AVAILQTY, PS_SUPPLYCOST, PS_COMMENT) VALUES 
(1, 1, 100, 10.00, 'PartSupp1 Comment'),
(2, 2, 200, 20.00, 'PartSupp2 Comment'),
(3, 3, 300, 30.00, 'PartSupp3 Comment'),
(4, 4, 400, 40.00, 'PartSupp4 Comment');

INSERT INTO CUSTOMER (C_CUSTKEY, C_NAME, C_ADDRESS, C_NATIONKEY, C_PHONE, C_ACCTBAL, C_MKTSEGMENT, C_COMMENT) VALUES 
(1, 'Customer1', 'CustomerAddress1', 1, '555-555-5555', 1000.00, 'Segment1', 'Customer1 Comment'),
(2, 'Customer2', 'CustomerAddress2', 2, '666-666-6666', 2000.00, 'Segment2', 'Customer2 Comment'),
(3, 'Customer3', 'CustomerAddress3', 3, '777-777-7777', 3000.00, 'Segment3', 'Customer3 Comment'),
(4, 'Customer4', 'CustomerAddress4', 4, '888-888-8888', 4000.00, 'Segment4', 'Customer4 Comment');

INSERT INTO ORDERS VALUES (1, 1, 'O', 1000.00, '2023-07-12', 'High', 'Clerk 1', 10, 'Comment 5');
INSERT INTO LINEITEM VALUES (1, 1, 1, 1, 10, 1000.00, 0.10, 0.10, 'R', 'O', '2023-07-12', '2023-07-12', '2023-07-12', 'Ship Instruct 1', 'shmode 1', 'Comment 6');

INSERT INTO ORDERS (O_ORDERKEY, O_CUSTKEY, O_ORDERSTATUS, O_TOTALPRICE, O_ORDERDATE, O_ORDERPRIORITY, O_CLERK, O_SHIPPRIORITY, O_COMMENT) VALUES 
(1, 1, 'O', 100.00, '2023-08-01', 'HIGH', 'Clerk1', 1, 'Order1 Comment'),
(2, 2, 'F', 200.00, '2023-08-02', 'LOW', 'Clerk2', 2, 'Order2 Comment'),
(3, 3, 'P', 300.00, '2023-08-03', 'MEDIUM', 'Clerk3', 3, 'Order3 Comment'),
(4, 4, 'O', 400.00, '2023-08-04', 'HIGH', 'Clerk4', 4, 'Order4 Comment');

INSERT INTO LINEITEM (L_ORDERKEY, L_PARTKEY, L_SUPPKEY, L_LINENUMBER, L_QUANTITY, L_EXTENDEDPRICE, L_DISCOUNT, L_TAX, L_RETURNFLAG, L_LINESTATUS, L_SHIPDATE, L_COMMITDATE, L_RECEIPTDATE, L_SHIPINSTRUCT, L_SHIPMODE, L_COMMENT) VALUES 
(1, 1, 1, 1, 10, 100.00, 0.05, 0.10, 'R', 'F', '2023-08-01', '2023-08-05', '2023-08-10', 'DELIVER IN PERSON', 'TRUCK', 'LineItem1 Comment'),
(2, 2, 2, 2, 20, 200.00, 0.10, 0.20, 'N', 'O', '2023-08-02', '2023-08-06', '2023-08-11', 'COLLECT BY CUSTOMER', 'AIR', 'LineItem2 Comment'),
(3, 3, 3, 3, 30, 300.00, 0.15, 0.30, 'R', 'F', '2023-08-03', '2023-08-07', '2023-08-12', 'DELIVER IN PERSON', 'TRUCK', 'LineItem3 Comment'),
(4, 4, 4, 4, 40, 400.00, 0.20, 0.40, 'N', 'O', '2023-08-04', '2023-08-08', '2023-08-13', 'COLLECT BY CUSTOMER', 'AIR', 'LineItem4 Comment');

INSERT INTO NATION VALUES (1, 'Nation 1', 1, 'Comment 7');
INSERT INTO REGION VALUES (1, 'Region 1', 'Comment 8');


INSERT INTO NATION (N_NATIONKEY, N_NAME, N_REGIONKEY, N_COMMENT) VALUES
(1, 'United States', 1, 'Nation 1 Comment'),
(2, 'Canada', 1, 'Nation 2 Comment'),
(3, 'Mexico', 1, 'Nation 3 Comment'),
(4, 'United Kingdom', 2, 'Nation 4 Comment'),
(5, 'France', 2, 'Nation 5 Comment'),
(6, 'Germany', 2, 'Nation 6 Comment');

INSERT INTO REGION (R_REGIONKEY, R_NAME, R_COMMENT) VALUES
(1, 'North America', 'Region 1 Comment'),
(2, 'Europe', 'Region 2 Comment'),
(3, 'Asia', 'Region 3 Comment'),
(4, 'Africa', 'Region 4 Comment'),
(5, 'South America', 'Region 5 Comment');
