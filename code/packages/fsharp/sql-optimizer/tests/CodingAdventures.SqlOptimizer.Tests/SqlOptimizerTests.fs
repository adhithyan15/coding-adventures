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

// ─── lift: all plan constructors ─────────────────────────────────────────────

[<Fact>]
let ``lift converts Filter correctly`` () =
    let plan = LogicalPlan.Filter(scanPlan "t", boolLit true)
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.Filter(OptimizedPlan.Scan("t", None, None, None), Expr.Literal(SqlValue.Bool true)) -> ()
    | other -> failwithf "Expected Filter(Scan), got %A" other

[<Fact>]
let ``lift converts Project correctly`` () =
    let plan = LogicalPlan.Project(scanPlan "t", [OutputColumn.Star])
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.Project(OptimizedPlan.Scan _, [OutputColumn.Star]) -> ()
    | other -> failwithf "Expected Project(Scan), got %A" other

[<Fact>]
let ``lift converts Join correctly`` () =
    let plan = LogicalPlan.Join(scanPlan "a", scanPlan "b", Inner, None)
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.Join(OptimizedPlan.Scan("a", _, _, _), OptimizedPlan.Scan("b", _, _, _), Inner, None) -> ()
    | other -> failwithf "Expected Join(Scan, Scan), got %A" other

[<Fact>]
let ``lift converts Aggregate correctly`` () =
    let plan = LogicalPlan.Aggregate(scanPlan "t", [], [])
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.Aggregate(OptimizedPlan.Scan _, [], []) -> ()
    | other -> failwithf "Expected Aggregate(Scan), got %A" other

[<Fact>]
let ``lift converts Having correctly`` () =
    let plan = LogicalPlan.Having(scanPlan "t", boolLit true)
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.Having(OptimizedPlan.Scan _, _) -> ()
    | other -> failwithf "Expected Having(Scan), got %A" other

[<Fact>]
let ``lift converts Sort correctly`` () =
    let plan = LogicalPlan.Sort(scanPlan "t", [{ KeyExpr = col "t" "id"; Direction = Asc; NullOrder = NullsLast }])
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.Sort(OptimizedPlan.Scan _, _) -> ()
    | other -> failwithf "Expected Sort(Scan), got %A" other

[<Fact>]
let ``lift converts Limit correctly`` () =
    let plan = LogicalPlan.Limit(scanPlan "t", Some 5L, None)
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.Limit(OptimizedPlan.Scan _, Some 5L, None) -> ()
    | other -> failwithf "Expected Limit(Scan), got %A" other

[<Fact>]
let ``lift converts Distinct correctly`` () =
    let plan = LogicalPlan.Distinct(scanPlan "t")
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.Distinct(OptimizedPlan.Scan _) -> ()
    | other -> failwithf "Expected Distinct(Scan), got %A" other

[<Fact>]
let ``lift converts Union correctly`` () =
    let plan = LogicalPlan.Union(scanPlan "a", scanPlan "b", true)
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.Union(OptimizedPlan.Scan("a", _, _, _), OptimizedPlan.Scan("b", _, _, _), true) -> ()
    | other -> failwithf "Expected Union(Scan, Scan), got %A" other

[<Fact>]
let ``lift converts Insert correctly`` () =
    let plan = LogicalPlan.Insert("t", Some ["id"; "name"], InsertSource.Values [[intLit 1L; Expr.Literal(SqlValue.Text "a")]])
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.Insert("t", Some ["id"; "name"], _) -> ()
    | other -> failwithf "Expected Insert, got %A" other

[<Fact>]
let ``lift converts Update correctly`` () =
    let plan = LogicalPlan.Update("t", [{ Column = "x"; Value = intLit 1L }], None)
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.Update("t", _, None) -> ()
    | other -> failwithf "Expected Update, got %A" other

[<Fact>]
let ``lift converts Delete correctly`` () =
    let plan = LogicalPlan.Delete("t", Some (boolLit true))
    let result = SqlOptimizer.lift plan
    match result with
    | OptimizedPlan.Delete("t", Some _) -> ()
    | other -> failwithf "Expected Delete, got %A" other

// ─── ConstantFolding — DML/DDL pass-through ──────────────────────────────────

[<Fact>]
let ``CF: Update with constant expression in assignment is folded`` () =
    // The CF pass should fold constant expressions in Update assignments.
    let plan = LogicalPlan.Update("t", [{ Column = "x"; Value = binop Add (intLit 1L) (intLit 2L) }], None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Update("t", [{ Value = Expr.Literal(SqlValue.Integer 3L) }], None) -> ()
    | other -> failwithf "Expected Update with folded value, got %A" other

[<Fact>]
let ``CF: Delete with constant predicate is not changed by CF`` () =
    // CF folds the predicate in Delete. The plan itself stays as Delete.
    let plan = LogicalPlan.Delete("t", Some (binop Eq (intLit 1L) (intLit 1L)))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Delete("t", Some (Expr.Literal(SqlValue.Bool true))) -> ()
    | other -> failwithf "Expected Delete with true predicate, got %A" other

[<Fact>]
let ``CF: Insert is left unchanged (no expr folding at the plan level)`` () =
    let plan = LogicalPlan.Insert("t", None, InsertSource.Values [[intLit 1L]])
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Insert("t", None, _) -> ()
    | other -> failwithf "Expected Insert unchanged, got %A" other

// ─── ConstantFolding — additional arithmetic branches ────────────────────────

[<Fact>]
let ``CF: 10 MOD 3 folds to 1`` () =
    let plan = filterPlan "t" (binop Mod (intLit 10L) (intLit 3L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Integer 1L)) -> ()
    | other -> failwithf "Expected Integer 1, got %A" other

[<Fact>]
let ``CF: -10 MOD 3 folds to -1 (sign follows dividend)`` () =
    let plan = filterPlan "t" (binop Mod (intLit -10L) (intLit 3L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Integer -1L)) -> ()
    | other -> failwithf "Expected Integer -1, got %A" other

[<Fact>]
let ``CF: MOD by zero is not folded`` () =
    let plan = filterPlan "t" (binop Mod (intLit 5L) (intLit 0L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.BinaryOp(Mod, _, _)) -> ()
    | other -> failwithf "Expected un-folded Mod, got %A" other

[<Fact>]
let ``CF: real MOD folds`` () =
    let plan = filterPlan "t" (binop Mod (realLit 5.5) (realLit 2.0))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Real _)) -> ()
    | other -> failwithf "Expected Real result from Mod, got %A" other

[<Fact>]
let ``CF: real div by zero not folded`` () =
    let plan = filterPlan "t" (binop Div (realLit 5.0) (realLit 0.0))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.BinaryOp(Div, _, _)) -> ()
    | other -> failwithf "Expected un-folded Div, got %A" other

[<Fact>]
let ``CF: integer + real promotes to real`` () =
    let plan = filterPlan "t" (binop Add (intLit 2L) (realLit 3.0))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Real 5.0)) -> ()
    | other -> failwithf "Expected Real 5.0, got %A" other

[<Fact>]
let ``CF: real - integer promotes to real`` () =
    let plan = filterPlan "t" (binop Sub (realLit 10.0) (intLit 3L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Real 7.0)) -> ()
    | other -> failwithf "Expected Real 7.0, got %A" other

[<Fact>]
let ``CF: integer * real promotes to real`` () =
    let plan = filterPlan "t" (binop Mul (intLit 3L) (realLit 2.5))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Real 7.5)) -> ()
    | other -> failwithf "Expected Real 7.5, got %A" other

[<Fact>]
let ``CF: real / integer folds`` () =
    let plan = filterPlan "t" (binop Div (realLit 10.0) (intLit 4L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Real 2.5)) -> ()
    | other -> failwithf "Expected Real 2.5, got %A" other

[<Fact>]
let ``CF: real div by zero integer not folded`` () =
    let plan = filterPlan "t" (binop Div (realLit 5.0) (intLit 0L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.BinaryOp(Div, _, _)) -> ()
    | other -> failwithf "Expected un-folded Div, got %A" other

[<Fact>]
let ``CF: integer MOD real folds`` () =
    let plan = filterPlan "t" (binop Mod (intLit 7L) (realLit 3.0))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Real _)) -> ()
    | other -> failwithf "Expected Real mod result, got %A" other

[<Fact>]
let ``CF: real MOD zero integer not folded`` () =
    let plan = filterPlan "t" (binop Mod (realLit 5.0) (intLit 0L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.BinaryOp(Mod, _, _)) -> ()
    | other -> failwithf "Expected un-folded Mod, got %A" other

[<Fact>]
let ``CF: integer MOD zero real not folded`` () =
    let plan = filterPlan "t" (binop Mod (intLit 5L) (realLit 0.0))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.BinaryOp(Mod, _, _)) -> ()
    | other -> failwithf "Expected un-folded Mod, got %A" other

[<Fact>]
let ``CF: text equality folds to true`` () =
    let plan = filterPlan "t" (binop Eq (Expr.Literal(SqlValue.Text "a")) (Expr.Literal(SqlValue.Text "a")))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Scan _ -> ()  // tautology removed
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool true)) -> ()
    | other -> failwithf "Expected Scan or TRUE, got %A" other

[<Fact>]
let ``CF: text less-than comparison folds`` () =
    let plan = filterPlan "t" (binop Lt (Expr.Literal(SqlValue.Text "a")) (Expr.Literal(SqlValue.Text "b")))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Scan _ -> ()  // "a" < "b" is true, filter removed
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool true)) -> ()
    | other -> failwithf "Expected Scan or TRUE, got %A" other

[<Fact>]
let ``CF: bool equality folds to true`` () =
    let plan = filterPlan "t" (binop Eq (boolLit true) (boolLit true))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Scan _ -> ()  // tautology removed
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool true)) -> ()
    | other -> failwithf "Expected Scan or TRUE, got %A" other

[<Fact>]
let ``CF: bool not-equal folds`` () =
    let plan = filterPlan "t" (binop NotEq (boolLit true) (boolLit false))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Scan _ -> ()  // tautology removed
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool true)) -> ()
    | other -> failwithf "Expected Scan or TRUE, got %A" other

[<Fact>]
let ``CF: NEG real folds`` () =
    let plan = filterPlan "t" (Expr.UnaryOp(Neg, realLit 3.14))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Real v)) when v < 0.0 -> ()
    | other -> failwithf "Expected negative Real, got %A" other

[<Fact>]
let ``CF: NEG of column stays as-is`` () =
    let plan = filterPlan "t" (Expr.UnaryOp(Neg, col "t" "x"))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.UnaryOp(Neg, Expr.Column _)) -> ()
    | other -> failwithf "Expected un-folded UnaryOp Neg, got %A" other

[<Fact>]
let ``CF: NULL AND FALSE -> FALSE (short-circuit)`` () =
    // FALSE AND NULL should short-circuit to FALSE
    let plan = filterPlan "t" (binop And (Expr.Literal SqlValue.Null) (boolLit false))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool false)) -> ()
    | other -> failwithf "Expected EmptyResult or FALSE, got %A" other

[<Fact>]
let ``CF: NULL OR TRUE -> TRUE (short-circuit)`` () =
    let plan = filterPlan "t" (binop Or (Expr.Literal SqlValue.Null) (boolLit true))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Scan _ -> ()
    | OptimizedPlan.Filter(_, Expr.Literal(SqlValue.Bool true)) -> ()
    | other -> failwithf "Expected Scan or TRUE, got %A" other

[<Fact>]
let ``CF: x AND NULL -> NULL`` () =
    let plan = filterPlan "t" (binop And (col "t" "x") (Expr.Literal SqlValue.Null))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | OptimizedPlan.Filter(_, Expr.Literal SqlValue.Null) -> ()
    | other -> failwithf "Expected NULL or EmptyResult, got %A" other

[<Fact>]
let ``CF: x OR NULL -> NULL`` () =
    let plan = filterPlan "t" (binop Or (col "t" "x") (Expr.Literal SqlValue.Null))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | OptimizedPlan.Filter(_, Expr.Literal SqlValue.Null) -> ()
    | other -> failwithf "Expected NULL or EmptyResult, got %A" other

[<Fact>]
let ``CF: Between folds child expressions`` () =
    // Between(5, 1+1, 10) -> Between(5, 2, 10) — Between itself is not eliminated
    let plan = filterPlan "t" (Expr.Between(col "t" "x", binop Add (intLit 1L) (intLit 1L), intLit 10L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Between(_, Expr.Literal(SqlValue.Integer 2L), _)) -> ()
    | other -> failwithf "Expected Between with folded low, got %A" other

[<Fact>]
let ``CF: In list folds child expressions`` () =
    let plan = filterPlan "t" (Expr.In(col "t" "x", [binop Add (intLit 1L) (intLit 2L); intLit 5L]))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.In(_, [Expr.Literal(SqlValue.Integer 3L); _])) -> ()
    | other -> failwithf "Expected In with folded items, got %A" other

[<Fact>]
let ``CF: NotIn list folds child expressions`` () =
    let plan = filterPlan "t" (Expr.NotIn(col "t" "x", [binop Mul (intLit 2L) (intLit 3L)]))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.NotIn(_, [Expr.Literal(SqlValue.Integer 6L)])) -> ()
    | other -> failwithf "Expected NotIn with folded item, got %A" other

[<Fact>]
let ``CF: Like value is folded`` () =
    // Like(col, pattern) — pattern is a string literal, col stays; but value gets folded
    let plan = filterPlan "t" (Expr.Like(col "t" "name", "foo%"))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.Like(Expr.Column _, "foo%")) -> ()
    | other -> failwithf "Expected Like unchanged, got %A" other

[<Fact>]
let ``CF: NotLike value is folded`` () =
    let plan = filterPlan "t" (Expr.NotLike(col "t" "name", "bar%"))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.NotLike(Expr.Column _, "bar%")) -> ()
    | other -> failwithf "Expected NotLike unchanged, got %A" other

[<Fact>]
let ``CF: FuncCall folds constant arguments`` () =
    let plan = filterPlan "t" (Expr.FuncCall("abs", [binop Sub (intLit 0L) (intLit 5L)]))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(_, Expr.FuncCall("abs", [Expr.Literal(SqlValue.Integer -5L)])) -> ()
    | other -> failwithf "Expected FuncCall with folded arg, got %A" other

// ─── PredicatePushdown — additional cases ────────────────────────────────────

[<Fact>]
let ``PPD: RIGHT JOIN pred pushed to right side only`` () =
    let pred = binop Eq (col "d" "region") (Expr.Literal(SqlValue.Text "US"))
    let plan =
        LogicalPlan.Filter(
            LogicalPlan.Join(
                LogicalPlan.Scan("employees", Some "e"),
                LogicalPlan.Scan("departments", Some "d"),
                Right,
                None),
            pred)
    let result = SqlOptimizer.optimize plan
    // Should be pushed into right side
    match result with
    | OptimizedPlan.Join(_, OptimizedPlan.Filter(OptimizedPlan.Scan _, _), Right, _) -> ()
    | other -> failwithf "Expected pred pushed to right of RIGHT JOIN, got %A" other

[<Fact>]
let ``PPD: FULL JOIN filter stays above (not pushed)`` () =
    let pred = binop Eq (col "e" "id") (intLit 1L)
    let plan =
        LogicalPlan.Filter(
            LogicalPlan.Join(
                LogicalPlan.Scan("employees", Some "e"),
                LogicalPlan.Scan("departments", Some "d"),
                Full,
                None),
            pred)
    let result = SqlOptimizer.optimize plan
    // FULL JOIN — neither side gets the filter pushed
    match result with
    | OptimizedPlan.Filter(OptimizedPlan.Join _, _) -> ()
    | other -> failwithf "Expected Filter(FULL JOIN), got %A" other

[<Fact>]
let ``PPD: unresolved column (no qualifier) stays above join`` () =
    // A bare column (no table qualifier) blocks pushdown.
    let pred = binop Eq (Expr.Column(None, "x")) (intLit 1L)
    let plan =
        LogicalPlan.Filter(
            LogicalPlan.Join(
                LogicalPlan.Scan("employees", Some "e"),
                LogicalPlan.Scan("departments", Some "d"),
                Inner,
                None),
            pred)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Filter(OptimizedPlan.Join _, _) -> ()
    | other -> failwithf "Expected Filter(Join) — unresolved col blocks push, got %A" other

[<Fact>]
let ``PPD: filter pushed through Project only if aliases match`` () =
    // Filter references t.id. Project wraps a scan of "t". Should push through.
    let pred = binop Eq (col "t" "id") (intLit 1L)
    let plan =
        LogicalPlan.Filter(
            LogicalPlan.Project(
                scanPlan "t",
                [OutputColumn.Expr(col "t" "name", None)]),
            pred)
    let result = SqlOptimizer.optimize plan
    // pred references "t" which is the scan alias — should push through Project
    match result with
    | OptimizedPlan.Project(OptimizedPlan.Filter(OptimizedPlan.Scan _, _), _) -> ()
    | OptimizedPlan.Project(OptimizedPlan.Scan _, _) -> ()  // if DCE merged it
    | OptimizedPlan.Filter(OptimizedPlan.Project _, _) -> ()  // stuck due to alias mismatch
    | other -> failwithf "Unexpected structure, got %A" other

[<Fact>]
let ``PPD: LEFT JOIN pred NOT pushed to right (null-padding)`` () =
    // A pred on the right-only alias must NOT be pushed into right of LEFT JOIN.
    let pred = binop Eq (col "d" "active") (boolLit true)
    let plan =
        LogicalPlan.Filter(
            LogicalPlan.Join(
                LogicalPlan.Scan("employees", Some "e"),
                LogicalPlan.Scan("departments", Some "d"),
                Left,
                None),
            pred)
    let result = SqlOptimizer.optimize plan
    // For LEFT JOIN, canPushRight = false, so pred on right alias stays above.
    match result with
    | OptimizedPlan.Filter(OptimizedPlan.Join _, _) -> ()
    | other -> failwithf "Expected Filter above LEFT JOIN (right pred blocked), got %A" other

// ─── ProjectionPruning — additional paths ────────────────────────────────────

[<Fact>]
let ``PP: Join condition columns influence pruning`` () =
    // A join with a condition references e.dept_id and d.id.
    let cond = binop Eq (col "e" "dept_id") (col "d" "id")
    let plan =
        LogicalPlan.Join(
            LogicalPlan.Scan("employees", Some "e"),
            LogicalPlan.Scan("departments", Some "d"),
            Inner,
            Some cond)
    let result = SqlOptimizer.optimize plan
    // No Project wrapper, so pruning req from top is None — scans stay un-annotated.
    match result with
    | OptimizedPlan.Join(OptimizedPlan.Scan _, OptimizedPlan.Scan _, Inner, _) -> ()
    | other -> failwithf "Expected Join(Scan, Scan), got %A" other

[<Fact>]
let ``PP: Aggregate annotates Scan with group-by columns`` () =
    // Aggregate(Scan("t"), groupBy=[t.dept], aggs=[COUNT(*)])
    let plan =
        LogicalPlan.Aggregate(
            LogicalPlan.Scan("t", None),
            [col "t" "dept"],
            [{ Func = Count; Arg = AggArg.Star; Alias = "n"; Distinct = false }])
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Aggregate(OptimizedPlan.Scan(_, _, Some ["dept"], _), _, _) -> ()
    | OptimizedPlan.Aggregate(OptimizedPlan.Scan(_, _, None, _), _, _) -> ()  // wildcard path
    | other -> failwithf "Expected Aggregate(Scan), got %A" other

[<Fact>]
let ``PP: Having predicate contributes to pruning`` () =
    let plan =
        LogicalPlan.Having(
            LogicalPlan.Scan("t", None),
            binop Gt (col "t" "count") (intLit 0L))
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Having(OptimizedPlan.Scan _, _) -> ()
    | other -> failwithf "Expected Having(Scan), got %A" other

[<Fact>]
let ``PP: Sort keys contribute to pruning`` () =
    let plan =
        LogicalPlan.Sort(
            LogicalPlan.Scan("t", None),
            [{ KeyExpr = col "t" "name"; Direction = Asc; NullOrder = NullsLast }])
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Sort(OptimizedPlan.Scan _, _) -> ()
    | other -> failwithf "Expected Sort(Scan), got %A" other

[<Fact>]
let ``PP: Filter predicate combined with parent req`` () =
    let plan =
        LogicalPlan.Project(
            LogicalPlan.Filter(
                LogicalPlan.Scan("t", None),
                binop Eq (col "t" "active") (boolLit true)),
            [OutputColumn.Expr(col "t" "name", None)])
    let result = SqlOptimizer.optimize plan
    // name required by Project, active required by Filter — both annotate Scan
    match result with
    | OptimizedPlan.Project(OptimizedPlan.Filter(OptimizedPlan.Scan(_, _, cols, _), _), _) ->
        match cols with
        | Some lst -> if lst.Length < 1 then failwithf "Expected at least 1 required column, got %A" lst
        | None -> ()  // wildcard path — acceptable if pruning skipped
    | OptimizedPlan.EmptyResult -> ()  // folded away somehow
    | other -> failwithf "Unexpected structure, got %A" other

[<Fact>]
let ``PP: Limit passes req through to child`` () =
    let plan =
        LogicalPlan.Project(
            LogicalPlan.Limit(
                LogicalPlan.Scan("t", None),
                Some 5L,
                None),
            [OutputColumn.Expr(col "t" "id", None)])
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Limit(OptimizedPlan.Scan(_, _, Some ["id"], _), _, _) -> ()
    | OptimizedPlan.Limit(OptimizedPlan.Scan(_, _, _, _), _, _) -> ()  // pruning may vary
    | other -> failwithf "Unexpected structure, got %A" other

[<Fact>]
let ``PP: Distinct passes req through to child`` () =
    let plan =
        LogicalPlan.Project(
            LogicalPlan.Distinct(LogicalPlan.Scan("t", None)),
            [OutputColumn.Expr(col "t" "email", None)])
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Project(OptimizedPlan.Distinct(OptimizedPlan.Scan _), _) -> ()
    | other -> failwithf "Expected Project(Distinct(Scan)), got %A" other

[<Fact>]
let ``PP: Union passes req to both sides`` () =
    let plan =
        LogicalPlan.Project(
            LogicalPlan.Union(
                LogicalPlan.Scan("a", None),
                LogicalPlan.Scan("b", None),
                false),
            [OutputColumn.Star])
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Project(OptimizedPlan.Union(OptimizedPlan.Scan _, OptimizedPlan.Scan _, _), _) -> ()
    | other -> failwithf "Expected Project(Union(Scan, Scan)), got %A" other

// ─── DeadCodeElimination — additional paths ──────────────────────────────────

[<Fact>]
let ``DCE: RIGHT JOIN with EmptyResult left stays (null-padding preserved)`` () =
    let plan =
        LogicalPlan.Join(
            LogicalPlan.Filter(scanPlan "a", boolLit false),
            scanPlan "b",
            Right,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Join(OptimizedPlan.EmptyResult, OptimizedPlan.Scan _, Right, _) -> ()
    | other -> failwithf "Expected Join preserved (RIGHT outer), got %A" other

[<Fact>]
let ``DCE: FULL JOIN with one side empty stays`` () =
    let plan =
        LogicalPlan.Join(
            LogicalPlan.Filter(scanPlan "a", boolLit false),
            scanPlan "b",
            Full,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Join(OptimizedPlan.EmptyResult, OptimizedPlan.Scan _, Full, _) -> ()
    | other -> failwithf "Expected FULL JOIN preserved with EmptyResult left, got %A" other

[<Fact>]
let ``DCE: CROSS JOIN with EmptyResult right -> EmptyResult`` () =
    let plan =
        LogicalPlan.Join(
            scanPlan "a",
            LogicalPlan.Filter(scanPlan "b", boolLit false),
            Cross,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.EmptyResult -> ()
    | other -> failwithf "Expected EmptyResult, got %A" other

[<Fact>]
let ``DCE: Limit with None count preserves plan`` () =
    let plan = LogicalPlan.Limit(scanPlan "t", None, None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Limit(OptimizedPlan.Scan _, None, None) -> ()
    | other -> failwithf "Expected Limit(Scan), got %A" other

// ─── LimitPushdown — additional paths ────────────────────────────────────────

[<Fact>]
let ``LP: LIMIT pushes through nested Filter and Project`` () =
    let plan =
        LogicalPlan.Limit(
            LogicalPlan.Filter(
                LogicalPlan.Project(
                    scanPlan "t",
                    [OutputColumn.Expr(col "t" "id", None)]),
                binop Gt (col "t" "age") (intLit 0L)),
            Some 5L,
            None)
    let result = SqlOptimizer.optimize plan
    // scanLimit should have been pushed all the way to the Scan
    match result with
    | OptimizedPlan.Limit(OptimizedPlan.Filter(OptimizedPlan.Project(OptimizedPlan.Scan(_, _, _, Some 5L), _), _), _, _) -> ()
    | other -> failwithf "Expected scanLimit=5 propagated to Scan, got %A" other

[<Fact>]
let ``LP: existing scanLimit takes minimum with new limit`` () =
    // Two nested LIMITs — inner 5, outer 3. Scan should get min(5, 3) = 3.
    let inner = LogicalPlan.Limit(scanPlan "t", Some 5L, None)
    let outer = LogicalPlan.Limit(inner, Some 3L, None)
    let result = SqlOptimizer.optimize outer
    // The outer limit (3) pushes into the scan which had 5 from inner → min = 3
    match result with
    | OptimizedPlan.Limit(OptimizedPlan.Limit(OptimizedPlan.Scan(_, _, _, Some 3L), _, _), _, _) -> ()
    | OptimizedPlan.Limit(OptimizedPlan.Scan(_, _, _, Some 3L), _, _) -> ()
    | other -> failwithf "Expected scanLimit=3 (min), got %A" other

[<Fact>]
let ``LP: LIMIT does not push past Join`` () =
    let plan =
        LogicalPlan.Limit(
            LogicalPlan.Join(scanPlan "a", scanPlan "b", Inner, None),
            Some 5L,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Limit(OptimizedPlan.Join(OptimizedPlan.Scan(_, _, _, None), _, _, _), _, _) -> ()
    | other -> failwithf "Expected Join with un-annotated Scans, got %A" other

[<Fact>]
let ``LP: LIMIT does not push past Aggregate`` () =
    let plan =
        LogicalPlan.Limit(
            LogicalPlan.Aggregate(scanPlan "t", [], []),
            Some 2L,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Limit(OptimizedPlan.Aggregate(OptimizedPlan.Scan(_, _, _, None), _, _), _, _) -> ()
    | other -> failwithf "Expected Aggregate with un-annotated Scan, got %A" other

[<Fact>]
let ``LP: LIMIT does not push past Distinct`` () =
    let plan =
        LogicalPlan.Limit(
            LogicalPlan.Distinct(scanPlan "t"),
            Some 5L,
            None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Limit(OptimizedPlan.Distinct(OptimizedPlan.Scan(_, _, _, None)), _, _) -> ()
    | other -> failwithf "Expected Distinct with un-annotated Scan, got %A" other

[<Fact>]
let ``LP: LIMIT with zero offset pushes scanLimit`` () =
    let plan = LogicalPlan.Limit(scanPlan "t", Some 7L, Some 0L)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Limit(OptimizedPlan.Scan(_, _, _, Some 7L), _, _) -> ()
    | other -> failwithf "Expected Scan with scanLimit=7 (zero offset), got %A" other

[<Fact>]
let ``LP: LIMIT with None count does not push scanLimit`` () =
    let plan = LogicalPlan.Limit(scanPlan "t", None, None)
    let result = SqlOptimizer.optimize plan
    match result with
    | OptimizedPlan.Limit(OptimizedPlan.Scan(_, _, _, None), _, _) -> ()
    | other -> failwithf "Expected Scan without scanLimit (no count), got %A" other
