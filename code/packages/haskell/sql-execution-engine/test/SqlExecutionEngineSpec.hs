module SqlExecutionEngineSpec (spec) where

import qualified Data.Map.Strict as Map
import SqlExecutionEngine
import Test.Hspec

spec :: Spec
spec = describe "SqlExecutionEngine" $ do
    it "scans in-memory tables" $ do
        let source = sampleSource
        dataSourceSchema source "employees" `shouldBe` Right ["id", "name", "dept", "salary", "active"]
        fmap length (dataSourceScan source "employees") `shouldBe` Right 5
        dataSourceSchema source "missing" `shouldSatisfy` isLeft

    it "selects and filters rows" $ do
        result <- expectRight (execute "SELECT name, salary FROM employees WHERE active = true AND salary >= 70000 ORDER BY salary DESC" sampleSource)
        resultColumns result `shouldBe` ["name", "salary"]
        take 2 (resultRows result) `shouldBe`
            [ [SqlText "Alice", SqlInteger 95000]
            , [SqlText "Bob", SqlInteger 72000]
            ]

    it "supports null predicates and LIKE" $ do
        nullResult <- expectRight (execute "SELECT name FROM employees WHERE dept IS NULL" sampleSource)
        resultRows nullResult `shouldBe` [[SqlText "Dave"]]

        likeResult <- expectRight (execute "SELECT name FROM employees WHERE name LIKE 'A%'" sampleSource)
        resultRows likeResult `shouldBe` [[SqlText "Alice"]]

    it "supports joins" $ do
        result <- expectRight (execute "SELECT e.name, d.budget FROM employees AS e INNER JOIN departments AS d ON e.dept = d.dept ORDER BY e.id" sampleSource)
        resultColumns result `shouldBe` ["name", "budget"]
        length (resultRows result) `shouldBe` 4
        head (resultRows result) `shouldBe` [SqlText "Alice", SqlInteger 500000]
        last (resultRows result) `shouldBe` [SqlText "Eve", SqlInteger 150000]

    it "supports grouping and aggregates" $ do
        result <- expectRight (execute "SELECT dept, COUNT(*) AS cnt, SUM(salary) AS total FROM employees WHERE dept IS NOT NULL GROUP BY dept HAVING COUNT(*) >= 1 ORDER BY dept" sampleSource)
        resultColumns result `shouldBe` ["dept", "cnt", "total"]
        resultRows result `shouldBe`
            [ [SqlText "Engineering", SqlInteger 2, SqlReal 183000.0]
            , [SqlText "HR", SqlInteger 1, SqlReal 70000.0]
            , [SqlText "Marketing", SqlInteger 1, SqlReal 72000.0]
            ]

    it "supports distinct limit and offset" $ do
        result <- expectRight (execute "SELECT DISTINCT dept FROM employees WHERE dept IS NOT NULL ORDER BY dept LIMIT 2 OFFSET 1" sampleSource)
        resultColumns result `shouldBe` ["dept"]
        resultRows result `shouldBe` [[SqlText "HR"], [SqlText "Marketing"]]

    it "reports errors through tryExecute" $ do
        let result = tryExecute "SELECT * FROM ghosts" sampleSource
        executionOk result `shouldBe` False
        executionError result `shouldSatisfy` maybe False (not . null)

    it "select star uses bare columns" $ do
        result <- expectRight (execute "SELECT * FROM employees WHERE id = 1" sampleSource)
        resultColumns result `shouldBe` ["active", "dept", "id", "name", "salary"]
        resultRows result `shouldBe` [[SqlBool True, SqlText "Engineering", SqlInteger 1, SqlText "Alice", SqlInteger 95000]]

sampleSource :: DataSource
sampleSource =
    inMemoryDataSource $
        addTable
            "departments"
            ["dept", "budget"]
            [ row ["dept" .= SqlText "Engineering", "budget" .= SqlInteger 500000]
            , row ["dept" .= SqlText "Marketing", "budget" .= SqlInteger 200000]
            , row ["dept" .= SqlText "HR", "budget" .= SqlInteger 150000]
            ]
            (addTable
                "employees"
                ["id", "name", "dept", "salary", "active"]
                [ row ["id" .= SqlInteger 1, "name" .= SqlText "Alice", "dept" .= SqlText "Engineering", "salary" .= SqlInteger 95000, "active" .= SqlBool True]
                , row ["id" .= SqlInteger 2, "name" .= SqlText "Bob", "dept" .= SqlText "Marketing", "salary" .= SqlInteger 72000, "active" .= SqlBool True]
                , row ["id" .= SqlInteger 3, "name" .= SqlText "Carol", "dept" .= SqlText "Engineering", "salary" .= SqlInteger 88000, "active" .= SqlBool False]
                , row ["id" .= SqlInteger 4, "name" .= SqlText "Dave", "dept" .= SqlNull, "salary" .= SqlInteger 60000, "active" .= SqlBool True]
                , row ["id" .= SqlInteger 5, "name" .= SqlText "Eve", "dept" .= SqlText "HR", "salary" .= SqlInteger 70000, "active" .= SqlBool False]
                ]
                emptyInMemoryDataSource)

(.=) :: String -> SqlValue -> (String, SqlValue)
(.=) = (,)

row :: [(String, SqlValue)] -> Row
row = Map.fromList

isLeft :: Either a b -> Bool
isLeft (Left _) = True
isLeft (Right _) = False

expectRight :: (Show err) => Either err a -> IO a
expectRight result =
    case result of
        Left failure -> expectationFailure ("expected Right, got Left: " ++ show failure) >> error "unreachable"
        Right value -> pure value
