-- SqlPlannerSpec.hs — conformance and unit tests for the Haskell sql-planner.
--
-- Test organisation mirrors the other language suites:
--   C1–C13  Conformance tests
--   Struct   Structural tests
--   Error    Error-path tests
--   Expr     Expression-type and error-propagation tests

module SqlPlannerSpec (spec) where

import Test.Hspec
import SqlPlanner

-- ── Schema fixture ────────────────────────────────────────────────────────────

testSchema :: SchemaProvider
testSchema = inMemorySchema
    [ ("users",    ["id", "name", "age", "email"])
    , ("orders",   ["id", "user_id", "amount", "status"])
    , ("products", ["id", "name", "price", "category"])
    ]

-- ── Helpers ───────────────────────────────────────────────────────────────────

-- | Bare SELECT * FROM <table>.
selectStar :: String -> Statement
selectStar tbl = SelectStmt
    { stmtDistinct = False
    , stmtColumns  = [OutputStar]
    , stmtFrom     = [TableRef tbl Nothing]
    , stmtJoins    = []
    , stmtWhere    = Nothing
    , stmtGroupBy  = []
    , stmtHaving   = Nothing
    , stmtOrderBy  = []
    , stmtLimit    = Nothing
    }

selectStarWhere :: String -> SqlExpr -> Statement
selectStarWhere tbl w = (selectStar tbl) { stmtWhere = Just w }

col :: String -> OutputColumn
col c = OutputExpr (Column Nothing c) Nothing

colAs :: String -> String -> OutputColumn
colAs c a = OutputExpr (Column Nothing c) (Just a)

-- ── C1 — SELECT * FROM users ──────────────────────────────────────────────────

specC1 :: Spec
specC1 = it "C1: SELECT * FROM users produces Scan + Project" $ do
    let result = plan testSchema (selectStar "users")
    result `shouldSatisfy` \r -> case r of
        Right (Project (Scan "users" Nothing) [OutputStar]) -> True
        _ -> False

-- ── C2 — WHERE clause ────────────────────────────────────────────────────────

specC2 :: Spec
specC2 = it "C2: WHERE age > 18 produces Filter node" $ do
    let w = BinaryOp BinGt (Column Nothing "age") (Literal (Just (LitInt 18)))
    let result = plan testSchema (selectStarWhere "users" w)
    result `shouldSatisfy` \r -> case r of
        Right (Project (Filter (Scan "users" Nothing) (BinaryOp BinGt (Column (Just "users") "age") _)) _) -> True
        _ -> False

-- ── C3 — column list ─────────────────────────────────────────────────────────

specC3 :: Spec
specC3 = it "C3: SELECT id, name resolves two columns" $ do
    let stmt = (selectStar "users") { stmtColumns = [col "id", col "name"] }
    case plan testSchema stmt of
        Left e -> expectationFailure (show e)
        Right (Project _ cols) -> length cols `shouldBe` 2
        Right p -> expectationFailure ("unexpected plan: " ++ show p)

-- ── C4 — alias ───────────────────────────────────────────────────────────────

specC4 :: Spec
specC4 = it "C4: SELECT name AS n preserves alias" $ do
    let stmt = (selectStar "users") { stmtColumns = [colAs "name" "n"] }
    case plan testSchema stmt of
        Left e -> expectationFailure (show e)
        Right (Project _ [OutputExpr _ (Just "n")]) -> return ()
        Right p -> expectationFailure ("unexpected plan: " ++ show p)

-- ── C5 — ORDER BY ────────────────────────────────────────────────────────────

specC5 :: Spec
specC5 = it "C5: ORDER BY name ASC produces Sort node" $ do
    let stmt = (selectStar "users")
            { stmtOrderBy = [SortKey (Column Nothing "name") SortAsc NullsLast] }
    case plan testSchema stmt of
        Left e -> expectationFailure (show e)
        Right (Project (Sort _ [SortKey _ SortAsc NullsLast]) _) -> return ()
        Right p -> expectationFailure ("unexpected plan: " ++ show p)

-- ── C6 — LIMIT ───────────────────────────────────────────────────────────────

specC6 :: Spec
specC6 = it "C6: LIMIT 10 produces Limit node" $ do
    let stmt = (selectStar "users") { stmtLimit = Just (LimitClause (Just 10) Nothing) }
    case plan testSchema stmt of
        Left e -> expectationFailure (show e)
        Right (Project (Limit _ (Just 10) Nothing) _) -> return ()
        Right p -> expectationFailure ("unexpected plan: " ++ show p)

-- ── C7 — DISTINCT ────────────────────────────────────────────────────────────

specC7 :: Spec
specC7 = it "C7: DISTINCT produces Distinct node" $ do
    let stmt = (selectStar "users") { stmtDistinct = True, stmtColumns = [col "name"] }
    case plan testSchema stmt of
        Left e -> expectationFailure (show e)
        Right (Project (Distinct _) _) -> return ()
        Right p -> expectationFailure ("unexpected plan: " ++ show p)

-- ── C8 — aggregate ───────────────────────────────────────────────────────────

specC8 :: Spec
specC8 = it "C8: COUNT(*) GROUP BY age produces Aggregate node" $ do
    let countStar = AggExpr AggCount AggStar False
    let stmt = (selectStar "users")
            { stmtColumns = [OutputExpr countStar Nothing]
            , stmtGroupBy = [Column Nothing "age"]
            }
    case plan testSchema stmt of
        Left e -> expectationFailure (show e)
        Right (Project (Aggregate _ _ aggs) _) ->
            length aggs `shouldSatisfy` (> 0)
        Right p -> expectationFailure ("unexpected plan: " ++ show p)

-- ── C9 — HAVING ──────────────────────────────────────────────────────────────

specC9 :: Spec
specC9 = it "C9: HAVING produces Having node after Aggregate" $ do
    let countStar = AggExpr AggCount AggStar False
    let having    = BinaryOp BinGt countStar (Literal (Just (LitInt 5)))
    let stmt = (selectStar "users")
            { stmtColumns = [col "age"]
            , stmtGroupBy = [Column Nothing "age"]
            , stmtHaving  = Just having
            }
    case plan testSchema stmt of
        Left e -> expectationFailure (show e)
        Right (Project (Having (Aggregate _ _ _) _) _) -> return ()
        Right p -> expectationFailure ("unexpected plan: " ++ show p)

-- ── C10 — INSERT ─────────────────────────────────────────────────────────────

specC10 :: Spec
specC10 = it "C10: INSERT into known table produces InsertPlan" $ do
    let stmt = InsertStmt "users" ["id", "name", "age", "email"]
            [[Literal (Just (LitInt 1)), Literal (Just (LitText "Alice")),
              Literal (Just (LitInt 30)), Literal (Just (LitText "a@b.com"))]]
    case plan testSchema stmt of
        Left e -> expectationFailure (show e)
        Right (InsertPlan "users" cols _) -> length cols `shouldBe` 4
        Right p -> expectationFailure ("unexpected plan: " ++ show p)

-- ── C11 — UPDATE ─────────────────────────────────────────────────────────────

specC11 :: Spec
specC11 = it "C11: UPDATE produces UpdatePlan" $ do
    let stmt = UpdateStmt "users"
            [Assignment "age" (Literal (Just (LitInt 31)))]
            (Just (BinaryOp BinEq (Column Nothing "id") (Literal (Just (LitInt 1)))))
    case plan testSchema stmt of
        Left e -> expectationFailure (show e)
        Right (UpdatePlan "users" asgns _) -> length asgns `shouldBe` 1
        Right p -> expectationFailure ("unexpected plan: " ++ show p)

-- ── C12 — DELETE ─────────────────────────────────────────────────────────────

specC12 :: Spec
specC12 = it "C12: DELETE produces DeletePlan" $ do
    let stmt = DeleteStmt "users"
            (Just (BinaryOp BinEq (Column Nothing "id") (Literal (Just (LitInt 1)))))
    case plan testSchema stmt of
        Left e -> expectationFailure (show e)
        Right (DeletePlan "users" (Just _)) -> return ()
        Right p -> expectationFailure ("unexpected plan: " ++ show p)

-- ── C13 — DDL ────────────────────────────────────────────────────────────────

specC13 :: Spec
specC13 = do
    it "C13a: CREATE TABLE produces CreateTablePlan" $ do
        let stmt = CreateTableStmt "logs" False
                [ColumnDef "id" "INTEGER" True True False Nothing]
        plan testSchema stmt `shouldBe`
            Right (CreateTablePlan "logs" False [ColumnDef "id" "INTEGER" True True False Nothing])

    it "C13b: DROP TABLE produces DropTablePlan" $ do
        plan testSchema (DropTableStmt "logs" True) `shouldBe`
            Right (DropTablePlan "logs" True)

-- ── Structural tests ──────────────────────────────────────────────────────────

specStruct :: Spec
specStruct = do
    it "Struct: multi-FROM generates CROSS JOIN" $ do
        let stmt = (selectStar "users")
                { stmtFrom = [TableRef "users" Nothing, TableRef "orders" Nothing] }
        case plan testSchema stmt of
            Left e -> expectationFailure (show e)
            Right (Project (JoinPlan _ _ JoinCross Nothing) _) -> return ()
            Right p -> expectationFailure ("unexpected plan: " ++ show p)

    it "Struct: INNER JOIN on condition" $ do
        let on  = BinaryOp BinEq (Column (Just "users") "id") (Column (Just "orders") "user_id")
        let jc  = JoinClause JoinInner "orders" Nothing (Just on)
        let stmt = (selectStar "users") { stmtJoins = [jc] }
        case plan testSchema stmt of
            Left e -> expectationFailure (show e)
            Right (Project (JoinPlan _ _ JoinInner _) _) -> return ()
            Right p -> expectationFailure ("unexpected plan: " ++ show p)

    it "Struct: table alias resolves correctly" $ do
        let stmt = SelectStmt False
                [OutputExpr (Column (Just "u") "name") Nothing]
                [TableRef "users" (Just "u")] [] Nothing [] Nothing [] Nothing
        case plan testSchema stmt of
            Left e -> expectationFailure (show e)
            Right (Project _ [OutputExpr (Column (Just "u") "name") Nothing]) -> return ()
            Right p -> expectationFailure ("unexpected plan: " ++ show p)

    it "Struct: DISTINCT + ORDER BY + LIMIT stacking" $ do
        let stmt = SelectStmt True [col "name"]
                [TableRef "users" Nothing] [] Nothing [] Nothing
                [SortKey (Column Nothing "name") SortDesc NullsLast]
                (Just (LimitClause (Just 5) (Just 2)))
        case plan testSchema stmt of
            Left e -> expectationFailure (show e)
            Right (Project (Limit (Sort (Distinct _) _) (Just 5) (Just 2)) _) -> return ()
            Right p -> expectationFailure ("unexpected plan: " ++ show p)

    it "Struct: planAll returns one plan per statement" $ do
        let stmts = [selectStar "users", DropTableStmt "nope" True]
        case planAll testSchema stmts of
            Left e -> expectationFailure (show e)
            Right plans -> length plans `shouldBe` 2

-- ── Error tests ───────────────────────────────────────────────────────────────

specErrors :: Spec
specErrors = do
    it "Error: unknown table in FROM" $
        plan testSchema (selectStar "ghost") `shouldBe` Left (UnknownTable "ghost")

    it "Error: unknown column in WHERE" $ do
        let w = BinaryOp BinEq (Column Nothing "no_such_col") (Literal (Just (LitInt 1)))
        plan testSchema (selectStarWhere "users" w)
            `shouldBe` Left (UnknownColumn Nothing "no_such_col")

    it "Error: ambiguous unqualified column" $ do
        let stmt = SelectStmt False
                [OutputExpr (Column Nothing "id") Nothing]
                [TableRef "users" Nothing, TableRef "orders" Nothing]
                [] Nothing [] Nothing [] Nothing
        case plan testSchema stmt of
            Left (AmbiguousColumn "id" tbls) -> length tbls `shouldSatisfy` (>= 2)
            other -> expectationFailure ("expected AmbiguousColumn, got: " ++ show other)

    it "Error: qualified column against unknown alias" $ do
        let stmt = SelectStmt False
                [OutputExpr (Column (Just "x") "id") Nothing]
                [TableRef "users" Nothing] [] Nothing [] Nothing [] Nothing
        plan testSchema stmt `shouldBe` Left (UnknownTable "x")

    it "Error: INSERT into unknown table" $
        plan testSchema (InsertStmt "nope" ["id"] [[Literal (Just (LitInt 1))]])
            `shouldBe` Left (UnknownTable "nope")

    it "Error: UPDATE on unknown table" $
        plan testSchema (UpdateStmt "nope" [Assignment "id" (Literal (Just (LitInt 1)))] Nothing)
            `shouldBe` Left (UnknownTable "nope")

    it "Error: DELETE from unknown table" $
        plan testSchema (DeleteStmt "nope" Nothing)
            `shouldBe` Left (UnknownTable "nope")

-- ── Expression-type tests ─────────────────────────────────────────────────────

specExprs :: Spec
specExprs = do
    it "Expr: IS NULL predicate resolves inner column" $ do
        let w = IsNull (Column Nothing "email")
        case plan testSchema (selectStarWhere "users" w) of
            Right (Project (Filter _ (IsNull (Column (Just "users") "email"))) _) -> return ()
            other -> expectationFailure ("unexpected: " ++ show other)

    it "Expr: IS NOT NULL resolves inner column" $ do
        let w = IsNotNull (Column Nothing "email")
        case plan testSchema (selectStarWhere "users" w) of
            Right (Project (Filter _ (IsNotNull _)) _) -> return ()
            other -> expectationFailure (show other)

    it "Expr: BETWEEN resolves value, lo, hi" $ do
        let w = Between (Column Nothing "age") (Literal (Just (LitInt 18))) (Literal (Just (LitInt 65)))
        case plan testSchema (selectStarWhere "users" w) of
            Right (Project (Filter _ (Between (Column (Just "users") "age") _ _)) _) -> return ()
            other -> expectationFailure (show other)

    it "Expr: IN resolves value and items" $ do
        let w = InExpr (Column Nothing "age") [Literal (Just (LitInt 20)), Literal (Just (LitInt 30))]
        case plan testSchema (selectStarWhere "users" w) of
            Right (Project (Filter _ (InExpr _ items)) _) -> length items `shouldBe` 2
            other -> expectationFailure (show other)

    it "Expr: NOT IN resolves value and items" $ do
        let w = NotInExpr (Column Nothing "age") [Literal (Just (LitInt 0))]
        case plan testSchema (selectStarWhere "users" w) of
            Right (Project (Filter _ (NotInExpr _ _)) _) -> return ()
            other -> expectationFailure (show other)

    it "Expr: LIKE resolves value column" $ do
        let w = Like (Column Nothing "name") "%Alice%"
        case plan testSchema (selectStarWhere "users" w) of
            Right (Project (Filter _ (Like _ "%Alice%")) _) -> return ()
            other -> expectationFailure (show other)

    it "Expr: NOT LIKE resolves value column" $ do
        let w = NotLike (Column Nothing "name") "%Bob%"
        case plan testSchema (selectStarWhere "users" w) of
            Right (Project (Filter _ (NotLike _ _)) _) -> return ()
            other -> expectationFailure (show other)

    it "Expr: Unary NOT resolves operand" $ do
        let w = UnaryOp UnaryNot (BinaryOp BinEq (Column Nothing "age") (Literal (Just (LitInt 0))))
        case plan testSchema (selectStarWhere "users" w) of
            Right (Project (Filter _ (UnaryOp UnaryNot _)) _) -> return ()
            other -> expectationFailure (show other)

    it "Expr: FuncCall resolves argument columns" $ do
        let w = BinaryOp BinGt (FuncCall "LENGTH" [Column Nothing "name"]) (Literal (Just (LitInt 3)))
        case plan testSchema (selectStarWhere "users" w) of
            Right (Project (Filter _ (BinaryOp _ (FuncCall "LENGTH" _) _)) _) -> return ()
            other -> expectationFailure (show other)

    -- Error propagation
    it "Expr error: BETWEEN bad value column" $
        plan testSchema (selectStarWhere "users"
            (Between (Column Nothing "ghost") (Literal (Just (LitInt 1))) (Literal (Just (LitInt 10)))))
            `shouldBe` Left (UnknownColumn Nothing "ghost")

    it "Expr error: BETWEEN bad lo column" $
        plan testSchema (selectStarWhere "users"
            (Between (Column Nothing "age") (Column Nothing "ghost_lo") (Literal (Just (LitInt 10)))))
            `shouldBe` Left (UnknownColumn Nothing "ghost_lo")

    it "Expr error: BETWEEN bad hi column" $
        plan testSchema (selectStarWhere "users"
            (Between (Column Nothing "age") (Literal (Just (LitInt 1))) (Column Nothing "ghost_hi")))
            `shouldBe` Left (UnknownColumn Nothing "ghost_hi")

    it "Expr error: IN bad item column" $
        plan testSchema (selectStarWhere "users"
            (InExpr (Column Nothing "age") [Column Nothing "ghost_col"]))
            `shouldBe` Left (UnknownColumn Nothing "ghost_col")

    it "Expr error: NOT IN bad item column" $
        plan testSchema (selectStarWhere "users"
            (NotInExpr (Column Nothing "age") [Column Nothing "ghost_col"]))
            `shouldBe` Left (UnknownColumn Nothing "ghost_col")

    it "Expr error: FuncCall bad arg column" $
        plan testSchema (selectStarWhere "users"
            (BinaryOp BinGt (FuncCall "LENGTH" [Column Nothing "ghost_col"]) (Literal (Just (LitInt 3)))))
            `shouldBe` Left (UnknownColumn Nothing "ghost_col")

    it "Expr error: IS NULL bad inner column" $
        plan testSchema (selectStarWhere "users" (IsNull (Column Nothing "ghost_col")))
            `shouldBe` Left (UnknownColumn Nothing "ghost_col")

    it "Expr error: IS NOT NULL bad inner column" $
        plan testSchema (selectStarWhere "users" (IsNotNull (Column Nothing "ghost_col")))
            `shouldBe` Left (UnknownColumn Nothing "ghost_col")

    it "Expr error: LIKE bad value column" $
        plan testSchema (selectStarWhere "users" (Like (Column Nothing "ghost_col") "%x%"))
            `shouldBe` Left (UnknownColumn Nothing "ghost_col")

    it "Expr error: NOT LIKE bad value column" $
        plan testSchema (selectStarWhere "users" (NotLike (Column Nothing "ghost_col") "%x%"))
            `shouldBe` Left (UnknownColumn Nothing "ghost_col")

    it "Expr: Literal null value preserved" $ do
        plan testSchema (selectStarWhere "users" (IsNull (Literal Nothing)))
            `shouldSatisfy` \r -> case r of
                Right (Project (Filter _ (IsNull (Literal Nothing))) _) -> True
                _ -> False

    it "Aggregate: SUM with no GROUP BY produces Aggregate node" $ do
        let sum_ = AggExpr AggSum (AggExprArg (Column Nothing "amount")) False
        let stmt = (selectStar "orders")
                { stmtColumns = [OutputExpr sum_ Nothing] }
        case plan testSchema stmt of
            Right (Project (Aggregate _ [] _) _) -> return ()
            other -> expectationFailure (show other)

    it "Aggregate: COUNT DISTINCT" $ do
        let cd   = AggExpr AggCount (AggExprArg (Column Nothing "name")) True
        let stmt = (selectStar "users")
                { stmtColumns = [OutputExpr cd Nothing] }
        case plan testSchema stmt of
            Right (Project (Aggregate _ _ aggs) _) ->
                aggDistinct (head aggs) `shouldBe` True
            other -> expectationFailure (show other)

-- ── Top-level spec ────────────────────────────────────────────────────────────

spec :: Spec
spec = do
    describe "Conformance" $ do
        specC1; specC2; specC3; specC4; specC5; specC6; specC7
        specC8; specC9; specC10; specC11; specC12; specC13
    describe "Structural" specStruct
    describe "Errors"     specErrors
    describe "Expressions" specExprs
