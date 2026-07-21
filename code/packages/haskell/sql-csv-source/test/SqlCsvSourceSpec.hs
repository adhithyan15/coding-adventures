module SqlCsvSourceSpec (spec) where

import Control.Exception (bracket)
import qualified Data.Map.Strict as Map
import qualified Data.Text as Text
import qualified Data.Text.IO as TextIO
import SqlCsvSource
import SqlExecutionEngine
import System.Directory
    ( createDirectory
    , getTemporaryDirectory
    , removeFile
    , removePathForcibly
    )
import System.FilePath ((</>))
import System.IO (hClose, openTempFile)
import Test.Hspec

spec :: Spec
spec = do
    describe "metadata" $
        it "reports the shared package version" $
            version `shouldBe` "0.1.0"

    describe "coerceCsvValue" $ do
        it "coerces empty and case-insensitive boolean fields" $ do
            coerceCsvValue "" `shouldBe` SqlNull
            coerceCsvValue "true" `shouldBe` SqlBool True
            coerceCsvValue "TRUE" `shouldBe` SqlBool True
            coerceCsvValue "False" `shouldBe` SqlBool False

        it "coerces integer and finite real fields" $ do
            coerceCsvValue "42" `shouldBe` SqlInteger 42
            coerceCsvValue "-7" `shouldBe` SqlInteger (-7)
            coerceCsvValue "0" `shouldBe` SqlInteger 0
            coerceCsvValue "3.14" `shouldBe` SqlReal 3.14

        it "preserves non-numeric and non-finite text" $ do
            coerceCsvValue "hello" `shouldBe` SqlText "hello"
            coerceCsvValue "Alice Smith" `shouldBe` SqlText "Alice Smith"
            coerceCsvValue "NaN" `shouldBe` SqlText "NaN"
            coerceCsvValue "Infinity" `shouldBe` SqlText "Infinity"

    describe "parseCsv" $ do
        it "parses headers and typed rows" $ do
            (columns, rows) <- expectRight (parseCsv
                "id,name,active,score\n1,Alice,true,9.5\n2,Bob,false,8")
            columns `shouldBe` ["id", "name", "active", "score"]
            rows `shouldBe`
                [ row
                    [ "id" .= SqlInteger 1
                    , "name" .= SqlText "Alice"
                    , "active" .= SqlBool True
                    , "score" .= SqlReal 9.5
                    ]
                , row
                    [ "id" .= SqlInteger 2
                    , "name" .= SqlText "Bob"
                    , "active" .= SqlBool False
                    , "score" .= SqlInteger 8
                    ]
                ]

        it "supports quoted commas, escaped quotes, embedded newlines, and CRLF" $ do
            (columns, rows) <- expectRight (parseCsv
                "id,display_name,note\r\n1,\"Ada, Jr.\",\"hello \"\"world\"\"\"\r\n2,Bob,\"two\nlines\"\r\n")
            columns `shouldBe` ["id", "display_name", "note"]
            Map.lookup "display_name" (head rows) `shouldBe`
                Just (SqlText "Ada, Jr.")
            Map.lookup "note" (head rows) `shouldBe`
                Just (SqlText "hello \"world\"")
            Map.lookup "note" (last rows) `shouldBe`
                Just (SqlText "two\nlines")

        it "accepts empty and header-only documents" $ do
            parseCsv "" `shouldBe` Right ([], [])
            parseCsv "\n" `shouldBe` Right ([], [])
            parseCsv "id,name\n" `shouldBe` Right (["id", "name"], [])

        it "rejects malformed quotes and row widths" $ do
            parseCsv "id,name\n1,\"Alice" `shouldSatisfy` isLeftContaining "unclosed quoted"
            parseCsv "id,name\n1" `shouldSatisfy` isLeftContaining "has 1 fields; expected 2"
            parseCsv "id,\"na\"me\n" `shouldSatisfy` isLeftContaining "after closing CSV quote"

        it "rejects empty and duplicate header names" $ do
            parseCsv "id,,name\n1,2,Alice" `shouldSatisfy`
                isLeftContaining "empty column name"
            parseCsv "id,name,id\n1,Alice,2" `shouldSatisfy`
                isLeftContaining "duplicate column names"

    describe "filesystem snapshot" $
        around withFixtureDirectory $ do
            it "loads CSV files and ignores other extensions" $ \directory -> do
                source <- loadSource directory
                csvDirectory source `shouldBe` directory
                csvSchema source "employees" `shouldBe`
                    Right ["id", "name", "dept_id", "salary", "active"]
                fmap length (csvScan source "employees") `shouldBe` Right 4
                csvSchema source "notes" `shouldSatisfy`
                    isLeftContaining "table not found: notes"

            it "scans rows with typed values and SQL NULL" $ \directory -> do
                source <- loadSource directory
                rows <- expectRight (csvScan source "employees")
                Map.lookup "id" (head rows) `shouldBe` Just (SqlInteger 1)
                Map.lookup "name" (head rows) `shouldBe` Just (SqlText "Alice")
                Map.lookup "salary" (head rows) `shouldBe` Just (SqlInteger 90000)
                Map.lookup "active" (head rows) `shouldBe` Just (SqlBool True)
                Map.lookup "dept_id" (last rows) `shouldBe` Just SqlNull

            it "reports missing tables through both source callbacks" $ \directory -> do
                source <- loadSource directory
                csvSchema source "ghosts" `shouldSatisfy`
                    isLeftContaining "table not found: ghosts"
                csvScan source "ghosts" `shouldSatisfy`
                    isLeftContaining "table not found: ghosts"

            it "executes filtered and ordered projections" $ \directory -> do
                result <- executeRight
                    "SELECT name, salary FROM employees WHERE active = true AND salary > 70000 ORDER BY salary DESC"
                    directory
                resultColumns result `shouldBe` ["name", "salary"]
                resultRows result `shouldBe`
                    [ [SqlText "Alice", SqlInteger 90000]
                    , [SqlText "Bob", SqlInteger 75000]
                    ]

            it "supports null predicates" $ \directory -> do
                result <- executeRight
                    "SELECT name FROM employees WHERE dept_id IS NULL"
                    directory
                resultRows result `shouldBe` [[SqlText "Dave"]]

            it "supports joins across CSV tables" $ \directory -> do
                result <- executeRight
                    ("SELECT e.name AS employee, d.name AS department "
                        ++ "FROM employees AS e INNER JOIN departments AS d "
                        ++ "ON e.dept_id = d.id ORDER BY e.id")
                    directory
                resultColumns result `shouldBe` ["employee", "department"]
                resultRows result `shouldBe`
                    [ [SqlText "Alice", SqlText "Engineering"]
                    , [SqlText "Bob", SqlText "Marketing"]
                    , [SqlText "Carol", SqlText "Engineering"]
                    ]

            it "supports grouping, aggregates, and limits" $ \directory -> do
                result <- executeRight
                    ("SELECT dept_id, COUNT(*) AS count FROM employees "
                        ++ "WHERE dept_id IS NOT NULL GROUP BY dept_id "
                        ++ "ORDER BY dept_id LIMIT 2")
                    directory
                resultColumns result `shouldBe` ["dept_id", "count"]
                resultRows result `shouldBe`
                    [ [SqlInteger 1, SqlInteger 2]
                    , [SqlInteger 2, SqlInteger 1]
                    ]

            it "returns total execution results for success and failure" $ \directory -> do
                success <- tryExecuteCsv "SELECT name FROM employees LIMIT 1" directory
                executionOk success `shouldBe` True
                executionResult success `shouldSatisfy` maybe False (const True)
                failure <- tryExecuteCsv "SELECT * FROM ghosts" directory
                executionOk failure `shouldBe` False
                executionError failure `shouldSatisfy`
                    maybe False ("table not found: ghosts" `contains`)

    describe "loading errors" $ do
        it "reports a missing directory" $ do
            result <- loadCsvDataSource "this-directory-does-not-exist-for-sql-csv-source"
            result `shouldSatisfy` isLeftContaining "reading CSV directory"

        it "adds table context to malformed CSV errors" $
            withFixtureDirectory $ \directory -> do
                TextIO.writeFile (directory </> "broken.csv")
                    (Text.pack "id,name\n1,\"unclosed")
                result <- loadCsvDataSource directory
                result `shouldSatisfy`
                    isLeftContaining "parsing CSV table broken"

loadSource :: FilePath -> IO CsvDataSource
loadSource directory = loadCsvDataSource directory >>= expectRight

executeRight :: String -> FilePath -> IO QueryResult
executeRight sql directory = executeCsv sql directory >>= expectRight

withFixtureDirectory :: ActionWith FilePath -> IO ()
withFixtureDirectory = bracket makeFixtureDirectory removePathForcibly

makeFixtureDirectory :: IO FilePath
makeFixtureDirectory = do
    temporaryRoot <- getTemporaryDirectory
    (directory, handle) <- openTempFile temporaryRoot "sql-csv-source-"
    hClose handle
    removeFile directory
    createDirectory directory
    TextIO.writeFile (directory </> "employees.csv") (Text.pack employeesCsv)
    TextIO.writeFile (directory </> "departments.csv") (Text.pack departmentsCsv)
    TextIO.writeFile (directory </> "notes.txt") (Text.pack "not a table")
    pure directory

employeesCsv :: String
employeesCsv = unlines
    [ "id,name,dept_id,salary,active"
    , "1,Alice,1,90000,true"
    , "2,Bob,2,75000,true"
    , "3,Carol,1,95000,false"
    , "4,Dave,,60000,true"
    ]

departmentsCsv :: String
departmentsCsv = unlines
    [ "id,name,budget"
    , "1,Engineering,500000"
    , "2,Marketing,200000"
    ]

(.=) :: String -> SqlValue -> (String, SqlValue)
(.=) = (,)

row :: [(String, SqlValue)] -> Row
row = Map.fromList

expectRight :: Show error => Either error value -> IO value
expectRight (Right value) = pure value
expectRight (Left failure) =
    expectationFailure ("expected Right, got Left: " ++ show failure)
        >> error "unreachable"

isLeftContaining :: String -> Either SqlExecutionError value -> Bool
isLeftContaining needle (Left failure) = needle `contains` sqlExecutionMessage failure
isLeftContaining _ (Right _) = False

contains :: String -> String -> Bool
contains needle haystack = any (needle `prefixOf`) (tails haystack)
  where
    tails [] = [[]]
    tails text@(_ : rest) = text : tails rest
    prefixOf [] _ = True
    prefixOf _ [] = False
    prefixOf (left : leftRest) (right : rightRest) =
        left == right && prefixOf leftRest rightRest
