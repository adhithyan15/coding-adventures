-- SqlPlanner.hs — logical query plan builder for SQL statements.
--
-- Transforms a Statement into a LogicalPlan tree using an 8-step bottom-up
-- SELECT pipeline:
--
--   Scan → Filter → Aggregate → Having → Project → Distinct → Sort → Limit
--
-- No I/O, no database connections — pure functional data transformation.
-- Errors are reported via Either PlanError LogicalPlan (consistent with
-- Haskell's conventional railway-oriented style).
--
-- Usage:
--   let schema = inMemorySchema [("users", ["id", "name", "age"])]
--   case plan schema stmt of
--     Left err   -> print err
--     Right plan -> print plan

module SqlPlanner
    ( -- * Enumerations
      BinaryOperator(..)
    , UnaryOperator(..)
    , AggFunction(..)
    , SortDir(..)
    , NullOrder(..)
    , JoinKind(..)
      -- * Aggregate argument
    , AggArg(..)
      -- * Scalar expressions
    , SqlExpr(..)
      -- * Output column
    , OutputColumn(..)
      -- * Structural types
    , JoinClause(..)
    , ColumnDef(..)
    , Assignment(..)
    , LimitClause(..)
    , SortKey(..)
    , AggregateItem(..)
    , TableRef(..)
      -- * Statement AST
    , Statement(..)
      -- * Logical plan nodes
    , LogicalPlan(..)
      -- * Errors
    , PlanError(..)
      -- * Schema provider
    , SchemaProvider(..)
    , inMemorySchema
      -- * Planner
    , plan
    , planAll
    ) where

import Data.Char (toLower)
import Data.List (find, nub)

-- ── Enumerations ──────────────────────────────────────────────────────────────

data BinaryOperator
    = BinEq | BinNotEq | BinLt | BinLte | BinGt | BinGte
    | BinAnd | BinOr
    | BinAdd | BinSub | BinMul | BinDiv | BinMod
    deriving (Show, Eq)

data UnaryOperator = UnaryNot | UnaryNeg
    deriving (Show, Eq)

data AggFunction = AggCount | AggSum | AggAvg | AggMin | AggMax
    deriving (Show, Eq)

data SortDir  = SortAsc | SortDesc
    deriving (Show, Eq)

data NullOrder = NullsFirst | NullsLast
    deriving (Show, Eq)

data JoinKind = JoinInner | JoinLeft | JoinRight | JoinFull | JoinCross
    deriving (Show, Eq)

-- ── Aggregate argument ────────────────────────────────────────────────────────

-- | Argument to an aggregate function: * or an expression.
data AggArg
    = AggStar
    | AggExprArg SqlExpr
    deriving (Show, Eq)

-- ── Scalar expressions ────────────────────────────────────────────────────────

-- | A scalar expression in a SQL query plan.
--   Haskell algebraic data types give us pattern-match exhaustiveness.
data SqlExpr
    = Literal (Maybe LiteralVal)
    | Column  (Maybe String) String    -- ^ (table alias, column name)
    | BinaryOp BinaryOperator SqlExpr SqlExpr
    | UnaryOp  UnaryOperator  SqlExpr
    | FuncCall String [SqlExpr]
    | IsNull    SqlExpr
    | IsNotNull SqlExpr
    | Between   SqlExpr SqlExpr SqlExpr  -- ^ value, low, high
    | InExpr    SqlExpr [SqlExpr]
    | NotInExpr SqlExpr [SqlExpr]
    | Like      SqlExpr String
    | NotLike   SqlExpr String
    | Wildcard
    | AggExpr   AggFunction AggArg Bool  -- ^ func, arg, distinct
    deriving (Show, Eq)

-- | Literal values SQL can carry.
data LiteralVal
    = LitInt  Integer
    | LitReal Double
    | LitText String
    | LitBool Bool
    | LitBlob [Int]  -- byte values 0–255
    deriving (Show, Eq)

-- ── Output column ─────────────────────────────────────────────────────────────

data OutputColumn
    = OutputStar
    | OutputExpr SqlExpr (Maybe String)  -- ^ expression, optional alias
    deriving (Show, Eq)

-- ── Structural types ──────────────────────────────────────────────────────────

data JoinClause = JoinClause
    { joinKind  :: JoinKind
    , joinTable :: String
    , joinAlias :: Maybe String
    , joinOn    :: Maybe SqlExpr
    } deriving (Show, Eq)

data ColumnDef = ColumnDef
    { colName       :: String
    , colTypeName   :: String
    , colNotNull    :: Bool
    , colPrimaryKey :: Bool
    , colUnique     :: Bool
    , colDefault    :: Maybe SqlExpr
    } deriving (Show, Eq)

data Assignment = Assignment
    { assignCol :: String
    , assignVal :: SqlExpr
    } deriving (Show, Eq)

data LimitClause = LimitClause
    { limitCount  :: Maybe Integer
    , limitOffset :: Maybe Integer
    } deriving (Show, Eq)

data SortKey = SortKey
    { sortExpr    :: SqlExpr
    , sortDir     :: SortDir
    , sortNulls   :: NullOrder
    } deriving (Show, Eq)

data AggregateItem = AggregateItem
    { aggFunc     :: AggFunction
    , aggArg      :: AggArg
    , aggAlias    :: String
    , aggDistinct :: Bool
    } deriving (Show, Eq)

data TableRef = TableRef
    { refTable :: String
    , refAlias :: Maybe String
    } deriving (Show, Eq)

-- ── Statement AST ─────────────────────────────────────────────────────────────

data Statement
    = SelectStmt
        { stmtDistinct :: Bool
        , stmtColumns  :: [OutputColumn]
        , stmtFrom     :: [TableRef]
        , stmtJoins    :: [JoinClause]
        , stmtWhere    :: Maybe SqlExpr
        , stmtGroupBy  :: [SqlExpr]
        , stmtHaving   :: Maybe SqlExpr
        , stmtOrderBy  :: [SortKey]
        , stmtLimit    :: Maybe LimitClause
        }
    | InsertStmt  String [String] [[SqlExpr]]
    | UpdateStmt  String [Assignment] (Maybe SqlExpr)
    | DeleteStmt  String (Maybe SqlExpr)
    | CreateTableStmt String Bool [ColumnDef]
    | DropTableStmt   String Bool
    deriving (Show, Eq)

-- ── Logical plan nodes ────────────────────────────────────────────────────────

data LogicalPlan
    = Scan    String (Maybe String)                          -- ^ table, alias
    | Filter  LogicalPlan SqlExpr
    | Project LogicalPlan [OutputColumn]
    | JoinPlan LogicalPlan LogicalPlan JoinKind (Maybe SqlExpr)
    | Aggregate LogicalPlan [SqlExpr] [AggregateItem]
    | Having  LogicalPlan SqlExpr
    | Sort    LogicalPlan [SortKey]
    | Limit   LogicalPlan (Maybe Integer) (Maybe Integer)   -- ^ count, offset
    | Distinct LogicalPlan
    | Union   LogicalPlan LogicalPlan Bool                   -- ^ left, right, all
    | InsertPlan  String [String] [[SqlExpr]]
    | UpdatePlan  String [Assignment] (Maybe SqlExpr)
    | DeletePlan  String (Maybe SqlExpr)
    | CreateTablePlan String Bool [ColumnDef]
    | DropTablePlan   String Bool
    deriving (Show, Eq)

-- ── Errors ────────────────────────────────────────────────────────────────────

data PlanError
    = AmbiguousColumn String [String]   -- ^ column name, list of table aliases
    | UnknownTable    String
    | UnknownColumn   (Maybe String) String  -- ^ optional qualifier, column
    | InvalidAggregate String
    | UnsupportedStatement String
    deriving (Show, Eq)

-- ── Schema provider ───────────────────────────────────────────────────────────

newtype SchemaProvider = SchemaProvider
    { schemaColumns :: String -> Either PlanError [String]
    }

-- | Build a schema provider from an association list.
inMemorySchema :: [(String, [String])] -> SchemaProvider
inMemorySchema pairs = SchemaProvider $ \tbl ->
    case lookup tbl pairs of
        Nothing   -> Left (UnknownTable tbl)
        Just cols -> Right cols

-- ── Planner internals ─────────────────────────────────────────────────────────

-- A scope entry: one FROM/JOIN source with its resolved alias and columns.
data ScopeEntry = ScopeEntry
    { seAlias :: String   -- alias or table name when no alias given
    , seTable :: String
    , seCols  :: [String]
    } deriving (Show, Eq)

buildScope :: SchemaProvider -> [TableRef] -> [JoinClause] -> Either PlanError [ScopeEntry]
buildScope schema refs joins = do
    refEntries  <- mapM (refToEntry schema) refs
    joinEntries <- mapM (joinToEntry schema) joins
    return (refEntries ++ joinEntries)

refToEntry :: SchemaProvider -> TableRef -> Either PlanError ScopeEntry
refToEntry schema (TableRef tbl alias) = do
    cols <- schemaColumns schema tbl
    return (ScopeEntry (maybe tbl id alias) tbl cols)

joinToEntry :: SchemaProvider -> JoinClause -> Either PlanError ScopeEntry
joinToEntry schema (JoinClause _ tbl alias _) = do
    cols <- schemaColumns schema tbl
    return (ScopeEntry (maybe tbl id alias) tbl cols)

resolveColumn :: [ScopeEntry] -> Maybe String -> String -> Either PlanError SqlExpr
resolveColumn scope (Just tbl) col =
    case find (\e -> seAlias e == tbl) scope of
        Nothing -> Left (UnknownTable tbl)
        Just e  ->
            if any (\c -> map toLower c == map toLower col) (seCols e)
                then Right (Column (Just (seAlias e)) col)
                else Left (UnknownColumn (Just tbl) col)
resolveColumn scope Nothing col =
    let matches = filter (\e -> any (\c -> map toLower c == map toLower col) (seCols e)) scope
    in case matches of
        []  -> Left (UnknownColumn Nothing col)
        [e] -> Right (Column (Just (seAlias e)) col)
        es  -> Left (AmbiguousColumn col (map seAlias es))

resolveExpr :: [ScopeEntry] -> SqlExpr -> Either PlanError SqlExpr
resolveExpr scope expr = case expr of
    Column tbl col -> resolveColumn scope tbl col
    Literal _      -> Right expr
    Wildcard       -> Right expr
    AggExpr _ _ _  -> Right expr
    BinaryOp op l r -> do
        l' <- resolveExpr scope l
        r' <- resolveExpr scope r
        return (BinaryOp op l' r')
    UnaryOp op e -> UnaryOp op <$> resolveExpr scope e
    FuncCall name args -> do
        args' <- mapM (resolveExpr scope) args
        return (FuncCall name args')
    IsNull e    -> IsNull    <$> resolveExpr scope e
    IsNotNull e -> IsNotNull <$> resolveExpr scope e
    Between v lo hi -> do
        v'  <- resolveExpr scope v
        lo' <- resolveExpr scope lo
        hi' <- resolveExpr scope hi
        return (Between v' lo' hi')
    InExpr v items -> do
        v'     <- resolveExpr scope v
        items' <- mapM (resolveExpr scope) items
        return (InExpr v' items')
    NotInExpr v items -> do
        v'     <- resolveExpr scope v
        items' <- mapM (resolveExpr scope) items
        return (NotInExpr v' items')
    Like v pat    -> (\v' -> Like v' pat)    <$> resolveExpr scope v
    NotLike v pat -> (\v' -> NotLike v' pat) <$> resolveExpr scope v

tryResolveExpr :: [ScopeEntry] -> SqlExpr -> Maybe SqlExpr
tryResolveExpr scope e = case resolveExpr scope e of
    Right e' -> Just e'
    Left (UnknownColumn _ _) -> Nothing
    Left _   -> Nothing

containsAgg :: SqlExpr -> Bool
containsAgg (AggExpr _ _ _)    = True
containsAgg (BinaryOp _ l r)   = containsAgg l || containsAgg r
containsAgg (UnaryOp _ e)      = containsAgg e
containsAgg (FuncCall _ args)  = any containsAgg args
containsAgg (IsNull e)         = containsAgg e
containsAgg (IsNotNull e)      = containsAgg e
containsAgg (Between v lo hi)  = containsAgg v || containsAgg lo || containsAgg hi
containsAgg (InExpr v items)   = containsAgg v || any containsAgg items
containsAgg (NotInExpr v items)= containsAgg v || any containsAgg items
containsAgg (Like v _)         = containsAgg v
containsAgg (NotLike v _)      = containsAgg v
containsAgg _                  = False

collectAggs :: [SqlExpr] -> [AggregateItem]
collectAggs exprs = go exprs 0 []
  where
    go []     _ acc = reverse acc
    go (e:es) n acc =
        let (n', newItems) = walkAgg e n []
        in go es n' (reverse newItems ++ acc)

    walkAgg :: SqlExpr -> Int -> [AggregateItem] -> (Int, [AggregateItem])
    walkAgg (AggExpr func arg distinct) n acc =
        (n + 1, AggregateItem func arg ("_agg" ++ show n) distinct : acc)
    walkAgg (BinaryOp _ l r) n acc =
        let (n', acc') = walkAgg l n acc
        in walkAgg r n' acc'
    walkAgg (UnaryOp _ e) n acc  = walkAgg e n acc
    walkAgg (FuncCall _ args) n acc =
        foldl (\(n', acc') a -> walkAgg a n' acc') (n, acc) args
    walkAgg (IsNull e) n acc     = walkAgg e n acc
    walkAgg (IsNotNull e) n acc  = walkAgg e n acc
    walkAgg (Between v lo hi) n acc =
        let (n1, acc1) = walkAgg v  n  acc
            (n2, acc2) = walkAgg lo n1 acc1
        in walkAgg hi n2 acc2
    walkAgg (InExpr v items) n acc =
        let (n', acc') = walkAgg v n acc
        in foldl (\(n'', acc'') i -> walkAgg i n'' acc'') (n', acc') items
    walkAgg (NotInExpr v items) n acc =
        let (n', acc') = walkAgg v n acc
        in foldl (\(n'', acc'') i -> walkAgg i n'' acc'') (n', acc') items
    walkAgg (Like v _) n acc    = walkAgg v n acc
    walkAgg (NotLike v _) n acc = walkAgg v n acc
    walkAgg _ n acc             = (n, acc)

buildFromTree :: SchemaProvider -> [TableRef] -> [JoinClause] -> Either PlanError LogicalPlan
buildFromTree schema refs joins
    | null refs = Left (UnsupportedStatement "SELECT without FROM")
    | otherwise = do
        _ <- schemaColumns schema (refTable (head refs))
        let root = Scan (refTable (head refs)) (refAlias (head refs))
        withExtra <- foldl addRef (Right root) (tail refs)
        foldl addJoin (Right withExtra) joins
  where
    addRef acc ref = do
        p <- acc
        _ <- schemaColumns schema (refTable ref)
        return (JoinPlan p (Scan (refTable ref) (refAlias ref)) JoinCross Nothing)

    addJoin acc j = do
        p <- acc
        _ <- schemaColumns schema (joinTable j)
        return (JoinPlan p (Scan (joinTable j) (joinAlias j)) (joinKind j) (joinOn j))

planSelect :: SchemaProvider -> Statement -> Either PlanError LogicalPlan
planSelect schema s@(SelectStmt {}) = do
    scope    <- buildScope schema (stmtFrom s) (stmtJoins s)
    fromPlan <- buildFromTree schema (stmtFrom s) (stmtJoins s)

    -- Step 1: WHERE → Filter
    filtered <- case stmtWhere s of
        Nothing -> Right fromPlan
        Just w  -> Filter fromPlan <$> resolveExpr scope w

    -- Determine whether aggregation is needed
    let colExprs    = [ e | OutputExpr e _ <- stmtColumns s ]
    let havingExprs = maybe [] pure (stmtHaving s)
    let needsAgg    = not (null (stmtGroupBy s))
                   || any containsAgg colExprs
                   || any containsAgg havingExprs

    -- Step 2: GROUP BY + Aggregate
    afterAgg <- if needsAgg
        then do
            let aggs = collectAggs (colExprs ++ havingExprs)
            groupBy' <- mapM (resolveExpr scope) (stmtGroupBy s)
            return (Aggregate filtered groupBy' aggs)
        else Right filtered

    -- Step 3: HAVING
    afterHaving <- case stmtHaving s of
        Nothing -> Right afterAgg
        Just h  ->
            let rh = maybe h id (tryResolveExpr scope h)
            in Right (Having afterAgg rh)

    -- Step 4: PROJECT
    projCols <- mapM (resolveOutputCol scope needsAgg) (stmtColumns s)
    let afterProject = Project afterHaving projCols

    -- Step 5: DISTINCT
    let afterDistinct = if stmtDistinct s then Distinct afterProject else afterProject

    -- Step 6: ORDER BY
    let resolveKey k =
            let rk = maybe (sortExpr k) id (tryResolveExpr scope (sortExpr k))
            in k { sortExpr = rk }
    let afterSort = if null (stmtOrderBy s)
            then afterDistinct
            else Sort afterDistinct (map resolveKey (stmtOrderBy s))

    -- Step 7: LIMIT / OFFSET
    let afterLimit = case stmtLimit s of
            Nothing -> afterSort
            Just lc -> Limit afterSort (limitCount lc) (limitOffset lc)

    return afterLimit
planSelect _ _ = Left (UnsupportedStatement "planSelect called on non-SELECT")

resolveOutputCol :: [ScopeEntry] -> Bool -> OutputColumn -> Either PlanError OutputColumn
resolveOutputCol _     _       OutputStar       = Right OutputStar
resolveOutputCol scope needsAgg (OutputExpr e alias) =
    if needsAgg
        then
            let re = maybe e id (tryResolveExpr scope e)
            in Right (OutputExpr re alias)
        else do
            re <- resolveExpr scope e
            return (OutputExpr re alias)

-- ── Public API ────────────────────────────────────────────────────────────────

-- | Transform a single statement into a logical plan.
plan :: SchemaProvider -> Statement -> Either PlanError LogicalPlan
plan schema stmt = case stmt of
    SelectStmt {} -> planSelect schema stmt
    InsertStmt tbl cols vals -> do
        _ <- schemaColumns schema tbl
        return (InsertPlan tbl cols vals)
    UpdateStmt tbl asgns w -> do
        _ <- schemaColumns schema tbl
        return (UpdatePlan tbl asgns w)
    DeleteStmt tbl w -> do
        _ <- schemaColumns schema tbl
        return (DeletePlan tbl w)
    CreateTableStmt tbl ifne cdefs ->
        return (CreateTablePlan tbl ifne cdefs)
    DropTableStmt tbl ife ->
        return (DropTablePlan tbl ife)

-- | Plan every statement in the list; fails on the first error.
planAll :: SchemaProvider -> [Statement] -> Either PlanError [LogicalPlan]
planAll schema = mapM (plan schema)
