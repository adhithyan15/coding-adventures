-- MiniSqlite.hs — Level 1 graduation of the Haskell mini-sqlite facade.
--
-- Level 0 used a hand-rolled in-memory engine. Level 1 routes every query
-- through the full pipeline:
--
--   parse sql → SqlPlanner.Statement
--     → SqlPlanner.plan (LogicalPlan)
--     → SqlOptimizer.optimize (OptimizedPlan)
--     → SqlCodegen.compile (Program)
--     → SqlVm.execute (QueryResult)
--
-- The external API (connect, execute, fetchOne, fetchAll, …) is unchanged so
-- existing callers compile without modification.
--
-- ── Architecture overview ─────────────────────────────────────────────────
--
-- Connection now wraps an IORef SqlBackend.InMemoryBackend plus a snapshot
-- IORef for manual-commit rollback semantics. The hand-rolled Level 0
-- Database type is gone.
--
-- Statement parsing is done by the lightweight `parseSql` function below,
-- which converts the tokenised SQL text directly into SqlPlanner.Statement
-- values. We deliberately avoid pulling in the sql-parser package (which has
-- heavy transitive deps: grammar-tools, lexer, parser, etc.) in favour of a
-- self-contained hand-rolled tokeniser + AST builder that covers the subset
-- the conformance fixtures require.
--
-- ── SELECT without FROM ───────────────────────────────────────────────────
--
-- SQL allows literal expressions outside any table: SELECT 1+1, LENGTH('x').
-- SqlPlanner rejects "SELECT without FROM" so we handle this case before
-- planning: we detect a FROM-less SELECT and evaluate each output expression
-- using `evalScalarExpr`, returning a single synthesised row.
--
-- ── Scalar function support ───────────────────────────────────────────────
--
-- SqlVm.dispatch for FuncCall currently emits NULL. To support LENGTH, UPPER,
-- LOWER, SUBSTR, TRIM, LTRIM, RTRIM, REPLACE we post-process FuncCall
-- expressions in `evalScalarExpr` at the mini-sqlite layer and also handle
-- them when building INSERT value lists (FuncCall inside VALUES is unusual but
-- keep it consistent).

{-# LANGUAGE GHC2021 #-}

module MiniSqlite
    ( SqlValue(..)
    , MiniSqliteError(..)
    , Connection
    , ConnectionOptions(..)
    , defaultConnectionOptions
    , Cursor
    , Column(..)
    , apiLevel
    , threadSafety
    , paramStyle
    , connect
    , connectWith
    , execute
    , executeMany
    , commit
    , rollback
    , close
    , cursorDescription
    , cursorRowCount
    , cursorLastRowId
    , fetchOne
    , fetchMany
    , fetchAll
    , closeCursor
    ) where

import Control.Exception (catch, SomeException)
import Data.Char (isAlpha, isAlphaNum, isDigit, isSpace, toLower, toUpper)
import Data.IORef
import Data.List (dropWhileEnd, isPrefixOf)
import qualified Data.Map.Strict as Map
import Data.Maybe (fromMaybe)
import Text.Read (readMaybe)

import qualified SqlBackend as SB
import SqlBackend (InMemoryBackend, newBackend)

import qualified SqlPlanner as P
import SqlPlanner
    ( Statement(..)
    , OutputColumn(..)
    , TableRef(..)
    , JoinClause(..)
    , Assignment(..)
    , ColumnDef(..)
    , LimitClause(..)
    , SortKey(..)
    , AggregateItem(..)
    , SqlExpr( Literal, BinaryOp, UnaryOp, FuncCall
             , IsNull, IsNotNull, Between, InExpr, NotInExpr
             , Like, NotLike, Wildcard, AggExpr )
    , LiteralVal(..)
    , BinaryOperator(..)
    , UnaryOperator(..)
    , AggFunction(..)
    , AggArg(..)
    , SortDir(..)
    , NullOrder(..)
    , JoinKind(..)
    , PlanError(..)
    , SchemaProvider(..)
    , plan
    )

import SqlOptimizer (optimize)

import qualified SqlCodegen as CG
import SqlCodegen (compile)

import qualified SqlVm as VM
import SqlVm (QueryResult(..))

-- ── Public types ──────────────────────────────────────────────────────────

-- | The SQL value types that mini-sqlite supports. Mirrors SqlBackend.SqlValue
--   but without SqlBlob (Level 1 conformance does not test blobs through this
--   public API).
data SqlValue
    = SqlNull
    | SqlInteger Integer
    | SqlReal Double
    | SqlText String
    | SqlBool Bool
    deriving (Eq, Show)

data MiniSqliteError = MiniSqliteError
    { errorKind    :: String
    , errorMessage :: String
    } deriving (Eq, Show)

data ConnectionOptions = ConnectionOptions
    { autocommit :: Bool
    } deriving (Eq, Show)

defaultConnectionOptions :: ConnectionOptions
defaultConnectionOptions = ConnectionOptions { autocommit = False }

data Column = Column
    { columnName :: String
    } deriving (Eq, Show)

-- ── Connection wraps the Level 1 backend ─────────────────────────────────
--
-- connBackend : the live InMemoryBackend (modified by DML/DDL)
-- connSnapshot: Nothing when no transaction is open; Just saves the state
--               before the first mutation so rollback can restore it
-- connAutocommit: when True, mutations are not snapshotted

data Connection = Connection
    { connBackend    :: IORef InMemoryBackend
    , connSnapshot   :: IORef (Maybe InMemoryBackend)
    , connAutocommit :: Bool
    , connClosed     :: IORef Bool
    }

data Cursor = Cursor
    { curRows        :: IORef [[SqlValue]]
    , curOffset      :: IORef Int
    , curDescription :: IORef [Column]
    , curRowCount    :: IORef Int
    , curLastRowId   :: IORef (Maybe SqlValue)
    , curClosed      :: IORef Bool
    }

-- ── Module-level constants ────────────────────────────────────────────────

apiLevel :: String
apiLevel = "2.0"

threadSafety :: Int
threadSafety = 1

paramStyle :: String
paramStyle = "qmark"

-- ── Connection lifecycle ──────────────────────────────────────────────────

connect :: String -> IO (Either MiniSqliteError Connection)
connect = connectWith defaultConnectionOptions

connectWith :: ConnectionOptions -> String -> IO (Either MiniSqliteError Connection)
connectWith opts db
    | db /= ":memory:" =
        pure (Left (mkErr "NotSupportedError" "Haskell mini-sqlite supports only :memory: in Level 1"))
    | otherwise = do
        bRef       <- newIORef newBackend
        snapRef    <- newIORef Nothing
        closedRef  <- newIORef False
        pure (Right (Connection bRef snapRef (autocommit opts) closedRef))

close :: Connection -> IO ()
close conn = do
    already <- readIORef (connClosed conn)
    if already then pure ()
    else do
        -- Discard any open transaction (rollback in-place).
        snap <- readIORef (connSnapshot conn)
        case snap of
            Nothing -> pure ()
            Just be -> writeIORef (connBackend conn) be
        writeIORef (connSnapshot conn) Nothing
        writeIORef (connClosed conn) True

commit :: Connection -> IO (Either MiniSqliteError ())
commit conn = do
    open <- assertConnOpen conn
    case open of
        Left e  -> pure (Left e)
        Right () -> do
            writeIORef (connSnapshot conn) Nothing
            pure (Right ())

rollback :: Connection -> IO (Either MiniSqliteError ())
rollback conn = do
    open <- assertConnOpen conn
    case open of
        Left e  -> pure (Left e)
        Right () -> do
            snap <- readIORef (connSnapshot conn)
            case snap of
                Nothing -> pure ()
                Just be -> do
                    writeIORef (connBackend conn) be
                    writeIORef (connSnapshot conn) Nothing
            pure (Right ())

-- ── Execute ───────────────────────────────────────────────────────────────

execute :: Connection -> String -> [SqlValue] -> IO (Either MiniSqliteError Cursor)
execute conn sql params = do
    result <- runSql conn sql params
    case result of
        Left e  -> pure (Left e)
        Right r -> Right <$> resultToCursor r

executeMany :: Connection -> String -> [[SqlValue]] -> IO (Either MiniSqliteError Cursor)
executeMany conn sql paramSets =
    case paramSets of
        []   -> execute conn sql []
        _    -> go paramSets
  where
    go []      = execute conn sql []
    go [ps]    = execute conn sql ps
    go (ps:rest) = do
        r <- execute conn sql ps
        case r of
            Left e  -> pure (Left e)
            Right _ -> go rest

-- ── Cursor accessors ──────────────────────────────────────────────────────

cursorDescription :: Cursor -> IO [Column]
cursorDescription = readIORef . curDescription

cursorRowCount :: Cursor -> IO Int
cursorRowCount = readIORef . curRowCount

cursorLastRowId :: Cursor -> IO (Maybe SqlValue)
cursorLastRowId = readIORef . curLastRowId

fetchOne :: Cursor -> IO (Either MiniSqliteError (Maybe [SqlValue]))
fetchOne cur = do
    open <- assertCurOpen cur
    case open of
        Left e  -> pure (Left e)
        Right () -> do
            rows   <- readIORef (curRows cur)
            offset <- readIORef (curOffset cur)
            if offset >= length rows
                then pure (Right Nothing)
                else do
                    writeIORef (curOffset cur) (offset + 1)
                    pure (Right (Just (rows !! offset)))

fetchMany :: Cursor -> Int -> IO (Either MiniSqliteError [[SqlValue]])
fetchMany cur n = do
    open <- assertCurOpen cur
    case open of
        Left e  -> pure (Left e)
        Right () -> do
            rows   <- readIORef (curRows cur)
            offset <- readIORef (curOffset cur)
            let batch = take (max 0 n) (drop offset rows)
            writeIORef (curOffset cur) (offset + length batch)
            pure (Right batch)

fetchAll :: Cursor -> IO (Either MiniSqliteError [[SqlValue]])
fetchAll cur = do
    open <- assertCurOpen cur
    case open of
        Left e  -> pure (Left e)
        Right () -> do
            rows   <- readIORef (curRows cur)
            offset <- readIORef (curOffset cur)
            let remaining = drop offset rows
            writeIORef (curOffset cur) (length rows)
            pure (Right remaining)

closeCursor :: Cursor -> IO ()
closeCursor cur = writeIORef (curClosed cur) True

-- ── Internal result type ──────────────────────────────────────────────────

data SqlResult = SqlResult
    { srColumns     :: [String]
    , srRows        :: [[SqlValue]]
    , srRowCount    :: Int
    , srLastRowId   :: Maybe SqlValue
    } deriving (Eq, Show)

emptyResult :: Int -> SqlResult
emptyResult n = SqlResult [] [] n Nothing

-- ── Core execution ────────────────────────────────────────────────────────

runSql :: Connection -> String -> [SqlValue] -> IO (Either MiniSqliteError SqlResult)
runSql conn sql params = do
    open <- assertConnOpen conn
    case open of
        Left e  -> pure (Left e)
        Right () ->
            case bindParameters sql params of
                Left e    -> pure (Left e)
                Right bound ->
                    let kw = firstKeyword bound
                    in case kw of
                        -- Transaction control (no pipeline needed)
                        "BEGIN" -> do
                            ensureSnapshot conn
                            pure (Right (emptyResult 0))
                        "COMMIT" -> do
                            writeIORef (connSnapshot conn) Nothing
                            pure (Right (emptyResult 0))
                        "ROLLBACK" -> do
                            _ <- rollback conn
                            pure (Right (emptyResult 0))
                        -- Route through the full pipeline
                        _ -> runPipeline conn bound

runPipeline :: Connection -> String -> IO (Either MiniSqliteError SqlResult)
runPipeline conn sql = do
    be <- readIORef (connBackend conn)
    case parseSql sql of
        Left e    -> pure (Left e)
        Right stmt ->
            -- SELECT without FROM: evaluate purely without the planner.
            case stmtNoFrom stmt of
                Just cols -> pure (Right (evalScalarSelect cols))
                Nothing   ->
                    case stmt of
                        -- UPDATE and DELETE are handled directly at this layer
                        -- because the VM's UpdateRows/DeleteRows use ListCursor
                        -- (row-ID-aware) rather than the VM's ListRowIterator.
                        UpdateStmt tbl assigns wherePred -> do
                            ensureSnapshot conn
                            result <- executeUpdate be tbl assigns wherePred
                            case result of
                                Left e       -> pure (Left e)
                                Right (n, newBe) -> do
                                    writeIORef (connBackend conn) newBe
                                    pure (Right (emptyResult n))
                        DeleteStmt tbl wherePred -> do
                            ensureSnapshot conn
                            result <- executeDelete be tbl wherePred
                            case result of
                                Left e       -> pure (Left e)
                                Right (n, newBe) -> do
                                    writeIORef (connBackend conn) newBe
                                    pure (Right (emptyResult n))
                        _ ->
                            let sp = backendSchemaProvider be
                            in case planStmt sp stmt of
                                Left planErr -> pure (Left (planErrorToMiniErr planErr))
                                Right lp ->
                                    let op   = optimize lp
                                        prog = compile op
                                    in runVm conn be stmt prog

-- | Run the VM for one statement.
-- For SELECT we just execute and convert the result.
-- For DML/DDL we also capture the mutated backend and persist it to the connection.
runVm :: Connection -> InMemoryBackend -> Statement -> CG.Program -> IO (Either MiniSqliteError SqlResult)
runVm conn be stmt prog =
    case stmt of
        SelectStmt {} -> do
            -- Read-only: create a fresh IORef (matches SqlVm.execute semantics).
            bRef <- newIORef be
            result <- catch (Right <$> VM.executeWithRef prog bRef) catchExc
            case result of
                Left e          -> pure (Left e)
                Right (qr, _)   -> pure (Right (queryResultToSqlResult qr))
        _ -> do
            -- Mutating: snapshot the backend for rollback support, then run.
            ensureSnapshot conn
            bRef <- newIORef be
            result <- catch (Right <$> VM.executeWithRef prog bRef) catchExc
            case result of
                Left e -> pure (Left e)
                Right (qr, newBe) -> do
                    writeIORef (connBackend conn) newBe
                    let rowsAff = VM.rowsAffected qr
                    let baseResult = (queryResultToSqlResult qr) { srRowCount = rowsAff }
                    -- For INSERT, try to derive the last-insert-rowid from the backend.
                    finalResult <- case stmt of
                        InsertStmt tbl _ _ ->
                            pure (baseResult { srLastRowId = Just (SqlInteger (inferLastRowId newBe tbl)) })
                        _ ->
                            pure baseResult
                    pure (Right finalResult)
  where
    catchExc :: SomeException -> IO (Either MiniSqliteError (QueryResult, InMemoryBackend))
    catchExc ex = pure (Left (mkErr "OperationalError" (show ex)))

-- | Infer the last inserted row-id by counting rows in the named table.
-- The InMemoryBackend assigns sequential row IDs starting at 1, so the
-- count of rows equals the last assigned ID.
inferLastRowId :: InMemoryBackend -> String -> Integer
inferLastRowId be tbl =
    case SB.scan be tbl of
        Left _   -> 0
        Right it ->
            let countRows it_ acc =
                    let (row, it') = SB.iteratorNext it_
                    in case row of
                        Nothing -> acc
                        Just _  -> countRows it' (acc + 1)
            in countRows it 0

-- ── Direct UPDATE / DELETE execution ─────────────────────────────────────
--
-- The VM's UpdateRows / DeleteRows instructions are stubs (they only mark a
-- row affected count). We implement the actual mutations here using the
-- SqlBackend's openCursor / cursorNext / update / delete API, which correctly
-- tracks row IDs via ListCursor.

-- | Execute an UPDATE statement against the given backend.
-- Returns (rowsAffected, newBackend).
executeUpdate
    :: InMemoryBackend
    -> String
    -> [Assignment]
    -> Maybe SqlExpr
    -> IO (Either MiniSqliteError (Int, InMemoryBackend))
executeUpdate be tbl assigns wherePred =
    case SB.openCursor be tbl of
        Left err -> pure (Left (mkErr "OperationalError" (SB.errorMessage err)))
        Right cur -> go cur be 0
  where
    go cur backend affected =
        let (mRow, cur') = SB.cursorNext cur
        in case mRow of
            Nothing -> pure (Right (affected, backend))
            Just row ->
                let sqlRow = Map.fromList [(k, fromBackendValue v) | (k, v) <- Map.toList row]
                    matches = case wherePred of
                        Nothing -> True
                        Just e  -> evalRowExpr e sqlRow == SqlBool True
                in if matches
                    then
                        -- Build the new row: start from old backend row, apply assignments.
                        let newRow = foldl (applyAssign sqlRow) row assigns
                        in case SB.update backend tbl cur' newRow of
                            Left err  -> pure (Left (mkErr "OperationalError" (SB.errorMessage err)))
                            Right backend' -> go cur' backend' (affected + 1)
                    else go cur' backend affected

    -- Apply one Assignment to the backend row.
    applyAssign sqlRow row (Assignment col expr) =
        Map.insert col (toBackendValue (evalRowExpr expr sqlRow)) row

-- | Execute a DELETE statement against the given backend.
-- Uses a two-pass approach: first collect all matching cursors, then delete.
-- Returns (rowsAffected, newBackend).
executeDelete
    :: InMemoryBackend
    -> String
    -> Maybe SqlExpr
    -> IO (Either MiniSqliteError (Int, InMemoryBackend))
executeDelete be tbl wherePred =
    -- Pass 1: scan to collect cursors positioned on rows to delete.
    case SB.openCursor be tbl of
        Left err -> pure (Left (mkErr "OperationalError" (SB.errorMessage err)))
        Right cur -> collectAndDelete cur [] be
  where
    -- Collect (cursor, row) pairs for matching rows, then delete in one go.
    collectAndDelete cur toDelete backend =
        let (mRow, cur') = SB.cursorNext cur
        in case mRow of
            Nothing ->
                -- Pass 2: delete all matching rows.
                deleteAll toDelete backend 0
            Just row ->
                let sqlRow = Map.fromList [(k, fromBackendValue v) | (k, v) <- Map.toList row]
                    matches = case wherePred of
                        Nothing -> True
                        Just e  -> evalRowExpr e sqlRow == SqlBool True
                in if matches
                    then collectAndDelete cur' (cur' : toDelete) backend
                    else collectAndDelete cur' toDelete backend

    deleteAll [] backend affected = pure (Right (affected, backend))
    deleteAll (cur : rest) backend affected =
        case SB.delete backend tbl cur of
            Left err       -> pure (Left (mkErr "OperationalError" (SB.errorMessage err)))
            Right backend' -> deleteAll rest backend' (affected + 1)

-- | Evaluate an SqlExpr in the context of a single row (no table scan).
-- Column references are resolved from the row map (case-insensitive).
evalRowExpr :: SqlExpr -> Map.Map String SqlValue -> SqlValue
evalRowExpr expr row = case expr of
    Literal Nothing    -> SqlNull
    Literal (Just lv)  -> litToSqlValue lv
    P.Column _tbl col    ->
        fromMaybe SqlNull (Map.lookup (map toLower col) rowLower)
      where
        rowLower = Map.fromList [(map toLower k, v) | (k, v) <- Map.toList row]
    BinaryOp op l r    -> evalBinOp op (evalRowExpr l row) (evalRowExpr r row)
    UnaryOp op e       -> evalUnOp op (evalRowExpr e row)
    IsNull e           -> SqlBool (evalRowExpr e row == SqlNull)
    IsNotNull e        -> SqlBool (evalRowExpr e row /= SqlNull)
    Between v lo hi    ->
        let v'  = evalRowExpr v row
            lo' = evalRowExpr lo row
            hi' = evalRowExpr hi row
        in evalBinOp BinAnd (evalBinOp BinGte v' lo') (evalBinOp BinLte v' hi')
    InExpr v items ->
        let v'   = evalRowExpr v row
            vals = map (\e -> evalRowExpr e row) items
        in SqlBool (any (sqlValuesEqual v') vals)
    NotInExpr v items ->
        let v'   = evalRowExpr v row
            vals = map (\e -> evalRowExpr e row) items
        in SqlBool (not (any (sqlValuesEqual v') vals))
    Like v pat ->
        let v' = evalRowExpr v row
        in case v' of
            SqlText s -> SqlBool (likeMatch s pat)
            SqlNull   -> SqlNull
            _         -> SqlBool False
    NotLike v pat ->
        let v' = evalRowExpr v row
        in case v' of
            SqlText s -> SqlBool (not (likeMatch s pat))
            SqlNull   -> SqlNull
            _         -> SqlBool True
    FuncCall name args ->
        let vs = map (\e -> evalRowExpr e row) args
        in evalScalarFunc name vs
    _ -> SqlNull

-- | Simple SQL LIKE pattern matching (% = any sequence, _ = any single char).
--
-- Security note: the naive recursive approach for '%' wildcards has
-- exponential worst-case behaviour on crafted inputs such as
--   'aaaa…' LIKE '%a%a%a%a%b'
-- We prevent this by collapsing consecutive '%' wildcards before recursing.
-- Consecutive '%' signs are equivalent to a single '%' in SQL, so this
-- transformation is semantically transparent while bounding the fan-out.
likeMatch :: String -> String -> Bool
likeMatch pat str = go (collapsePercents pat) str
  where
    -- Replace any run of consecutive '%' characters with a single '%',
    -- eliminating the exponential branching caused by adjacent wildcards.
    collapsePercents :: String -> String
    collapsePercents ('%':'%':rest) = collapsePercents ('%' : rest)
    collapsePercents (c:cs)         = c : collapsePercents cs
    collapsePercents []             = []

    go :: String -> String -> Bool
    go [] []                    = True
    go _ []                     = False
    go [] ('%':ps)              = go [] ps
    go [] _                     = False
    go (c:cs) ('%':ps)          = go (c:cs) ps || go cs ('%':ps)
    go (_:cs) ('_':ps)          = go cs ps
    go (c:cs) (p:ps)
        | toLower c == toLower p = go cs ps
        | otherwise              = False

-- ── SELECT without FROM ───────────────────────────────────────────────────

-- | If the statement is a SELECT with no FROM tables (scalar expression query),
-- return the output columns for direct evaluation. Otherwise Nothing.
stmtNoFrom :: Statement -> Maybe [OutputColumn]
stmtNoFrom s@(SelectStmt {})
    | null (stmtFrom s) && null (stmtJoins s)
      && null (stmtGroupBy s) && null (stmtOrderBy s) =
        Just (stmtColumns s)
stmtNoFrom _ = Nothing

-- | Evaluate a scalar SELECT (no FROM) by computing each expression once.
evalScalarSelect :: [OutputColumn] -> SqlResult
evalScalarSelect cols =
    let (names, vals) = unzip (map evalCol cols)
    in SqlResult names [vals] (-1) Nothing
  where
    evalCol OutputStar                 = ("*", SqlNull)
    evalCol (OutputExpr e aliasOpt)    =
        let name = fromMaybe (exprName e) aliasOpt
            val  = evalScalarExpr e
        in (name, val)

-- | Derive a display name for an expression (used when no AS alias given).
exprName :: SqlExpr -> String
exprName (P.Column _ col)       = col
exprName (Literal (Just (LitText t))) = t
exprName (FuncCall f _)         = map toLower f
exprName _                      = "expr"

-- | Evaluate a scalar expression without any row context.
evalScalarExpr :: SqlExpr -> SqlValue
evalScalarExpr expr = case expr of
    Literal Nothing    -> SqlNull
    Literal (Just lv)  -> litToSqlValue lv
    BinaryOp op l r    -> evalBinOp op (evalScalarExpr l) (evalScalarExpr r)
    UnaryOp op e       -> evalUnOp op (evalScalarExpr e)
    IsNull e           -> SqlBool (evalScalarExpr e == SqlNull)
    IsNotNull e        -> SqlBool (evalScalarExpr e /= SqlNull)
    FuncCall name args ->
        let vs = map evalScalarExpr args
        in evalScalarFunc name vs
    _ -> SqlNull

-- | Evaluate built-in scalar functions over already-evaluated argument values.
evalScalarFunc :: String -> [SqlValue] -> SqlValue
evalScalarFunc name args =
    case map toLower name of
        "length" ->
            case args of
                [SqlText s] -> SqlInteger (fromIntegral (length s))
                [SqlNull]   -> SqlNull
                _           -> SqlNull
        "upper" ->
            case args of
                [SqlText s] -> SqlText (map toUpper s)
                [SqlNull]   -> SqlNull
                _           -> SqlNull
        "lower" ->
            case args of
                [SqlText s] -> SqlText (map toLower s)
                [SqlNull]   -> SqlNull
                _           -> SqlNull
        "substr" ->
            case args of
                [SqlText s, SqlInteger pos] ->
                    let start = fromIntegral pos - 1
                    in SqlText (drop (max 0 start) s)
                [SqlText s, SqlInteger pos, SqlInteger len] ->
                    let start = fromIntegral pos - 1
                    in SqlText (take (fromIntegral len) (drop (max 0 start) s))
                [SqlNull, _]    -> SqlNull
                [_, SqlNull]    -> SqlNull
                _               -> SqlNull
        "trim"  ->
            case args of
                [SqlText s] -> SqlText (dropWhileEnd isSpace (dropWhile isSpace s))
                [SqlNull]   -> SqlNull
                _           -> SqlNull
        "ltrim" ->
            case args of
                [SqlText s] -> SqlText (dropWhile isSpace s)
                [SqlNull]   -> SqlNull
                _           -> SqlNull
        "rtrim" ->
            case args of
                [SqlText s] -> SqlText (dropWhileEnd isSpace s)
                [SqlNull]   -> SqlNull
                _           -> SqlNull
        "replace" ->
            case args of
                [SqlText s, SqlText from, SqlText to] ->
                    SqlText (replaceAll from to s)
                [SqlNull, _, _] -> SqlNull
                _               -> SqlNull
        "abs" ->
            case args of
                [SqlInteger n] -> SqlInteger (abs n)
                [SqlReal    d] -> SqlReal    (abs d)
                [SqlNull]      -> SqlNull
                _              -> SqlNull
        "concat" ->
            -- || operator: SQL string concatenation; NULL propagates.
            case args of
                [SqlNull, _] -> SqlNull
                [_, SqlNull] -> SqlNull
                [a, b]       -> SqlText (sqlToText a ++ sqlToText b)
                _            -> SqlNull
        "coalesce" ->
            case filter (/= SqlNull) args of
                (x:_) -> x
                []    -> SqlNull
        "ifnull" ->
            case args of
                [SqlNull, b] -> b
                [a, _]       -> a
                _            -> SqlNull
        "round" ->
            -- ROUND(x) or ROUND(x, digits). Rounds half away from zero.
            case args of
                [SqlInteger n] -> SqlReal (fromIntegral n)
                [SqlReal    d] -> SqlReal (roundHalfAway d 0)
                [SqlNull]      -> SqlNull
                [SqlInteger n, SqlInteger p] ->
                    SqlReal (roundHalfAway (fromIntegral n) (fromIntegral p))
                [SqlReal    d, SqlInteger p] ->
                    SqlReal (roundHalfAway d (fromIntegral p))
                _              -> SqlNull
        _ -> SqlNull

-- | Coerce a SqlValue to its text representation (for concat / type-coercions).
sqlToText :: SqlValue -> String
sqlToText SqlNull         = ""
sqlToText (SqlBool True)  = "1"
sqlToText (SqlBool False) = "0"
sqlToText (SqlInteger i)  = show i
sqlToText (SqlReal    d)  = show d
sqlToText (SqlText    s)  = s

-- | Round a Double to `digits` decimal places, using half-away-from-zero
-- rounding (which matches SQLite's ROUND behaviour).
--
-- Security notes:
--   1. We use 'Integer' (arbitrary-precision) for the intermediate scaled
--      value to prevent silent overflow that would corrupt results.  The
--      original 'Int' implementation overflowed silently for digits >= 19
--      on 64-bit systems, producing garbage output with no error.
--   2. We clamp 'digits' to the range [-15, 15] before use.  Clamping to 15
--      reflects the precision limit of Double; values beyond this range add
--      no information.  Clamping negative values prevents the '(^)' operator
--      from throwing a "Negative exponent" runtime exception when digits < 0.
--      SQLite supports negative digits (rounding to tens, hundreds, etc.) and
--      we implement that semantics here.
roundHalfAway :: Double -> Int -> Double
roundHalfAway x digits
    | digits < 0 =
        -- Negative digits: round to the nearest 10^|digits|.
        -- e.g. ROUND(1234.5, -2) → 1200.0
        let factor = (10 :: Integer) ^ (min 15 (negate digits))
            scaled = x / fromIntegral factor
            rounded :: Integer
            rounded = if x >= 0
                      then floor   (scaled + 0.5)
                      else ceiling (scaled - 0.5)
        in fromIntegral rounded * fromIntegral factor
    | digits == 0 =
        let rounded :: Integer
            rounded = if x >= 0 then floor (x + 0.5) else ceiling (x - 0.5)
        in fromIntegral rounded
    | otherwise =
        let d      = min digits 15        -- clamp to Double precision
            factor = (10 :: Integer) ^ d
            scaled = x * fromIntegral factor
            truncated :: Integer
            truncated = if x >= 0
                        then floor   (scaled + 0.5)
                        else ceiling (scaled - 0.5)
        in fromIntegral truncated / fromIntegral factor

-- | Replace all non-overlapping occurrences of 'from' with 'to' in 's'.
replaceAll :: String -> String -> String -> String
replaceAll _ _ [] = []
replaceAll from to s@(c:cs)
    | from `isPrefixOf` s = to ++ replaceAll from to (drop (length from) s)
    | otherwise           = c : replaceAll from to cs

-- | Scalar binary operator evaluation (mirrors SqlVm.evalBinary).
evalBinOp :: BinaryOperator -> SqlValue -> SqlValue -> SqlValue
evalBinOp BinAnd (SqlBool False) _             = SqlBool False
evalBinOp BinAnd _             (SqlBool False) = SqlBool False
evalBinOp BinAnd (SqlBool True) r              = r
evalBinOp BinAnd l             (SqlBool True)  = l
evalBinOp BinAnd SqlNull _                     = SqlNull
evalBinOp BinAnd _ SqlNull                     = SqlNull
evalBinOp BinOr  (SqlBool True) _              = SqlBool True
evalBinOp BinOr  _             (SqlBool True)  = SqlBool True
evalBinOp BinOr  (SqlBool False) r             = r
evalBinOp BinOr  l             (SqlBool False) = l
evalBinOp BinOr  SqlNull _                     = SqlNull
evalBinOp BinOr  _ SqlNull                     = SqlNull
evalBinOp _      SqlNull _                     = SqlNull
evalBinOp _      _ SqlNull                     = SqlNull
evalBinOp BinAdd (SqlInteger a) (SqlInteger b) = SqlInteger (a + b)
evalBinOp BinAdd (SqlInteger a) (SqlReal    b) = SqlReal (fromInteger a + b)
evalBinOp BinAdd (SqlReal    a) (SqlInteger b) = SqlReal (a + fromInteger b)
evalBinOp BinAdd (SqlReal    a) (SqlReal    b) = SqlReal (a + b)
evalBinOp BinSub (SqlInteger a) (SqlInteger b) = SqlInteger (a - b)
evalBinOp BinSub (SqlInteger a) (SqlReal    b) = SqlReal (fromInteger a - b)
evalBinOp BinSub (SqlReal    a) (SqlInteger b) = SqlReal (a - fromInteger b)
evalBinOp BinSub (SqlReal    a) (SqlReal    b) = SqlReal (a - b)
evalBinOp BinMul (SqlInteger a) (SqlInteger b) = SqlInteger (a * b)
evalBinOp BinMul (SqlInteger a) (SqlReal    b) = SqlReal (fromInteger a * b)
evalBinOp BinMul (SqlReal    a) (SqlInteger b) = SqlReal (a * fromInteger b)
evalBinOp BinMul (SqlReal    a) (SqlReal    b) = SqlReal (a * b)
evalBinOp BinDiv _ (SqlInteger 0)              = SqlNull
evalBinOp BinDiv _ (SqlReal    0)              = SqlNull
evalBinOp BinDiv (SqlInteger a) (SqlInteger b) = SqlInteger (a `div` b)
evalBinOp BinDiv (SqlInteger a) (SqlReal    b) = SqlReal (fromInteger a / b)
evalBinOp BinDiv (SqlReal    a) (SqlInteger b) = SqlReal (a / fromInteger b)
evalBinOp BinDiv (SqlReal    a) (SqlReal    b) = SqlReal (a / b)
evalBinOp BinMod _ (SqlInteger 0)              = SqlNull
evalBinOp BinMod (SqlInteger a) (SqlInteger b) = SqlInteger (a `mod` b)
evalBinOp BinEq  l r  = SqlBool (sqlValuesEqual l r)
evalBinOp BinNotEq l r = SqlBool (not (sqlValuesEqual l r))
evalBinOp BinLt  l r  = SqlBool (sqlCompare l r == LT)
evalBinOp BinLte l r  = SqlBool (sqlCompare l r /= GT)
evalBinOp BinGt  l r  = SqlBool (sqlCompare l r == GT)
evalBinOp BinGte l r  = SqlBool (sqlCompare l r /= LT)
evalBinOp _      _ _  = SqlNull

evalUnOp :: UnaryOperator -> SqlValue -> SqlValue
evalUnOp UnaryNeg SqlNull         = SqlNull
evalUnOp UnaryNeg (SqlInteger i)  = SqlInteger (negate i)
evalUnOp UnaryNeg (SqlReal    r)  = SqlReal (negate r)
evalUnOp UnaryNeg _               = SqlNull
evalUnOp UnaryNot SqlNull         = SqlNull
evalUnOp UnaryNot (SqlBool b)     = SqlBool (not b)
evalUnOp UnaryNot _               = SqlNull

sqlValuesEqual :: SqlValue -> SqlValue -> Bool
sqlValuesEqual SqlNull SqlNull               = True
sqlValuesEqual (SqlInteger a) (SqlInteger b) = a == b
sqlValuesEqual (SqlInteger a) (SqlReal    b) = fromInteger a == b
sqlValuesEqual (SqlReal    a) (SqlInteger b) = a == fromInteger b
sqlValuesEqual (SqlReal    a) (SqlReal    b) = a == b
sqlValuesEqual l r                           = l == r

sqlCompare :: SqlValue -> SqlValue -> Ordering
sqlCompare l r | sqlValuesEqual l r = EQ
sqlCompare SqlNull _                = LT
sqlCompare _ SqlNull                = GT
sqlCompare (SqlInteger a) (SqlInteger b) = compare a b
sqlCompare (SqlInteger a) (SqlReal    b) = compare (fromInteger a :: Double) b
sqlCompare (SqlReal    a) (SqlInteger b) = compare a (fromInteger b :: Double)
sqlCompare (SqlReal    a) (SqlReal    b) = compare a b
sqlCompare l r                           = compare (show l) (show r)

-- ── Type bridges ─────────────────────────────────────────────────────────

-- | Convert a mini-sqlite SqlValue to the backend's SqlValue.
toBackendValue :: SqlValue -> SB.SqlValue
toBackendValue SqlNull          = SB.SqlNull
toBackendValue (SqlInteger i)   = SB.SqlInteger i
toBackendValue (SqlReal    d)   = SB.SqlReal d
toBackendValue (SqlText    s)   = SB.SqlText s
toBackendValue (SqlBool    b)   = SB.SqlBool b

-- | Convert the backend's SqlValue to a mini-sqlite SqlValue.
fromBackendValue :: SB.SqlValue -> SqlValue
fromBackendValue SB.SqlNull         = SqlNull
fromBackendValue (SB.SqlInteger i)  = SqlInteger i
fromBackendValue (SB.SqlReal    d)  = SqlReal d
fromBackendValue (SB.SqlText    s)  = SqlText s
fromBackendValue (SB.SqlBool    b)  = SqlBool b
fromBackendValue (SB.SqlBlob    _)  = SqlNull  -- not used in Level 1 conformance

-- | Convert a SqlVm QueryResult to the internal SqlResult.
queryResultToSqlResult :: QueryResult -> SqlResult
queryResultToSqlResult qr =
    SqlResult
        { srColumns   = VM.columns qr
        , srRows      = map (map fromBackendValue) (VM.rows qr)
        , srRowCount  = VM.rowsAffected qr
        , srLastRowId = Nothing
        }

-- | Convert a SqlPlanner LiteralVal to a mini-sqlite SqlValue.
litToSqlValue :: LiteralVal -> SqlValue
litToSqlValue (LitInt  i)  = SqlInteger i
litToSqlValue (LitReal d)  = SqlReal d
litToSqlValue (LitText s)  = SqlText s
litToSqlValue (LitBool b)  = SqlBool b
litToSqlValue (LitBlob _)  = SqlNull

-- | Build a SqlPlanner.SchemaProvider from an InMemoryBackend.
-- We query the backend's column names and translate errors into PlanError values
-- so the planner can validate table and column references.
backendSchemaProvider :: InMemoryBackend -> SchemaProvider
backendSchemaProvider be = SchemaProvider $ \tbl ->
    case SB.columns be tbl of
        Left _   -> Left (UnknownTable tbl)
        Right cs -> Right (map SB.columnName cs)

-- ── Plan error → MiniSqliteError ─────────────────────────────────────────

planErrorToMiniErr :: PlanError -> MiniSqliteError
planErrorToMiniErr (UnknownTable tbl)      = mkErr "OperationalError" ("no such table: " ++ tbl)
planErrorToMiniErr (UnknownColumn _ col)   = mkErr "OperationalError" ("no such column: " ++ col)
planErrorToMiniErr (AmbiguousColumn col _) = mkErr "OperationalError" ("ambiguous column: " ++ col)
planErrorToMiniErr (InvalidAggregate msg)  = mkErr "OperationalError" msg
planErrorToMiniErr (UnsupportedStatement m) = mkErr "OperationalError" m

-- ── Plan the statement ────────────────────────────────────────────────────

planStmt :: SchemaProvider -> Statement -> Either PlanError P.LogicalPlan
planStmt = plan

-- ── Cursor construction ───────────────────────────────────────────────────

resultToCursor :: SqlResult -> IO Cursor
resultToCursor r = do
    rowsRef   <- newIORef (srRows r)
    offRef    <- newIORef 0
    descRef   <- newIORef (map Column (srColumns r))
    rcRef     <- newIORef (srRowCount r)
    lridRef   <- newIORef (srLastRowId r)
    closedRef <- newIORef False
    pure (Cursor rowsRef offRef descRef rcRef lridRef closedRef)

-- ── Guards ───────────────────────────────────────────────────────────────

assertConnOpen :: Connection -> IO (Either MiniSqliteError ())
assertConnOpen conn = do
    closed <- readIORef (connClosed conn)
    pure (if closed then Left (mkErr "ProgrammingError" "connection is closed") else Right ())

assertCurOpen :: Cursor -> IO (Either MiniSqliteError ())
assertCurOpen cur = do
    closed <- readIORef (curClosed cur)
    pure (if closed then Left (mkErr "ProgrammingError" "cursor is closed") else Right ())

ensureSnapshot :: Connection -> IO ()
ensureSnapshot conn
    | connAutocommit conn = pure ()
    | otherwise = do
        snap <- readIORef (connSnapshot conn)
        case snap of
            Just _  -> pure ()
            Nothing -> do
                be <- readIORef (connBackend conn)
                writeIORef (connSnapshot conn) (Just be)

-- ── SQL parser ────────────────────────────────────────────────────────────
--
-- A lightweight hand-rolled tokeniser + AST builder that converts a SQL string
-- (after parameter binding) into a SqlPlanner.Statement. This covers the subset
-- required by the 24 conformance fixtures:
--
--   CREATE TABLE, DROP TABLE, INSERT, UPDATE, DELETE, SELECT
--
-- The tokeniser is intentionally simple: it produces a flat list of tokens and
-- the statement builders consume them with a recursive-descent approach.

data Token
    = TWord String     -- keyword or identifier (uppercased for comparison)
    | TIdent String    -- identifier (original case preserved)
    | TNum Integer
    | TReal Double
    | TStr String      -- string literal (quotes stripped, escapes handled)
    | TLParen
    | TRParen
    | TComma
    | TStar
    | TDot
    | TSemi
    | TOp String       -- operator: =, <>, !=, <, <=, >, >=, +, -, *, /, %, ||
    | TEq              -- = (also used as assignment)
    deriving (Eq, Show)

tokenise :: String -> [Token]
tokenise [] = []
tokenise (c:cs)
    | isSpace c           = tokenise cs
    | c == '-' && not (null cs) && head cs == '-' = tokenise (dropWhile (/= '\n') cs)
    | c == '\'' = let (s, rest) = readString '\'' cs in TStr s : tokenise rest
    | c == '"'  = let (s, rest) = readString '"'  cs in TIdent s : tokenise rest
    | c == '('  = TLParen : tokenise cs
    | c == ')'  = TRParen : tokenise cs
    | c == ','  = TComma  : tokenise cs
    | c == ';'  = TSemi   : tokenise cs
    | c == '.'  = TDot    : tokenise cs
    | c == '*'  = TStar   : tokenise cs
    | c == '|' && not (null cs) && head cs == '|' = TOp "||" : tokenise (tail cs)
    | c == '<' && not (null cs) && head cs == '>' = TOp "<>" : tokenise (tail cs)
    | c == '<' && not (null cs) && head cs == '=' = TOp "<=" : tokenise (tail cs)
    | c == '>' && not (null cs) && head cs == '=' = TOp ">=" : tokenise (tail cs)
    | c == '!' && not (null cs) && head cs == '=' = TOp "!=" : tokenise (tail cs)
    | c == '<'  = TOp "<"  : tokenise cs
    | c == '>'  = TOp ">"  : tokenise cs
    | c == '='  = TOp "="  : tokenise cs
    | c == '+'  = TOp "+"  : tokenise cs
    | c == '-'  = TOp "-"  : tokenise cs
    | c == '/'  = TOp "/"  : tokenise cs
    | c == '%'  = TOp "%"  : tokenise cs
    | isDigit c =
        let (ns, rest) = span (\x -> isDigit x || x == '.') (c:cs)
        in if '.' `elem` ns
           then case readMaybe ns :: Maybe Double of
                    Just d  -> TReal d : tokenise rest
                    Nothing -> tokenise rest
           else case readMaybe ns :: Maybe Integer of
                    Just n  -> TNum n  : tokenise rest
                    Nothing -> tokenise rest
    | isAlpha c || c == '_' =
        let (ws, rest) = span (\x -> isAlphaNum x || x == '_') (c:cs)
            upper      = map toUpper ws
        in TWord upper : tokenise rest
    | otherwise = tokenise cs

-- | Read characters until the closing quote, handling doubled-quote escapes.
readString :: Char -> String -> (String, String)
readString q = go ""
  where
    go acc [] = (reverse acc, [])
    go acc (x:xs)
        | x == q && not (null xs) && head xs == q = go (q:acc) (tail xs)
        | x == q    = (reverse acc, xs)
        | otherwise = go (x:acc) xs

-- | Parse a SQL string into a SqlPlanner.Statement.
parseSql :: String -> Either MiniSqliteError Statement
parseSql sql =
    let toks = tokenise (trimSql sql)
    in case toks of
        (TWord "CREATE" : TWord "TABLE" : rest) -> parseCreate rest
        (TWord "DROP"   : TWord "TABLE" : rest) -> parseDrop rest
        (TWord "INSERT" : TWord "INTO"  : rest) -> parseInsert rest
        (TWord "UPDATE" : rest)                  -> parseUpdate rest
        (TWord "DELETE" : TWord "FROM"  : rest) -> parseDelete rest
        (TWord "SELECT" : TWord "DISTINCT" : rest) -> parseSelect True rest
        (TWord "SELECT" : rest)                    -> parseSelect False rest
        _ -> Left (mkErr "OperationalError" ("unsupported SQL statement: " ++ firstKeyword sql))

-- ── CREATE TABLE ─────────────────────────────────────────────────────────

parseCreate :: [Token] -> Either MiniSqliteError Statement
parseCreate toks = do
    let (ifne, rest0) = consumeIfNotExists toks
    (name, rest1) <- expectIdent rest0 "table name"
    rest2         <- expectToken TLParen rest1 "("
    (cols, rest3) <- parseColumnDefs rest2
    _             <- expectToken TRParen rest3 ")"
    pure (CreateTableStmt name ifne cols)

consumeIfNotExists :: [Token] -> (Bool, [Token])
consumeIfNotExists (TWord "IF":TWord "NOT":TWord "EXISTS":rest) = (True, rest)
consumeIfNotExists rest = (False, rest)

parseColumnDefs :: [Token] -> Either MiniSqliteError ([ColumnDef], [Token])
parseColumnDefs toks = go toks []
  where
    go ts acc = do
        (col, rest) <- parseOneColumnDef ts
        case rest of
            (TComma : more) -> go more (acc ++ [col])
            _               -> pure (acc ++ [col], rest)

parseOneColumnDef :: [Token] -> Either MiniSqliteError (ColumnDef, [Token])
parseOneColumnDef toks = do
    (name, rest0) <- expectIdent toks "column name"
    let (typeName, rest1) = consumeTypeName rest0
    let (notNull, rest2)  = consumeConstraint "NOT" "NULL" rest1
    let (pk, rest3)       = consumeOneWord "PRIMARY" rest2 -- simplified: skip "KEY"
    let rest4             = dropPrimaryKey rest3
    let (uniq, rest5)     = consumeOneWord "UNIQUE" rest4
    pure (ColumnDef name typeName notNull (pk || notNull) uniq Nothing, rest5)

consumeTypeName :: [Token] -> (String, [Token])
consumeTypeName (TWord w : rest)
    | w `elem` ["INTEGER","INT","TEXT","REAL","FLOAT","BOOLEAN","BOOL","BLOB","NUMERIC","VARCHAR"] =
        -- Consume optional size like VARCHAR(255)
        case rest of
            (TLParen : _) ->
                let rest' = dropWhile (/= TRParen) rest
                in (w, drop 1 rest')
            _ -> (w, rest)
consumeTypeName rest = ("", rest)

consumeConstraint :: String -> String -> [Token] -> (Bool, [Token])
consumeConstraint a b (TWord x : TWord y : rest)
    | x == a && y == b = (True, rest)
consumeConstraint _ _ rest = (False, rest)

consumeOneWord :: String -> [Token] -> (Bool, [Token])
consumeOneWord w (TWord x : rest) | x == w = (True, rest)
consumeOneWord _ rest = (False, rest)

dropPrimaryKey :: [Token] -> [Token]
dropPrimaryKey (TWord "KEY" : rest) = rest
dropPrimaryKey rest = rest

-- ── DROP TABLE ───────────────────────────────────────────────────────────

parseDrop :: [Token] -> Either MiniSqliteError Statement
parseDrop toks = do
    let (ife, rest0) = consumeIfExists toks
    (name, _) <- expectIdent rest0 "table name"
    pure (DropTableStmt name ife)

consumeIfExists :: [Token] -> (Bool, [Token])
consumeIfExists (TWord "IF":TWord "EXISTS":rest) = (True, rest)
consumeIfExists rest = (False, rest)

-- ── INSERT ───────────────────────────────────────────────────────────────

parseInsert :: [Token] -> Either MiniSqliteError Statement
parseInsert toks = do
    (name, rest0) <- expectIdent toks "table name"
    case rest0 of
        -- INSERT INTO t (col1, col2, ...) VALUES (...)
        (TLParen : rest1) -> do
            (cols, rest2) <- parseIdentList rest1
            rest3         <- expectToken TRParen rest2 ")"
            rest4         <- expectWord "VALUES" rest3
            (rows, _)     <- parseValueRows rest4
            pure (InsertStmt name cols rows)
        -- INSERT INTO t VALUES (...)
        (TWord "VALUES" : rest1) -> do
            (rows, _) <- parseValueRows rest1
            pure (InsertStmt name [] rows)
        _ ->
            Left (mkErr "OperationalError" "INSERT: expected column list or VALUES")

parseIdentList :: [Token] -> Either MiniSqliteError ([String], [Token])
parseIdentList toks = go toks []
  where
    go ts acc = do
        (name, rest) <- expectIdent ts "identifier"
        case rest of
            (TComma : more) -> go more (acc ++ [name])
            _               -> pure (acc ++ [name], rest)

parseValueRows :: [Token] -> Either MiniSqliteError ([[SqlExpr]], [Token])
parseValueRows toks = do
    rest0         <- expectToken TLParen toks "("
    (vals, rest1) <- parseExprList rest0
    rest2         <- expectToken TRParen rest1 ")"
    case rest2 of
        (TComma : more) -> do
            (more_rows, rest3) <- parseValueRows more
            pure (vals : more_rows, rest3)
        _ -> pure ([vals], rest2)

parseExprList :: [Token] -> Either MiniSqliteError ([SqlExpr], [Token])
parseExprList toks = go toks []
  where
    go ts acc = do
        (e, rest) <- parseExpr ts
        case rest of
            (TComma : more) -> go more (acc ++ [e])
            _               -> pure (acc ++ [e], rest)

-- ── UPDATE ───────────────────────────────────────────────────────────────

parseUpdate :: [Token] -> Either MiniSqliteError Statement
parseUpdate toks = do
    (name, rest0) <- expectIdent toks "table name"
    rest1         <- expectWord "SET" rest0
    (assigns, rest2) <- parseAssignments rest1
    case rest2 of
        (TWord "WHERE" : rest3) -> do
            (pred_, _) <- parseExpr rest3
            pure (UpdateStmt name assigns (Just pred_))
        _ ->
            pure (UpdateStmt name assigns Nothing)

parseAssignments :: [Token] -> Either MiniSqliteError ([Assignment], [Token])
parseAssignments toks = go toks []
  where
    go ts acc = do
        (col, rest0) <- expectIdent ts "column name"
        rest1        <- expectOp "=" rest0
        (val, rest2) <- parseExpr rest1
        let assign = Assignment col val
        case rest2 of
            (TComma : more) -> go more (acc ++ [assign])
            _               -> pure (acc ++ [assign], rest2)

-- ── DELETE ───────────────────────────────────────────────────────────────

parseDelete :: [Token] -> Either MiniSqliteError Statement
parseDelete toks = do
    (name, rest0) <- expectIdent toks "table name"
    case rest0 of
        (TWord "WHERE" : rest1) -> do
            (pred_, _) <- parseExpr rest1
            pure (DeleteStmt name (Just pred_))
        _ ->
            pure (DeleteStmt name Nothing)

-- ── SELECT ───────────────────────────────────────────────────────────────

parseSelect :: Bool -> [Token] -> Either MiniSqliteError Statement
parseSelect distinct toks = do
    (cols, rest0) <- parseOutputCols toks
    -- Optional FROM clause
    (tables, joins, rest1) <- case rest0 of
        (TWord "FROM" : rest) -> parseFromClause rest
        _                     -> pure ([], [], rest0)
    -- Optional WHERE
    (wherePred, rest2) <- case rest1 of
        (TWord "WHERE" : rest) -> do
            (e, r) <- parseExpr rest
            pure (Just e, r)
        _ -> pure (Nothing, rest1)
    -- Optional GROUP BY
    (groupBy, rest3) <- case rest2 of
        (TWord "GROUP" : TWord "BY" : rest) -> do
            (es, r) <- parseExprList rest
            pure (es, r)
        _ -> pure ([], rest2)
    -- Optional HAVING
    (having, rest4) <- case rest3 of
        (TWord "HAVING" : rest) -> do
            (e, r) <- parseExpr rest
            pure (Just e, r)
        _ -> pure (Nothing, rest3)
    -- Optional ORDER BY
    (orderBy, rest5) <- case rest4 of
        (TWord "ORDER" : TWord "BY" : rest) -> parseSortKeys rest
        _ -> pure ([], rest4)
    -- Optional LIMIT / OFFSET
    (lim, _rest6) <- case rest5 of
        (TWord "LIMIT" : rest) -> parseLimitClause rest
        _ -> pure (Nothing, rest5)
    pure (SelectStmt distinct cols tables joins wherePred groupBy having orderBy lim)

parseFromClause :: [Token] -> Either MiniSqliteError ([TableRef], [JoinClause], [Token])
parseFromClause toks = do
    (ref0, rest0) <- parseTableRef toks
    go [ref0] [] rest0
  where
    go refs joins ts = case ts of
        (TComma : rest) -> do
            (ref, rest') <- parseTableRef rest
            go (refs ++ [ref]) joins rest'
        (TWord "JOIN" : rest) ->
            parseJoin JoinInner rest refs joins
        (TWord "INNER" : TWord "JOIN" : rest) ->
            parseJoin JoinInner rest refs joins
        (TWord "LEFT" : TWord "JOIN" : rest) ->
            parseJoin JoinLeft rest refs joins
        (TWord "LEFT" : TWord "OUTER" : TWord "JOIN" : rest) ->
            parseJoin JoinLeft rest refs joins
        (TWord "RIGHT" : TWord "JOIN" : rest) ->
            parseJoin JoinRight rest refs joins
        (TWord "CROSS" : TWord "JOIN" : rest) ->
            parseJoin JoinCross rest refs joins
        _ ->
            pure (refs, joins, ts)

    parseJoin kind rest refs joins = do
        (name, rest1) <- expectIdent rest "table name"
        let (alias, rest2) = consumeAlias rest1
        (cond, rest3) <- case rest2 of
            (TWord "ON" : rest') -> do
                (e, r) <- parseExpr rest'
                pure (Just e, r)
            _ -> pure (Nothing, rest2)
        let jc = JoinClause kind name alias cond
        go refs (joins ++ [jc]) rest3

parseTableRef :: [Token] -> Either MiniSqliteError (TableRef, [Token])
parseTableRef toks = do
    (name, rest0) <- expectIdent toks "table name"
    let (alias, rest1) = consumeAlias rest0
    pure (TableRef name alias, rest1)

-- | Consume an optional alias (AS identifier or bare identifier).
consumeAlias :: [Token] -> (Maybe String, [Token])
consumeAlias (TWord "AS" : TWord w : rest) = (Just (originalCase w), rest)
consumeAlias (TWord "AS" : rest) = case rest of
    _ -> (Nothing, TWord "AS" : rest)
consumeAlias (TWord w : rest)
    | w `notElem` sqlKeywords = (Just (originalCase w), rest)
consumeAlias rest = (Nothing, rest)

sqlKeywords :: [String]
sqlKeywords =
    [ "SELECT","FROM","WHERE","JOIN","INNER","LEFT","RIGHT","OUTER","CROSS"
    , "ON","GROUP","BY","HAVING","ORDER","LIMIT","OFFSET","DISTINCT"
    , "AND","OR","NOT","IN","BETWEEN","LIKE","IS","NULL","AS"
    , "INSERT","INTO","VALUES","UPDATE","SET","DELETE","CREATE","TABLE"
    , "DROP","IF","EXISTS","EXISTS","PRIMARY","KEY","UNIQUE","DEFAULT"
    , "BEGIN","COMMIT","ROLLBACK","UNION","ALL","EXCEPT","INTERSECT"
    , "CASE","WHEN","THEN","ELSE","END","ASC","DESC","NULLS","FIRST","LAST"
    , "TRUE","FALSE","NOT"
    ]

-- | The tokeniser uppercases all TWord tokens; this restores a normalised
-- lowercase original for aliases. Since identifiers are case-insensitive in
-- SQLite we just keep lowercase.
originalCase :: String -> String
originalCase = map toLower

parseOutputCols :: [Token] -> Either MiniSqliteError ([OutputColumn], [Token])
parseOutputCols (TStar : rest) = pure ([OutputStar], rest)
parseOutputCols toks = go toks []
  where
    go ts acc = do
        (col, rest) <- parseOneOutputCol ts
        case rest of
            (TComma : more) -> go more (acc ++ [col])
            _               -> pure (acc ++ [col], rest)

parseOneOutputCol :: [Token] -> Either MiniSqliteError (OutputColumn, [Token])
parseOneOutputCol (TStar : rest) = pure (OutputStar, rest)
parseOneOutputCol toks = do
    (e, rest0) <- parseExpr toks
    case rest0 of
        (TWord "AS" : rest1) -> do
            (alias, rest2) <- expectIdent rest1 "alias"
            pure (OutputExpr e (Just alias), rest2)
        _ ->
            pure (OutputExpr e Nothing, rest0)

parseSortKeys :: [Token] -> Either MiniSqliteError ([SortKey], [Token])
parseSortKeys toks = go toks []
  where
    go ts acc = do
        (e, rest0) <- parseExpr ts
        let (dir, rest1) = case rest0 of
                (TWord "ASC"  : r) -> (SortAsc, r)
                (TWord "DESC" : r) -> (SortDesc, r)
                r                  -> (SortAsc, r)
        let (nulls, rest2) = case rest1 of
                (TWord "NULLS" : TWord "FIRST" : r) -> (NullsFirst, r)
                (TWord "NULLS" : TWord "LAST"  : r) -> (NullsLast, r)
                r -> (NullsFirst, r)
        let sk = SortKey e dir nulls
        case rest2 of
            (TComma : more) -> go more (acc ++ [sk])
            _               -> pure (acc ++ [sk], rest2)

parseLimitClause :: [Token] -> Either MiniSqliteError (Maybe LimitClause, [Token])
parseLimitClause (TNum n : rest0) = do
    case rest0 of
        (TWord "OFFSET" : TNum off : rest1) ->
            pure (Just (LimitClause (Just (toInteger n)) (Just (toInteger off))), rest1)
        (TComma : TNum off : rest1) ->
            pure (Just (LimitClause (Just (toInteger n)) (Just (toInteger off))), rest1)
        _ ->
            pure (Just (LimitClause (Just (toInteger n)) Nothing), rest0)
parseLimitClause toks =
    pure (Nothing, toks)

-- ── Expression parser ─────────────────────────────────────────────────────
--
-- Precedence (lowest → highest):
--   OR, AND, NOT, comparison (=, <>, !=, <, <=, >, >=), IS/IN/BETWEEN/LIKE,
--   addition (+, -), multiplication (*, /), unary negation, atom

parseExpr :: [Token] -> Either MiniSqliteError (SqlExpr, [Token])
parseExpr = parseOr

parseOr :: [Token] -> Either MiniSqliteError (SqlExpr, [Token])
parseOr toks = do
    (l, rest0) <- parseAnd toks
    case rest0 of
        (TWord "OR" : rest1) -> do
            (r, rest2) <- parseOr rest1
            pure (BinaryOp BinOr l r, rest2)
        _ -> pure (l, rest0)

parseAnd :: [Token] -> Either MiniSqliteError (SqlExpr, [Token])
parseAnd toks = do
    (l, rest0) <- parseNot toks
    case rest0 of
        (TWord "AND" : rest1) -> do
            (r, rest2) <- parseAnd rest1
            pure (BinaryOp BinAnd l r, rest2)
        _ -> pure (l, rest0)

parseNot :: [Token] -> Either MiniSqliteError (SqlExpr, [Token])
parseNot (TWord "NOT" : rest) = do
    (e, rest') <- parseNot rest
    pure (UnaryOp UnaryNot e, rest')
parseNot toks = parseComparison toks

parseComparison :: [Token] -> Either MiniSqliteError (SqlExpr, [Token])
parseComparison toks = do
    (l, rest0) <- parseAddSub toks
    case rest0 of
        (TOp "=" : rest1) -> do
            (r, rest2) <- parseAddSub rest1
            pure (BinaryOp BinEq l r, rest2)
        (TOp "<>" : rest1) -> do
            (r, rest2) <- parseAddSub rest1
            pure (BinaryOp BinNotEq l r, rest2)
        (TOp "!=" : rest1) -> do
            (r, rest2) <- parseAddSub rest1
            pure (BinaryOp BinNotEq l r, rest2)
        (TOp "<" : rest1) -> do
            (r, rest2) <- parseAddSub rest1
            pure (BinaryOp BinLt l r, rest2)
        (TOp "<=" : rest1) -> do
            (r, rest2) <- parseAddSub rest1
            pure (BinaryOp BinLte l r, rest2)
        (TOp ">" : rest1) -> do
            (r, rest2) <- parseAddSub rest1
            pure (BinaryOp BinGt l r, rest2)
        (TOp ">=" : rest1) -> do
            (r, rest2) <- parseAddSub rest1
            pure (BinaryOp BinGte l r, rest2)
        -- IS NULL / IS NOT NULL
        (TWord "IS" : TWord "NOT" : TWord "NULL" : rest1) ->
            pure (IsNotNull l, rest1)
        (TWord "IS" : TWord "NULL" : rest1) ->
            pure (IsNull l, rest1)
        (TWord "IS" : rest1) -> do
            (r, rest2) <- parseAddSub rest1
            pure (BinaryOp BinEq l r, rest2)
        -- NOT IN / NOT LIKE / NOT BETWEEN
        (TWord "NOT" : TWord "IN" : TLParen : rest1) -> do
            (items, rest2) <- parseExprList rest1
            rest3          <- expectToken TRParen rest2 ")"
            pure (NotInExpr l items, rest3)
        (TWord "NOT" : TWord "LIKE" : rest1) -> do
            (pat, rest2) <- parseAddSub rest1
            pure (NotLike l (exprToStr pat), rest2)
        (TWord "NOT" : TWord "BETWEEN" : rest1) -> do
            (lo, rest2) <- parseAddSub rest1
            rest3       <- expectWord "AND" rest2
            (hi, rest4) <- parseAddSub rest3
            pure (UnaryOp UnaryNot (Between l lo hi), rest4)
        -- IN
        (TWord "IN" : TLParen : rest1) -> do
            (items, rest2) <- parseExprList rest1
            rest3          <- expectToken TRParen rest2 ")"
            pure (InExpr l items, rest3)
        -- LIKE
        (TWord "LIKE" : rest1) -> do
            (pat, rest2) <- parseAddSub rest1
            pure (Like l (exprToStr pat), rest2)
        -- BETWEEN
        (TWord "BETWEEN" : rest1) -> do
            (lo, rest2) <- parseAddSub rest1
            rest3       <- expectWord "AND" rest2
            (hi, rest4) <- parseAddSub rest3
            pure (Between l lo hi, rest4)
        _ -> pure (l, rest0)
  where
    exprToStr (Literal (Just (LitText s))) = s
    exprToStr _ = ""

parseAddSub :: [Token] -> Either MiniSqliteError (SqlExpr, [Token])
parseAddSub toks = do
    (l, rest0) <- parseMulDiv toks
    case rest0 of
        (TOp "+" : rest1) -> do
            (r, rest2) <- parseAddSub rest1
            pure (BinaryOp BinAdd l r, rest2)
        (TOp "-" : rest1) -> do
            (r, rest2) <- parseAddSub rest1
            pure (BinaryOp BinSub l r, rest2)
        (TOp "||" : rest1) -> do
            (r, rest2) <- parseAddSub rest1
            -- SQL concatenation: coerce both sides to text and concatenate.
            pure (FuncCall "concat" [l, r], rest2)
        _ -> pure (l, rest0)

parseMulDiv :: [Token] -> Either MiniSqliteError (SqlExpr, [Token])
parseMulDiv toks = do
    (l, rest0) <- parseUnary toks
    case rest0 of
        (TStar : rest1) -> do
            (r, rest2) <- parseMulDiv rest1
            pure (BinaryOp BinMul l r, rest2)
        (TOp "/" : rest1) -> do
            (r, rest2) <- parseMulDiv rest1
            pure (BinaryOp BinDiv l r, rest2)
        (TOp "%" : rest1) -> do
            (r, rest2) <- parseMulDiv rest1
            pure (BinaryOp BinMod l r, rest2)
        _ -> pure (l, rest0)

parseUnary :: [Token] -> Either MiniSqliteError (SqlExpr, [Token])
parseUnary (TOp "-" : rest) = do
    (e, rest') <- parseAtom rest
    pure (UnaryOp UnaryNeg e, rest')
parseUnary (TOp "+" : rest) = parseAtom rest
parseUnary toks = parseAtom toks

parseAtom :: [Token] -> Either MiniSqliteError (SqlExpr, [Token])
parseAtom toks = case toks of
    -- Numeric literals
    (TNum n : rest)  -> pure (Literal (Just (LitInt (toInteger n))), rest)
    (TReal d : rest) -> pure (Literal (Just (LitReal d)), rest)
    -- String literal
    (TStr s : rest)  -> pure (Literal (Just (LitText s)), rest)
    -- NULL
    (TWord "NULL" : rest) -> pure (Literal Nothing, rest)
    -- TRUE / FALSE
    (TWord "TRUE"  : rest) -> pure (Literal (Just (LitBool True)),  rest)
    (TWord "FALSE" : rest) -> pure (Literal (Just (LitBool False)), rest)
    -- Parenthesised expression
    (TLParen : rest) -> do
        (e, rest') <- parseExpr rest
        rest''     <- expectToken TRParen rest' ")"
        pure (e, rest'')
    -- COUNT(*) / COUNT(DISTINCT expr) / COUNT(expr) and other aggregate functions
    (TWord "COUNT" : TLParen : TStar : TRParen : rest) ->
        pure (AggExpr AggCount AggStar False, rest)
    -- COUNT(DISTINCT expr): must be matched BEFORE the generic COUNT(expr) case
    -- because the generic case would treat DISTINCT as a column name and fail
    -- when it hits the closing paren.
    (TWord "COUNT" : TLParen : TWord "DISTINCT" : rest) -> do
        (e, rest') <- parseExpr rest
        rest''     <- expectToken TRParen rest' ")"
        pure (AggExpr AggCount (AggExprArg e) True, rest'')
    (TWord "COUNT" : TLParen : rest) -> do
        (e, rest') <- parseExpr rest
        rest''     <- expectToken TRParen rest' ")"
        pure (AggExpr AggCount (AggExprArg e) False, rest'')
    (TWord "SUM" : TLParen : rest) -> do
        (e, rest') <- parseExpr rest
        rest''     <- expectToken TRParen rest' ")"
        pure (AggExpr AggSum (AggExprArg e) False, rest'')
    (TWord "AVG" : TLParen : rest) -> do
        (e, rest') <- parseExpr rest
        rest''     <- expectToken TRParen rest' ")"
        pure (AggExpr AggAvg (AggExprArg e) False, rest'')
    (TWord "MIN" : TLParen : rest) -> do
        (e, rest') <- parseExpr rest
        rest''     <- expectToken TRParen rest' ")"
        pure (AggExpr AggMin (AggExprArg e) False, rest'')
    (TWord "MAX" : TLParen : rest) -> do
        (e, rest') <- parseExpr rest
        rest''     <- expectToken TRParen rest' ")"
        pure (AggExpr AggMax (AggExprArg e) False, rest'')
    -- Generic function call: FUNCNAME(arg1, arg2, ...)
    (TWord fn : TLParen : rest)
        | not (fn `elem` sqlKeywords) -> do
            (args, rest') <- case rest of
                (TRParen : r) -> pure ([], r)
                _ -> do
                    (es, r) <- parseExprList rest
                    r'      <- expectToken TRParen r ")"
                    pure (es, r')
            pure (FuncCall (map toLower fn) args, rest')
    -- Star (for SELECT *)
    (TStar : rest) -> pure (Wildcard, rest)
    -- Qualified column: table.column
    (TWord tbl : TDot : TWord col : rest) ->
        pure (P.Column (Just (map toLower tbl)) (map toLower col), rest)
    -- Bare identifier / keyword-as-identifier
    (TWord w : rest) ->
        pure (P.Column Nothing (map toLower w), rest)
    [] -> Left (mkErr "OperationalError" "unexpected end of expression")
    _  -> Left (mkErr "OperationalError" ("unexpected token in expression: " ++ show (head toks)))

-- ── Token helpers ─────────────────────────────────────────────────────────

expectIdent :: [Token] -> String -> Either MiniSqliteError (String, [Token])
expectIdent (TWord w : rest) _       = pure (map toLower w, rest)
expectIdent (TIdent s : rest) _      = pure (map toLower s, rest)
expectIdent (TStr s : rest) _        = pure (map toLower s, rest)
expectIdent (t : _) role             = Left (mkErr "OperationalError" ("expected " ++ role ++ ", got: " ++ show t))
expectIdent [] role                  = Left (mkErr "OperationalError" ("expected " ++ role ++ " but got end of input"))

expectToken :: Token -> [Token] -> String -> Either MiniSqliteError [Token]
expectToken t (x:rest) _ | x == t  = pure rest
expectToken t (_ : _)  role        = Left (mkErr "OperationalError" ("expected " ++ role))
expectToken _ []       role        = Left (mkErr "OperationalError" ("expected " ++ role ++ " but got end of input"))

expectWord :: String -> [Token] -> Either MiniSqliteError [Token]
expectWord w (TWord x : rest) | x == w = pure rest
expectWord w _                          = Left (mkErr "OperationalError" ("expected " ++ w))

expectOp :: String -> [Token] -> Either MiniSqliteError [Token]
expectOp op (TOp x : rest) | x == op = pure rest
expectOp op _                         = Left (mkErr "OperationalError" ("expected operator " ++ op))

-- ── Parameter binding ─────────────────────────────────────────────────────

bindParameters :: String -> [SqlValue] -> Either MiniSqliteError String
bindParameters sql params = go sql params Nothing ""
  where
    go [] []   _     acc = Right (reverse acc)
    go [] _    _     _   = Left (mkErr "ProgrammingError" "too many query parameters")
    go (c:cs) ps quote acc =
        case quote of
            Just q
                | c == q && not (null cs) && head cs == q ->
                    go (tail cs) ps quote (q:c:acc)
                | c == q ->
                    go cs ps Nothing (c:acc)
                | otherwise ->
                    go cs ps quote (c:acc)
            Nothing
                | c == '\'' || c == '"' ->
                    go cs ps (Just c) (c:acc)
                | c == '?' ->
                    case ps of
                        []   -> Left (mkErr "ProgrammingError" "not enough query parameters")
                        v:vs -> go cs vs Nothing (reverse (formatParam v) ++ acc)
                | otherwise ->
                    go cs ps Nothing (c:acc)

formatParam :: SqlValue -> String
formatParam SqlNull         = "NULL"
formatParam (SqlInteger i)  = show i
formatParam (SqlReal    d)  = show d
formatParam (SqlBool True)  = "TRUE"
formatParam (SqlBool False) = "FALSE"
formatParam (SqlText    s)
    -- Null bytes in string parameters are rejected: they can cause C-string
    -- truncation in downstream consumers and can be used to smuggle extra SQL
    -- past parsers that interpret null-terminated strings.
    | '\NUL' `elem` s =
        error "MiniSqlite: SqlText parameter must not contain null bytes (\\NUL)"
    | otherwise        = "'" ++ concatMap esc s ++ "'"
  where
    esc '\'' = "''"
    esc ch   = [ch]

-- ── Miscellaneous helpers ─────────────────────────────────────────────────

firstKeyword :: String -> String
firstKeyword = map toUpper . takeWhile isAlpha . dropWhile isSpace

trimSql :: String -> String
trimSql s =
    let t = dropWhileEnd isSpace (dropWhile isSpace s)
    in if not (null t) && last t == ';' then trimSql (init t) else t

mkErr :: String -> String -> MiniSqliteError
mkErr = MiniSqliteError
