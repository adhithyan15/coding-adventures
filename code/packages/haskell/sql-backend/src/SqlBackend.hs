module SqlBackend
    ( SqlValue(..)
    , isSqlValue
    , sqlValueTypeName
    , compareSqlValues
    , Row
    , BackendError(..)
    , ColumnDef(..)
    , defaultColumnDef
    , effectiveNotNull
    , effectiveUnique
    , IndexDef(..)
    , TriggerDef(..)
    , triggerDef
    , ListRowIterator
    , listRowIterator
    , iteratorNext
    , iteratorClose
    , iteratorToList
    , ListCursor
    , cursorNext
    , cursorCurrentRow
    , InMemoryBackend(..)
    , newBackend
    , backendAsSchemaProvider
    , schemaProviderColumns
    , schemaProviderListIndexes
    , tables
    , columns
    , scan
    , openCursor
    , insert
    , update
    , delete
    , createTable
    , dropTable
    , addColumn
    , createIndex
    , dropIndex
    , listIndexes
    , scanIndex
    , scanByRowIds
    , beginTransaction
    , commit
    , rollback
    , createSavepoint
    , releaseSavepoint
    , rollbackToSavepoint
    , createTrigger
    , dropTrigger
    , listTriggers
    ) where

import Data.Char (toLower, toUpper)
import Data.List (find, sort, sortBy)
import qualified Data.Map.Strict as Map
import Data.Maybe (mapMaybe)
import Data.Word (Word8)

data SqlValue
    = SqlNull
    | SqlBool Bool
    | SqlInteger Integer
    | SqlReal Double
    | SqlText String
    | SqlBlob [Word8]
    deriving (Eq, Show)

isSqlValue :: SqlValue -> Bool
isSqlValue _ = True

sqlValueTypeName :: SqlValue -> String
sqlValueTypeName value =
    case value of
        SqlNull -> "NULL"
        SqlBool _ -> "BOOLEAN"
        SqlInteger _ -> "INTEGER"
        SqlReal _ -> "REAL"
        SqlText _ -> "TEXT"
        SqlBlob _ -> "BLOB"

compareSqlValues :: SqlValue -> SqlValue -> Ordering
compareSqlValues left right =
    case compare (rank left) (rank right) of
        EQ -> compareSameRank left right
        other -> other
  where
    rank SqlNull = (0 :: Int)
    rank (SqlBool _) = 1
    rank (SqlInteger _) = 2
    rank (SqlReal _) = 2
    rank (SqlText _) = 3
    rank (SqlBlob _) = 4

    compareSameRank SqlNull SqlNull = EQ
    compareSameRank (SqlBool a) (SqlBool b) = compare a b
    compareSameRank (SqlInteger a) (SqlInteger b) = compare a b
    compareSameRank (SqlInteger a) (SqlReal b) = compare (fromInteger a :: Double) b
    compareSameRank (SqlReal a) (SqlInteger b) = compare a (fromInteger b :: Double)
    compareSameRank (SqlReal a) (SqlReal b) = compare a b
    compareSameRank (SqlText a) (SqlText b) = compare a b
    compareSameRank (SqlBlob a) (SqlBlob b) = compare a b
    compareSameRank _ _ = EQ

type Row = Map.Map String SqlValue

data BackendError = BackendError
    { errorKind :: String
    , errorMessage :: String
    , errorTable :: Maybe String
    , errorColumn :: Maybe String
    } deriving (Eq, Show)

tableNotFound :: String -> BackendError
tableNotFound table = BackendError "TableNotFound" ("table not found: " ++ table) (Just table) Nothing

tableAlreadyExists :: String -> BackendError
tableAlreadyExists table = BackendError "TableAlreadyExists" ("table already exists: " ++ table) (Just table) Nothing

columnNotFound :: String -> String -> BackendError
columnNotFound table column = BackendError "ColumnNotFound" ("column not found: " ++ table ++ "." ++ column) (Just table) (Just column)

columnAlreadyExists :: String -> String -> BackendError
columnAlreadyExists table column = BackendError "ColumnAlreadyExists" ("column already exists: " ++ table ++ "." ++ column) (Just table) (Just column)

constraintViolation :: String -> String -> String -> BackendError
constraintViolation table column message = BackendError "ConstraintViolation" message (Just table) (Just column)

unsupported :: String -> BackendError
unsupported operation = BackendError "Unsupported" ("operation not supported: " ++ operation) Nothing Nothing

internalError :: String -> BackendError
internalError message = BackendError "Internal" message Nothing Nothing

indexAlreadyExists :: String -> BackendError
indexAlreadyExists name = BackendError "IndexAlreadyExists" ("index already exists: " ++ name) Nothing Nothing

indexNotFound :: String -> BackendError
indexNotFound name = BackendError "IndexNotFound" ("index not found: " ++ name) Nothing Nothing

triggerAlreadyExists :: String -> BackendError
triggerAlreadyExists name = BackendError "TriggerAlreadyExists" ("trigger already exists: " ++ name) Nothing Nothing

triggerNotFound :: String -> BackendError
triggerNotFound name = BackendError "TriggerNotFound" ("trigger not found: " ++ name) Nothing Nothing

data ColumnDef = ColumnDef
    { columnName :: String
    , columnTypeName :: String
    , columnNotNull :: Bool
    , columnPrimaryKey :: Bool
    , columnUnique :: Bool
    , columnAutoincrement :: Bool
    , columnDefaultValue :: SqlValue
    , columnHasDefault :: Bool
    , columnCheckExpression :: Maybe String
    , columnForeignKey :: Maybe String
    } deriving (Eq, Show)

defaultColumnDef :: String -> String -> ColumnDef
defaultColumnDef name typeName =
    ColumnDef
        { columnName = name
        , columnTypeName = typeName
        , columnNotNull = False
        , columnPrimaryKey = False
        , columnUnique = False
        , columnAutoincrement = False
        , columnDefaultValue = SqlNull
        , columnHasDefault = False
        , columnCheckExpression = Nothing
        , columnForeignKey = Nothing
        }

effectiveNotNull :: ColumnDef -> Bool
effectiveNotNull column = columnNotNull column || columnPrimaryKey column

effectiveUnique :: ColumnDef -> Bool
effectiveUnique column = columnUnique column || columnPrimaryKey column

data IndexDef = IndexDef
    { indexName :: String
    , indexTable :: String
    , indexColumns :: [String]
    , indexUnique :: Bool
    , indexAuto :: Bool
    } deriving (Eq, Show)

data TriggerDef = TriggerDef
    { triggerName :: String
    , triggerTable :: String
    , triggerTiming :: String
    , triggerEvent :: String
    , triggerBody :: String
    } deriving (Eq, Show)

triggerDef :: String -> String -> String -> String -> String -> TriggerDef
triggerDef name table timing event body =
    TriggerDef name table (map toUpper timing) (map toUpper event) body

data ListRowIterator = ListRowIterator [Row] Int Bool
    deriving (Eq, Show)

listRowIterator :: [Row] -> ListRowIterator
listRowIterator rows = ListRowIterator rows 0 False

iteratorNext :: ListRowIterator -> (Maybe Row, ListRowIterator)
iteratorNext iterator@(ListRowIterator rows index closed)
    | closed = (Nothing, iterator)
    | index >= length rows = (Nothing, iterator)
    | otherwise = (Just (rows !! index), ListRowIterator rows (index + 1) closed)

iteratorClose :: ListRowIterator -> ListRowIterator
iteratorClose (ListRowIterator rows index _) = ListRowIterator rows index True

iteratorToList :: ListRowIterator -> [Row]
iteratorToList iterator =
    case iteratorNext iterator of
        (Nothing, _) -> []
        (Just row, rest) -> row : iteratorToList rest

data StoredRow = StoredRow
    { storedRowId :: Int
    , storedRow :: Row
    } deriving (Eq, Show)

data ListCursor = ListCursor
    { cursorTableKey :: String
    , cursorRows :: [StoredRow]
    , cursorIndex :: Int
    , cursorCurrentRowId :: Maybe Int
    } deriving (Eq, Show)

cursorNext :: ListCursor -> (Maybe Row, ListCursor)
cursorNext cursor =
    let nextIndex = cursorIndex cursor + 1
    in case drop nextIndex (cursorRows cursor) of
        [] -> (Nothing, cursor { cursorIndex = nextIndex, cursorCurrentRowId = Nothing })
        (stored:_) ->
            ( Just (storedRow stored)
            , cursor { cursorIndex = nextIndex, cursorCurrentRowId = Just (storedRowId stored) }
            )

cursorCurrentRow :: ListCursor -> Maybe Row
cursorCurrentRow cursor = do
    rowId <- cursorCurrentRowId cursor
    storedRow <$> find ((== rowId) . storedRowId) (cursorRows cursor)

data TableState = TableState
    { tableName :: String
    , tableColumns :: [ColumnDef]
    , tableRows :: [StoredRow]
    , tableNextRowId :: Int
    } deriving (Eq, Show)

data BackendSnapshot = BackendSnapshot
    { snapshotTables :: Map.Map String TableState
    , snapshotIndexes :: Map.Map String IndexDef
    , snapshotTriggers :: Map.Map String TriggerDef
    , snapshotTriggersByTable :: Map.Map String [String]
    , snapshotUserVersion :: Int
    , snapshotSchemaVersion :: Int
    } deriving (Eq, Show)

data InMemoryBackend = InMemoryBackend
    { backendTables :: Map.Map String TableState
    , backendIndexes :: Map.Map String IndexDef
    , backendTriggers :: Map.Map String TriggerDef
    , backendTriggersByTable :: Map.Map String [String]
    , backendUserVersion :: Int
    , backendSchemaVersion :: Int
    , backendTransactionSnapshot :: Maybe BackendSnapshot
    , backendCurrentTransaction :: Maybe Int
    , backendNextTransaction :: Int
    , backendSavepoints :: [(String, BackendSnapshot)]
    } deriving (Eq, Show)

newBackend :: InMemoryBackend
newBackend =
    InMemoryBackend
        { backendTables = Map.empty
        , backendIndexes = Map.empty
        , backendTriggers = Map.empty
        , backendTriggersByTable = Map.empty
        , backendUserVersion = 0
        , backendSchemaVersion = 0
        , backendTransactionSnapshot = Nothing
        , backendCurrentTransaction = Nothing
        , backendNextTransaction = 1
        , backendSavepoints = []
        }

data BackendSchemaProvider = BackendSchemaProvider InMemoryBackend

backendAsSchemaProvider :: InMemoryBackend -> BackendSchemaProvider
backendAsSchemaProvider = BackendSchemaProvider

schemaProviderColumns :: BackendSchemaProvider -> String -> Either BackendError [String]
schemaProviderColumns (BackendSchemaProvider backend) table = map columnName <$> columns backend table

schemaProviderListIndexes :: BackendSchemaProvider -> Maybe String -> [IndexDef]
schemaProviderListIndexes (BackendSchemaProvider backend) = listIndexes backend

tables :: InMemoryBackend -> [String]
tables backend = sort (map tableName (Map.elems (backendTables backend)))

columns :: InMemoryBackend -> String -> Either BackendError [ColumnDef]
columns backend table = tableColumns <$> tableState backend table

scan :: InMemoryBackend -> String -> Either BackendError ListRowIterator
scan backend table = listRowIterator . map storedRow . tableRows <$> tableState backend table

openCursor :: InMemoryBackend -> String -> Either BackendError ListCursor
openCursor backend table = do
    state <- tableState backend table
    Right (ListCursor (normalize (tableName state)) (tableRows state) (-1) Nothing)

insert :: InMemoryBackend -> String -> Row -> Either BackendError InMemoryBackend
insert backend table row = do
    state <- tableState backend table
    candidate <- materializeRow state row
    validateRow backend state candidate Nothing
    let stored = StoredRow (tableNextRowId state) candidate
        state' = state { tableRows = tableRows state ++ [stored], tableNextRowId = tableNextRowId state + 1 }
    Right (putState backend state')

update :: InMemoryBackend -> String -> ListCursor -> Row -> Either BackendError InMemoryBackend
update backend table cursor assignments = do
    state <- tableState backend table
    rowId <- maybe (Left (internalError ("cursor is not positioned on " ++ tableName state))) Right (cursorCurrentRowId cursor)
    if cursorTableKey cursor /= normalize (tableName state)
        then Left (internalError ("cursor is not positioned on " ++ tableName state))
        else do
            stored <- maybe (Left (internalError "cursor row vanished")) Right (find ((== rowId) . storedRowId) (tableRows state))
            candidate <- foldl applyAssignment (Right (storedRow stored)) (Map.toList assignments)
            validateRow backend state candidate (Just rowId)
            let replace existing
                    | storedRowId existing == rowId = existing { storedRow = candidate }
                    | otherwise = existing
            Right (putState backend state { tableRows = map replace (tableRows state) })
  where
    applyAssignment (Left failure) _ = Left failure
    applyAssignment (Right acc) (name, value) =
        case findColumn stateForAssignment name of
            Nothing -> Left (columnNotFound (tableName stateForAssignment) name)
            Just column -> Right (Map.insert (columnName column) value acc)
    stateForAssignment = either (const (TableState table [] [] 0)) id (tableState backend table)

delete :: InMemoryBackend -> String -> ListCursor -> Either BackendError InMemoryBackend
delete backend table cursor = do
    state <- tableState backend table
    rowId <- maybe (Left (internalError ("cursor is not positioned on " ++ tableName state))) Right (cursorCurrentRowId cursor)
    if cursorTableKey cursor /= normalize (tableName state)
        then Left (internalError ("cursor is not positioned on " ++ tableName state))
        else Right (putState backend state { tableRows = filter ((/= rowId) . storedRowId) (tableRows state) })

createTable :: InMemoryBackend -> String -> [ColumnDef] -> Bool -> Either BackendError InMemoryBackend
createTable backend table columnDefs ifNotExists =
    let key = normalize table
    in case Map.lookup key (backendTables backend) of
        Just _
            | ifNotExists -> Right backend
            | otherwise -> Left (tableAlreadyExists table)
        Nothing ->
            case firstDuplicate (map (normalize . columnName) columnDefs) of
                Just duplicate -> Left (columnAlreadyExists table duplicate)
                Nothing ->
                    let state = TableState table columnDefs [] 0
                    in Right (putState (backend { backendSchemaVersion = backendSchemaVersion backend + 1 }) state)

dropTable :: InMemoryBackend -> String -> Bool -> Either BackendError InMemoryBackend
dropTable backend table ifExists =
    let key = normalize table
    in if Map.notMember key (backendTables backend)
        then if ifExists then Right backend else Left (tableNotFound table)
        else Right backend
            { backendTables = Map.delete key (backendTables backend)
            , backendIndexes = Map.filter ((/= key) . normalize . indexTable) (backendIndexes backend)
            , backendTriggers = Map.filter ((/= key) . normalize . triggerTable) (backendTriggers backend)
            , backendTriggersByTable = Map.delete key (backendTriggersByTable backend)
            , backendSchemaVersion = backendSchemaVersion backend + 1
            }

addColumn :: InMemoryBackend -> String -> ColumnDef -> Either BackendError InMemoryBackend
addColumn backend table column = do
    state <- tableState backend table
    case findColumn state (columnName column) of
        Just _ -> Left (columnAlreadyExists (tableName state) (columnName column))
        Nothing
            | effectiveNotNull column && not (columnHasDefault column) && not (null (tableRows state)) ->
                Left (constraintViolation (tableName state) (columnName column) ("NOT NULL constraint failed: " ++ tableName state ++ "." ++ columnName column))
            | otherwise ->
                let addDefault stored = stored { storedRow = Map.insert (columnName column) (columnDefaultValue column) (storedRow stored) }
                    state' = state { tableColumns = tableColumns state ++ [column], tableRows = map addDefault (tableRows state) }
                in Right (putState (backend { backendSchemaVersion = backendSchemaVersion backend + 1 }) state')

createIndex :: InMemoryBackend -> IndexDef -> Either BackendError InMemoryBackend
createIndex backend indexDef =
    let key = normalize (indexName indexDef)
    in if Map.member key (backendIndexes backend)
        then Left (indexAlreadyExists (indexName indexDef))
        else do
            state <- tableState backend (indexTable indexDef)
            mapM_ (realColumn state) (indexColumns indexDef)
            if indexUnique indexDef then validateUniqueIndex state indexDef Nothing Nothing else Right ()
            Right backend { backendIndexes = Map.insert key indexDef (backendIndexes backend), backendSchemaVersion = backendSchemaVersion backend + 1 }

dropIndex :: InMemoryBackend -> String -> Bool -> Either BackendError InMemoryBackend
dropIndex backend name ifExists =
    let key = normalize name
    in if Map.member key (backendIndexes backend)
        then Right backend { backendIndexes = Map.delete key (backendIndexes backend), backendSchemaVersion = backendSchemaVersion backend + 1 }
        else if ifExists then Right backend else Left (indexNotFound name)

listIndexes :: InMemoryBackend -> Maybe String -> [IndexDef]
listIndexes backend table =
    sortBy (\a b -> compare (indexName a) (indexName b)) $
        filter matchesTable (Map.elems (backendIndexes backend))
  where
    matchesTable indexDef =
        case table of
            Nothing -> True
            Just tableName' -> normalize (indexTable indexDef) == normalize tableName'

scanIndex :: InMemoryBackend -> String -> Maybe [SqlValue] -> Maybe [SqlValue] -> Bool -> Bool -> Either BackendError [Int]
scanIndex backend indexName' lo hi loInclusive hiInclusive = do
    indexDef <- maybe (Left (indexNotFound indexName')) Right (Map.lookup (normalize indexName') (backendIndexes backend))
    state <- tableState backend (indexTable indexDef)
    entries <- mapM (\stored -> do
        key <- indexKey state (storedRow stored) (indexColumns indexDef)
        Right (key, storedRowId stored)
        ) (tableRows state)
    let ordered = sortBy compareEntry entries
    Right [rowId | (key, rowId) <- ordered, lowerOk key, upperOk key]
  where
    compareEntry (left, leftId) (right, rightId) =
        case compareKeys left right of
            EQ -> compare leftId rightId
            other -> other

    lowerOk key =
        case lo of
            Nothing -> True
            Just bound -> compareKeys key bound == GT || (loInclusive && compareKeys key bound == EQ)

    upperOk key =
        case hi of
            Nothing -> True
            Just bound -> compareKeys key bound == LT || (hiInclusive && compareKeys key bound == EQ)

scanByRowIds :: InMemoryBackend -> String -> [Int] -> Either BackendError ListRowIterator
scanByRowIds backend table rowIds = do
    state <- tableState backend table
    let rowsById = Map.fromList [(storedRowId stored, storedRow stored) | stored <- tableRows state]
    Right (listRowIterator (mapMaybe (`Map.lookup` rowsById) rowIds))

beginTransaction :: InMemoryBackend -> Either BackendError (InMemoryBackend, Int)
beginTransaction backend =
    case backendCurrentTransaction backend of
        Just _ -> Left (unsupported "nested transactions")
        Nothing ->
            let handle = backendNextTransaction backend
            in Right
                ( backend
                    { backendTransactionSnapshot = Just (snapshot backend)
                    , backendCurrentTransaction = Just handle
                    , backendNextTransaction = handle + 1
                    }
                , handle
                )

commit :: InMemoryBackend -> Int -> Either BackendError InMemoryBackend
commit backend handle =
    if backendCurrentTransaction backend == Just handle
        then Right backend { backendTransactionSnapshot = Nothing, backendCurrentTransaction = Nothing, backendSavepoints = [] }
        else Left (internalError "invalid transaction handle")

rollback :: InMemoryBackend -> Int -> Either BackendError InMemoryBackend
rollback backend handle =
    case (backendCurrentTransaction backend, backendTransactionSnapshot backend) of
        (Just current, Just original)
            | current == handle ->
                Right ((restore backend original) { backendTransactionSnapshot = Nothing, backendCurrentTransaction = Nothing, backendSavepoints = [] })
        _ -> Left (internalError "invalid transaction handle")

createSavepoint :: InMemoryBackend -> String -> Either BackendError InMemoryBackend
createSavepoint backend name =
    case backendCurrentTransaction backend of
        Nothing -> Left (unsupported "savepoints outside transaction")
        Just _ -> Right backend { backendSavepoints = backendSavepoints backend ++ [(name, snapshot backend)] }

releaseSavepoint :: InMemoryBackend -> String -> Either BackendError InMemoryBackend
releaseSavepoint backend name = do
    index <- savepointIndex backend name
    Right backend { backendSavepoints = take index (backendSavepoints backend) }

rollbackToSavepoint :: InMemoryBackend -> String -> Either BackendError InMemoryBackend
rollbackToSavepoint backend name = do
    index <- savepointIndex backend name
    let (_, saved) = backendSavepoints backend !! index
        restored = restore backend saved
    Right restored { backendSavepoints = take (index + 1) (backendSavepoints backend) }

createTrigger :: InMemoryBackend -> TriggerDef -> Either BackendError InMemoryBackend
createTrigger backend rawTrigger =
    let key = normalize (triggerName rawTrigger)
        normalizedTrigger = triggerDef (triggerName rawTrigger) (triggerTable rawTrigger) (triggerTiming rawTrigger) (triggerEvent rawTrigger) (triggerBody rawTrigger)
    in if Map.member key (backendTriggers backend)
        then Left (triggerAlreadyExists (triggerName rawTrigger))
        else do
            _ <- tableState backend (triggerTable rawTrigger)
            let tableKey = normalize (triggerTable rawTrigger)
                tableTriggers = Map.findWithDefault [] tableKey (backendTriggersByTable backend)
            Right backend
                { backendTriggers = Map.insert key normalizedTrigger (backendTriggers backend)
                , backendTriggersByTable = Map.insert tableKey (tableTriggers ++ [key]) (backendTriggersByTable backend)
                , backendSchemaVersion = backendSchemaVersion backend + 1
                }

dropTrigger :: InMemoryBackend -> String -> Bool -> Either BackendError InMemoryBackend
dropTrigger backend name ifExists =
    let key = normalize name
    in case Map.lookup key (backendTriggers backend) of
        Nothing -> if ifExists then Right backend else Left (triggerNotFound name)
        Just trigger ->
            let tableKey = normalize (triggerTable trigger)
                remaining = filter (/= key) (Map.findWithDefault [] tableKey (backendTriggersByTable backend))
            in Right backend
                { backendTriggers = Map.delete key (backendTriggers backend)
                , backendTriggersByTable = Map.insert tableKey remaining (backendTriggersByTable backend)
                , backendSchemaVersion = backendSchemaVersion backend + 1
                }

listTriggers :: InMemoryBackend -> String -> [TriggerDef]
listTriggers backend table =
    mapMaybe (\key -> Map.lookup key (backendTriggers backend)) (Map.findWithDefault [] (normalize table) (backendTriggersByTable backend))

tableState :: InMemoryBackend -> String -> Either BackendError TableState
tableState backend table = maybe (Left (tableNotFound table)) Right (Map.lookup (normalize table) (backendTables backend))

putState :: InMemoryBackend -> TableState -> InMemoryBackend
putState backend state = backend { backendTables = Map.insert (normalize (tableName state)) state (backendTables backend) }

materializeRow :: TableState -> Row -> Either BackendError Row
materializeRow state row = do
    candidate <- foldl addColumnValue (Right Map.empty) (tableColumns state)
    case find (\name -> maybe True (const False) (findColumn state name)) (Map.keys row) of
        Just missing -> Left (columnNotFound (tableName state) missing)
        Nothing -> Right candidate
  where
    addColumnValue (Left failure) _ = Left failure
    addColumnValue (Right acc) column =
        let value =
                case findValue row (columnName column) of
                    Just present -> present
                    Nothing
                        | columnAutoincrement column && columnPrimaryKey column -> SqlInteger (nextAutoincrementValue state column)
                        | columnHasDefault column -> columnDefaultValue column
                        | otherwise -> SqlNull
        in Right (Map.insert (columnName column) value acc)

findValue :: Row -> String -> Maybe SqlValue
findValue row name = snd <$> find ((== normalize name) . normalize . fst) (Map.toList row)

nextAutoincrementValue :: TableState -> ColumnDef -> Integer
nextAutoincrementValue state column =
    let values =
            [ value
            | stored <- tableRows state
            , Just (SqlInteger value) <- [Map.lookup (columnName column) (storedRow stored)]
            ]
    in case values of
        [] -> 1
        _ -> maximum values + 1

validateRow :: InMemoryBackend -> TableState -> Row -> Maybe Int -> Either BackendError ()
validateRow backend state row skipRowId = do
    mapM_ validateColumn (tableColumns state)
    mapM_ validateIndex [indexDef | indexDef <- Map.elems (backendIndexes backend), indexUnique indexDef, normalize (indexTable indexDef) == normalize (tableName state)]
  where
    validateColumn column = do
        let value = Map.findWithDefault SqlNull (columnName column) row
        if effectiveNotNull column && value == SqlNull
            then Left (constraintViolation (tableName state) (columnName column) ("NOT NULL constraint failed: " ++ tableName state ++ "." ++ columnName column))
            else Right ()
        if effectiveUnique column && value /= SqlNull
            then mapM_ (checkUniqueColumn column value) (tableRows state)
            else Right ()

    checkUniqueColumn column value stored =
        if Just (storedRowId stored) /= skipRowId && compareSqlValues (Map.findWithDefault SqlNull (columnName column) (storedRow stored)) value == EQ
            then
                let label = if columnPrimaryKey column then "PRIMARY KEY" else "UNIQUE"
                in Left (constraintViolation (tableName state) (columnName column) (label ++ " constraint failed: " ++ tableName state ++ "." ++ columnName column))
            else Right ()

    validateIndex indexDef = validateUniqueIndex state indexDef (Just row) skipRowId

validateUniqueIndex :: TableState -> IndexDef -> Maybe Row -> Maybe Int -> Either BackendError ()
validateUniqueIndex state indexDef candidate skipRowId =
    case candidate of
        Just row -> do
            candidateKey <- indexKey state row (indexColumns indexDef)
            if SqlNull `elem` candidateKey
                then Right ()
                else mapM_ (checkCandidate candidateKey) (tableRows state)
        Nothing -> do
            keys <- mapM (\stored -> indexKey state (storedRow stored) (indexColumns indexDef)) (tableRows state)
            let presentKeys = filter (notElem SqlNull) keys
            if hasDuplicateKey presentKeys
                then Left uniqueFailure
                else Right ()
  where
    checkCandidate candidateKey stored =
        if Just (storedRowId stored) /= skipRowId && compareKeysForStored stored candidateKey == Right EQ
            then Left uniqueFailure
            else Right ()

    compareKeysForStored stored candidateKey = (`compareKeys` candidateKey) <$> indexKey state (storedRow stored) (indexColumns indexDef)
    columnsLabel = joinWithComma (indexColumns indexDef)
    uniqueFailure = constraintViolation (tableName state) columnsLabel ("UNIQUE constraint failed: " ++ tableName state ++ "." ++ columnsLabel)

findColumn :: TableState -> String -> Maybe ColumnDef
findColumn state name = find ((== normalize name) . normalize . columnName) (tableColumns state)

realColumn :: TableState -> String -> Either BackendError String
realColumn state name = maybe (Left (columnNotFound (tableName state) name)) (Right . columnName) (findColumn state name)

indexKey :: TableState -> Row -> [String] -> Either BackendError [SqlValue]
indexKey state row names = mapM valueFor names
  where
    valueFor name = do
        realName <- realColumn state name
        Right (Map.findWithDefault SqlNull realName row)

compareKeys :: [SqlValue] -> [SqlValue] -> Ordering
compareKeys left right =
    case dropWhile (== EQ) (zipWith compareSqlValues left right) of
        (ordering:_) -> ordering
        [] -> compare (length left) (length right)

hasDuplicateKey :: [[SqlValue]] -> Bool
hasDuplicateKey [] = False
hasDuplicateKey (key:rest) = any ((== EQ) . compareKeys key) rest || hasDuplicateKey rest

firstDuplicate :: [String] -> Maybe String
firstDuplicate = go []
  where
    go _ [] = Nothing
    go seen (value:rest)
        | value `elem` seen = Just value
        | otherwise = go (value:seen) rest

savepointIndex :: InMemoryBackend -> String -> Either BackendError Int
savepointIndex backend name =
    case backendCurrentTransaction backend of
        Nothing -> Left (unsupported "savepoints outside transaction")
        Just _ ->
            case findIndexByName 0 (backendSavepoints backend) of
                Nothing -> Left (internalError ("savepoint not found: " ++ name))
                Just index -> Right index
  where
    findIndexByName _ [] = Nothing
    findIndexByName index ((candidate, _):rest)
        | candidate == name = Just index
        | otherwise = findIndexByName (index + 1) rest

snapshot :: InMemoryBackend -> BackendSnapshot
snapshot backend =
    BackendSnapshot
        { snapshotTables = backendTables backend
        , snapshotIndexes = backendIndexes backend
        , snapshotTriggers = backendTriggers backend
        , snapshotTriggersByTable = backendTriggersByTable backend
        , snapshotUserVersion = backendUserVersion backend
        , snapshotSchemaVersion = backendSchemaVersion backend
        }

restore :: InMemoryBackend -> BackendSnapshot -> InMemoryBackend
restore backend saved =
    backend
        { backendTables = snapshotTables saved
        , backendIndexes = snapshotIndexes saved
        , backendTriggers = snapshotTriggers saved
        , backendTriggersByTable = snapshotTriggersByTable saved
        , backendUserVersion = snapshotUserVersion saved
        , backendSchemaVersion = snapshotSchemaVersion saved
        }

normalize :: String -> String
normalize = map toLower

joinWithComma :: [String] -> String
joinWithComma [] = ""
joinWithComma [value] = value
joinWithComma (value:rest) = value ++ "," ++ joinWithComma rest
