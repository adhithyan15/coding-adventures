// SqlCodegen.fs — bytecode code generator for the Mini-SQLite Level 1 pipeline.
//
// This module transforms an OptimizedPlan (produced by sql-optimizer) into a
// flat list of stack-machine instructions (Program) that the sql-vm can execute.
//
// ┌─────────────────────────────────────────────────────────────────────────┐
// │  PIPELINE POSITION                                                       │
// │                                                                          │
// │  sql-lexer → sql-parser → sql-planner → sql-optimizer → [sql-codegen]  │
// │           → sql-vm → mini-sqlite                                         │
// │                                                                          │
// │  Input : OptimizedPlan (from the optimizer)                              │
// │  Output: Program — a flat list of Instruction values                    │
// └─────────────────────────────────────────────────────────────────────────┘
//
// ── What is a stack machine? ───────────────────────────────────────────────
//
// A stack machine is the simplest possible virtual computer. It has no named
// registers — just a single stack of values and a sequence of instructions.
// Each instruction pops zero, one, or two values from the top of the stack,
// does some work, and pushes a result back. For example:
//
//   PUSH 3          stack: [3]
//   PUSH 4          stack: [3, 4]
//   ADD             stack: [7]   (popped 3 and 4, pushed their sum)
//
// SQL expressions compile to a straight-line sequence of such instructions.
// The expression `a + (b * 2)` compiles to:
//
//   LoadColumn(None, "a")    ← push the value of column a
//   LoadColumn(None, "b")    ← push b
//   LoadConst(Integer 2)     ← push literal 2
//   BinaryOpInstr(Mul)       ← pop 2 and b, push b*2
//   BinaryOpInstr(Add)       ← pop b*2 and a, push a + b*2
//
// ── Why stack machines for SQL? ───────────────────────────────────────────
//
// SQLite's own query engine (VDBE), the JVM, and CPython all use stack
// machines for the same reason: they're easy to generate code for, easy to
// execute in a tight interpreter loop, and require no register allocation.
//
// ── Two-phase aggregate compilation ──────────────────────────────────────
//
// Aggregates (COUNT, SUM, …) need two phases:
//   1. ACCUMULATE — scan every row, feeding values into accumulators.
//   2. FINALIZE   — after the scan, read each accumulator and emit a row.
//
// The codegen handles this by emitting InitAgg before the loop (initialise
// accumulators to their zero state), UpdateAgg inside the loop (feed each
// row's value to the right accumulator), and FinalizeAgg after the loop
// (compute the final value — e.g. SUM divides by COUNT for AVG).
//
// ── Label-based control flow ──────────────────────────────────────────────
//
// Loops and branches use named labels as jump targets. A Label instruction is
// a no-op marker; Jump/JumpIfFalse/JumpIfTrue transfer control to it. The VM
// resolves labels to instruction indices at runtime in O(1).
//
// Example scan loop for `SELECT name FROM users`:
//
//   OpenScan("users", None)          ← open iterator on the table
//   Label("loop_0")                  ← top of loop
//   JumpIfExhausted(None, "end_0")   ← exit when no more rows
//   AdvanceCursor(None)              ← move to next row
//   BeginRow                         ← start assembling output row
//   LoadColumn(None, "name")         ← push name value
//   EmitColumn("name")               ← store it in the row buffer
//   EmitRow                          ← flush row to result
//   Jump("loop_0")                   ← back to top of loop
//   Label("end_0")                   ← loop exit point
//   CloseScan(None)                  ← release cursor
//   Halt                             ← stop execution

namespace CodingAdventures.SqlCodegen.FSharp

open CodingAdventures.SqlPlanner.FSharp
open CodingAdventures.SqlOptimizer.FSharp

// ── Supporting operator types ─────────────────────────────────────────────
//
// These mirror the planner's BinaryOperator / UnaryOperator / AggFunction,
// but live in the codegen namespace so the VM layer does not need to import
// the planner directly. The compiler maps between the two.

/// Binary infix operators for arithmetic, comparison, and logic.
/// Two arguments are popped, the result is pushed.
type BinaryOp =
    | Add    // a + b
    | Sub    // a - b
    | Mul    // a * b
    | Div    // a / b
    | Mod    // a % b
    | Eq     // a = b
    | Neq    // a <> b
    | Lt     // a < b
    | Lte    // a <= b
    | Gt     // a > b
    | Gte    // a >= b
    | And    // a AND b (short-circuit: FALSE AND NULL = FALSE)
    | Or     // a OR  b (short-circuit: TRUE  OR  NULL = TRUE)
    | Concat // a || b  (string concatenation)

/// Unary prefix operators. One argument is popped, one result is pushed.
type UnaryOp =
    | Neg  // -a   (arithmetic negation)
    | Not  // NOT a (logical negation, three-valued: NOT NULL = NULL)

/// Aggregate function kinds — COUNT, SUM, etc.
/// Each aggregate has an accumulator slot in the VM's aggregate state.
///
/// Think of aggregates like running tallies: COUNT keeps a counter, SUM keeps
/// a running total, MIN/MAX track the smallest/largest value seen so far, and
/// AVG keeps both a sum and a count so it can divide at the end.
type AggFn =
    | Count     // COUNT(expr) — count non-NULL values
    | CountStar // COUNT(*)    — count all rows including NULLs
    | Sum       // SUM(expr)   — sum all non-NULL values
    | Avg       // AVG(expr)   — arithmetic mean of non-NULL values
    | Min       // MIN(expr)   — smallest non-NULL value
    | Max       // MAX(expr)   — largest non-NULL value

// ── Instruction discriminated union ──────────────────────────────────────
//
// Every VM operation is one case of this DU. Instructions are pure data:
// no functions, no side effects — just a description of what to do.
// The VM interprets them in a loop, maintaining the stack and cursors.
//
// [RequireQualifiedAccess] forces callers to write Instruction.LoadConst
// rather than LoadConst, which prevents name clashes with F# built-ins
// and makes code easier to read at a glance.

[<RequireQualifiedAccess>]
type Instruction =
    // ── Stack / memory operations ─────────────────────────────────────────
    //
    // These push or pop values on the expression evaluation stack.
    // Think of the stack as a scratch pad: push inputs, compute, pop results.

    /// Push a compile-time constant onto the stack.
    /// Example: the literal `42` in `WHERE age > 42` becomes `LoadConst(Integer 42)`.
    | LoadConst of value: SqlValue

    /// Push the value of a named column from the current row of a table scan.
    /// `table` is the optional alias (e.g. Some "u" for `u.name`); when None
    /// the VM searches all open cursors for the column name.
    | LoadColumn of table: string option * column: string

    /// Push a runtime-bound parameter (placeholder `?` in the query).
    /// `index` is the 0-based position of the parameter in the binding list.
    | LoadParam of index: int

    /// Push the i-th value from the current group-by key snapshot.
    /// Used in the group-emit phase to reproduce group-key column values
    /// without re-reading the underlying table (which may have advanced).
    | LoadGroupKey of index: int

    /// Push a column value from the OUTER query's current row.
    /// Used for correlated subqueries where an inner expression references
    /// a column from the enclosing query's scan.
    | LoadOuterColumn of table: string option * column: string

    /// Discard the top value of the stack (used to clean up expressions
    /// whose result is not needed, e.g. a SELECT-list expression in a DML).
    | Pop

    // ── Arithmetic and comparison ─────────────────────────────────────────
    //
    // These instructions pop TWO values and push ONE result. The right
    // operand is popped first (it was pushed last), then the left operand.

    /// Pop right and left operands; apply the operator; push the result.
    /// Covers all SQL binary operators: arithmetic, comparison, AND/OR, ||.
    | BinaryOpInstr of op: BinaryOp

    // ── Unary operations ──────────────────────────────────────────────────

    /// Pop one value; apply the unary operator; push the result.
    | UnaryOpInstr of op: UnaryOp

    // ── Predicate / test instructions ─────────────────────────────────────
    //
    // SQL has NULL as a first-class value, so every test must handle three
    // outcomes: TRUE, FALSE, and NULL (unknown). These instructions implement
    // the SQL three-valued logic for common predicates.

    /// Pop value; push TRUE if it is SQL NULL, FALSE otherwise.
    | IsNull

    /// Pop value; push TRUE if it is NOT NULL, FALSE if it is NULL.
    | IsNotNull

    /// Pop hi, lo, value; push TRUE if lo <= value <= high.
    /// `inclusive` controls whether the bounds are inclusive (SQL BETWEEN
    /// uses inclusive; exclusive variants exist for open-interval range scans).
    | Between of inclusive: bool

    /// Pop pattern, value; push TRUE if value LIKE pattern.
    /// SQL LIKE uses `%` for "zero or more chars" and `_` for "exactly one char".
    | Like

    /// Pop `count` items from the stack as the list, then pop the needle;
    /// push TRUE if the needle is in the list (SQL IN operator).
    | InList of count: int

    // ── Scan / cursor control ─────────────────────────────────────────────
    //
    // A cursor is an iterator over a table's rows. Think of it like a file
    // handle: you open it, read rows one at a time, and close it when done.
    // Multiple cursors can be open simultaneously (for JOINs).

    /// Ask the VM to open an iterator (cursor) for `table`.
    /// `alias` is the query alias, used to distinguish cursors in JOINs.
    | OpenScan of table: string * alias: string option

    /// Move the cursor identified by `alias` to its next row.
    /// If the cursor is exhausted, the VM jumps to `label`.
    /// This is the "next row" instruction — the heart of every scan loop.
    | AdvanceCursor of alias: string option

    /// Jump to `label` if the cursor identified by `alias` is exhausted.
    /// Used to test exhaustion without advancing, useful for nested loops.
    | JumpIfExhausted of alias: string option * label: string

    /// Release the cursor, freeing any resources it holds.
    | CloseScan of alias: string option

    // ── Row construction ──────────────────────────────────────────────────
    //
    // Output rows are assembled instruction by instruction.
    // BeginRow clears the row buffer; EmitColumn stores one field;
    // EmitRow flushes the assembled row to the result set.

    /// Clear the row buffer and start assembling a new output row.
    | BeginRow

    /// Pop the top of the stack and store it as column `name` in the row buffer.
    | EmitColumn of name: string

    /// Finalize the row buffer and append the assembled row to the result set.
    | EmitRow

    // ── Aggregation ───────────────────────────────────────────────────────
    //
    // Aggregation is a two-phase process: accumulate during the scan loop,
    // then finalize and emit after the scan completes.
    //
    // Example for `SELECT COUNT(*), SUM(price) FROM orders`:
    //
    //   InitAgg(2)                 ← create 2 accumulators: [count=0, sum=0]
    //   ...scan loop...
    //     UpdateAgg(0, CountStar)  ← accumulator 0: increment count
    //     LoadColumn(_, "price")   ← push price
    //     UpdateAgg(1, Sum)        ← accumulator 1: add price to sum
    //   ...end loop...
    //   BeginRow
    //   FinalizeAgg(0, CountStar)  ← push finalized count
    //   EmitColumn("count(*)")
    //   FinalizeAgg(1, Sum)        ← push finalized sum
    //   EmitColumn("sum(price)")
    //   EmitRow

    /// Initialize `count` aggregate accumulators to their zero states.
    | InitAgg of count: int

    /// Pop the top of the stack and feed it into accumulator `index`.
    /// The `fn` tells the accumulator how to combine the new value
    /// (e.g. SUM adds, MIN takes the smaller, COUNT increments).
    | UpdateAgg of index: int * fn: AggFn

    /// Compute the final value of accumulator `index` and push it.
    /// For AVG this divides sum by count; for COUNT it returns the count; etc.
    | FinalizeAgg of index: int * fn: AggFn

    /// Save the current group-by key values for use in the emit phase.
    /// `keys` is the list of column names that form the GROUP BY key.
    | SaveGroupKey of keys: string list

    /// Advance the group iterator to the next group.
    /// Similar to AdvanceCursor but for the group accumulator map.
    | AdvanceGroup

    // ── Control flow ──────────────────────────────────────────────────────
    //
    // Jump targets are named strings during code generation. At runtime the
    // VM resolves them to instruction indices for O(1) dispatch.

    /// A no-op marker that names a position in the instruction stream.
    /// Jump instructions refer to labels by name.
    | Label of name: string

    /// Unconditional jump to the named label.
    | Jump of label: string

    /// Pop value; jump to `label` if the value is TRUE.
    | JumpIfTrue of label: string

    /// Pop value; jump to `label` if the value is FALSE or NULL.
    | JumpIfFalse of label: string

    /// Stop execution; the result set holds the output rows.
    | Halt

    // ── DDL (Data Definition Language) ───────────────────────────────────
    //
    // CREATE TABLE and DROP TABLE produce a single instruction each.
    // No scan loop needed — these are schema operations, not row operations.

    /// Ask the VM/backend to create a table with the given columns.
    /// `ifNotExists` mirrors `CREATE TABLE IF NOT EXISTS`.
    | CreateTable of name: string * ifNotExists: bool * columns: ColumnDef list

    /// Ask the VM/backend to drop a table.
    /// `ifExists` mirrors `DROP TABLE IF EXISTS`.
    | DropTable of name: string * ifExists: bool

    // ── DML (Data Manipulation Language) ─────────────────────────────────

    /// Insert one row into `table`. The values for `columns` are on the stack
    /// in order (leftmost column was pushed first).
    | InsertRow of table: string * columns: string list option

    /// Update column values for the row under the current cursor.
    /// `assignments` is a list of (column_name, expression) pairs.
    | UpdateRows of table: string * assignments: (string * Expr) list

    /// Delete the row currently under the cursor.
    | DeleteRows of table: string

    // ── Transaction control ───────────────────────────────────────────────
    //
    // These mirror the SQL `BEGIN`, `COMMIT`, and `ROLLBACK` statements.
    // Most queries do not need explicit transactions (the VM handles
    // auto-commit), but explicit transaction support is needed for ACID tests.

    | BeginTransaction
    | CommitTransaction
    | RollbackTransaction

    // ── Result post-processing ────────────────────────────────────────────
    //
    // After the scan loop fills the result buffer, these instructions apply
    // sorting, deduplication, and pagination as post-processing steps.
    //
    // These are emitted AFTER the scan loop closes, not inside it.

    /// Sort the result buffer by the given sort keys.
    /// Each SortKey specifies a column, direction (Asc/Desc), and NULL ordering.
    | SortResult of keys: SortKey list

    /// Deduplicate the result buffer, keeping only distinct rows.
    /// Applied after SortResult so duplicates are adjacent (O(n) scan).
    | DistinctResult

    /// Keep at most `count` rows starting at `offset` (0-based).
    /// `None` means "no limit" or "no offset" respectively.
    | LimitResult of count: int64 option * offset: int64 option

// ── Program — the compiled output ────────────────────────────────────────
//
// A Program is simply a flat list of Instructions. The VM executes them in
// order, jumping when it encounters Jump/JumpIfTrue/JumpIfFalse/AdvanceCursor.
//
// Why a list? It's the simplest representation and sufficient for Level 1.
// A future optimizer could convert to an array for O(1) index access.

/// The compiled output of the code generator.
type Program = { Instructions: Instruction list }

// ── Label counter (module-level mutable state) ───────────────────────────
//
// A counter produces unique suffixes like "0", "1", "2" for label names.
// Each scan gets its own loop/end label pair so nested scans don't clash.
//
// Example: a JOIN produces two scan pairs:
//   loop_0 / end_0 for the outer (left) table
//   loop_1 / end_1 for the inner (right) table
//
// NOTE: Module-level `let mutable` is idiomatic F# for stateful helpers.
// This is reset at the start of each `compile` call so tests are isolated.

[<AutoOpen>]
module private LabelState =
    let mutable labelCounter = 0

    let freshLabel () =
        let n = labelCounter
        labelCounter <- labelCounter + 1
        string n

    let resetLabelCounter () =
        labelCounter <- 0

// ── SqlCodegen module ─────────────────────────────────────────────────────
//
// The SqlCodegen module holds the two public entry points:
//   `compile`           : OptimizedPlan → Program
//   `compileExpression` : Expr → Instruction list   (exported for testing)

module SqlCodegen =

    // ── Operator mapping ──────────────────────────────────────────────────
    //
    // The planner uses its own operator types (BinaryOperator, UnaryOperator,
    // AggFunction). We map them to the codegen's operator types here.
    // This keeps the VM layer decoupled from the planner's type hierarchy.

    let private mapBinaryOp (op: BinaryOperator) : BinaryOp =
        match op with
        | BinaryOperator.Add   -> BinaryOp.Add
        | BinaryOperator.Sub   -> BinaryOp.Sub
        | BinaryOperator.Mul   -> BinaryOp.Mul
        | BinaryOperator.Div   -> BinaryOp.Div
        | BinaryOperator.Mod   -> BinaryOp.Mod
        | BinaryOperator.Eq    -> BinaryOp.Eq
        | BinaryOperator.NotEq -> BinaryOp.Neq
        | BinaryOperator.Lt    -> BinaryOp.Lt
        | BinaryOperator.Lte   -> BinaryOp.Lte
        | BinaryOperator.Gt    -> BinaryOp.Gt
        | BinaryOperator.Gte   -> BinaryOp.Gte
        | BinaryOperator.And   -> BinaryOp.And
        | BinaryOperator.Or    -> BinaryOp.Or

    let private mapUnaryOp (op: UnaryOperator) : UnaryOp =
        match op with
        | UnaryOperator.Neg -> UnaryOp.Neg
        | UnaryOperator.Not -> UnaryOp.Not

    let private mapAggFn (fn: AggFunction) : AggFn =
        match fn with
        | AggFunction.Count -> AggFn.Count
        | AggFunction.Sum   -> AggFn.Sum
        | AggFunction.Avg   -> AggFn.Avg
        | AggFunction.Min   -> AggFn.Min
        | AggFunction.Max   -> AggFn.Max

    // ── Expression compiler ───────────────────────────────────────────────
    //
    // `compileExpression` translates an Expr tree into a flat sequence of
    // Instructions that, when executed by the VM, leaves one value on the
    // stack. Recursive calls handle sub-expressions, building up the sequence
    // from the leaves inward (post-order traversal).

    /// Compile an expression to a flat instruction sequence.
    /// Each instruction pushes one value; the last instruction leaves the
    /// final value on top of the stack.
    ///
    /// Examples:
    ///   Expr.Literal(Integer 42)      → [LoadConst(Integer 42)]
    ///   Expr.Column(None, "age")      → [LoadColumn(None, "age")]
    ///   Expr.BinaryOp(Add, a, b)      → [compile(a) @ compile(b) @ [BinaryOpInstr Add]]
    let rec compileExpression (expr: Expr) : Instruction list =
        match expr with

        // ── Literals ─────────────────────────────────────────────────────
        // A literal is simply pushed as a constant. No computation needed.
        | Expr.Literal value ->
            [ Instruction.LoadConst value ]

        // ── Column references ─────────────────────────────────────────────
        // The table qualifier is preserved so the VM can distinguish
        // columns from different tables in a JOIN.
        | Expr.Column(tableOpt, col) ->
            [ Instruction.LoadColumn(tableOpt, col) ]

        // ── Binary operators ──────────────────────────────────────────────
        // Compile left, then right (so left is on the stack first), then apply
        // the operator. The VM pops right first (it's on top), then left.
        | Expr.BinaryOp(op, left, right) ->
            compileExpression left
            @ compileExpression right
            @ [ Instruction.BinaryOpInstr(mapBinaryOp op) ]

        // ── Unary operators ───────────────────────────────────────────────
        // Compile the operand, then apply the unary operator.
        | Expr.UnaryOp(op, operand) ->
            compileExpression operand
            @ [ Instruction.UnaryOpInstr(mapUnaryOp op) ]

        // ── NULL tests ────────────────────────────────────────────────────
        // IS NULL / IS NOT NULL push a boolean (never NULL) regardless of
        // whether the operand is NULL. These are distinct from comparisons
        // because `NULL = NULL` is NULL, but `NULL IS NULL` is TRUE.
        | Expr.IsNull e ->
            compileExpression e @ [ Instruction.IsNull ]

        | Expr.IsNotNull e ->
            compileExpression e @ [ Instruction.IsNotNull ]

        // ── BETWEEN ───────────────────────────────────────────────────────
        // `value BETWEEN lo AND hi` compiles to: push value, push lo, push hi,
        // then the Between instruction pops all three and pushes TRUE/FALSE/NULL.
        | Expr.Between(value, lo, hi) ->
            compileExpression value
            @ compileExpression lo
            @ compileExpression hi
            @ [ Instruction.Between(inclusive = true) ]

        // ── LIKE ──────────────────────────────────────────────────────────
        // Push value, push pattern (as a constant string), then Like.
        | Expr.Like(value, pattern) ->
            compileExpression value
            @ [ Instruction.LoadConst(SqlValue.Text pattern) ]
            @ [ Instruction.Like ]

        // ── NOT LIKE ─────────────────────────────────────────────────────
        // Compile as LIKE then negate the result.
        | Expr.NotLike(value, pattern) ->
            compileExpression value
            @ [ Instruction.LoadConst(SqlValue.Text pattern) ]
            @ [ Instruction.Like ]
            @ [ Instruction.UnaryOpInstr(UnaryOp.Not) ]

        // ── IN (list) ────────────────────────────────────────────────────
        // Push the needle (value to test), then push each list item, then
        // the InList instruction pops count items and the needle and pushes
        // TRUE/FALSE/NULL.
        | Expr.In(value, items) ->
            compileExpression value
            @ List.collect compileExpression items
            @ [ Instruction.InList(List.length items) ]

        // ── NOT IN ───────────────────────────────────────────────────────
        // Compile as IN then negate. Note: NOT IN has special NULL semantics
        // (x NOT IN (...NULL...) = NULL), handled by the VM's InList instruction
        // combined with the NOT operator.
        | Expr.NotIn(value, items) ->
            compileExpression value
            @ List.collect compileExpression items
            @ [ Instruction.InList(List.length items) ]
            @ [ Instruction.UnaryOpInstr(UnaryOp.Not) ]

        // ── Aggregate expressions ─────────────────────────────────────────
        // Aggregates within expressions are handled by the scan-loop compiler
        // (which knows the accumulator slot numbers). If we reach here without
        // aggregate context, emit a null constant as a safe no-op.
        | Expr.AggExpr(fn, arg, _distinct) ->
            ignore (fn, arg)
            [ Instruction.LoadConst SqlValue.Null ]

        // ── Function calls ────────────────────────────────────────────────
        // SQL scalar functions compile each argument onto the stack.
        // For Level 1, unknown scalar functions return NULL as a placeholder.
        | Expr.FuncCall(name, args) ->
            ignore name
            List.collect compileExpression args
            @ [ Instruction.LoadConst SqlValue.Null ]

        // ── Wildcard ─────────────────────────────────────────────────────
        // SELECT * is handled at the plan level; bare Wildcard pushes NULL.
        | Expr.Wildcard ->
            [ Instruction.LoadConst SqlValue.Null ]

    // ── Aggregate helpers ─────────────────────────────────────────────────

    let private compileUpdateAgg (index: int) (fn: AggFunction) (arg: AggArg) : Instruction list =
        match fn, arg with
        | AggFunction.Count, AggArg.Star ->
            // COUNT(*) — no column reference needed; just update the counter
            [ Instruction.UpdateAgg(index, AggFn.CountStar) ]
        | AggFunction.Count, AggArg.Expr e ->
            compileExpression e @ [ Instruction.UpdateAgg(index, AggFn.Count) ]
        | AggFunction.Sum, AggArg.Expr e ->
            compileExpression e @ [ Instruction.UpdateAgg(index, AggFn.Sum) ]
        | AggFunction.Avg, AggArg.Expr e ->
            compileExpression e @ [ Instruction.UpdateAgg(index, AggFn.Avg) ]
        | AggFunction.Min, AggArg.Expr e ->
            compileExpression e @ [ Instruction.UpdateAgg(index, AggFn.Min) ]
        | AggFunction.Max, AggArg.Expr e ->
            compileExpression e @ [ Instruction.UpdateAgg(index, AggFn.Max) ]
        | _ ->
            [ Instruction.UpdateAgg(index, AggFn.CountStar) ]

    let private compileFinalizeAgg (index: int) (fn: AggFunction) : Instruction =
        let aggFn =
            match fn with
            | AggFunction.Count -> AggFn.Count
            | AggFunction.Sum   -> AggFn.Sum
            | AggFunction.Avg   -> AggFn.Avg
            | AggFunction.Min   -> AggFn.Min
            | AggFunction.Max   -> AggFn.Max
        Instruction.FinalizeAgg(index, aggFn)

    let private outputColName (col: OutputColumn) : string =
        match col with
        | OutputColumn.Star -> "*"
        | OutputColumn.Expr(_, Some alias) -> alias
        | OutputColumn.Expr(Expr.Column(_, colName), None) -> colName
        | OutputColumn.Expr(Expr.AggExpr(fn, _, _), None) ->
            match fn with
            | AggFunction.Count -> "count"
            | AggFunction.Sum   -> "sum"
            | AggFunction.Avg   -> "avg"
            | AggFunction.Min   -> "min"
            | AggFunction.Max   -> "max"
        | OutputColumn.Expr(_, None) -> "expr"

    // ── Scan loop codegen ─────────────────────────────────────────────────
    //
    // The fundamental pattern for any table scan is:
    //
    //   OpenScan(table, alias)
    //   Label("loop_N")
    //   JumpIfExhausted(alias, "end_N")
    //   AdvanceCursor(alias)
    //   <body — filter predicate + row construction>
    //   Jump("loop_N")
    //   Label("end_N")
    //   CloseScan(alias)
    //
    // The `body` is provided as a list of instructions by the caller.

    let private compileScanLoop (table: string) (alias: string option) (body: Instruction list) : Instruction list =
        let n = freshLabel ()
        let loopLabel = sprintf "loop_%s" n
        let endLabel  = sprintf "end_%s"  n

        [ Instruction.OpenScan(table, alias)
          Instruction.Label loopLabel
          Instruction.JumpIfExhausted(alias, endLabel)
          Instruction.AdvanceCursor alias ]
        @ body
        @ [ Instruction.Jump loopLabel
            Instruction.Label endLabel
            Instruction.CloseScan alias ]

    // ── Mutually recursive compilation functions ──────────────────────────
    //
    // `compilePlan`, `compileOutputColumns`, `compileAggregateQuery`, and
    // `compileSelect` call each other, so they are declared with `let rec ... and ...`.
    //
    // F# requires ALL mutually recursive functions to be declared in a single
    // `let rec ... and ...` block. This is the idiomatic pattern.

    /// Compile the scan phase of a plan node, inserting `body` as the loop body.
    /// Returns the instruction sequence for opening cursors, iterating rows,
    /// and closing cursors — without post-processing (Sort/Limit/Distinct).
    let rec private compilePlan (plan: OptimizedPlan) (body: Instruction list) : Instruction list =
        match plan with

        // ── Base scan ─────────────────────────────────────────────────────
        // The leaf of most query trees — opens a cursor and iterates rows.
        | OptimizedPlan.Scan(table, alias, _reqCols, _scanLimit) ->
            compileScanLoop table alias body

        // ── Filter — compile the predicate as a guard inside the loop ─────
        // The filter sits "between" the cursor advance and the body.
        // If the predicate is false/null, we jump past the body for this row.
        | OptimizedPlan.Filter(inner, pred) ->
            let skipLabel = sprintf "filter_skip_%s" (freshLabel ())
            let filterGuard =
                compileExpression pred
                @ [ Instruction.JumpIfFalse skipLabel ]
            compilePlan inner (filterGuard @ body @ [ Instruction.Label skipLabel ])

        // ── Project — just wrap the body; projection is in EmitColumn ─────
        | OptimizedPlan.Project(inner, _cols) ->
            compilePlan inner body

        // ── Join — nested loop over two tables ────────────────────────────
        // A JOIN is implemented as nested scan loops: for each row in the
        // left table, scan all rows of the right table.
        | OptimizedPlan.Join(left, right, kind, condOpt) ->
            let condGuard =
                match condOpt with
                | None -> []
                | Some cond ->
                    let skipLabel = sprintf "join_cond_%s" (freshLabel ())
                    compileExpression cond
                    @ [ Instruction.JumpIfFalse skipLabel ]
                    @ body
                    @ [ Instruction.Label skipLabel ]
            ignore kind
            let innerBody =
                match condOpt with
                | None -> body
                | Some _ -> condGuard
            let innerLoop = compilePlan right innerBody
            compilePlan left innerLoop

        // ── Aggregate / Having ─────────────────────────────────────────────
        // These are handled by compileAggregateQuery above the scan level.
        // If we encounter them here (nested context), compile the inner plan.
        | OptimizedPlan.Aggregate(inner, _groupBy, _aggs) ->
            compilePlan inner body

        | OptimizedPlan.Having(inner, _pred) ->
            compilePlan inner body

        // ── Pass-through wrappers ─────────────────────────────────────────
        // Sort, Limit, Distinct are post-ops applied after the scan completes.
        // In compilePlan we just recurse into the inner plan.
        | OptimizedPlan.Sort(inner, _)
        | OptimizedPlan.Limit(inner, _, _)
        | OptimizedPlan.Distinct(inner) ->
            compilePlan inner body

        | OptimizedPlan.Union(left, _right, _all) ->
            compilePlan left body

        | OptimizedPlan.EmptyResult ->
            []

        | _ ->
            body

    /// Compile output columns for a non-aggregate SELECT.
    /// Returns a flat instruction sequence that, for each output column,
    /// pushes the column's value and emits it with a name.
    and private compileOutputColumns (cols: OutputColumn list) : Instruction list =
        if List.isEmpty cols then
            // No Project node — emit a wildcard marker
            [ Instruction.LoadConst(SqlValue.Text "*") ]
        else
            cols |> List.collect (fun col ->
                match col with
                | OutputColumn.Star ->
                    [ Instruction.LoadConst(SqlValue.Text "*") ]
                | OutputColumn.Expr(Expr.AggExpr _, _) ->
                    // Aggregate in non-aggregate context — emit null
                    [ Instruction.LoadConst SqlValue.Null
                      Instruction.EmitColumn (outputColName col) ]
                | OutputColumn.Expr(expr, _) ->
                    compileExpression expr @ [ Instruction.EmitColumn (outputColName col) ])

    /// Compile an aggregate query (with or without GROUP BY).
    ///
    /// Aggregates require two phases:
    /// 1. ACCUMULATE — UpdateAgg in the scan loop
    /// 2. FINALIZE   — FinalizeAgg after the loop
    and private compileAggregateQuery
        (innerPlan:  OptimizedPlan)
        (aggs:       AggregateItem list)
        (groupBy:    Expr list)
        (havingOpt:  Expr option)
        : Instruction list =

        let numAggs = List.length aggs
        let aggSlots = aggs |> List.mapi (fun i a -> (i, mapAggFn a.Func)) |> Map.ofList

        // Build the group-key column names (for SaveGroupKey / LoadGroupKey)
        let groupKeyNames =
            groupBy |> List.mapi (fun i e ->
                match e with
                | Expr.Column(_, c) -> c
                | _ -> sprintf "key_%d" i)

        // Inside the loop: save group key and update accumulators
        let saveKeyInstrs =
            if List.isEmpty groupBy then []
            else
                List.collect compileExpression groupBy
                @ [ Instruction.SaveGroupKey groupKeyNames ]

        let updateInstrs =
            aggs |> List.mapi (fun i a -> compileUpdateAgg i a.Func a.Arg)
            |> List.concat

        // Compile the scan loop with accumulate-body
        // The inner plan may have a Filter wrapping the base scan
        let scanInstrs = compilePlan innerPlan (saveKeyInstrs @ updateInstrs)

        // After the scan: emit one row per group
        // Group-key columns first, then aggregate finalizations
        let keyEmitInstrs =
            groupKeyNames |> List.mapi (fun i name ->
                [ Instruction.LoadGroupKey i
                  Instruction.EmitColumn name ])
            |> List.concat

        let aggEmitInstrs =
            aggs |> List.mapi (fun i a ->
                let aggFn =
                    match aggSlots |> Map.tryFind i with
                    | Some f -> f
                    | None   -> AggFn.CountStar
                [ compileFinalizeAgg i a.Func
                  Instruction.EmitColumn a.Alias ])
            |> List.concat

        // HAVING clause filters out groups whose aggregate result fails the predicate
        let emitPhase =
            match havingOpt with
            | None ->
                [ Instruction.BeginRow ]
                @ keyEmitInstrs
                @ aggEmitInstrs
                @ [ Instruction.EmitRow ]
            | Some pred ->
                let skipLabel = sprintf "having_skip_%s" (freshLabel ())
                [ Instruction.BeginRow ]
                @ keyEmitInstrs
                @ aggEmitInstrs
                @ compileExpression pred
                @ [ Instruction.JumpIfFalse skipLabel
                    Instruction.EmitRow
                    Instruction.Label skipLabel ]

        [ Instruction.InitAgg numAggs ]
        @ scanInstrs
        @ emitPhase

    /// Compile a SELECT query: peel post-ops, detect aggregation, emit scan + post-ops.
    ///
    /// Pipeline:
    ///   1. Peel Sort/Limit/Distinct wrappers into post-op list
    ///   2. Detect aggregate vs non-aggregate
    ///   3. Emit the scan loop (with filter + projection inside)
    ///   4. Append post-ops (Sort, Distinct, Limit)
    and private compileSelect (plan: OptimizedPlan) : Instruction list =

        // ── Step 1: Peel post-processing wrappers ─────────────────────────
        let rec peelWrappers (p: OptimizedPlan) (postOps: Instruction list) =
            match p with
            | OptimizedPlan.Sort(inner, keys) ->
                peelWrappers inner (postOps @ [ Instruction.SortResult keys ])
            | OptimizedPlan.Limit(inner, count, offset) ->
                peelWrappers inner (postOps @ [ Instruction.LimitResult(count, offset) ])
            | OptimizedPlan.Distinct(inner) ->
                peelWrappers inner (postOps @ [ Instruction.DistinctResult ])
            | other ->
                (other, postOps)

        let (corePlan, postOps) = peelWrappers plan []

        // ── Step 2: Detect aggregation ────────────────────────────────────
        // Find the Aggregate node (possibly wrapped in Project or Having).
        let rec findAggregate (p: OptimizedPlan) =
            match p with
            | OptimizedPlan.Project(OptimizedPlan.Aggregate(inner, gb, aggs), cols) ->
                Some (inner, gb, aggs, None, cols)
            | OptimizedPlan.Project(OptimizedPlan.Having(OptimizedPlan.Aggregate(inner, gb, aggs), pred), cols) ->
                Some (inner, gb, aggs, Some pred, cols)
            | OptimizedPlan.Aggregate(inner, gb, aggs) ->
                Some (inner, gb, aggs, None, [])
            | OptimizedPlan.Having(OptimizedPlan.Aggregate(inner, gb, aggs), pred) ->
                Some (inner, gb, aggs, Some pred, [])
            | _ -> None

        let scanInstrs =
            match findAggregate corePlan with
            | Some (innerPlan, groupBy, aggs, havingOpt, _outputCols) ->
                // ── Aggregate query ───────────────────────────────────────
                compileAggregateQuery innerPlan aggs groupBy havingOpt

            | None ->
                // ── Non-aggregate query ───────────────────────────────────
                // Find the output columns from the outermost Project node
                let (outputCols, innerPlan) =
                    match corePlan with
                    | OptimizedPlan.Project(inner, cols) -> (cols, inner)
                    | other -> ([], other)

                // Build the loop body: BeginRow + emit each column + EmitRow
                let emitBody =
                    [ Instruction.BeginRow ]
                    @ compileOutputColumns outputCols
                    @ [ Instruction.EmitRow ]

                compilePlan innerPlan emitBody

        scanInstrs @ postOps @ [ Instruction.Halt ]

    // ── INSERT compilation ────────────────────────────────────────────────
    //
    // `INSERT INTO t (col1, col2) VALUES (v1, v2)`
    //
    // For each row in VALUES, we compile each value expression and emit
    // one InsertRow instruction. The VM pops the values and passes them
    // to the backend.

    let private compileInsert (table: string) (colsOpt: string list option) (source: InsertSource) : Instruction list =
        match source with
        | InsertSource.Values rowsList ->
            rowsList |> List.collect (fun row ->
                List.collect compileExpression row
                @ [ Instruction.InsertRow(table, colsOpt) ])
        | InsertSource.Query subplan ->
            compileSelect (SqlOptimizer.lift subplan)

    // ── UPDATE compilation ────────────────────────────────────────────────
    //
    // UPDATE is cursor-based: scan the table with a filter (WHERE clause),
    // and for each matching row, apply the SET assignments.

    let private compileUpdate (table: string) (assignments: Assignment list) (predOpt: Expr option) : Instruction list =
        let pairs = assignments |> List.map (fun a -> (a.Column, a.Value))
        let body =
            match predOpt with
            | None ->
                [ Instruction.UpdateRows(table, pairs) ]
            | Some pred ->
                let skipLabel = sprintf "upd_skip_%s" (freshLabel ())
                compileExpression pred
                @ [ Instruction.JumpIfFalse skipLabel
                    Instruction.UpdateRows(table, pairs)
                    Instruction.Label skipLabel ]
        compileScanLoop table None body @ [ Instruction.Halt ]

    // ── DELETE compilation ────────────────────────────────────────────────

    let private compileDelete (table: string) (predOpt: Expr option) : Instruction list =
        let body =
            match predOpt with
            | None ->
                [ Instruction.DeleteRows table ]
            | Some pred ->
                let skipLabel = sprintf "del_skip_%s" (freshLabel ())
                compileExpression pred
                @ [ Instruction.JumpIfFalse skipLabel
                    Instruction.DeleteRows table
                    Instruction.Label skipLabel ]
        compileScanLoop table None body @ [ Instruction.Halt ]

    // ── Main compile entry point ──────────────────────────────────────────
    //
    // `compile` is the top-level function: given an OptimizedPlan, it returns
    // a Program (a flat list of instructions).

    /// Compile an OptimizedPlan to a Program.
    ///
    /// This is the primary entry point. Call it with the output of SqlOptimizer.optimize.
    ///
    /// Example:
    ///   let plan = SqlOptimizer.optimize (SqlPlanner.plan schema stmt)
    ///   let program = SqlCodegen.compile plan
    ///   // program.Instructions is now ready for the VM
    let compile (plan: OptimizedPlan) : Program =
        resetLabelCounter ()

        let instructions =
            match plan with

            // ── SELECT queries ─────────────────────────────────────────────
            | OptimizedPlan.Project _
            | OptimizedPlan.Filter _
            | OptimizedPlan.Sort _
            | OptimizedPlan.Limit _
            | OptimizedPlan.Distinct _
            | OptimizedPlan.Aggregate _
            | OptimizedPlan.Having _
            | OptimizedPlan.Scan _
            | OptimizedPlan.Join _
            | OptimizedPlan.Union _ ->
                compileSelect plan

            // ── EmptyResult — proven to produce zero rows ──────────────────
            | OptimizedPlan.EmptyResult ->
                [ Instruction.Halt ]

            // ── DML ────────────────────────────────────────────────────────

            | OptimizedPlan.Insert(table, colsOpt, source) ->
                compileInsert table colsOpt source @ [ Instruction.Halt ]

            | OptimizedPlan.Update(table, assignments, predOpt) ->
                compileUpdate table assignments predOpt

            | OptimizedPlan.Delete(table, predOpt) ->
                compileDelete table predOpt

            // ── DDL ────────────────────────────────────────────────────────

            | OptimizedPlan.CreateTable(name, ifNotExists, columns) ->
                [ Instruction.CreateTable(name, ifNotExists, columns)
                  Instruction.Halt ]

            | OptimizedPlan.DropTable(name, ifExists) ->
                [ Instruction.DropTable(name, ifExists)
                  Instruction.Halt ]

        { Instructions = instructions }
