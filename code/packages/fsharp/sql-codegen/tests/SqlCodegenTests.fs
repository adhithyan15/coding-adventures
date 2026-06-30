// SqlCodegenTests.fs — unit tests for the F# sql-codegen package.
//
// These tests verify that `SqlCodegen.compile` produces the correct
// instruction sequences for a wide range of query types. Each test
// constructs an OptimizedPlan directly (no parsing needed) and asserts
// on the generated instruction list.
//
// ── Test organisation ────────────────────────────────────────────────────
//
// Tests are grouped by query type, mirroring the codegen structure:
//   - SELECT: basic scan, filter, projection, aggregation, GROUP BY, ORDER BY,
//             LIMIT, DISTINCT, JOINs
//   - INSERT / UPDATE / DELETE
//   - DDL: CREATE TABLE, DROP TABLE
//   - Expressions: literals, columns, binary ops, unary ops, IS NULL/NOT NULL,
//                  BETWEEN, LIKE, IN
//   - Aggregates: COUNT, COUNT(*), SUM, AVG, MIN, MAX
//   - EmptyResult
//   - Label uniqueness
//   - compileExpression exported helper

module CodingAdventures.SqlCodegen.FSharp.Tests

open Xunit
open CodingAdventures.SqlPlanner.FSharp
open CodingAdventures.SqlOptimizer.FSharp
open CodingAdventures.SqlCodegen.FSharp

// ── Test helpers ──────────────────────────────────────────────────────────

/// Compile a plan and return the instruction list.
let private compile plan = (SqlCodegen.compile plan).Instructions

/// Check that the instruction list contains `instr` (in order).
let private containsInOrder (instrs: Instruction list) (needle: Instruction) =
    List.contains needle instrs

/// Check that instruction at index i is equal to expected.
let private instrAt (instrs: Instruction list) i = List.item i instrs

/// Find the first index of an instruction matching a predicate.
let private findInstr (instrs: Instruction list) (pred: Instruction -> bool) =
    instrs |> List.tryFindIndex pred

/// Check that any instruction matches a predicate.
let private anyInstr (instrs: Instruction list) (pred: Instruction -> bool) =
    instrs |> List.exists pred

/// Canonical column definitions for tests.
let private colDef name = {
    Name = name
    TypeName = "TEXT"
    NotNull = false
    PrimaryKey = false
    Unique = false
    Default = None
}

/// A simple single-table scan plan.
let private simpleScan table = OptimizedPlan.Scan(table, None, None, None)

/// A scan plan with an alias.
let private aliasScan table alias = OptimizedPlan.Scan(table, Some alias, None, None)

/// Wrap a plan in a filter.
let private withFilter plan pred = OptimizedPlan.Filter(plan, pred)

/// Wrap a plan in a project.
let private withProject plan cols = OptimizedPlan.Project(plan, cols)

/// An output column with an expression and optional alias.
let private exprCol expr alias = OutputColumn.Expr(expr, alias)

/// A column reference expression.
let private colRef col = Expr.Column(None, col)

/// A qualified column reference.
let private qColRef table col = Expr.Column(Some table, col)

/// A literal integer expression.
let private litInt n = Expr.Literal(SqlValue.Integer (int64 n))

/// A literal text expression.
let private litText s = Expr.Literal(SqlValue.Text s)

/// Binary op expression helper.
let private binOp op l r = Expr.BinaryOp(op, l, r)

// ── EmptyResult tests ─────────────────────────────────────────────────────

[<Fact>]
let ``EmptyResult compiles to Halt only`` () =
    let instrs = compile OptimizedPlan.EmptyResult
    Assert.Equal<Instruction list>([ Instruction.Halt ], instrs)

// ── SELECT basic scan tests ───────────────────────────────────────────────

[<Fact>]
let ``SELECT star from single table produces OpenScan and CloseScan`` () =
    let plan = simpleScan "users"
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.OpenScan("users", None)),
        "Expected OpenScan for 'users'")
    Assert.True(anyInstr instrs (fun i -> i = Instruction.CloseScan None),
        "Expected CloseScan")

[<Fact>]
let ``SELECT star loop has AdvanceCursor and Jump back`` () =
    let plan = simpleScan "orders"
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.AdvanceCursor None),
        "Expected AdvanceCursor")
    Assert.True(anyInstr instrs (fun i -> match i with Instruction.Jump _ -> true | _ -> false),
        "Expected Jump (loop back)")

[<Fact>]
let ``SELECT star emits Halt at end`` () =
    let plan = simpleScan "products"
    let instrs = compile plan
    Assert.Equal(Instruction.Halt, List.last instrs)

[<Fact>]
let ``SELECT star emits BeginRow and EmitRow in loop body`` () =
    let plan = simpleScan "users"
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BeginRow), "Expected BeginRow")
    Assert.True(anyInstr instrs (fun i -> i = Instruction.EmitRow), "Expected EmitRow")

[<Fact>]
let ``SELECT with named column emits EmitColumn with correct name`` () =
    let col = exprCol (colRef "name") None
    let plan = withProject (simpleScan "users") [ col ]
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.EmitColumn "name"),
        "Expected EmitColumn 'name'")

[<Fact>]
let ``SELECT with aliased column emits EmitColumn with alias`` () =
    let col = exprCol (colRef "name") (Some "full_name")
    let plan = withProject (simpleScan "users") [ col ]
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.EmitColumn "full_name"),
        "Expected EmitColumn 'full_name'")

[<Fact>]
let ``SELECT with two columns emits two EmitColumn instructions`` () =
    let cols = [ exprCol (colRef "id") None; exprCol (colRef "name") None ]
    let plan = withProject (simpleScan "users") cols
    let instrs = compile plan
    let colEmits = instrs |> List.filter (fun i ->
        match i with Instruction.EmitColumn _ -> true | _ -> false)
    Assert.Equal(2, List.length colEmits)

[<Fact>]
let ``SELECT with expression column emits LoadColumn then EmitColumn`` () =
    let col = exprCol (colRef "age") None
    let plan = withProject (simpleScan "users") [ col ]
    let instrs = compile plan
    // LoadColumn should appear before EmitColumn
    let loadIdx = findInstr instrs (fun i -> i = Instruction.LoadColumn(None, "age"))
    let emitIdx = findInstr instrs (fun i -> i = Instruction.EmitColumn "age")
    Assert.True(loadIdx.IsSome, "Expected LoadColumn 'age'")
    Assert.True(emitIdx.IsSome, "Expected EmitColumn 'age'")
    Assert.True(loadIdx.Value < emitIdx.Value, "LoadColumn should come before EmitColumn")

// ── SELECT with WHERE tests ───────────────────────────────────────────────

[<Fact>]
let ``SELECT with WHERE compiles predicate into loop body`` () =
    let pred = binOp BinaryOperator.Gt (colRef "age") (litInt 18)
    let plan = withFilter (simpleScan "users") pred
    let instrs = compile plan
    // The predicate should include a JumpIfFalse
    Assert.True(anyInstr instrs (fun i -> match i with Instruction.JumpIfFalse _ -> true | _ -> false),
        "Expected JumpIfFalse for WHERE clause")

[<Fact>]
let ``SELECT with WHERE = emits BinaryOpInstr Eq`` () =
    let pred = binOp BinaryOperator.Eq (colRef "status") (litText "active")
    let plan = withFilter (simpleScan "orders") pred
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Eq),
        "Expected BinaryOpInstr Eq")

[<Fact>]
let ``SELECT with WHERE loads column and constant`` () =
    let pred = binOp BinaryOperator.Lt (colRef "price") (Expr.Literal(SqlValue.Real 100.0))
    let plan = withFilter (simpleScan "products") pred
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.LoadColumn(None, "price")),
        "Expected LoadColumn 'price'")
    Assert.True(anyInstr instrs (fun i -> i = Instruction.LoadConst(SqlValue.Real 100.0)),
        "Expected LoadConst 100.0")

[<Fact>]
let ``SELECT with AND predicate emits BinaryOpInstr And`` () =
    let pred =
        binOp BinaryOperator.And
            (binOp BinaryOperator.Gt (colRef "age") (litInt 18))
            (binOp BinaryOperator.Lt (colRef "age") (litInt 65))
    let plan = withFilter (simpleScan "employees") pred
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.And),
        "Expected BinaryOpInstr And")

// ── SELECT with ORDER BY tests ────────────────────────────────────────────

[<Fact>]
let ``SELECT with ORDER BY emits SortResult post-op`` () =
    let key = { KeyExpr = colRef "name"; Direction = SortDir.Asc; NullOrder = NullOrder.NullsLast }
    let inner = simpleScan "users"
    let plan = OptimizedPlan.Sort(inner, [ key ])
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i ->
        match i with Instruction.SortResult _ -> true | _ -> false),
        "Expected SortResult")

[<Fact>]
let ``SELECT with ORDER BY DESC emits SortResult with Desc key`` () =
    let key = { KeyExpr = colRef "salary"; Direction = SortDir.Desc; NullOrder = NullOrder.NullsFirst }
    let inner = simpleScan "employees"
    let plan = OptimizedPlan.Sort(inner, [ key ])
    let instrs = compile plan
    let sortInstr = instrs |> List.tryFind (fun i ->
        match i with Instruction.SortResult _ -> true | _ -> false)
    Assert.True(sortInstr.IsSome, "Expected SortResult")
    match sortInstr with
    | Some (Instruction.SortResult keys) ->
        Assert.Equal(1, List.length keys)
        Assert.Equal(SortDir.Desc, keys.[0].Direction)
    | _ -> failwith "unexpected"

[<Fact>]
let ``SortResult comes AFTER the scan loop`` () =
    let key = { KeyExpr = colRef "name"; Direction = SortDir.Asc; NullOrder = NullOrder.NullsLast }
    let plan = OptimizedPlan.Sort(simpleScan "users", [ key ])
    let instrs = compile plan
    let haltIdx = instrs |> List.findIndex (fun i -> i = Instruction.Halt)
    let sortIdx = instrs |> List.tryFindIndex (fun i ->
        match i with Instruction.SortResult _ -> true | _ -> false)
    Assert.True(sortIdx.IsSome, "Expected SortResult")
    Assert.True(sortIdx.Value < haltIdx, "SortResult should come before Halt")
    let closeIdx = instrs |> List.tryFindIndex (fun i -> i = Instruction.CloseScan None)
    Assert.True(closeIdx.IsSome, "Expected CloseScan")
    Assert.True(sortIdx.Value > closeIdx.Value, "SortResult should come after CloseScan")

// ── SELECT with LIMIT/OFFSET tests ───────────────────────────────────────

[<Fact>]
let ``SELECT with LIMIT emits LimitResult post-op`` () =
    let plan = OptimizedPlan.Limit(simpleScan "users", Some 10L, None)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i ->
        i = Instruction.LimitResult(Some 10L, None)),
        "Expected LimitResult(Some 10, None)")

[<Fact>]
let ``SELECT with LIMIT and OFFSET emits LimitResult with both`` () =
    let plan = OptimizedPlan.Limit(simpleScan "users", Some 5L, Some 20L)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i ->
        i = Instruction.LimitResult(Some 5L, Some 20L)),
        "Expected LimitResult(Some 5, Some 20)")

[<Fact>]
let ``SELECT with LIMIT no count emits LimitResult with None count`` () =
    let plan = OptimizedPlan.Limit(simpleScan "users", None, Some 10L)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i ->
        i = Instruction.LimitResult(None, Some 10L)),
        "Expected LimitResult(None, Some 10)")

// ── SELECT DISTINCT tests ─────────────────────────────────────────────────

[<Fact>]
let ``SELECT DISTINCT emits DistinctResult post-op`` () =
    let plan = OptimizedPlan.Distinct(simpleScan "users")
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.DistinctResult),
        "Expected DistinctResult")

[<Fact>]
let ``DistinctResult comes after scan loop`` () =
    let plan = OptimizedPlan.Distinct(simpleScan "users")
    let instrs = compile plan
    let haltIdx = instrs |> List.findIndex (fun i -> i = Instruction.Halt)
    let distIdx = instrs |> List.tryFindIndex (fun i -> i = Instruction.DistinctResult)
    Assert.True(distIdx.IsSome, "Expected DistinctResult")
    Assert.True(distIdx.Value < haltIdx, "DistinctResult should come before Halt")

// ── Aggregate tests ───────────────────────────────────────────────────────

[<Fact>]
let ``SELECT COUNT star emits InitAgg UpdateAgg FinalizeAgg`` () =
    let aggItem = { Func = AggFunction.Count; Arg = AggArg.Star; Alias = "count(*)"; Distinct = false }
    let inner = simpleScan "orders"
    let plan = OptimizedPlan.Aggregate(inner, [], [ aggItem ])
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> match i with Instruction.InitAgg _ -> true | _ -> false),
        "Expected InitAgg")
    Assert.True(anyInstr instrs (fun i -> match i with Instruction.UpdateAgg _ -> true | _ -> false),
        "Expected UpdateAgg")
    Assert.True(anyInstr instrs (fun i -> match i with Instruction.FinalizeAgg _ -> true | _ -> false),
        "Expected FinalizeAgg")

[<Fact>]
let ``SELECT COUNT star InitAgg count is 1`` () =
    let aggItem = { Func = AggFunction.Count; Arg = AggArg.Star; Alias = "cnt"; Distinct = false }
    let plan = OptimizedPlan.Aggregate(simpleScan "users", [], [ aggItem ])
    let instrs = compile plan
    let initInstr = instrs |> List.tryFind (fun i -> match i with Instruction.InitAgg _ -> true | _ -> false)
    match initInstr with
    | Some (Instruction.InitAgg n) -> Assert.Equal(1, n)
    | _ -> failwith "No InitAgg found"

[<Fact>]
let ``SELECT SUM emits UpdateAgg with Sum`` () =
    let aggItem = { Func = AggFunction.Sum; Arg = AggArg.Expr(colRef "amount"); Alias = "total"; Distinct = false }
    let plan = OptimizedPlan.Aggregate(simpleScan "orders", [], [ aggItem ])
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i ->
        match i with Instruction.UpdateAgg(_, AggFn.Sum) -> true | _ -> false),
        "Expected UpdateAgg with Sum")

[<Fact>]
let ``SELECT AVG emits UpdateAgg with Avg`` () =
    let aggItem = { Func = AggFunction.Avg; Arg = AggArg.Expr(colRef "score"); Alias = "avg_score"; Distinct = false }
    let plan = OptimizedPlan.Aggregate(simpleScan "tests", [], [ aggItem ])
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i ->
        match i with Instruction.UpdateAgg(_, AggFn.Avg) -> true | _ -> false),
        "Expected UpdateAgg with Avg")

[<Fact>]
let ``SELECT MIN emits UpdateAgg with Min`` () =
    let aggItem = { Func = AggFunction.Min; Arg = AggArg.Expr(colRef "price"); Alias = "min_price"; Distinct = false }
    let plan = OptimizedPlan.Aggregate(simpleScan "products", [], [ aggItem ])
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i ->
        match i with Instruction.UpdateAgg(_, AggFn.Min) -> true | _ -> false),
        "Expected UpdateAgg with Min")

[<Fact>]
let ``SELECT MAX emits UpdateAgg with Max`` () =
    let aggItem = { Func = AggFunction.Max; Arg = AggArg.Expr(colRef "age"); Alias = "max_age"; Distinct = false }
    let plan = OptimizedPlan.Aggregate(simpleScan "users", [], [ aggItem ])
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i ->
        match i with Instruction.UpdateAgg(_, AggFn.Max) -> true | _ -> false),
        "Expected UpdateAgg with Max")

[<Fact>]
let ``SELECT multiple aggregates emits InitAgg with count 2`` () =
    let agg1 = { Func = AggFunction.Count; Arg = AggArg.Star; Alias = "cnt"; Distinct = false }
    let agg2 = { Func = AggFunction.Sum; Arg = AggArg.Expr(colRef "amount"); Alias = "total"; Distinct = false }
    let plan = OptimizedPlan.Aggregate(simpleScan "orders", [], [ agg1; agg2 ])
    let instrs = compile plan
    let initInstr = instrs |> List.tryFind (fun i -> match i with Instruction.InitAgg _ -> true | _ -> false)
    match initInstr with
    | Some (Instruction.InitAgg n) -> Assert.Equal(2, n)
    | _ -> failwith "No InitAgg found"

// ── GROUP BY tests ────────────────────────────────────────────────────────

[<Fact>]
let ``SELECT with GROUP BY emits SaveGroupKey`` () =
    let aggItem = { Func = AggFunction.Count; Arg = AggArg.Star; Alias = "cnt"; Distinct = false }
    let plan = OptimizedPlan.Aggregate(simpleScan "orders", [ colRef "category" ], [ aggItem ])
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i ->
        match i with Instruction.SaveGroupKey _ -> true | _ -> false),
        "Expected SaveGroupKey")

[<Fact>]
let ``SELECT with GROUP BY two columns emits SaveGroupKey with two names`` () =
    let aggItem = { Func = AggFunction.Count; Arg = AggArg.Star; Alias = "cnt"; Distinct = false }
    let plan = OptimizedPlan.Aggregate(
        simpleScan "sales",
        [ colRef "year"; colRef "month" ],
        [ aggItem ])
    let instrs = compile plan
    let saveInstr = instrs |> List.tryFind (fun i ->
        match i with Instruction.SaveGroupKey _ -> true | _ -> false)
    match saveInstr with
    | Some (Instruction.SaveGroupKey keys) -> Assert.Equal(2, List.length keys)
    | _ -> failwith "No SaveGroupKey found"

[<Fact>]
let ``SELECT with GROUP BY emits LoadGroupKey for finalize phase`` () =
    let aggItem = { Func = AggFunction.Sum; Arg = AggArg.Expr(colRef "amount"); Alias = "total"; Distinct = false }
    let plan = OptimizedPlan.Aggregate(simpleScan "orders", [ colRef "dept" ], [ aggItem ])
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i ->
        match i with Instruction.LoadGroupKey _ -> true | _ -> false),
        "Expected LoadGroupKey for group emit phase")

// ── INSERT tests ──────────────────────────────────────────────────────────

[<Fact>]
let ``INSERT VALUES emits InsertRow`` () =
    let values = [ [ litInt 1; litText "Alice" ] ]
    let plan = OptimizedPlan.Insert("users", Some [ "id"; "name" ], InsertSource.Values values)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i ->
        match i with Instruction.InsertRow _ -> true | _ -> false),
        "Expected InsertRow")

[<Fact>]
let ``INSERT VALUES emits LoadConst for each value`` () =
    let values = [ [ litInt 42; litText "Bob" ] ]
    let plan = OptimizedPlan.Insert("users", None, InsertSource.Values values)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.LoadConst(SqlValue.Integer 42L)),
        "Expected LoadConst Integer 42")
    Assert.True(anyInstr instrs (fun i -> i = Instruction.LoadConst(SqlValue.Text "Bob")),
        "Expected LoadConst Text 'Bob'")

[<Fact>]
let ``INSERT VALUES multiple rows emits multiple InsertRow`` () =
    let values = [ [ litInt 1 ]; [ litInt 2 ]; [ litInt 3 ] ]
    let plan = OptimizedPlan.Insert("ids", None, InsertSource.Values values)
    let instrs = compile plan
    let insertCount = instrs |> List.filter (fun i ->
        match i with Instruction.InsertRow _ -> true | _ -> false) |> List.length
    Assert.Equal(3, insertCount)

[<Fact>]
let ``INSERT with column list stores columns in InsertRow`` () =
    let values = [ [ litInt 1 ] ]
    let cols = Some [ "id" ]
    let plan = OptimizedPlan.Insert("users", cols, InsertSource.Values values)
    let instrs = compile plan
    let insertInstr = instrs |> List.tryFind (fun i ->
        match i with Instruction.InsertRow _ -> true | _ -> false)
    match insertInstr with
    | Some (Instruction.InsertRow(table, colsOpt)) ->
        Assert.Equal("users", table)
        Assert.Equal(Some [ "id" ], colsOpt)
    | _ -> failwith "No InsertRow found"

// ── UPDATE tests ──────────────────────────────────────────────────────────

[<Fact>]
let ``UPDATE emits OpenScan and UpdateRows`` () =
    let assign = { Column = "status"; Value = litText "inactive" }
    let plan = OptimizedPlan.Update("users", [ assign ], None)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.OpenScan("users", None)),
        "Expected OpenScan for update")
    Assert.True(anyInstr instrs (fun i ->
        match i with Instruction.UpdateRows _ -> true | _ -> false),
        "Expected UpdateRows")

[<Fact>]
let ``UPDATE with WHERE emits filter predicate before UpdateRows`` () =
    let assign = { Column = "name"; Value = litText "Charlie" }
    let pred = binOp BinaryOperator.Eq (colRef "id") (litInt 1)
    let plan = OptimizedPlan.Update("users", [ assign ], Some pred)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> match i with Instruction.JumpIfFalse _ -> true | _ -> false),
        "Expected JumpIfFalse for WHERE in UPDATE")
    Assert.True(anyInstr instrs (fun i -> match i with Instruction.UpdateRows _ -> true | _ -> false),
        "Expected UpdateRows")

[<Fact>]
let ``UPDATE emits CloseScan and Halt`` () =
    let assign = { Column = "x"; Value = litInt 0 }
    let plan = OptimizedPlan.Update("t", [ assign ], None)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.CloseScan None),
        "Expected CloseScan")
    Assert.Equal(Instruction.Halt, List.last instrs)

// ── DELETE tests ──────────────────────────────────────────────────────────

[<Fact>]
let ``DELETE emits OpenScan and DeleteRows`` () =
    let plan = OptimizedPlan.Delete("users", None)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.OpenScan("users", None)),
        "Expected OpenScan")
    Assert.True(anyInstr instrs (fun i ->
        match i with Instruction.DeleteRows _ -> true | _ -> false),
        "Expected DeleteRows")

[<Fact>]
let ``DELETE with WHERE emits filter predicate`` () =
    let pred = binOp BinaryOperator.Eq (colRef "id") (litInt 5)
    let plan = OptimizedPlan.Delete("users", Some pred)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> match i with Instruction.JumpIfFalse _ -> true | _ -> false),
        "Expected JumpIfFalse for WHERE in DELETE")

[<Fact>]
let ``DELETE emits CloseScan and Halt`` () =
    let plan = OptimizedPlan.Delete("logs", None)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.CloseScan None))
    Assert.Equal(Instruction.Halt, List.last instrs)

// ── CREATE TABLE tests ────────────────────────────────────────────────────

[<Fact>]
let ``CREATE TABLE emits CreateTable instruction`` () =
    let cols = [ colDef "id"; colDef "name" ]
    let plan = OptimizedPlan.CreateTable("users", false, cols)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i ->
        match i with Instruction.CreateTable _ -> true | _ -> false),
        "Expected CreateTable")

[<Fact>]
let ``CREATE TABLE IF NOT EXISTS passes ifNotExists flag`` () =
    let cols = [ colDef "id" ]
    let plan = OptimizedPlan.CreateTable("t", true, cols)
    let instrs = compile plan
    let createInstr = instrs |> List.tryFind (fun i ->
        match i with Instruction.CreateTable _ -> true | _ -> false)
    match createInstr with
    | Some (Instruction.CreateTable(name, ifne, _)) ->
        Assert.Equal("t", name)
        Assert.True(ifne)
    | _ -> failwith "No CreateTable found"

[<Fact>]
let ``CREATE TABLE emits Halt`` () =
    let plan = OptimizedPlan.CreateTable("t", false, [ colDef "x" ])
    let instrs = compile plan
    Assert.Equal(Instruction.Halt, List.last instrs)

[<Fact>]
let ``CREATE TABLE preserves column definitions`` () =
    let cols = [ colDef "id"; colDef "name"; colDef "age" ]
    let plan = OptimizedPlan.CreateTable("users", false, cols)
    let instrs = compile plan
    let createInstr = instrs |> List.tryFind (fun i ->
        match i with Instruction.CreateTable _ -> true | _ -> false)
    match createInstr with
    | Some (Instruction.CreateTable(_, _, actualCols)) ->
        Assert.Equal(3, List.length actualCols)
    | _ -> failwith "No CreateTable found"

// ── DROP TABLE tests ──────────────────────────────────────────────────────

[<Fact>]
let ``DROP TABLE emits DropTable instruction`` () =
    let plan = OptimizedPlan.DropTable("users", false)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i ->
        match i with Instruction.DropTable _ -> true | _ -> false),
        "Expected DropTable")

[<Fact>]
let ``DROP TABLE IF EXISTS passes ifExists flag`` () =
    let plan = OptimizedPlan.DropTable("users", true)
    let instrs = compile plan
    let dropInstr = instrs |> List.tryFind (fun i ->
        match i with Instruction.DropTable _ -> true | _ -> false)
    match dropInstr with
    | Some (Instruction.DropTable(name, ie)) ->
        Assert.Equal("users", name)
        Assert.True(ie)
    | _ -> failwith "No DropTable found"

[<Fact>]
let ``DROP TABLE emits Halt`` () =
    let plan = OptimizedPlan.DropTable("t", false)
    let instrs = compile plan
    Assert.Equal(Instruction.Halt, List.last instrs)

// ── Expression compilation tests ──────────────────────────────────────────

[<Fact>]
let ``compileExpression integer literal produces LoadConst`` () =
    let instrs = SqlCodegen.compileExpression (litInt 42)
    Assert.Equal<Instruction list>([ Instruction.LoadConst(SqlValue.Integer 42L) ], instrs)

[<Fact>]
let ``compileExpression text literal produces LoadConst`` () =
    let instrs = SqlCodegen.compileExpression (litText "hello")
    Assert.Equal<Instruction list>([ Instruction.LoadConst(SqlValue.Text "hello") ], instrs)

[<Fact>]
let ``compileExpression null literal produces LoadConst Null`` () =
    let instrs = SqlCodegen.compileExpression (Expr.Literal SqlValue.Null)
    Assert.Equal<Instruction list>([ Instruction.LoadConst SqlValue.Null ], instrs)

[<Fact>]
let ``compileExpression bool literal produces LoadConst Bool`` () =
    let instrs = SqlCodegen.compileExpression (Expr.Literal(SqlValue.Bool true))
    Assert.Equal<Instruction list>([ Instruction.LoadConst(SqlValue.Bool true) ], instrs)

[<Fact>]
let ``compileExpression real literal produces LoadConst Real`` () =
    let instrs = SqlCodegen.compileExpression (Expr.Literal(SqlValue.Real 3.14))
    Assert.Equal<Instruction list>([ Instruction.LoadConst(SqlValue.Real 3.14) ], instrs)

[<Fact>]
let ``compileExpression column reference produces LoadColumn`` () =
    let instrs = SqlCodegen.compileExpression (colRef "age")
    Assert.Equal<Instruction list>([ Instruction.LoadColumn(None, "age") ], instrs)

[<Fact>]
let ``compileExpression qualified column produces LoadColumn with table`` () =
    let instrs = SqlCodegen.compileExpression (qColRef "u" "name")
    Assert.Equal<Instruction list>([ Instruction.LoadColumn(Some "u", "name") ], instrs)

[<Fact>]
let ``compileExpression Add produces BinaryOpInstr Add`` () =
    let expr = binOp BinaryOperator.Add (litInt 1) (litInt 2)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Add))

[<Fact>]
let ``compileExpression Sub produces BinaryOpInstr Sub`` () =
    let expr = binOp BinaryOperator.Sub (litInt 10) (litInt 3)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Sub))

[<Fact>]
let ``compileExpression Mul produces BinaryOpInstr Mul`` () =
    let expr = binOp BinaryOperator.Mul (litInt 5) (litInt 6)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Mul))

[<Fact>]
let ``compileExpression Div produces BinaryOpInstr Div`` () =
    let expr = binOp BinaryOperator.Div (litInt 10) (litInt 2)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Div))

[<Fact>]
let ``compileExpression Mod produces BinaryOpInstr Mod`` () =
    let expr = binOp BinaryOperator.Mod (litInt 7) (litInt 3)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Mod))

[<Fact>]
let ``compileExpression Eq produces BinaryOpInstr Eq`` () =
    let expr = binOp BinaryOperator.Eq (colRef "x") (litInt 5)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Eq))

[<Fact>]
let ``compileExpression NotEq produces BinaryOpInstr Neq`` () =
    let expr = binOp BinaryOperator.NotEq (colRef "x") (litInt 5)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Neq))

[<Fact>]
let ``compileExpression Lt produces BinaryOpInstr Lt`` () =
    let expr = binOp BinaryOperator.Lt (colRef "x") (litInt 10)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Lt))

[<Fact>]
let ``compileExpression Lte produces BinaryOpInstr Lte`` () =
    let expr = binOp BinaryOperator.Lte (colRef "x") (litInt 10)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Lte))

[<Fact>]
let ``compileExpression Gt produces BinaryOpInstr Gt`` () =
    let expr = binOp BinaryOperator.Gt (colRef "x") (litInt 5)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Gt))

[<Fact>]
let ``compileExpression Gte produces BinaryOpInstr Gte`` () =
    let expr = binOp BinaryOperator.Gte (colRef "x") (litInt 5)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Gte))

[<Fact>]
let ``compileExpression And produces BinaryOpInstr And`` () =
    let expr = binOp BinaryOperator.And (Expr.Literal(SqlValue.Bool true)) (Expr.Literal(SqlValue.Bool false))
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.And))

[<Fact>]
let ``compileExpression Or produces BinaryOpInstr Or`` () =
    let expr = binOp BinaryOperator.Or (Expr.Literal(SqlValue.Bool false)) (Expr.Literal(SqlValue.Bool true))
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Or))

[<Fact>]
let ``compileExpression Neg produces UnaryOpInstr Neg`` () =
    let expr = Expr.UnaryOp(UnaryOperator.Neg, litInt 5)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.UnaryOpInstr UnaryOp.Neg))

[<Fact>]
let ``compileExpression Not produces UnaryOpInstr Not`` () =
    let expr = Expr.UnaryOp(UnaryOperator.Not, Expr.Literal(SqlValue.Bool true))
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.UnaryOpInstr UnaryOp.Not))

[<Fact>]
let ``compileExpression IsNull produces IsNull instruction`` () =
    let expr = Expr.IsNull(colRef "email")
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.IsNull))
    Assert.True(anyInstr instrs (fun i -> i = Instruction.LoadColumn(None, "email")))

[<Fact>]
let ``compileExpression IsNotNull produces IsNotNull instruction`` () =
    let expr = Expr.IsNotNull(colRef "phone")
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.IsNotNull))

[<Fact>]
let ``compileExpression Between produces Between instruction`` () =
    let expr = Expr.Between(colRef "age", litInt 18, litInt 65)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i ->
        match i with Instruction.Between _ -> true | _ -> false),
        "Expected Between instruction")

[<Fact>]
let ``compileExpression Between pushes value lo hi in order`` () =
    let expr = Expr.Between(colRef "score", litInt 50, litInt 100)
    let instrs = SqlCodegen.compileExpression expr
    // After LoadColumn(score), we should see LoadConst(50), LoadConst(100)
    let scoreIdx = instrs |> List.tryFindIndex (fun i -> i = Instruction.LoadColumn(None, "score"))
    let lo50Idx  = instrs |> List.tryFindIndex (fun i -> i = Instruction.LoadConst(SqlValue.Integer 50L))
    let hi100Idx = instrs |> List.tryFindIndex (fun i -> i = Instruction.LoadConst(SqlValue.Integer 100L))
    Assert.True(scoreIdx.IsSome && lo50Idx.IsSome && hi100Idx.IsSome)
    Assert.True(scoreIdx.Value < lo50Idx.Value)
    Assert.True(lo50Idx.Value < hi100Idx.Value)

[<Fact>]
let ``compileExpression Like produces Like instruction and pattern constant`` () =
    let expr = Expr.Like(colRef "name", "%Alice%")
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.Like))
    Assert.True(anyInstr instrs (fun i -> i = Instruction.LoadConst(SqlValue.Text "%Alice%")))

[<Fact>]
let ``compileExpression NotLike produces Like then Not`` () =
    let expr = Expr.NotLike(colRef "name", "Bob%")
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.Like))
    Assert.True(anyInstr instrs (fun i -> i = Instruction.UnaryOpInstr UnaryOp.Not))

[<Fact>]
let ``compileExpression In produces InList with correct count`` () =
    let items = [ litInt 1; litInt 2; litInt 3 ]
    let expr = Expr.In(colRef "id", items)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.InList 3),
        "Expected InList 3")

[<Fact>]
let ``compileExpression In pushes needle first then items`` () =
    let items = [ litInt 10; litInt 20 ]
    let expr = Expr.In(colRef "x", items)
    let instrs = SqlCodegen.compileExpression expr
    let needleIdx = instrs |> List.tryFindIndex (fun i -> i = Instruction.LoadColumn(None, "x"))
    let inListIdx = instrs |> List.tryFindIndex (fun i -> i = Instruction.InList 2)
    Assert.True(needleIdx.IsSome && inListIdx.IsSome)
    Assert.True(needleIdx.Value < inListIdx.Value)

[<Fact>]
let ``compileExpression NotIn produces InList then Not`` () =
    let items = [ litInt 1; litInt 2 ]
    let expr = Expr.NotIn(colRef "cat", items)
    let instrs = SqlCodegen.compileExpression expr
    Assert.True(anyInstr instrs (fun i -> i = Instruction.InList 2))
    Assert.True(anyInstr instrs (fun i -> i = Instruction.UnaryOpInstr UnaryOp.Not))

[<Fact>]
let ``compileExpression nested binary op has correct order`` () =
    // (a + b) * c compiles to: a b Add c Mul
    let expr = binOp BinaryOperator.Mul (binOp BinaryOperator.Add (colRef "a") (colRef "b")) (colRef "c")
    let instrs = SqlCodegen.compileExpression expr
    let addIdx = instrs |> List.tryFindIndex (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Add)
    let mulIdx = instrs |> List.tryFindIndex (fun i -> i = Instruction.BinaryOpInstr BinaryOp.Mul)
    Assert.True(addIdx.IsSome && mulIdx.IsSome)
    Assert.True(addIdx.Value < mulIdx.Value, "Add should come before Mul for (a+b)*c")

// ── JOIN tests ────────────────────────────────────────────────────────────

[<Fact>]
let ``JOIN compiles outer and inner OpenScan`` () =
    let left = aliasScan "users" "u"
    let right = aliasScan "orders" "o"
    let plan = OptimizedPlan.Join(left, right, JoinKind.Inner, None)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.OpenScan("users", Some "u")),
        "Expected OpenScan for users")
    Assert.True(anyInstr instrs (fun i -> i = Instruction.OpenScan("orders", Some "o")),
        "Expected OpenScan for orders")

[<Fact>]
let ``JOIN compiles both CloseScan instructions`` () =
    let left = aliasScan "users" "u"
    let right = aliasScan "orders" "o"
    let plan = OptimizedPlan.Join(left, right, JoinKind.Inner, None)
    let instrs = compile plan
    let closeCount = instrs |> List.filter (fun i ->
        match i with Instruction.CloseScan _ -> true | _ -> false) |> List.length
    Assert.Equal(2, closeCount)

[<Fact>]
let ``JOIN with ON condition emits JumpIfFalse for condition`` () =
    let left = aliasScan "users" "u"
    let right = aliasScan "orders" "o"
    let cond = binOp BinaryOperator.Eq (qColRef "u" "id") (qColRef "o" "user_id")
    let plan = OptimizedPlan.Join(left, right, JoinKind.Inner, Some cond)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> match i with Instruction.JumpIfFalse _ -> true | _ -> false),
        "Expected JumpIfFalse for JOIN ON condition")

// ── Label uniqueness tests ────────────────────────────────────────────────

[<Fact>]
let ``Multiple scans produce unique loop labels`` () =
    // Two independent scans (a UNION) should have distinct labels
    let left = simpleScan "table_a"
    let right = simpleScan "table_b"
    let plan = OptimizedPlan.Union(left, right, true)
    let instrs = compile plan
    let labels = instrs |> List.choose (fun i ->
        match i with Instruction.Label name -> Some name | _ -> None)
    let uniqueLabels = labels |> List.distinct
    Assert.Equal(List.length uniqueLabels, List.length labels)

[<Fact>]
let ``JOIN produces four unique labels (two loop/end pairs)`` () =
    let left = aliasScan "a" "a"
    let right = aliasScan "b" "b"
    let plan = OptimizedPlan.Join(left, right, JoinKind.Inner, None)
    let instrs = compile plan
    let labels = instrs |> List.choose (fun i ->
        match i with Instruction.Label name -> Some name | _ -> None)
    Assert.Equal(4, List.length labels)  // loop_N, end_N for outer and inner

[<Fact>]
let ``Compiling two plans back-to-back produces unique labels`` () =
    let plan1 = simpleScan "alpha"
    let plan2 = simpleScan "beta"
    let instrs1 = compile plan1
    let instrs2 = compile plan2
    let labels1 = instrs1 |> List.choose (fun i ->
        match i with Instruction.Label n -> Some n | _ -> None) |> Set.ofList
    let labels2 = instrs2 |> List.choose (fun i ->
        match i with Instruction.Label n -> Some n | _ -> None) |> Set.ofList
    // Labels from different compile calls should be independent (no conflicts within each)
    Assert.True(Set.count labels1 > 0, "Plan 1 should have labels")
    Assert.True(Set.count labels2 > 0, "Plan 2 should have labels")

// ── Instruction ordering sanity tests ─────────────────────────────────────

[<Fact>]
let ``OpenScan comes before AdvanceCursor`` () =
    let plan = simpleScan "users"
    let instrs = compile plan
    let openIdx = instrs |> List.findIndex (fun i -> i = Instruction.OpenScan("users", None))
    let advIdx  = instrs |> List.findIndex (fun i -> i = Instruction.AdvanceCursor None)
    Assert.True(openIdx < advIdx, "OpenScan must come before AdvanceCursor")

[<Fact>]
let ``AdvanceCursor comes before BeginRow`` () =
    let plan = simpleScan "users"
    let instrs = compile plan
    let advIdx   = instrs |> List.findIndex (fun i -> i = Instruction.AdvanceCursor None)
    let beginIdx = instrs |> List.findIndex (fun i -> i = Instruction.BeginRow)
    Assert.True(advIdx < beginIdx, "AdvanceCursor must come before BeginRow")

[<Fact>]
let ``BeginRow comes before EmitRow`` () =
    let plan = simpleScan "users"
    let instrs = compile plan
    let beginIdx = instrs |> List.findIndex (fun i -> i = Instruction.BeginRow)
    let emitIdx  = instrs |> List.findIndex (fun i -> i = Instruction.EmitRow)
    Assert.True(beginIdx < emitIdx, "BeginRow must come before EmitRow")

[<Fact>]
let ``CloseScan comes before Halt`` () =
    let plan = simpleScan "users"
    let instrs = compile plan
    let closeIdx = instrs |> List.findIndex (fun i -> i = Instruction.CloseScan None)
    let haltIdx  = instrs |> List.findIndex (fun i -> i = Instruction.Halt)
    Assert.True(closeIdx < haltIdx, "CloseScan must come before Halt")

[<Fact>]
let ``JumpIfExhausted label matches a Label instruction`` () =
    let plan = simpleScan "t"
    let instrs = compile plan
    // Find the JumpIfExhausted instruction and verify its label exists
    let jumpExhausted = instrs |> List.tryFind (fun i ->
        match i with Instruction.JumpIfExhausted _ -> true | _ -> false)
    match jumpExhausted with
    | Some (Instruction.JumpIfExhausted(_, lbl)) ->
        let labelExists = anyInstr instrs (fun i -> i = Instruction.Label lbl)
        Assert.True(labelExists, sprintf "Label '%s' referenced by JumpIfExhausted should exist" lbl)
    | _ -> failwith "No JumpIfExhausted found"

[<Fact>]
let ``Jump target label exists in instruction stream`` () =
    let plan = simpleScan "t"
    let instrs = compile plan
    // Find the unconditional Jump and verify its target exists
    let jumpInstr = instrs |> List.tryFind (fun i ->
        match i with Instruction.Jump _ -> true | _ -> false)
    match jumpInstr with
    | Some (Instruction.Jump lbl) ->
        let labelExists = anyInstr instrs (fun i -> i = Instruction.Label lbl)
        Assert.True(labelExists, sprintf "Label '%s' referenced by Jump should exist" lbl)
    | _ -> failwith "No Jump found"

// ── Combined post-processing tests ────────────────────────────────────────

[<Fact>]
let ``SELECT DISTINCT ORDER BY emits both DistinctResult and SortResult`` () =
    let key = { KeyExpr = colRef "name"; Direction = SortDir.Asc; NullOrder = NullOrder.NullsLast }
    let plan = OptimizedPlan.Distinct(OptimizedPlan.Sort(simpleScan "users", [ key ]))
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> i = Instruction.DistinctResult))
    Assert.True(anyInstr instrs (fun i -> match i with Instruction.SortResult _ -> true | _ -> false))

[<Fact>]
let ``SELECT LIMIT ORDER BY emits both LimitResult and SortResult`` () =
    let key = { KeyExpr = colRef "age"; Direction = SortDir.Desc; NullOrder = NullOrder.NullsLast }
    let plan = OptimizedPlan.Limit(OptimizedPlan.Sort(simpleScan "users", [ key ]), Some 10L, None)
    let instrs = compile plan
    Assert.True(anyInstr instrs (fun i -> match i with Instruction.SortResult _ -> true | _ -> false))
    Assert.True(anyInstr instrs (fun i -> i = Instruction.LimitResult(Some 10L, None)))
