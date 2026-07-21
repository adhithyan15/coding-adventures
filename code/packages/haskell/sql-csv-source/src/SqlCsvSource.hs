-- | CSV-backed tables for the educational Haskell SQL execution engine.
module SqlCsvSource
    ( CsvDataSource
    , csvDirectory
    , loadCsvDataSource
    , csvDataSource
    , csvSchema
    , csvScan
    , executeCsv
    , tryExecuteCsv
    , coerceCsvValue
    , parseCsv
    , version
    ) where

import Control.Exception (IOException, displayException, try)
import Control.Monad (filterM)
import Data.Char (isSpace, toLower)
import Data.List (nub, sort)
import qualified Data.Map.Strict as Map
import qualified Data.Text as Text
import qualified Data.Text.IO as TextIO
import SqlExecutionEngine
    ( DataSource(..)
    , ExecutionResult(..)
    , QueryResult
    , Row
    , SqlExecutionError(..)
    , SqlValue(..)
    , execute
    )
import System.Directory (doesFileExist, listDirectory)
import System.FilePath ((</>), dropExtension, takeExtension)
import Text.Read (readMaybe)

-- | Package version shared with the established implementations.
version :: String
version = "0.1.0"

-- | An immutable snapshot of every CSV table found in one directory.
data CsvDataSource = CsvDataSource
    { csvDirectory :: FilePath
    , sourceSchemas :: Map.Map String [String]
    , sourceTables :: Map.Map String [Row]
    } deriving (Eq, Show)

-- | List and load every @*.csv@ file in a directory.
--
-- File IO is completed here so the resulting 'DataSource' callbacks remain
-- pure, matching the existing SQL execution engine contract.
loadCsvDataSource :: FilePath -> IO (Either SqlExecutionError CsvDataSource)
loadCsvDataSource directory = do
    listing <- listDirectoryChecked directory
    case listing of
        Left failure -> pure (Left (ioErrorMessage "reading CSV directory" directory failure))
        Right entries -> do
            let csvEntries = sort
                    [ entry
                    | entry <- entries
                    , map toLower (takeExtension entry) == ".csv"
                    ]
            files <- filterM (doesFileExist . (directory </>)) csvEntries
            loaded <- mapM (loadTable directory) files
            pure (buildSource directory =<< sequence loaded)

-- | Convert a loaded snapshot to the engine's pure data-source record.
csvDataSource :: CsvDataSource -> DataSource
csvDataSource source = DataSource
    { dataSourceSchema = csvSchema source
    , dataSourceScan = csvScan source
    }

-- | Look up a table's columns in header order.
csvSchema :: CsvDataSource -> String -> Either SqlExecutionError [String]
csvSchema source tableName =
    maybe (Left (tableNotFound tableName)) Right
        (Map.lookup tableName (sourceSchemas source))

-- | Look up all typed rows for a table.
csvScan :: CsvDataSource -> String -> Either SqlExecutionError [Row]
csvScan source tableName =
    maybe (Left (tableNotFound tableName)) Right
        (Map.lookup tableName (sourceTables source))

-- | Load a directory snapshot and execute one SQL query against it.
executeCsv
    :: String
    -> FilePath
    -> IO (Either SqlExecutionError QueryResult)
executeCsv sql directory = do
    loaded <- loadCsvDataSource directory
    pure (loaded >>= execute sql . csvDataSource)

-- | Total convenience wrapper matching the execution engine's result shape.
tryExecuteCsv :: String -> FilePath -> IO ExecutionResult
tryExecuteCsv sql directory = do
    result <- executeCsv sql directory
    pure $ case result of
        Left failure -> ExecutionResult
            { executionOk = False
            , executionResult = Nothing
            , executionError = Just (sqlExecutionMessage failure)
            }
        Right queryResult -> ExecutionResult
            { executionOk = True
            , executionResult = Just queryResult
            , executionError = Nothing
            }

-- | Parse one CSV document into a header and typed rows.
parseCsv :: String -> Either SqlExecutionError ([String], [Row])
parseCsv source = do
    records <- parseCsvRecords source
    case records of
        [] -> Right ([], [])
        rawHeader : rawRows -> do
            let header = case map trim rawHeader of
                    [""] -> []
                    columns -> columns
            validateHeader header
            rows <- traverse (uncurry (recordToRow header))
                (zip [2 :: Int ..] rawRows)
            Right (header, rows)

-- | Coerce one untyped CSV field into the engine's SQL value family.
coerceCsvValue :: String -> SqlValue
coerceCsvValue value
    | null value = SqlNull
    | lowered == "true" = SqlBool True
    | lowered == "false" = SqlBool False
    | Just integer <- readMaybe value = SqlInteger integer
    | Just real <- readMaybe value
    , not (isNaN real || isInfinite real) = SqlReal real
    | otherwise = SqlText value
  where
    lowered = map toLower value

loadTable
    :: FilePath
    -> FilePath
    -> IO (Either SqlExecutionError (String, [String], [Row]))
loadTable directory entry = do
    let path = directory </> entry
        tableName = dropExtension entry
    content <- readUtf8 path
    pure $ case content of
        Left failure -> Left (ioErrorMessage "reading CSV table" path failure)
        Right text -> do
            (columns, rows) <- firstContext tableName (parseCsv (Text.unpack text))
            Right (tableName, columns, rows)

buildSource
    :: FilePath
    -> [(String, [String], [Row])]
    -> Either SqlExecutionError CsvDataSource
buildSource directory tables
    | length normalizedNames /= length (nub normalizedNames) =
        Left (csvError "CSV table names collide when compared case-insensitively")
    | otherwise = Right CsvDataSource
        { csvDirectory = directory
        , sourceSchemas = Map.fromList
            [(name, columns) | (name, columns, _) <- tables]
        , sourceTables = Map.fromList
            [(name, rows) | (name, _, rows) <- tables]
        }
  where
    normalizedNames = map (map toLower . firstOf3) tables
    firstOf3 (name, _, _) = name

validateHeader :: [String] -> Either SqlExecutionError ()
validateHeader [] = Right ()
validateHeader header
    | any null header = Left (csvError "CSV header contains an empty column name")
    | length header /= length (nub header) =
        Left (csvError "CSV header contains duplicate column names")
    | otherwise = Right ()

recordToRow
    :: [String]
    -> Int
    -> [String]
    -> Either SqlExecutionError Row
recordToRow header rowNumber fields
    | length fields /= length header = Left (csvError
        ("CSV row " ++ show rowNumber ++ " has " ++ show (length fields)
            ++ " fields; expected " ++ show (length header)))
    | otherwise = Right (Map.fromList
        (zip header (map coerceCsvValue fields)))

-- A small strict record parser keeps this adapter useful even though the
-- current Haskell csv-parser package is still a token-level starter wrapper.
data CsvMode = FieldStart | UnquotedField | QuotedField | AfterQuote
    deriving (Eq, Show)

parseCsvRecords :: String -> Either SqlExecutionError [[String]]
parseCsvRecords = go FieldStart [] [] [] False . stripBom
  where
    go mode field fields records touched remaining = case remaining of
        [] -> finishInput mode field fields records touched
        '\r' : '\n' : rest -> lineBreak mode field fields records rest
        '\r' : rest -> lineBreak mode field fields records rest
        '\n' : rest -> lineBreak mode field fields records rest
        '"' : '"' : rest
            | mode == QuotedField ->
                go QuotedField ('"' : field) fields records True rest
        '"' : rest -> case mode of
            FieldStart -> go QuotedField field fields records True rest
            QuotedField -> go AfterQuote field fields records True rest
            UnquotedField -> Left (csvError "unexpected quote in unquoted CSV field")
            AfterQuote -> Left (csvError "unexpected quote after closing CSV quote")
        ',' : rest -> case mode of
            QuotedField -> go QuotedField (',' : field) fields records True rest
            _ -> go FieldStart [] (reverse field : fields) records True rest
        character : rest -> case mode of
            FieldStart -> go UnquotedField [character] fields records True rest
            UnquotedField -> go UnquotedField (character : field) fields records True rest
            QuotedField -> go QuotedField (character : field) fields records True rest
            AfterQuote
                | character == ' ' || character == '\t' ->
                    go AfterQuote field fields records True rest
                | otherwise -> Left (csvError
                    ("unexpected character " ++ show character
                        ++ " after closing CSV quote"))

    lineBreak QuotedField field fields records rest =
        go QuotedField ('\n' : field) fields records True rest
    lineBreak _ field fields records rest =
        go FieldStart [] []
            (finishRecord field fields : records) False rest

    finishInput QuotedField _ _ _ _ =
        Left (csvError "unclosed quoted CSV field")
    finishInput _ field fields records touched
        | touched || not (null field) || not (null fields) =
            Right (reverse (finishRecord field fields : records))
        | otherwise = Right (reverse records)

    finishRecord field fields = reverse (reverse field : fields)

stripBom :: String -> String
stripBom ('\xfeff' : rest) = rest
stripBom text = text

trim :: String -> String
trim = reverse . dropWhile isSpace . reverse . dropWhile isSpace

tableNotFound :: String -> SqlExecutionError
tableNotFound name = csvError ("table not found: " ++ name)

csvError :: String -> SqlExecutionError
csvError = SqlExecutionError

firstContext
    :: String
    -> Either SqlExecutionError value
    -> Either SqlExecutionError value
firstContext tableName result = case result of
    Left failure -> Left (csvError
        ("parsing CSV table " ++ tableName ++ ": "
            ++ sqlExecutionMessage failure))
    Right value -> Right value

ioErrorMessage :: String -> FilePath -> IOException -> SqlExecutionError
ioErrorMessage action path failure = csvError
    (action ++ " " ++ show path ++ ": " ++ displayException failure)

listDirectoryChecked :: FilePath -> IO (Either IOException [FilePath])
listDirectoryChecked = try . listDirectory

readUtf8 :: FilePath -> IO (Either IOException Text.Text)
readUtf8 = try . TextIO.readFile
