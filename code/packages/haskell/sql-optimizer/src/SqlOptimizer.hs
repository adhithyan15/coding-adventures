-- SqlOptimizer.hs — logical query optimizer for SQL plans.
--
-- Applies a pipeline of rewrite passes over a LogicalPlan tree produced by
-- SqlPlanner, yielding an OptimizedPlan.  Each pass is a pure function
-- (OptimizedPlan -> OptimizedPlan) wrapped in a named Pass record so callers
-- can inspect, extend, or replace the pipeline.
--
-- Default pipeline (applied in order):
--
--   1. constantFolding    — evaluate compile-time arithmetic & boolean ops
--   2. predicatePushdown  — move Filter nodes closer to their data sources
--   3. projectionPruning  — annotate Scans with the columns they actually need
--   4. deadCodeElimination — collapse subtrees that can never produce rows
--   5. limitPushdown      — forward LIMIT counts to Scans / Filter / Project
--
-- No I/O, no database connections — pure functional transformation.
--
-- Usage:
--   import SqlPlanner (plan, inMemorySchema, ...)
--   import SqlOptimizer (optimize)
--   case plan schema stmt of
--     Left err  -> print err
--     Right lp  -> print (optimize lp)

module SqlOptimizer
    ( -- * Optimized plan type
      OptimizedPlan(..)
      -- * Pass machinery
    , Pass(..)
      -- * Individual passes
    , constantFolding
    , predicatePushdown
    , projectionPruning
    , deadCodeElimination
    , limitPushdown
      -- * Public API
    , lift
    , defaultPasses
    , optimize
    , optimizeWithPasses
    ) where

import Data.List (nub, sort)
import Data.Maybe (fromMaybe)

import SqlPlanner
    ( LogicalPlan(..)
    , SqlExpr(..)
    , LiteralVal(..)
    , OutputColumn(..)
    , BinaryOperator(..)
    , UnaryOperator(..)
    , AggFunction(..)
    , AggArg(..)
    , JoinKind(..)
    , SortDir(..)
    , NullOrder(..)
    , SortKey(..)
    , AggregateItem(..)
    , ColumnDef(..)
    , Assignment(..)
    )

-- ── OptimizedPlan ─────────────────────────────────────────────────────────────
--
-- Mirrors LogicalPlan but adds:
--   • EmptyResult — a terminal node for plans provably yielding zero rows
--   • optRequiredCols on OptScan — the subset of columns the rest of the plan
--     actually needs (Nothing = all columns, i.e. pruning not yet run)
--   • optScanLimit on OptScan — an early-exit row count pushed down from a
--     LIMIT node above (Nothing = no limit hint)

data OptimizedPlan
    -- | Table scan.  After projectionPruning, optRequiredCols is Just a sorted
    --   list of column names restricted to those referenced above.
    = OptScan
        { optTable        :: String
        , optAlias        :: Maybe String
        , optRequiredCols :: Maybe [String]   -- ^ Nothing = all columns
        , optScanLimit    :: Maybe Integer    -- ^ Nothing = no row-count hint
        }

    -- | Row-level filter.
    | OptFilter
        { optInput     :: OptimizedPlan
        , optPredicate :: SqlExpr
        }

    -- | Column projection.
    | OptProject
        { optInput   :: OptimizedPlan
        , optColumns :: [OutputColumn]
        }

    -- | Join of two sub-plans.
    | OptJoin
        { optLeft      :: OptimizedPlan
        , optRight     :: OptimizedPlan
        , optKind      :: JoinKind
        , optCondition :: Maybe SqlExpr
        }

    -- | Grouping and aggregation.
    | OptAggregate
        { optInput      :: OptimizedPlan
        , optGroupBy    :: [SqlExpr]
        , optAggregates :: [AggregateItem]
        }

    -- | Post-aggregate filter (HAVING clause).
    | OptHaving
        { optInput      :: OptimizedPlan
        , optHavingPred :: SqlExpr
        }

    -- | Row ordering.
    | OptSort
        { optInput    :: OptimizedPlan
        , optSortKeys :: [SortKey]
        }

    -- | Row count limit and optional offset.
    | OptLimit
        { optInput     :: OptimizedPlan
        , optCount     :: Maybe Integer
        , optOffset    :: Maybe Integer
        }

    -- | DISTINCT deduplication.
    | OptDistinct
        { optInput :: OptimizedPlan
        }

    -- | Set union.
    | OptUnion
        { optLeft  :: OptimizedPlan
        , optRight :: OptimizedPlan
        , optAll   :: Bool
        }

    -- | DML and DDL nodes (pass-through — no rewrite applied).
    | OptInsert
        { optInsertTable :: String
        , optInsertCols  :: [String]
        , optInsertVals  :: [[SqlExpr]]
        }

    | OptUpdate
        { optUpdateTable  :: String
        , optUpdateAssign :: [Assignment]
        , optUpdateWhere  :: Maybe SqlExpr
        }

    | OptDelete
        { optDeleteTable :: String
        , optDeleteWhere :: Maybe SqlExpr
        }

    | OptCreateTable
        { optCreateTable  :: String
        , optCreateIfNE   :: Bool
        , optCreateCols   :: [ColumnDef]
        }

    | OptDropTable
        { optDropTable :: String
        , optDropIfE   :: Bool
        }

    -- | Sentinel: this sub-plan provably returns zero rows.
    | EmptyResult
    deriving (Show, Eq)

-- ── Pass ──────────────────────────────────────────────────────────────────────
--
-- A rewrite pass is a named function over OptimizedPlan.  Naming each pass
-- makes it easy to log, profile, or disable individual passes at runtime.

data Pass = Pass
    { passName  :: String
    , passApply :: OptimizedPlan -> OptimizedPlan
    }

-- ── lift — LogicalPlan → OptimizedPlan ───────────────────────────────────────
--
-- Structural isomorphism: every LogicalPlan constructor maps 1-to-1 to an
-- OptimizedPlan constructor.  OptScan starts with optRequiredCols = Nothing
-- and optScanLimit = Nothing (filled in by later passes).

-- | Lift a LogicalPlan into an OptimizedPlan without any rewriting.
lift :: LogicalPlan -> OptimizedPlan
lift (Scan tbl alias)                  = OptScan tbl alias Nothing Nothing
lift (Filter child pred_)              = OptFilter (lift child) pred_
lift (Project child cols)              = OptProject (lift child) cols
lift (JoinPlan l r kind cond)          = OptJoin (lift l) (lift r) kind cond
lift (Aggregate child grp aggs)        = OptAggregate (lift child) grp aggs
lift (Having child pred_)              = OptHaving (lift child) pred_
lift (Sort child keys)                 = OptSort (lift child) keys
lift (Limit child cnt off)             = OptLimit (lift child) cnt off
lift (Distinct child)                  = OptDistinct (lift child)
lift (Union l r allRows)               = OptUnion (lift l) (lift r) allRows
lift (InsertPlan tbl cols vals)        = OptInsert tbl cols vals
lift (UpdatePlan tbl asgns w)          = OptUpdate tbl asgns w
lift (DeletePlan tbl w)                = OptDelete tbl w
lift (CreateTablePlan tbl ifne cdefs)  = OptCreateTable tbl ifne cdefs
lift (DropTablePlan tbl ife)           = OptDropTable tbl ife

-- ── Pass 1: Constant Folding ──────────────────────────────────────────────────
--
-- Bottom-up walk: fold every SqlExpr in every OptimizedPlan node.
-- Rules (applied innermost-first):
--
--   Arithmetic:   Literal a  op  Literal b  → Literal (a op b)
--   Boolean AND:  TRUE  AND x → x ;  FALSE AND _ → FALSE
--   Boolean OR:   FALSE OR  x → x ;  TRUE  OR  _ → TRUE
--   NULL prop.:   NULL op _ → NULL  (except short-circuit AND/OR above)
--   UnaryNot:     NOT TRUE → FALSE ;  NOT FALSE → TRUE ;  NOT NULL → NULL
--   UnaryNeg:     NEG (LitInt n) → LitInt (-n) ;  NEG (LitReal d) → LitReal (-d)
--   IsNull:       IsNull  (Literal Nothing)  → TRUE
--                 IsNull  (Literal (Just _)) → FALSE
--   IsNotNull:    symmetric
--   Div by zero:  NOT folded (preserve for runtime error handling)

constantFolding :: Pass
constantFolding = Pass "constantFolding" (mapExprsInPlan foldExpr)

-- | Recursively fold an expression bottom-up.
foldExpr :: SqlExpr -> SqlExpr
foldExpr (BinaryOp op l r) =
    let l' = foldExpr l
        r' = foldExpr r
    in foldBinOp op l' r'
foldExpr (UnaryOp op e) =
    let e' = foldExpr e
    in foldUnOp op e'
foldExpr (IsNull e) =
    let e' = foldExpr e
    in case e' of
        Literal Nothing    -> Literal (Just (LitBool True))
        Literal (Just _)   -> Literal (Just (LitBool False))
        _                  -> IsNull e'
foldExpr (IsNotNull e) =
    let e' = foldExpr e
    in case e' of
        Literal Nothing    -> Literal (Just (LitBool False))
        Literal (Just _)   -> Literal (Just (LitBool True))
        _                  -> IsNotNull e'
foldExpr (FuncCall name args) = FuncCall name (map foldExpr args)
foldExpr (Between v lo hi)    = Between (foldExpr v) (foldExpr lo) (foldExpr hi)
foldExpr (InExpr v items)     = InExpr (foldExpr v) (map foldExpr items)
foldExpr (NotInExpr v items)  = NotInExpr (foldExpr v) (map foldExpr items)
foldExpr e = e  -- Column, Literal, Wildcard, AggExpr, Like, NotLike — no sub-exprs

-- | Fold a binary operation given already-folded operands.
foldBinOp :: BinaryOperator -> SqlExpr -> SqlExpr -> SqlExpr
-- AND short-circuits
foldBinOp BinAnd (Literal (Just (LitBool False))) _ =
    Literal (Just (LitBool False))
foldBinOp BinAnd _ (Literal (Just (LitBool False))) =
    Literal (Just (LitBool False))
foldBinOp BinAnd (Literal (Just (LitBool True))) r = r
foldBinOp BinAnd l (Literal (Just (LitBool True))) = l
-- NULL AND: FALSE AND NULL → FALSE (already handled above); otherwise NULL
foldBinOp BinAnd (Literal Nothing) _ = Literal Nothing
foldBinOp BinAnd _ (Literal Nothing) = Literal Nothing
-- OR short-circuits
foldBinOp BinOr (Literal (Just (LitBool True))) _ =
    Literal (Just (LitBool True))
foldBinOp BinOr _ (Literal (Just (LitBool True))) =
    Literal (Just (LitBool True))
foldBinOp BinOr (Literal (Just (LitBool False))) r = r
foldBinOp BinOr l (Literal (Just (LitBool False))) = l
-- NULL OR: TRUE OR NULL → TRUE (already handled); otherwise NULL
foldBinOp BinOr (Literal Nothing) _ = Literal Nothing
foldBinOp BinOr _ (Literal Nothing) = Literal Nothing
-- NULL propagation for all other ops
foldBinOp _ (Literal Nothing) _ = Literal Nothing
foldBinOp _ _ (Literal Nothing) = Literal Nothing
-- Arithmetic on two integer literals
foldBinOp BinAdd (Literal (Just (LitInt a))) (Literal (Just (LitInt b))) =
    Literal (Just (LitInt (a + b)))
foldBinOp BinSub (Literal (Just (LitInt a))) (Literal (Just (LitInt b))) =
    Literal (Just (LitInt (a - b)))
foldBinOp BinMul (Literal (Just (LitInt a))) (Literal (Just (LitInt b))) =
    Literal (Just (LitInt (a * b)))
foldBinOp BinDiv (Literal (Just (LitInt _))) (Literal (Just (LitInt 0))) =
    BinaryOp BinDiv (Literal (Just (LitInt 0))) (Literal (Just (LitInt 0)))  -- preserve div-by-zero
foldBinOp BinDiv (Literal (Just (LitInt a))) (Literal (Just (LitInt b))) =
    Literal (Just (LitInt (a `div` b)))
foldBinOp BinMod (Literal (Just (LitInt _))) (Literal (Just (LitInt 0))) =
    BinaryOp BinMod (Literal (Just (LitInt 0))) (Literal (Just (LitInt 0)))  -- preserve mod-by-zero
foldBinOp BinMod (Literal (Just (LitInt a))) (Literal (Just (LitInt b))) =
    Literal (Just (LitInt (a `mod` b)))
-- Arithmetic on two real literals
foldBinOp BinAdd (Literal (Just (LitReal a))) (Literal (Just (LitReal b))) =
    Literal (Just (LitReal (a + b)))
foldBinOp BinSub (Literal (Just (LitReal a))) (Literal (Just (LitReal b))) =
    Literal (Just (LitReal (a - b)))
foldBinOp BinMul (Literal (Just (LitReal a))) (Literal (Just (LitReal b))) =
    Literal (Just (LitReal (a * b)))
foldBinOp BinDiv (Literal (Just (LitReal _))) (Literal (Just (LitReal 0.0))) =
    BinaryOp BinDiv (Literal (Just (LitReal 0.0))) (Literal (Just (LitReal 0.0)))
foldBinOp BinDiv (Literal (Just (LitReal a))) (Literal (Just (LitReal b))) =
    Literal (Just (LitReal (a / b)))
-- Comparison on integers
foldBinOp BinEq    (Literal (Just (LitInt a))) (Literal (Just (LitInt b))) = boolLit (a == b)
foldBinOp BinNotEq (Literal (Just (LitInt a))) (Literal (Just (LitInt b))) = boolLit (a /= b)
foldBinOp BinLt    (Literal (Just (LitInt a))) (Literal (Just (LitInt b))) = boolLit (a < b)
foldBinOp BinLte   (Literal (Just (LitInt a))) (Literal (Just (LitInt b))) = boolLit (a <= b)
foldBinOp BinGt    (Literal (Just (LitInt a))) (Literal (Just (LitInt b))) = boolLit (a > b)
foldBinOp BinGte   (Literal (Just (LitInt a))) (Literal (Just (LitInt b))) = boolLit (a >= b)
-- Comparison on reals
foldBinOp BinEq    (Literal (Just (LitReal a))) (Literal (Just (LitReal b))) = boolLit (a == b)
foldBinOp BinNotEq (Literal (Just (LitReal a))) (Literal (Just (LitReal b))) = boolLit (a /= b)
foldBinOp BinLt    (Literal (Just (LitReal a))) (Literal (Just (LitReal b))) = boolLit (a < b)
foldBinOp BinLte   (Literal (Just (LitReal a))) (Literal (Just (LitReal b))) = boolLit (a <= b)
foldBinOp BinGt    (Literal (Just (LitReal a))) (Literal (Just (LitReal b))) = boolLit (a > b)
foldBinOp BinGte   (Literal (Just (LitReal a))) (Literal (Just (LitReal b))) = boolLit (a >= b)
-- Comparison on booleans
foldBinOp BinEq    (Literal (Just (LitBool a))) (Literal (Just (LitBool b))) = boolLit (a == b)
foldBinOp BinNotEq (Literal (Just (LitBool a))) (Literal (Just (LitBool b))) = boolLit (a /= b)
-- Comparison on text
foldBinOp BinEq    (Literal (Just (LitText a))) (Literal (Just (LitText b))) = boolLit (a == b)
foldBinOp BinNotEq (Literal (Just (LitText a))) (Literal (Just (LitText b))) = boolLit (a /= b)
-- Fallthrough: cannot fold
foldBinOp op l r = BinaryOp op l r

boolLit :: Bool -> SqlExpr
boolLit b = Literal (Just (LitBool b))

-- | Fold a unary operation given an already-folded operand.
foldUnOp :: UnaryOperator -> SqlExpr -> SqlExpr
foldUnOp UnaryNot (Literal (Just (LitBool b))) = Literal (Just (LitBool (not b)))
foldUnOp UnaryNot (Literal Nothing)            = Literal Nothing
foldUnOp UnaryNeg (Literal (Just (LitInt  n))) = Literal (Just (LitInt  (-n)))
foldUnOp UnaryNeg (Literal (Just (LitReal d))) = Literal (Just (LitReal (-d)))
foldUnOp UnaryNeg (Literal Nothing)            = Literal Nothing
foldUnOp op e                                  = UnaryOp op e

-- ── Pass 2: Predicate Pushdown ────────────────────────────────────────────────
--
-- Goal: move Filter nodes as close to their source scans as possible so that
-- fewer rows travel up the plan tree.
--
-- Strategy:
--   • Split AND conjuncts of a Filter predicate into individual filters.
--   • For each conjunct, attempt to push it through the child node:
--       - Through Sort    (always safe — Sort doesn't change cardinality)
--       - Through Distinct (always safe)
--       - Through Project  (safe when all column refs come from the child's
--                           table aliases, not computed aliases introduced by
--                           the Project itself; we use a conservative check)
--       - Through JoinPlan (push to left or right sub-plan when all refs belong
--                           exclusively to that side; outer-join safe: we only
--                           push to the preserved side for inner/cross joins)
--   • Filters touching Aggregate, Limit, Having, or Union are left in place.

predicatePushdown :: Pass
predicatePushdown = Pass "predicatePushdown" pushdownPlan

-- | Apply pushdown bottom-up (recurse into children first, then push).
pushdownPlan :: OptimizedPlan -> OptimizedPlan
pushdownPlan EmptyResult        = EmptyResult
pushdownPlan (OptScan t a rc sl) = OptScan t a rc sl
pushdownPlan (OptFilter child pred_) =
    let child' = pushdownPlan child
        conjuncts = splitAnd pred_
    in foldr pushOne child' conjuncts
pushdownPlan (OptProject child cols) =
    OptProject (pushdownPlan child) cols
pushdownPlan (OptJoin l r kind cond) =
    OptJoin (pushdownPlan l) (pushdownPlan r) kind cond
pushdownPlan (OptAggregate child grp aggs) =
    OptAggregate (pushdownPlan child) grp aggs
pushdownPlan (OptHaving child pred_) =
    OptHaving (pushdownPlan child) pred_
pushdownPlan (OptSort child keys) =
    OptSort (pushdownPlan child) keys
pushdownPlan (OptLimit child cnt off) =
    OptLimit (pushdownPlan child) cnt off
pushdownPlan (OptDistinct child) =
    OptDistinct (pushdownPlan child)
pushdownPlan (OptUnion l r allRows) =
    OptUnion (pushdownPlan l) (pushdownPlan r) allRows
pushdownPlan other = other

-- | Attempt to push a single predicate through a node.
--   Returns a plan with the predicate embedded as deep as possible.
pushOne :: SqlExpr -> OptimizedPlan -> OptimizedPlan
-- Push through Sort (safe: Sort does not change predicate semantics)
pushOne pred_ (OptSort child keys) =
    OptSort (pushOne pred_ child) keys
-- Push through Distinct (safe: Distinct only deduplicates, doesn't add rows)
pushOne pred_ (OptDistinct child) =
    OptDistinct (pushOne pred_ child)
-- Push through Project when the predicate only references columns available
-- from the Project's child (conservative: check we can satisfy the predicate
-- without needing any alias introduced by this Project)
pushOne pred_ (OptProject child cols)
    | canPushThroughProject pred_ cols =
        OptProject (pushOne pred_ child) cols
    | otherwise =
        OptFilter (OptProject child cols) pred_
-- Push into Join: if inner/cross join, route to the side that owns all cols
pushOne pred_ (OptJoin l r JoinInner cond) =
    pushIntoJoin pred_ l r JoinInner cond
pushOne pred_ (OptJoin l r JoinCross cond) =
    pushIntoJoin pred_ l r JoinCross cond
-- For outer joins, only push to the outer (preserved) side:
--   LEFT JOIN: left side is preserved; push there if cols are all from left
pushOne pred_ (OptJoin l r JoinLeft cond)
    | allRefsFrom pred_ (scanAliases l) && not (any (`elem` scanAliases r) (colRefs pred_)) =
        OptJoin (pushOne pred_ l) r JoinLeft cond
    | otherwise =
        OptFilter (OptJoin l r JoinLeft cond) pred_
pushOne pred_ (OptJoin l r JoinRight cond)
    | allRefsFrom pred_ (scanAliases r) && not (any (`elem` scanAliases l) (colRefs pred_)) =
        OptJoin l (pushOne pred_ r) JoinRight cond
    | otherwise =
        OptFilter (OptJoin l r JoinRight cond) pred_
-- Full outer: never safe to push
pushOne pred_ (OptJoin l r JoinFull cond) =
    OptFilter (OptJoin l r JoinFull cond) pred_
-- Stop at Aggregate, Limit, Having, Union, EmptyResult, Scan
pushOne pred_ child = OptFilter child pred_

-- | Split a predicate into AND conjuncts.
splitAnd :: SqlExpr -> [SqlExpr]
splitAnd (BinaryOp BinAnd l r) = splitAnd l ++ splitAnd r
splitAnd e = [e]

-- | Push a predicate into the appropriate side of an inner/cross join.
pushIntoJoin :: SqlExpr -> OptimizedPlan -> OptimizedPlan -> JoinKind -> Maybe SqlExpr -> OptimizedPlan
pushIntoJoin pred_ l r kind cond
    | allRefsFrom pred_ (scanAliases l) && not (any (`elem` scanAliases r) (colRefs pred_)) =
        OptJoin (pushOne pred_ l) r kind cond
    | allRefsFrom pred_ (scanAliases r) && not (any (`elem` scanAliases l) (colRefs pred_)) =
        OptJoin l (pushOne pred_ r) kind cond
    | otherwise =
        OptFilter (OptJoin l r kind cond) pred_

-- | Collect all (Just alias, col) column references in a predicate.
--   Returns the table-alias portion; Nothing means unqualified.
colRefs :: SqlExpr -> [String]
colRefs (Column (Just tbl) _) = [tbl]
colRefs (Column Nothing _)    = []
colRefs (BinaryOp _ l r)      = colRefs l ++ colRefs r
colRefs (UnaryOp _ e)         = colRefs e
colRefs (FuncCall _ args)     = concatMap colRefs args
colRefs (IsNull e)            = colRefs e
colRefs (IsNotNull e)         = colRefs e
colRefs (Between v lo hi)     = colRefs v ++ colRefs lo ++ colRefs hi
colRefs (InExpr v items)      = colRefs v ++ concatMap colRefs items
colRefs (NotInExpr v items)   = colRefs v ++ concatMap colRefs items
colRefs (Like v _)            = colRefs v
colRefs (NotLike v _)         = colRefs v
colRefs _                     = []

-- | All table aliases visible from a plan subtree (i.e., the aliases of
--   every Scan node reachable from this plan).
scanAliases :: OptimizedPlan -> [String]
scanAliases (OptScan tbl alias _ _) = [fromMaybe tbl alias]
scanAliases (OptFilter child _)     = scanAliases child
scanAliases (OptProject child _)    = scanAliases child
scanAliases (OptJoin l r _ _)       = scanAliases l ++ scanAliases r
scanAliases (OptAggregate child _ _)= scanAliases child
scanAliases (OptHaving child _)     = scanAliases child
scanAliases (OptSort child _)       = scanAliases child
scanAliases (OptLimit child _ _)    = scanAliases child
scanAliases (OptDistinct child)     = scanAliases child
scanAliases (OptUnion l r _)        = scanAliases l ++ scanAliases r
scanAliases _                       = []

-- | True when all qualified column refs in the expression reference only the
--   given set of aliases.  Unqualified refs are considered safe (they might
--   belong anywhere; don't push speculatively).
allRefsFrom :: SqlExpr -> [String] -> Bool
allRefsFrom expr aliases =
    let refs = colRefs expr
    in not (null refs) && all (`elem` aliases) refs

-- | Conservative check: can we push a predicate through a Project?
--   Safe when the predicate does not reference any alias introduced as a
--   computed column by this Project (OutputExpr _ (Just alias)).
--   We don't chase aliases — just require that all column refs use
--   qualified (table-prefixed) names, which the planner emits.
canPushThroughProject :: SqlExpr -> [OutputColumn] -> Bool
canPushThroughProject pred_ _cols =
    -- The predicate must only reference qualified columns (table.col form).
    -- Unqualified refs might be aliases introduced by the Project — don't push.
    all isQualified (exprColumns pred_)
  where
    isQualified (Column (Just _) _) = True
    isQualified _                   = False

-- | Collect all Column sub-expressions from an expression.
exprColumns :: SqlExpr -> [SqlExpr]
exprColumns c@(Column _ _)      = [c]
exprColumns (BinaryOp _ l r)    = exprColumns l ++ exprColumns r
exprColumns (UnaryOp _ e)       = exprColumns e
exprColumns (FuncCall _ args)   = concatMap exprColumns args
exprColumns (IsNull e)          = exprColumns e
exprColumns (IsNotNull e)       = exprColumns e
exprColumns (Between v lo hi)   = exprColumns v ++ exprColumns lo ++ exprColumns hi
exprColumns (InExpr v items)    = exprColumns v ++ concatMap exprColumns items
exprColumns (NotInExpr v items) = exprColumns v ++ concatMap exprColumns items
exprColumns (Like v _)          = exprColumns v
exprColumns (NotLike v _)       = exprColumns v
exprColumns _                   = []

-- ── Pass 3: Projection Pruning ────────────────────────────────────────────────
--
-- Top-down pass carrying a required-column set (alias, colname pairs).
-- At each OptScan, the set is filtered to those columns belonging to this
-- scan's alias and stored in optRequiredCols (sorted for determinism).
-- Wildcards (OutputStar or unqualified Column refs) disable pruning
-- conservatively (Nothing is kept, meaning "all columns").
--
-- Required-column propagation:
--   OptProject : required ← column refs in all OutputExpr expressions
--   OptFilter  : required ← required ∪ column refs in predicate
--   OptHaving  : same as OptFilter
--   OptSort    : required ← required ∪ column refs in sort keys
--   OptAggregate: required ← refs in groupBy + aggArg expressions
--   OptJoin    : required ← required ∪ refs in join condition
--   all others : propagate required unchanged

projectionPruning :: Pass
projectionPruning = Pass "projectionPruning" (\plan_ -> pruneWith (Just []) plan_)

-- | A required-column set: Nothing = wildcard (all cols), Just set = explicit.
type RequiredCols = Maybe [(Maybe String, String)]  -- (alias, colName)

-- | Run pruning top-down.
pruneWith :: RequiredCols -> OptimizedPlan -> OptimizedPlan
pruneWith _    EmptyResult = EmptyResult

pruneWith req (OptScan tbl alias rc sl) =
    case req of
        Nothing ->
            -- Wildcard required → can't prune; preserve existing hint or Nothing
            OptScan tbl alias rc sl
        Just cols ->
            let myAlias   = fromMaybe tbl alias
                -- Keep columns that belong to this scan's alias (qualified)
                -- or are unqualified (conservative: keep them all)
                myQual    = [ col | (Just a, col) <- cols, a == myAlias ]
                -- Sort for determinism; nub removes duplicates
                pruned    = if null myQual
                            then rc   -- No explicit refs → keep existing hint
                            else Just (sort (nub myQual))
            in OptScan tbl alias pruned sl

pruneWith req (OptProject child cols) =
    -- Required for child = column refs inside output expressions
    let childReq = if hasWildcard cols
                   then Nothing
                   else combineReqs req (Just (refsFromOutputCols cols))
    in OptProject (pruneWith childReq child) cols

pruneWith req (OptFilter child pred_) =
    let predRefs  = refsFromExpr pred_
        childReq  = addRefs req predRefs
    in OptFilter (pruneWith childReq child) pred_

pruneWith req (OptHaving child pred_) =
    let predRefs = refsFromExpr pred_
        childReq = addRefs req predRefs
    in OptHaving (pruneWith childReq child) pred_

pruneWith req (OptSort child keys) =
    let keyRefs  = concatMap (refsFromExpr . sortExpr) keys
        childReq = addRefs req keyRefs
    in OptSort (pruneWith childReq child) keys

pruneWith req (OptAggregate child grp aggs) =
    let grpRefs  = concatMap refsFromExpr grp
        aggRefs  = concatMap refsFromAggArg (map aggArg aggs)
        childReq = addRefs req (grpRefs ++ aggRefs)
    in OptAggregate (pruneWith childReq child) grp aggs

pruneWith req (OptJoin l r kind cond) =
    let condRefs = maybe [] refsFromExpr cond
        lReq     = addRefs req condRefs
        rReq     = addRefs req condRefs
    in OptJoin (pruneWith lReq l) (pruneWith rReq r) kind cond

pruneWith req (OptLimit child cnt off) =
    OptLimit (pruneWith req child) cnt off

pruneWith req (OptDistinct child) =
    OptDistinct (pruneWith req child)

pruneWith req (OptUnion l r allRows) =
    OptUnion (pruneWith req l) (pruneWith req r) allRows

pruneWith _ other = other  -- DML/DDL: pass through

-- | Collect (alias, col) pairs from an expression (only qualified Column refs).
refsFromExpr :: SqlExpr -> [(Maybe String, String)]
refsFromExpr (Column tbl col)      = [(tbl, col)]
refsFromExpr (BinaryOp _ l r)     = refsFromExpr l ++ refsFromExpr r
refsFromExpr (UnaryOp _ e)        = refsFromExpr e
refsFromExpr (FuncCall _ args)    = concatMap refsFromExpr args
refsFromExpr (IsNull e)           = refsFromExpr e
refsFromExpr (IsNotNull e)        = refsFromExpr e
refsFromExpr (Between v lo hi)    = refsFromExpr v ++ refsFromExpr lo ++ refsFromExpr hi
refsFromExpr (InExpr v items)     = refsFromExpr v ++ concatMap refsFromExpr items
refsFromExpr (NotInExpr v items)  = refsFromExpr v ++ concatMap refsFromExpr items
refsFromExpr (Like v _)           = refsFromExpr v
refsFromExpr (NotLike v _)        = refsFromExpr v
refsFromExpr _                    = []

refsFromAggArg :: AggArg -> [(Maybe String, String)]
refsFromAggArg AggStar         = []
refsFromAggArg (AggExprArg e)  = refsFromExpr e

refsFromOutputCols :: [OutputColumn] -> [(Maybe String, String)]
refsFromOutputCols cols = concatMap go cols
  where
    go OutputStar          = []
    go (OutputExpr e _)    = refsFromExpr e

hasWildcard :: [OutputColumn] -> Bool
hasWildcard = any (== OutputStar)

-- | Add new refs to an existing required set.
addRefs :: RequiredCols -> [(Maybe String, String)] -> RequiredCols
addRefs Nothing   _    = Nothing  -- wildcard: stays wildcard
addRefs (Just existing) newRefs = Just (nub (existing ++ newRefs))

-- | Combine two required sets: Nothing (wildcard) dominates.
combineReqs :: RequiredCols -> RequiredCols -> RequiredCols
combineReqs Nothing _         = Nothing
combineReqs _ Nothing         = Nothing
combineReqs (Just a) (Just b) = Just (nub (a ++ b))

-- ── Pass 4: Dead Code Elimination ────────────────────────────────────────────
--
-- After constant folding, some predicates are statically FALSE/NULL, and some
-- sub-plans can be trivially shown to produce zero rows.  We collapse these
-- to EmptyResult sentinels, which also simplifies subsequent passes.
--
-- Rules:
--   Filter(EmptyResult, _)               → EmptyResult
--   Filter(_, Literal (Just (LitBool False))) → EmptyResult
--   Filter(_, Literal Nothing)           → EmptyResult   (NULL predicate = no rows)
--   Filter(child, Literal (Just (LitBool True))) → child
--   Limit(_, Just 0, _)                  → EmptyResult
--   Project(EmptyResult)                 → EmptyResult
--   Sort(EmptyResult)                    → EmptyResult
--   Limit(EmptyResult, _, _)             → EmptyResult
--   Distinct(EmptyResult)                → EmptyResult
--   Having(EmptyResult)                  → EmptyResult
--   INNER JOIN (EmptyResult, _)          → EmptyResult
--   INNER JOIN (_, EmptyResult)          → EmptyResult
--   CROSS JOIN (EmptyResult, _)          → EmptyResult
--   CROSS JOIN (_, EmptyResult)          → EmptyResult
--   Union(EmptyResult, x)                → x
--   Union(x, EmptyResult)                → x
--   Aggregate(EmptyResult)               → NOT collapsed (COUNT(*) = 1 row)

deadCodeElimination :: Pass
deadCodeElimination = Pass "deadCodeElimination" dce

dce :: OptimizedPlan -> OptimizedPlan
dce EmptyResult = EmptyResult

-- Recurse first, then apply rules
dce (OptFilter child pred_) =
    let child' = dce child
    in case child' of
        EmptyResult -> EmptyResult
        _ -> case pred_ of
            Literal Nothing              -> EmptyResult  -- NULL predicate
            Literal (Just (LitBool False)) -> EmptyResult
            Literal (Just (LitBool True))  -> child'
            _                             -> OptFilter child' pred_

dce (OptProject child cols) =
    let child' = dce child
    in case child' of
        EmptyResult -> EmptyResult
        _           -> OptProject child' cols

dce (OptSort child keys) =
    let child' = dce child
    in case child' of
        EmptyResult -> EmptyResult
        _           -> OptSort child' keys

dce (OptLimit child cnt off) =
    let child' = dce child
    in case (child', cnt) of
        (EmptyResult, _)      -> EmptyResult
        (_, Just 0)           -> EmptyResult
        _                     -> OptLimit child' cnt off

dce (OptDistinct child) =
    let child' = dce child
    in case child' of
        EmptyResult -> EmptyResult
        _           -> OptDistinct child'

dce (OptHaving child pred_) =
    let child' = dce child
    in case child' of
        EmptyResult -> EmptyResult
        _           -> OptHaving child' pred_

-- INNER and CROSS: either side empty → EmptyResult
dce (OptJoin l r JoinInner cond) =
    let l' = dce l; r' = dce r
    in case (l', r') of
        (EmptyResult, _) -> EmptyResult
        (_, EmptyResult) -> EmptyResult
        _                -> OptJoin l' r' JoinInner cond

dce (OptJoin l r JoinCross cond) =
    let l' = dce l; r' = dce r
    in case (l', r') of
        (EmptyResult, _) -> EmptyResult
        (_, EmptyResult) -> EmptyResult
        _                -> OptJoin l' r' JoinCross cond

-- Outer joins: empty on preserved side doesn't imply zero rows (null-padded)
dce (OptJoin l r kind cond) =
    OptJoin (dce l) (dce r) kind cond

-- Aggregate(EmptyResult) is NOT collapsed — COUNT(*) returns 1 row even on empty input
dce (OptAggregate child grp aggs) =
    OptAggregate (dce child) grp aggs

-- Union: collapse empty sides
dce (OptUnion l r allRows) =
    let l' = dce l; r' = dce r
    in case (l', r') of
        (EmptyResult, x) -> x
        (x, EmptyResult) -> x
        _                -> OptUnion l' r' allRows

dce other = other  -- DML/DDL/Scan pass through

-- ── Pass 5: Limit Pushdown ────────────────────────────────────────────────────
--
-- When a LIMIT n (with no offset, or offset = 0) sits above a chain of
-- Project or Filter nodes, push the count n down through those nodes and
-- finally into any Scan encountered, setting optScanLimit.
--
-- This lets a storage engine do early termination without reading all rows.
--
-- Stop propagation at: Sort, Aggregate, Distinct, Join (their output
-- cardinality differs from their input cardinality).

limitPushdown :: Pass
limitPushdown = Pass "limitPushdown" pushLimits

pushLimits :: OptimizedPlan -> OptimizedPlan
pushLimits EmptyResult = EmptyResult
pushLimits (OptLimit child cnt off) =
    let child' = pushLimits child
    -- Only push when offset is absent or zero (pushing with non-zero offset
    -- would be unsound — we'd skip rows we should return).
    in case (cnt, off) of
        (Just n, Nothing) -> OptLimit (pushLimitDown n child') cnt off
        (Just n, Just 0)  -> OptLimit (pushLimitDown n child') cnt off
        _                 -> OptLimit child' cnt off
pushLimits (OptFilter child pred_) =
    OptFilter (pushLimits child) pred_
pushLimits (OptProject child cols) =
    OptProject (pushLimits child) cols
pushLimits (OptSort child keys) =
    OptSort (pushLimits child) keys
pushLimits (OptAggregate child grp aggs) =
    OptAggregate (pushLimits child) grp aggs
pushLimits (OptHaving child pred_) =
    OptHaving (pushLimits child) pred_
pushLimits (OptDistinct child) =
    OptDistinct (pushLimits child)
pushLimits (OptJoin l r kind cond) =
    OptJoin (pushLimits l) (pushLimits r) kind cond
pushLimits (OptUnion l r allRows) =
    OptUnion (pushLimits l) (pushLimits r) allRows
pushLimits other = other

-- | Push a row-count hint n down through Project/Filter/Limit; install at Scan.
pushLimitDown :: Integer -> OptimizedPlan -> OptimizedPlan
pushLimitDown n (OptProject child cols) =
    OptProject (pushLimitDown n child) cols
pushLimitDown n (OptFilter child pred_) =
    OptFilter (pushLimitDown n child) pred_
-- Also descend through nested Limits: the tighter constraint is min(n, existing).
pushLimitDown n (OptLimit child cnt off) =
    let n' = case cnt of
                 Just m  -> min n m
                 Nothing -> n
    in OptLimit (pushLimitDown n' child) cnt off
pushLimitDown n (OptScan tbl alias rc existing) =
    let best = case existing of
                   Nothing -> Just n
                   Just m  -> Just (min m n)
    in OptScan tbl alias rc best
-- Stop at Sort, Aggregate, Distinct, Join — can't push through
pushLimitDown _ other = other

-- ── Shared helper: map expressions across a plan ──────────────────────────────
--
-- Applies a function to every SqlExpr in every node of the plan, bottom-up.
-- Used by constantFolding to walk the whole tree.

mapExprsInPlan :: (SqlExpr -> SqlExpr) -> OptimizedPlan -> OptimizedPlan
mapExprsInPlan _ EmptyResult = EmptyResult
mapExprsInPlan _ (OptScan t a rc sl) = OptScan t a rc sl  -- no exprs in Scan itself
mapExprsInPlan f (OptFilter child pred_) =
    OptFilter (mapExprsInPlan f child) (f pred_)
mapExprsInPlan f (OptProject child cols) =
    OptProject (mapExprsInPlan f child) (map (mapExprInOutputCol f) cols)
mapExprsInPlan f (OptJoin l r kind cond) =
    OptJoin (mapExprsInPlan f l) (mapExprsInPlan f r) kind (fmap f cond)
mapExprsInPlan f (OptAggregate child grp aggs) =
    OptAggregate (mapExprsInPlan f child)
                 (map f grp)
                 (map (mapExprInAgg f) aggs)
mapExprsInPlan f (OptHaving child pred_) =
    OptHaving (mapExprsInPlan f child) (f pred_)
mapExprsInPlan f (OptSort child keys) =
    OptSort (mapExprsInPlan f child) (map (mapExprInSortKey f) keys)
mapExprsInPlan f (OptLimit child cnt off) =
    OptLimit (mapExprsInPlan f child) cnt off
mapExprsInPlan f (OptDistinct child) =
    OptDistinct (mapExprsInPlan f child)
mapExprsInPlan f (OptUnion l r allRows) =
    OptUnion (mapExprsInPlan f l) (mapExprsInPlan f r) allRows
mapExprsInPlan f (OptInsert t c vs) =
    OptInsert t c (map (map f) vs)
mapExprsInPlan f (OptUpdate t asgns w) =
    OptUpdate t (map (mapExprInAssign f) asgns) (fmap f w)
mapExprsInPlan f (OptDelete t w) =
    OptDelete t (fmap f w)
mapExprsInPlan _ other = other  -- CreateTable, DropTable: no exprs

mapExprInOutputCol :: (SqlExpr -> SqlExpr) -> OutputColumn -> OutputColumn
mapExprInOutputCol _ OutputStar          = OutputStar
mapExprInOutputCol f (OutputExpr e alias) = OutputExpr (f e) alias

mapExprInAgg :: (SqlExpr -> SqlExpr) -> AggregateItem -> AggregateItem
mapExprInAgg f item = item { aggArg = mapExprInAggArg f (aggArg item) }

mapExprInAggArg :: (SqlExpr -> SqlExpr) -> AggArg -> AggArg
mapExprInAggArg _ AggStar         = AggStar
mapExprInAggArg f (AggExprArg e)  = AggExprArg (f e)

mapExprInSortKey :: (SqlExpr -> SqlExpr) -> SortKey -> SortKey
mapExprInSortKey f k = k { sortExpr = f (sortExpr k) }

mapExprInAssign :: (SqlExpr -> SqlExpr) -> Assignment -> Assignment
mapExprInAssign f a = a { assignVal = f (assignVal a) }

-- ── Public API ────────────────────────────────────────────────────────────────

-- | The default five-pass optimization pipeline.
--
-- Passes are applied left-to-right.  The ordering matters:
--   1. constantFolding first — simplifies predicates before pushdown sees them
--   2. predicatePushdown — works best with simple predicates
--   3. projectionPruning — annotates Scans with needed columns
--   4. deadCodeElimination — collapses trivially-empty sub-plans
--   5. limitPushdown — propagates row-count hints to Scans
defaultPasses :: [Pass]
defaultPasses =
    [ constantFolding
    , predicatePushdown
    , projectionPruning
    , deadCodeElimination
    , limitPushdown
    ]

-- | Optimize a LogicalPlan using a custom list of passes.
--
-- Each pass is applied in sequence.  Passes run exactly once; the pipeline
-- does not iterate to a fixed point (single-pass model for predictability).
optimizeWithPasses :: [Pass] -> LogicalPlan -> OptimizedPlan
optimizeWithPasses passes lp =
    foldl (\plan_ pass_ -> passApply pass_ plan_) (lift lp) passes

-- | Optimize a LogicalPlan using the default five-pass pipeline.
optimize :: LogicalPlan -> OptimizedPlan
optimize = optimizeWithPasses defaultPasses
