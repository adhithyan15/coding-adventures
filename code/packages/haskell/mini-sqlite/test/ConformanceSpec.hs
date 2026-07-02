{-# LANGUAGE OverloadedStrings #-}
-- ConformanceSpec.hs — runs the 24 conformance fixtures from
-- code/specs/mini-sqlite-conformance/fixtures/ against the public MiniSqlite API.
--
-- Each fixture is a JSON file describing a sequence of operations:
--   "execute"             — run SQL with optional params; no expected rows
--   "query"               — run SQL; compare columns and rows against expected
--   "executemany"         — run SQL for each param tuple in param_seq
--   "commit"              — call commit on the connection
--   "rollback"            — call rollback on the connection
--   "expect_error"        — run SQL that must fail with a given error_type
--   "connect_expect_error"— connect(...) that must fail with a given error_type
--   "fetchone_test"       — run SQL; call fetchOne twice; compare rows
--   "fetchmany_test"      — run SQL; call fetchMany twice; compare batches
--   "fetchall_test"       — run SQL; call fetchAll once; compare all rows
--   "fetchall_empty_test" — run SQL; fetchAll must return empty list
--
-- Fixtures live relative to this source file; we resolve them at runtime
-- using the Haskell source-file path trick: the test must be invoked from
-- the package root (cabal does this by default via `cabal test`), so we use
-- a relative path from the package root.

module ConformanceSpec (conformanceSpec) where

import Control.Monad (forM_)
import Data.Aeson (Value(..), decode)
import qualified Data.Aeson.Key as AKey
import qualified Data.Aeson.KeyMap as KM
import Data.ByteString.Lazy (readFile)
import Data.List (sort)
import Data.Maybe (fromMaybe)
import qualified Data.Map.Strict as Map
import qualified Data.Text as T
import Data.Scientific (isInteger, toRealFloat)
import Prelude hiding (readFile)
import System.Directory (doesFileExist, getDirectoryContents)
import System.FilePath ((</>), takeExtension)
import Test.Hspec

import MiniSqlite

-- | Path to the fixture directory relative to the package root.
-- cabal test changes cwd to the package root (code/packages/haskell/mini-sqlite).
-- Three levels up reaches code/, then specs/ holds the fixtures.
fixtureDir :: FilePath
fixtureDir = "../../../specs/mini-sqlite-conformance/fixtures"

conformanceSpec :: Spec
conformanceSpec = describe "conformance fixtures" $ do
    -- runIO executes IO at spec-discovery time, making fixture paths available
    -- for building individual test items.
    files <- runIO loadFixtureFiles
    if null files
        then it "fixture directory found" $
                 expectationFailure ("No fixtures found. Expected dir: " ++ fixtureDir)
        else forM_ files $ \(name, path) ->
                 it name $ do
                     content <- readFile path
                     case decode content :: Maybe Value of
                         Nothing  -> expectationFailure ("Failed to parse JSON: " ++ path)
                         Just obj -> runFixture obj

-- | Load the list of fixture files from the fixture directory.
-- Returns (name, path) pairs sorted by name.
loadFixtureFiles :: IO [(String, FilePath)]
loadFixtureFiles = do
    let tryDirs = [fixtureDir, "code/specs/mini-sqlite-conformance/fixtures"]
    result <- findFixtureDir tryDirs
    case result of
        Nothing        -> pure []
        Just (dir, fs) -> pure (sort [(f, dir </> f) | f <- fs])

-- | Try a list of directories; return (dir, json-file-names) for the first
-- that contains the sentinel fixture file. Returns Nothing if none found.
findFixtureDir :: [FilePath] -> IO (Maybe (FilePath, [FilePath]))
findFixtureDir [] = pure Nothing
findFixtureDir (d:ds) = do
    exists <- doesFileExist (d </> "01-create-select.json")
    if exists
        then do
            allFiles <- getDirectoryContents d
            let jsonFiles = filter (\f -> takeExtension f == ".json") allFiles
            pure (Just (d, jsonFiles))
        else findFixtureDir ds

-- | Run a single fixture object.
runFixture :: Value -> IO ()
runFixture jv@(Object _) = do
    let m = mapFromObj jv
    let fixtureId    = getStr "id" m
    let steps        = fromMaybe (Array mempty) (Map.lookup "steps" m)
    let connectSteps = fromMaybe (Array mempty) (Map.lookup "connect_steps" m)
    conn <- do
        r <- connect ":memory:"
        case r of
            Left e  -> do
                expectationFailure ("connect failed: " ++ errorMessage e)
                error "unreachable"
            Right c -> pure c
    runConnectSteps m connectSteps conn
    runSteps fixtureId steps conn
    close conn
runFixture _ = expectationFailure "fixture root must be a JSON object"

-- | Execute connect_steps (fixtures that test connection rejection).
runConnectSteps :: Map.Map T.Text Value -> Value -> Connection -> IO ()
runConnectSteps _parentMap (Array steps) _ =
    forM_ steps $ \step ->
        case step of
            Object _ ->
                let m = mapFromObj step
                in case getStr "op" m of
                    "connect_expect_error" -> do
                        let db = getStr "database" m
                        let errType = getStr "error_type" m
                        result <- connect (T.unpack db)
                        case result of
                            Right _ -> expectationFailure
                                ("connect(\"" ++ T.unpack db ++ "\") should fail with " ++ T.unpack errType)
                            Left e  ->
                                errorKind e `shouldBe` T.unpack errType
                    _ -> pure ()
            _ -> pure ()
runConnectSteps _ _ _ = pure ()

-- | Execute the steps array for a fixture.
runSteps :: T.Text -> Value -> Connection -> IO ()
runSteps fixId (Array steps) conn =
    forM_ steps $ \step ->
        case step of
            Object _ -> runStep fixId (mapFromObj step) conn
            _        -> pure ()
runSteps _ _ _ = pure ()

-- | Execute a single step.
runStep :: T.Text -> Map.Map T.Text Value -> Connection -> IO ()
runStep fixId obj conn =
    case getStr "op" obj of
        "execute" -> do
            let sql    = T.unpack (getStr "sql" obj)
            let params = getParams obj
            result <- execute conn sql params
            case result of
                Left e  -> expectationFailure
                    ("execute failed (" ++ T.unpack fixId ++ "): " ++ errorMessage e ++ " SQL: " ++ sql)
                Right _ -> pure ()

        "executemany" -> do
            let sql    = T.unpack (getStr "sql" obj)
            let paramSeq = getParamSeq obj
            result <- executeMany conn sql paramSeq
            case result of
                Left e  -> expectationFailure
                    ("executemany failed (" ++ T.unpack fixId ++ "): " ++ errorMessage e)
                Right _ -> pure ()

        "query" -> do
            let sql    = T.unpack (getStr "sql" obj)
            let params = getParams obj
            let expCols = map T.unpack (getStrList "expected_columns" obj)
            let expRows = getExpectedRows obj
            result <- execute conn sql params
            case result of
                Left e  -> expectationFailure
                    ("query failed (" ++ T.unpack fixId ++ "): " ++ errorMessage e ++ " SQL: " ++ sql)
                Right cur -> do
                    desc <- cursorDescription cur
                    let gotCols = map columnName desc
                    gotCols `shouldBe` expCols
                    allRows <- fetchAll cur
                    case allRows of
                        Left e  -> expectationFailure ("fetchAll failed: " ++ errorMessage e)
                        Right rs -> do
                            let gotRows = map (map sqlValueToJson) rs
                            gotRows `shouldBe` expRows

        "expect_error" -> do
            let sql    = T.unpack (getStr "sql" obj)
            let params = getParams obj
            let errType = T.unpack (getStr "error_type" obj)
            result <- execute conn sql params
            case result of
                Right _ -> expectationFailure
                    ("expected error " ++ errType ++ " but got success for: " ++ sql)
                Left e  ->
                    errorKind e `shouldBe` errType

        "commit" -> do
            r <- commit conn
            case r of
                Left e  -> expectationFailure ("commit failed: " ++ errorMessage e)
                Right _ -> pure ()

        "rollback" -> do
            r <- rollback conn
            case r of
                Left e  -> expectationFailure ("rollback failed: " ++ errorMessage e)
                Right _ -> pure ()

        "fetchone_test" -> do
            -- Execute the query, then call fetchOne twice and compare the two
            -- returned rows against expected_first and expected_second.
            let sql    = T.unpack (getStr "sql" obj)
            let params = getParams obj
            let expFirst  = getSingleRow "expected_first"  obj
            let expSecond = getSingleRow "expected_second" obj
            result <- execute conn sql params
            case result of
                Left e  -> expectationFailure
                    ("fetchone_test execute failed (" ++ T.unpack fixId ++ "): "
                     ++ errorMessage e ++ " SQL: " ++ sql)
                Right cur -> do
                    r1 <- fetchOne cur
                    case r1 of
                        Left e  -> expectationFailure ("fetchOne #1 failed: " ++ errorMessage e)
                        Right Nothing  -> expectationFailure "fetchOne #1 returned Nothing; expected a row"
                        Right (Just row1) -> map sqlValueToJson row1 `shouldBe` expFirst
                    r2 <- fetchOne cur
                    case r2 of
                        Left e  -> expectationFailure ("fetchOne #2 failed: " ++ errorMessage e)
                        Right Nothing  -> expectationFailure "fetchOne #2 returned Nothing; expected a row"
                        Right (Just row2) -> map sqlValueToJson row2 `shouldBe` expSecond

        "fetchmany_test" -> do
            -- Execute the query, then call fetchMany twice with the given
            -- batch size, comparing each batch against the fixture expectations.
            let sql    = T.unpack (getStr "sql" obj)
            let params = getParams obj
            let sz     = getInt "size" obj
            let expFirst  = getExpectedRows' "expected_first_batch"  obj
            let expSecond = getExpectedRows' "expected_second_batch" obj
            result <- execute conn sql params
            case result of
                Left e  -> expectationFailure
                    ("fetchmany_test execute failed (" ++ T.unpack fixId ++ "): "
                     ++ errorMessage e ++ " SQL: " ++ sql)
                Right cur -> do
                    r1 <- fetchMany cur sz
                    case r1 of
                        Left e   -> expectationFailure ("fetchMany #1 failed: " ++ errorMessage e)
                        Right b1 -> map (map sqlValueToJson) b1 `shouldBe` expFirst
                    r2 <- fetchMany cur sz
                    case r2 of
                        Left e   -> expectationFailure ("fetchMany #2 failed: " ++ errorMessage e)
                        Right b2 -> map (map sqlValueToJson) b2 `shouldBe` expSecond

        "fetchall_test" -> do
            let sql    = T.unpack (getStr "sql" obj)
            let params = getParams obj
            let expRows = getExpectedRows' "expected_rows" obj
            result <- execute conn sql params
            case result of
                Left e  -> expectationFailure
                    ("fetchall_test execute failed (" ++ T.unpack fixId ++ "): "
                     ++ errorMessage e ++ " SQL: " ++ sql)
                Right cur -> do
                    r <- fetchAll cur
                    case r of
                        Left e    -> expectationFailure ("fetchAll failed: " ++ errorMessage e)
                        Right got -> map (map sqlValueToJson) got `shouldBe` expRows

        "fetchall_empty_test" -> do
            let sql    = T.unpack (getStr "sql" obj)
            let params = getParams obj
            result <- execute conn sql params
            case result of
                Left e  -> expectationFailure
                    ("fetchall_empty_test execute failed (" ++ T.unpack fixId ++ "): "
                     ++ errorMessage e ++ " SQL: " ++ sql)
                Right cur -> do
                    r <- fetchAll cur
                    case r of
                        Left e    -> expectationFailure ("fetchAll (empty) failed: " ++ errorMessage e)
                        Right got -> got `shouldBe` []

        op -> expectationFailure ("unknown op: " ++ T.unpack op)

-- ── JSON helpers ──────────────────────────────────────────────────────────

mapFromObj :: Value -> Map.Map T.Text Value
mapFromObj (Object m) = Map.fromList [(AKey.toText k, v) | (k, v) <- KM.toList m]
mapFromObj _          = Map.empty

getStr :: T.Text -> Map.Map T.Text Value -> T.Text
getStr key m = case Map.lookup key m of
    Just (String s) -> s
    _               -> ""

getStrList :: T.Text -> Map.Map T.Text Value -> [T.Text]
getStrList key m = case Map.lookup key m of
    Just (Array arr) -> [s | String s <- foldr (:) [] arr]
    _                -> []

getParams :: Map.Map T.Text Value -> [SqlValue]
getParams m = case Map.lookup "params" m of
    Just (Array arr) -> map jsonToSqlValue (foldr (:) [] arr)
    _                -> []

getParamSeq :: Map.Map T.Text Value -> [[SqlValue]]
getParamSeq m = case Map.lookup "param_seq" m of
    Just (Array rows_) ->
        [ map jsonToSqlValue (foldr (:) [] arr)
        | Array arr <- foldr (:) [] rows_
        ]
    _ -> []

-- | Convert a JSON value (fixture param) to a SqlValue.
jsonToSqlValue :: Value -> SqlValue
jsonToSqlValue Null          = SqlNull
jsonToSqlValue (Bool b)      = SqlBool b
jsonToSqlValue (Number n)
    | isInteger n = SqlInteger (round n)
    | otherwise   = SqlReal (toRealFloat n)
jsonToSqlValue (String s)    = SqlText (T.unpack s)
jsonToSqlValue _             = SqlNull

-- | Convert a SqlValue to a canonical JSON representation for comparison.
sqlValueToJson :: SqlValue -> Value
sqlValueToJson SqlNull         = Null
sqlValueToJson (SqlBool b)     = Bool b
sqlValueToJson (SqlInteger i)  = Number (fromIntegral i)
sqlValueToJson (SqlReal d)     = Number (realToFrac d)
sqlValueToJson (SqlText s)     = String (T.pack s)

-- | Get expected_rows as a list of JSON value lists.
getExpectedRows :: Map.Map T.Text Value -> [[Value]]
getExpectedRows = getExpectedRows' "expected_rows"

-- | Get an arbitrary key's array-of-arrays value as a list of JSON value lists.
-- Used for expected_first_batch, expected_second_batch, etc.
getExpectedRows' :: T.Text -> Map.Map T.Text Value -> [[Value]]
getExpectedRows' key m = case Map.lookup key m of
    Just (Array rows_) ->
        [ normaliseRow arr
        | Array arr <- foldr (:) [] rows_
        ]
    _ -> []
  where
    normaliseRow arr = map normaliseVal (foldr (:) [] arr)
    normaliseVal Null       = Null
    normaliseVal (Bool b)   = Bool b
    normaliseVal (Number n)
        | isInteger n = Number (fromInteger (round n :: Integer))
        | otherwise   = Number n
    normaliseVal v          = v

-- | Get a single expected row (flat JSON array) from the given key.
-- Used for expected_first / expected_second in fetchone_test.
getSingleRow :: T.Text -> Map.Map T.Text Value -> [Value]
getSingleRow key m = case Map.lookup key m of
    Just (Array arr) -> map normaliseVal (foldr (:) [] arr)
    _                -> []
  where
    normaliseVal Null       = Null
    normaliseVal (Bool b)   = Bool b
    normaliseVal (Number n)
        | isInteger n = Number (fromInteger (round n :: Integer))
        | otherwise   = Number n
    normaliseVal v          = v

-- | Get an integer field from the step object (used for fetchmany size).
getInt :: T.Text -> Map.Map T.Text Value -> Int
getInt key m = case Map.lookup key m of
    Just (Number n) -> round n
    _               -> 0
