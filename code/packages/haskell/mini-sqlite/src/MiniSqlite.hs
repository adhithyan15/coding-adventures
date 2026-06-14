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

import Data.Char (isAlpha, isAlphaNum, isDigit, isSpace, toLower, toUpper)
import Data.IORef
import Data.List (dropWhileEnd, isPrefixOf, sortBy)
import qualified Data.Map.Strict as Map
import Text.Read (readMaybe)

data SqlValue
    = SqlNull
    | SqlInteger Integer
    | SqlReal Double
    | SqlText String
    | SqlBool Bool
    deriving (Eq, Show)

data MiniSqliteError = MiniSqliteError
    { errorKind :: String
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

data Connection = Connection
    { connDb :: IORef Database
    , connSnapshot :: IORef (Maybe Database)
    , connAutocommit :: Bool
    , connClosed :: IORef Bool
    }

data Cursor = Cursor
    { curRows :: IORef [[SqlValue]]
    , curOffset :: IORef Int
    , curDescription :: IORef [Column]
    , curRowCount :: IORef Int
    , curLastRowId :: IORef (Maybe SqlValue)
    , curClosed :: IORef Bool
    }

apiLevel :: String
apiLevel = "2.0"

threadSafety :: Int
threadSafety = 1

paramStyle :: String
paramStyle = "qmark"

connect :: String -> IO (Either MiniSqliteError Connection)
connect database = connectWith defaultConnectionOptions database

connectWith :: ConnectionOptions -> String -> IO (Either MiniSqliteError Connection)
connectWith options database
    | database /= ":memory:" =
        pure (Left (err "NotSupportedError" "Haskell mini-sqlite supports only :memory: in Level 0"))
    | otherwise = do
        dbRef <- newIORef emptyDatabase
        snapshotRef <- newIORef Nothing
        closedRef <- newIORef False
        pure (Right (Connection dbRef snapshotRef (autocommit options) closedRef))

execute :: Connection -> String -> [SqlValue] -> IO (Either MiniSqliteError Cursor)
execute connection sql parameters = do
    result <- executeBound connection sql parameters
    case result of
        Left failure -> pure (Left failure)
        Right executionResult -> do
            cursor <- newCursor
            writeIORef (curRows cursor) (resultRows executionResult)
            writeIORef (curDescription cursor) (map Column (resultColumns executionResult))
            writeIORef (curRowCount cursor) (resultRowCount executionResult)
            writeIORef (curLastRowId cursor) (resultLastRowId executionResult)
            pure (Right cursor)

executeMany :: Connection -> String -> [[SqlValue]] -> IO (Either MiniSqliteError Cursor)
executeMany connection sql parameterSets =
    case parameterSets of
        [] -> execute connection sql []
        _ -> go parameterSets
  where
    go [] = execute connection sql []
    go [parameters] = execute connection sql parameters
    go (parameters:rest) = do
        result <- execute connection sql parameters
        case result of
            Left failure -> pure (Left failure)
            Right _ -> go rest

commit :: Connection -> IO (Either MiniSqliteError ())
commit connection = do
    open <- assertConnectionOpen connection
    case open of
        Left failure -> pure (Left failure)
        Right () -> do
            writeIORef (connSnapshot connection) Nothing
            pure (Right ())

rollback :: Connection -> IO (Either MiniSqliteError ())
rollback connection = do
    open <- assertConnectionOpen connection
    case open of
        Left failure -> pure (Left failure)
        Right () -> do
            snapshot <- readIORef (connSnapshot connection)
            case snapshot of
                Nothing -> pure ()
                Just original -> do
                    writeIORef (connDb connection) original
                    writeIORef (connSnapshot connection) Nothing
            pure (Right ())

close :: Connection -> IO ()
close connection = do
    alreadyClosed <- readIORef (connClosed connection)
    if alreadyClosed
        then pure ()
        else do
            snapshot <- readIORef (connSnapshot connection)
            case snapshot of
                Nothing -> pure ()
                Just original -> writeIORef (connDb connection) original
            writeIORef (connSnapshot connection) Nothing
            writeIORef (connClosed connection) True

cursorDescription :: Cursor -> IO [Column]
cursorDescription = readIORef . curDescription

cursorRowCount :: Cursor -> IO Int
cursorRowCount = readIORef . curRowCount

cursorLastRowId :: Cursor -> IO (Maybe SqlValue)
cursorLastRowId = readIORef . curLastRowId

fetchOne :: Cursor -> IO (Either MiniSqliteError (Maybe [SqlValue]))
fetchOne cursor = do
    open <- assertCursorOpen cursor
    case open of
        Left failure -> pure (Left failure)
        Right () -> do
            rows <- readIORef (curRows cursor)
            offset <- readIORef (curOffset cursor)
            if offset >= length rows
                then pure (Right Nothing)
                else do
                    writeIORef (curOffset cursor) (offset + 1)
                    pure (Right (Just (rows !! offset)))

fetchMany :: Cursor -> Int -> IO (Either MiniSqliteError [[SqlValue]])
fetchMany cursor size = do
    open <- assertCursorOpen cursor
    case open of
        Left failure -> pure (Left failure)
        Right () -> do
            rows <- readIORef (curRows cursor)
            offset <- readIORef (curOffset cursor)
            let limit = max 0 size
                batch = take limit (drop offset rows)
            writeIORef (curOffset cursor) (offset + length batch)
            pure (Right batch)

fetchAll :: Cursor -> IO (Either MiniSqliteError [[SqlValue]])
fetchAll cursor = do
    open <- assertCursorOpen cursor
    case open of
        Left failure -> pure (Left failure)
        Right () -> do
            rows <- readIORef (curRows cursor)
            offset <- readIORef (curOffset cursor)
            let remaining = drop offset rows
            writeIORef (curOffset cursor) (length rows)
            pure (Right remaining)

closeCursor :: Cursor -> IO ()
closeCursor cursor = writeIORef (curClosed cursor) True

newCursor :: IO Cursor
newCursor = do
    rowsRef <- newIORef []
    offsetRef <- newIORef 0
    descriptionRef <- newIORef []
    rowCountRef <- newIORef (-1)
    lastRowIdRef <- newIORef Nothing
    closedRef <- newIORef False
    pure (Cursor rowsRef offsetRef descriptionRef rowCountRef lastRowIdRef closedRef)

executeBound :: Connection -> String -> [SqlValue] -> IO (Either MiniSqliteError ExecutionResult)
executeBound connection sql parameters = do
    open <- assertConnectionOpen connection
    case open of
        Left failure -> pure (Left failure)
        Right () ->
            case bindParameters sql parameters of
                Left failure -> pure (Left failure)
                Right bound -> dispatch (firstKeyword bound) bound
  where
    dispatch keyword bound =
        case keyword of
            "BEGIN" -> do
                ensureSnapshot connection
                pure (Right (emptyResult 0))
            "COMMIT" -> do
                writeIORef (connSnapshot connection) Nothing
                pure (Right (emptyResult 0))
            "ROLLBACK" -> do
                _ <- rollback connection
                pure (Right (emptyResult 0))
            "SELECT" -> readOnly (\db -> selectRows (parseSelect bound) db)
            "CREATE" -> mutate (\db -> createTable (parseCreate bound) db)
            "DROP" -> mutate (\db -> dropTable (parseDrop bound) db)
            "INSERT" -> mutate (\db -> insertRow (parseInsert bound) db)
            "UPDATE" -> mutate (\db -> updateRows (parseUpdate bound) db)
            "DELETE" -> mutate (\db -> deleteRows (parseDelete bound) db)
            _ -> pure (Left (err "OperationalError" "unsupported SQL statement"))

    readOnly action = do
        db <- readIORef (connDb connection)
        pure (action db)

    mutate action = do
        ensureSnapshot connection
        db <- readIORef (connDb connection)
        case action db of
            Left failure -> pure (Left failure)
            Right (nextDb, result) -> do
                writeIORef (connDb connection) nextDb
                pure (Right result)

assertConnectionOpen :: Connection -> IO (Either MiniSqliteError ())
assertConnectionOpen connection = do
    closed <- readIORef (connClosed connection)
    pure (if closed then Left (err "ProgrammingError" "connection is closed") else Right ())

assertCursorOpen :: Cursor -> IO (Either MiniSqliteError ())
assertCursorOpen cursor = do
    closed <- readIORef (curClosed cursor)
    pure (if closed then Left (err "ProgrammingError" "cursor is closed") else Right ())

ensureSnapshot :: Connection -> IO ()
ensureSnapshot connection =
    if connAutocommit connection
        then pure ()
        else do
            snapshot <- readIORef (connSnapshot connection)
            case snapshot of
                Just _ -> pure ()
                Nothing -> do
                    db <- readIORef (connDb connection)
                    writeIORef (connSnapshot connection) (Just db)

data ExecutionResult = ExecutionResult
    { resultColumns :: [String]
    , resultRows :: [[SqlValue]]
    , resultRowCount :: Int
    , resultLastRowId :: Maybe SqlValue
    } deriving (Eq, Show)

emptyResult :: Int -> ExecutionResult
emptyResult rowCount = ExecutionResult [] [] rowCount Nothing

data Database = Database (Map.Map String Table)
    deriving (Eq, Show)

data Table = Table
    { tableColumns :: [String]
    , tableRows :: [Row]
    , tableNextRowId :: Integer
    } deriving (Eq, Show)

type Row = Map.Map String SqlValue

emptyDatabase :: Database
emptyDatabase = Database Map.empty

createTable :: Either MiniSqliteError CreateStatement -> Database -> Either MiniSqliteError (Database, ExecutionResult)
createTable parsed (Database tables) = do
    statement <- parsed
    let key = identifierKey (createTableName statement)
    if Map.member key tables
        then Left (err "OperationalError" ("table already exists: " ++ createTableName statement))
        else
            let table = Table (createColumns statement) [] 1
            in Right (Database (Map.insert key table tables), emptyResult 0)

dropTable :: Either MiniSqliteError String -> Database -> Either MiniSqliteError (Database, ExecutionResult)
dropTable parsed (Database tables) = do
    tableName <- parsed
    let key = identifierKey tableName
    if Map.member key tables
        then Right (Database (Map.delete key tables), emptyResult 0)
        else Left (err "OperationalError" ("no such table: " ++ tableName))

insertRow :: Either MiniSqliteError InsertStatement -> Database -> Either MiniSqliteError (Database, ExecutionResult)
insertRow parsed database@(Database tables) = do
    statement <- parsed
    table <- requireTable (insertTableName statement) database
    let columns = maybe (tableColumns table) id (insertColumns statement)
    if length columns /= length (insertValues statement)
        then Left (err "ProgrammingError" "column/value count mismatch")
        else do
            values <- mapM parseLiteral (insertValues statement)
            let baseRow = Map.fromList [(identifierKey column, SqlNull) | column <- tableColumns table]
                row = foldl (\acc (column, value) -> Map.insert (identifierKey column) value acc) baseRow (zip columns values)
                rowId = tableNextRowId table
                nextTable = table { tableRows = tableRows table ++ [row], tableNextRowId = rowId + 1 }
                nextDb = Database (Map.insert (identifierKey (insertTableName statement)) nextTable tables)
            Right (nextDb, (emptyResult 1) { resultLastRowId = Just (SqlInteger rowId) })

updateRows :: Either MiniSqliteError UpdateStatement -> Database -> Either MiniSqliteError (Database, ExecutionResult)
updateRows parsed database@(Database tables) = do
    statement <- parsed
    table <- requireTable (updateTableName statement) database
    assignments <- mapM parseAssignmentValue (updateAssignments statement)
    let apply row =
            if matchesWhere (updateWhere statement) row
                then (True, foldl (\acc (column, value) -> Map.insert (identifierKey column) value acc) row assignments)
                else (False, row)
        results = map apply (tableRows table)
        count = length (filter fst results)
        nextRows = map snd results
        nextTable = table { tableRows = nextRows }
        nextDb = Database (Map.insert (identifierKey (updateTableName statement)) nextTable tables)
    Right (nextDb, emptyResult count)
  where
    parseAssignmentValue assignment = do
        value <- parseLiteral (assignmentValue assignment)
        Right (assignmentColumn assignment, value)

deleteRows :: Either MiniSqliteError DeleteStatement -> Database -> Either MiniSqliteError (Database, ExecutionResult)
deleteRows parsed database@(Database tables) = do
    statement <- parsed
    table <- requireTable (deleteTableName statement) database
    let shouldKeep row = not (matchesWhere (deleteWhere statement) row)
        nextRows = filter shouldKeep (tableRows table)
        count = length (tableRows table) - length nextRows
        nextTable = table { tableRows = nextRows }
        nextDb = Database (Map.insert (identifierKey (deleteTableName statement)) nextTable tables)
    Right (nextDb, emptyResult count)

selectRows :: Either MiniSqliteError SelectStatement -> Database -> Either MiniSqliteError ExecutionResult
selectRows parsed database = do
    statement <- parsed
    table <- requireTable (selectTableName statement) database
    let projection =
            case selectProjection statement of
                ["*"] -> tableColumns table
                columns -> columns
        rows = filter (matchesWhere (selectWhere statement)) (tableRows table)
        orderedRows = applyOrder (selectOrderBy statement) rows
        projected = [[Map.findWithDefault SqlNull (identifierKey column) row | column <- projection] | row <- orderedRows]
    Right (ExecutionResult projection projected (-1) Nothing)

requireTable :: String -> Database -> Either MiniSqliteError Table
requireTable tableName (Database tables) =
    case Map.lookup (identifierKey tableName) tables of
        Just table -> Right table
        Nothing -> Left (err "OperationalError" ("no such table: " ++ tableName))

data CreateStatement = CreateStatement
    { createTableName :: String
    , createColumns :: [String]
    } deriving (Eq, Show)

data InsertStatement = InsertStatement
    { insertTableName :: String
    , insertColumns :: Maybe [String]
    , insertValues :: [String]
    } deriving (Eq, Show)

data Assignment = Assignment
    { assignmentColumn :: String
    , assignmentValue :: String
    } deriving (Eq, Show)

data UpdateStatement = UpdateStatement
    { updateTableName :: String
    , updateAssignments :: [Assignment]
    , updateWhere :: Maybe String
    } deriving (Eq, Show)

data DeleteStatement = DeleteStatement
    { deleteTableName :: String
    , deleteWhere :: Maybe String
    } deriving (Eq, Show)

data SelectStatement = SelectStatement
    { selectTableName :: String
    , selectProjection :: [String]
    , selectWhere :: Maybe String
    , selectOrderBy :: Maybe String
    } deriving (Eq, Show)

parseCreate :: String -> Either MiniSqliteError CreateStatement
parseCreate sql = do
    rest <- stripKeyword "CREATE TABLE" (trimSql sql)
    let (name, afterName) = takeIdentifier rest
    inside <- parenthesized afterName
    let columns = map identifierFromColumn (splitTopLevel ',' inside)
    if null name || null columns
        then Left (err "OperationalError" "could not parse CREATE TABLE")
        else Right (CreateStatement name columns)

parseDrop :: String -> Either MiniSqliteError String
parseDrop sql = do
    rest <- stripKeyword "DROP TABLE" (trimSql sql)
    let (name, remainder) = takeIdentifier rest
    if null name || not (null (trim remainder))
        then Left (err "OperationalError" "could not parse DROP TABLE")
        else Right name

parseInsert :: String -> Either MiniSqliteError InsertStatement
parseInsert sql = do
    rest <- stripKeyword "INSERT INTO" (trimSql sql)
    let (name, afterName) = takeIdentifier rest
        trimmedAfterName = trim afterName
        (columns, beforeValues) =
            if not (null trimmedAfterName) && head trimmedAfterName == '('
                then case takeParenthesized trimmedAfterName of
                    Just (inside, remaining) -> (Just (map identifierFromColumn (splitTopLevel ',' inside)), trim remaining)
                    Nothing -> (Nothing, trimmedAfterName)
                else (Nothing, trimmedAfterName)
    valuesRest <- stripKeyword "VALUES" beforeValues
    inside <- parenthesized valuesRest
    if null name
        then Left (err "OperationalError" "could not parse INSERT")
        else Right (InsertStatement name columns (splitTopLevel ',' inside))

parseUpdate :: String -> Either MiniSqliteError UpdateStatement
parseUpdate sql = do
    rest <- stripKeyword "UPDATE" (trimSql sql)
    let (name, afterName) = takeIdentifier rest
    setRest <- stripKeyword "SET" afterName
    let (assignmentSql, whereSql) = splitOptionalKeyword "WHERE" setRest
    assignments <- mapM parseAssignment (splitTopLevel ',' assignmentSql)
    if null name || null assignments
        then Left (err "OperationalError" "could not parse UPDATE")
        else Right (UpdateStatement name assignments whereSql)

parseDelete :: String -> Either MiniSqliteError DeleteStatement
parseDelete sql = do
    rest <- stripKeyword "DELETE FROM" (trimSql sql)
    let (name, afterName) = takeIdentifier rest
        whereSql = snd (splitOptionalKeyword "WHERE" afterName)
    if null name
        then Left (err "OperationalError" "could not parse DELETE")
        else Right (DeleteStatement name whereSql)

parseSelect :: String -> Either MiniSqliteError SelectStatement
parseSelect sql = do
    rest <- stripKeyword "SELECT" (trimSql sql)
    (projectionSql, fromRest) <- splitRequiredKeyword "FROM" rest
    let (name, suffix) = takeIdentifier fromRest
        (beforeOrder, orderSql) = splitOptionalKeyword "ORDER BY" suffix
        (_, whereSql) = splitOptionalKeyword "WHERE" beforeOrder
        projection = map identifierFromColumn (splitTopLevel ',' projectionSql)
    if null name || null projection
        then Left (err "OperationalError" "could not parse SELECT")
        else Right (SelectStatement name projection whereSql orderSql)

parseAssignment :: String -> Either MiniSqliteError Assignment
parseAssignment sql =
    case break (== '=') sql of
        (column, '=':valueSql) -> Right (Assignment (identifierFromColumn column) (trim valueSql))
        _ -> Left (err "OperationalError" "invalid assignment")

matchesWhere :: Maybe String -> Row -> Bool
matchesWhere Nothing _ = True
matchesWhere (Just sql) row
    | null (trim sql) = True
    | otherwise = any matchesDisjunct (splitByKeyword "OR" sql)
  where
    matchesDisjunct disjunct = all (`matchesAtom` row) (splitByKeyword "AND" disjunct)

matchesAtom :: String -> Row -> Bool
matchesAtom atom row =
    case splitOptionalKeyword "IS" text of
        (left, Just rest) ->
            let parts = words rest
                value = resolveValue row left
            in case map upper parts of
                ["NULL"] -> value == SqlNull
                ["NOT", "NULL"] -> value /= SqlNull
                _ -> comparisonFallback
        _ -> comparisonFallback
  where
    text = trim atom
    comparisonFallback =
        case parseInExpression text of
            Just (leftSql, valueSqls) ->
                let left = resolveValue row leftSql
                in any (\valueSql -> valuesEqual left (resolveValue row valueSql)) valueSqls
            Nothing ->
                case parseComparison text of
                    Just (leftSql, operator, rightSql) ->
                        let left = resolveValue row leftSql
                            right = resolveValue row rightSql
                        in compareByOperator operator left right
                    Nothing ->
                        case resolveValue row text of
                            SqlBool flag -> flag
                            SqlNull -> False
                            _ -> True

parseInExpression :: String -> Maybe (String, [String])
parseInExpression sql =
    case splitOptionalKeyword "IN" sql of
        (left, Just rest) ->
            case takeParenthesized (trim rest) of
                Just (inside, remainder) | null (trim remainder) -> Just (left, splitTopLevel ',' inside)
                _ -> Nothing
        _ -> Nothing

parseComparison :: String -> Maybe (String, String, String)
parseComparison sql =
    firstMatch ["!=", "<>", "<=", ">=", "=", "<", ">"]
  where
    firstMatch [] = Nothing
    firstMatch (operator:operators) =
        case splitOperator operator sql of
            Just (left, right) -> Just (left, operator, right)
            Nothing -> firstMatch operators

splitOperator :: String -> String -> Maybe (String, String)
splitOperator operator sql = go "" sql
  where
    go _ [] = Nothing
    go prefix rest
        | operator `isPrefixOf` rest = Just (trim (reverse prefix), trim (drop (length operator) rest))
        | otherwise = go (head rest : prefix) (tail rest)

compareByOperator :: String -> SqlValue -> SqlValue -> Bool
compareByOperator operator left right =
    case operator of
        "=" -> valuesEqual left right
        "!=" -> not (valuesEqual left right)
        "<>" -> not (valuesEqual left right)
        "<" -> compareValues left right == LT
        "<=" -> compareValues left right /= GT
        ">" -> compareValues left right == GT
        ">=" -> compareValues left right /= LT
        _ -> False

applyOrder :: Maybe String -> [Row] -> [Row]
applyOrder Nothing rows = rows
applyOrder (Just orderSql) rows =
    let orderParts = words orderSql
        column = if null orderParts then "" else head orderParts
        descending = length orderParts > 1 && upper (orderParts !! 1) == "DESC"
        sorted = sortBy (\left right -> compareValues (resolveValue left column) (resolveValue right column)) rows
    in if descending then reverse sorted else sorted

bindParameters :: String -> [SqlValue] -> Either MiniSqliteError String
bindParameters sql parameters = go sql parameters Nothing ""
  where
    go [] [] _ acc = Right (reverse acc)
    go [] _ _ _ = Left (err "ProgrammingError" "too many query parameters")
    go (ch:rest) params quote acc =
        case quote of
            Just quoteChar
                | ch == quoteChar && not (null rest) && head rest == quoteChar ->
                    go (tail rest) params quote (quoteChar : ch : acc)
                | ch == quoteChar -> go rest params Nothing (ch : acc)
                | otherwise -> go rest params quote (ch : acc)
            Nothing
                | ch == '\'' || ch == '"' -> go rest params (Just ch) (ch : acc)
                | ch == '?' ->
                    case params of
                        [] -> Left (err "ProgrammingError" "not enough query parameters")
                        value:remaining -> go rest remaining Nothing (reverse (formatParameter value) ++ acc)
                | otherwise -> go rest params Nothing (ch : acc)

formatParameter :: SqlValue -> String
formatParameter SqlNull = "NULL"
formatParameter (SqlInteger value) = show value
formatParameter (SqlReal value) = show value
formatParameter (SqlBool True) = "TRUE"
formatParameter (SqlBool False) = "FALSE"
formatParameter (SqlText value) = "'" ++ concatMap escape value ++ "'"
  where
    escape '\'' = "''"
    escape ch = [ch]

resolveValue :: Row -> String -> SqlValue
resolveValue row token =
    Map.findWithDefault (either (const (SqlText text)) id (parseLiteral text)) (identifierKey text) row
  where
    text = trim token

parseLiteral :: String -> Either MiniSqliteError SqlValue
parseLiteral token
    | quoted '\'' text = Right (SqlText (unquote '\'' text))
    | quoted '"' text = Right (SqlText (unquote '"' text))
    | upper text == "NULL" = Right SqlNull
    | upper text == "TRUE" = Right (SqlBool True)
    | upper text == "FALSE" = Right (SqlBool False)
    | otherwise =
        case readMaybe text :: Maybe Integer of
            Just integer -> Right (SqlInteger integer)
            Nothing ->
                case readMaybe text :: Maybe Double of
                    Just real -> Right (SqlReal real)
                    Nothing -> Right (SqlText text)
  where
    text = trim token

valuesEqual :: SqlValue -> SqlValue -> Bool
valuesEqual SqlNull SqlNull = True
valuesEqual (SqlInteger left) (SqlInteger right) = left == right
valuesEqual (SqlInteger left) (SqlReal right) = fromInteger left == right
valuesEqual (SqlReal left) (SqlInteger right) = left == fromInteger right
valuesEqual (SqlReal left) (SqlReal right) = left == right
valuesEqual left right = left == right

compareValues :: SqlValue -> SqlValue -> Ordering
compareValues left right
    | valuesEqual left right = EQ
compareValues SqlNull _ = LT
compareValues _ SqlNull = GT
compareValues (SqlInteger left) (SqlInteger right) = compare left right
compareValues (SqlInteger left) (SqlReal right) = compare (fromInteger left :: Double) right
compareValues (SqlReal left) (SqlInteger right) = compare left (fromInteger right :: Double)
compareValues (SqlReal left) (SqlReal right) = compare left right
compareValues left right = compare (show left) (show right)

stripKeyword :: String -> String -> Either MiniSqliteError String
stripKeyword keyword sql =
    let trimmed = trim sql
        prefix = keyword
    in if upper prefix `isPrefixOf` upper trimmed
        then Right (trim (drop (length prefix) trimmed))
        else Left (err "OperationalError" ("expected " ++ keyword))

splitRequiredKeyword :: String -> String -> Either MiniSqliteError (String, String)
splitRequiredKeyword keyword sql =
    case findKeyword keyword sql of
        Nothing -> Left (err "OperationalError" ("expected " ++ keyword))
        Just index -> Right (trim (take index sql), trim (drop (index + length keyword) sql))

splitOptionalKeyword :: String -> String -> (String, Maybe String)
splitOptionalKeyword keyword sql =
    case findKeyword keyword sql of
        Nothing -> (trim sql, Nothing)
        Just index -> (trim (take index sql), Just (trim (drop (index + length keyword) sql)))

findKeyword :: String -> String -> Maybe Int
findKeyword keyword sql = go 0 sql
  where
    go _ [] = Nothing
    go index rest
        | keywordAt keyword index sql = Just index
        | otherwise = go (index + 1) (tail rest)

keywordAt :: String -> Int -> String -> Bool
keywordAt keyword index sql =
    let candidate = take (length keyword) (drop index sql)
        before = if index == 0 then ' ' else sql !! (index - 1)
        afterIndex = index + length keyword
        after = if afterIndex >= length sql then ' ' else sql !! afterIndex
    in upper candidate == upper keyword && not (identifierChar before) && not (identifierChar after)

takeIdentifier :: String -> (String, String)
takeIdentifier sql =
    let trimmed = trim sql
        (name, rest) = span identifierChar trimmed
    in (name, rest)

identifierFromColumn :: String -> String
identifierFromColumn = takeWhile (\ch -> not (isSpace ch)) . trimQuotes . trim

identifierKey :: String -> String
identifierKey = map toLower . trimQuotes . trim

identifierChar :: Char -> Bool
identifierChar ch = isAlphaNum ch || ch == '_'

parenthesized :: String -> Either MiniSqliteError String
parenthesized sql =
    case takeParenthesized (trim sql) of
        Just (inside, remainder) | null (trim remainder) -> Right inside
        _ -> Left (err "OperationalError" "expected parenthesized expression")

takeParenthesized :: String -> Maybe (String, String)
takeParenthesized sql
    | null trimmed || head trimmed /= '(' = Nothing
    | otherwise = go 0 Nothing "" trimmed
  where
    trimmed = trim sql
    go _ _ _ [] = Nothing
    go depth quote acc (ch:rest) =
        case quote of
            Just quoteChar
                | ch == quoteChar && not (null rest) && head rest == quoteChar ->
                    go depth quote (head rest : ch : acc) (tail rest)
                | ch == quoteChar -> go depth Nothing (ch : acc) rest
                | otherwise -> go depth quote (ch : acc) rest
            Nothing
                | ch == '\'' || ch == '"' -> go depth (Just ch) (ch : acc) rest
                | ch == '(' -> go (depth + 1) Nothing (if depth == 0 then acc else ch : acc) rest
                | ch == ')' && depth == 1 -> Just (reverse acc, rest)
                | ch == ')' -> go (depth - 1) Nothing (ch : acc) rest
                | otherwise -> go depth Nothing (ch : acc) rest

splitTopLevel :: Char -> String -> [String]
splitTopLevel separator sql = filter (not . null) (map trim (go 0 Nothing "" sql))
  where
    go _ _ acc [] = [reverse acc]
    go depth quote acc (ch:rest) =
        case quote of
            Just quoteChar
                | ch == quoteChar && not (null rest) && head rest == quoteChar ->
                    go depth quote (head rest : ch : acc) (tail rest)
                | ch == quoteChar -> go depth Nothing (ch : acc) rest
                | otherwise -> go depth quote (ch : acc) rest
            Nothing
                | ch == '\'' || ch == '"' -> go depth (Just ch) (ch : acc) rest
                | ch == '(' -> go (depth + 1) Nothing (ch : acc) rest
                | ch == ')' -> go (max 0 (depth - 1)) Nothing (ch : acc) rest
                | ch == separator && depth == 0 -> reverse acc : go depth Nothing "" rest
                | otherwise -> go depth Nothing (ch : acc) rest

splitByKeyword :: String -> String -> [String]
splitByKeyword keyword sql = filter (not . null) (map trim (go 0 Nothing "" sql))
  where
    go _ _ acc [] = [reverse acc]
    go depth quote acc text@(ch:rest) =
        case quote of
            Just quoteChar
                | ch == quoteChar -> go depth Nothing (ch : acc) rest
                | otherwise -> go depth quote (ch : acc) rest
            Nothing
                | ch == '\'' || ch == '"' -> go depth (Just ch) (ch : acc) rest
                | ch == '(' -> go (depth + 1) Nothing (ch : acc) rest
                | ch == ')' -> go (max 0 (depth - 1)) Nothing (ch : acc) rest
                | depth == 0 && keywordAt keyword 0 text -> reverse acc : go depth Nothing "" (drop (length keyword) text)
                | otherwise -> go depth Nothing (ch : acc) rest

trimSql :: String -> String
trimSql sql =
    let trimmed = trim sql
    in if not (null trimmed) && last trimmed == ';' then trim (init trimmed) else trimmed

firstKeyword :: String -> String
firstKeyword = upper . takeWhile isAlpha . trim

trim :: String -> String
trim = dropWhileEnd isSpace . dropWhile isSpace

trimQuotes :: String -> String
trimQuotes text
    | quoted '"' text = init (tail text)
    | quoted '\'' text = init (tail text)
    | otherwise = text

quoted :: Char -> String -> Bool
quoted quote text = length text >= 2 && head text == quote && last text == quote

unquote :: Char -> String -> String
unquote quote = go . init . tail
  where
    go [] = []
    go (ch:next:rest)
        | ch == quote && next == quote = quote : go rest
    go (ch:rest) = ch : go rest

upper :: String -> String
upper = map toUpper

err :: String -> String -> MiniSqliteError
err = MiniSqliteError
