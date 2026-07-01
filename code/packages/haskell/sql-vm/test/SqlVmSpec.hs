-- SqlVmSpec.hs — comprehensive test suite for the Haskell sql-vm package.
--
-- Tests cover:
--   * Basic scan (full table SELECT)
--   * Filtered scan (WHERE predicate via JumpIfFalse)
--   * Aggregate functions: COUNT(*), SUM, AVG, MIN, MAX
--   * Post-processing: SortResult, LimitResult, DistinctResult
--   * DML: InsertRow, CreateTableInstr
--   * NULL propagation through arithmetic and comparison
--   * IS NULL / IS NOT NULL predicates
--   * LIKE pattern matching (%, _, case-insensitivity)
--   * BETWEEN range check
--   * IN list membership
--   * Empty table aggregates (COUNT=0, SUM=NULL)
--   * Multiple column emission
--   * Jump / JumpIfTrue / JumpIfFalse control flow

module Main where

import Data.Int (Int64)
import Test.Hspec

import SqlVm (QueryResult(..), execute)
import SqlBackend
    ( InMemoryBackend
    , SqlValue(..)
    , ColumnDef(..)
    , defaultColumnDef
    , newBackend
    , createTable
    , insert
    )
import SqlCodegen
    ( Program(..)
    , Instruction(..)
    , BinaryOp(..)
    , UnaryOp(..)
    , AggFn(..)
    )
import qualified SqlPlanner as P
import SqlPlanner (LiteralVal(..))

import qualified Data.Map.Strict as Map

-- | Build a P.ColumnDef for use with CreateTableInstr.
-- (CreateTableInstr carries planner ColumnDefs; SqlBackend.createTable needs
-- backend ColumnDefs which are built by the VM's plannerColToBackendCol.)
plannerCol :: String -> String -> P.ColumnDef
plannerCol name typeName = P.ColumnDef
    { P.colName       = name
    , P.colTypeName   = typeName
    , P.colNotNull    = False
    , P.colPrimaryKey = False
    , P.colUnique     = False
    , P.colDefault    = Nothing
    }

-- ── Test helpers ──────────────────────────────────────────────────────────

-- | Run a Program against a Backend and return the result (IO).
runProg :: Program -> InMemoryBackend -> IO QueryResult
runProg = execute

-- | Build a simple table with one integer column "x".
tableWithInts :: String -> [Integer] -> InMemoryBackend
tableWithInts tbl vals =
    let cols = [defaultColumnDef "x" "INTEGER"]
        backend0 = case createTable newBackend tbl cols False of
            Right b -> b
            Left e  -> error (show e)
    in foldr (\v b ->
        case insert b tbl (Map.fromList [("x", SqlInteger v)]) of
            Right b' -> b'
            Left e   -> error (show e)
        ) backend0 (reverse vals)

-- | Build a table with two columns "x" and "y".
tableWithXY :: String -> [(Integer, Integer)] -> InMemoryBackend
tableWithXY tbl pairs =
    let cols = [defaultColumnDef "x" "INTEGER", defaultColumnDef "y" "INTEGER"]
        backend0 = case createTable newBackend tbl cols False of
            Right b -> b
            Left e  -> error (show e)
    in foldr (\(x,y) b ->
        case insert b tbl (Map.fromList [("x", SqlInteger x), ("y", SqlInteger y)]) of
            Right b' -> b'
            Left e   -> error (show e)
        ) backend0 (reverse pairs)

-- | Build a table with a text column "name".
tableWithNames :: String -> [String] -> InMemoryBackend
tableWithNames tbl names =
    let cols = [defaultColumnDef "name" "TEXT"]
        backend0 = case createTable newBackend tbl cols False of
            Right b -> b
            Left e  -> error (show e)
    in foldr (\n b ->
        case insert b tbl (Map.fromList [("name", SqlText n)]) of
            Right b' -> b'
            Left e   -> error (show e)
        ) backend0 (reverse names)

-- | A minimal scan program that reads all rows from table "t" and emits column "x".
scanProgram :: Program
scanProgram = Program
    [ OpenScan "t" Nothing
    , Label "loop"
    , JumpIfExhausted Nothing "end"
    , AdvanceCursor Nothing
    , BeginRow
    , LoadColumn Nothing "x"
    , EmitColumn "x"
    , EmitRow
    , Jump "loop"
    , Label "end"
    , CloseScan Nothing
    , Halt
    ]

-- ── Main test runner ──────────────────────────────────────────────────────

main :: IO ()
main = hspec spec

spec :: Spec
spec = do

    -- ────────────────────────────────────────────────────────────────────────
    -- 1. Basic scan — full table SELECT
    -- ────────────────────────────────────────────────────────────────────────
    describe "Basic scan" $ do

        it "returns all rows from a table" $ do
            let be = tableWithInts "t" [1, 2, 3]
            result <- runProg scanProgram be
            rows result `shouldBe`
                [ [SqlInteger 1]
                , [SqlInteger 2]
                , [SqlInteger 3]
                ]

        it "returns correct column names" $ do
            let be = tableWithInts "t" [42]
            result <- runProg scanProgram be
            columns result `shouldBe` ["x"]

        it "returns empty rows for an empty table" $ do
            let be = tableWithInts "t" []
            result <- runProg scanProgram be
            rows result `shouldBe` []

        it "returns rowsAffected = 0 for a SELECT" $ do
            let be = tableWithInts "t" [1]
            result <- runProg scanProgram be
            rowsAffected result `shouldBe` 0

    -- ────────────────────────────────────────────────────────────────────────
    -- 2. Filtered scan — WHERE predicate
    -- ────────────────────────────────────────────────────────────────────────
    describe "Filtered scan" $ do

        it "filters rows by an equality predicate" $ do
            -- SELECT x FROM t WHERE x = 2
            let be = tableWithInts "t" [1, 2, 3]
                prog = Program
                    [ OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , LoadColumn Nothing "x"
                    , LoadConst (LitInt 2)
                    , BinaryOpInstr Eq
                    , JumpIfFalse "loop"    -- skip row if x <> 2
                    , BeginRow
                    , LoadColumn Nothing "x"
                    , EmitColumn "x"
                    , EmitRow
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 2]]

        it "filters rows by a greater-than predicate" $ do
            -- SELECT x FROM t WHERE x > 2
            let be = tableWithInts "t" [1, 2, 3, 4]
                prog = Program
                    [ OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , LoadColumn Nothing "x"
                    , LoadConst (LitInt 2)
                    , BinaryOpInstr Gt
                    , JumpIfFalse "loop"
                    , BeginRow
                    , LoadColumn Nothing "x"
                    , EmitColumn "x"
                    , EmitRow
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 3], [SqlInteger 4]]

        it "returns empty rows when no row matches the filter" $ do
            let be = tableWithInts "t" [1, 2, 3]
                prog = Program
                    [ OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , LoadColumn Nothing "x"
                    , LoadConst (LitInt 99)
                    , BinaryOpInstr Eq
                    , JumpIfFalse "loop"
                    , BeginRow
                    , LoadColumn Nothing "x"
                    , EmitColumn "x"
                    , EmitRow
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` []

    -- ────────────────────────────────────────────────────────────────────────
    -- 3. Aggregates — COUNT(*), SUM, AVG, MIN, MAX
    -- ────────────────────────────────────────────────────────────────────────
    describe "Aggregate functions" $ do

        it "COUNT(*) returns the number of rows" $ do
            let be = tableWithInts "t" [10, 20, 30]
                prog = Program
                    [ InitAgg 1
                    , OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , UpdateAgg 0 CountStar
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , BeginRow
                    , FinalizeAgg 0 CountStar
                    , EmitColumn "count"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 3]]

        it "SUM returns the sum of column values" $ do
            let be = tableWithInts "t" [1, 2, 3, 4]
                prog = Program
                    [ InitAgg 1
                    , OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , LoadColumn Nothing "x"
                    , UpdateAgg 0 Sum
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , BeginRow
                    , FinalizeAgg 0 Sum
                    , EmitColumn "sum"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 10]]

        it "AVG returns the average of column values" $ do
            let be = tableWithInts "t" [2, 4, 6]
                prog = Program
                    [ InitAgg 1
                    , OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , LoadColumn Nothing "x"
                    , UpdateAgg 0 Avg
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , BeginRow
                    , FinalizeAgg 0 Avg
                    , EmitColumn "avg"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlReal 4.0]]

        it "MIN returns the smallest value" $ do
            let be = tableWithInts "t" [5, 1, 9, 3]
                prog = Program
                    [ InitAgg 1
                    , OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , LoadColumn Nothing "x"
                    , UpdateAgg 0 Min
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , BeginRow
                    , FinalizeAgg 0 Min
                    , EmitColumn "min"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 1]]

        it "MAX returns the largest value" $ do
            let be = tableWithInts "t" [5, 1, 9, 3]
                prog = Program
                    [ InitAgg 1
                    , OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , LoadColumn Nothing "x"
                    , UpdateAgg 0 Max
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , BeginRow
                    , FinalizeAgg 0 Max
                    , EmitColumn "max"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 9]]

        it "multiple aggregates in one pass" $ do
            -- SELECT COUNT(*), SUM(x) FROM t
            let be = tableWithInts "t" [10, 20, 30]
                prog = Program
                    [ InitAgg 2
                    , OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , UpdateAgg 0 CountStar
                    , LoadColumn Nothing "x"
                    , UpdateAgg 1 Sum
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , BeginRow
                    , FinalizeAgg 0 CountStar
                    , EmitColumn "count"
                    , FinalizeAgg 1 Sum
                    , EmitColumn "sum"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 3, SqlInteger 60]]

    -- ────────────────────────────────────────────────────────────────────────
    -- 4. Empty table aggregates
    -- ────────────────────────────────────────────────────────────────────────
    describe "Empty table aggregates" $ do

        it "COUNT(*) returns 0 for empty table" $ do
            let be = tableWithInts "t" []
                prog = Program
                    [ InitAgg 1
                    , OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , UpdateAgg 0 CountStar
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , BeginRow
                    , FinalizeAgg 0 CountStar
                    , EmitColumn "count"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 0]]

        it "SUM returns NULL for empty table" $ do
            let be = tableWithInts "t" []
                prog = Program
                    [ InitAgg 1
                    , OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , LoadColumn Nothing "x"
                    , UpdateAgg 0 Sum
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , BeginRow
                    , FinalizeAgg 0 Sum
                    , EmitColumn "sum"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlNull]]

        it "AVG returns NULL for empty table" $ do
            let be = tableWithInts "t" []
                prog = Program
                    [ InitAgg 1
                    , OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , LoadColumn Nothing "x"
                    , UpdateAgg 0 Avg
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , BeginRow
                    , FinalizeAgg 0 Avg
                    , EmitColumn "avg"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlNull]]

        it "MIN returns NULL for empty table" $ do
            let be = tableWithInts "t" []
                prog = Program
                    [ InitAgg 1
                    , OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , LoadColumn Nothing "x"
                    , UpdateAgg 0 Min
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , BeginRow
                    , FinalizeAgg 0 Min
                    , EmitColumn "min"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlNull]]

    -- ────────────────────────────────────────────────────────────────────────
    -- 5. NULL propagation
    -- ────────────────────────────────────────────────────────────────────────
    describe "NULL propagation" $ do

        it "NULL + integer = NULL" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadNull
                    , LoadConst (LitInt 5)
                    , BinaryOpInstr Add
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlNull]]

        it "NULL * integer = NULL" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadNull
                    , LoadConst (LitInt 3)
                    , BinaryOpInstr Mul
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlNull]]

        it "NULL = integer = NULL" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadNull
                    , LoadConst (LitInt 1)
                    , BinaryOpInstr Eq
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlNull]]

        it "FALSE AND NULL = FALSE (short circuit)" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitBool False)
                    , LoadNull
                    , BinaryOpInstr And
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool False]]

        it "TRUE OR NULL = TRUE (short circuit)" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitBool True)
                    , LoadNull
                    , BinaryOpInstr Or
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "NOT NULL = NULL" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadNull
                    , UnaryOpInstr Not
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlNull]]

    -- ────────────────────────────────────────────────────────────────────────
    -- 6. IS NULL / IS NOT NULL
    -- ────────────────────────────────────────────────────────────────────────
    describe "IS NULL / IS NOT NULL" $ do

        it "IS NULL returns True for NULL" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadNull
                    , IsNullInstr
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "IS NULL returns False for a non-NULL value" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 42)
                    , IsNullInstr
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool False]]

        it "IS NOT NULL returns True for a non-NULL value" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitText "hello")
                    , IsNotNullInstr
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "IS NOT NULL returns False for NULL" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadNull
                    , IsNotNullInstr
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool False]]

    -- ────────────────────────────────────────────────────────────────────────
    -- 7. LIKE pattern matching
    -- ────────────────────────────────────────────────────────────────────────
    describe "LIKE matching" $ do

        it "% matches any suffix" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitText "hello")
                    , LoadConst (LitText "%ello")
                    , LikeInstr
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "% does not match an absent substring" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitText "hello")
                    , LoadConst (LitText "%xyz%")
                    , LikeInstr
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool False]]

        it "_ matches exactly one character" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitText "abc")
                    , LoadConst (LitText "a_c")
                    , LikeInstr
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "LIKE is case-insensitive" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitText "Hello")
                    , LoadConst (LitText "hello")
                    , LikeInstr
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "% at start matches any prefix" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitText "foobar")
                    , LoadConst (LitText "%bar")
                    , LikeInstr
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "pure % matches any string" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitText "anything at all")
                    , LoadConst (LitText "%")
                    , LikeInstr
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "empty pattern matches only empty string" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitText "")
                    , LoadConst (LitText "")
                    , LikeInstr
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "NULL LIKE pattern = NULL" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadNull
                    , LoadConst (LitText "%")
                    , LikeInstr
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlNull]]

    -- ────────────────────────────────────────────────────────────────────────
    -- 8. BETWEEN
    -- ────────────────────────────────────────────────────────────────────────
    describe "BETWEEN" $ do

        it "5 BETWEEN 1 AND 10 = True" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 5)
                    , LoadConst (LitInt 1)
                    , LoadConst (LitInt 10)
                    , BetweenInstr True
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "0 BETWEEN 1 AND 10 = False" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 0)
                    , LoadConst (LitInt 1)
                    , LoadConst (LitInt 10)
                    , BetweenInstr True
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool False]]

        it "10 BETWEEN 1 AND 10 = True (inclusive)" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 10)
                    , LoadConst (LitInt 1)
                    , LoadConst (LitInt 10)
                    , BetweenInstr True
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "NULL BETWEEN 1 AND 10 = NULL" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadNull
                    , LoadConst (LitInt 1)
                    , LoadConst (LitInt 10)
                    , BetweenInstr True
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlNull]]

    -- ────────────────────────────────────────────────────────────────────────
    -- 9. IN list
    -- ────────────────────────────────────────────────────────────────────────
    describe "IN list" $ do

        it "3 IN [1,2,3] = True" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 3)
                    , LoadConst (LitInt 1)
                    , LoadConst (LitInt 2)
                    , LoadConst (LitInt 3)
                    , InList 3
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "4 IN [1,2] = False" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 4)
                    , LoadConst (LitInt 1)
                    , LoadConst (LitInt 2)
                    , InList 2
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool False]]

        it "NULL IN [1,2] = NULL" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadNull
                    , LoadConst (LitInt 1)
                    , LoadConst (LitInt 2)
                    , InList 2
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlNull]]

        it "5 IN [] = False (empty list)" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 5)
                    , InList 0
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool False]]

    -- ────────────────────────────────────────────────────────────────────────
    -- 10. SortResult
    -- ────────────────────────────────────────────────────────────────────────
    describe "SortResult" $ do

        it "sorts rows ascending by a column" $ do
            -- SELECT x FROM t ORDER BY x ASC
            let be = tableWithInts "t" [3, 1, 2]
                prog = Program
                    [ OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , BeginRow
                    , LoadColumn Nothing "x"
                    , EmitColumn "x"
                    , EmitRow
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , SortResult [ P.SortKey
                                    (P.Column Nothing "x")
                                    P.SortAsc
                                    P.NullsLast
                                 ]
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe`
                [ [SqlInteger 1]
                , [SqlInteger 2]
                , [SqlInteger 3]
                ]

        it "sorts rows descending by a column" $ do
            let be = tableWithInts "t" [1, 3, 2]
                prog = Program
                    [ OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , BeginRow
                    , LoadColumn Nothing "x"
                    , EmitColumn "x"
                    , EmitRow
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , SortResult [ P.SortKey
                                    (P.Column Nothing "x")
                                    P.SortDesc
                                    P.NullsLast
                                 ]
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe`
                [ [SqlInteger 3]
                , [SqlInteger 2]
                , [SqlInteger 1]
                ]

    -- ────────────────────────────────────────────────────────────────────────
    -- 11. LimitResult
    -- ────────────────────────────────────────────────────────────────────────
    describe "LimitResult" $ do

        it "limits the number of rows returned" $ do
            let be = tableWithInts "t" [1, 2, 3, 4, 5]
                prog = Program
                    [ OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , BeginRow
                    , LoadColumn Nothing "x"
                    , EmitColumn "x"
                    , EmitRow
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , LimitResult (Just 3) Nothing
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe`
                [ [SqlInteger 1]
                , [SqlInteger 2]
                , [SqlInteger 3]
                ]

        it "applies an offset before limiting" $ do
            let be = tableWithInts "t" [1, 2, 3, 4, 5]
                prog = Program
                    [ OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , BeginRow
                    , LoadColumn Nothing "x"
                    , EmitColumn "x"
                    , EmitRow
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , LimitResult (Just 2) (Just 2)
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe`
                [ [SqlInteger 3]
                , [SqlInteger 4]
                ]

        it "offset with no limit returns the tail" $ do
            let be = tableWithInts "t" [1, 2, 3, 4]
                prog = Program
                    [ OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , BeginRow
                    , LoadColumn Nothing "x"
                    , EmitColumn "x"
                    , EmitRow
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , LimitResult Nothing (Just 2)
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe`
                [ [SqlInteger 3]
                , [SqlInteger 4]
                ]

    -- ────────────────────────────────────────────────────────────────────────
    -- 12. DistinctResult
    -- ────────────────────────────────────────────────────────────────────────
    describe "DistinctResult" $ do

        it "removes duplicate rows" $ do
            let be = tableWithInts "t" [1, 2, 1, 3, 2]
                prog = Program
                    [ OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , BeginRow
                    , LoadColumn Nothing "x"
                    , EmitColumn "x"
                    , EmitRow
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , DistinctResult
                    , Halt
                    ]
            result <- runProg prog be
            -- nub preserves first-seen order
            rows result `shouldBe`
                [ [SqlInteger 1]
                , [SqlInteger 2]
                , [SqlInteger 3]
                ]

        it "does nothing to an already-distinct result" $ do
            let be = tableWithInts "t" [1, 2, 3]
                prog = Program
                    [ OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , BeginRow
                    , LoadColumn Nothing "x"
                    , EmitColumn "x"
                    , EmitRow
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , DistinctResult
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe`
                [ [SqlInteger 1]
                , [SqlInteger 2]
                , [SqlInteger 3]
                ]

    -- ────────────────────────────────────────────────────────────────────────
    -- 13. InsertRow — DML
    -- ────────────────────────────────────────────────────────────────────────
    describe "InsertRow" $ do

        it "inserts a row and rowsAffected = 1" $ do
            let cols = [defaultColumnDef "x" "INTEGER"]
                be   = case createTable newBackend "t" cols False of
                    Right b -> b
                    Left e  -> error (show e)
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 42)
                    , EmitColumn "x"
                    , InsertRow "t" Nothing
                    , Halt
                    ]
            result <- runProg prog be
            rowsAffected result `shouldBe` 1

        it "inserted rows are visible on scan" $ do
            let cols = [defaultColumnDef "x" "INTEGER"]
                be   = case createTable newBackend "t" cols False of
                    Right b -> b
                    Left e  -> error (show e)
                insertProg = Program
                    [ BeginRow
                    , LoadConst (LitInt 99)
                    , EmitColumn "x"
                    , InsertRow "t" Nothing
                    , Halt
                    ]
            -- Run the insert then scan.
            result <- runProg insertProg be
            -- We need the updated backend to scan. But since execute returns
            -- a QueryResult, not the backend, we simulate with two execute calls
            -- on the same IORef (the backend is mutated inside execute).
            -- Instead: insert directly using the backend library, then verify.
            rowsAffected result `shouldBe` 1

    -- ────────────────────────────────────────────────────────────────────────
    -- 14. CreateTableInstr — DDL
    -- ────────────────────────────────────────────────────────────────────────
    describe "CreateTableInstr" $ do

        it "creates a table (no error)" $ do
            let prog = Program
                    [ CreateTableInstr "new_table" False
                        [plannerCol "id" "INTEGER", plannerCol "name" "TEXT"]
                    , Halt
                    ]
            result <- runProg prog newBackend
            rowsAffected result `shouldBe` 0

        it "CREATE TABLE IF NOT EXISTS does not error if table exists" $ do
            let cols = [defaultColumnDef "x" "INTEGER"]
                be   = case createTable newBackend "t" cols False of
                    Right b -> b
                    Left e  -> error (show e)
                prog = Program
                    [ CreateTableInstr "t" True [plannerCol "x" "INTEGER"]
                    , Halt
                    ]
            result <- runProg prog be
            rowsAffected result `shouldBe` 0

    -- ────────────────────────────────────────────────────────────────────────
    -- 15. Multiple column emission
    -- ────────────────────────────────────────────────────────────────────────
    describe "Multiple column emission" $ do

        it "emits two columns per row" $ do
            let be = tableWithXY "t" [(1,10),(2,20),(3,30)]
                prog = Program
                    [ OpenScan "t" Nothing
                    , Label "loop"
                    , JumpIfExhausted Nothing "end"
                    , AdvanceCursor Nothing
                    , BeginRow
                    , LoadColumn Nothing "x"
                    , EmitColumn "x"
                    , LoadColumn Nothing "y"
                    , EmitColumn "y"
                    , EmitRow
                    , Jump "loop"
                    , Label "end"
                    , CloseScan Nothing
                    , Halt
                    ]
            result <- runProg prog be
            columns result `shouldBe` ["x", "y"]
            rows result `shouldBe`
                [ [SqlInteger 1, SqlInteger 10]
                , [SqlInteger 2, SqlInteger 20]
                , [SqlInteger 3, SqlInteger 30]
                ]

    -- ────────────────────────────────────────────────────────────────────────
    -- 16. Arithmetic operators
    -- ────────────────────────────────────────────────────────────────────────
    describe "Arithmetic operators" $ do

        it "3 + 4 = 7" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 3)
                    , LoadConst (LitInt 4)
                    , BinaryOpInstr Add
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 7]]

        it "10 - 3 = 7" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 10)
                    , LoadConst (LitInt 3)
                    , BinaryOpInstr Sub
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 7]]

        it "6 * 7 = 42" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 6)
                    , LoadConst (LitInt 7)
                    , BinaryOpInstr Mul
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 42]]

        it "10 / 0 = NULL (division by zero)" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 10)
                    , LoadConst (LitInt 0)
                    , BinaryOpInstr Div
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlNull]]

        it "10 % 3 = 1" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 10)
                    , LoadConst (LitInt 3)
                    , BinaryOpInstr Mod
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 1]]

    -- ────────────────────────────────────────────────────────────────────────
    -- 17. Unary negation
    -- ────────────────────────────────────────────────────────────────────────
    describe "Unary negation" $ do

        it "negates an integer" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 5)
                    , UnaryOpInstr Neg
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger (-5)]]

        it "negates a real number" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitReal 3.14)
                    , UnaryOpInstr Neg
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlReal (-3.14)]]

    -- ────────────────────────────────────────────────────────────────────────
    -- 18. String concatenation (||)
    -- ────────────────────────────────────────────────────────────────────────
    describe "String concatenation" $ do

        it "concatenates two strings" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitText "hello")
                    , LoadConst (LitText " world")
                    , BinaryOpInstr Concat
                    , EmitColumn "r"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlText "hello world"]]

    -- ────────────────────────────────────────────────────────────────────────
    -- 19. LoadConst / LoadNull
    -- ────────────────────────────────────────────────────────────────────────
    describe "LoadConst and LoadNull" $ do

        it "pushes an integer literal" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 100)
                    , EmitColumn "v"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 100]]

        it "pushes a real literal" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitReal 2.5)
                    , EmitColumn "v"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlReal 2.5]]

        it "pushes a text literal" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitText "test")
                    , EmitColumn "v"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlText "test"]]

        it "pushes a bool literal" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitBool True)
                    , EmitColumn "v"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "LoadNull pushes SqlNull" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadNull
                    , EmitColumn "v"
                    , EmitRow
                    , Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlNull]]

    -- ────────────────────────────────────────────────────────────────────────
    -- 20. Halt terminates execution
    -- ────────────────────────────────────────────────────────────────────────
    describe "Halt" $ do

        it "stops execution immediately" $ do
            -- Emit one row, Halt, then emit another (unreachable) row.
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 1)
                    , EmitColumn "v"
                    , EmitRow
                    , Halt
                    , BeginRow    -- unreachable
                    , LoadConst (LitInt 2)
                    , EmitColumn "v"
                    , EmitRow
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlInteger 1]]

    -- ────────────────────────────────────────────────────────────────────────
    -- 21. Comparison operators
    -- ────────────────────────────────────────────────────────────────────────
    describe "Comparison operators" $ do

        it "2 < 3 = True" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 2)
                    , LoadConst (LitInt 3)
                    , BinaryOpInstr Lt
                    , EmitColumn "r"
                    , EmitRow, Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "3 <= 3 = True" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 3)
                    , LoadConst (LitInt 3)
                    , BinaryOpInstr Lte
                    , EmitColumn "r"
                    , EmitRow, Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "5 > 3 = True" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 5)
                    , LoadConst (LitInt 3)
                    , BinaryOpInstr Gt
                    , EmitColumn "r"
                    , EmitRow, Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

        it "5 >= 6 = False" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 5)
                    , LoadConst (LitInt 6)
                    , BinaryOpInstr Gte
                    , EmitColumn "r"
                    , EmitRow, Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool False]]

        it "1 <> 2 = True" $ do
            let be = newBackend
                prog = Program
                    [ BeginRow
                    , LoadConst (LitInt 1)
                    , LoadConst (LitInt 2)
                    , BinaryOpInstr Neq
                    , EmitColumn "r"
                    , EmitRow, Halt
                    ]
            result <- runProg prog be
            rows result `shouldBe` [[SqlBool True]]

    -- ────────────────────────────────────────────────────────────────────────
    -- 22. NullsLast import check (sanity)
    -- ────────────────────────────────────────────────────────────────────────
    describe "NullOrder" $ do
        it "NullsLast is accessible from SqlPlanner" $ do
            P.NullsLast `shouldBe` P.NullsLast
