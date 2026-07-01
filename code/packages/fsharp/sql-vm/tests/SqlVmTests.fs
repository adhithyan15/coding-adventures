// SqlVmTests.fs — unit tests for the F# sql-vm package.
//
// We test the VM by constructing Program values (lists of Instruction) directly,
// without going through the parser or codegen. This tests the VM in isolation:
// each test controls exactly which instructions run and in what order.
//
// ── Test organisation ────────────────────────────────────────────────────
//
// 1.  QueryResult structure
// 2.  Halt / trivial programs
// 3.  LoadConst / stack
// 4.  BinaryOpInstr — arithmetic
// 5.  BinaryOpInstr — comparison
// 6.  BinaryOpInstr — AND / OR (three-valued logic)
// 7.  UnaryOpInstr
// 8.  IsNull / IsNotNull
// 9.  Between
// 10. Like
// 11. InList
// 12. Scan / OpenScan / AdvanceCursor / JumpIfExhausted / CloseScan
// 13. Row construction (BeginRow / EmitColumn / EmitRow)
// 14. Aggregates (InitAgg / UpdateAgg / FinalizeAgg)
// 15. Control flow (Jump / JumpIfTrue / JumpIfFalse / Label)
// 16. DDL (CreateTable / DropTable)
// 17. DML (InsertRow / DeleteRows)
// 18. Transaction control (BeginTransaction / Commit / Rollback)
// 19. Post-processing (SortResult / DistinctResult / LimitResult)
// 20. Pop
// 21. SaveGroupKey / LoadGroupKey
// 22. End-to-end scan loops

module CodingAdventures.SqlVm.FSharp.Tests

open Xunit
open CodingAdventures.SqlPlanner.FSharp
open CodingAdventures.SqlCodegen.FSharp
open CodingAdventures.SqlBackend.FSharp
open CodingAdventures.SqlVm.FSharp

// ── Test helpers ──────────────────────────────────────────────────────────

/// Build a Program from an instruction list.
let private prog (instrs: Instruction list) : Program =
    { Instructions = instrs }

/// Execute a program against a fresh InMemoryBackend.
let private runFresh (instrs: Instruction list) : QueryResult =
    SqlVm.execute (prog instrs) (InMemoryBackend())

/// Execute a program against a given backend.
let private run (backend: Backend) (instrs: Instruction list) : QueryResult =
    SqlVm.execute (prog instrs) backend

/// Build a backend pre-loaded with a "users" table (id INTEGER, name TEXT).
let private usersBackend () =
    let b = InMemoryBackend()
    b.CreateTable("users",
        [| ColumnDef("id",   "INTEGER"); ColumnDef("name", "TEXT") |],
        false)
    b

/// Insert a row into the users table.
let private insertUser (b: Backend) (id: int64) (name: string) =
    let row = Row()
    row["id"]   <- box id
    row["name"] <- box name
    b.Insert("users", row)

/// Shorthand constructors.
let private intVal i   = SqlValue.Integer i
let private textVal s  = SqlValue.Text s
let private boolVal b  = SqlValue.Bool b
let private nullVal    = SqlValue.Null
let private realVal r  = SqlValue.Real r

// ── Type-annotated assertion helpers ─────────────────────────────────────
//
// xUnit's Assert.Equal is overloaded: one overload for 'T (structural equality)
// and one for IEnumerable<'T> (element-wise). F# lists implement both, so the
// compiler cannot resolve the call without a type annotation. These helpers
// pin the type so callers can write assertRow/assertCols without annotation noise.

let private assertRow (expected: SqlValue list) (actual: SqlValue list) =
    Assert.Equal<SqlValue list>(expected, actual)

let private assertCols (expected: string list) (actual: string list) =
    Assert.Equal<string list>(expected, actual)

// ── 1. QueryResult structure ──────────────────────────────────────────────

[<Fact>]
let ``Empty program with Halt returns empty result`` () =
    let r = runFresh [ Instruction.Halt ]
    Assert.Empty(r.Columns)
    Assert.Empty(r.Rows)
    Assert.Equal(0, r.RowsAffected)

[<Fact>]
let ``Empty instruction list halts naturally`` () =
    let r = runFresh []
    Assert.Equal(0, r.RowsAffected)

// ── 2. LoadConst ──────────────────────────────────────────────────────────

[<Fact>]
let ``LoadConst Integer pushes integer value`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 42L)
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 42L ] r.Rows.[0]

[<Fact>]
let ``LoadConst Text pushes text value`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (textVal "hello")
        Instruction.EmitColumn "s"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ textVal "hello" ] r.Rows.[0]

[<Fact>]
let ``LoadConst Null pushes SQL NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst nullVal
        Instruction.EmitColumn "x"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

[<Fact>]
let ``LoadConst Real pushes real value`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (realVal 3.14)
        Instruction.EmitColumn "pi"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 3.14 ] r.Rows.[0]

[<Fact>]
let ``LoadConst Bool pushes bool value`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (boolVal true)
        Instruction.EmitColumn "b"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

// ── 3. BinaryOpInstr — arithmetic ─────────────────────────────────────────

[<Fact>]
let ``BinaryOpInstr Add integers`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 3L)
        Instruction.LoadConst (intVal 4L)
        Instruction.BinaryOpInstr BinaryOp.Add
        Instruction.EmitColumn "sum"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 7L ] r.Rows.[0]

[<Fact>]
let ``BinaryOpInstr Sub integers`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 10L)
        Instruction.LoadConst (intVal 3L)
        Instruction.BinaryOpInstr BinaryOp.Sub
        Instruction.EmitColumn "diff"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 7L ] r.Rows.[0]

[<Fact>]
let ``BinaryOpInstr Mul integers`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 6L)
        Instruction.LoadConst (intVal 7L)
        Instruction.BinaryOpInstr BinaryOp.Mul
        Instruction.EmitColumn "prod"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 42L ] r.Rows.[0]

[<Fact>]
let ``BinaryOpInstr Div integers`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 10L)
        Instruction.LoadConst (intVal 3L)
        Instruction.BinaryOpInstr BinaryOp.Div
        Instruction.EmitColumn "q"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 3L ] r.Rows.[0]

[<Fact>]
let ``BinaryOpInstr Div by zero returns NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 10L)
        Instruction.LoadConst (intVal 0L)
        Instruction.BinaryOpInstr BinaryOp.Div
        Instruction.EmitColumn "q"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

[<Fact>]
let ``BinaryOpInstr Mod integers`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 10L)
        Instruction.LoadConst (intVal 3L)
        Instruction.BinaryOpInstr BinaryOp.Mod
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 1L ] r.Rows.[0]

[<Fact>]
let ``BinaryOpInstr Concat strings`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (textVal "hello ")
        Instruction.LoadConst (textVal "world")
        Instruction.BinaryOpInstr BinaryOp.Concat
        Instruction.EmitColumn "s"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ textVal "hello world" ] r.Rows.[0]

[<Fact>]
let ``BinaryOpInstr with NULL propagates NULL for arithmetic`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 5L)
        Instruction.LoadConst nullVal
        Instruction.BinaryOpInstr BinaryOp.Add
        Instruction.EmitColumn "x"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

// ── 4. BinaryOpInstr — comparison ─────────────────────────────────────────

[<Fact>]
let ``BinaryOpInstr Eq integers equal`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 5L)
        Instruction.LoadConst (intVal 5L)
        Instruction.BinaryOpInstr BinaryOp.Eq
        Instruction.EmitColumn "eq"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``BinaryOpInstr Eq integers not equal`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 5L)
        Instruction.LoadConst (intVal 6L)
        Instruction.BinaryOpInstr BinaryOp.Eq
        Instruction.EmitColumn "eq"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal false ] r.Rows.[0]

[<Fact>]
let ``BinaryOpInstr Lt`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 3L)
        Instruction.LoadConst (intVal 5L)
        Instruction.BinaryOpInstr BinaryOp.Lt
        Instruction.EmitColumn "lt"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``BinaryOpInstr Gt`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 7L)
        Instruction.LoadConst (intVal 5L)
        Instruction.BinaryOpInstr BinaryOp.Gt
        Instruction.EmitColumn "gt"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``Comparison with NULL returns NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 5L)
        Instruction.LoadConst nullVal
        Instruction.BinaryOpInstr BinaryOp.Eq
        Instruction.EmitColumn "eq"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

// ── 5. BinaryOpInstr — AND / OR (three-valued logic) ──────────────────────

[<Fact>]
let ``AND: FALSE AND NULL = FALSE (short-circuit)`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (boolVal false)
        Instruction.LoadConst nullVal
        Instruction.BinaryOpInstr BinaryOp.And
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal false ] r.Rows.[0]

[<Fact>]
let ``AND: TRUE AND NULL = NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (boolVal true)
        Instruction.LoadConst nullVal
        Instruction.BinaryOpInstr BinaryOp.And
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

[<Fact>]
let ``OR: TRUE OR NULL = TRUE (short-circuit)`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (boolVal true)
        Instruction.LoadConst nullVal
        Instruction.BinaryOpInstr BinaryOp.Or
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``OR: FALSE OR NULL = NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (boolVal false)
        Instruction.LoadConst nullVal
        Instruction.BinaryOpInstr BinaryOp.Or
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

// ── 6. UnaryOpInstr ───────────────────────────────────────────────────────

[<Fact>]
let ``UnaryOp Neg negates integer`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 5L)
        Instruction.UnaryOpInstr UnaryOp.Neg
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal -5L ] r.Rows.[0]

[<Fact>]
let ``UnaryOp Not inverts bool`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (boolVal true)
        Instruction.UnaryOpInstr UnaryOp.Not
        Instruction.EmitColumn "b"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal false ] r.Rows.[0]

[<Fact>]
let ``UnaryOp Not on NULL returns NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst nullVal
        Instruction.UnaryOpInstr UnaryOp.Not
        Instruction.EmitColumn "b"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

// ── 7. IsNull / IsNotNull ─────────────────────────────────────────────────

[<Fact>]
let ``IsNull: NULL value pushes TRUE`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst nullVal
        Instruction.IsNull
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``IsNull: non-NULL value pushes FALSE`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 1L)
        Instruction.IsNull
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal false ] r.Rows.[0]

[<Fact>]
let ``IsNotNull: non-NULL pushes TRUE`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (textVal "hi")
        Instruction.IsNotNull
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``IsNotNull: NULL pushes FALSE`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst nullVal
        Instruction.IsNotNull
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal false ] r.Rows.[0]

// ── 8. Between ───────────────────────────────────────────────────────────

[<Fact>]
let ``Between: value within range pushes TRUE`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 5L)
        Instruction.LoadConst (intVal 1L)
        Instruction.LoadConst (intVal 10L)
        Instruction.Between(inclusive = true)
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``Between: value outside range pushes FALSE`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 15L)
        Instruction.LoadConst (intVal 1L)
        Instruction.LoadConst (intVal 10L)
        Instruction.Between(inclusive = true)
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal false ] r.Rows.[0]

[<Fact>]
let ``Between: NULL value pushes NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst nullVal
        Instruction.LoadConst (intVal 1L)
        Instruction.LoadConst (intVal 10L)
        Instruction.Between(inclusive = true)
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

// ── 9. Like ──────────────────────────────────────────────────────────────

[<Fact>]
let ``Like: percent matches any sequence`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (textVal "hello world")
        Instruction.LoadConst (textVal "hello%")
        Instruction.Like
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``Like: underscore matches single char`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (textVal "abc")
        Instruction.LoadConst (textVal "a_c")
        Instruction.Like
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``Like: no match pushes FALSE`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (textVal "xyz")
        Instruction.LoadConst (textVal "abc%")
        Instruction.Like
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal false ] r.Rows.[0]

[<Fact>]
let ``Like: NULL value pushes NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst nullVal
        Instruction.LoadConst (textVal "%")
        Instruction.Like
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

// ── 10. InList ────────────────────────────────────────────────────────────

[<Fact>]
let ``InList: needle found pushes TRUE`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 2L)
        Instruction.LoadConst (intVal 1L)
        Instruction.LoadConst (intVal 2L)
        Instruction.LoadConst (intVal 3L)
        Instruction.InList 3
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``InList: needle not found pushes FALSE`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 5L)
        Instruction.LoadConst (intVal 1L)
        Instruction.LoadConst (intVal 2L)
        Instruction.LoadConst (intVal 3L)
        Instruction.InList 3
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal false ] r.Rows.[0]

[<Fact>]
let ``InList: empty list pushes FALSE even for NULL needle`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst nullVal
        Instruction.InList 0
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal false ] r.Rows.[0]

[<Fact>]
let ``InList: NULL needle pushes NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst nullVal
        Instruction.LoadConst (intVal 1L)
        Instruction.InList 1
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

// ── 11. Control flow ──────────────────────────────────────────────────────

[<Fact>]
let ``Jump unconditionally redirects pc`` () =
    // Without Jump, the second LoadConst (42) would be pushed.
    // With Jump we skip it entirely and emit 99.
    let r = runFresh [
        Instruction.Jump "end"
        Instruction.LoadConst (intVal 42L)   // skipped
        Instruction.Label "end"
        Instruction.BeginRow
        Instruction.LoadConst (intVal 99L)
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 99L ] r.Rows.[0]

[<Fact>]
let ``JumpIfTrue jumps when value is truthy`` () =
    let r = runFresh [
        Instruction.LoadConst (boolVal true)
        Instruction.JumpIfTrue "skip"
        Instruction.BeginRow
        Instruction.LoadConst (intVal 1L)
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.Label "skip"
        Instruction.Halt
    ]
    // The BeginRow/EmitColumn/EmitRow after JumpIfTrue is skipped.
    Assert.Empty(r.Rows)

[<Fact>]
let ``JumpIfFalse jumps when value is falsy`` () =
    let r = runFresh [
        Instruction.LoadConst (boolVal false)
        Instruction.JumpIfFalse "skip"
        Instruction.BeginRow
        Instruction.LoadConst (intVal 1L)
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.Label "skip"
        Instruction.Halt
    ]
    Assert.Empty(r.Rows)

[<Fact>]
let ``JumpIfFalse does not jump when value is true`` () =
    let r = runFresh [
        Instruction.LoadConst (boolVal true)
        Instruction.JumpIfFalse "skip"
        Instruction.BeginRow
        Instruction.LoadConst (intVal 42L)
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.Label "skip"
        Instruction.Halt
    ]
    Assert.Equal(1, r.Rows.Length)
    assertRow [ intVal 42L ] r.Rows.[0]

// ── 12. Pop ───────────────────────────────────────────────────────────────

[<Fact>]
let ``Pop discards top of stack`` () =
    // Push two values; pop one; emit the remaining one.
    let r = runFresh [
        Instruction.LoadConst (intVal 10L)
        Instruction.LoadConst (intVal 20L)
        Instruction.Pop                          // discard 20
        Instruction.BeginRow
        Instruction.EmitColumn "n"              // emits 10
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 10L ] r.Rows.[0]

// ── 13. DDL (CreateTable / DropTable) ────────────────────────────────────

[<Fact>]
let ``CreateTable creates a table`` () =
    let b = InMemoryBackend()
    run b [
        Instruction.CreateTable("products",
            false,
            [ { Name = "id"; TypeName = "INTEGER"; NotNull = false; PrimaryKey = false; Unique = false; Default = None }
              { Name = "price"; TypeName = "REAL"; NotNull = false; PrimaryKey = false; Unique = false; Default = None } ])
        Instruction.Halt
    ] |> ignore
    let cols = b.Columns("products")
    Assert.Equal(2, cols.Count)
    Assert.Equal("id",    cols.[0].Name)
    Assert.Equal("price", cols.[1].Name)

[<Fact>]
let ``CreateTable IF NOT EXISTS is idempotent`` () =
    let b = InMemoryBackend()
    run b [
        Instruction.CreateTable("t", false,
            [ { Name = "x"; TypeName = "TEXT"; NotNull = false; PrimaryKey = false; Unique = false; Default = None } ])
        Instruction.Halt
    ] |> ignore
    // Second create with ifNotExists = true should not throw.
    run b [
        Instruction.CreateTable("t", true,
            [ { Name = "x"; TypeName = "TEXT"; NotNull = false; PrimaryKey = false; Unique = false; Default = None } ])
        Instruction.Halt
    ] |> ignore
    Assert.Equal(1, b.Tables().Count)

[<Fact>]
let ``DropTable removes the table`` () =
    let b = InMemoryBackend()
    run b [
        Instruction.CreateTable("tmp", false,
            [ { Name = "v"; TypeName = "TEXT"; NotNull = false; PrimaryKey = false; Unique = false; Default = None } ])
        Instruction.Halt
    ] |> ignore
    run b [
        Instruction.DropTable("tmp", false)
        Instruction.Halt
    ] |> ignore
    Assert.Equal(0, b.Tables().Count)

[<Fact>]
let ``DropTable IF EXISTS on missing table does not throw`` () =
    let b = InMemoryBackend()
    run b [
        Instruction.DropTable("nonexistent", true)
        Instruction.Halt
    ] |> ignore

// ── 14. DML — InsertRow ──────────────────────────────────────────────────

[<Fact>]
let ``InsertRow inserts a row and increments RowsAffected`` () =
    let b = usersBackend ()
    let r = run b [
        Instruction.LoadConst (intVal 1L)
        Instruction.LoadConst (textVal "Alice")
        Instruction.InsertRow("users", Some ["id"; "name"])
        Instruction.Halt
    ]
    Assert.Equal(1, r.RowsAffected)
    let rows = b.Scan("users")
    let mutable count = 0
    let mutable next = rows.Next()
    while not (obj.ReferenceEquals(next, null)) do
        count <- count + 1
        next <- rows.Next()
    Assert.Equal(1, count)

[<Fact>]
let ``Multiple InsertRow calls accumulate RowsAffected`` () =
    let b = usersBackend ()
    let r = run b [
        Instruction.LoadConst (intVal 1L)
        Instruction.LoadConst (textVal "Alice")
        Instruction.InsertRow("users", Some ["id"; "name"])
        Instruction.LoadConst (intVal 2L)
        Instruction.LoadConst (textVal "Bob")
        Instruction.InsertRow("users", Some ["id"; "name"])
        Instruction.Halt
    ]
    Assert.Equal(2, r.RowsAffected)

// ── 15. Scan loop ─────────────────────────────────────────────────────────

[<Fact>]
let ``Scan loop over single row emits one output row`` () =
    let b = usersBackend ()
    insertUser b 1L "Alice"
    let r = run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None           // advance after JumpIfExhausted already peeked
        Instruction.BeginRow
        Instruction.LoadColumn(None, "name")
        Instruction.EmitColumn "name"
        Instruction.EmitRow
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ]
    Assert.Equal(1, r.Rows.Length)
    assertRow [ textVal "Alice" ] r.Rows.[0]

[<Fact>]
let ``Scan loop over empty table emits no rows`` () =
    let b = usersBackend ()
    let r = run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.BeginRow
        Instruction.LoadColumn(None, "name")
        Instruction.EmitColumn "name"
        Instruction.EmitRow
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ]
    Assert.Empty(r.Rows)

[<Fact>]
let ``Scan loop over multiple rows emits all rows`` () =
    let b = usersBackend ()
    insertUser b 1L "Alice"
    insertUser b 2L "Bob"
    insertUser b 3L "Carol"
    let r = run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.BeginRow
        Instruction.LoadColumn(None, "id")
        Instruction.EmitColumn "id"
        Instruction.EmitRow
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ]
    Assert.Equal(3, r.Rows.Length)

[<Fact>]
let ``Scan loop with filter skips non-matching rows`` () =
    let b = usersBackend ()
    insertUser b 1L "Alice"
    insertUser b 2L "Bob"
    let r = run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        // Filter: WHERE id = 2
        Instruction.LoadColumn(None, "id")
        Instruction.LoadConst (intVal 2L)
        Instruction.BinaryOpInstr BinaryOp.Eq
        Instruction.JumpIfFalse "loop"         // skip to next row if false
        Instruction.BeginRow
        Instruction.LoadColumn(None, "name")
        Instruction.EmitColumn "name"
        Instruction.EmitRow
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ]
    Assert.Equal(1, r.Rows.Length)
    assertRow [ textVal "Bob" ] r.Rows.[0]

// ── 16. Aggregate instructions ────────────────────────────────────────────

[<Fact>]
let ``COUNT STAR over three rows returns 3`` () =
    let b = usersBackend ()
    insertUser b 1L "Alice"
    insertUser b 2L "Bob"
    insertUser b 3L "Carol"
    let r = run b [
        Instruction.InitAgg 1
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateAgg(0, AggFn.CountStar)
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.CountStar)
        Instruction.EmitColumn "count"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 3L ] r.Rows.[0]

[<Fact>]
let ``SUM of integers over scan returns correct sum`` () =
    let b = usersBackend ()
    insertUser b 10L "A"
    insertUser b 20L "B"
    insertUser b 30L "C"
    let r = run b [
        Instruction.InitAgg 1
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.LoadColumn(None, "id")
        Instruction.UpdateAgg(0, AggFn.Sum)
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.Sum)
        Instruction.EmitColumn "sum"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 60L ] r.Rows.[0]

[<Fact>]
let ``MIN over scan returns smallest value`` () =
    let b = usersBackend ()
    insertUser b 5L "A"
    insertUser b 2L "B"
    insertUser b 8L "C"
    let r = run b [
        Instruction.InitAgg 1
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.LoadColumn(None, "id")
        Instruction.UpdateAgg(0, AggFn.Min)
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.Min)
        Instruction.EmitColumn "min"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 2L ] r.Rows.[0]

[<Fact>]
let ``MAX over scan returns largest value`` () =
    let b = usersBackend ()
    insertUser b 5L "A"
    insertUser b 2L "B"
    insertUser b 8L "C"
    let r = run b [
        Instruction.InitAgg 1
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.LoadColumn(None, "id")
        Instruction.UpdateAgg(0, AggFn.Max)
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.Max)
        Instruction.EmitColumn "max"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 8L ] r.Rows.[0]

[<Fact>]
let ``AVG over integers returns real`` () =
    let b = usersBackend ()
    insertUser b 10L "A"
    insertUser b 20L "B"
    let r = run b [
        Instruction.InitAgg 1
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.LoadColumn(None, "id")
        Instruction.UpdateAgg(0, AggFn.Avg)
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.Avg)
        Instruction.EmitColumn "avg"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 15.0 ] r.Rows.[0]

[<Fact>]
let ``COUNT STAR over empty table returns 0`` () =
    let b = usersBackend ()
    let r = run b [
        Instruction.InitAgg 1
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateAgg(0, AggFn.CountStar)
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.CountStar)
        Instruction.EmitColumn "count"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 0L ] r.Rows.[0]

[<Fact>]
let ``SUM over empty table returns NULL`` () =
    let b = usersBackend ()
    let r = run b [
        Instruction.InitAgg 1
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.LoadColumn(None, "id")
        Instruction.UpdateAgg(0, AggFn.Sum)
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.Sum)
        Instruction.EmitColumn "sum"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

// ── 17. Post-processing ───────────────────────────────────────────────────

[<Fact>]
let ``LimitResult limits output rows`` () =
    let b = usersBackend ()
    insertUser b 1L "A"
    insertUser b 2L "B"
    insertUser b 3L "C"
    let r = run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.BeginRow
        Instruction.LoadColumn(None, "id")
        Instruction.EmitColumn "id"
        Instruction.EmitRow
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.LimitResult(Some 2L, None)
        Instruction.Halt
    ]
    Assert.Equal(2, r.Rows.Length)

[<Fact>]
let ``LimitResult with offset skips rows`` () =
    let b = usersBackend ()
    insertUser b 1L "A"
    insertUser b 2L "B"
    insertUser b 3L "C"
    let r = run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.BeginRow
        Instruction.LoadColumn(None, "id")
        Instruction.EmitColumn "id"
        Instruction.EmitRow
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.LimitResult(Some 1L, Some 1L)
        Instruction.Halt
    ]
    Assert.Equal(1, r.Rows.Length)
    assertRow [ intVal 2L ] r.Rows.[0]

[<Fact>]
let ``DistinctResult removes duplicate rows`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 1L)
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.BeginRow
        Instruction.LoadConst (intVal 1L)
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.BeginRow
        Instruction.LoadConst (intVal 2L)
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.DistinctResult
        Instruction.Halt
    ]
    Assert.Equal(2, r.Rows.Length)

// ── 18. SaveGroupKey / LoadGroupKey ──────────────────────────────────────

[<Fact>]
let ``SaveGroupKey and LoadGroupKey round-trip values`` () =
    let r = runFresh [
        Instruction.LoadConst (textVal "grp1")
        Instruction.SaveGroupKey ["key"]
        Instruction.BeginRow
        Instruction.LoadGroupKey 0
        Instruction.EmitColumn "key"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ textVal "grp1" ] r.Rows.[0]

// ── 19. Transaction control ───────────────────────────────────────────────

[<Fact>]
let ``BeginTransaction and CommitTransaction commit changes`` () =
    let b = usersBackend ()
    run b [
        Instruction.BeginTransaction
        Instruction.LoadConst (intVal 99L)
        Instruction.LoadConst (textVal "Tx")
        Instruction.InsertRow("users", Some ["id"; "name"])
        Instruction.CommitTransaction
        Instruction.Halt
    ] |> ignore
    // After commit, the row should be visible.
    let it = b.Scan("users")
    let row = it.Next()
    Assert.NotNull(row)

[<Fact>]
let ``RollbackTransaction undoes changes`` () =
    let b = usersBackend ()
    run b [
        Instruction.BeginTransaction
        Instruction.LoadConst (intVal 99L)
        Instruction.LoadConst (textVal "Tx")
        Instruction.InsertRow("users", Some ["id"; "name"])
        Instruction.RollbackTransaction
        Instruction.Halt
    ] |> ignore
    // After rollback, no rows should exist.
    let it = b.Scan("users")
    let row = it.Next()
    Assert.True(obj.ReferenceEquals(row, null))

// ── 20. Multiple columns ──────────────────────────────────────────────────

[<Fact>]
let ``EmitRow with multiple columns sets Columns list`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 1L)
        Instruction.EmitColumn "id"
        Instruction.LoadConst (textVal "Alice")
        Instruction.EmitColumn "name"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertCols [ "id"; "name" ] r.Columns
    assertRow [ intVal 1L; textVal "Alice" ] r.Rows.[0]

[<Fact>]
let ``Multiple EmitRow calls build multiple rows`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 1L)
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.BeginRow
        Instruction.LoadConst (intVal 2L)
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.Halt
    ]
    Assert.Equal(2, r.Rows.Length)
    assertRow [ intVal 1L ] r.Rows.[0]
    assertRow [ intVal 2L ] r.Rows.[1]

// ── 21. Real arithmetic ───────────────────────────────────────────────────

[<Fact>]
let ``Integer plus Real promotes to Real`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 3L)
        Instruction.LoadConst (realVal 0.5)
        Instruction.BinaryOpInstr BinaryOp.Add
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 3.5 ] r.Rows.[0]

[<Fact>]
let ``Neg on Real negates`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (realVal 2.5)
        Instruction.UnaryOpInstr UnaryOp.Neg
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal -2.5 ] r.Rows.[0]
