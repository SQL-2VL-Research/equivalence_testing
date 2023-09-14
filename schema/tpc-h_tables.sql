CREATE TABLE PART (
  P_PARTKEY        integer,
  P_NAME           varchar(55),
  P_MFGR           char(25),
  P_BRAND          char(10),
  P_TYPE           varchar(25),
  P_SIZE           integer,
  P_CONTAINER      char(10),
  P_RETAILPRICE    numeric,
  P_COMMENT        varchar(23)
);

CREATE TABLE SUPPLIER (
  S_SUPPKEY        integer,
  S_NAME           char(25),
  S_ADDRESS        varchar(40),
  S_NATIONKEY      integer,
  S_PHONE          char(15),
  S_ACCTBAL        numeric,
  S_COMMENT        varchar(101)
);

CREATE TABLE PARTSUPP (
  PS_PARTKEY       integer,
  PS_SUPPKEY       integer,
  PS_AVAILQTY      integer,
  PS_SUPPLYCOST    numeric,
  PS_COMMENT       varchar(199)
);

CREATE TABLE CUSTOMER (
  C_CUSTKEY        integer,
  C_NAME           varchar(25),
  C_ADDRESS        varchar(40),
  C_NATIONKEY      integer,
  C_PHONE          char(15),
  C_ACCTBAL        numeric,
  C_MKTSEGMENT     char(10),
  C_COMMENT        varchar(117)
);

CREATE TABLE ORDERS (
  O_ORDERKEY       integer,
  O_CUSTKEY        integer,
  O_ORDERSTATUS    char(1),
  O_TOTALPRICE     numeric,
  O_ORDERDATE      date,  -- integer
  O_ORDERPRIORITY  char(15),
  O_CLERK          char(15),
  O_SHIPPRIORITY   integer,
  O_COMMENT        varchar(79)
);

CREATE TABLE LINEITEM (
  L_ORDERKEY       integer,
  L_PARTKEY        integer,
  L_SUPPKEY        integer,
  L_LINENUMBER     integer,
  L_QUANTITY       numeric,
  L_EXTENDEDPRICE  numeric,
  L_DISCOUNT       numeric,
  L_TAX            numeric,
  L_RETURNFLAG     char(1),
  L_LINESTATUS     char(1),
  L_SHIPDATE       date,  -- integer
  L_COMMITDATE     date,  -- integer
  L_RECEIPTDATE    date,  -- integer
  L_SHIPINSTRUCT   char(25),
  L_SHIPMODE       char(10),
  L_COMMENT        varchar(44)
);

CREATE TABLE NATION (
  N_NATIONKEY      integer,
  N_NAME           char(25),
  N_REGIONKEY      integer,
  N_COMMENT        varchar(152)
);

CREATE TABLE REGION (
  R_REGIONKEY      integer,
  R_NAME           char(25),
  R_COMMENT        varchar(152)
);
