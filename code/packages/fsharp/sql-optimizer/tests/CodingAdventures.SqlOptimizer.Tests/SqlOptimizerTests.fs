// SqlOptimizerTests.fs — xUnit tests for the F# sql-optimizer.
//
// Coverage targets:
//   ConstantFolding   — arithmetic, comparison, boolean short-circuit, NULL propagation
//   PredicatePushdown — through Sort, Distinct, Join, Project; outer-join safety
//   ProjectionPruning — requiredColumns annotation on Scans
//   DeadCodeElimination — EmptyResult propagation, LIMIT 0
//   LimitPushdown     — scanLimit annotation
//   Compose pipeline  — multiple passes interact correctly

module CodingAdventures.SqlOptimizer.Tests

open Xunit
open CodingAdventures.SqlPlanner.FSharp
open CodingAdventures.SqlOptimizer.FSharp

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build a simple Scan-backed plan for a named table.
let scanPlan table = LogicalPlan.Scan(table, None)

/// Build a Filter over a scan.
let filterPlan table pred = LogicalPlan.Filter(scanPlan table, pred)

/// Integer literal shorthand.
let intLit n = Expr.Literal(SqlValue.Integer n)

/// Real literal shorthand.
let realLit r = Expr.Literal(SqlValue.Real r)

/// Bool literal shorthand.
let boolLit b = Expr.Literal(SqlValue.Bool b)

/// Null literal shorthand.
let nullLit = Expr.Literal SqlValue.Null

/// Column reference with table qualifier.
let col table name = Expr.Column(Some table, name)

/// Binary expression shorthand.
let binop op l r = Expr.BinaryOp(op, l, r)

// ─── ConstantFolding ─────────────────────────────────────────────────────────

[<Fact>]
let ``CF: 1 + 2 folds to 3`` () =
    let plan = filterPlan "t" (binop Add (intLit 1L) (intLit 2L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Integer 3L)) -> ()
    | other -> failwithf "Expected Filter with Integer 3, got %A" other

[<Fact>]
let ``CF: 10 - 4 folds to 6`` () =
    let plan = filterPlan "t" (binop Sub (intLit 10L) (intLit 4L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Integer 6L)) -> ()
    | other -> failwithf "Expected Integer 6, got %A" other

[<Fact>]
let ``CF: 3 * 4 folds to 12`` () =
    let plan = filterPlan "t" (binop Mul (intLit 3L) (intLit 4L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Integer 12L)) -> ()
    | other -> failwithf "Expected Integer 12, got %A" other

[<Fact>]
let ``CF: 7 / 2 folds to 3 (truncate toward zero)`` () =
    let plan = filterPlan "t" (binop Div (intLit 7L) (intLit 2L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Integer 3L)) -> ()
    | other -> failwithf "Expected Integer 3 (truncated), got %A" other

[<Fact>]
let ``CF: -7 / 2 folds to -3 (truncate toward zero)`` () =
    let plan = filterPlan "t" (binop Div (intLit -7L) (intLit 2L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Integer -3L)) -> ()
    | other -> failwithf "Expected Integer -3, got %A" other

[<Fact>]
let ``CF: division by zero is not folded`` () =
    let plan = filterPlan "t" (binop Div (intLit 5L) (intLit 0L))
    let result = SqlOptimizer.optimize plan
    // Should remain a BinaryOp, not fold (div by zero deferred to VM)
    match result with
    | OptimizedPlan.Filter(_, Expr.BinaryOp(Div, _, _)) -> ()
    | other -> failwithf "Expected un-folded Div, got %A" other

[<Fact>]
let ``CF: 1.5 + 2.5 folds to 4.0`` () =
    let plan = filterPlan "t" (binop Add (realLit 1.5) (realLit 2.5))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Real 4.0)) -> ()
    | other -> failwithf "Expected Real 4.0, got %A" other

[<Fact>]
let ``CF: 3 = 3 folds to true, DCE removes tautological filter`` () =
    // After ConstantFolding: Filter(Scan, TRUE). After DeadCodeElimination: Scan.
    let plan = filterPlan "t" (binop Eq (intLit 3L) (intLit 3L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Scan _ -> ()  // TRUE filter eliminated by DCE
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool true)) -> ()
    | other -> failwithf "Expected Scan (DCE) or TRUE filter, got %A" other

[<Fact>]
let ``CF: 1 < 2 folds to true, DCE removes tautological filter`` () =
    // After ConstantFolding: Filter(Scan, TRUE). After DeadCodeElimination: Scan.
    let plan = filterPlan "t" (binop Lt (intLit 1L) (intLit 2L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Scan _ -> ()  // TRUE filter eliminated by DCE
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool true)) -> ()
    | other -> failwithf "Expected Scan (DCE) or TRUE filter, got %A" other

[<Fact>]
let ``CF: 5 > 10 folds to false, DCE produces EmptyResult`` () =
    // After ConstantFolding: Filter(Scan, FALSE). After DeadCodeElimination: EmptyResult.
    let plan = filterPlan "t" (binop Gt (intLit 5L) (intLit 10L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool false)) -> ()
    | other -> failwithf "Expected EmptyResult or FALSE filter, got %A" other

[<Fact>]
let ``CF: FALSE AND x -> FALSE (short-circuit)`` () =
    let plan = filterPlan "t" (binop And (boolLit false) (col "t" "x"))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()  // DCE will catch FALSE filter
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool false)) -> ()
    | other -> failwithf "Expected EmptyResult or FALSE filter, got %A" other

[<Fact>]
let ``CF: TRUE AND x -> x (identity)`` () =
    let plan = filterPlan "t" (binop And (boolLit true) (col "t" "active"))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Column(Some "t", "active")) -> ()
    | other -> failwithf "Expected Filter with Column, got %A" other

[<Fact>]
let ``CF: TRUE OR x -> TRUE (short-circuit)`` () =
    let plan = filterPlan "t" (binop Or (boolLit true) (col "t" "x"))
    let result = SqlOptimizer.optimize plan
    // TRUE OR x = TRUE, then DCE removes the tautology filter
    match result with
    | OptimizedPlan.Scan _ -> ()   // after DCE removes the tautological filter
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool true)) -> ()
    | other -> failwithf "Expected Scan (tautology removed) or TRUE filter, got %A" other

[<Fact>]
let ``CF: FALSE OR x -> x (identity)`` () =
    let plan = filterPlan "t" (binop Or (boolLit false) (col "t" "active"))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Column(Some "t", "active")) -> ()
    | other -> failwithf "Expected Filter with Column, got %A" other

[<Fact>]
let ``CF: NULL propagates in arithmetic`` () =
    let plan = filterPlan "t" (binop Add nullLit (intLit 5L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()  // NULL filter -> EmptyResult via DCE
    | OptimizedPlan.Filter(_, Expr.Literal SqlValue.Null) -> ()
    | other -> failwithf "Expected NULL or EmptyResult, got %A" other

[<Fact>]
let ``CF: NOT true folds to false`` () =
    let plan = filterPlan "t" (Expr.UnaryOp(Not, boolLit true))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool false)) -> ()
    | other -> failwithf "Expected false filter or EmptyResult, got %A" other

[<Fact>]
let ``CF: NOT false folds to true`` () =
    let plan = filterPlan "t" (Expr.UnaryOp(Not, boolLit false))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Scan _ -> ()  // tautology removed
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool true)) -> ()
    | other -> failwithf "Expected Scan or TRUE, got %A" other

[<Fact>]
let ``CF: NEG integer folds`` () =
    let plan = filterPlan "t" (Expr.UnaryOp(Neg, intLit 5L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Integer -5L)) -> ()
    | other -> failwithf "Expected Integer -5, got %A" other

[<Fact>]
let ``CF: IsNull of NULL folds to true`` () =
    let plan = filterPlan "t" (Expr.IsNull(Expr.Literal SqlValue.Null))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Scan _ -> ()   // tautology removed by DCE
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool true)) -> ()
    | other -> failwithf "Expected Scan or TRUE, got %A" other

[<Fact>]
let ``CF: IsNull of non-null literal folds to false`` () =
    let plan = filterPlan "t" (Expr.IsNull(intLit 42L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool false)) -> ()
    | other -> failwithf "Expected EmptyResult or FALSE, got %A" other

[<Fact>]
let ``CF: IsNotNull of NULL folds to false`` () =
    let plan = filterPlan "t" (Expr.IsNotNull(Expr.Literal SqlValue.Null))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool false)) -> ()
    | other -> failwithf "Expected EmptyResult or FALSE, got %A" other

[<Fact>]
let ``CF: IsNotNull of non-null literal folds to true`` () =
    let plan = filterPlan "t" (Expr.IsNotNull(intLit 1L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Scan _ -> ()
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool true)) -> ()
    | other -> failwithf "Expected Scan or TRUE, got %A" other

[<Fact>]
let ``CF: nested arithmetic folds bottom-up`` () =
    // (1 + 2) * (3 + 4) -> 3 * 7 -> 21
    let expr = binop Mul (binop Add (intLit 1L) (intLit 2L)) (binop Add (intLit 3L) (intLit 4L))
    let plan = filterPlan "t" expr
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Integer 21L)) -> ()
    | other -> failwithf "Expected Integer 21, got %A" other

// ─── PredicatePushdown ───────────────────────────────────────────────────────

[<Fact>]
let ``PPD: filter pushed through Sort`` () =
    // Filter(Sort(Scan("t")), pred) should become Sort(Filter(Scan("t"), pred))
    let pred = binop Eq (col "t" "id") (intLit 1L)
    let plan =
        LogicalPlan.Filter(
            LogicalPlan.Sort(
                scanPlan "t",
                [{ KeyExpr = col "t" "name"; Direction = Asc; NullOrder = NullsLast }]),
            pred)
    let result = SqlOptimizer.optimize plan
    // The filter should have been pushed below the sort
    match result with
    | OptimizedPlan.Sort(OptimizedPlan.Filter(OptimizedPlan.Scan _, _), _) -> ()
    | other -> failwithf "Expected Sort(Filter(Scan)), got %A" other

[<Fact>]
let ``PPD: filter pushed through Distinct`` () =
    let pred = binop Gt (col "t" "age") (intLit 18L)
    let plan =
        LogicalPlan.Filter(
            LogicalPlan.Distinct(scanPlan "t"),
            pred)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Distinct(OptimizedPlan.Filter(OptimizedPlan.Scan _, _)) -> ()
    | other -> failwithf "Expected Distinct(Filter(Scan)), got %A" other

[<Fact>]
let ``PPD: left predicate pushed into INNER JOIN left side`` () =
    // Filter over INNER JOIN where pred only references left table
    let pred = binop Eq (col "e" "dept") (Expr.Literal(SqlValue.Text "eng"))
    let plan =
        LogicalPlan.Filter(
            LogicalPlan.Join(
                LogicalPlan.Scan("employees", Some "e"),
                LogicalPlan.Scan("departments", Some "d"),
                Inner,
                Some (binop Eq (col "e" "dept_id") (col "d" "id"))),
            pred)
    let result = SqlOptimizer.optimize plan
    // The pred should have been pushed into the left scan side
    match result with
    | OptimizedPlan.Join(OptimizedPlan.Filter(OptimizedPlan.Scan _, _), _, _, _) -> ()
    | other -> failwithf "Expected Join(Filter(Scan), ...), got %A" other

[<Fact>]
let ``PPD: AND conjuncts split and distributed`` () =
    // Filter with two conjuncts — one per side of the join
    let predL = binop Gt (col "e" "salary") (intLit 50000L)
    let predR = binop Eq (col "d" "active") (boolLit true)
    let plan =
        LogicalPlan.Filter(
            LogicalPlan.Join(
                LogicalPlan.Scan("employees", Some "e"),
                LogicalPlan.Scan("departments", Some "d"),
                Inner,
                None),
            binop And predL predR)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Join(OptimizedPlan.Filter(OptimizedPlan.Scan _, _),
                         OptimizedPlan.Filter(OptimizedPlan.Scan _, _), _, _) -> ()
    | other -> failwithf "Expected both sides filtered, got %A" other

[<Fact>]
let ``PPD: filter not pushed through Aggregate`` () =
    // A HAVING-like filter above an Aggregate must stay above
    let pred = binop Gt (col "t" "total") (intLit 100L)
    let plan =
        LogicalPlan.Filter(
            LogicalPlan.Aggregate(scanPlan "t", [], []),
            pred)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(OptimizedPlan.Aggregate _, _) -> ()
    | other -> failwithf "Expected Filter(Aggregate), got %A" other

// ─── ProjectionPruning ───────────────────────────────────────────────────────

[<Fact>]
let ``PP: qualified column in Project annotates Scan`` () =
    let plan =
        LogicalPlan.Project(
            scanPlan "employees",
            [OutputColumn.Expr(col "employees" "name", None)])
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Project(OptimizedPlan.Scan(_, _, Some ["name"], _), _) -> ()
    | other -> failwithf "Expected Scan with requiredColumns [name], got %A" other

[<Fact>]
let ``PP: wildcard in Project suppresses pruning`` () =
    let plan =
        LogicalPlan.Project(
            scanPlan "t",
            [OutputColumn.Star])
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Project(OptimizedPlan.Scan(_, _, None, _), _) -> ()
    | other -> failwithf "Expected Scan with no requiredColumns, got %A" other

[<Fact>]
let ``PP: multiple columns from same scan collected`` () =
    let plan =
        LogicalPlan.Project(
            scanPlan "users",
            [OutputColumn.Expr(col "users" "id", None)
             OutputColumn.Expr(col "users" "email", None)])
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Project(OptimizedPlan.Scan(_, _, Some cols, _), _) ->
        let sorted = List.sort cols
        if sorted <> ["email"; "id"] then
            failwithf "Expected [email; id], got %A" sorted
    | other -> failwithf "Expected annotated Scan, got %A" other

// ─── DeadCodeElimination ─────────────────────────────────────────────────────

[<Fact>]
let ``DCE: Filter with FALSE predicate -> EmptyResult`` () =
    let plan = filterPlan "t" (boolLit false)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | other -> failwithf "Expected EmptyResult, got %A" other

[<Fact>]
let ``DCE: Filter with NULL predicate -> EmptyResult`` () =
    let plan = filterPlan "t" nullLit
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | other -> failwithf "Expected EmptyResult, got %A" other

[<Fact>]
let ``DCE: Filter with TRUE predicate removed`` () =
    let plan = filterPlan "t" (boolLit true)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Scan _ -> ()
    | other -> failwithf "Expected Scan (filter removed), got %A" other

[<Fact>]
let ``DCE: LIMIT 0 -> EmptyResult`` () =
    let plan = LogicalPlan.Limit(scanPlan "t", Some 0L, None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | other -> failwithf "Expected EmptyResult, got %A" other

[<Fact>]
let ``DCE: Project over EmptyResult -> EmptyResult`` () =
    let plan =
        LogicalPlan.Project(
            LogicalPlan.Filter(scanPlan "t", boolLit false),
            [OutputColumn.Star])
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | other -> failwithf "Expected EmptyResult, got %A" other

[<Fact>]
let ``DCE: Sort over EmptyResult -> EmptyResult`` () =
    let plan =
        LogicalPlan.Sort(
            LogicalPlan.Filter(scanPlan "t", boolLit false),
            [{ KeyExpr = col "t" "id"; Direction = Asc; NullOrder = NullsLast }])
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | other -> failwithf "Expected EmptyResult, got %A" other

[<Fact>]
let ``DCE: Distinct over EmptyResult -> EmptyResult`` () =
    let plan = LogicalPlan.Distinct(LogicalPlan.Filter(scanPlan "t", boolLit false))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | other -> failwithf "Expected EmptyResult, got %A" other

[<Fact>]
let ``DCE: Having over EmptyResult -> EmptyResult`` () =
    let plan =
        LogicalPlan.Having(
            LogicalPlan.Filter(scanPlan "t", boolLit false),
            binop Gt (col "t" "n") (intLit 0L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | other -> failwithf "Expected EmptyResult, got %A" other

[<Fact>]
let ``DCE: INNER JOIN with EmptyResult left -> EmptyResult`` () =
    let plan =
        LogicalPlan.Join(
            LogicalPlan.Filter(scanPlan "a", boolLit false),
            scanPlan "b",
            Inner,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | other -> failwithf "Expected EmptyResult, got %A" other

[<Fact>]
let ``DCE: INNER JOIN with EmptyResult right -> EmptyResult`` () =
    let plan =
        LogicalPlan.Join(
            scanPlan "a",
            LogicalPlan.Filter(scanPlan "b", boolLit false),
            Inner,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | other -> failwithf "Expected EmptyResult, got %A" other

[<Fact>]
let ``DCE: LEFT JOIN with EmptyResult right stays (null-padding preserved)`` () =
    let plan =
        LogicalPlan.Join(
            scanPlan "a",
            LogicalPlan.Filter(scanPlan "b", boolLit false),
            Left,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Join(OptimizedPlan.Scan _, OptimizedPlan.EmptyResult, Left, _) -> ()
    | other -> failwithf "Expected Join preserved (LEFT outer), got %A" other

[<Fact>]
let ``DCE: Union where left is empty reduces to right`` () =
    let plan =
        LogicalPlan.Union(
            LogicalPlan.Filter(scanPlan "a", boolLit false),
            scanPlan "b",
            false)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Scan("b", _, _, _) -> ()
    | other -> failwithf "Expected right scan, got %A" other

[<Fact>]
let ``DCE: Union where right is empty reduces to left`` () =
    let plan =
        LogicalPlan.Union(
            scanPlan "a",
            LogicalPlan.Filter(scanPlan "b", boolLit false),
            false)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Scan("a", _, _, _) -> ()
    | other -> failwithf "Expected left scan, got %A" other

[<Fact>]
let ``DCE: Aggregate over EmptyResult NOT eliminated (COUNT(*) = 0 semantics)`` () =
    let plan =
        LogicalPlan.Aggregate(
            LogicalPlan.Filter(scanPlan "t", boolLit false),
            [],
            [{ Func = Count; Arg = AggArg.Star; Alias = "n"; Distinct = false }])
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Aggregate(OptimizedPlan.EmptyResult, _, _) -> ()
    | other -> failwithf "Expected Aggregate(EmptyResult) preserved, got %A" other

// ─── LimitPushdown ───────────────────────────────────────────────────────────

[<Fact>]
let ``LP: LIMIT annotates Scan with scanLimit`` () =
    let plan = LogicalPlan.Limit(scanPlan "t", Some 10L, None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Limit(OptimizedPlan.Scan(_, _, _, Some 10L), _, _) -> ()
    | other -> failwithf "Expected Scan with scanLimit=10, got %A" other

[<Fact>]
let ``LP: LIMIT with offset does not push scanLimit`` () =
    // Non-zero offset: conservative — don't push
    let plan = LogicalPlan.Limit(scanPlan "t", Some 10L, Some 5L)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Limit(OptimizedPlan.Scan(_, _, _, None), _, _) -> ()
    | other -> failwithf "Expected Scan without scanLimit (offset present), got %A" other

[<Fact>]
let ``LP: LIMIT pushes through Project to Scan`` () =
    let plan =
        LogicalPlan.Limit(
            LogicalPlan.Project(
                scanPlan "t",
                [OutputColumn.Expr(col "t" "id", None)]),
            Some 5L,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Limit(
        OptimizedPlan.Project(
            OptimizedPlan.Scan(_, _, _, Some 5L), _), _, _) -> ()
    | other -> failwithf "Expected Scan with scanLimit=5 under Project, got %A" other

[<Fact>]
let ``LP: LIMIT pushes through Filter to Scan`` () =
    let plan =
        LogicalPlan.Limit(
            LogicalPlan.Filter(scanPlan "t", binop Gt (col "t" "age") (intLit 18L)),
            Some 3L,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Limit(
        OptimizedPlan.Filter(
            OptimizedPlan.Scan(_, _, _, Some 3L), _), _, _) -> ()
    | other -> failwithf "Expected Scan with scanLimit=3 under Filter, got %A" other

[<Fact>]
let ``LP: LIMIT does not push past Sort`` () =
    let plan =
        LogicalPlan.Limit(
            LogicalPlan.Sort(
                scanPlan "t",
                [{ KeyExpr = col "t" "name"; Direction = Asc; NullOrder = NullsLast }]),
            Some 5L,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Limit(
        OptimizedPlan.Sort(OptimizedPlan.Scan(_, _, _, None), _), _, _) -> ()
    | other -> failwithf "Expected Sort with un-annotated Scan, got %A" other

// ─── Lift ────────────────────────────────────────────────────────────────────

[<Fact>]
let ``lift converts Scan correctly`` () =
    let plan = LogicalPlan.Scan("users", Some "u")
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.Scan("users", Some "u", None, None) -> ()
    | other -> failwithf "Expected Scan with None hints, got %A" other

[<Fact>]
let ``lift converts DropTable correctly`` () =
    let plan = LogicalPlan.DropTable("t", false)
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.DropTable("t", false) -> ()
    | other -> failwithf "Expected DropTable, got %A" other

[<Fact>]
let ``lift converts CreateTable correctly`` () =
    let plan = LogicalPlan.CreateTable("t", true, [])
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.CreateTable("t", true, []) -> ()
    | other -> failwithf "Expected CreateTable, got %A" other

// ─── defaultPasses / optimizeWithPasses ──────────────────────────────────────

[<Fact>]
let ``defaultPasses returns exactly 5 named passes`` () =
    let passes = SqlOptimizer.defaultPasses ()
    Assert.Equal(5, List.length passes)
    let names = passes |> List.map (fun p -> p.Name)
    Assert.Contains("ConstantFolding", names)
    Assert.Contains("PredicatePushdown", names)
    Assert.Contains("ProjectionPruning", names)
    Assert.Contains("DeadCodeElimination", names)
    Assert.Contains("LimitPushdown", names)

[<Fact>]
let ``optimizeWithPasses with empty passes == lift`` () =
    let plan = LogicalPlan.Scan("t", None)
    let result = SqlOptimizer.optimizeWithPasses [] plan
    match result with
    | OptimizedPlan.Scan("t", None, None, None) -> ()
    | other -> failwithf "Expected bare Scan, got %A" other

[<Fact>]
let ``optimizeWithPasses with CF only folds constants`` () =
    let passes = SqlOptimizer.defaultPasses () |> List.filter (fun p -> p.Name = "ConstantFolding")
    let plan = filterPlan "t" (binop Add (intLit 2L) (intLit 3L))
    let result = SqlOptimizer.optimizeWithPasses passes plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Integer 5L)) -> ()
    | other -> failwithf "Expected Integer 5, got %A" other

// ─── Integration / multi-pass ────────────────────────────────────────────────

[<Fact>]
let ``Integration: constant false filter becomes EmptyResult after full pipeline`` () =
    // 1 = 2 folds to FALSE, then DCE removes the subtree.
    let plan = filterPlan "t" (binop Eq (intLit 1L) (intLit 2L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | other -> failwithf "Expected EmptyResult, got %A" other

[<Fact>]
let ``Integration: LIMIT + FALSE filter -> EmptyResult`` () =
    let plan =
        LogicalPlan.Limit(
            LogicalPlan.Filter(scanPlan "t", boolLit false),
            Some 10L,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | other -> failwithf "Expected EmptyResult, got %A" other

[<Fact>]
let ``Integration: NULL AND x -> NULL (not short-circuited to x)`` () =
    // NULL AND x = NULL (not TRUE), so the filter should be EmptyResult
    let plan = filterPlan "t" (binop And nullLit (col "t" "active"))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | OptimizedPlan.Filter(_, Expr.Literal SqlValue.Null) -> ()
    | other -> failwithf "Expected EmptyResult or NULL filter, got %A" other

[<Fact>]
let ``Integration: CROSS JOIN EmptyResult left -> EmptyResult`` () =
    let plan =
        LogicalPlan.Join(
            LogicalPlan.Filter(scanPlan "a", boolLit false),
            scanPlan "b",
            Cross,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | other -> failwithf "Expected EmptyResult, got %A" other
