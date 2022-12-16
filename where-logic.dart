ANY = [column, null, query(single value), numeric, boolean, array, list expr, string]
ANY_STRING = [column, null, query(single value), string]
ANY_BOOLEAN = [column, null, query(single value), boolean]
ANY_NUMERIC = [column, null, query(single value), numeric]
ANY_BETWEEN = ANY - [array, boolean]
ANY_IN_SUBQUERY = ANY - [list expr]

2vl:
IsNull, IsDistinctFrom, Exists

3vl:
InList, InSubquery, Between, BinaryComp, BinaryStringLike, BinaryBooleanOp, UnaryNotOp

numeric:
BinaryNumericOp, UnaryNumericOp, Position

string:
BinaryStringConcat, Trim, Substring

other:
AnyOp, AllOp, Nested

IsNull (ANY) 1 [neg]

IsDistinctFrom (ANY ||| SAME TYPE OR NULL) 2 [neg]

InList (ANY ||| list expr OF SAME TYPE OR NULL) 2 [neg]

InSubquery (ANY_IN_SUBQUERY ||| subquery OF SAME TYPE OR NULL) 2 [neg]

Between (ANY_BETWEEN ||| SAME TYPE OR NULL ||| SAME TYPE OR NULL) 3 [neg]

BinaryOp (<(Gt, Lt), =(Eq), <=(GtEq, LtEq), <>(NotEq)) (ANY ||| SAME TYPE OR NULL) 2

BinaryOp (Like, NotLike) (ANY_STRING ||| SAME TYPE OR NULL) 2

BinaryOp (StringConcat) (ANY_STRING ||| SAME TYPE OR NULL) 2

BinaryOp (And, Or, Xor) (ANY_BOOLEAN ||| SAME TYPE OR NULL) 2

BinaryOp (BitwiseOr, BitwiseAnd, BitwiseXor) (ANY_NUMERIC ||| SAME TYPE OR NULL) 2

AnyOp 1 (ANY ||| <, =, <=, <> ||| array / subquery (SAME TYPE)) (only the right side of BinaryOp)
AnyOp 1 (list expr (ANY) ||| <, =, <=, <> ||| array / subquery (SAME TYPE)) (only the right side of BinaryOp)
AllOp 1 (ANY ||| <, =, <=, <> ||| array / subquery (SAME TYPE)) (only the right side of BinaryOp)
AllOp 1 (list expr (ANY) ||| <, =, <=, <> ||| array / subquery (SAME TYPE)) (only the right side of BinaryOp)

UnaryOp (Plus, Minus, PGBitwiseNot, PGSquareRoot, PGCubeRoot, PGPostfixFactorial, PGPrefixFactorial, PGAbs) (ANY_NUMERIC) 1

UnaryOp (Not) (ANY_BOOLEAN) 1

Position (ANY_STRING ||| ANY_STRING) -> int 2

Substring (ANY_STRING ||| int ||| optional[int]) 3

Trim ([optional] ANY_STRING + BOTH/LEADING/TRAILING ||| ANY_STRING)

Exists (subquery) 1 [neg]

Nested (ANY) 1

// IN CODE, transform IN UNNEST -> IN (SELECT UNNEST) (TODO)
// transform <> -> != (auto-done by lib?) (TODO)
// list expr is record in my notation :)
// transform <=> (MySQL) -> IS NOT DISTINCT (TODO)
// list expr type is type of its elements (when we’ll do type casting)
// type casts… int to str and backwards (TODO)
// EXTRACT MONTH FROM … (TODO)
// IS FALSE / IS TRUE -> =FALSE, =TRUE
// Add COLLATE?
// Add array index?