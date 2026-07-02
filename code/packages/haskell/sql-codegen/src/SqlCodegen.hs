-- SqlCodegen.hs — bytecode code generator for the Mini-SQLite Level 1 pipeline.
--
-- This module transforms an OptimizedPlan (produced by sql-optimizer) into a
-- flat list of stack-machine instructions (Program) that the sql-vm can execute.
--
-- ┌─────────────────────────────────────────────────────────────────────────┐
-- │  PIPELINE POSITION                                                      │
-- │                                                                         │
-- │  sql-lexer → sql-parser → sql-planner → sql-optimizer → [sql-codegen] │
-- │           → sql-vm → mini-sqlite                                        │
-- │                                                                         │
-- │  Input : OptimizedPlan (from the optimizer)                             │
-- │  Output: Program — a flat list of Instruction values                   │
-- └─────────────────────────────────────────────────────────────────────────┘
--
-- ── What is a stack machine? ──────────────────────────────────────────────
--
-- A stack machine is the simplest possible virtual computer. It has no named
-- registers — just a single stack of values and a sequence of instructions.
-- Each instruction pops zero, one, or two values from the top of the stack,
-- does some work, and pushes a result back. For example:
--
--   PUSH 3          stack: [3]
--   PUSH 4          stack: [3, 4]
--   ADD             stack: [7]   (popped 3 and 4, pushed their sum)
--
-- SQL expressions compile to a straight-line sequence of such instructions.
-- The expression `a + (b * 2)` compiles to:
--
--   LoadColumn Nothing "a"    ← push the value of column a
--   LoadColumn Nothing "b"    ← push b
--   LoadConst (LitInt 2)      ← push literal 2
--   BinaryOpInstr Mul         ← pop 2 and b, push b*2
--   BinaryOpInstr Add         ← pop b*2 and a, push a + b*2
--
-- ── Why stack machines for SQL? ───────────────────────────────────────────
--
-- SQLite's own query engine (VDBE), the JVM, and CPython all use stack
-- machines for the same reason: they're easy to generate code for, easy to
-- execute in a tight interpreter loop, and require no register allocation.
--
-- ── Two-phase aggregate compilation ──────────────────────────────────────
--
-- Aggregates (COUNT, SUM, …) need two phases:
--   1. ACCUMULATE — scan every row, feeding values into accumulators.
--   2. FINALIZE   — after the scan, read each accumulator and emit a row.
--
-- The codegen handles this by emitting InitAgg before the loop, UpdateAgg
-- inside the loop, and FinalizeAgg after the loop.
--
-- ── Label-based control flow ──────────────────────────────────────────────
--
-- Loops and branches use named labels as jump targets. A Label instruction is
-- a no-op marker; Jump/JumpIfFalse/JumpIfTrue transfer control to it. The VM
-- resolves labels to instruction indices at runtime in O(1).
--
-- Example scan loop for `SELECT name FROM users`:
--
--   OpenScan "users" Nothing          ← open iterator on the table
--   Label "loop_0"                    ← top of loop
--   JumpIfExhausted Nothing "end_0"   ← exit when no more rows
--   AdvanceCursor Nothing             ← move to next row
--   BeginRow                          ← start assembling output row
--   LoadColumn Nothing "name"         ← push name value
--   EmitColumn "name"                 ← store it in the row buffer
--   EmitRow                           ← flush row to result
--   Jump "loop_0"                     ← back to top of loop
--   Label "end_0"                     ← loop exit point
--   CloseScan Nothing                 ← release cursor
--   Halt                              ← stop execution
--
-- ── Name-collision strategy ───────────────────────────────────────────────
--
-- SqlPlanner.SqlExpr has constructors named IsNull, IsNotNull, Between, Like
-- which would clash with the Instruction constructors of the same name.
-- To avoid ambiguity, SqlPlanner is imported QUALIFIED as `P`, and SqlExpr
-- constructors are accessed as P.Literal, P.Column, P.IsNull, etc.
-- Non-conflicting planner types (LiteralVal, BinaryOperator, etc.) are still
-- imported unqualified for convenience.

module SqlCodegen
    ( -- * Supporting operator types
      BinaryOp(..)
    , UnaryOp(..)
    , AggFn(..)
      -- * Instruction ADT
    , Instruction(..)
      -- * Program (compiled output)
    , Program(..)
      -- * Public API
    , compile
    , compileExpr
    ) where

import Data.Int (Int64)

-- SqlPlanner is imported BOTH qualified (as P) and unqualified (for non-conflicting types).
-- The qualified import gives us P.IsNull, P.Between, etc. for SqlExpr constructors.
-- The unqualified import gives us LiteralVal(..), AggFunction(..), etc. directly.
import qualified SqlPlanner as P
import SqlPlanner
    ( LiteralVal(..)
    , BinaryOperator(..)
    , UnaryOperator(..)
    , AggFunction(..)
    , AggArg(..)
    , SortKey(..)
    , AggregateItem(..)
    , OutputColumn(..)
    , ColumnDef(..)
    , SqlExpr
    )

import SqlOptimizer
    ( OptimizedPlan(..)
    )

-- ── Supporting operator types ─────────────────────────────────────────────
--
-- These mirror the planner's BinaryOperator / UnaryOperator / AggFunction,
-- but live in the codegen namespace so the VM layer does not need to import
-- the planner directly. The compiler maps between the two below.

-- | Binary infix operators for arithmetic, comparison, and logic.
--
-- Truth table for AND (three-valued SQL logic):
--
--   T AND T = T,  T AND F = F,  T AND N = N
--   F AND T = F,  F AND F = F,  F AND N = F   ← NULL short-circuits to FALSE
--   N AND T = N,  N AND F = F,  N AND N = N
--
-- Concat is SQL's || string-concatenation operator.
data BinaryOp
    = Add    -- ^ a + b
    | Sub    -- ^ a - b
    | Mul    -- ^ a * b
    | Div    -- ^ a / b (integer: truncates; real: floating point)
    | Mod    -- ^ a % b (remainder)
    | Eq     -- ^ a = b  (SQL equality; NULL = anything = NULL)
    | Neq    -- ^ a <> b
    | Lt     -- ^ a < b
    | Lte    -- ^ a <= b
    | Gt     -- ^ a > b
    | Gte    -- ^ a >= b
    | And    -- ^ a AND b (short-circuit: FALSE AND NULL = FALSE)
    | Or     -- ^ a OR  b (short-circuit: TRUE  OR  NULL = TRUE)
    | Concat -- ^ a || b  (string concatenation)
    deriving (Show, Eq)

-- | Unary prefix operators.
--
-- NOT uses three-valued logic: NOT NULL = NULL.
-- Neg negates a numeric value: Neg (LitInt 5) = LitInt (-5).
data UnaryOp
    = Neg -- ^ -a  (arithmetic negation)
    | Not -- ^ NOT a (logical negation, three-valued: NOT NULL = NULL)
    deriving (Show, Eq)

-- | Aggregate function kinds.
--
-- Think of aggregates like running tallies:
--   COUNT         — counts non-NULL values in a column
--   COUNT*        — counts all rows including NULLs
--   CountDistinct — counts UNIQUE non-NULL values (uses a Set internally)
--   SUM    — running sum, ignoring NULLs
--   AVG    — average: sum / count (both over non-NULLs)
--   MIN    — smallest non-NULL value seen
--   MAX    — largest non-NULL value seen
data AggFn
    = Count         -- ^ COUNT(expr) — count non-NULL values
    | CountStar     -- ^ COUNT(*)    — count all rows including NULLs
    | CountDistinct -- ^ COUNT(DISTINCT expr) — count unique non-NULL values
    | Sum           -- ^ SUM(expr)   — sum all non-NULL values
    | Avg           -- ^ AVG(expr)   — arithmetic mean of non-NULL values
    | Min           -- ^ MIN(expr)   — smallest non-NULL value
    | Max           -- ^ MAX(expr)   — largest non-NULL value
    deriving (Show, Eq)

-- ── Instruction ADT ───────────────────────────────────────────────────────
--
-- Every VM operation is one constructor of this type. Instructions are pure
-- data: no functions, no side effects — just a description of what to do.
-- The VM interprets them in a loop, maintaining the stack and cursors.
--
-- Notation used in comments:
--   stack: [top, next, ...]  (top is the first element)
--   pop X  — remove the top item and bind it to X
--   push V — put V on top

-- | A single stack-machine instruction for the Mini-SQLite VM.
data Instruction
    -- ── Stack / memory operations ─────────────────────────────────────────
    --
    -- These push or pop values on the expression evaluation stack.

    -- | Push a compile-time constant onto the stack.
    --   Example: the literal `42` in `WHERE age > 42` becomes LoadConst (LitInt 42).
    = LoadConst LiteralVal

    -- | Push a SQL NULL constant onto the stack.
    --   NULL is the "unknown" value in SQL's three-valued logic.
    | LoadNull

    -- | Push the value of a named column from the current row.
    --   The first field is the optional table alias (Nothing = search all cursors).
    --   Example: column reference `u.name` → LoadColumn (Just "u") "name".
    | LoadColumn (Maybe String) String

    -- | Push a runtime-bound parameter (placeholder ? in a prepared statement).
    --   index is the 0-based position in the binding list.
    | LoadParam Int

    -- | Push the i-th value from the current group-by key snapshot.
    --   Used in the group-emit phase so GROUP BY column values are available
    --   after the scan has advanced past the last row.
    | LoadGroupKey Int

    -- | Discard the top value of the stack.
    --   Used when an expression's result is not needed.
    | Pop

    -- ── Arithmetic and comparison ─────────────────────────────────────────
    --
    -- These pop TWO values and push ONE result.
    -- The right operand is popped first (pushed last), then the left.

    -- | Pop right and left operands; apply the binary operator; push the result.
    --   Covers arithmetic (+, -, *, /, %), comparison (=, <>, <, <=, >, >=),
    --   logic (AND, OR), and string concatenation (||).
    | BinaryOpInstr BinaryOp

    -- ── Unary operations ──────────────────────────────────────────────────

    -- | Pop one value; apply the unary operator; push the result.
    | UnaryOpInstr UnaryOp

    -- ── Predicate / test instructions ─────────────────────────────────────
    --
    -- SQL has NULL as a first-class value, so every test must handle three
    -- outcomes: TRUE, FALSE, and NULL (unknown).

    -- | Pop value; push True if it is SQL NULL, False otherwise.
    --   Note: `NULL = NULL` is NULL, but `NULL IS NULL` is TRUE — these are
    --   semantically different and require distinct instructions.
    | IsNullInstr

    -- | Pop value; push True if it is NOT NULL, False if it is NULL.
    | IsNotNullInstr

    -- | Pop hi, lo, value; push True if lo <= value <= hi.
    --   The Bool field controls inclusive vs exclusive bounds
    --   (True = inclusive, as SQL BETWEEN requires).
    | BetweenInstr Bool

    -- | Pop pattern, value; push True if value LIKE pattern.
    --   SQL LIKE uses % for "zero or more chars" and _ for "exactly one char".
    | LikeInstr

    -- | Pop `count` items as the list, then pop the needle;
    --   push True if the needle is in the list (SQL IN operator).
    | InList Int

    -- ── Scan / cursor control ─────────────────────────────────────────────
    --
    -- A cursor is an iterator over a table's rows. Think of it like a file
    -- handle: you open it, read rows one at a time, and close it when done.
    -- Multiple cursors can be open simultaneously (for JOINs).

    -- | Open an iterator (cursor) for the named table.
    --   The second field is the query alias, used to distinguish cursors in JOINs.
    | OpenScan String (Maybe String)

    -- | Move the cursor to its next row.
    | AdvanceCursor (Maybe String)

    -- | Jump to the named label if the cursor is exhausted (no more rows).
    | JumpIfExhausted (Maybe String) String

    -- | Release the cursor, freeing any resources it holds.
    | CloseScan (Maybe String)

    -- ── Row construction ──────────────────────────────────────────────────
    --
    -- Output rows are assembled instruction by instruction.
    -- BeginRow clears the row buffer; EmitColumn stores one field;
    -- EmitRow flushes the assembled row to the result set.

    -- | Clear the row buffer and start assembling a new output row.
    | BeginRow

    -- | Pop the top of the stack and store it as the named column in the row buffer.
    | EmitColumn String

    -- | Finalize the row buffer and append the assembled row to the result set.
    | EmitRow

    -- ── Aggregation ───────────────────────────────────────────────────────
    --
    -- Aggregation is a two-phase process: accumulate during the scan loop,
    -- then finalize and emit after the scan completes.
    --
    -- Example for `SELECT COUNT(*), SUM(price) FROM orders`:
    --
    --   InitAgg 2                   ← create 2 accumulators: [count=0, sum=0]
    --   ...scan loop...
    --     UpdateAgg 0 CountStar     ← accumulator 0: increment count
    --     LoadColumn Nothing "price" ← push price
    --     UpdateAgg 1 Sum           ← accumulator 1: add price to sum
    --   ...end loop...
    --   BeginRow
    --   FinalizeAgg 0 CountStar     ← push finalized count
    --   EmitColumn "count(*)"
    --   FinalizeAgg 1 Sum           ← push finalized sum
    --   EmitColumn "sum(price)"
    --   EmitRow

    -- | Initialize `count` aggregate accumulators to their zero states.
    | InitAgg Int

    -- | Pop the top of the stack and feed it into accumulator at index.
    --   The AggFn tells the accumulator how to combine the new value.
    | UpdateAgg Int AggFn

    -- | Compute the final value of accumulator at index and push it.
    --   For AVG this divides sum by count; for COUNT it returns the count.
    | FinalizeAgg Int AggFn

    -- | Save the current group-by key values for use in the emit phase.
    --   The list of strings names the columns that form the GROUP BY key.
    | SaveGroupKey [String]

    -- | Advance the group iterator to the next group.
    | AdvanceGroup

    -- | Jump to the named label if all groups have been emitted.
    --   Used to terminate the group-emit loop after GROUP BY.
    | JumpIfGroupsDone String

    -- ── Control flow ──────────────────────────────────────────────────────
    --
    -- Jump targets are named strings during code generation. At runtime the
    -- VM resolves them to instruction indices for O(1) dispatch.

    -- | A no-op marker that names a position in the instruction stream.
    --   Jump instructions refer to labels by name.
    | Label String

    -- | Unconditional jump to the named label.
    | Jump String

    -- | Pop value; jump to the named label if the value is TRUE.
    | JumpIfTrue String

    -- | Pop value; jump to the named label if the value is FALSE or NULL.
    | JumpIfFalse String

    -- | Stop execution; the result set holds the output rows.
    | Halt

    -- ── DDL (Data Definition Language) ───────────────────────────────────
    --
    -- CREATE TABLE and DROP TABLE produce a single instruction each.
    -- No scan loop needed — these are schema operations, not row operations.

    -- | Ask the VM/backend to create a table with the given columns.
    --   The Bool field mirrors CREATE TABLE IF NOT EXISTS.
    | CreateTableInstr String Bool [ColumnDef]

    -- | Ask the VM/backend to drop a table.
    --   The Bool field mirrors DROP TABLE IF EXISTS.
    | DropTableInstr String Bool

    -- ── DML (Data Manipulation Language) ─────────────────────────────────

    -- | Insert one row into the table.
    --   The values for the optional column list are on the stack in order.
    | InsertRow String (Maybe [String])

    -- | Update the row currently under the cursor for the named table.
    | UpdateRows String

    -- | Delete the row currently under the cursor for the named table.
    | DeleteRows String

    -- ── Transaction control ───────────────────────────────────────────────

    | BeginTransaction
    | CommitTransaction
    | RollbackTransaction

    -- ── Result post-processing ────────────────────────────────────────────
    --
    -- After the scan loop fills the result buffer, these instructions apply
    -- sorting, deduplication, and pagination as post-processing steps.
    -- They are emitted AFTER the scan loop closes, not inside it.

    -- | Sort the result buffer by the given sort keys.
    | SortResult [SortKey]

    -- | Deduplicate the result buffer, keeping only distinct rows.
    | DistinctResult

    -- | Keep at most `count` rows starting at `offset` (0-based).
    --   Nothing means "no limit" or "no offset" respectively.
    | LimitResult (Maybe Int64) (Maybe Int64)

    -- | Call a built-in scalar function with `arity` arguments already on the
    --   stack (rightmost argument was pushed last).  Pop the arguments, apply
    --   the function, and push the result.
    --   Examples: LENGTH(s) → CallBuiltin "length" 1
    --             SUBSTR(s,p,n) → CallBuiltin "substr" 3
    | CallBuiltin String Int

    deriving (Show, Eq)

-- ── Program — the compiled output ────────────────────────────────────────
--
-- A Program is simply a flat list of Instructions. The VM executes them in
-- order, jumping when it encounters Jump/JumpIfTrue/JumpIfFalse/AdvanceCursor.
--
-- Why a list? It's the simplest representation and sufficient for Level 1.
-- A future optimizer could convert to an array for O(1) index access.

-- | The compiled output of the code generator.
newtype Program = Program { instructions :: [Instruction] }
    deriving (Show, Eq)

-- ── Label counter ─────────────────────────────────────────────────────────
--
-- A counter produces unique suffixes like "0", "1", "2" for label names.
-- Each scan gets its own loop/end label pair so nested scans don't clash.
--
-- Example: a JOIN produces two scan pairs:
--   loop_0 / end_0  for the outer (left) table
--   loop_1 / end_1  for the inner (right) table
--
-- We thread the counter through all compilation functions as a plain Int,
-- returning (instructions, newCounter) so the approach is purely functional.
-- This avoids the complexity of IORef or State monad for a Level 1 compiler.

type Counter = Int

freshLabel :: Counter -> (String, Counter)
freshLabel n = (show n, n + 1)

-- ── Operator mapping ──────────────────────────────────────────────────────
--
-- The planner uses its own operator types (BinaryOperator, UnaryOperator,
-- AggFunction). We map them to the codegen's operator types here.
-- This keeps the VM layer decoupled from the planner's type hierarchy.

-- | Map a planner BinaryOperator to a codegen BinaryOp.
mapBinaryOp :: BinaryOperator -> BinaryOp
mapBinaryOp BinAdd   = Add
mapBinaryOp BinSub   = Sub
mapBinaryOp BinMul   = Mul
mapBinaryOp BinDiv   = Div
mapBinaryOp BinMod   = Mod
mapBinaryOp BinEq    = Eq
mapBinaryOp BinNotEq = Neq
mapBinaryOp BinLt    = Lt
mapBinaryOp BinLte   = Lte
mapBinaryOp BinGt    = Gt
mapBinaryOp BinGte   = Gte
mapBinaryOp BinAnd   = And
mapBinaryOp BinOr    = Or

-- | Map a planner UnaryOperator to a codegen UnaryOp.
mapUnaryOp :: UnaryOperator -> UnaryOp
mapUnaryOp UnaryNeg = Neg
mapUnaryOp UnaryNot = Not

-- | Map a planner AggFunction to a codegen AggFn.
--   COUNT is the column-expression version; CountStar is COUNT(*).
mapAggFn :: AggFunction -> AggFn
mapAggFn AggCount = Count
mapAggFn AggSum   = Sum
mapAggFn AggAvg   = Avg
mapAggFn AggMin   = Min
mapAggFn AggMax   = Max

-- ── Expression compiler ───────────────────────────────────────────────────
--
-- `compileExpr` translates a SqlExpr tree into a flat sequence of
-- Instructions that, when executed by the VM, leaves exactly one value on
-- the stack. Recursive calls handle sub-expressions bottom-up (post-order).
--
-- The function is exported so tests can verify individual expression cases
-- without having to compile a whole plan.
--
-- IMPORTANT: SqlExpr constructors are accessed via the `P.` qualified prefix
-- (e.g. P.Literal, P.IsNull) to avoid name collision with the Instruction ADT
-- constructors IsNullInstr, BetweenInstr, etc. in this same module.

-- | Compile a SQL expression to a flat instruction sequence.
--
-- Each call pushes exactly one value onto the stack. Compound expressions
-- recursively push sub-values before the operator instruction pops them.
--
-- Examples:
--   P.Literal (Just (LitInt 42))       → [LoadConst (LitInt 42)]
--   P.Column (Just "u") "name"         → [LoadColumn (Just "u") "name"]
--   P.BinaryOp BinAdd a b              → compileExpr a ++ compileExpr b ++ [BinaryOpInstr Add]
compileExpr :: SqlExpr -> [Instruction]
compileExpr expr = case expr of

    -- ── Literals ─────────────────────────────────────────────────────────
    -- A literal is simply pushed as a constant. NULL becomes LoadNull.
    P.Literal Nothing  -> [LoadNull]
    P.Literal (Just v) -> [LoadConst v]

    -- ── Column references ─────────────────────────────────────────────────
    -- The table qualifier is preserved so the VM can distinguish columns
    -- from different tables in a JOIN (e.g. u.id vs. o.id).
    P.Column tOpt col -> [LoadColumn tOpt col]

    -- ── Binary operators ──────────────────────────────────────────────────
    -- Compile left, then right (left on stack first), then apply the
    -- operator. The VM pops right first (it's on top), then left.
    P.BinaryOp op l r ->
        compileExpr l ++ compileExpr r ++ [BinaryOpInstr (mapBinaryOp op)]

    -- ── Unary operators ───────────────────────────────────────────────────
    -- Compile the operand, then apply the unary operator.
    P.UnaryOp op e ->
        compileExpr e ++ [UnaryOpInstr (mapUnaryOp op)]

    -- ── NULL tests ────────────────────────────────────────────────────────
    -- IS NULL / IS NOT NULL push a boolean (never NULL) regardless of
    -- whether the operand is NULL. These differ from `= NULL` which is NULL.
    P.IsNull e ->
        compileExpr e ++ [IsNullInstr]

    P.IsNotNull e ->
        compileExpr e ++ [IsNotNullInstr]

    -- ── BETWEEN ───────────────────────────────────────────────────────────
    -- `value BETWEEN lo AND hi` → push value, lo, hi, then BetweenInstr True.
    -- The VM pops all three and pushes TRUE/FALSE/NULL.
    P.Between v lo hi ->
        compileExpr v ++ compileExpr lo ++ compileExpr hi ++ [BetweenInstr True]

    -- ── LIKE ──────────────────────────────────────────────────────────────
    -- Push value, push pattern as a constant, then LikeInstr.
    -- The pattern is a compile-time string literal in this Level 1 compiler.
    P.Like v pat ->
        compileExpr v ++ [LoadConst (LitText pat)] ++ [LikeInstr]

    -- ── NOT LIKE ─────────────────────────────────────────────────────────
    -- Compile as LIKE then negate the result.
    P.NotLike v pat ->
        compileExpr v ++ [LoadConst (LitText pat)] ++ [LikeInstr] ++ [UnaryOpInstr Not]

    -- ── IN (list) ────────────────────────────────────────────────────────
    -- Push the needle (value to test), push each list item, then InList n.
    -- The VM pops n items and the needle and pushes TRUE/FALSE/NULL.
    P.InExpr v items ->
        compileExpr v ++ concatMap compileExpr items ++ [InList (length items)]

    -- ── NOT IN ───────────────────────────────────────────────────────────
    -- Compile as IN then negate.
    -- Note: NOT IN has special NULL semantics (x NOT IN (...NULL...) = NULL),
    -- handled by the VM's InList + NOT combination.
    P.NotInExpr v items ->
        compileExpr v ++ concatMap compileExpr items
            ++ [InList (length items)] ++ [UnaryOpInstr Not]

    -- ── Aggregate expressions ─────────────────────────────────────────────
    -- Aggregates within expressions are handled at the plan level
    -- (compileAggregateQuery knows the slot index). In non-aggregate context,
    -- emit NULL as a safe fallback so the expression still type-checks.
    P.AggExpr _ _ _ ->
        [LoadNull]

    -- ── Function calls ────────────────────────────────────────────────────
    -- Compile each argument onto the stack, then emit a CallBuiltin
    -- instruction naming the function and its arity.  The VM dispatches to
    -- a table of built-in scalar functions (LENGTH, UPPER, LOWER, …).
    -- Unknown function names produce NULL at runtime (the VM falls through).
    P.FuncCall name args ->
        concatMap compileExpr args ++ [CallBuiltin name (length args)]

    -- ── Wildcard ─────────────────────────────────────────────────────────
    -- SELECT * is handled at the plan level; bare Wildcard pushes NULL.
    P.Wildcard ->
        [LoadNull]

-- ── Aggregate helpers ─────────────────────────────────────────────────────

-- | Compile the accumulate (update) phase for one aggregate slot.
--
-- COUNT(*)             → just increment; no column load needed.
-- COUNT(DISTINCT expr) → load value; VM uses a Set to track unique non-NULLs.
-- COUNT(expr)          → load the value (NULL check done by VM), then UpdateAgg.
-- SUM/AVG/MIN/MAX → load the expression value, then UpdateAgg.
--
-- The fourth argument is the distinct flag from the AggregateItem.
compileUpdateAgg :: Int -> AggFunction -> AggArg -> Bool -> [Instruction]
compileUpdateAgg idx AggCount AggStar  _     = [UpdateAgg idx CountStar]
compileUpdateAgg idx AggCount (AggExprArg e) True  = compileExpr e ++ [UpdateAgg idx CountDistinct]
compileUpdateAgg idx AggCount (AggExprArg e) False = compileExpr e ++ [UpdateAgg idx Count]
compileUpdateAgg idx AggSum   (AggExprArg e) _     = compileExpr e ++ [UpdateAgg idx Sum]
compileUpdateAgg idx AggAvg   (AggExprArg e) _     = compileExpr e ++ [UpdateAgg idx Avg]
compileUpdateAgg idx AggMin   (AggExprArg e) _     = compileExpr e ++ [UpdateAgg idx Min]
compileUpdateAgg idx AggMax   (AggExprArg e) _     = compileExpr e ++ [UpdateAgg idx Max]
compileUpdateAgg idx _        _              _     = [UpdateAgg idx CountStar]

-- | Compile the finalize (emit) phase for one aggregate slot.
-- For COUNT(DISTINCT), we look up the distinct-flag on the AggregateItem
-- and emit FinalizeAgg with CountDistinct so the VM returns the set size.
compileFinalizeAgg :: Int -> AggFunction -> Instruction
compileFinalizeAgg idx AggCount = FinalizeAgg idx Count
compileFinalizeAgg idx AggSum   = FinalizeAgg idx Sum
compileFinalizeAgg idx AggAvg   = FinalizeAgg idx Avg
compileFinalizeAgg idx AggMin   = FinalizeAgg idx Min
compileFinalizeAgg idx AggMax   = FinalizeAgg idx Max

-- | Variant of compileFinalizeAgg that respects the distinct flag.
compileFinalizeAggItem :: Int -> AggregateItem -> Instruction
compileFinalizeAggItem idx a
    | aggFunc a == AggCount && aggDistinct a = FinalizeAgg idx CountDistinct
    | otherwise                              = compileFinalizeAgg idx (aggFunc a)

-- | Derive a display name for an output column.
--
-- Used as the EmitColumn argument. Priority:
--   explicit alias > bare column name > aggregate function name > "expr"
outputColName :: OutputColumn -> String
outputColName OutputStar = "*"
outputColName (OutputExpr _ (Just alias)) = alias
outputColName (OutputExpr (P.Column _ col) Nothing) = col
outputColName (OutputExpr (P.AggExpr fn _ _) Nothing) =
    case fn of
        AggCount -> "count"
        AggSum   -> "sum"
        AggAvg   -> "avg"
        AggMin   -> "min"
        AggMax   -> "max"
outputColName (OutputExpr _ Nothing) = "expr"

-- ── Scan loop codegen ─────────────────────────────────────────────────────
--
-- The fundamental pattern for any table scan is:
--
--   OpenScan table alias
--   Label "loop_N"
--   JumpIfExhausted alias "end_N"
--   AdvanceCursor alias
--   <body>
--   Jump "loop_N"
--   Label "end_N"
--   CloseScan alias
--
-- The `body` parameter is a list of instructions spliced inside the loop.

-- | Compile a table scan loop, wrapping the given body instructions.
--
-- The cursor alias must be consistent with how the planner resolves column
-- references.  The planner's `resolveColumn` assigns each column the alias
-- `seAlias`, which is the explicit AS-alias when present, and the bare table
-- name otherwise (see `refToEntry` in SqlPlanner).  So `LoadColumn (Just tbl)
-- col` refers to the cursor opened for table `tbl`.
--
-- When the optimizer passes `alias = Nothing` we therefore normalise it to
-- `Just table`, making the cursor key equal to the table name — the same
-- string that the planner used when it emitted `Column (Just table) col`.
-- Without this, `OpenScan table Nothing` stores the row under key `""` while
-- `LoadColumn (Just table) col` looks it up under key `table`, resulting in a
-- miss and `SqlNull` for every column.
--
-- Returns (instructions, newCounter).
compileScanLoop :: String -> Maybe String -> [Instruction] -> Counter
                -> ([Instruction], Counter)
compileScanLoop table alias body counter =
    let (n, counter1) = freshLabel counter
        loopLabel = "loop_" ++ n
        endLabel  = "end_"  ++ n
        -- Normalise: use table name as cursor key when no explicit alias given.
        effectiveAlias = Just (maybe table id alias)
        instrs =
            [ OpenScan table effectiveAlias
            , Label loopLabel
            , JumpIfExhausted effectiveAlias endLabel
            , AdvanceCursor effectiveAlias
            ]
            ++ body ++
            [ Jump loopLabel
            , Label endLabel
            , CloseScan effectiveAlias
            ]
    in (instrs, counter1)

-- ── Core plan compilation ─────────────────────────────────────────────────
--
-- `compilePlanCore` compiles the scan phase of a plan, inserting `body` as
-- the per-row loop body. It does NOT handle Sort/Limit/Distinct (those are
-- post-ops emitted after the loop). It threads the label counter through and
-- returns (instructions, newCounter).
--
-- The key insight: each node wraps or augments the body before passing it
-- inward. When we reach a Scan node, we have the complete body assembled
-- and can emit the full scan loop in one shot.

-- | Compile the scan phase of a plan node with the given per-row body.
compilePlanCore :: OptimizedPlan -> [Instruction] -> Counter
                -> ([Instruction], Counter)
compilePlanCore plan body counter = case plan of

    -- ── Base scan ─────────────────────────────────────────────────────────
    -- The leaf of most query trees — opens a cursor and iterates rows.
    OptScan table alias _ _ ->
        compileScanLoop table alias body counter

    -- ── Filter — compile the predicate as a guard inside the loop ─────────
    --
    -- The filter sits "between" the cursor advance and the row body.
    -- If the predicate is false/null, jump past the body for this row.
    --
    -- Structure:
    --   <compile inner scan>
    --     <predicate instructions>
    --     JumpIfFalse "skip_N"
    --     <body>
    --     Label "skip_N"
    --   <end inner scan>
    OptFilter inner pred ->
        let (n, counter1) = freshLabel counter
            skipLabel = "filter_skip_" ++ n
            guard = compileExpr pred ++ [JumpIfFalse skipLabel]
            newBody = guard ++ body ++ [Label skipLabel]
        in compilePlanCore inner newBody counter1

    -- ── Project — projection is expressed via EmitColumn in the body ──────
    -- The body already contains BeginRow + EmitColumn + EmitRow; we just
    -- recurse into the inner plan to provide the scan infrastructure.
    OptProject inner _ ->
        compilePlanCore inner body counter

    -- ── Join — nested loop over two tables ────────────────────────────────
    --
    -- A JOIN is a nested scan: for each row in the outer (left) table, scan
    -- all rows of the inner (right) table. The join condition (if any) acts
    -- as a filter inside the inner loop.
    --
    -- Outer loop → inner loop → [condition check] → body
    OptJoin left right _ condOpt ->
        case condOpt of
            Nothing ->
                -- No ON condition: cross join — emit body for every pair
                let (rightInstrs, counter1) = compilePlanCore right body counter
                in compilePlanCore left rightInstrs counter1
            Just cond ->
                let (n, counter1) = freshLabel counter
                    skipLabel = "join_skip_" ++ n
                    condGuard =
                        compileExpr cond
                        ++ [JumpIfFalse skipLabel]
                        ++ body
                        ++ [Label skipLabel]
                    (rightInstrs, counter2) = compilePlanCore right condGuard counter1
                in compilePlanCore left rightInstrs counter2

    -- ── Aggregate/Having — handled at the SELECT level ────────────────────
    -- If encountered here (nested), recurse into the inner plan.
    OptAggregate inner _ _ ->
        compilePlanCore inner body counter

    OptHaving inner _ ->
        compilePlanCore inner body counter

    -- ── Post-processing wrappers — recurse through them ───────────────────
    -- Sort, Limit, Distinct are applied after the scan; recurse inward.
    OptSort inner _ ->
        compilePlanCore inner body counter

    OptLimit inner _ _ ->
        compilePlanCore inner body counter

    OptDistinct inner ->
        compilePlanCore inner body counter

    OptUnion left _ _ ->
        compilePlanCore left body counter

    -- ── Empty result — proven zero rows ───────────────────────────────────
    EmptyResult ->
        ([], counter)

    -- ── DML/DDL fallthrough — no scan needed ──────────────────────────────
    _ ->
        (body, counter)

-- ── Aggregate query compilation ───────────────────────────────────────────
--
-- An aggregate query has two distinct phases:
--
--   PHASE 1 (accumulate): loop over all rows, updating accumulators
--   PHASE 2 (finalize):   after the loop, emit one row per group
--
-- For simple aggregates (no GROUP BY), there is exactly one group covering
-- all rows. For grouped aggregates, each unique combination of GROUP BY
-- column values forms a group.

-- | Compile an aggregate query.
--
-- The `projCols` argument is the outer Project's column list.  When a column
-- has a user-supplied alias (e.g. `SELECT COUNT(*) AS n`), the alias lives in
-- the OutputColumn, not in the AggregateItem (which only has `_agg0` etc.).
-- We use projCols to recover those aliases so EmitColumn uses the right name.
--
-- GROUP BY queries: for each unique combination of GROUP BY column values, the
-- VM accumulates into per-group slots.  After the scan, we emit a loop that
-- advances through each group in turn and emits one output row per group.
--
-- Returns (instructions, newCounter).
compileAggregateQuery :: OptimizedPlan    -- ^ inner scan plan (below the Aggregate node)
                      -> [AggregateItem]  -- ^ aggregate functions to compute
                      -> [SqlExpr]        -- ^ GROUP BY expressions
                      -> Maybe SqlExpr    -- ^ HAVING predicate (optional)
                      -> [OutputColumn]   -- ^ outer Project column list (for aliases)
                      -> Counter
                      -> ([Instruction], Counter)
compileAggregateQuery innerPlan aggs groupBy havingOpt projCols counter =
    let numAggs = length aggs

        -- Derive group-key column names for SaveGroupKey / LoadGroupKey.
        groupKeyNames =
            zipWith (\i e -> case e of
                P.Column _ c -> c
                _            -> "key_" ++ show i)
            [0..] groupBy

        -- Build a mapping from synthetic aggregate alias → user alias.
        -- projCols may have OutputExpr (AggExpr ...) (Just "alias") which
        -- contains the AS-alias the user wrote.  Zip with aggs by position.
        -- If projCols has fewer entries, fall back to the synthetic alias.
        userAliasFor :: Int -> String -> String
        userAliasFor i synth =
            case drop i projCols of
                (OutputExpr _ (Just userAlias) : _) -> userAlias
                _                                    -> synth

        -- The group-key columns come first in projCols, then the aggregates.
        aggProjOffset = length groupKeyNames

        -- Count how many aggregates appear in the SELECT list (projCols) vs.
        -- HAVING.  Aggregates collected from HAVING are extra slots that must
        -- NOT be emitted as output columns.
        numSelectAggs =
            length (filter isAggInProj projCols)
          where
            isAggInProj (OutputExpr (P.AggExpr _ _ _) _) = True
            isAggInProj _                                 = False

        -- PHASE 1: inside the loop — save the group key, then update each accumulator.
        saveKeyInstrs
            | null groupBy = []
            | otherwise    =
                concatMap compileExpr groupBy
                ++ [SaveGroupKey groupKeyNames]

        updateInstrs =
            concatMap (\(i, a) -> compileUpdateAgg i (aggFunc a) (aggArg a) (aggDistinct a))
                      (zip [0..] aggs)

        accumulateBody = saveKeyInstrs ++ updateInstrs

        -- Compile the scan loop with the accumulate body.
        (scanInstrs, counter1) = compilePlanCore innerPlan accumulateBody counter

        -- PHASE 2: after the scan — emit group-key columns then finalized aggregates.
        keyEmitInstrs =
            concatMap (\(i, name) -> [LoadGroupKey i, EmitColumn name])
                      (zip [0..] groupKeyNames)

        -- Only emit the SELECT-list aggregates (not HAVING-only aggregate slots).
        -- Use compileFinalizeAggItem to respect the distinct flag (CountDistinct).
        aggEmitInstrs =
            concatMap (\(i, a) ->
                let colName = userAliasFor (aggProjOffset + i) (aggAlias a)
                in [compileFinalizeAggItem i a, EmitColumn colName])
                (zip [0..] (take numSelectAggs aggs))

        -- Compile a HAVING predicate expression, substituting AggExpr nodes
        -- with the appropriate FinalizeAgg instruction (looking up the slot
        -- index from the `aggs` list).  This fixes the bug where
        -- compileExpr (AggExpr ...) would emit LoadNull, causing HAVING to
        -- always evaluate to NULL and skip all rows.
        compileHavingExpr :: SqlExpr -> [Instruction]
        compileHavingExpr expr = case expr of
            P.AggExpr fn arg _ ->
                -- Find the first slot in `aggs` whose function and argument
                -- match this AggExpr, and emit FinalizeAgg for that slot.
                case findAggSlot fn arg (zip [0..] aggs) of
                    Just (i, _) -> [compileFinalizeAgg i fn]
                    Nothing     -> [LoadNull]
            P.BinaryOp op l r ->
                compileHavingExpr l
                ++ compileHavingExpr r
                ++ [BinaryOpInstr (mapBinaryOp op)]
            P.UnaryOp op e ->
                compileHavingExpr e
                ++ [UnaryOpInstr (mapUnaryOp op)]
            other -> compileExpr other

        -- Look up the first accumulator slot whose function and argument match.
        findAggSlot :: AggFunction -> AggArg -> [(Int, AggregateItem)] -> Maybe (Int, AggregateItem)
        findAggSlot fn arg =
            foldr (\(i, a) acc ->
                case acc of
                    Just _ -> acc
                    Nothing ->
                        if aggFunc a == fn && aggArg a == arg
                            then Just (i, a)
                            else Nothing)
                Nothing

        -- PHASE 2 emit: single row (no GROUP BY) or loop per group (GROUP BY).
        (emitPhase, counter2)
            | null groupBy =
                -- Simple aggregates: emit exactly one row.
                case havingOpt of
                    Nothing ->
                        ( [BeginRow] ++ keyEmitInstrs ++ aggEmitInstrs ++ [EmitRow]
                        , counter1 )
                    Just pred ->
                        let (n, c1) = freshLabel counter1
                            skipLabel = "having_skip_" ++ n
                        in ( [BeginRow] ++ keyEmitInstrs ++ aggEmitInstrs
                                ++ compileHavingExpr pred
                                ++ [JumpIfFalse skipLabel, EmitRow, Label skipLabel]
                           , c1 )
            | otherwise =
                -- GROUP BY: loop over all accumulated groups and emit one row each.
                let (n, c1)     = freshLabel counter1
                    loopLabel   = "group_loop_" ++ n
                    doneLabel   = "group_done_" ++ n
                    rowInstrs = [BeginRow] ++ keyEmitInstrs ++ aggEmitInstrs
                    (emitBody, c2) = case havingOpt of
                        Nothing ->
                            ( rowInstrs ++ [EmitRow]
                            , c1 )
                        Just pred ->
                            let (m, c1') = freshLabel c1
                                skipLabel = "having_skip_" ++ m
                            in ( rowInstrs
                                    ++ compileHavingExpr pred
                                    ++ [JumpIfFalse skipLabel, EmitRow, Label skipLabel]
                               , c1' )
                in ( [ Label loopLabel
                     , AdvanceGroup
                     , JumpIfGroupsDone doneLabel
                     ]
                     ++ emitBody
                     ++ [ Jump loopLabel
                        , Label doneLabel
                        ]
                   , c2 )

    in ([InitAgg numAggs] ++ scanInstrs ++ emitPhase, counter2)

-- ── Output column compilation ─────────────────────────────────────────────

-- | Compile the projection phase for a non-aggregate SELECT.
--
-- For each output column:
--   * OutputStar   → push a "*" marker (VM expands to all columns)
--   * OutputExpr e → compile e, then EmitColumn with the derived name
compileOutputCols :: [OutputColumn] -> [Instruction]
compileOutputCols [] =
    -- No Project node — emit a wildcard marker
    [LoadConst (LitText "*")]
compileOutputCols cols = concatMap go cols
  where
    go OutputStar =
        [LoadConst (LitText "*")]
    go col@(OutputExpr expr _) =
        compileExpr expr ++ [EmitColumn (outputColName col)]

-- ── Peel post-processing wrappers ────────────────────────────────────────
--
-- Sort/Limit/Distinct are applied AFTER the scan loop completes, not inside
-- the loop body. We peel them off the top of the plan tree first, collect
-- their post-op instructions, then compile the core plan.

-- | Peel Sort/Limit/Distinct wrappers and collect their post-op instructions.
--
-- Returns (postOpInstructions, corePlan).
--
-- The planner wraps post-processing nodes (Sort, Limit, Distinct) INSIDE
-- a Project node — i.e. the tree is Project → Sort → Limit → ... → Scan.
-- We therefore also peel through OptProject so that Sort/Limit/Distinct
-- nested under a Project are correctly hoisted into postOps.  Without this
-- the sort key instructions are silently discarded (compilePlanCore recurses
-- through OptSort as a no-op) and the result comes out unsorted.
peelWrappers :: OptimizedPlan -> ([Instruction], OptimizedPlan)
peelWrappers (OptProject inner cols) =
    -- Peel through the projection wrapper so that Sort/Limit/Distinct nodes
    -- nested beneath a Project are hoisted into postOps.
    let (postOps, core) = peelWrappers inner
    in (postOps, OptProject core cols)
peelWrappers (OptSort inner keys) =
    let (postOps, core) = peelWrappers inner
    in (postOps ++ [SortResult keys], core)
peelWrappers (OptLimit inner cnt off) =
    let (postOps, core) = peelWrappers inner
        cnt64 = fmap fromIntegral cnt
        off64 = fmap fromIntegral off
    in (postOps ++ [LimitResult cnt64 off64], core)
peelWrappers (OptDistinct inner) =
    let (postOps, core) = peelWrappers inner
    in (postOps ++ [DistinctResult], core)
peelWrappers other =
    ([], other)

-- ── SELECT compilation ────────────────────────────────────────────────────
--
-- Pipeline for a SELECT query:
--   1. Peel Sort/Limit/Distinct post-ops
--   2. Detect aggregate vs non-aggregate
--   3. Emit the scan loop (with filter + projection inside)
--   4. Append post-ops + Halt

-- | Compile a SELECT-style plan (Project/Filter/Scan tree, possibly with
-- aggregation and post-processing wrappers).
--
-- Returns (instructions, newCounter).
compileSelect :: OptimizedPlan -> Counter -> ([Instruction], Counter)
compileSelect plan counter =
    let (postOps, corePlan) = peelWrappers plan
    in case findAggregate corePlan of

        -- ── Aggregate query ───────────────────────────────────────────────
        Just (innerPlan, groupBy, aggs, havingOpt, projCols) ->
            let (scanInstrs, c1) =
                    compileAggregateQuery innerPlan aggs groupBy havingOpt projCols counter
            in (scanInstrs ++ postOps ++ [Halt], c1)

        -- ── Non-aggregate query ───────────────────────────────────────────
        Nothing ->
            let (outputCols, innerPlan) = case corePlan of
                    OptProject inner cols -> (cols, inner)
                    other                -> ([], other)
                emitBody =
                    [BeginRow]
                    ++ compileOutputCols outputCols
                    ++ [EmitRow]
                (scanInstrs, c1) = compilePlanCore innerPlan emitBody counter
            in (scanInstrs ++ postOps ++ [Halt], c1)

-- | Search for an Aggregate node in the plan, possibly wrapped by Project/Having.
--
-- Returns Just (innerScanPlan, groupByExprs, aggItems, havingPred, projectCols)
-- where projectCols is the outer Project's column list (used to recover the
-- user-supplied column aliases for aggregate outputs such as
-- `SELECT COUNT(*) AS n` — the `AS n` lives in the Project, while the
-- AggregateItem only has the synthetic `_agg0` alias).
findAggregate :: OptimizedPlan
              -> Maybe (OptimizedPlan, [SqlExpr], [AggregateItem], Maybe SqlExpr, [OutputColumn])
findAggregate (OptProject (OptAggregate inner gb aggs) projCols) =
    Just (inner, gb, aggs, Nothing, projCols)
findAggregate (OptProject (OptHaving (OptAggregate inner gb aggs) pred) projCols) =
    Just (inner, gb, aggs, Just pred, projCols)
findAggregate (OptAggregate inner gb aggs) =
    Just (inner, gb, aggs, Nothing, [])
findAggregate (OptHaving (OptAggregate inner gb aggs) pred) =
    Just (inner, gb, aggs, Just pred, [])
findAggregate _ =
    Nothing

-- ── DML compilation ───────────────────────────────────────────────────────

-- | Compile an INSERT statement.
--
-- For each VALUES row, compile each expression and emit InsertRow.
-- The VM pops the values and passes them to the backend.
compileInsert :: String -> [String] -> [[SqlExpr]] -> Counter
              -> ([Instruction], Counter)
compileInsert table cols rows counter =
    let colsOpt = if null cols then Nothing else Just cols
        instrs = concatMap (\row -> concatMap compileExpr row ++ [InsertRow table colsOpt]) rows
    in (instrs, counter)

-- | Compile an UPDATE statement.
--
-- UPDATE is cursor-based: scan the table, and for each row that matches
-- the optional WHERE predicate, emit UpdateRows.
--
-- Structure:
--   OpenScan table Nothing
--   Label "loop_N"
--   JumpIfExhausted Nothing "end_N"
--   AdvanceCursor Nothing
--   [if WHERE: compile predicate, JumpIfFalse "skip_N"]
--   UpdateRows table
--   [Label "skip_N"]
--   Jump "loop_N"
--   Label "end_N"
--   CloseScan Nothing
--   Halt
compileUpdate :: String -> Maybe SqlExpr -> Counter
              -> ([Instruction], Counter)
compileUpdate table predOpt counter =
    let (body, counter1) = case predOpt of
            Nothing ->
                ([UpdateRows table], counter)
            Just pred ->
                let (n, c1) = freshLabel counter
                    skipLabel = "upd_skip_" ++ n
                in ( compileExpr pred
                        ++ [JumpIfFalse skipLabel, UpdateRows table, Label skipLabel]
                   , c1 )
        (scanInstrs, counter2) = compileScanLoop table Nothing body counter1
    in (scanInstrs ++ [Halt], counter2)

-- | Compile a DELETE statement.
--
-- DELETE is cursor-based: scan the table, and for each row that matches
-- the optional WHERE predicate, emit DeleteRows.
compileDelete :: String -> Maybe SqlExpr -> Counter
              -> ([Instruction], Counter)
compileDelete table predOpt counter =
    let (body, counter1) = case predOpt of
            Nothing ->
                ([DeleteRows table], counter)
            Just pred ->
                let (n, c1) = freshLabel counter
                    skipLabel = "del_skip_" ++ n
                in ( compileExpr pred
                        ++ [JumpIfFalse skipLabel, DeleteRows table, Label skipLabel]
                   , c1 )
        (scanInstrs, counter2) = compileScanLoop table Nothing body counter1
    in (scanInstrs ++ [Halt], counter2)

-- ── Main compile entry point ──────────────────────────────────────────────
--
-- `compile` is the top-level function: given an OptimizedPlan, it produces
-- a Program (a flat list of instructions). The label counter starts at 0.

-- | Compile an OptimizedPlan to a Program.
--
-- This is the primary entry point. Call it with the output of SqlOptimizer.optimize.
--
-- Example:
--   let prog = compile (optimize lp)
--   -- prog.instructions is now ready for the VM
--
-- The label counter starts at 0 for each call so tests are deterministic.
compile :: OptimizedPlan -> Program
compile plan = Program (fst (go plan))
  where
    go :: OptimizedPlan -> ([Instruction], Counter)
    go p = case p of

        -- ── SELECT queries ────────────────────────────────────────────────
        -- Any plan node that could be part of a SELECT is routed through
        -- compileSelect, which handles aggregate detection, post-ops, and Halt.
        OptProject {}   -> compileSelect p 0
        OptFilter {}    -> compileSelect p 0
        OptSort {}      -> compileSelect p 0
        OptLimit {}     -> compileSelect p 0
        OptDistinct {}  -> compileSelect p 0
        OptAggregate {} -> compileSelect p 0
        OptHaving {}    -> compileSelect p 0
        OptScan {}      -> compileSelect p 0
        OptJoin {}      -> compileSelect p 0
        OptUnion {}     -> compileSelect p 0

        -- ── EmptyResult — proven zero rows ────────────────────────────────
        EmptyResult ->
            ([Halt], 0)

        -- ── INSERT ───────────────────────────────────────────────────────
        OptInsert table cols valRows ->
            let (instrs, c1) = compileInsert table cols valRows 0
            in (instrs ++ [Halt], c1)

        -- ── UPDATE ───────────────────────────────────────────────────────
        OptUpdate table _ predOpt ->
            -- For Level 1, the predicate drives the WHERE filter.
            -- Assignment values are compiled in the VM's UpdateRows handler.
            compileUpdate table predOpt 0

        -- ── DELETE ───────────────────────────────────────────────────────
        OptDelete table predOpt ->
            compileDelete table predOpt 0

        -- ── CREATE TABLE ─────────────────────────────────────────────────
        OptCreateTable name ifne cdefs ->
            ([CreateTableInstr name ifne cdefs, Halt], 0)

        -- ── DROP TABLE ───────────────────────────────────────────────────
        OptDropTable name ife ->
            ([DropTableInstr name ife, Halt], 0)
