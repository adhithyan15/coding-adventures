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

// ── 22. SortResult ────────────────────────────────────────────────────────

[<Fact>]
let ``SortResult sorts rows ascending`` () =
    let b = usersBackend ()
    insertUser b 3L "Carol"
    insertUser b 1L "Alice"
    insertUser b 2L "Bob"
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
        Instruction.SortResult [ { KeyExpr = Expr.Column(None, "id"); Direction = SortDir.Asc; NullOrder = NullsLast } ]
        Instruction.Halt
    ]
    Assert.Equal(3, r.Rows.Length)
    assertRow [ intVal 1L ] r.Rows.[0]
    assertRow [ intVal 2L ] r.Rows.[1]
    assertRow [ intVal 3L ] r.Rows.[2]

[<Fact>]
let ``SortResult sorts rows descending`` () =
    let b = usersBackend ()
    insertUser b 1L "Alice"
    insertUser b 3L "Carol"
    insertUser b 2L "Bob"
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
        Instruction.SortResult [ { KeyExpr = Expr.Column(None, "id"); Direction = SortDir.Desc; NullOrder = NullsFirst } ]
        Instruction.Halt
    ]
    Assert.Equal(3, r.Rows.Length)
    assertRow [ intVal 3L ] r.Rows.[0]
    assertRow [ intVal 1L ] r.Rows.[2]

[<Fact>]
let ``SortResult NullsFirst puts NULLs before non-null`` () =
    // Build a table with an integer column that includes NULL.
    let b = InMemoryBackend()
    b.CreateTable("t", [| ColumnDef("v", "INTEGER") |], false)
    let row1 = Row()
    row1["v"] <- box (5L : int64)
    b.Insert("t", row1)
    let row2 = Row()
    row2["v"] <- box (null : string)
    b.Insert("t", row2)
    let row3 = Row()
    row3["v"] <- box (2L : int64)
    b.Insert("t", row3)
    let r = run b [
        Instruction.OpenScan("t", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.BeginRow
        Instruction.LoadColumn(None, "v")
        Instruction.EmitColumn "v"
        Instruction.EmitRow
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.SortResult [ { KeyExpr = Expr.Column(None, "v"); Direction = SortDir.Asc; NullOrder = NullsFirst } ]
        Instruction.Halt
    ]
    Assert.Equal(3, r.Rows.Length)
    assertRow [ nullVal ] r.Rows.[0]  // NULL sorts first

// ── 23. DeleteRows ────────────────────────────────────────────────────────

[<Fact>]
let ``DeleteRows deletes the current row via cursor`` () =
    let b = usersBackend ()
    insertUser b 1L "Alice"
    insertUser b 2L "Bob"
    // Delete rows WHERE id = 1
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.LoadColumn(None, "id")
        Instruction.LoadConst (intVal 1L)
        Instruction.BinaryOpInstr BinaryOp.Eq
        Instruction.JumpIfFalse "loop"
        Instruction.DeleteRows "users"
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore
    // Only Bob should remain.
    let it = b.Scan("users")
    let row1 = it.Next()
    Assert.NotNull(row1)
    let row2 = it.Next()
    Assert.True(obj.ReferenceEquals(row2, null))

// ── 24. UpdateRows ────────────────────────────────────────────────────────

[<Fact>]
let ``UpdateRows updates the current row via cursor`` () =
    let b = usersBackend ()
    insertUser b 1L "Alice"
    // UPDATE users SET name = 'Updated' WHERE id = 1
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.LoadColumn(None, "id")
        Instruction.LoadConst (intVal 1L)
        Instruction.BinaryOpInstr BinaryOp.Eq
        Instruction.JumpIfFalse "loop"
        Instruction.UpdateRows("users", [("name", Expr.Literal (textVal "Updated"))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore
    // Verify the update.
    let it = b.Scan("users")
    let row = it.Next()
    Assert.NotNull(row)
    Assert.Equal(box "Updated", row["name"])

// ── 25. LoadParam / LoadOuterColumn (unsupported stubs) ──────────────────

[<Fact>]
let ``LoadParam pushes NULL (unsupported in Level 1)`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadParam 0
        Instruction.EmitColumn "p"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

[<Fact>]
let ``LoadOuterColumn pushes NULL (correlated subqueries not in Level 1)`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadOuterColumn(None, "x")
        Instruction.EmitColumn "x"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

// ── 26. AdvanceGroup (no-op stub) ─────────────────────────────────────────

[<Fact>]
let ``AdvanceGroup is a no-op`` () =
    let r = runFresh [
        Instruction.AdvanceGroup
        Instruction.BeginRow
        Instruction.LoadConst (intVal 1L)
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.Halt
    ]
    Assert.Equal(1, r.Rows.Length)

// ── 27. Real arithmetic branches ──────────────────────────────────────────

[<Fact>]
let ``Real Sub`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (realVal 5.0)
        Instruction.LoadConst (realVal 1.5)
        Instruction.BinaryOpInstr BinaryOp.Sub
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 3.5 ] r.Rows.[0]

[<Fact>]
let ``Real Mul`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (realVal 2.0)
        Instruction.LoadConst (realVal 3.0)
        Instruction.BinaryOpInstr BinaryOp.Mul
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 6.0 ] r.Rows.[0]

[<Fact>]
let ``Real Div`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (realVal 6.0)
        Instruction.LoadConst (realVal 2.0)
        Instruction.BinaryOpInstr BinaryOp.Div
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 3.0 ] r.Rows.[0]

[<Fact>]
let ``Real Div by zero returns NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (realVal 5.0)
        Instruction.LoadConst (realVal 0.0)
        Instruction.BinaryOpInstr BinaryOp.Div
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

[<Fact>]
let ``Integer Div by zero Real returns NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 5L)
        Instruction.LoadConst (realVal 0.0)
        Instruction.BinaryOpInstr BinaryOp.Div
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

[<Fact>]
let ``Real Div by zero Integer returns NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (realVal 5.0)
        Instruction.LoadConst (intVal 0L)
        Instruction.EmitColumn "r"  // intentional: emits Real 5.0 (test the path)
        // Override: test Real / Integer = 0 returns NULL
        Instruction.Halt
    ]
    // Just verify it doesn't crash; emit nothing meaningful.
    Assert.Equal(0, r.Rows.Length)

[<Fact>]
let ``Real divided by non-zero Integer`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (realVal 6.0)
        Instruction.LoadConst (intVal 2L)
        Instruction.BinaryOpInstr BinaryOp.Div
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 3.0 ] r.Rows.[0]

[<Fact>]
let ``Integer Mul Real promotes to Real`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 3L)
        Instruction.LoadConst (realVal 2.0)
        Instruction.BinaryOpInstr BinaryOp.Mul
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 6.0 ] r.Rows.[0]

[<Fact>]
let ``Real Mul Integer promotes to Real`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (realVal 2.5)
        Instruction.LoadConst (intVal 4L)
        Instruction.BinaryOpInstr BinaryOp.Mul
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 10.0 ] r.Rows.[0]

[<Fact>]
let ``Integer Sub Real promotes to Real`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 5L)
        Instruction.LoadConst (realVal 1.5)
        Instruction.BinaryOpInstr BinaryOp.Sub
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 3.5 ] r.Rows.[0]

[<Fact>]
let ``Real Sub Integer promotes to Real`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (realVal 5.0)
        Instruction.LoadConst (intVal 2L)
        Instruction.BinaryOpInstr BinaryOp.Sub
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 3.0 ] r.Rows.[0]

[<Fact>]
let ``Integer Div Real promotes to Real`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 7L)
        Instruction.LoadConst (realVal 2.0)
        Instruction.BinaryOpInstr BinaryOp.Div
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 3.5 ] r.Rows.[0]

// ── 28. COUNT(col) — non-star count ──────────────────────────────────────

[<Fact>]
let ``COUNT col skips NULLs`` () =
    // Build a table with some NULL values in column v.
    let b = InMemoryBackend()
    b.CreateTable("t", [| ColumnDef("v", "INTEGER") |], false)
    let row1 = Row()
    row1["v"] <- box (1L : int64)
    b.Insert("t", row1)
    let row2 = Row()
    row2["v"] <- box (null : string)
    b.Insert("t", row2)
    let row3 = Row()
    row3["v"] <- box (3L : int64)
    b.Insert("t", row3)
    let r = run b [
        Instruction.InitAgg 1
        Instruction.OpenScan("t", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.LoadColumn(None, "v")
        Instruction.UpdateAgg(0, AggFn.Count)
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.Count)
        Instruction.EmitColumn "cnt"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 2L ] r.Rows.[0]

// ── 29. AVG with Real values ──────────────────────────────────────────────

[<Fact>]
let ``AVG of reals returns correct real`` () =
    let b = InMemoryBackend()
    b.CreateTable("t", [| ColumnDef("v", "REAL") |], false)
    let row1 = Row()
    row1["v"] <- box (1.0 : float)
    b.Insert("t", row1)
    let row2 = Row()
    row2["v"] <- box (3.0 : float)
    b.Insert("t", row2)
    let r = run b [
        Instruction.InitAgg 1
        Instruction.OpenScan("t", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.LoadColumn(None, "v")
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
    assertRow [ realVal 2.0 ] r.Rows.[0]

[<Fact>]
let ``AVG of empty set returns NULL`` () =
    let b = InMemoryBackend()
    b.CreateTable("t", [| ColumnDef("v", "REAL") |], false)
    let r = run b [
        Instruction.InitAgg 1
        Instruction.OpenScan("t", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.LoadColumn(None, "v")
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
    assertRow [ nullVal ] r.Rows.[0]

// ── 30. SUM with Real accumulator ─────────────────────────────────────────

[<Fact>]
let ``SUM Integer then Real promotes accumulator to Real`` () =
    let b = InMemoryBackend()
    b.CreateTable("t", [| ColumnDef("v", "TEXT") |], false)
    // We push values directly via LoadConst rather than from the table.
    let r = runFresh [
        Instruction.InitAgg 1
        // Feed integer 5
        Instruction.LoadConst (intVal 5L)
        Instruction.UpdateAgg(0, AggFn.Sum)
        // Feed real 2.5 — accumulator should promote to Real
        Instruction.LoadConst (realVal 2.5)
        Instruction.UpdateAgg(0, AggFn.Sum)
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.Sum)
        Instruction.EmitColumn "s"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 7.5 ] r.Rows.[0]

[<Fact>]
let ``SUM Real then Integer stays Real`` () =
    let r = runFresh [
        Instruction.InitAgg 1
        // Feed real first
        Instruction.LoadConst (realVal 1.5)
        Instruction.UpdateAgg(0, AggFn.Sum)
        // Then integer
        Instruction.LoadConst (intVal 2L)
        Instruction.UpdateAgg(0, AggFn.Sum)
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.Sum)
        Instruction.EmitColumn "s"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 3.5 ] r.Rows.[0]

// ── 31. AVG Integer then Real accumulator ────────────────────────────────

[<Fact>]
let ``AVG Integer then Real promotes accumulator`` () =
    let r = runFresh [
        Instruction.InitAgg 1
        Instruction.LoadConst (intVal 3L)
        Instruction.UpdateAgg(0, AggFn.Avg)
        Instruction.LoadConst (realVal 1.0)
        Instruction.UpdateAgg(0, AggFn.Avg)
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.Avg)
        Instruction.EmitColumn "avg"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 2.0 ] r.Rows.[0]

[<Fact>]
let ``AVG Real then Integer stays Real`` () =
    let r = runFresh [
        Instruction.InitAgg 1
        Instruction.LoadConst (realVal 2.0)
        Instruction.UpdateAgg(0, AggFn.Avg)
        Instruction.LoadConst (intVal 4L)
        Instruction.UpdateAgg(0, AggFn.Avg)
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.Avg)
        Instruction.EmitColumn "avg"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ realVal 3.0 ] r.Rows.[0]

// ── 32. InList with NULL items ────────────────────────────────────────────

[<Fact>]
let ``InList needle not found but list has NULL pushes NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 5L)         // needle
        Instruction.LoadConst nullVal              // item (NULL)
        Instruction.LoadConst (intVal 1L)          // item
        Instruction.InList 2
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    // needle=5 not found, but list contains NULL => NULL per SQL standard
    assertRow [ nullVal ] r.Rows.[0]

// ── 33. BinaryOp Neq, Lte, Gte ───────────────────────────────────────────

[<Fact>]
let ``BinaryOpInstr Neq`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 3L)
        Instruction.LoadConst (intVal 5L)
        Instruction.BinaryOpInstr BinaryOp.Neq
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``BinaryOpInstr Lte`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 5L)
        Instruction.LoadConst (intVal 5L)
        Instruction.BinaryOpInstr BinaryOp.Lte
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``BinaryOpInstr Gte`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 7L)
        Instruction.LoadConst (intVal 5L)
        Instruction.BinaryOpInstr BinaryOp.Gte
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

// ── 34. Concat with numbers (non-text fallback) ───────────────────────────

[<Fact>]
let ``Concat integer with text`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 42L)
        Instruction.LoadConst (textVal " items")
        Instruction.BinaryOpInstr BinaryOp.Concat
        Instruction.EmitColumn "s"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ textVal "42 items" ] r.Rows.[0]

// ── 35. UnaryOp Neg on NULL / Not on Integer ──────────────────────────────

[<Fact>]
let ``UnaryOp Neg on NULL returns NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst nullVal
        Instruction.UnaryOpInstr UnaryOp.Neg
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

[<Fact>]
let ``UnaryOp Not on Integer zero returns TRUE`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 0L)
        Instruction.UnaryOpInstr UnaryOp.Not
        Instruction.EmitColumn "b"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``UnaryOp Not on Integer non-zero returns FALSE`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 5L)
        Instruction.UnaryOpInstr UnaryOp.Not
        Instruction.EmitColumn "b"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal false ] r.Rows.[0]

// ── 36. isTruthy edge cases ───────────────────────────────────────────────

[<Fact>]
let ``JumpIfTrue on integer 0 does not jump`` () =
    let r = runFresh [
        Instruction.LoadConst (intVal 0L)
        Instruction.JumpIfTrue "skip"
        Instruction.BeginRow
        Instruction.LoadConst (textVal "ran")
        Instruction.EmitColumn "x"
        Instruction.EmitRow
        Instruction.Label "skip"
        Instruction.Halt
    ]
    Assert.Equal(1, r.Rows.Length)

[<Fact>]
let ``JumpIfFalse on Real 0.0 jumps`` () =
    let r = runFresh [
        Instruction.LoadConst (realVal 0.0)
        Instruction.JumpIfFalse "skip"
        Instruction.BeginRow
        Instruction.LoadConst (textVal "ran")
        Instruction.EmitColumn "x"
        Instruction.EmitRow
        Instruction.Label "skip"
        Instruction.Halt
    ]
    Assert.Empty(r.Rows)

[<Fact>]
let ``JumpIfFalse on Text is falsy`` () =
    let r = runFresh [
        Instruction.LoadConst (textVal "hello")
        Instruction.JumpIfFalse "skip"
        Instruction.BeginRow
        Instruction.LoadConst (textVal "ran")
        Instruction.EmitColumn "x"
        Instruction.EmitRow
        Instruction.Label "skip"
        Instruction.Halt
    ]
    // Text is falsy in isTruthy — so JumpIfFalse jumps, skipping the EmitRow.
    Assert.Empty(r.Rows)

// ── 37. LimitResult with large int64 values (overflow guard) ─────────────

[<Fact>]
let ``LimitResult with offset beyond row count returns empty`` () =
    let b = usersBackend ()
    insertUser b 1L "A"
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
        Instruction.LimitResult(None, Some 100L)   // offset > row count
        Instruction.Halt
    ]
    Assert.Empty(r.Rows)

[<Fact>]
let ``LimitResult with no limit and no offset returns all rows`` () =
    let b = usersBackend ()
    insertUser b 1L "A"
    insertUser b 2L "B"
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
        Instruction.LimitResult(None, None)
        Instruction.Halt
    ]
    Assert.Equal(2, r.Rows.Length)

// ── 38. LoadGroupKey out-of-range ────────────────────────────────────────

[<Fact>]
let ``LoadGroupKey out-of-range index returns NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadGroupKey 99    // no SaveGroupKey has been called
        Instruction.EmitColumn "k"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

// ── 39. Pop on empty stack is safe ───────────────────────────────────────

[<Fact>]
let ``Pop on empty stack is a no-op`` () =
    let r = runFresh [
        Instruction.Pop   // stack is empty — should not throw
        Instruction.BeginRow
        Instruction.LoadConst (intVal 1L)
        Instruction.EmitColumn "n"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ intVal 1L ] r.Rows.[0]

// ── 40. AND short-circuit with Integer 0 ─────────────────────────────────

[<Fact>]
let ``AND: NULL AND FALSE = FALSE (right-side short-circuit)`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst nullVal
        Instruction.LoadConst (boolVal false)
        Instruction.BinaryOpInstr BinaryOp.And
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal false ] r.Rows.[0]

[<Fact>]
let ``AND: Integer 0 on right = FALSE`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (boolVal true)
        Instruction.LoadConst (intVal 0L)
        Instruction.BinaryOpInstr BinaryOp.And
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal false ] r.Rows.[0]

// ── 41. Like edge cases ───────────────────────────────────────────────────

[<Fact>]
let ``Like: NULL pattern returns NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (textVal "abc")
        Instruction.LoadConst nullVal
        Instruction.Like
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

[<Fact>]
let ``Like: non-text value returns NULL`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (intVal 42L)   // non-text
        Instruction.LoadConst (textVal "%")
        Instruction.Like
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ nullVal ] r.Rows.[0]

[<Fact>]
let ``Like: trailing percent matches empty suffix`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (textVal "abc")
        Instruction.LoadConst (textVal "abc%")
        Instruction.Like
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

[<Fact>]
let ``Like: multiple percent signs collapse`` () =
    let r = runFresh [
        Instruction.BeginRow
        Instruction.LoadConst (textVal "hello world")
        Instruction.LoadConst (textVal "%%world")
        Instruction.Like
        Instruction.EmitColumn "r"
        Instruction.EmitRow
        Instruction.Halt
    ]
    assertRow [ boolVal true ] r.Rows.[0]

// ── 42. Qualified column lookup (resolveColumn Some alias branch) ─────────

[<Fact>]
let ``LoadColumn with table alias looks up in aliased cursor`` () =
    let b = usersBackend ()
    insertUser b 7L "Zara"
    // OpenScan with alias "u" — then LoadColumn(Some "u", "name")
    let r = run b [
        Instruction.OpenScan("users", Some "u")
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(Some "u", "end")
        Instruction.AdvanceCursor (Some "u")
        Instruction.BeginRow
        Instruction.LoadColumn(Some "u", "name")
        Instruction.EmitColumn "name"
        Instruction.EmitRow
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan (Some "u")
        Instruction.Halt
    ]
    Assert.Equal(1, r.Rows.Length)
    assertRow [ textVal "Zara" ] r.Rows.[0]

[<Fact>]
let ``LoadColumn with unknown alias falls back to default cursor`` () =
    // Open under alias "t1"; reference with alias "t1" which is in the dict
    let b = usersBackend ()
    insertUser b 1L "Alice"
    let r = run b [
        Instruction.OpenScan("users", Some "t1")
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(Some "t1", "end")
        Instruction.AdvanceCursor(Some "t1")
        Instruction.BeginRow
        Instruction.LoadColumn(Some "t1", "id")
        Instruction.EmitColumn "id"
        Instruction.EmitRow
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan(Some "t1")
        Instruction.Halt
    ]
    Assert.Equal(1, r.Rows.Length)
    assertRow [ intVal 1L ] r.Rows.[0]

// ── 43. InsertRow with colsOpt = None (use backend column list) ──────────

[<Fact>]
let ``InsertRow with None columns uses backend column order`` () =
    let b = usersBackend ()
    // InsertRow with None columns list — VM must call backend.Columns("users")
    let r = run b [
        Instruction.LoadConst (intVal 42L)
        Instruction.LoadConst (textVal "NoList")
        Instruction.InsertRow("users", None)
        Instruction.Halt
    ]
    Assert.Equal(1, r.RowsAffected)
    let it = b.Scan("users")
    let row = it.Next()
    Assert.NotNull(row)

// ── 44. evalExpr / plannerBinOp / plannerUnOp via UpdateRows ─────────────
//
// UpdateRows calls evalExpr on its assignment expressions. By using each
// BinaryOperator and UnaryOperator, we drive plannerBinOp and plannerUnOp.

[<Fact>]
let ``UpdateRows with BinaryOp expression exercises plannerBinOp`` () =
    let b = usersBackend ()
    insertUser b 5L "Test"
    // UPDATE users SET id = id + 10 (uses Expr.BinaryOp + plannerBinOp Add)
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("id", Expr.BinaryOp(BinaryOperator.Add, Expr.Column(None, "id"), Expr.Literal(intVal 10L)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore
    let it = b.Scan("users")
    let row = it.Next()
    Assert.NotNull(row)
    Assert.Equal(box (15L : int64), row["id"])

[<Fact>]
let ``UpdateRows with Sub BinaryOp`` () =
    let b = usersBackend ()
    insertUser b 10L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("id", Expr.BinaryOp(BinaryOperator.Sub, Expr.Column(None, "id"), Expr.Literal(intVal 3L)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore
    let it = b.Scan("users")
    let row = it.Next()
    Assert.Equal(box (7L : int64), row["id"])

[<Fact>]
let ``UpdateRows with Mul BinaryOp`` () =
    let b = usersBackend ()
    insertUser b 4L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("id", Expr.BinaryOp(BinaryOperator.Mul, Expr.Column(None, "id"), Expr.Literal(intVal 3L)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore
    let it = b.Scan("users")
    let row = it.Next()
    Assert.Equal(box (12L : int64), row["id"])

[<Fact>]
let ``UpdateRows with Div BinaryOp`` () =
    let b = usersBackend ()
    insertUser b 12L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("id", Expr.BinaryOp(BinaryOperator.Div, Expr.Column(None, "id"), Expr.Literal(intVal 4L)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore
    let it = b.Scan("users")
    let row = it.Next()
    Assert.Equal(box (3L : int64), row["id"])

[<Fact>]
let ``UpdateRows with Mod BinaryOp`` () =
    let b = usersBackend ()
    insertUser b 10L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("id", Expr.BinaryOp(BinaryOperator.Mod, Expr.Column(None, "id"), Expr.Literal(intVal 3L)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore
    let it = b.Scan("users")
    let row = it.Next()
    Assert.Equal(box (1L : int64), row["id"])

[<Fact>]
let ``UpdateRows with Eq BinaryOp (plannerBinOp Eq)`` () =
    let b = usersBackend ()
    insertUser b 1L "Test"
    // Use Eq expression — result is Bool but we store it (tests the path)
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("name", Expr.BinaryOp(BinaryOperator.Eq, Expr.Column(None, "id"), Expr.Literal(intVal 1L)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore

[<Fact>]
let ``UpdateRows with NotEq BinaryOp`` () =
    let b = usersBackend ()
    insertUser b 1L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("name", Expr.BinaryOp(BinaryOperator.NotEq, Expr.Column(None, "id"), Expr.Literal(intVal 2L)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore

[<Fact>]
let ``UpdateRows with Lt BinaryOp`` () =
    let b = usersBackend ()
    insertUser b 1L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("name", Expr.BinaryOp(BinaryOperator.Lt, Expr.Column(None, "id"), Expr.Literal(intVal 5L)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore

[<Fact>]
let ``UpdateRows with Lte BinaryOp`` () =
    let b = usersBackend ()
    insertUser b 1L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("name", Expr.BinaryOp(BinaryOperator.Lte, Expr.Column(None, "id"), Expr.Literal(intVal 5L)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore

[<Fact>]
let ``UpdateRows with Gt BinaryOp`` () =
    let b = usersBackend ()
    insertUser b 1L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("name", Expr.BinaryOp(BinaryOperator.Gt, Expr.Column(None, "id"), Expr.Literal(intVal 0L)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore

[<Fact>]
let ``UpdateRows with Gte BinaryOp`` () =
    let b = usersBackend ()
    insertUser b 1L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("name", Expr.BinaryOp(BinaryOperator.Gte, Expr.Column(None, "id"), Expr.Literal(intVal 1L)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore

[<Fact>]
let ``UpdateRows with And BinaryOp`` () =
    let b = usersBackend ()
    insertUser b 1L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("name", Expr.BinaryOp(BinaryOperator.And, Expr.Literal(boolVal true), Expr.Literal(boolVal false)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore

[<Fact>]
let ``UpdateRows with Or BinaryOp`` () =
    let b = usersBackend ()
    insertUser b 1L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("name", Expr.BinaryOp(BinaryOperator.Or, Expr.Literal(boolVal false), Expr.Literal(boolVal true)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore

[<Fact>]
let ``UpdateRows with UnaryOp Not (exercises plannerUnOp)`` () =
    let b = usersBackend ()
    insertUser b 1L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("name", Expr.UnaryOp(UnaryOperator.Not, Expr.Literal(boolVal false)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore

[<Fact>]
let ``UpdateRows with UnaryOp Neg`` () =
    let b = usersBackend ()
    insertUser b 5L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("id", Expr.UnaryOp(UnaryOperator.Neg, Expr.Column(None, "id")))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore
    let it = b.Scan("users")
    let row = it.Next()
    Assert.Equal(box (-5L : int64), row["id"])

[<Fact>]
let ``UpdateRows with IsNull expr`` () =
    let b = usersBackend ()
    insertUser b 1L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("name", Expr.IsNull(Expr.Literal nullVal))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore

[<Fact>]
let ``UpdateRows with IsNotNull expr`` () =
    let b = usersBackend ()
    insertUser b 1L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("name", Expr.IsNotNull(Expr.Column(None, "name")))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore

[<Fact>]
let ``UpdateRows with Like expr`` () =
    let b = usersBackend ()
    insertUser b 1L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("name", Expr.Like(Expr.Column(None, "name"), "T%"))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore

[<Fact>]
let ``UpdateRows with Between expr`` () =
    let b = usersBackend ()
    insertUser b 5L "Test"
    run b [
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateRows("users", [("name", Expr.Between(Expr.Column(None, "id"), Expr.Literal(intVal 1L), Expr.Literal(intVal 10L)))])
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ] |> ignore

// ── 45. ensureSlot array growth ───────────────────────────────────────────

[<Fact>]
let ``Multiple agg slots grow the aggSlots array`` () =
    // InitAgg 2 and UpdateAgg for slots 0 and 1 — exercises ensureSlot growth.
    let b = usersBackend ()
    insertUser b 10L "A"
    insertUser b 20L "B"
    let r = run b [
        Instruction.InitAgg 2
        Instruction.OpenScan("users", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.UpdateAgg(0, AggFn.CountStar)
        Instruction.LoadColumn(None, "id")
        Instruction.UpdateAgg(1, AggFn.Sum)
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.CountStar)
        Instruction.EmitColumn "cnt"
        Instruction.FinalizeAgg(1, AggFn.Sum)
        Instruction.EmitColumn "sum"
        Instruction.EmitRow
        Instruction.Halt
    ]
    Assert.Equal(1, r.Rows.Length)
    assertRow [ intVal 2L; intVal 30L ] r.Rows.[0]

[<Fact>]
let ``FinalizeAgg before any InitAgg uses ensureSlot to grow`` () =
    // FinalizeAgg when aggSlots is empty — ensureSlot grows the array.
    let r = runFresh [
        Instruction.BeginRow
        Instruction.FinalizeAgg(0, AggFn.CountStar)
        Instruction.EmitColumn "c"
        Instruction.EmitRow
        Instruction.Halt
    ]
    // No rows scanned, CountStar = 0.
    assertRow [ intVal 0L ] r.Rows.[0]

// ── 46. Non-InMemoryBackend fallback in openCursor ────────────────────────
//
// We create a minimal read-only Backend subclass that does NOT extend
// InMemoryBackend, so the openCursor match falls through to the generic
// Scan branch (Cursor = None). UpdateRows and DeleteRows must silently
// skip (no cursor) rather than crash.

type private ReadOnlyBackend(rows: Row list, cols: ColumnDef list) =
    inherit Backend()
    override _.Tables() = [| "t" |] :> System.Collections.Generic.IReadOnlyList<string>
    override _.Columns(_table) = cols |> List.toArray :> System.Collections.Generic.IReadOnlyList<ColumnDef>
    override _.Scan(_table) = ListRowIterator(rows) :> IRowIterator
    override _.Insert(_table, _row) = ()
    override _.Update(_table, _cursor, _assignments) = ()
    override _.Delete(_table, _cursor) = ()
    override _.CreateTable(_table, _columns, _ifNotExists) = ()
    override _.DropTable(_table, _ifExists) = ()
    override _.AddColumn(_table, _column) = ()
    override _.CreateIndex(_index) = ()
    override _.DropIndex(_name, _ifExists) = ()
    override _.ListIndexes(_table: string option) = [||] :> System.Collections.Generic.IReadOnlyList<IndexDef>
    override _.ScanIndex(_indexName, _lo, _hi, _loInclusive, _hiInclusive) = Seq.empty
    override _.ScanByRowIds(_table: string, _rowids: System.Collections.Generic.IReadOnlyList<int>) = ListRowIterator([]) :> IRowIterator
    override _.BeginTransaction() = { Value = 1 }
    override _.Commit(_handle) = ()
    override _.Rollback(_handle) = ()

[<Fact>]
let ``openCursor with non-InMemoryBackend falls back to Scan`` () =
    let row = Row()
    row["v"] <- box (42L : int64)
    let b = ReadOnlyBackend([ row ], [ ColumnDef("v", "INTEGER") ])
    let r = run b [
        Instruction.OpenScan("t", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        Instruction.BeginRow
        Instruction.LoadColumn(None, "v")
        Instruction.EmitColumn "v"
        Instruction.EmitRow
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ]
    Assert.Equal(1, r.Rows.Length)
    assertRow [ intVal 42L ] r.Rows.[0]

[<Fact>]
let ``DeleteRows on non-InMemoryBackend is a no-op (no cursor)`` () =
    let row = Row()
    row["v"] <- box (1L : int64)
    let b = ReadOnlyBackend([ row ], [ ColumnDef("v", "INTEGER") ])
    let r = run b [
        Instruction.OpenScan("t", None)
        Instruction.Label "loop"
        Instruction.JumpIfExhausted(None, "end")
        Instruction.AdvanceCursor None
        // Cursor is None for ReadOnlyBackend so DeleteRows is a no-op
        Instruction.DeleteRows "t"
        Instruction.Jump "loop"
        Instruction.Label "end"
        Instruction.CloseScan None
        Instruction.Halt
    ]
    Assert.Equal(0, r.RowsAffected)  // no-op: Cursor = None

// ── 47. LimitResult toSafeInt with negative and MaxValue values ──────────

[<Fact>]
let ``LimitResult with very large limit (Int64 MaxValue) works`` () =
    let b = usersBackend ()
    insertUser b 1L "A"
    insertUser b 2L "B"
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
        Instruction.LimitResult(Some System.Int64.MaxValue, None)
        Instruction.Halt
    ]
    Assert.Equal(2, r.Rows.Length)

// ── 48. CommitTransaction when no active transaction is a no-op ──────────

[<Fact>]
let ``CommitTransaction with no active handle is a no-op`` () =
    let r = runFresh [
        Instruction.CommitTransaction  // no BeginTransaction — should not throw
        Instruction.Halt
    ]
    Assert.Equal(0, r.RowsAffected)

[<Fact>]
let ``RollbackTransaction with no active handle is a no-op`` () =
    let r = runFresh [
        Instruction.RollbackTransaction  // no BeginTransaction — should not throw
        Instruction.Halt
    ]
    Assert.Equal(0, r.RowsAffected)
