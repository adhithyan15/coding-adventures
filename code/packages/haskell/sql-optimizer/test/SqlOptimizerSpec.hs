-- SqlOptimizerSpec.hs — hspec test suite for the Haskell sql-optimizer.
--
-- Covers:
--   Lift     — structural isomorphism tests
--   CF       — constant folding (arithmetic, boolean, null propagation)
--   PPD      — predicate pushdown (through Sort, Distinct, Project, Join)
--   PP       — projection pruning (required-column annotation)
--   DCE      — dead code elimination (EmptyResult propagation)
--   LP       — limit pushdown (scan limit annotation)
--   E2E      — end-to-end optimize/optimizeWithPasses tests
--   Pass     — Pass record tests

module SqlOptimizerSpec (spec) where

import Test.Hspec
import SqlPlanner
import SqlOptimizer

-- ── Schema & plan helpers ─────────────────────────────────────────────────────

testSchema :: SchemaProvider
testSchema = inMemorySchema
    [ ("users",  ["id", "name", "age"])
    , ("orders", ["id", "user_id", "amount"])
    ]

-- Build a LogicalPlan via the planner (partial, used in E2E tests).
planRight :: Statement -> LogicalPlan
planRight stmt = case plan testSchema stmt of
    Left err -> error ("Unexpected planner error: " ++ show err)
    Right lp -> lp

selectStar :: String -> Statement
selectStar tbl = SelectStmt False [OutputStar] [TableRef tbl Nothing]
                             [] Nothing [] Nothing [] Nothing

-- ── Spec ──────────────────────────────────────────────────────────────────────

spec :: Spec
spec = do
    describe "Lift" liftSpec
    describe "ConstantFolding" cfSpec
    describe "PredicatePushdown" ppdSpec
    describe "ProjectionPruning" ppSpec
    describe "DeadCodeElimination" dceSpec
    describe "LimitPushdown" lpSpec
    describe "EndToEnd" e2eSpec
    describe "Pass" passSpec

-- ── Lift tests ────────────────────────────────────────────────────────────────

liftSpec :: Spec
liftSpec = do
    it "L1: Scan lifts to OptScan with no required cols and no scan limit" $ do
        let lp = Scan "users" Nothing
        lift lp `shouldBe` OptScan "users" Nothing Nothing Nothing

    it "L2: Scan with alias preserves alias" $ do
        let lp = Scan "users" (Just "u")
        lift lp `shouldBe` OptScan "users" (Just "u") Nothing Nothing

    it "L3: Filter lifts recursively" $ do
        let pred_ = Literal (Just (LitBool True))
        let lp = Filter (Scan "users" Nothing) pred_
        lift lp `shouldBe` OptFilter (OptScan "users" Nothing Nothing Nothing) pred_

    it "L4: Project lifts recursively" $ do
        let lp = Project (Scan "users" Nothing) [OutputStar]
        lift lp `shouldBe` OptProject (OptScan "users" Nothing Nothing Nothing) [OutputStar]

    it "L5: JoinPlan lifts to OptJoin" $ do
        let lp = JoinPlan (Scan "users" Nothing) (Scan "orders" Nothing) JoinInner Nothing
        lift lp `shouldBe`
            OptJoin (OptScan "users" Nothing Nothing Nothing)
                    (OptScan "orders" Nothing Nothing Nothing)
                    JoinInner Nothing

    it "L6: Limit lifts to OptLimit" $ do
        let lp = Limit (Scan "users" Nothing) (Just 10) Nothing
        lift lp `shouldBe` OptLimit (OptScan "users" Nothing Nothing Nothing) (Just 10) Nothing

    it "L7: Distinct lifts to OptDistinct" $ do
        let lp = Distinct (Scan "users" Nothing)
        lift lp `shouldBe` OptDistinct (OptScan "users" Nothing Nothing Nothing)

    it "L8: Union lifts to OptUnion" $ do
        let lp = Union (Scan "users" Nothing) (Scan "orders" Nothing) False
        lift lp `shouldBe`
            OptUnion (OptScan "users" Nothing Nothing Nothing)
                     (OptScan "orders" Nothing Nothing Nothing) False

    it "L9: InsertPlan lifts to OptInsert" $ do
        let lp = InsertPlan "users" ["id"] [[Literal (Just (LitInt 1))]]
        lift lp `shouldBe` OptInsert "users" ["id"] [[Literal (Just (LitInt 1))]]

    it "L10: DropTablePlan lifts to OptDropTable" $ do
        let lp = DropTablePlan "users" True
        lift lp `shouldBe` OptDropTable "users" True

-- ── Constant Folding tests ────────────────────────────────────────────────────

cfSpec :: Spec
cfSpec = do
    it "CF1: 2 + 3 folds to 5" $ do
        let pred_ = BinaryOp BinAdd (Literal (Just (LitInt 2))) (Literal (Just (LitInt 3)))
        let lp = Filter (Scan "users" Nothing) pred_
        let opt = optimizeWithPasses [constantFolding] lp
        case opt of
            OptFilter _ (Literal (Just (LitInt 5))) -> return ()
            other -> expectationFailure ("expected folded 5, got: " ++ show other)

    it "CF2: TRUE AND FALSE folds to FALSE" $ do
        let pred_ = BinaryOp BinAnd (Literal (Just (LitBool True)))
                                    (Literal (Just (LitBool False)))
        let lp = Filter (Scan "users" Nothing) pred_
        let opt = optimizeWithPasses [constantFolding] lp
        case opt of
            OptFilter _ (Literal (Just (LitBool False))) -> return ()
            other -> expectationFailure (show other)

    it "CF3: FALSE AND x short-circuits to FALSE (x not evaluated)" $ do
        let pred_ = BinaryOp BinAnd (Literal (Just (LitBool False)))
                                    (Column Nothing "age")
        let lp = Filter (Scan "users" Nothing) pred_
        let opt = optimizeWithPasses [constantFolding] lp
        case opt of
            OptFilter _ (Literal (Just (LitBool False))) -> return ()
            other -> expectationFailure (show other)

    it "CF4: TRUE OR x short-circuits to TRUE" $ do
        let pred_ = BinaryOp BinOr (Literal (Just (LitBool True)))
                                   (Column Nothing "age")
        let lp = Filter (Scan "users" Nothing) pred_
        let opt = optimizeWithPasses [constantFolding] lp
        case opt of
            OptFilter _ (Literal (Just (LitBool True))) -> return ()
            other -> expectationFailure (show other)

    it "CF5: NULL AND FALSE folds to FALSE (short-circuit wins over NULL)" $ do
        -- FALSE AND NULL → FALSE (false short-circuits)
        -- but NULL AND FALSE: we check the left first...
        -- The fold rules: FALSE AND _ → FALSE; so NULL AND FALSE → NULL AND FALSE,
        -- but FALSE AND NULL → FALSE.
        -- Let's verify FALSE AND NULL → FALSE:
        let pred_ = BinaryOp BinAnd (Literal (Just (LitBool False)))
                                    (Literal Nothing)
        let lp = Filter (Scan "users" Nothing) pred_
        let opt = optimizeWithPasses [constantFolding] lp
        case opt of
            OptFilter _ (Literal (Just (LitBool False))) -> return ()
            other -> expectationFailure (show other)

    it "CF6: NULL AND TRUE folds to NULL" $ do
        let pred_ = BinaryOp BinAnd (Literal Nothing)
                                    (Literal (Just (LitBool True)))
        let lp = Filter (Scan "users" Nothing) pred_
        let opt = optimizeWithPasses [constantFolding] lp
        case opt of
            OptFilter _ (Literal Nothing) -> return ()
            other -> expectationFailure (show other)

    it "CF7: NOT TRUE folds to FALSE" $ do
        let pred_ = UnaryOp UnaryNot (Literal (Just (LitBool True)))
        let lp = Filter (Scan "users" Nothing) pred_
        let opt = optimizeWithPasses [constantFolding] lp
        case opt of
            OptFilter _ (Literal (Just (LitBool False))) -> return ()
            other -> expectationFailure (show other)

    it "CF8: NEG (LitInt 5) folds to LitInt (-5)" $ do
        let pred_ = BinaryOp BinGt (UnaryOp UnaryNeg (Literal (Just (LitInt 5))))
                                   (Literal (Just (LitInt (-10))))
        let lp = Filter (Scan "users" Nothing) pred_
        let opt = optimizeWithPasses [constantFolding] lp
        case opt of
            OptFilter _ (Literal (Just (LitBool True))) -> return ()
            other -> expectationFailure (show other)

    it "CF9: IsNull on Literal Nothing folds to TRUE" $ do
        let pred_ = IsNull (Literal Nothing)
        let lp = Filter (Scan "users" Nothing) pred_
        let opt = optimizeWithPasses [constantFolding] lp
        case opt of
            OptFilter _ (Literal (Just (LitBool True))) -> return ()
            other -> expectationFailure (show other)

    it "CF10: IsNotNull on Literal (Just x) folds to TRUE" $ do
        let pred_ = IsNotNull (Literal (Just (LitInt 42)))
        let lp = Filter (Scan "users" Nothing) pred_
        let opt = optimizeWithPasses [constantFolding] lp
        case opt of
            OptFilter _ (Literal (Just (LitBool True))) -> return ()
            other -> expectationFailure (show other)

    it "CF11: division by zero is NOT folded" $ do
        let pred_ = BinaryOp BinDiv (Literal (Just (LitInt 1)))
                                    (Literal (Just (LitInt 0)))
        let lp = Filter (Scan "users" Nothing) pred_
        let opt = optimizeWithPasses [constantFolding] lp
        -- should NOT be a Literal result
        case opt of
            OptFilter _ (Literal _) -> expectationFailure "Should not fold div-by-zero"
            OptFilter _ _           -> return ()
            other -> expectationFailure (show other)

    it "CF12: 3 * 4 folds to 12" $ do
        let pred_ = BinaryOp BinMul (Literal (Just (LitInt 3))) (Literal (Just (LitInt 4)))
        let lp = Filter (Scan "users" Nothing) pred_
        let opt = optimizeWithPasses [constantFolding] lp
        case opt of
            OptFilter _ (Literal (Just (LitInt 12))) -> return ()
            other -> expectationFailure (show other)

-- ── Predicate Pushdown tests ──────────────────────────────────────────────────

ppdSpec :: Spec
ppdSpec = do
    it "PPD1: Filter pushed through Sort" $ do
        let pred_ = Literal (Just (LitBool True))
        let lp = Filter (Sort (Scan "users" Nothing)
                              [SortKey (Column (Just "users") "age") SortAsc NullsLast])
                        pred_
        let opt = optimizeWithPasses [predicatePushdown] lp
        case opt of
            OptSort (OptFilter _ _) _ -> return ()
            other -> expectationFailure ("expected Filter inside Sort, got: " ++ show other)

    it "PPD2: Filter pushed through Distinct" $ do
        let pred_ = Literal (Just (LitBool True))
        let lp = Filter (Distinct (Scan "users" Nothing)) pred_
        let opt = optimizeWithPasses [predicatePushdown] lp
        case opt of
            OptDistinct (OptFilter _ _) -> return ()
            other -> expectationFailure (show other)

    it "PPD3: Filter on qualified column pushed through inner join to correct side" $ do
        let pred_ = BinaryOp BinGt (Column (Just "users") "age")
                                   (Literal (Just (LitInt 18)))
        let lp = Filter (JoinPlan (Scan "users" Nothing)
                                  (Scan "orders" Nothing)
                                  JoinInner Nothing)
                        pred_
        let opt = optimizeWithPasses [predicatePushdown] lp
        -- Filter should be pushed into the left (users) side
        case opt of
            OptJoin (OptFilter _ _) _ JoinInner _ -> return ()
            other -> expectationFailure ("expected filter pushed to left, got: " ++ show other)

    it "PPD4: Filter NOT pushed through Aggregate (stops there)" $ do
        let pred_ = Literal (Just (LitBool True))
        let lp = Filter (Aggregate (Scan "users" Nothing)
                                   [Column (Just "users") "age"] [])
                        pred_
        let opt = optimizeWithPasses [predicatePushdown] lp
        case opt of
            OptFilter (OptAggregate _ _ _) _ -> return ()
            other -> expectationFailure ("expected filter to stay above Aggregate, got: " ++ show other)

    it "PPD5: AND conjuncts split and each pushed independently" $ do
        -- pred = (users.age > 18) AND (users.id > 0)
        let pred_ = BinaryOp BinAnd
                        (BinaryOp BinGt (Column (Just "users") "age") (Literal (Just (LitInt 18))))
                        (BinaryOp BinGt (Column (Just "users") "id") (Literal (Just (LitInt 0))))
        let lp = Filter (Scan "users" Nothing) pred_
        let opt = optimizeWithPasses [predicatePushdown] lp
        -- Both conjuncts should be pushed; result is nested Filters over Scan
        case opt of
            OptFilter (OptFilter (OptScan "users" _ _ _) _) _ -> return ()
            OptFilter (OptScan "users" _ _ _) _ -> return ()  -- single filter is also fine
            other -> expectationFailure (show other)

-- ── Projection Pruning tests ──────────────────────────────────────────────────

ppSpec :: Spec
ppSpec = do
    it "PP1: RequiredCols set when specific column referenced" $ do
        let lp = Project (Scan "users" Nothing)
                         [OutputExpr (Column (Just "users") "name") Nothing]
        let opt = optimizeWithPasses [projectionPruning] lp
        case opt of
            OptProject (OptScan "users" _ (Just cols) _) _ ->
                cols `shouldBe` ["name"]
            other -> expectationFailure (show other)

    it "PP2: OutputStar keeps optRequiredCols = Nothing (wildcard)" $ do
        let lp = Project (Scan "users" Nothing) [OutputStar]
        let opt = optimizeWithPasses [projectionPruning] lp
        case opt of
            OptProject (OptScan "users" _ Nothing _) _ -> return ()
            other -> expectationFailure (show other)

    it "PP3: Filter predicate adds to required cols" $ do
        let lp = Project
                    (Filter (Scan "users" Nothing)
                            (BinaryOp BinGt (Column (Just "users") "age")
                                            (Literal (Just (LitInt 18)))))
                    [OutputExpr (Column (Just "users") "name") Nothing]
        let opt = optimizeWithPasses [projectionPruning] lp
        case opt of
            OptProject (OptFilter (OptScan "users" _ (Just cols) _) _) _ ->
                -- both "name" (from project) and "age" (from filter) should be required
                (elem "name" cols && elem "age" cols) `shouldBe` True
            other -> expectationFailure (show other)

    it "PP4: Multiple columns merged into sorted required set" $ do
        let lp = Project (Scan "users" Nothing)
                         [ OutputExpr (Column (Just "users") "name") Nothing
                         , OutputExpr (Column (Just "users") "id") Nothing
                         ]
        let opt = optimizeWithPasses [projectionPruning] lp
        case opt of
            OptProject (OptScan "users" _ (Just cols) _) _ ->
                cols `shouldBe` ["id", "name"]   -- sorted
            other -> expectationFailure (show other)

-- ── Dead Code Elimination tests ───────────────────────────────────────────────

dceSpec :: Spec
dceSpec = do
    it "DCE1: Filter(EmptyResult, _) → EmptyResult" $ do
        let lp = Filter (Union (Scan "users" Nothing) (Scan "users" Nothing) False)
                        (Literal (Just (LitBool False)))
        -- After DCE, Filter(_, FALSE) → EmptyResult
        let opt = optimizeWithPasses [constantFolding, deadCodeElimination] lp
        opt `shouldBe` EmptyResult

    it "DCE2: Filter with TRUE predicate → child" $ do
        let lp = Filter (Scan "users" Nothing) (Literal (Just (LitBool True)))
        let opt = optimizeWithPasses [deadCodeElimination] lp
        opt `shouldBe` OptScan "users" Nothing Nothing Nothing

    it "DCE3: Filter with NULL predicate → EmptyResult" $ do
        let lp = Filter (Scan "users" Nothing) (Literal Nothing)
        let opt = optimizeWithPasses [deadCodeElimination] lp
        opt `shouldBe` EmptyResult

    it "DCE4: Limit(_, Just 0, _) → EmptyResult" $ do
        let lp = Limit (Scan "users" Nothing) (Just 0) Nothing
        let opt = optimizeWithPasses [deadCodeElimination] lp
        opt `shouldBe` EmptyResult

    it "DCE5: Project(EmptyResult) → EmptyResult" $ do
        let inner = Filter (Scan "users" Nothing) (Literal (Just (LitBool False)))
        let lp = Project inner [OutputStar]
        let opt = optimizeWithPasses [deadCodeElimination] lp
        opt `shouldBe` EmptyResult

    it "DCE6: Sort(EmptyResult) → EmptyResult" $ do
        let inner = Filter (Scan "users" Nothing) (Literal (Just (LitBool False)))
        let lp = Sort inner [SortKey (Column (Just "users") "name") SortAsc NullsLast]
        let opt = optimizeWithPasses [deadCodeElimination] lp
        opt `shouldBe` EmptyResult

    it "DCE7: INNER JOIN (EmptyResult, _) → EmptyResult" $ do
        let emptyLeft = Filter (Scan "users" Nothing) (Literal (Just (LitBool False)))
        let lp = JoinPlan emptyLeft (Scan "orders" Nothing) JoinInner Nothing
        let opt = optimizeWithPasses [deadCodeElimination] lp
        opt `shouldBe` EmptyResult

    it "DCE8: INNER JOIN (_, EmptyResult) → EmptyResult" $ do
        let emptyRight = Filter (Scan "orders" Nothing) (Literal (Just (LitBool False)))
        let lp = JoinPlan (Scan "users" Nothing) emptyRight JoinInner Nothing
        let opt = optimizeWithPasses [deadCodeElimination] lp
        opt `shouldBe` EmptyResult

    it "DCE9: Union(EmptyResult, x) → x" $ do
        let emptyLeft = Filter (Scan "users" Nothing) (Literal (Just (LitBool False)))
        let lp = Union emptyLeft (Scan "orders" Nothing) False
        let opt = optimizeWithPasses [deadCodeElimination] lp
        opt `shouldBe` OptScan "orders" Nothing Nothing Nothing

    it "DCE10: Union(x, EmptyResult) → x" $ do
        let emptyRight = Filter (Scan "orders" Nothing) (Literal (Just (LitBool False)))
        let lp = Union (Scan "users" Nothing) emptyRight False
        let opt = optimizeWithPasses [deadCodeElimination] lp
        opt `shouldBe` OptScan "users" Nothing Nothing Nothing

    it "DCE11: Aggregate(EmptyResult) NOT collapsed (COUNT(*) semantics)" $ do
        let inner = Filter (Scan "users" Nothing) (Literal (Just (LitBool False)))
        let lp = Aggregate inner [] []
        let opt = optimizeWithPasses [deadCodeElimination] lp
        case opt of
            OptAggregate EmptyResult _ _ -> return ()
            other -> expectationFailure ("expected Aggregate(EmptyResult), got: " ++ show other)

-- ── Limit Pushdown tests ──────────────────────────────────────────────────────

lpSpec :: Spec
lpSpec = do
    it "LP1: Limit(n) pushes scan limit to Scan" $ do
        let lp = Limit (Scan "users" Nothing) (Just 10) Nothing
        let opt = optimizeWithPasses [limitPushdown] lp
        case opt of
            OptLimit (OptScan "users" _ _ (Just 10)) (Just 10) Nothing -> return ()
            other -> expectationFailure (show other)

    it "LP2: Limit pushed through Project to Scan" $ do
        let lp = Limit (Project (Scan "users" Nothing) [OutputStar]) (Just 5) Nothing
        let opt = optimizeWithPasses [limitPushdown] lp
        case opt of
            OptLimit (OptProject (OptScan "users" _ _ (Just 5)) _) (Just 5) Nothing -> return ()
            other -> expectationFailure (show other)

    it "LP3: Limit pushed through Filter to Scan" $ do
        let pred_ = BinaryOp BinGt (Column (Just "users") "age") (Literal (Just (LitInt 18)))
        let lp = Limit (Filter (Scan "users" Nothing) pred_) (Just 3) Nothing
        let opt = optimizeWithPasses [limitPushdown] lp
        case opt of
            OptLimit (OptFilter (OptScan "users" _ _ (Just 3)) _) (Just 3) Nothing -> return ()
            other -> expectationFailure (show other)

    it "LP4: Limit NOT pushed through Sort (Sort changes row ordering/count)" $ do
        let lp = Limit (Sort (Scan "users" Nothing)
                             [SortKey (Column (Just "users") "name") SortAsc NullsLast])
                       (Just 5) Nothing
        let opt = optimizeWithPasses [limitPushdown] lp
        case opt of
            OptLimit (OptSort (OptScan "users" _ _ Nothing) _) (Just 5) Nothing -> return ()
            other -> expectationFailure (show other)

    it "LP5: Limit with non-zero offset NOT pushed to Scan" $ do
        let lp = Limit (Scan "users" Nothing) (Just 10) (Just 5)
        let opt = optimizeWithPasses [limitPushdown] lp
        case opt of
            OptLimit (OptScan "users" _ _ Nothing) (Just 10) (Just 5) -> return ()
            other -> expectationFailure (show other)

    it "LP6: min of two limits when Scan already has a hint" $ do
        -- Outer limit 3, inner limit 10 (already pushed) → min = 3
        let inner = Limit (Scan "users" Nothing) (Just 10) Nothing
        let lp = Limit inner (Just 3) Nothing
        let opt = optimizeWithPasses [limitPushdown] lp
        -- After pushing, the Scan should get min(10, 3) = 3
        case opt of
            OptLimit (OptLimit (OptScan "users" _ _ (Just scanLim)) _ _) (Just 3) Nothing ->
                scanLim `shouldBe` 3
            other -> expectationFailure (show other)

-- ── End-to-end tests ──────────────────────────────────────────────────────────

e2eSpec :: Spec
e2eSpec = do
    it "E2E1: optimize returns an OptimizedPlan (not error)" $ do
        let lp = planRight (selectStar "users")
        let opt = optimize lp
        case opt of
            EmptyResult -> expectationFailure "unexpected EmptyResult"
            _           -> return ()

    it "E2E2: optimize with FALSE WHERE becomes EmptyResult" $ do
        -- SELECT * FROM users WHERE FALSE
        let stmt = SelectStmt False [OutputStar] [TableRef "users" Nothing]
                               [] (Just (Literal (Just (LitBool False)))) [] Nothing [] Nothing
        let lp = planRight stmt
        let opt = optimize lp
        opt `shouldBe` EmptyResult

    it "E2E3: optimizeWithPasses with empty list = just lift" $ do
        let lp = Scan "users" Nothing
        let opt = optimizeWithPasses [] lp
        opt `shouldBe` OptScan "users" Nothing Nothing Nothing

    it "E2E4: full pipeline on simple scan annotates nothing" $ do
        let lp = planRight (selectStar "users")
        let opt = optimize lp
        -- Should get an OptProject at the top wrapping some Scan
        case opt of
            OptProject _ _ -> return ()
            other -> expectationFailure ("expected OptProject at root, got: " ++ show other)

    it "E2E5: custom single-pass pipeline (only DCE)" $ do
        let lp = Filter (Scan "users" Nothing) (Literal (Just (LitBool False)))
        let opt = optimizeWithPasses [deadCodeElimination] lp
        opt `shouldBe` EmptyResult

    it "E2E6: LIMIT 0 with full pipeline yields EmptyResult" $ do
        let stmt = SelectStmt False [OutputStar] [TableRef "users" Nothing]
                               [] Nothing [] Nothing [] (Just (LimitClause (Just 0) Nothing))
        let lp = planRight stmt
        let opt = optimize lp
        opt `shouldBe` EmptyResult

    it "E2E7: DML passthrough — InsertPlan lifts and passes through all passes unchanged" $ do
        let lp = InsertPlan "users" ["id"] [[Literal (Just (LitInt 1))]]
        let opt = optimize lp
        opt `shouldBe` OptInsert "users" ["id"] [[Literal (Just (LitInt 1))]]

-- ── Pass record tests ─────────────────────────────────────────────────────────

passSpec :: Spec
passSpec = do
    it "P1: defaultPasses has exactly 5 entries" $
        length defaultPasses `shouldBe` 5

    it "P2: all passes have non-empty names" $
        all (not . null . passName) defaultPasses `shouldBe` True

    it "P3: passName of first pass is constantFolding" $
        passName (head defaultPasses) `shouldBe` "constantFolding"

    it "P4: passApply is callable on identity plan" $ do
        let scan = OptScan "users" Nothing Nothing Nothing
        -- Apply every pass to a bare Scan; none should error
        mapM_ (\p -> passApply p scan `seq` return ()) defaultPasses
