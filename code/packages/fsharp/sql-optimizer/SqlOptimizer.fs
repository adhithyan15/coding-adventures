// SqlOptimizer.fs — logical query optimizer for the Mini-SQLite pipeline.
//
// This module applies a sequence of rewriting passes over a LogicalPlan tree,
// producing an OptimizedPlan that is cheaper to execute. No I/O, no state —
// every pass is a pure function over the plan tree.
//
// Pipeline (applied in order):
//   1. ConstantFolding   — evaluate constant sub-expressions at plan time
//   2. PredicatePushdown — move filters closer to scans
//   3. ProjectionPruning — annotate scans with the columns they actually need
//   4. DeadCodeElimination — remove provably-empty subtrees
//   5. LimitPushdown     — attach scan_limit hints where safe
//
// The OptimizedPlan mirrors LogicalPlan but adds:
//   - Scan.requiredColumns  — column subset hint for columnar backends
//   - Scan.scanLimit        — row-count hint for early termination
//   - EmptyResult           — sentinel for provably-empty subtrees

namespace CodingAdventures.SqlOptimizer.FSharp

open CodingAdventures.SqlPlanner.FSharp

// ── OptimizedPlan ────────────────────────────────────────────────────────────
// Mirrors every case of LogicalPlan. The two differences from LogicalPlan:
//   * Scan carries optional requiredColumns and scanLimit hints.
//   * EmptyResult is a new terminal case (no fields — just signals "zero rows").
//
// All child pointers use OptimizedPlan recursively so passes can compose.

[<RequireQualifiedAccess>]
type OptimizedPlan =
    // ── Scan with backend hints ───────────────────────────────────────────────
    // requiredColumns: None = "all columns", Some ["id"; "name"] = subset hint.
    // scanLimit:       None = "no limit", Some 10L = "read at most 10 rows".
    | Scan        of table: string * alias: string option * requiredColumns: string list option * scanLimit: int64 option
    // ── Transparent pass-through nodes ───────────────────────────────────────
    | Filter      of input: OptimizedPlan * predicate: Expr
    | Project     of input: OptimizedPlan * columns: OutputColumn list
    | Join        of left: OptimizedPlan * right: OptimizedPlan * kind: JoinKind * condition: Expr option
    | Aggregate   of input: OptimizedPlan * groupBy: Expr list * aggregates: AggregateItem list
    | Having      of input: OptimizedPlan * predicate: Expr
    | Sort        of input: OptimizedPlan * keys: SortKey list
    | Limit       of input: OptimizedPlan * count: int64 option * offset: int64 option
    | Distinct    of input: OptimizedPlan
    | Union       of left: OptimizedPlan * right: OptimizedPlan * all: bool
    // ── DML / DDL — no optimization, carried through unchanged ────────────────
    | Insert      of table: string * columns: string list option * source: InsertSource
    | Update      of table: string * assignments: Assignment list * predicate: Expr option
    | Delete      of table: string * predicate: Expr option
    | CreateTable of table: string * ifNotExists: bool * columns: ColumnDef list
    | DropTable   of table: string * ifExists: bool
    // ── Terminal sentinel — proven to produce zero rows ───────────────────────
    | EmptyResult

// ── Pass record ──────────────────────────────────────────────────────────────
// A named transformation. The pipeline is just a list of Passes applied in
// order, so adding or removing a pass only touches defaultPasses().

type Pass = { Name: string; Apply: OptimizedPlan -> OptimizedPlan }

// ── SqlOptimizer module ──────────────────────────────────────────────────────

module SqlOptimizer =

    // ── lift: LogicalPlan → OptimizedPlan ────────────────────────────────────
    // Recursively converts the planner's output into the optimizer's input.
    // Scans start with None/None hints; passes fill them in.

    let rec lift (plan: LogicalPlan) : OptimizedPlan =
        match plan with
        | LogicalPlan.Scan(t, a)               -> OptimizedPlan.Scan(t, a, None, None)
        | LogicalPlan.Filter(inner, pred)       -> OptimizedPlan.Filter(lift inner, pred)
        | LogicalPlan.Project(inner, cols)      -> OptimizedPlan.Project(lift inner, cols)
        | LogicalPlan.Join(l, r, k, cond)       -> OptimizedPlan.Join(lift l, lift r, k, cond)
        | LogicalPlan.Aggregate(inner, gb, agg) -> OptimizedPlan.Aggregate(lift inner, gb, agg)
        | LogicalPlan.Having(inner, pred)       -> OptimizedPlan.Having(lift inner, pred)
        | LogicalPlan.Sort(inner, keys)         -> OptimizedPlan.Sort(lift inner, keys)
        | LogicalPlan.Limit(inner, c, o)        -> OptimizedPlan.Limit(lift inner, c, o)
        | LogicalPlan.Distinct(inner)           -> OptimizedPlan.Distinct(lift inner)
        | LogicalPlan.Union(l, r, all)          -> OptimizedPlan.Union(lift l, lift r, all)
        | LogicalPlan.Insert(t, cols, src)      -> OptimizedPlan.Insert(t, cols, src)
        | LogicalPlan.Update(t, asgn, pred)     -> OptimizedPlan.Update(t, asgn, pred)
        | LogicalPlan.Delete(t, pred)           -> OptimizedPlan.Delete(t, pred)
        | LogicalPlan.CreateTable(t, ine, cols) -> OptimizedPlan.CreateTable(t, ine, cols)
        | LogicalPlan.DropTable(t, ie)          -> OptimizedPlan.DropTable(t, ie)

    // =========================================================================
    // Pass 1 — ConstantFolding
    // =========================================================================
    // Evaluate constant sub-expressions at plan time so the VM doesn't pay
    // the cost of re-evaluating them for every row. Bottom-up: children fold
    // before parents, so 1 + (2 + 3) → 1 + 5 → 6 in a single pass.
    //
    // NULL semantics: SQL three-valued logic means most ops propagate NULL.
    // AND/OR are exceptions (short-circuit): FALSE AND NULL = FALSE,
    // TRUE OR NULL = TRUE.

    module private ConstantFolding =

        // Compare two SqlValues to produce a Bool literal.
        // Returns None if comparison is undefined (e.g. mixed types).
        let private compareValues (lv: SqlValue) (rv: SqlValue) (op: BinaryOperator) : SqlValue option =
            match lv, rv with
            | SqlValue.Integer a, SqlValue.Integer b ->
                match op with
                | Eq    -> Some (SqlValue.Bool (a = b))
                | NotEq -> Some (SqlValue.Bool (a <> b))
                | Lt    -> Some (SqlValue.Bool (a < b))
                | Lte   -> Some (SqlValue.Bool (a <= b))
                | Gt    -> Some (SqlValue.Bool (a > b))
                | Gte   -> Some (SqlValue.Bool (a >= b))
                | _     -> None
            | SqlValue.Real a, SqlValue.Real b ->
                match op with
                | Eq    -> Some (SqlValue.Bool (a = b))
                | NotEq -> Some (SqlValue.Bool (a <> b))
                | Lt    -> Some (SqlValue.Bool (a < b))
                | Lte   -> Some (SqlValue.Bool (a <= b))
                | Gt    -> Some (SqlValue.Bool (a > b))
                | Gte   -> Some (SqlValue.Bool (a >= b))
                | _     -> None
            | SqlValue.Integer a, SqlValue.Real b ->
                let af = float a
                match op with
                | Eq    -> Some (SqlValue.Bool (af = b))
                | NotEq -> Some (SqlValue.Bool (af <> b))
                | Lt    -> Some (SqlValue.Bool (af < b))
                | Lte   -> Some (SqlValue.Bool (af <= b))
                | Gt    -> Some (SqlValue.Bool (af > b))
                | Gte   -> Some (SqlValue.Bool (af >= b))
                | _     -> None
            | SqlValue.Real a, SqlValue.Integer b ->
                let bf = float b
                match op with
                | Eq    -> Some (SqlValue.Bool (a = bf))
                | NotEq -> Some (SqlValue.Bool (a <> bf))
                | Lt    -> Some (SqlValue.Bool (a < bf))
                | Lte   -> Some (SqlValue.Bool (a <= bf))
                | Gt    -> Some (SqlValue.Bool (a > bf))
                | Gte   -> Some (SqlValue.Bool (a >= bf))
                | _     -> None
            | SqlValue.Text a, SqlValue.Text b ->
                match op with
                | Eq    -> Some (SqlValue.Bool (a = b))
                | NotEq -> Some (SqlValue.Bool (a <> b))
                | Lt    -> Some (SqlValue.Bool (a < b))
                | Lte   -> Some (SqlValue.Bool (a <= b))
                | Gt    -> Some (SqlValue.Bool (a > b))
                | Gte   -> Some (SqlValue.Bool (a >= b))
                | _     -> None
            | SqlValue.Bool a, SqlValue.Bool b ->
                match op with
                | Eq    -> Some (SqlValue.Bool (a = b))
                | NotEq -> Some (SqlValue.Bool (a <> b))
                | _     -> None
            | _ -> None

        // Arithmetic on literals (integers and reals). Division by zero stays as-is.
        let private foldArith (lv: SqlValue) (rv: SqlValue) (op: BinaryOperator) : SqlValue option =
            match lv, rv with
            | SqlValue.Integer a, SqlValue.Integer b ->
                match op with
                | Add -> Some (SqlValue.Integer (a + b))
                | Sub -> Some (SqlValue.Integer (a - b))
                | Mul -> Some (SqlValue.Integer (a * b))
                | Div ->
                    // SQLite truncates toward zero: -7 / 2 = -3 (not -4).
                    if b = 0L then None
                    else
                        let q = abs a / abs b
                        Some (SqlValue.Integer (if (a < 0L) <> (b < 0L) then -q else q))
                | Mod ->
                    if b = 0L then None
                    else
                        let mag = abs a % abs b
                        Some (SqlValue.Integer (if a < 0L then -mag else mag))
                | _ -> None
            | SqlValue.Real a, SqlValue.Real b ->
                match op with
                | Add -> Some (SqlValue.Real (a + b))
                | Sub -> Some (SqlValue.Real (a - b))
                | Mul -> Some (SqlValue.Real (a * b))
                | Div ->
                    if b = 0.0 then None
                    else Some (SqlValue.Real (a / b))
                | Mod ->
                    if b = 0.0 then None
                    else Some (SqlValue.Real (a % b))
                | _ -> None
            | SqlValue.Integer a, SqlValue.Real b ->
                let af = float a
                match op with
                | Add -> Some (SqlValue.Real (af + b))
                | Sub -> Some (SqlValue.Real (af - b))
                | Mul -> Some (SqlValue.Real (af * b))
                | Div ->
                    if b = 0.0 then None
                    else Some (SqlValue.Real (af / b))
                | Mod ->
                    if b = 0.0 then None
                    else Some (SqlValue.Real (af % b))
                | _ -> None
            | SqlValue.Real a, SqlValue.Integer b ->
                let bf = float b
                match op with
                | Add -> Some (SqlValue.Real (a + bf))
                | Sub -> Some (SqlValue.Real (a - bf))
                | Mul -> Some (SqlValue.Real (a * bf))
                | Div ->
                    if b = 0L then None
                    else Some (SqlValue.Real (a / bf))
                | Mod ->
                    if b = 0L then None
                    else Some (SqlValue.Real (a % bf))
                | _ -> None
            | _ -> None

        // Fold a binary expression where both children have already been folded.
        let rec private foldBinary (op: BinaryOperator) (left: Expr) (right: Expr) : Expr =
            // AND short-circuit rules (three-valued logic):
            // FALSE AND anything  = FALSE
            // anything AND FALSE  = FALSE
            // TRUE AND x          = x
            // x AND TRUE          = x
            // NULL AND NULL       = NULL
            match op with
            | And ->
                match left, right with
                | Expr.Literal(SqlValue.Bool false), _  -> Expr.Literal(SqlValue.Bool false)
                | _, Expr.Literal(SqlValue.Bool false)  -> Expr.Literal(SqlValue.Bool false)
                | Expr.Literal(SqlValue.Bool true), x   -> x
                | x, Expr.Literal(SqlValue.Bool true)   -> x
                | Expr.Literal SqlValue.Null, _
                | _, Expr.Literal SqlValue.Null          -> Expr.Literal SqlValue.Null
                | _ -> Expr.BinaryOp(And, left, right)
            // OR short-circuit rules:
            // TRUE OR anything    = TRUE
            // anything OR TRUE    = TRUE
            // FALSE OR x          = x
            // x OR FALSE          = x
            | Or ->
                match left, right with
                | Expr.Literal(SqlValue.Bool true), _   -> Expr.Literal(SqlValue.Bool true)
                | _, Expr.Literal(SqlValue.Bool true)   -> Expr.Literal(SqlValue.Bool true)
                | Expr.Literal(SqlValue.Bool false), x  -> x
                | x, Expr.Literal(SqlValue.Bool false)  -> x
                | Expr.Literal SqlValue.Null, _
                | _, Expr.Literal SqlValue.Null          -> Expr.Literal SqlValue.Null
                | _ -> Expr.BinaryOp(Or, left, right)
            | _ ->
                // For non-boolean ops: NULL propagates.
                match left, right with
                | Expr.Literal SqlValue.Null, _ -> Expr.Literal SqlValue.Null
                | _, Expr.Literal SqlValue.Null -> Expr.Literal SqlValue.Null
                | Expr.Literal lv, Expr.Literal rv ->
                    // Comparison operators
                    match op with
                    | Eq | NotEq | Lt | Lte | Gt | Gte ->
                        match compareValues lv rv op with
                        | Some v -> Expr.Literal v
                        | None   -> Expr.BinaryOp(op, left, right)
                    // Arithmetic operators
                    | Add | Sub | Mul | Div | Mod ->
                        match foldArith lv rv op with
                        | Some v -> Expr.Literal v
                        | None   -> Expr.BinaryOp(op, left, right)  // e.g. div by zero
                    | _ -> Expr.BinaryOp(op, left, right)
                | _ -> Expr.BinaryOp(op, left, right)

        // Bottom-up expression fold. Non-constant sub-trees are returned as-is.
        let rec foldExpr (e: Expr) : Expr =
            match e with
            | Expr.Literal _ | Expr.Column _ | Expr.Wildcard | Expr.AggExpr _ -> e
            | Expr.BinaryOp(op, l, r)   -> foldBinary op (foldExpr l) (foldExpr r)
            | Expr.UnaryOp(op, operand) ->
                let fe = foldExpr operand
                match fe with
                | Expr.Literal SqlValue.Null ->
                    Expr.Literal SqlValue.Null
                | Expr.Literal(SqlValue.Bool b) when op = Not ->
                    Expr.Literal(SqlValue.Bool (not b))
                | Expr.Literal(SqlValue.Integer n) when op = Neg ->
                    Expr.Literal(SqlValue.Integer -n)
                | Expr.Literal(SqlValue.Real r) when op = Neg ->
                    Expr.Literal(SqlValue.Real -r)
                | _ -> Expr.UnaryOp(op, fe)
            | Expr.IsNull inner ->
                match foldExpr inner with
                | Expr.Literal SqlValue.Null -> Expr.Literal(SqlValue.Bool true)
                | Expr.Literal _             -> Expr.Literal(SqlValue.Bool false)
                | fe                         -> Expr.IsNull fe
            | Expr.IsNotNull inner ->
                match foldExpr inner with
                | Expr.Literal SqlValue.Null -> Expr.Literal(SqlValue.Bool false)
                | Expr.Literal _             -> Expr.Literal(SqlValue.Bool true)
                | fe                         -> Expr.IsNotNull fe
            | Expr.Between(value, low, high) ->
                Expr.Between(foldExpr value, foldExpr low, foldExpr high)
            | Expr.In(value, items) ->
                Expr.In(foldExpr value, List.map foldExpr items)
            | Expr.NotIn(value, items) ->
                Expr.NotIn(foldExpr value, List.map foldExpr items)
            | Expr.Like(value, pattern) ->
                Expr.Like(foldExpr value, pattern)
            | Expr.NotLike(value, pattern) ->
                Expr.NotLike(foldExpr value, pattern)
            | Expr.FuncCall(name, args) ->
                Expr.FuncCall(name, List.map foldExpr args)

        // Fold all expressions in a plan node (bottom-up on the plan tree too).
        let rec applyPlan (plan: OptimizedPlan) : OptimizedPlan =
            match plan with
            | OptimizedPlan.Scan _
            | OptimizedPlan.EmptyResult
            | OptimizedPlan.Insert _
            | OptimizedPlan.CreateTable _
            | OptimizedPlan.DropTable _       -> plan
            | OptimizedPlan.Filter(inner, pred) ->
                OptimizedPlan.Filter(applyPlan inner, foldExpr pred)
            | OptimizedPlan.Project(inner, cols) ->
                let foldedCols =
                    cols |> List.map (fun col ->
                        match col with
                        | OutputColumn.Star       -> OutputColumn.Star
                        | OutputColumn.Expr(e, a) -> OutputColumn.Expr(foldExpr e, a))
                OptimizedPlan.Project(applyPlan inner, foldedCols)
            | OptimizedPlan.Join(l, r, k, cond) ->
                OptimizedPlan.Join(applyPlan l, applyPlan r, k, Option.map foldExpr cond)
            | OptimizedPlan.Aggregate(inner, gb, aggs) ->
                OptimizedPlan.Aggregate(applyPlan inner, List.map foldExpr gb, aggs)
            | OptimizedPlan.Having(inner, pred) ->
                OptimizedPlan.Having(applyPlan inner, foldExpr pred)
            | OptimizedPlan.Sort(inner, keys) ->
                let foldedKeys = keys |> List.map (fun k -> { k with KeyExpr = foldExpr k.KeyExpr })
                OptimizedPlan.Sort(applyPlan inner, foldedKeys)
            | OptimizedPlan.Limit(inner, c, o) ->
                OptimizedPlan.Limit(applyPlan inner, c, o)
            | OptimizedPlan.Distinct(inner) ->
                OptimizedPlan.Distinct(applyPlan inner)
            | OptimizedPlan.Union(l, r, all) ->
                OptimizedPlan.Union(applyPlan l, applyPlan r, all)
            | OptimizedPlan.Update(t, asgn, pred) ->
                let foldedAsgn = asgn |> List.map (fun a -> { a with Value = foldExpr a.Value })
                OptimizedPlan.Update(t, foldedAsgn, Option.map foldExpr pred)
            | OptimizedPlan.Delete(t, pred) ->
                OptimizedPlan.Delete(t, Option.map foldExpr pred)

    // =========================================================================
    // Pass 2 — PredicatePushdown
    // =========================================================================
    // Split AND conjuncts in Filter predicates and push each conjunct as close
    // to its Scan as safe. This reduces rows processed by joins, sorts, etc.
    //
    // Outer-join safety:
    //   LEFT JOIN  → push to left side, NOT right (null-padding)
    //   RIGHT JOIN → push to right side, NOT left
    //   FULL JOIN  → don't push to either side
    //   INNER/CROSS → both sides safe

    module private PredicatePushdown =

        // Collect the set of table-alias strings referenced by qualified columns.
        // An unresolved (bare) column adds "__unknown__" to poison pushability.
        let rec private columnAliases (e: Expr) : Set<string> =
            match e with
            | Expr.Column(Some tbl, _)   -> Set.singleton tbl
            | Expr.Column(None, _)       -> Set.singleton "__unknown__"
            | Expr.Literal _ | Expr.Wildcard | Expr.AggExpr _ -> Set.empty
            | Expr.BinaryOp(_, l, r)     -> Set.union (columnAliases l) (columnAliases r)
            | Expr.UnaryOp(_, inner)     -> columnAliases inner
            | Expr.IsNull inner | Expr.IsNotNull inner -> columnAliases inner
            | Expr.Between(v, lo, hi)    ->
                columnAliases v |> Set.union (columnAliases lo) |> Set.union (columnAliases hi)
            | Expr.In(v, items) | Expr.NotIn(v, items) ->
                items |> List.fold (fun acc i -> Set.union acc (columnAliases i)) (columnAliases v)
            | Expr.Like(v, _) | Expr.NotLike(v, _) -> columnAliases v
            | Expr.FuncCall(_, args)     ->
                args |> List.fold (fun acc a -> Set.union acc (columnAliases a)) Set.empty

        // Walk the plan tree to collect every scan alias (for scope checking).
        let rec private aliasSet (plan: OptimizedPlan) : Set<string> =
            match plan with
            | OptimizedPlan.Scan(t, aliasOpt, _, _) ->
                Set.singleton (aliasOpt |> Option.defaultValue t)
            | OptimizedPlan.Filter(inner, _)
            | OptimizedPlan.Project(inner, _)
            | OptimizedPlan.Aggregate(inner, _, _)
            | OptimizedPlan.Having(inner, _)
            | OptimizedPlan.Sort(inner, _)
            | OptimizedPlan.Limit(inner, _, _)
            | OptimizedPlan.Distinct(inner)     -> aliasSet inner
            | OptimizedPlan.Join(l, r, _, _)
            | OptimizedPlan.Union(l, r, _)      -> Set.union (aliasSet l) (aliasSet r)
            | _                                 -> Set.empty

        // Split "a AND b AND c" into [a; b; c].
        let rec private splitConjuncts (e: Expr) : Expr list =
            match e with
            | Expr.BinaryOp(And, l, r) -> splitConjuncts l @ splitConjuncts r
            | _                        -> [e]

        // Re-AND a list of conjuncts (left-associative).
        let private combineAnd (exprs: Expr list) : Expr =
            match exprs with
            | []     -> Expr.Literal(SqlValue.Bool true)
            | [e]    -> e
            | h :: t -> List.fold (fun acc x -> Expr.BinaryOp(And, acc, x)) h t

        // Wrap a plan with a filter if there are leftover conjuncts.
        let private wrapKeeps (tree: OptimizedPlan) (keeps: Expr list) : OptimizedPlan =
            if keeps.IsEmpty then tree
            else OptimizedPlan.Filter(tree, combineAnd keeps)

        // Attempt to distribute conjuncts into the tree. Returns (stuck, rewritten).
        let rec private distributeConjuncts
            (tree: OptimizedPlan) (conjuncts: Expr list) : Expr list * OptimizedPlan =

            match tree with
            // Through Project — push conjuncts that only reference input aliases.
            | OptimizedPlan.Project(child, cols) ->
                let childAliases = aliasSet child
                let eligible, stuck =
                    conjuncts |> List.partition (fun c ->
                        let cols = columnAliases c
                        (not cols.IsEmpty) && cols.IsSubsetOf childAliases)
                if eligible.IsEmpty then (conjuncts, tree)
                else
                    let innerStuck, newChild = distributeConjuncts child eligible
                    (stuck, OptimizedPlan.Project(wrapKeeps newChild innerStuck, cols))

            // Through Sort — always safe (sort doesn't change which rows exist).
            | OptimizedPlan.Sort(child, keys) ->
                let innerStuck, newChild = distributeConjuncts child conjuncts
                ([], OptimizedPlan.Sort(wrapKeeps newChild innerStuck, keys))

            // Through Distinct — always safe.
            | OptimizedPlan.Distinct(child) ->
                let innerStuck, newChild = distributeConjuncts child conjuncts
                ([], OptimizedPlan.Distinct(wrapKeeps newChild innerStuck))

            // Through Join — per-side alias routing with outer-join safety.
            | OptimizedPlan.Join(l, r, k, cond) ->
                let leftAliases  = aliasSet l
                let rightAliases = aliasSet r
                let canPushLeft  = k = Inner || k = Cross || k = Left
                let canPushRight = k = Inner || k = Cross || k = Right

                let leftPush  = System.Collections.Generic.List<Expr>()
                let rightPush = System.Collections.Generic.List<Expr>()
                let stuck     = System.Collections.Generic.List<Expr>()

                for c in conjuncts do
                    let cols = columnAliases c
                    if canPushLeft && not cols.IsEmpty && cols.IsSubsetOf leftAliases then
                        leftPush.Add(c)
                    elif canPushRight && not cols.IsEmpty && cols.IsSubsetOf rightAliases then
                        rightPush.Add(c)
                    else
                        stuck.Add(c)

                let mutable newL = l
                let mutable newR = r
                if leftPush.Count > 0 then
                    let innerStuck, newLInner = distributeConjuncts l (List.ofSeq leftPush)
                    newL <- wrapKeeps newLInner innerStuck
                if rightPush.Count > 0 then
                    let innerStuck, newRInner = distributeConjuncts r (List.ofSeq rightPush)
                    newR <- wrapKeeps newRInner innerStuck

                (List.ofSeq stuck, OptimizedPlan.Join(newL, newR, k, cond))

            // Leaves and unsafe nodes — can't push further.
            | _ -> (conjuncts, tree)

        // Apply a filter on top of an already-recursed child, pushing what we can.
        let private pushFilter (inner: OptimizedPlan) (pred: Expr) : OptimizedPlan =
            let conjuncts = splitConjuncts pred
            let keep, pushed = distributeConjuncts inner conjuncts
            if keep.IsEmpty then pushed
            else OptimizedPlan.Filter(pushed, combineAnd keep)

        // Main recursive descent.
        let rec applyPlan (plan: OptimizedPlan) : OptimizedPlan =
            match plan with
            | OptimizedPlan.Filter(inner, pred) ->
                pushFilter (applyPlan inner) pred
            | OptimizedPlan.Project(inner, cols) ->
                OptimizedPlan.Project(applyPlan inner, cols)
            | OptimizedPlan.Join(l, r, k, cond) ->
                OptimizedPlan.Join(applyPlan l, applyPlan r, k, cond)
            | OptimizedPlan.Aggregate(inner, gb, aggs) ->
                OptimizedPlan.Aggregate(applyPlan inner, gb, aggs)
            | OptimizedPlan.Having(inner, pred) ->
                OptimizedPlan.Having(applyPlan inner, pred)
            | OptimizedPlan.Sort(inner, keys) ->
                OptimizedPlan.Sort(applyPlan inner, keys)
            | OptimizedPlan.Limit(inner, c, o) ->
                OptimizedPlan.Limit(applyPlan inner, c, o)
            | OptimizedPlan.Distinct(inner) ->
                OptimizedPlan.Distinct(applyPlan inner)
            | OptimizedPlan.Union(l, r, all) ->
                OptimizedPlan.Union(applyPlan l, applyPlan r, all)
            | _ -> plan  // Scan, EmptyResult, DML/DDL

    // =========================================================================
    // Pass 3 — ProjectionPruning
    // =========================================================================
    // Top-down traversal carrying the set of (alias, column) pairs required by
    // the parent. At each Scan, intersect with that scan's alias to populate
    // requiredColumns. None means "everything needed" (no annotation).

    module private ProjectionPruning =

        // A requirement is a set of (tableAlias, columnName) pairs.
        type Req = Set<string * string>

        // Extract the (alias, col) pairs a list of expressions reference.
        // Returns None if any Wildcard is found (means "all columns needed").
        let rec private requiredFromExpr (e: Expr) : Req option =
            match e with
            | Expr.Wildcard -> None  // wildcard = we need everything
            | Expr.Column(Some tbl, col) -> Some (Set.singleton (tbl, col))
            | Expr.Column(None, _)       -> Some Set.empty  // unresolved, can't prune
            | Expr.Literal _ | Expr.AggExpr _ -> Some Set.empty
            | Expr.BinaryOp(_, l, r) ->
                match requiredFromExpr l, requiredFromExpr r with
                | Some ls, Some rs -> Some (Set.union ls rs)
                | _                -> None
            | Expr.UnaryOp(_, inner) -> requiredFromExpr inner
            | Expr.IsNull inner | Expr.IsNotNull inner -> requiredFromExpr inner
            | Expr.Between(v, lo, hi) ->
                match requiredFromExpr v, requiredFromExpr lo, requiredFromExpr hi with
                | Some vs, Some los, Some his -> Some (vs |> Set.union los |> Set.union his)
                | _ -> None
            | Expr.In(v, items) | Expr.NotIn(v, items) ->
                let vReq = requiredFromExpr v
                let iReqs = List.map requiredFromExpr items
                if List.exists Option.isNone (vReq :: iReqs) then None
                else
                    Some (List.fold (fun acc opt ->
                        match opt with Some s -> Set.union acc s | None -> acc)
                        (Option.defaultValue Set.empty vReq) iReqs)
            | Expr.Like(v, _) | Expr.NotLike(v, _) -> requiredFromExpr v
            | Expr.FuncCall(_, args) ->
                let argReqs = List.map requiredFromExpr args
                if List.exists Option.isNone argReqs then None
                else Some (argReqs |> List.fold (fun acc opt ->
                    match opt with Some s -> Set.union acc s | None -> acc) Set.empty)

        let private requiredFromExprs (exprs: Expr list) : Req option =
            List.fold (fun accOpt e ->
                match accOpt, requiredFromExpr e with
                | Some acc, Some r -> Some (Set.union acc r)
                | _                -> None
            ) (Some Set.empty) exprs

        let private outputColExprs (cols: OutputColumn list) : Expr list =
            cols |> List.map (fun col ->
                match col with
                | OutputColumn.Star       -> Expr.Wildcard
                | OutputColumn.Expr(e, _) -> e)

        // Main top-down pruning recursion. req=None means "all columns needed".
        let rec applyPlan (plan: OptimizedPlan) (req: Req option) : OptimizedPlan =
            match plan with
            | OptimizedPlan.Scan(t, aliasOpt, _, sl) ->
                match req with
                | None -> plan  // no pruning info
                | Some needed ->
                    let scanAlias = aliasOpt |> Option.defaultValue t
                    let cols =
                        needed
                        |> Set.toList
                        |> List.choose (fun (alias, col) -> if alias = scanAlias then Some col else None)
                        |> List.sort
                    // Only annotate if we have actual columns to prune to.
                    if cols.IsEmpty then plan
                    else OptimizedPlan.Scan(t, aliasOpt, Some cols, sl)

            | OptimizedPlan.EmptyResult -> plan

            | OptimizedPlan.Project(inner, cols) ->
                // The Project defines what the parent can see. Build requirements
                // from the Project's own expressions (not the parent's req).
                let newReq = requiredFromExprs (outputColExprs cols)
                OptimizedPlan.Project(applyPlan inner newReq, cols)

            | OptimizedPlan.Filter(inner, pred) ->
                let predReq = requiredFromExpr pred
                let combined =
                    match req, predReq with
                    | Some r, Some pr -> Some (Set.union r pr)
                    | _               -> None
                OptimizedPlan.Filter(applyPlan inner combined, pred)

            | OptimizedPlan.Aggregate(inner, gb, aggs) ->
                // Need whatever the group-by exprs and agg args reference.
                let aggExprs =
                    aggs |> List.choose (fun a ->
                        match a.Arg with AggArg.Expr e -> Some e | AggArg.Star -> None)
                let newReq = requiredFromExprs (gb @ aggExprs)
                OptimizedPlan.Aggregate(applyPlan inner newReq, gb, aggs)

            | OptimizedPlan.Having(inner, pred) ->
                let predReq = requiredFromExpr pred
                let combined =
                    match req, predReq with
                    | Some r, Some pr -> Some (Set.union r pr)
                    | _               -> None
                OptimizedPlan.Having(applyPlan inner combined, pred)

            | OptimizedPlan.Sort(inner, keys) ->
                let keyReq = requiredFromExprs (keys |> List.map (fun k -> k.KeyExpr))
                let combined =
                    match req, keyReq with
                    | Some r, Some kr -> Some (Set.union r kr)
                    | _               -> None
                OptimizedPlan.Sort(applyPlan inner combined, keys)

            | OptimizedPlan.Limit(inner, c, o) ->
                OptimizedPlan.Limit(applyPlan inner req, c, o)

            | OptimizedPlan.Distinct(inner) ->
                OptimizedPlan.Distinct(applyPlan inner req)

            | OptimizedPlan.Join(l, r, k, cond) ->
                let condReq =
                    match cond with
                    | Some c -> requiredFromExpr c
                    | None   -> Some Set.empty
                let combined =
                    match req, condReq with
                    | Some r, Some cr -> Some (Set.union r cr)
                    | _               -> None
                OptimizedPlan.Join(applyPlan l combined, applyPlan r combined, k, cond)

            | OptimizedPlan.Union(l, r, all) ->
                OptimizedPlan.Union(applyPlan l req, applyPlan r req, all)

            // DML/DDL — no pruning.
            | _ -> plan

    // =========================================================================
    // Pass 4 — DeadCodeElimination
    // =========================================================================
    // Remove provably-empty subtrees and pointless filters.
    //
    // Sources of EmptyResult:
    //   * Filter(_, FALSE) or Filter(_, NULL) — predicate blocks all rows
    //   * Limit(_, Some 0L, _)               — zero rows requested
    //
    // EmptyResult propagates upward through most unary and binary operators.
    // Exceptions:
    //   * Aggregate — SELECT COUNT(*) from empty table still returns one row
    //   * Outer joins — a LEFT JOIN with empty right still produces left rows

    module private DeadCodeElimination =

        let rec applyPlan (plan: OptimizedPlan) : OptimizedPlan =
            match plan with
            | OptimizedPlan.Scan _
            | OptimizedPlan.EmptyResult
            | OptimizedPlan.Insert _
            | OptimizedPlan.CreateTable _
            | OptimizedPlan.DropTable _
            | OptimizedPlan.Update _
            | OptimizedPlan.Delete _       -> plan

            | OptimizedPlan.Filter(inner, pred) ->
                let inner' = applyPlan inner
                match pred with
                | Expr.Literal(SqlValue.Bool true) -> inner'  // tautology — drop filter
                | Expr.Literal(SqlValue.Bool false) -> OptimizedPlan.EmptyResult
                | Expr.Literal(SqlValue.Null)       -> OptimizedPlan.EmptyResult
                | _ ->
                    match inner' with
                    | OptimizedPlan.EmptyResult -> OptimizedPlan.EmptyResult
                    | _                         -> OptimizedPlan.Filter(inner', pred)

            | OptimizedPlan.Project(inner, cols) ->
                let inner' = applyPlan inner
                match inner' with
                | OptimizedPlan.EmptyResult -> OptimizedPlan.EmptyResult
                | _                         -> OptimizedPlan.Project(inner', cols)

            | OptimizedPlan.Sort(inner, keys) ->
                let inner' = applyPlan inner
                match inner' with
                | OptimizedPlan.EmptyResult -> OptimizedPlan.EmptyResult
                | _                         -> OptimizedPlan.Sort(inner', keys)

            | OptimizedPlan.Limit(inner, count, offset) ->
                // LIMIT 0 produces zero rows, regardless of input.
                match count with
                | Some 0L -> OptimizedPlan.EmptyResult
                | _ ->
                    let inner' = applyPlan inner
                    match inner' with
                    | OptimizedPlan.EmptyResult -> OptimizedPlan.EmptyResult
                    | _                         -> OptimizedPlan.Limit(inner', count, offset)

            | OptimizedPlan.Distinct(inner) ->
                let inner' = applyPlan inner
                match inner' with
                | OptimizedPlan.EmptyResult -> OptimizedPlan.EmptyResult
                | _                         -> OptimizedPlan.Distinct(inner')

            | OptimizedPlan.Having(inner, pred) ->
                let inner' = applyPlan inner
                match inner' with
                | OptimizedPlan.EmptyResult -> OptimizedPlan.EmptyResult
                | _                         -> OptimizedPlan.Having(inner', pred)

            | OptimizedPlan.Aggregate(inner, gb, aggs) ->
                // Do NOT eliminate Aggregate(EmptyResult) — COUNT(*) must return 0.
                OptimizedPlan.Aggregate(applyPlan inner, gb, aggs)

            | OptimizedPlan.Join(l, r, k, cond) ->
                let l' = applyPlan l
                let r' = applyPlan r
                // INNER and CROSS join: either side empty → whole join is empty.
                match l', r', k with
                | OptimizedPlan.EmptyResult, _, (Inner | Cross)
                | _, OptimizedPlan.EmptyResult, (Inner | Cross) -> OptimizedPlan.EmptyResult
                | _ -> OptimizedPlan.Join(l', r', k, cond)

            | OptimizedPlan.Union(l, r, all) ->
                let l' = applyPlan l
                let r' = applyPlan r
                match l', r' with
                | OptimizedPlan.EmptyResult, x -> x
                | x, OptimizedPlan.EmptyResult -> x
                | _                            -> OptimizedPlan.Union(l', r', all)

    // =========================================================================
    // Pass 5 — LimitPushdown
    // =========================================================================
    // Attach scan_limit hints to Scans that can early-terminate. The real Limit
    // node is preserved — the hint is advisory for backends. It is only safe to
    // push through Project and Filter (with the caveat that filters may need to
    // read more rows). We don't push through Sort, Aggregate, Join, or Distinct.

    module private LimitPushdown =

        // Thread a scan_limit hint down through safe pass-through nodes.
        let rec private attach (plan: OptimizedPlan) (limit: int64) : OptimizedPlan =
            match plan with
            | OptimizedPlan.Scan(t, a, rc, existing) ->
                let newLimit = match existing with Some e -> min e limit | None -> limit
                OptimizedPlan.Scan(t, a, rc, Some newLimit)
            | OptimizedPlan.Project(inner, cols) ->
                OptimizedPlan.Project(attach inner limit, cols)
            | OptimizedPlan.Filter(inner, pred) ->
                OptimizedPlan.Filter(attach inner limit, pred)
            | _ ->
                // Unsafe to annotate past anything else — apply normal recursion.
                applyPlan plan

        // Main recursion.
        and applyPlan (plan: OptimizedPlan) : OptimizedPlan =
            match plan with
            | OptimizedPlan.Limit(inner, count, offset) ->
                // Only push the count when offset is zero or absent.
                // With a non-zero offset the backend still needs to skip rows,
                // so the raw scan needs count+offset rows; we conservatively
                // don't push in that case to keep the hint simple.
                let pushedCount =
                    match count, offset with
                    | Some c, (None | Some 0L) -> Some c
                    | _                        -> None
                let newInner =
                    match pushedCount with
                    | Some c -> attach inner c
                    | None   -> applyPlan inner
                OptimizedPlan.Limit(newInner, count, offset)
            | OptimizedPlan.Filter(inner, pred) ->
                OptimizedPlan.Filter(applyPlan inner, pred)
            | OptimizedPlan.Project(inner, cols) ->
                OptimizedPlan.Project(applyPlan inner, cols)
            | OptimizedPlan.Sort(inner, keys) ->
                OptimizedPlan.Sort(applyPlan inner, keys)
            | OptimizedPlan.Aggregate(inner, gb, aggs) ->
                OptimizedPlan.Aggregate(applyPlan inner, gb, aggs)
            | OptimizedPlan.Having(inner, pred) ->
                OptimizedPlan.Having(applyPlan inner, pred)
            | OptimizedPlan.Distinct(inner) ->
                OptimizedPlan.Distinct(applyPlan inner)
            | OptimizedPlan.Join(l, r, k, cond) ->
                OptimizedPlan.Join(applyPlan l, applyPlan r, k, cond)
            | OptimizedPlan.Union(l, r, all) ->
                OptimizedPlan.Union(applyPlan l, applyPlan r, all)
            | _ -> plan

    // =========================================================================
    // Public API
    // =========================================================================

    /// Build the default five-pass optimization pipeline.
    let defaultPasses () : Pass list =
        [ { Name = "ConstantFolding";    Apply = ConstantFolding.applyPlan }
          { Name = "PredicatePushdown";  Apply = PredicatePushdown.applyPlan }
          { Name = "ProjectionPruning";  Apply = fun p -> ProjectionPruning.applyPlan p None }
          { Name = "DeadCodeElimination"; Apply = DeadCodeElimination.applyPlan }
          { Name = "LimitPushdown";      Apply = LimitPushdown.applyPlan } ]

    /// Apply a custom list of passes in order to a lifted plan.
    let optimizeWithPasses (passes: Pass list) (plan: LogicalPlan) : OptimizedPlan =
        let initial = lift plan
        List.fold (fun acc pass -> pass.Apply acc) initial passes

    /// Lift and optimize using the default five-pass pipeline.
    let optimize (plan: LogicalPlan) : OptimizedPlan =
        optimizeWithPasses (defaultPasses ()) plan
