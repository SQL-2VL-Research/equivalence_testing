-- PRIMARY KEYS
ALTER TABLE PART    
  ADD PRIMARY KEY (P_PARTKEY);
ALTER TABLE SUPPLIER
  ADD PRIMARY KEY (S_SUPPKEY);
ALTER TABLE PARTSUPP
  ADD PRIMARY KEY (PS_PARTKEY, PS_SUPPKEY);
ALTER TABLE CUSTOMER
  ADD PRIMARY KEY (C_CUSTKEY);
ALTER TABLE ORDERS  
  ADD PRIMARY KEY (O_ORDERKEY);
ALTER TABLE LINEITEM
  ADD PRIMARY KEY (L_ORDERKEY, L_LINENUMBER);
ALTER TABLE NATION  
  ADD PRIMARY KEY (N_NATIONKEY);
ALTER TABLE REGION  
  ADD PRIMARY KEY (R_REGIONKEY);

-- FOREIGN KEYS
ALTER TABLE PARTSUPP
  ADD FOREIGN KEY (PS_PARTKEY)
  REFERENCES PART (P_PARTKEY);
ALTER TABLE PARTSUPP
  ADD FOREIGN KEY (PS_SUPPKEY)
  REFERENCES SUPPLIER (S_SUPPKEY);
ALTER TABLE CUSTOMER
  ADD FOREIGN KEY (C_NATIONKEY)
  REFERENCES NATION (N_NATIONKEY);
ALTER TABLE ORDERS 
  ADD FOREIGN KEY (O_CUSTKEY)
  REFERENCES CUSTOMER (C_CUSTKEY);
ALTER TABLE LINEITEM
  ADD FOREIGN KEY (L_ORDERKEY)
  REFERENCES ORDERS (O_ORDERKEY);
ALTER TABLE LINEITEM
  ADD FOREIGN KEY (L_PARTKEY)
  REFERENCES PART (P_PARTKEY);
ALTER TABLE LINEITEM
  ADD FOREIGN KEY (L_SUPPKEY)
  REFERENCES SUPPLIER (S_SUPPKEY);
ALTER TABLE LINEITEM
  ADD FOREIGN KEY (L_PARTKEY, L_SUPPKEY)
  REFERENCES PARTSUPP (PS_PARTKEY, PS_SUPPKEY);
ALTER TABLE NATION 
  ADD FOREIGN KEY (N_REGIONKEY)
  REFERENCES REGION (R_REGIONKEY);
-- missing in clause 1.4.2.3 but specified in 1.4.1
ALTER TABLE SUPPLIER
  ADD FOREIGN KEY (S_NATIONKEY)
  REFERENCES NATION (N_NATIONKEY);

-- CHECK CONSTRAINTS
ALTER TABLE PART
  ADD CHECK (P_PARTKEY >= 0);
ALTER TABLE SUPPLIER
  ADD CHECK (S_SUPPKEY >= 0);
ALTER TABLE CUSTOMER
  ADD CHECK (C_CUSTKEY >= 0);
ALTER TABLE PARTSUPP
  ADD CHECK (PS_PARTKEY >= 0);
ALTER TABLE REGION
  ADD CHECK (R_REGIONKEY >= 0);
ALTER TABLE NATION
  ADD CHECK (N_NATIONKEY >= 0);

ALTER TABLE PART
  ADD CHECK (P_SIZE >= 0);
ALTER TABLE PART
  ADD CHECK (P_RETAILPRICE >= 0);
ALTER TABLE PARTSUPP
  ADD CHECK (PS_AVAILQTY >= 0);
ALTER TABLE PARTSUPP
  ADD CHECK (PS_SUPPLYCOST >= 0);
ALTER TABLE SUPPLIER
  ADD CHECK (S_SUPPKEY >= 0);
ALTER TABLE ORDERS
  ADD CHECK (O_TOTALPRICE >= 0);
ALTER TABLE LINEITEM
  ADD CHECK (L_QUANTITY >= 0);
ALTER TABLE LINEITEM
  ADD CHECK (L_EXTENDEDPRICE >= 0);
ALTER TABLE LINEITEM
  ADD CHECK (L_TAX >= 0);
ALTER TABLE LINEITEM
  ADD CHECK (L_DISCOUNT BETWEEN 0.00 AND 1.00);
ALTER TABLE LINEITEM
  ADD CHECK (L_SHIPDATE < L_RECEIPTDATE);