// SqlPlannerTests.fs — conformance tests for the F# sql-planner.
//
// Covers the 13 required conformance points from sql-planner.md plus
// extensive expression-type and error-path coverage.

module CodingAdventures.SqlPlanner.FSharp.Tests

open Xunit
open CodingAdventures.SqlPlanner.FSharp

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Schema: users(id, name, age)  orders(id, user_id, amount)
let private schema =
    InMemorySchemaProvider(
        Map.ofList
            [ "users",  ["id"; "name"; "age"]
              "orders", ["id"; "user_id"; "amount"] ]) :> ISchemaProvider

let private ok (r: Result<'a, PlanError>) =
    match r with
    | Ok v    -> v
    | Error e -> failwith (sprintf "Expected Ok but got Error: %A" e)

let private err (r: Result<'a, PlanError>) =
    match r with
    | Error e -> e
    | Ok v    -> failwith (sprintf "Expected Error but got Ok: %A" v)

let private col tblOpt name =
    OutputColumn.Expr(Expr.Column(tblOpt, name), None)

let private selectFrom cols from =
    { Distinct = false
      Columns  = cols
      From     = from
      Joins    = []
      Where    = None
      GroupBy  = []
      Having   = None
      OrderBy  = []
      Limit    = None }

// ──────────────────────────────────────────────────────────────────────────────
// Conformance tests (13 required by spec)
// ──────────────────────────────────────────────────────────────────────────────

[<Fact>]
let ``C1: Simple SELECT produces Scan wrapped in Project`` () =
    let plan = ok (Planner.plan schema (Statement.Select (selectFrom [col None "id"; col None "name"] ["users", None])))
    match plan with
    | LogicalPlan.Project(LogicalPlan.Scan("users", None), cols) ->
        Assert.Equal(2, cols.Length)
    | other -> failwith (sprintf "Expected Project(Scan), got %A" other)

[<Fact>]
let ``C2: WHERE clause inserts Filter between Scan and Project`` () =
    let pred = Expr.BinaryOp(Gt, Expr.Column(None, "age"), Expr.Literal(SqlValue.Integer 18L))
    let stmt = selectFrom [col None "name"] ["users", None]
    let plan = ok (Planner.plan schema (Statement.Select { stmt with Where = Some pred }))
    match plan with
    | LogicalPlan.Project(LogicalPlan.Filter(LogicalPlan.Scan("users", None), _), _) -> ()
    | other -> failwith (sprintf "Expected Project(Filter(Scan)), got %A" other)

[<Fact>]
let ``C3: GROUP BY with aggregate and HAVING produces Aggregate then Having then Project`` () =
    let aggExpr = Expr.AggExpr(Count, AggArg.Star, false)
    let stmt =
        { Distinct = false
          Columns  = [ col None "name"; OutputColumn.Expr(aggExpr, Some "cnt") ]
          From     = [ "users", None ]
          Joins    = []
          Where    = None
          GroupBy  = [ Expr.Column(None, "name") ]
          Having   = Some (Expr.BinaryOp(Gt, aggExpr, Expr.Literal(SqlValue.Integer 1L)))
          OrderBy  = []
          Limit    = None }
    let plan = ok (Planner.plan schema (Statement.Select stmt))
    match plan with
    | LogicalPlan.Project(LogicalPlan.Having(LogicalPlan.Aggregate(LogicalPlan.Scan("users", None), groupBy, aggs), _), _) ->
        Assert.Equal(1, groupBy.Length)
        Assert.NotEmpty(aggs)
    | other -> failwith (sprintf "Expected Project(Having(Aggregate(Scan))), got %A" other)

[<Fact>]
let ``C4: JOIN clause produces a Join node above the Scans`` () =
    let onCond = Expr.BinaryOp(Eq, Expr.Column(Some "users", "id"), Expr.Column(Some "orders", "user_id"))
    let stmt =
        { Distinct = false
          Columns  = [ col (Some "users") "name"; col (Some "orders") "amount" ]
          From     = [ "users", None ]
          Joins    = [ { Kind = Inner; Table = "orders"; Alias = None; On = Some onCond } ]
          Where    = None
          GroupBy  = []
          Having   = None
          OrderBy  = []
          Limit    = None }
    let plan = ok (Planner.plan schema (Statement.Select stmt))
    match plan with
    | LogicalPlan.Project(LogicalPlan.Join(LogicalPlan.Scan("users", _), LogicalPlan.Scan("orders", _), Inner, Some _), _) -> ()
    | other -> failwith (sprintf "Expected Project(Join(Scan,Scan)), got %A" other)

[<Fact>]
let ``C5: ORDER BY produces Sort at the top`` () =
    let stmt = selectFrom [col None "name"] ["users", None]
    let plan = ok (Planner.plan schema (Statement.Select
        { stmt with OrderBy = [ { KeyExpr = Expr.Column(None, "age"); Direction = Desc; NullOrder = NullsLast } ] }))
    match plan with
    | LogicalPlan.Sort(_, keys) -> Assert.Equal(1, keys.Length)
    | other -> failwith (sprintf "Expected Sort at top, got %A" other)

[<Fact>]
let ``C6: LIMIT and OFFSET produce Limit at the top`` () =
    let stmt = selectFrom [col None "id"] ["users", None]
    let plan = ok (Planner.plan schema (Statement.Select
        { stmt with Limit = Some { Count = Some 10L; Offset = Some 20L } }))
    match plan with
    | LogicalPlan.Limit(_, Some 10L, Some 20L) -> ()
    | other -> failwith (sprintf "Expected Limit(_, 10, 20), got %A" other)

[<Fact>]
let ``C7: SELECT DISTINCT wraps Project in Distinct`` () =
    let stmt = selectFrom [col None "name"] ["users", None]
    let plan = ok (Planner.plan schema (Statement.Select { stmt with Distinct = true }))
    match plan with
    | LogicalPlan.Distinct(LogicalPlan.Project _) -> ()
    | other -> failwith (sprintf "Expected Distinct(Project(_)), got %A" other)

[<Fact>]
let ``C8: INSERT produces Insert node with Values source`` () =
    let stmt =
        Statement.Insert
            { Table   = "users"
              Columns = Some ["id"; "name"; "age"]
              Values  = [ [ Expr.Literal(SqlValue.Integer 1L)
                            Expr.Literal(SqlValue.Text "Alice")
                            Expr.Literal(SqlValue.Integer 30L) ] ] }
    let plan = ok (Planner.plan schema stmt)
    match plan with
    | LogicalPlan.Insert("users", Some cols, InsertSource.Values rows) ->
        Assert.Equal(3, cols.Length); Assert.Equal(1, rows.Length)
    | other -> failwith (sprintf "Expected Insert, got %A" other)

[<Fact>]
let ``C9: UPDATE produces Update node with assignments and predicate`` () =
    let stmt =
        Statement.Update
            { Table       = "users"
              Assignments = [ { Column = "name"; Value = Expr.Literal(SqlValue.Text "Bob") } ]
              Where       = Some (Expr.BinaryOp(Eq, Expr.Column(None, "id"), Expr.Literal(SqlValue.Integer 1L))) }
    let plan = ok (Planner.plan schema stmt)
    match plan with
    | LogicalPlan.Update("users", asgns, Some _) ->
        Assert.Equal(1, asgns.Length); Assert.Equal("name", asgns.[0].Column)
    | other -> failwith (sprintf "Expected Update, got %A" other)

[<Fact>]
let ``C10: DELETE produces Delete node`` () =
    let stmt = Statement.Delete { Table = "users"; Where = Some (Expr.Literal(SqlValue.Bool true)) }
    let plan = ok (Planner.plan schema stmt)
    match plan with
    | LogicalPlan.Delete("users", Some _) -> ()
    | other -> failwith (sprintf "Expected Delete, got %A" other)

[<Fact>]
let ``C11a: CREATE TABLE produces CreateTable leaf node`` () =
    let stmt =
        Statement.CreateTable
            { Table       = "products"
              IfNotExists = true
              Columns     =
                [ { Name = "id";    TypeName = "INTEGER"; NotNull = true;  PrimaryKey = true;  Unique = false; Default = None }
                  { Name = "name";  TypeName = "TEXT";    NotNull = true;  PrimaryKey = false; Unique = false; Default = None }
                  { Name = "price"; TypeName = "REAL";    NotNull = false; PrimaryKey = false; Unique = false; Default = None } ] }
    let plan = ok (Planner.plan schema stmt)
    match plan with
    | LogicalPlan.CreateTable("products", true, cols) -> Assert.Equal(3, cols.Length)
    | other -> failwith (sprintf "Expected CreateTable, got %A" other)

[<Fact>]
let ``C11b: DROP TABLE produces DropTable leaf node`` () =
    let plan = ok (Planner.plan schema (Statement.DropTable { Table = "users"; IfExists = false }))
    match plan with
    | LogicalPlan.DropTable("users", false) -> ()
    | other -> failwith (sprintf "Expected DropTable, got %A" other)

[<Fact>]
let ``C12: Ambiguous column reference yields AmbiguousColumn error`` () =
    // Both "users" and "orders" have "id".
    let stmt =
        { Distinct = false
          Columns  = [ col None "id" ]
          From     = [ "users", None ]
          Joins    = [ { Kind = Inner; Table = "orders"; Alias = None; On = None } ]
          Where    = None
          GroupBy  = []
          Having   = None
          OrderBy  = []
          Limit    = None }
    let e = err (Planner.plan schema (Statement.Select stmt))
    match e with
    | PlanError.AmbiguousColumn("id", tables) -> Assert.Equal(2, tables.Length)
    | other -> failwith (sprintf "Expected AmbiguousColumn, got %A" other)

[<Fact>]
let ``C13: Unknown table yields UnknownTable error`` () =
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["ghost", None])))
    match e with
    | PlanError.UnknownTable "ghost" -> ()
    | other -> failwith (sprintf "Expected UnknownTable, got %A" other)

// ──────────────────────────────────────────────────────────────────────────────
// planAll helper
// ──────────────────────────────────────────────────────────────────────────────

[<Fact>]
let ``planAll returns list of plans, fails on first error`` () =
    let stmts =
        [ Statement.Insert { Table = "users"; Columns = None; Values = [] }
          Statement.Delete { Table = "users"; Where = None } ]
    let plans = ok (Planner.planAll schema stmts)
    Assert.Equal(2, plans.Length)

    let e = err (Planner.planAll schema [Statement.Select (selectFrom [col None "x"] ["ghost", None])])
    match e with
    | PlanError.UnknownTable "ghost" -> ()
    | other -> failwith (sprintf "Expected UnknownTable, got %A" other)

// ──────────────────────────────────────────────────────────────────────────────
// Expression type coverage — exercise resolveExpr for every Expr variant
// ──────────────────────────────────────────────────────────────────────────────

[<Fact>]
let ``WHERE: FuncCall with resolved args is accepted`` () =
    let pred = Expr.FuncCall("upper", [ Expr.Column(None, "name") ])
    let stmt = selectFrom [col None "id"] ["users", None]
    let plan = ok (Planner.plan schema (Statement.Select { stmt with Where = Some pred }))
    match plan with
    | LogicalPlan.Project(LogicalPlan.Filter(_, Expr.FuncCall("upper", [Expr.Column(Some "users", "name")])), _) -> ()
    | other -> failwith (sprintf "Expected Filter with FuncCall, got %A" other)

[<Fact>]
let ``WHERE: IsNull and IsNotNull are resolved`` () =
    let pred1 = Expr.IsNull(Expr.Column(None, "age"))
    let stmt = selectFrom [col None "id"] ["users", None]
    let plan1 = ok (Planner.plan schema (Statement.Select { stmt with Where = Some pred1 }))
    match plan1 with
    | LogicalPlan.Project(LogicalPlan.Filter(_, Expr.IsNull(Expr.Column(Some "users", "age"))), _) -> ()
    | other -> failwith (sprintf "Expected Filter with IsNull, got %A" other)

    let pred2 = Expr.IsNotNull(Expr.Column(None, "name"))
    let plan2 = ok (Planner.plan schema (Statement.Select { stmt with Where = Some pred2 }))
    match plan2 with
    | LogicalPlan.Project(LogicalPlan.Filter(_, Expr.IsNotNull(Expr.Column(Some "users", "name"))), _) -> ()
    | other -> failwith (sprintf "Expected Filter with IsNotNull, got %A" other)

[<Fact>]
let ``WHERE: Between is resolved`` () =
    let pred = Expr.Between(Expr.Column(None, "age"), Expr.Literal(SqlValue.Integer 18L), Expr.Literal(SqlValue.Integer 65L))
    let stmt = selectFrom [col None "id"] ["users", None]
    let plan = ok (Planner.plan schema (Statement.Select { stmt with Where = Some pred }))
    match plan with
    | LogicalPlan.Project(LogicalPlan.Filter(_, Expr.Between _), _) -> ()
    | other -> failwith (sprintf "Expected Filter with Between, got %A" other)

[<Fact>]
let ``WHERE: In and NotIn are resolved`` () =
    let items = [ Expr.Literal(SqlValue.Integer 1L); Expr.Literal(SqlValue.Integer 2L) ]
    let pred1 = Expr.In(Expr.Column(None, "id"), items)
    let stmt = selectFrom [col None "name"] ["users", None]
    let plan1 = ok (Planner.plan schema (Statement.Select { stmt with Where = Some pred1 }))
    match plan1 with
    | LogicalPlan.Project(LogicalPlan.Filter(_, Expr.In _), _) -> ()
    | other -> failwith (sprintf "Expected Filter with In, got %A" other)

    let pred2 = Expr.NotIn(Expr.Column(None, "id"), items)
    let plan2 = ok (Planner.plan schema (Statement.Select { stmt with Where = Some pred2 }))
    match plan2 with
    | LogicalPlan.Project(LogicalPlan.Filter(_, Expr.NotIn _), _) -> ()
    | other -> failwith (sprintf "Expected Filter with NotIn, got %A" other)

[<Fact>]
let ``WHERE: Like and NotLike are resolved`` () =
    let pred1 = Expr.Like(Expr.Column(None, "name"), "A%")
    let stmt = selectFrom [col None "id"] ["users", None]
    let plan1 = ok (Planner.plan schema (Statement.Select { stmt with Where = Some pred1 }))
    match plan1 with
    | LogicalPlan.Project(LogicalPlan.Filter(_, Expr.Like(Expr.Column(Some "users", "name"), "A%")), _) -> ()
    | other -> failwith (sprintf "Expected Filter with Like, got %A" other)

    let pred2 = Expr.NotLike(Expr.Column(None, "name"), "B%")
    let plan2 = ok (Planner.plan schema (Statement.Select { stmt with Where = Some pred2 }))
    match plan2 with
    | LogicalPlan.Project(LogicalPlan.Filter(_, Expr.NotLike _), _) -> ()
    | other -> failwith (sprintf "Expected Filter with NotLike, got %A" other)

[<Fact>]
let ``WHERE: UnaryOp (Not) is resolved`` () =
    let pred = Expr.UnaryOp(Not, Expr.BinaryOp(Eq, Expr.Column(None, "age"), Expr.Literal(SqlValue.Integer 0L)))
    let stmt = selectFrom [col None "id"] ["users", None]
    let plan = ok (Planner.plan schema (Statement.Select { stmt with Where = Some pred }))
    match plan with
    | LogicalPlan.Project(LogicalPlan.Filter(_, Expr.UnaryOp(Not, _)), _) -> ()
    | other -> failwith (sprintf "Expected Filter with UnaryOp(Not), got %A" other)

[<Fact>]
let ``WHERE: qualified column reference is resolved`` () =
    let pred = Expr.BinaryOp(Gt, Expr.Column(Some "users", "age"), Expr.Literal(SqlValue.Integer 21L))
    let stmt = selectFrom [col None "id"] ["users", None]
    let plan = ok (Planner.plan schema (Statement.Select { stmt with Where = Some pred }))
    match plan with
    | LogicalPlan.Project(LogicalPlan.Filter(_, Expr.BinaryOp(Gt, Expr.Column(Some "users", "age"), _)), _) -> ()
    | other -> failwith (sprintf "Expected resolved qualified column, got %A" other)

[<Fact>]
let ``WHERE: qualified column with wrong table yields UnknownTable`` () =
    let pred = Expr.BinaryOp(Eq, Expr.Column(Some "no_such", "id"), Expr.Literal(SqlValue.Integer 1L))
    let stmt = selectFrom [col None "id"] ["users", None]
    let e = err (Planner.plan schema (Statement.Select { stmt with Where = Some pred }))
    match e with
    | PlanError.UnknownTable "no_such" -> ()
    | other -> failwith (sprintf "Expected UnknownTable, got %A" other)

[<Fact>]
let ``WHERE: qualified column not in table yields UnknownColumn`` () =
    let pred = Expr.Column(Some "users", "no_col")
    let stmt = selectFrom [col None "id"] ["users", None]
    let e = err (Planner.plan schema (Statement.Select { stmt with Where = Some pred }))
    match e with
    | PlanError.UnknownColumn(Some "users", "no_col") -> ()
    | other -> failwith (sprintf "Expected UnknownColumn, got %A" other)

// ──────────────────────────────────────────────────────────────────────────────
// Error paths
// ──────────────────────────────────────────────────────────────────────────────

[<Fact>]
let ``INSERT on unknown table yields UnknownTable`` () =
    let e = err (Planner.plan schema (Statement.Insert { Table = "ghost"; Columns = None; Values = [] }))
    match e with
    | PlanError.UnknownTable "ghost" -> ()
    | other -> failwith (sprintf "Expected UnknownTable, got %A" other)

[<Fact>]
let ``UPDATE on unknown table yields UnknownTable`` () =
    let e = err (Planner.plan schema (Statement.Update { Table = "ghost"; Assignments = []; Where = None }))
    match e with
    | PlanError.UnknownTable "ghost" -> ()
    | other -> failwith (sprintf "Expected UnknownTable, got %A" other)

[<Fact>]
let ``DELETE on unknown table yields UnknownTable`` () =
    let e = err (Planner.plan schema (Statement.Delete { Table = "ghost"; Where = None }))
    match e with
    | PlanError.UnknownTable "ghost" -> ()
    | other -> failwith (sprintf "Expected UnknownTable, got %A" other)

[<Fact>]
let ``WHERE BinaryOp propagates left error`` () =
    let pred = Expr.BinaryOp(Eq, Expr.Column(None, "no_col"), Expr.Literal(SqlValue.Integer 1L))
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn, got %A" other)

[<Fact>]
let ``WHERE BinaryOp propagates right error`` () =
    let pred = Expr.BinaryOp(Eq, Expr.Column(None, "id"), Expr.Column(None, "no_col"))
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn, got %A" other)

[<Fact>]
let ``JOIN with unknown join table yields UnknownTable`` () =
    let stmt =
        { Distinct = false
          Columns  = [ col None "id" ]
          From     = [ "users", None ]
          Joins    = [ { Kind = Inner; Table = "no_such"; Alias = None; On = None } ]
          Where    = None
          GroupBy  = []
          Having   = None
          OrderBy  = []
          Limit    = None }
    let e = err (Planner.plan schema (Statement.Select stmt))
    match e with
    | PlanError.UnknownTable "no_such" -> ()
    | other -> failwith (sprintf "Expected UnknownTable for join table, got %A" other)

// ──────────────────────────────────────────────────────────────────────────────
// Aggregate expression variants
// ──────────────────────────────────────────────────────────────────────────────

[<Fact>]
let ``SELECT with SUM, AVG, MIN, MAX aggregates produces Aggregate node`` () =
    let aggs =
        [ Expr.AggExpr(Sum, AggArg.Expr(Expr.Column(None, "age")), false)
          Expr.AggExpr(Avg, AggArg.Expr(Expr.Column(None, "age")), false)
          Expr.AggExpr(Min, AggArg.Expr(Expr.Column(None, "age")), false)
          Expr.AggExpr(Max, AggArg.Expr(Expr.Column(None, "age")), false) ]
    let cols = aggs |> List.mapi (fun i e -> OutputColumn.Expr(e, Some (sprintf "a%d" i)))
    let stmt = selectFrom cols ["users", None]
    let plan = ok (Planner.plan schema (Statement.Select stmt))
    match plan with
    | LogicalPlan.Project(LogicalPlan.Aggregate(LogicalPlan.Scan("users", None), [], aggItems), _) ->
        Assert.Equal(4, aggItems.Length)
    | other -> failwith (sprintf "Expected Project(Aggregate(Scan)), got %A" other)

[<Fact>]
let ``SELECT with COUNT DISTINCT produces Aggregate with distinct=true`` () =
    let aggExpr = Expr.AggExpr(Count, AggArg.Expr(Expr.Column(None, "name")), true)
    let stmt = selectFrom [ OutputColumn.Expr(aggExpr, Some "cnt") ] ["users", None]
    let plan = ok (Planner.plan schema (Statement.Select stmt))
    match plan with
    | LogicalPlan.Project(LogicalPlan.Aggregate(_, _, aggs), _) ->
        Assert.True(aggs |> List.exists (fun a -> a.Distinct))
    | other -> failwith (sprintf "Expected Aggregate with distinct=true, got %A" other)

// ──────────────────────────────────────────────────────────────────────────────
// Multi-table FROM (cross join)
// ──────────────────────────────────────────────────────────────────────────────

[<Fact>]
let ``Two bare FROM tables produce cross-join`` () =
    let stmt =
        { Distinct = false
          Columns  = [ col (Some "users") "id"; col (Some "orders") "amount" ]
          From     = [ "users", None; "orders", None ]
          Joins    = []
          Where    = None
          GroupBy  = []
          Having   = None
          OrderBy  = []
          Limit    = None }
    let plan = ok (Planner.plan schema (Statement.Select stmt))
    match plan with
    | LogicalPlan.Project(LogicalPlan.Join(LogicalPlan.Scan("users", _), LogicalPlan.Scan("orders", _), Cross, None), _) -> ()
    | other -> failwith (sprintf "Expected cross-join from two FROM tables, got %A" other)

// ──────────────────────────────────────────────────────────────────────────────
// Table aliases
// ──────────────────────────────────────────────────────────────────────────────

[<Fact>]
let ``Table alias is threaded through scope and plan`` () =
    let pred = Expr.BinaryOp(Eq, Expr.Column(Some "u", "id"), Expr.Literal(SqlValue.Integer 1L))
    let stmt =
        { Distinct = false
          Columns  = [ OutputColumn.Expr(Expr.Column(Some "u", "name"), None) ]
          From     = [ "users", Some "u" ]
          Joins    = []
          Where    = Some pred
          GroupBy  = []
          Having   = None
          OrderBy  = []
          Limit    = None }
    let plan = ok (Planner.plan schema (Statement.Select stmt))
    match plan with
    | LogicalPlan.Project(LogicalPlan.Filter(LogicalPlan.Scan("users", Some "u"), _), _) -> ()
    | other -> failwith (sprintf "Expected Project(Filter(Scan(alias=u))), got %A" other)

// ──────────────────────────────────────────────────────────────────────────────
// Wildcard
// ──────────────────────────────────────────────────────────────────────────────

[<Fact>]
let ``SELECT star is preserved in Project columns`` () =
    let stmt = selectFrom [ OutputColumn.Star ] ["users", None]
    let plan = ok (Planner.plan schema (Statement.Select stmt))
    match plan with
    | LogicalPlan.Project(LogicalPlan.Scan("users", None), [OutputColumn.Star]) -> ()
    | other -> failwith (sprintf "Expected Project with Star, got %A" other)

// ──────────────────────────────────────────────────────────────────────────────
// DISTINCT + ORDER BY + LIMIT stacking
// ──────────────────────────────────────────────────────────────────────────────

[<Fact>]
let ``DISTINCT + ORDER BY + LIMIT stacks Limit(Sort(Distinct(Project)))`` () =
    let stmt =
        { Distinct = true
          Columns  = [ col None "name" ]
          From     = [ "users", None ]
          Joins    = []
          Where    = None
          GroupBy  = []
          Having   = None
          OrderBy  = [ { KeyExpr = Expr.Column(None, "name"); Direction = Asc; NullOrder = NullsFirst } ]
          Limit    = Some { Count = Some 5L; Offset = None } }
    let plan = ok (Planner.plan schema (Statement.Select stmt))
    match plan with
    | LogicalPlan.Limit(LogicalPlan.Sort(LogicalPlan.Distinct(LogicalPlan.Project _), _), Some 5L, None) -> ()
    | other -> failwith (sprintf "Expected Limit(Sort(Distinct(Project))), got %A" other)

// ──────────────────────────────────────────────────────────────────────────────
// Error propagation inside complex expressions
// ──────────────────────────────────────────────────────────────────────────────

[<Fact>]
let ``Between: error in low propagates`` () =
    let pred = Expr.Between(Expr.Column(None, "age"), Expr.Column(None, "no_col"), Expr.Literal(SqlValue.Integer 65L))
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from Between.lo, got %A" other)

[<Fact>]
let ``Between: error in high propagates`` () =
    let pred = Expr.Between(Expr.Column(None, "age"), Expr.Literal(SqlValue.Integer 0L), Expr.Column(None, "no_col"))
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from Between.hi, got %A" other)

[<Fact>]
let ``In: error in item propagates`` () =
    let pred = Expr.In(Expr.Column(None, "id"), [ Expr.Literal(SqlValue.Integer 1L); Expr.Column(None, "no_col") ])
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from In.items, got %A" other)

[<Fact>]
let ``NotIn: error in item propagates`` () =
    let pred = Expr.NotIn(Expr.Column(None, "id"), [ Expr.Column(None, "no_col") ])
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from NotIn.items, got %A" other)

[<Fact>]
let ``FuncCall: error in arg propagates`` () =
    let pred = Expr.FuncCall("upper", [ Expr.Column(None, "no_col") ])
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from FuncCall.args, got %A" other)

[<Fact>]
let ``IsNull: error in inner expr propagates`` () =
    let pred = Expr.IsNull(Expr.Column(None, "no_col"))
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from IsNull, got %A" other)

[<Fact>]
let ``IsNotNull: error in inner expr propagates`` () =
    let pred = Expr.IsNotNull(Expr.Column(None, "no_col"))
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from IsNotNull, got %A" other)

[<Fact>]
let ``Like: error in value expr propagates`` () =
    let pred = Expr.Like(Expr.Column(None, "no_col"), "A%")
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from Like, got %A" other)

[<Fact>]
let ``NotLike: error in value expr propagates`` () =
    let pred = Expr.NotLike(Expr.Column(None, "no_col"), "B%")
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from NotLike, got %A" other)

[<Fact>]
let ``UnaryOp: error in inner expr propagates`` () =
    let pred = Expr.UnaryOp(Not, Expr.Column(None, "no_col"))
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from UnaryOp, got %A" other)

[<Fact>]
let ``PROJECT: unknown column in non-agg context yields UnknownColumn`` () =
    let stmt = selectFrom [ col None "no_col" ] ["users", None]
    let e = err (Planner.plan schema (Statement.Select stmt))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from Project, got %A" other)

[<Fact>]
let ``GROUP BY: unknown column yields error`` () =
    let stmt = selectFrom [ OutputColumn.Expr(Expr.AggExpr(Count, AggArg.Star, false), Some "cnt") ] ["users", None]
    let e = err (Planner.plan schema (Statement.Select { stmt with GroupBy = [ Expr.Column(None, "no_col") ] }))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from GROUP BY, got %A" other)

[<Fact>]
let ``Multi-FROM: second table unknown yields UnknownTable`` () =
    let stmt =
        { selectFrom [ col (Some "users") "id" ] ["users", None] with
            From = [ "users", None; "ghost", None ]
            Columns = [ col (Some "users") "id" ] }
    let e = err (Planner.plan schema (Statement.Select stmt))
    match e with
    | PlanError.UnknownTable "ghost" -> ()
    | other -> failwith (sprintf "Expected UnknownTable for second FROM, got %A" other)

[<Fact>]
let ``In: error in value expr propagates`` () =
    let pred = Expr.In(Expr.Column(None, "no_col"), [ Expr.Literal(SqlValue.Integer 1L) ])
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from In.value, got %A" other)

[<Fact>]
let ``NotIn: error in value expr propagates`` () =
    let pred = Expr.NotIn(Expr.Column(None, "no_col"), [ Expr.Literal(SqlValue.Integer 1L) ])
    let e = err (Planner.plan schema (Statement.Select (selectFrom [col None "id"] ["users", None] |> fun s -> { s with Where = Some pred })))
    match e with
    | PlanError.UnknownColumn _ -> ()
    | other -> failwith (sprintf "Expected UnknownColumn from NotIn.value, got %A" other)

// ──────────────────────────────────────────────────────────────────────────────
// SqlValue literals coverage
// ──────────────────────────────────────────────────────────────────────────────

[<Fact>]
let ``Literal variants are preserved unchanged through resolveExpr`` () =
    let literals =
        [ SqlValue.Null
          SqlValue.Integer 42L
          SqlValue.Real 3.14
          SqlValue.Text "hello"
          SqlValue.Bool true ]
        |> List.map (fun v -> OutputColumn.Expr(Expr.Literal v, None))
    let stmt = selectFrom literals ["users", None]
    let plan = ok (Planner.plan schema (Statement.Select stmt))
    match plan with
    | LogicalPlan.Project(_, cols) -> Assert.Equal(5, cols.Length)
    | other -> failwith (sprintf "Expected Project with 5 literal cols, got %A" other)
