// SqlPlanner.fs — logical query plan builder for SQL statements.
//
// This module transforms a parsed SQL statement (Statement DU) into a
// tree of LogicalPlan nodes. No I/O, no execution — planning only.
//
// Architecture: bottom-up for SELECT
//   Scan → Filter → Aggregate → Having → Project → Distinct → Sort → Limit
//
// Usage:
//   let schema = InMemorySchemaProvider(Map.ofList [("users", ["id";"name";"age"])])
//   let plan = Planner.plan schema (Statement.Select { ... })

namespace CodingAdventures.SqlPlanner.FSharp

// ── SQL primitive values ──────────────────────────────────────────────────────
// A minimal value type for literal expressions in query plans.

[<RequireQualifiedAccess>]
type SqlValue =
    | Null
    | Integer of int64
    | Real    of float
    | Text    of string
    | Bool    of bool

// ── Operator enumerations ─────────────────────────────────────────────────────
// Suffix names avoid DU case conflicts with Expr.BinaryOp, Expr.UnaryOp, Expr.AggExpr.

/// Binary infix operators.
type BinaryOperator =
    | Eq | NotEq | Lt | Lte | Gt | Gte
    | And | Or
    | Add | Sub | Mul | Div | Mod

/// Unary prefix operators.
type UnaryOperator = Not | Neg

/// Aggregate functions.
type AggFunction = Count | Sum | Avg | Min | Max

// ── Expression IR ────────────────────────────────────────────────────────────
// AggArg and Expr are mutually recursive via the `and` keyword.

/// Argument to an aggregate function.
[<RequireQualifiedAccess>]
type AggArg =
    | Star
    | Expr of Expr

/// A scalar expression in a query plan.
and [<RequireQualifiedAccess>] Expr =
    | Literal   of value: SqlValue
    | Column    of table: string option * col: string
    | BinaryOp  of op: BinaryOperator * left: Expr * right: Expr
    | UnaryOp   of op: UnaryOperator  * operand: Expr
    | FuncCall  of name: string * args: Expr list
    | IsNull    of Expr
    | IsNotNull of Expr
    | Between   of value: Expr * low: Expr * high: Expr
    | In        of value: Expr * items: Expr list
    | NotIn     of value: Expr * items: Expr list
    | Like      of value: Expr * pattern: string
    | NotLike   of value: Expr * pattern: string
    | Wildcard
    | AggExpr   of func: AggFunction * arg: AggArg * distinct: bool

// ── Sort / join helpers ───────────────────────────────────────────────────────

type SortDir   = Asc | Desc
type NullOrder = NullsFirst | NullsLast
type JoinKind  = Inner | Left | Right | Full | Cross

type SortKey = { KeyExpr: Expr; Direction: SortDir; NullOrder: NullOrder }

// ── SELECT output columns ─────────────────────────────────────────────────────

[<RequireQualifiedAccess>]
type OutputColumn =
    | Expr of expr: Expr * alias: string option
    | Star

// ── Structural types ──────────────────────────────────────────────────────────

type JoinClause =
    { Kind:  JoinKind
      Table: string
      Alias: string option
      On:    Expr option }

type ColumnDef =
    { Name:       string
      TypeName:   string
      NotNull:    bool
      PrimaryKey: bool
      Unique:     bool
      Default:    Expr option }

type Assignment = { Column: string; Value: Expr }

type LimitClause = { Count: int64 option; Offset: int64 option }

// ── Statement AST ─────────────────────────────────────────────────────────────
// The F# sql-parser is a stub, so this package defines its own statement types.

type SelectStmt =
    { Distinct: bool
      Columns:  OutputColumn list
      From:     (string * string option) list
      Joins:    JoinClause list
      Where:    Expr option
      GroupBy:  Expr list
      Having:   Expr option
      OrderBy:  SortKey list
      Limit:    LimitClause option }

type InsertStmt =
    { Table:   string
      Columns: string list option
      Values:  Expr list list }

type UpdateStmt =
    { Table:       string
      Assignments: Assignment list
      Where:       Expr option }

type DeleteStmt = { Table: string; Where: Expr option }

type CreateTableStmt =
    { Table:       string
      IfNotExists: bool
      Columns:     ColumnDef list }

type DropTableStmt = { Table: string; IfExists: bool }

[<RequireQualifiedAccess>]
type Statement =
    | Select      of SelectStmt
    | Insert      of InsertStmt
    | Update      of UpdateStmt
    | Delete      of DeleteStmt
    | CreateTable of CreateTableStmt
    | DropTable   of DropTableStmt

// ── Aggregate item ────────────────────────────────────────────────────────────

type AggregateItem =
    { Func:     AggFunction
      Arg:      AggArg
      Alias:    string
      Distinct: bool }

// ── Logical plan ─────────────────────────────────────────────────────────────
// InsertSource and LogicalPlan are mutually recursive.

[<RequireQualifiedAccess>]
type InsertSource =
    | Values of Expr list list
    | Query  of LogicalPlan

and [<RequireQualifiedAccess>] LogicalPlan =
    | Scan        of table: string * alias: string option
    | Filter      of input: LogicalPlan * predicate: Expr
    | Project     of input: LogicalPlan * columns: OutputColumn list
    | Join        of left: LogicalPlan * right: LogicalPlan * kind: JoinKind * condition: Expr option
    | Aggregate   of input: LogicalPlan * groupBy: Expr list * aggregates: AggregateItem list
    | Having      of input: LogicalPlan * predicate: Expr
    | Sort        of input: LogicalPlan * keys: SortKey list
    | Limit       of input: LogicalPlan * count: int64 option * offset: int64 option
    | Distinct    of input: LogicalPlan
    | Union       of left: LogicalPlan * right: LogicalPlan * all: bool
    | Insert      of table: string * columns: string list option * source: InsertSource
    | Update      of table: string * assignments: Assignment list * predicate: Expr option
    | Delete      of table: string * predicate: Expr option
    | CreateTable of table: string * ifNotExists: bool * columns: ColumnDef list
    | DropTable   of table: string * ifExists: bool

// ── Plan errors ───────────────────────────────────────────────────────────────

[<RequireQualifiedAccess>]
type PlanError =
    | AmbiguousColumn      of column: string * tables: string list
    | UnknownTable         of table: string
    | UnknownColumn        of table: string option * column: string
    | InvalidAggregate     of message: string
    | UnsupportedStatement of kind: string
    | InternalError        of message: string

// ── Schema provider ───────────────────────────────────────────────────────────

type ISchemaProvider =
    abstract member Columns : table: string -> Result<string list, PlanError>

type InMemorySchemaProvider(tables: Map<string, string list>) =
    interface ISchemaProvider with
        member _.Columns(table) =
            match Map.tryFind table tables with
            | Some cols -> Ok cols
            | None      -> Error (PlanError.UnknownTable table)

// ── Internal helpers ──────────────────────────────────────────────────────────

module private Helpers =
    let traverseResult (f: 'a -> Result<'b, 'e>) (lst: 'a list) : Result<'b list, 'e> =
        let rec loop acc = function
            | []      -> Ok (List.rev acc)
            | x :: xs ->
                match f x with
                | Error e -> Error e
                | Ok v    -> loop (v :: acc) xs
        loop [] lst

// ── Planner ───────────────────────────────────────────────────────────────────

module Planner =
    open Helpers

    type private ScopeEntry = { Alias: string; Table: string; Cols: string list }

    // Build scope from FROM + JOIN sources.
    let private buildScope
        (schema: ISchemaProvider)
        (from:   (string * string option) list)
        (joins:  JoinClause list)
        : Result<ScopeEntry list, PlanError> =

        let fromResult =
            from |> traverseResult (fun (tbl, aliasOpt) ->
                match schema.Columns(tbl) with
                | Error e -> Error e
                | Ok cols ->
                    Ok { Alias = aliasOpt |> Option.defaultValue tbl
                         Table = tbl
                         Cols  = cols })

        let joinResult =
            joins |> traverseResult (fun j ->
                match schema.Columns(j.Table) with
                | Error e -> Error e
                | Ok cols ->
                    Ok { Alias = j.Alias |> Option.defaultValue j.Table
                         Table = j.Table
                         Cols  = cols })

        match fromResult, joinResult with
        | Ok fe, Ok je -> Ok (fe @ je)
        | Error e, _   -> Error e
        | _, Error e   -> Error e

    // Resolve a column reference against scope.
    let private resolveColumn
        (scope:    ScopeEntry list)
        (tableOpt: string option)
        (col:      string)
        : Result<Expr, PlanError> =

        let ciEq a b =
            System.String.Compare(a, b, System.StringComparison.OrdinalIgnoreCase) = 0

        match tableOpt with
        | Some tbl ->
            match scope |> List.tryFind (fun e -> e.Alias = tbl) with
            | None ->
                Error (PlanError.UnknownTable tbl)
            | Some entry ->
                if entry.Cols |> List.exists (ciEq col)
                then Ok (Expr.Column(Some entry.Alias, col))
                else Error (PlanError.UnknownColumn(Some tbl, col))
        | None ->
            let matches = scope |> List.filter (fun e -> e.Cols |> List.exists (ciEq col))
            match matches with
            | []  -> Error (PlanError.UnknownColumn(None, col))
            | [m] -> Ok (Expr.Column(Some m.Alias, col))
            | ms  -> Error (PlanError.AmbiguousColumn(col, ms |> List.map (fun e -> e.Alias)))

    // Recursively resolve column references inside an expression.
    let rec private resolveExpr (scope: ScopeEntry list) (expr: Expr) : Result<Expr, PlanError> =
        match expr with
        | Expr.Column(tOpt, col) -> resolveColumn scope tOpt col
        | Expr.Literal _         -> Ok expr
        | Expr.Wildcard          -> Ok expr
        | Expr.AggExpr _         -> Ok expr
        | Expr.BinaryOp(op, l, r) ->
            match resolveExpr scope l with
            | Error e -> Error e
            | Ok rl   ->
                match resolveExpr scope r with
                | Error e -> Error e
                | Ok rr   -> Ok (Expr.BinaryOp(op, rl, rr))
        | Expr.UnaryOp(op, e) ->
            match resolveExpr scope e with
            | Error e2 -> Error e2
            | Ok re    -> Ok (Expr.UnaryOp(op, re))
        | Expr.FuncCall(name, args) ->
            match args |> traverseResult (resolveExpr scope) with
            | Error e    -> Error e
            | Ok rArgs   -> Ok (Expr.FuncCall(name, rArgs))
        | Expr.IsNull e ->
            match resolveExpr scope e with
            | Error e2 -> Error e2
            | Ok re    -> Ok (Expr.IsNull re)
        | Expr.IsNotNull e ->
            match resolveExpr scope e with
            | Error e2 -> Error e2
            | Ok re    -> Ok (Expr.IsNotNull re)
        | Expr.Between(v, lo, hi) ->
            match resolveExpr scope v with
            | Error e -> Error e
            | Ok rv   ->
                match resolveExpr scope lo with
                | Error e  -> Error e
                | Ok rlo   ->
                    match resolveExpr scope hi with
                    | Error e  -> Error e
                    | Ok rhi   -> Ok (Expr.Between(rv, rlo, rhi))
        | Expr.In(v, items) ->
            match resolveExpr scope v with
            | Error e -> Error e
            | Ok rv   ->
                match items |> traverseResult (resolveExpr scope) with
                | Error e    -> Error e
                | Ok rItems  -> Ok (Expr.In(rv, rItems))
        | Expr.NotIn(v, items) ->
            match resolveExpr scope v with
            | Error e -> Error e
            | Ok rv   ->
                match items |> traverseResult (resolveExpr scope) with
                | Error e    -> Error e
                | Ok rItems  -> Ok (Expr.NotIn(rv, rItems))
        | Expr.Like(v, p) ->
            match resolveExpr scope v with
            | Error e -> Error e
            | Ok rv   -> Ok (Expr.Like(rv, p))
        | Expr.NotLike(v, p) ->
            match resolveExpr scope v with
            | Error e -> Error e
            | Ok rv   -> Ok (Expr.NotLike(rv, p))

    // Collect AggExpr nodes from a list of expressions.
    let private collectAggregates (exprs: Expr list) : AggregateItem list =
        let mutable found: AggregateItem list = []
        let mutable counter = 0
        let rec walk e =
            match e with
            | Expr.AggExpr(func, arg, distinct) ->
                let alias = sprintf "_agg%d" counter
                counter <- counter + 1
                found <- { Func = func; Arg = arg; Alias = alias; Distinct = distinct } :: found
            | Expr.BinaryOp(_, l, r) -> walk l; walk r
            | Expr.UnaryOp(_, e2)    -> walk e2
            | Expr.FuncCall(_, args) -> List.iter walk args
            | Expr.IsNull e2         -> walk e2
            | Expr.IsNotNull e2      -> walk e2
            | Expr.Between(v, lo, hi)-> walk v; walk lo; walk hi
            | Expr.In(v, items)      -> walk v; List.iter walk items
            | Expr.NotIn(v, items)   -> walk v; List.iter walk items
            | Expr.Like(v, _)        -> walk v
            | Expr.NotLike(v, _)     -> walk v
            | _                      -> ()
        List.iter walk exprs
        List.rev found

    // Check whether any expression contains an aggregate call.
    let private containsAgg (exprs: Expr list) : bool =
        let rec check e =
            match e with
            | Expr.AggExpr _          -> true
            | Expr.BinaryOp(_, l, r)  -> check l || check r
            | Expr.UnaryOp(_, e2)     -> check e2
            | Expr.FuncCall(_, args)  -> List.exists check args
            | Expr.IsNull e2          -> check e2
            | Expr.IsNotNull e2       -> check e2
            | Expr.Between(v, lo, hi) -> check v || check lo || check hi
            | Expr.In(v, items)       -> check v || List.exists check items
            | Expr.NotIn(v, items)    -> check v || List.exists check items
            | Expr.Like(v, _)         -> check v
            | Expr.NotLike(v, _)      -> check v
            | _                       -> false
        List.exists check exprs

    let private exprOfOutputCol = function
        | OutputColumn.Star      -> Expr.Wildcard
        | OutputColumn.Expr(e,_) -> e

    // Validate table exists and return a Scan node.
    let private buildScan (schema: ISchemaProvider) (tbl: string) (aliasOpt: string option)
        : Result<LogicalPlan, PlanError> =
        match schema.Columns(tbl) with
        | Error e -> Error e
        | Ok _    -> Ok (LogicalPlan.Scan(tbl, aliasOpt))

    // Build the FROM + JOIN tree left-associatively.
    let private buildFromTree
        (schema: ISchemaProvider)
        (from:   (string * string option) list)
        (joins:  JoinClause list)
        : Result<LogicalPlan, PlanError> =

        match from with
        | [] -> Error (PlanError.UnsupportedStatement "SELECT without FROM")
        | (tbl0, alias0) :: rest ->
            match buildScan schema tbl0 alias0 with
            | Error e -> Error e
            | Ok root ->
                let withRest =
                    rest |> List.fold (fun accR (tbl, aliasOpt) ->
                        match accR with
                        | Error e -> Error e
                        | Ok acc  ->
                            match buildScan schema tbl aliasOpt with
                            | Error e    -> Error e
                            | Ok right   -> Ok (LogicalPlan.Join(acc, right, Cross, None)))
                        (Ok root)
                match withRest with
                | Error e     -> Error e
                | Ok fromPlan ->
                    joins |> List.fold (fun accR jc ->
                        match accR with
                        | Error e -> Error e
                        | Ok acc  ->
                            match buildScan schema jc.Table jc.Alias with
                            | Error e  -> Error e
                            | Ok right -> Ok (LogicalPlan.Join(acc, right, jc.Kind, jc.On)))
                        (Ok fromPlan)

    // Plan a SELECT using the 8-step bottom-up pipeline.
    // Uses explicit match chains (railway pattern) to avoid F# indentation issues
    // with `let` bindings nested inside `|> Result.bind` lambda continuations.
    let private planSelect (schema: ISchemaProvider) (s: SelectStmt) : Result<LogicalPlan, PlanError> =
        match buildScope schema s.From s.Joins with
        | Error e -> Error e
        | Ok scope ->
        match buildFromTree schema s.From s.Joins with
        | Error e -> Error e
        | Ok fromPlan ->

        // Step 1: WHERE → Filter
        let filtered =
            match s.Where with
            | None      -> Ok fromPlan
            | Some pred ->
                match resolveExpr scope pred with
                | Error e    -> Error e
                | Ok rPred   -> Ok (LogicalPlan.Filter(fromPlan, rPred))

        match filtered with
        | Error e -> Error e
        | Ok afterFilter ->

        let colExprs    = s.Columns |> List.map exprOfOutputCol
        let havingExprs = s.Having  |> Option.toList

        // Step 2: GROUP BY + Aggregate
        let needsAgg =
            s.GroupBy <> [] || containsAgg colExprs || containsAgg havingExprs

        let aggregated =
            if needsAgg then
                let aggs = collectAggregates (colExprs @ havingExprs)
                match s.GroupBy |> traverseResult (resolveExpr scope) with
                | Error e       -> Error e
                | Ok rGroupBy   -> Ok (LogicalPlan.Aggregate(afterFilter, rGroupBy, aggs))
            else
                Ok afterFilter

        match aggregated with
        | Error e -> Error e
        | Ok afterAgg ->

        // Step 3: HAVING
        let having =
            match s.Having with
            | None      -> Ok afterAgg
            | Some pred ->
                match resolveExpr scope pred with
                | Ok rPred                        -> Ok (LogicalPlan.Having(afterAgg, rPred))
                | Error (PlanError.UnknownColumn _) -> Ok (LogicalPlan.Having(afterAgg, pred))
                | Error e                         -> Error e

        match having with
        | Error e -> Error e
        | Ok afterHaving ->

        // Step 4: PROJECT
        let projected =
            match s.Columns |> traverseResult (fun outCol ->
                match outCol with
                | OutputColumn.Star -> Ok OutputColumn.Star
                | OutputColumn.Expr(e, alias) ->
                    match resolveExpr scope e with
                    | Ok re -> Ok (OutputColumn.Expr(re, alias))
                    | Error (PlanError.UnknownColumn _) when needsAgg ->
                        Ok (OutputColumn.Expr(e, alias))
                    | Error e2 -> Error e2) with
            | Error e    -> Error e
            | Ok cols    -> Ok (LogicalPlan.Project(afterHaving, cols))

        match projected with
        | Error e -> Error e
        | Ok afterProject ->

        // Step 5: DISTINCT
        let distinct =
            if s.Distinct
            then LogicalPlan.Distinct(afterProject)
            else afterProject

        // Step 6: ORDER BY
        let sorted =
            if s.OrderBy = [] then
                Ok distinct
            else
                match s.OrderBy |> traverseResult (fun key ->
                    match resolveExpr scope key.KeyExpr with
                    | Ok re                           -> Ok { key with KeyExpr = re }
                    | Error (PlanError.UnknownColumn _) -> Ok key
                    | Error e                         -> Error e) with
                | Error e    -> Error e
                | Ok keys    -> Ok (LogicalPlan.Sort(distinct, keys))

        match sorted with
        | Error e -> Error e
        | Ok afterSort ->

        // Step 7: LIMIT / OFFSET
        match s.Limit with
        | None    -> Ok afterSort
        | Some lc -> Ok (LogicalPlan.Limit(afterSort, lc.Count, lc.Offset))

    // ── Public API ────────────────────────────────────────────────────────────

    /// Transform a single Statement into a LogicalPlan, or return a PlanError.
    let plan (schema: ISchemaProvider) (stmt: Statement) : Result<LogicalPlan, PlanError> =
        match stmt with
        | Statement.Select s ->
            planSelect schema s
        | Statement.Insert i ->
            match schema.Columns(i.Table) with
            | Error e -> Error e
            | Ok _    -> Ok (LogicalPlan.Insert(i.Table, i.Columns, InsertSource.Values i.Values))
        | Statement.Update u ->
            match schema.Columns(u.Table) with
            | Error e -> Error e
            | Ok _    -> Ok (LogicalPlan.Update(u.Table, u.Assignments, u.Where))
        | Statement.Delete d ->
            match schema.Columns(d.Table) with
            | Error e -> Error e
            | Ok _    -> Ok (LogicalPlan.Delete(d.Table, d.Where))
        | Statement.CreateTable ct ->
            Ok (LogicalPlan.CreateTable(ct.Table, ct.IfNotExists, ct.Columns))
        | Statement.DropTable dt ->
            Ok (LogicalPlan.DropTable(dt.Table, dt.IfExists))

    /// Plan every statement in the list, failing on the first error.
    let planAll (schema: ISchemaProvider) (stmts: Statement list) : Result<LogicalPlan list, PlanError> =
        stmts |> Helpers.traverseResult (plan schema)
