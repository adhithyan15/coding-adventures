// MiniSqlite.fs — Level 1 F# facade: full SQL pipeline integration.
//
// Level 1 graduates the mini-sqlite facade from its Level 0 in-house
// regex engine to the full five-stage query pipeline:
//
//   sql text
//     → SqlTextParser.parse          (text → Statement DU)
//     → Planner.plan                 (Statement → LogicalPlan)
//     → SqlOptimizer.optimize        (LogicalPlan → OptimizedPlan)
//     → SqlCodegen.compile           (OptimizedPlan → Program)
//     → SqlVm.execute                (Program × Backend → QueryResult)
//
// ── Design decisions ──────────────────────────────────────────────────────
//
// SqlParser (the F# package) is a stub that returns a ping string; the
// actual text → Statement conversion is done here by SqlTextParser, a module
// that converts raw SQL text into the Statement DU defined in sql-planner.
//
// Scalar functions (LENGTH, UPPER, LOWER, SUBSTR, TRIM, ABS, ROUND, etc.)
// are not yet implemented in the codegen/vm pipeline — the codegen pushes
// NULL as a placeholder for any Expr.FuncCall node.  To support them at
// Level 1 we add a plan rewrite pass (FuncRewriter) before the codegen step
// that evaluates or partially evaluates function calls with constant arguments
// and substitutes them with Expr.Literal.  Column-dependent function calls
// (e.g. UPPER(word) where word is a column) emit an Expr.Literal(Null)
// placeholder; a future Level 2 pass can evaluate those inside the VM loop.
//
// Transaction semantics are delegated entirely to the InMemoryBackend, which
// supports snapshot-based begin/commit/rollback.  The Level 0 snapshot logic
// that lived in Connection is removed; the backend owns it.
//
// Parameter binding (qmark `?` substitution) and semicolon stripping are
// retained from Level 0 since those pre-process the raw SQL text before
// any parsing occurs.
//
// ── Security notes ────────────────────────────────────────────────────────
//
// * All parameters are passed as SqlValue.Literal nodes after binding, so
//   there is no string-interpolation SQL injection vector.
// * Query-text length is bounded: the connection rejects SQL strings longer
//   than 1 MB (1_048_576 characters) as a DoS defence.
// * Recursion depth inside the expression parser is bounded at 64 levels.
// * The VM itself defends against runaway loops via the InMemoryBackend
//   row-count ceiling (no special tuning needed at this level).

#nowarn "3261" // nullness warnings managed explicitly
#nowarn "3264" // nullness downcast

namespace CodingAdventures.MiniSqlite.FSharp

open System
open System.Collections.Generic
open System.Globalization
open System.Text
open System.Text.RegularExpressions

open CodingAdventures.SqlPlanner.FSharp
open CodingAdventures.SqlOptimizer.FSharp
open CodingAdventures.SqlCodegen.FSharp
open CodingAdventures.SqlVm.FSharp
open CodingAdventures.SqlBackend.FSharp

// Alias to resolve ColumnDef ambiguity between SqlPlanner and SqlBackend.
// SqlPlanner.ColumnDef is a record; SqlBackend.ColumnDef is a class.
// The Statement DU uses SqlPlanner.ColumnDef.  Backend operations use
// CodingAdventures.SqlBackend.FSharp.ColumnDef (the class).
// ISchemaProvider also exists in both — Planner.plan expects SqlPlanner.ISchemaProvider.
type private PlannerColumnDef   = CodingAdventures.SqlPlanner.FSharp.ColumnDef
type private PlannerSchemaProvider = CodingAdventures.SqlPlanner.FSharp.ISchemaProvider

// ── Public types ───────────────────────────────────────────────────────────

/// Column metadata returned by SELECT cursors.
type Column = { Name: string }

/// Exception raised by the mini-sqlite facade.
type MiniSqliteException(kind: string, message: string) =
    inherit Exception(message)
    member _.Kind = kind

/// Connection options for the in-memory mini-sqlite facade.
type ConnectionOptions =
    { Autocommit: bool }
    static member Default = { Autocommit = false }

// ── Internal execution result ──────────────────────────────────────────────

type internal ExecutionResult =
    { Columns: string list
      Rows: IReadOnlyList<obj> list
      RowCount: int
      LastRowId: obj }

module private ExecutionResult =
    let empty rowCount =
        { Columns = []
          Rows = []
          RowCount = rowCount
          LastRowId = box (null: string) }

// ── SQL pre-processing helpers ─────────────────────────────────────────────
//
// These utilities operate on raw SQL text before the Statement parser sees it.
// They handle:
//   * Semicolon stripping
//   * First-keyword extraction (for routing)
//   * qmark parameter binding
//   * Text-level top-level splitting (used by the text-to-Statement parser)

module private SqlText =
    let private invariant = CultureInfo.InvariantCulture

    /// Remove a trailing semicolon and surrounding whitespace.
    let trim (sql: string) =
        let trimmed = sql.Trim()
        if trimmed.EndsWith(";", StringComparison.Ordinal) then
            trimmed.Substring(0, trimmed.Length - 1).Trim()
        else
            trimmed

    /// Return the first keyword of `sql` in upper-case (e.g. "SELECT").
    let firstKeyword (sql: string) =
        let m = Regex.Match(sql.TrimStart(), "^[A-Za-z]+")
        if m.Success then m.Value.ToUpperInvariant() else ""

    let private isIdentifierChar ch = Char.IsLetterOrDigit(ch) || ch = '_'

    /// Split `text` at unquoted, non-nested occurrences of `separator`.
    let splitTopLevel (separator: char) (text: string) =
        let parts = ResizeArray<string>()
        let mutable start = 0
        let mutable depth = 0
        let mutable quote = '\000'
        let mutable i = 0
        while i < text.Length do
            let ch = text[i]
            if quote <> '\000' then
                if ch = quote then
                    if i + 1 < text.Length && text[i + 1] = quote then i <- i + 1
                    else quote <- '\000'
            else
                match ch with
                | '\'' | '"' -> quote <- ch
                | '('        -> depth <- depth + 1
                | ')' when depth > 0 -> depth <- depth - 1
                | _ when ch = separator && depth = 0 ->
                    parts.Add(text.Substring(start, i - start).Trim())
                    start <- i + 1
                | _ -> ()
            i <- i + 1
        parts.Add(text.Substring(start).Trim())
        parts |> Seq.filter (String.IsNullOrWhiteSpace >> not) |> Seq.toList

    /// Split `text` at unquoted, non-nested occurrences of `keyword`.
    let splitByKeyword (keyword: string) (text: string) =
        let parts = ResizeArray<string>()
        let mutable start = 0
        let mutable depth = 0
        let mutable quote = '\000'
        let mutable i = 0
        let matchesAt index =
            index + keyword.Length <= text.Length
            && String.Compare(text, index, keyword, 0, keyword.Length, true, invariant) = 0
            && (index = 0 || not (isIdentifierChar text[index - 1]))
            && (index + keyword.Length = text.Length || not (isIdentifierChar text[index + keyword.Length]))
        while i < text.Length do
            let ch = text[i]
            if quote <> '\000' then
                if ch = quote then
                    if i + 1 < text.Length && text[i + 1] = quote then i <- i + 1
                    else quote <- '\000'
            else
                match ch with
                | '\'' | '"' -> quote <- ch
                | '('        -> depth <- depth + 1
                | ')' when depth > 0 -> depth <- depth - 1
                | _ when depth = 0 && matchesAt i ->
                    parts.Add(text.Substring(start, i - start).Trim())
                    i <- i + keyword.Length - 1
                    start <- i + 1
                | _ -> ()
            i <- i + 1
        parts.Add(text.Substring(start).Trim())
        parts |> Seq.filter (String.IsNullOrWhiteSpace >> not) |> Seq.toList

    let private formatParameter (value: obj) =
        match value with
        | null -> "NULL"
        | :? string as s -> "'" + s.Replace("'", "''") + "'"
        | :? bool   as b -> if b then "TRUE" else "FALSE"
        | :? IFormattable as f -> f.ToString(null, invariant)
        | other -> "'" + other.ToString().Replace("'", "''") + "'"

    /// Substitute qmark `?` placeholders in `sql` with the given parameters.
    /// Raises MiniSqliteException("ProgrammingError", …) on count mismatch.
    let bindParameters (sql: string) (parameters: IReadOnlyList<obj>) =
        let output = StringBuilder()
        let mutable paramIndex = 0
        let mutable quote = '\000'
        let mutable i = 0
        while i < sql.Length do
            let ch = sql[i]
            if quote <> '\000' then
                output.Append(ch) |> ignore
                if ch = quote then
                    if i + 1 < sql.Length && sql[i + 1] = quote then
                        i <- i + 1
                        output.Append(sql[i]) |> ignore
                    else quote <- '\000'
            elif ch = '\'' || ch = '"' then
                quote <- ch
                output.Append(ch) |> ignore
            elif ch = '-' && i + 1 < sql.Length && sql[i + 1] = '-' then
                while i < sql.Length && sql[i] <> '\n' do
                    output.Append(sql[i]) |> ignore
                    i <- i + 1
                if i < sql.Length then output.Append(sql[i]) |> ignore
            elif ch = '/' && i + 1 < sql.Length && sql[i + 1] = '*' then
                output.Append("/*") |> ignore
                i <- i + 2
                while i + 1 < sql.Length && not (sql[i] = '*' && sql[i + 1] = '/') do
                    output.Append(sql[i]) |> ignore
                    i <- i + 1
                if i + 1 < sql.Length then
                    output.Append("*/") |> ignore
                    i <- i + 1
            elif ch = '?' then
                if paramIndex >= parameters.Count then
                    raise (MiniSqliteException("ProgrammingError", "not enough query parameters"))
                output.Append(formatParameter parameters[paramIndex]) |> ignore
                paramIndex <- paramIndex + 1
            else
                output.Append(ch) |> ignore
            i <- i + 1
        if paramIndex <> parameters.Count then
            raise (MiniSqliteException("ProgrammingError", "too many query parameters"))
        output.ToString()

// ── Expression parser ──────────────────────────────────────────────────────
//
// Recursive-descent parser for SQL scalar expressions.  The grammar (informal):
//
//   expr        ::= or_expr
//   or_expr     ::= and_expr (OR and_expr)*
//   and_expr    ::= not_expr (AND not_expr)*
//   not_expr    ::= NOT not_expr | compare_expr
//   compare_expr::= add_expr ((= | <> | != | < | <= | > | >=) add_expr)*
//                 | add_expr IS [NOT] NULL
//                 | add_expr [NOT] BETWEEN add_expr AND add_expr
//                 | add_expr [NOT] IN ( expr_list )
//                 | add_expr [NOT] LIKE string_literal
//   add_expr    ::= mul_expr ((+ | - | ||) mul_expr)*
//   mul_expr    ::= unary_expr ((* | / | %) unary_expr)*
//   unary_expr  ::= - unary_expr | primary
//   primary     ::= literal | column | func_call | ( expr )
//   func_call   ::= NAME ( expr_list )
//
// Recursion depth is bounded at 64 via a counter passed through each call.
// The parser works on a (text, offset) pair threaded through closures.

module private ExprParser =
    let private ci (a: string) (b: string) =
        String.Compare(a, b, StringComparison.OrdinalIgnoreCase) = 0

    let private MAX_DEPTH = 64

    /// Tokenise `text` into a list of (token_string, start_offset) pairs,
    /// stripping comments and collapsing whitespace.  Quoted strings and
    /// identifiers are preserved verbatim.
    let tokenize (text: string) : (string * int) list =
        let tokens = ResizeArray<string * int>()
        let mutable i = 0
        let len = text.Length

        while i < len do
            // Skip whitespace
            while i < len && Char.IsWhiteSpace(text[i]) do i <- i + 1
            if i >= len then ()
            else
                let ch = text[i]

                // Skip -- comments
                if ch = '-' && i + 1 < len && text[i + 1] = '-' then
                    while i < len && text[i] <> '\n' do i <- i + 1

                // Skip /* … */ comments
                elif ch = '/' && i + 1 < len && text[i + 1] = '*' then
                    i <- i + 2
                    while i + 1 < len && not (text[i] = '*' && text[i + 1] = '/') do i <- i + 1
                    if i + 1 < len then i <- i + 2

                // Quoted string literal
                elif ch = '\'' then
                    let start = i
                    i <- i + 1
                    while i < len && not (text[i] = '\'' && (i + 1 >= len || text[i + 1] <> '\'')) do
                        if i < len && text[i] = '\'' && i + 1 < len && text[i + 1] = '\'' then
                            i <- i + 2
                        else
                            i <- i + 1
                    if i < len then i <- i + 1
                    tokens.Add(text.Substring(start, i - start), start)

                // Quoted identifier "…"
                elif ch = '"' then
                    let start = i
                    i <- i + 1
                    while i < len && text[i] <> '"' do i <- i + 1
                    if i < len then i <- i + 1
                    // Strip quotes; treat as bare identifier
                    let raw = text.Substring(start, i - start)
                    let inner = raw.Substring(1, raw.Length - 2)
                    tokens.Add(inner, start)

                // Two-character operators
                elif i + 1 < len && (ch = '<' || ch = '>' || ch = '!' || ch = '|') then
                    let two = text.Substring(i, 2)
                    if two = "<=" || two = ">=" || two = "<>" || two = "!=" || two = "||" then
                        tokens.Add(two, i)
                        i <- i + 2
                    else
                        tokens.Add(string ch, i)
                        i <- i + 1

                // Single-character tokens: = < > ( ) , * + - / % .
                elif "=<>(),*+-/%." |> Seq.contains ch then
                    tokens.Add(string ch, i)
                    i <- i + 1

                // Numeric literal
                elif Char.IsDigit(ch) then
                    let start = i
                    while i < len && (Char.IsDigit(text[i]) || text[i] = '.') do i <- i + 1
                    // Optional exponent
                    if i < len && (text[i] = 'e' || text[i] = 'E') then
                        i <- i + 1
                        if i < len && (text[i] = '+' || text[i] = '-') then i <- i + 1
                        while i < len && Char.IsDigit(text[i]) do i <- i + 1
                    tokens.Add(text.Substring(start, i - start), start)

                // Identifier or keyword
                elif Char.IsLetter(ch) || ch = '_' then
                    let start = i
                    while i < len && (Char.IsLetterOrDigit(text[i]) || text[i] = '_') do i <- i + 1
                    tokens.Add(text.Substring(start, i - start), start)

                else
                    i <- i + 1

        tokens |> Seq.toList

    // Parser state: a reference to the current token position.
    // We use a ref cell so the recursive descent functions can advance it.

    type private State = { Tokens: (string * int) array; mutable Pos: int }

    let private peek (s: State) =
        if s.Pos < s.Tokens.Length then fst s.Tokens.[s.Pos] else ""

    let private advance (s: State) =
        let tok = peek s
        s.Pos <- s.Pos + 1
        tok

    let private consume (expected: string) (s: State) =
        let tok = advance s
        if not (ci tok expected) then
            failwithf "expected '%s' but got '%s'" expected tok

    let private isKeyword (tok: string) =
        let kws = [| "SELECT";"FROM";"WHERE";"ORDER";"BY";"GROUP";"HAVING"
                     "INSERT";"INTO";"VALUES";"UPDATE";"SET";"DELETE";"FROM"
                     "CREATE";"TABLE";"DROP";"AS";"AND";"OR";"NOT";"IS";"NULL"
                     "TRUE";"FALSE";"BETWEEN";"IN";"LIKE";"DISTINCT";"JOIN"
                     "INNER";"LEFT";"RIGHT";"FULL";"CROSS";"ON";"LIMIT";"OFFSET"
                     "ASC";"DESC";"NULLS";"FIRST";"LAST";"IF";"EXISTS";"ALL"
                     "BEGIN";"COMMIT";"ROLLBACK";"TRANSACTION";"UNION";"COUNT"
                     "SUM";"AVG";"MIN";"MAX";"END" |]
        kws |> Array.exists (ci tok)

    let private isIdentifier (tok: string) =
        tok.Length > 0 && (Char.IsLetter(tok[0]) || tok[0] = '_') && not (isKeyword tok)

    let private isIdentifierOrKeyword (tok: string) =
        tok.Length > 0 && (Char.IsLetter(tok[0]) || tok[0] = '_')

    let private parseLiteralValue (tok: string) : SqlValue =
        if tok.StartsWith("'") then
            let inner = tok.Substring(1, tok.Length - 2).Replace("''", "'")
            SqlValue.Text inner
        elif ci tok "NULL"  then SqlValue.Null
        elif ci tok "TRUE"  then SqlValue.Bool true
        elif ci tok "FALSE" then SqlValue.Bool false
        else
            let mutable iv = 0L
            let mutable dv = 0.0
            if Int64.TryParse(tok, NumberStyles.Integer, CultureInfo.InvariantCulture, &iv) then
                SqlValue.Integer iv
            elif Double.TryParse(tok, NumberStyles.Float, CultureInfo.InvariantCulture, &dv) then
                SqlValue.Real dv
            else SqlValue.Text tok

    /// Parse an aggregate function call: COUNT(*), SUM(expr), etc.
    let private tryParseAgg (name: string) (s: State) (parseExpr: State -> int -> Expr) (depth: int) : Expr option =
        let aggFn =
            match name.ToUpperInvariant() with
            | "COUNT" -> Some AggFunction.Count
            | "SUM"   -> Some AggFunction.Sum
            | "AVG"   -> Some AggFunction.Avg
            | "MIN"   -> Some AggFunction.Min
            | "MAX"   -> Some AggFunction.Max
            | _       -> None
        match aggFn with
        | None -> None
        | Some fn ->
            // distinct flag
            let distinct =
                if ci (peek s) "DISTINCT" then advance s |> ignore; true
                else false
            // argument: * or expr
            let arg =
                if peek s = "*" then
                    advance s |> ignore
                    AggArg.Star
                else
                    AggArg.Expr (parseExpr s (depth + 1))
            consume ")" s
            Some (Expr.AggExpr(fn, arg, distinct))

    /// Parse a primary expression: literal, column, func call, or parenthesised expr.
    let rec private parsePrimary (s: State) (depth: int) : Expr =
        if depth > MAX_DEPTH then failwith "expression too deeply nested"
        let tok = peek s
        if tok = "" then failwith "unexpected end of expression"

        // Parenthesised expression
        if tok = "(" then
            advance s |> ignore
            let inner = parseExpr s (depth + 1)
            consume ")" s
            inner

        // String literal
        elif tok.StartsWith("'") then
            advance s |> ignore
            Expr.Literal(parseLiteralValue tok)

        // Numeric literal
        elif tok.Length > 0 && (Char.IsDigit(tok[0]) || (tok[0] = '-' && tok.Length > 1 && Char.IsDigit(tok[1]))) then
            advance s |> ignore
            Expr.Literal(parseLiteralValue tok)

        // NULL / TRUE / FALSE
        elif ci tok "NULL" then
            advance s |> ignore
            Expr.Literal SqlValue.Null
        elif ci tok "TRUE" then
            advance s |> ignore
            Expr.Literal(SqlValue.Bool true)
        elif ci tok "FALSE" then
            advance s |> ignore
            Expr.Literal(SqlValue.Bool false)

        // CASE WHEN … THEN … [ELSE …] END  (simple passthrough as NULL for now)
        elif ci tok "CASE" then
            advance s |> ignore
            let mutable depth2 = 1
            while depth2 > 0 do
                let t2 = advance s
                if ci t2 "CASE" then depth2 <- depth2 + 1
                elif ci t2 "END" then depth2 <- depth2 - 1
            Expr.Literal SqlValue.Null

        // Identifier, keyword-as-identifier, or function call
        elif isIdentifierOrKeyword tok then
            advance s |> ignore
            // Check for table.column  (dot notation)
            let tableOpt, colName =
                if peek s = "." then
                    advance s |> ignore  // consume "."
                    let col = advance s
                    Some tok, col
                else
                    None, tok

            // If no dot was consumed, check for function call
            if tableOpt.IsNone && peek s = "(" then
                advance s |> ignore  // consume "("
                // Aggregate function?
                match tryParseAgg colName s parseExpr depth with
                | Some aggExpr -> aggExpr
                | None ->
                    // Scalar function call — parse arg list
                    let args = ResizeArray<Expr>()
                    if peek s <> ")" then
                        args.Add(parseExpr s (depth + 1))
                        while peek s = "," do
                            advance s |> ignore
                            args.Add(parseExpr s (depth + 1))
                    consume ")" s
                    Expr.FuncCall(colName.ToUpperInvariant(), args |> Seq.toList)
            else
                Expr.Column(tableOpt, colName)

        else
            // Fallback: treat as a literal text token
            advance s |> ignore
            Expr.Literal(SqlValue.Text tok)

    and private parseUnary (s: State) (depth: int) : Expr =
        if depth > MAX_DEPTH then failwith "expression too deeply nested"
        if peek s = "-" then
            advance s |> ignore
            let e = parseUnary s (depth + 1)
            Expr.UnaryOp(UnaryOperator.Neg, e)
        elif ci (peek s) "NOT" then
            advance s |> ignore
            let e = parseUnary s (depth + 1)
            Expr.UnaryOp(UnaryOperator.Not, e)
        else
            parsePrimary s depth

    and private parseMul (s: State) (depth: int) : Expr =
        if depth > MAX_DEPTH then failwith "expression too deeply nested"
        let mutable left = parseUnary s (depth + 1)
        let mutable loop = true
        while loop do
            let tok = peek s
            if tok = "*" || tok = "/" || tok = "%" then
                advance s |> ignore
                let right = parseUnary s (depth + 1)
                let op = match tok with "*" -> BinaryOperator.Mul | "/" -> BinaryOperator.Div | _ -> BinaryOperator.Mod
                left <- Expr.BinaryOp(op, left, right)
            else loop <- false
        left

    and private parseAdd (s: State) (depth: int) : Expr =
        if depth > MAX_DEPTH then failwith "expression too deeply nested"
        let mutable left = parseMul s (depth + 1)
        let mutable loop = true
        while loop do
            let tok = peek s
            if tok = "+" || tok = "-" || tok = "||" then
                advance s |> ignore
                let right = parseMul s (depth + 1)
                let op = match tok with "+" -> BinaryOperator.Add | "-" -> BinaryOperator.Sub | _ -> BinaryOperator.Add
                if tok = "||" then
                    // String concatenation operator.  Save the current left before updating
                    // so we can pass the correct operand to FuncCall.
                    let savedLeft = left
                    left <- Expr.FuncCall("__CONCAT__", [savedLeft; right])
                else
                    left <- Expr.BinaryOp(op, left, right)
            else loop <- false
        left

    and private parseCompare (s: State) (depth: int) : Expr =
        if depth > MAX_DEPTH then failwith "expression too deeply nested"
        let left = parseAdd s (depth + 1)

        let tok = peek s
        // IS [NOT] NULL
        if ci tok "IS" then
            advance s |> ignore
            let negate = ci (peek s) "NOT"
            if negate then advance s |> ignore
            consume "NULL" s
            if negate then Expr.IsNotNull left else Expr.IsNull left

        // [NOT] BETWEEN
        elif ci tok "NOT" && ci (fst (if s.Pos + 1 < s.Tokens.Length then s.Tokens.[s.Pos + 1] else ("", 0))) "BETWEEN" then
            advance s |> ignore  // NOT
            advance s |> ignore  // BETWEEN
            let lo = parseAdd s (depth + 1)
            consume "AND" s
            let hi = parseAdd s (depth + 1)
            Expr.UnaryOp(UnaryOperator.Not, Expr.Between(left, lo, hi))

        elif ci tok "BETWEEN" then
            advance s |> ignore
            let lo = parseAdd s (depth + 1)
            consume "AND" s
            let hi = parseAdd s (depth + 1)
            Expr.Between(left, lo, hi)

        // [NOT] IN ( … )
        elif ci tok "NOT" && s.Pos + 1 < s.Tokens.Length && ci (fst s.Tokens.[s.Pos + 1]) "IN" then
            advance s |> ignore  // NOT
            advance s |> ignore  // IN
            consume "(" s
            let items = ResizeArray<Expr>()
            if peek s <> ")" then
                items.Add(parseExpr s (depth + 1))
                while peek s = "," do
                    advance s |> ignore
                    items.Add(parseExpr s (depth + 1))
            consume ")" s
            Expr.NotIn(left, items |> Seq.toList)

        elif ci tok "IN" then
            advance s |> ignore
            consume "(" s
            let items = ResizeArray<Expr>()
            if peek s <> ")" then
                items.Add(parseExpr s (depth + 1))
                while peek s = "," do
                    advance s |> ignore
                    items.Add(parseExpr s (depth + 1))
            consume ")" s
            Expr.In(left, items |> Seq.toList)

        // [NOT] LIKE
        elif ci tok "NOT" && s.Pos + 1 < s.Tokens.Length && ci (fst s.Tokens.[s.Pos + 1]) "LIKE" then
            advance s |> ignore  // NOT
            advance s |> ignore  // LIKE
            let patternTok = advance s
            let pattern =
                if patternTok.StartsWith("'") then
                    patternTok.Substring(1, patternTok.Length - 2).Replace("''", "'")
                else patternTok
            Expr.NotLike(left, pattern)

        elif ci tok "LIKE" then
            advance s |> ignore
            let patternTok = advance s
            let pattern =
                if patternTok.StartsWith("'") then
                    patternTok.Substring(1, patternTok.Length - 2).Replace("''", "'")
                else patternTok
            Expr.Like(left, pattern)

        // Comparison operators
        elif tok = "=" || tok = "<>" || tok = "!=" || tok = "<" || tok = "<=" || tok = ">" || tok = ">=" then
            advance s |> ignore
            let right = parseAdd s (depth + 1)
            let op =
                match tok with
                | "=" -> BinaryOperator.Eq
                | "<>" | "!=" -> BinaryOperator.NotEq
                | "<"  -> BinaryOperator.Lt
                | "<=" -> BinaryOperator.Lte
                | ">"  -> BinaryOperator.Gt
                | ">=" -> BinaryOperator.Gte
                | _    -> BinaryOperator.Eq
            Expr.BinaryOp(op, left, right)
        else
            left

    and private parseAnd (s: State) (depth: int) : Expr =
        if depth > MAX_DEPTH then failwith "expression too deeply nested"
        let mutable left = parseCompare s (depth + 1)
        let mutable loop = true
        while loop do
            if ci (peek s) "AND" then
                advance s |> ignore
                let right = parseCompare s (depth + 1)
                left <- Expr.BinaryOp(BinaryOperator.And, left, right)
            else loop <- false
        left

    and private parseExpr (s: State) (depth: int) : Expr =
        if depth > MAX_DEPTH then failwith "expression too deeply nested"
        let mutable left = parseAnd s (depth + 1)
        let mutable loop = true
        while loop do
            if ci (peek s) "OR" then
                advance s |> ignore
                let right = parseAnd s (depth + 1)
                left <- Expr.BinaryOp(BinaryOperator.Or, left, right)
            else loop <- false
        left

    /// Parse a complete scalar expression from a text fragment.
    let parseExprText (text: string) : Expr =
        let tokens = tokenize text |> List.toArray
        let s = { Tokens = tokens; Pos = 0 }
        parseExpr s 0

// ── SQL text → Statement parser ────────────────────────────────────────────
//
// Converts raw (parameter-bound) SQL text into the Statement DU defined in
// sql-planner.  This is the replacement for the Level 0 regex engine.
//
// Supported statements:
//   CREATE TABLE name ( col_def, … )
//   DROP TABLE name
//   INSERT INTO name [(col, …)] VALUES (expr, …) [, (expr, …)]
//   UPDATE name SET col = expr [, …] WHERE …
//   DELETE FROM name WHERE …
//   SELECT … FROM … [JOIN …] [WHERE …] [GROUP BY …] [HAVING …]
//          [ORDER BY …] [LIMIT …] [OFFSET …]
//
// Transaction control words (BEGIN, COMMIT, ROLLBACK) are routed separately
// by the Connection.ExecuteBound method and never reach this parser.

module private SqlStatementParser =
    open ExprParser

    let private ci (a: string) (b: string) =
        String.Compare(a, b, StringComparison.OrdinalIgnoreCase) = 0

    /// Extract an unquoted identifier from `tok`, handling quoted identifiers.
    let private ident (tok: string) =
        if tok.StartsWith("\"") && tok.EndsWith("\"") then
            tok.Substring(1, tok.Length - 2)
        else tok

    /// Parse a column definition: name [type] [NOT NULL] [PRIMARY KEY] [UNIQUE] [DEFAULT expr]
    let private parseColumnDef (text: string) : PlannerColumnDef =
        let tokens = tokenize text |> List.toArray
        if tokens.Length = 0 then failwith "empty column definition"
        let name = ident (fst tokens.[0])
        let mutable typeName = "TEXT"
        let mutable notNull = false
        let mutable primaryKey = false
        let mutable unique = false
        let mutable defaultExpr : Expr option = None
        let mutable i = 1
        // Optional type name
        if i < tokens.Length && not (ci (fst tokens.[i]) "NOT")
                              && not (ci (fst tokens.[i]) "PRIMARY")
                              && not (ci (fst tokens.[i]) "UNIQUE")
                              && not (ci (fst tokens.[i]) "DEFAULT") then
            typeName <- (fst tokens.[i]).ToUpperInvariant()
            i <- i + 1
        // Constraints
        while i < tokens.Length do
            let tok = fst tokens.[i]
            if ci tok "NOT" && i + 1 < tokens.Length && ci (fst tokens.[i + 1]) "NULL" then
                notNull <- true; i <- i + 2
            elif ci tok "PRIMARY" && i + 1 < tokens.Length && ci (fst tokens.[i + 1]) "KEY" then
                primaryKey <- true; i <- i + 2
            elif ci tok "UNIQUE" then
                unique <- true; i <- i + 1
            elif ci tok "DEFAULT" then
                i <- i + 1
                // Collect remaining tokens as default expr
                let remaining = tokens.[i..] |> Array.map fst |> String.concat " "
                defaultExpr <- Some (parseExprText remaining)
                i <- tokens.Length
            else
                i <- i + 1
        { PlannerColumnDef.Name = name; TypeName = typeName
          NotNull = notNull; PrimaryKey = primaryKey; Unique = unique
          Default = defaultExpr }

    /// Parse a CREATE TABLE statement.
    let private parseCreate (sql: string) : Statement =
        // CREATE TABLE [IF NOT EXISTS] name ( col_defs )
        let m = Regex.Match(sql,
            @"^\s*CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)\s*$",
            RegexOptions.IgnoreCase ||| RegexOptions.Singleline)
        if not m.Success then failwith "could not parse CREATE TABLE"
        let tableName = m.Groups.[1].Value
        let ifNotExists = Regex.IsMatch(sql, @"\bIF\s+NOT\s+EXISTS\b", RegexOptions.IgnoreCase)
        let colDefs =
            SqlText.splitTopLevel ',' m.Groups.[2].Value
            |> List.map parseColumnDef
        Statement.CreateTable { Table = tableName; IfNotExists = ifNotExists; Columns = colDefs }

    /// Parse a DROP TABLE statement.
    let private parseDrop (sql: string) : Statement =
        let m = Regex.Match(sql,
            @"^\s*DROP\s+TABLE\s+(?:(IF)\s+(EXISTS)\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*$",
            RegexOptions.IgnoreCase)
        if not m.Success then failwith "could not parse DROP TABLE"
        let ifExists = m.Groups.[1].Success
        let tableName = m.Groups.[3].Value
        Statement.DropTable { Table = tableName; IfExists = ifExists }

    /// Parse an INSERT statement.
    let private parseInsert (sql: string) : Statement =
        // INSERT INTO name [(cols)] VALUES (vals) [, (vals) …]
        let m = Regex.Match(sql,
            @"^\s*INSERT\s+INTO\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*?)\))?\s+VALUES\s*(.*)\s*$",
            RegexOptions.IgnoreCase ||| RegexOptions.Singleline)
        if not m.Success then failwith "could not parse INSERT"
        let tableName = m.Groups.[1].Value
        let columns =
            if m.Groups.[2].Success then
                Some (SqlText.splitTopLevel ',' m.Groups.[2].Value
                      |> List.map (fun s -> s.Trim().Trim('"', '\'').Trim()))
            else None
        let valuesText = m.Groups.[3].Value.Trim()
        // Split multiple value tuples: (…), (…)
        let tupleRx = Regex(@"\(([^()]*)\)", RegexOptions.Singleline)
        let tupleMatches = tupleRx.Matches(valuesText)
        if tupleMatches.Count = 0 then failwith "could not parse INSERT VALUES"
        let valueLists =
            [ for mat in tupleMatches do
                yield SqlText.splitTopLevel ',' mat.Groups.[1].Value
                      |> List.map parseExprText ]
        Statement.Insert { Table = tableName; Columns = columns; Values = valueLists }

    /// Parse an UPDATE statement.
    let private parseUpdate (sql: string) : Statement =
        let m = Regex.Match(sql,
            @"^\s*UPDATE\s+([A-Za-z_][A-Za-z0-9_]*)\s+SET\s+(.+?)(?:\s+WHERE\s+(.+))?\s*$",
            RegexOptions.IgnoreCase ||| RegexOptions.Singleline)
        if not m.Success then failwith "could not parse UPDATE"
        let tableName = m.Groups.[1].Value
        let assignments =
            SqlText.splitTopLevel ',' m.Groups.[2].Value
            |> List.map (fun a ->
                let pieces = a.Split([|'='|], 2)
                if pieces.Length <> 2 then failwith "invalid SET assignment"
                let col = pieces.[0].Trim().Trim('"').Trim()
                let expr = parseExprText (pieces.[1].Trim())
                { Column = col; Value = expr })
        let wherePred =
            if m.Groups.[3].Success && m.Groups.[3].Value.Trim() <> "" then
                Some (parseExprText (m.Groups.[3].Value.Trim()))
            else None
        Statement.Update { Table = tableName; Assignments = assignments; Where = wherePred }

    /// Parse a DELETE statement.
    let private parseDelete (sql: string) : Statement =
        let m = Regex.Match(sql,
            @"^\s*DELETE\s+FROM\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s+WHERE\s+(.+))?\s*$",
            RegexOptions.IgnoreCase ||| RegexOptions.Singleline)
        if not m.Success then failwith "could not parse DELETE"
        let tableName = m.Groups.[1].Value
        let wherePred =
            if m.Groups.[2].Success && m.Groups.[2].Value.Trim() <> "" then
                Some (parseExprText (m.Groups.[2].Value.Trim()))
            else None
        Statement.Delete { Table = tableName; Where = wherePred }

    /// Parse sort keys from an ORDER BY clause text.
    let private parseSortKeys (orderText: string) : SortKey list =
        SqlText.splitTopLevel ',' orderText
        |> List.map (fun part ->
            let tokens = part.Trim().Split([|' '; '\t'|], StringSplitOptions.RemoveEmptyEntries)
            let exprTokens, dirTokens =
                if tokens.Length > 1 &&
                   (ci tokens.[tokens.Length - 1] "ASC" || ci tokens.[tokens.Length - 1] "DESC") then
                    tokens.[..tokens.Length - 2], [| tokens.[tokens.Length - 1] |]
                else tokens, [||]
            let exprText = String.concat " " exprTokens
            let direction =
                if dirTokens.Length > 0 && ci dirTokens.[0] "DESC" then SortDir.Desc else SortDir.Asc
            // SQLite NULL-sort rule: NULLs are less than every other value.
            //   ASC  → NULLs appear first  (NullsFirst)
            //   DESC → NULLs appear last   (NullsLast)
            let nullOrder =
                match direction with
                | SortDir.Asc  -> NullOrder.NullsFirst
                | SortDir.Desc -> NullOrder.NullsLast
            { KeyExpr = parseExprText exprText
              Direction = direction
              NullOrder = nullOrder })

    /// Parse a SELECT output column: expr [AS alias] | *
    let private parseOutputColumn (text: string) : OutputColumn =
        let trimmed = text.Trim()
        if trimmed = "*" then OutputColumn.Star
        else
            // Check for AS alias at the top level
            let tokens = tokenize trimmed |> List.toArray
            // Find the LAST top-level AS keyword (avoid matching AS inside nested exprs)
            let asIdx =
                let mutable found = -1
                let mutable depth = 0
                for i in 0 .. tokens.Length - 1 do
                    let t = fst tokens.[i]
                    if t = "(" then depth <- depth + 1
                    elif t = ")" then depth <- depth - 1
                    elif depth = 0 && ci t "AS" then found <- i
                found
            if asIdx > 0 && asIdx < tokens.Length - 1 then
                let exprText = tokens.[..asIdx - 1] |> Array.map fst |> String.concat " "
                let alias = ident (fst tokens.[asIdx + 1])
                OutputColumn.Expr(parseExprText exprText, Some alias)
            else
                OutputColumn.Expr(parseExprText trimmed, None)

    /// Parse a SELECT statement.
    let private parseSelect (sql: string) : Statement =
        // We parse the SELECT by splitting the main clauses using regex.
        // Order is: SELECT cols FROM table [JOIN …] [WHERE …] [GROUP BY …]
        //           [HAVING …] [ORDER BY …] [LIMIT …] [OFFSET …]
        //
        // FROM is optional: `SELECT expr AS alias` (no FROM) is valid SQL
        // and is used for constant expressions and scalar function calls.

        // Check for FROM-less SELECT first.
        let hasFrom = Regex.IsMatch(sql, @"(?i)\bFROM\b")

        let colsText, rest =
            if hasFrom then
                let m = Regex.Match(sql,
                    @"^\s*SELECT\s+(.*?)\s+FROM\s+(.*?)\s*$",
                    RegexOptions.IgnoreCase ||| RegexOptions.Singleline)
                if not m.Success then failwith "could not parse SELECT"
                m.Groups.[1].Value.Trim(), m.Groups.[2].Value.Trim()
            else
                // FROM-less: everything after SELECT is the column list.
                let m = Regex.Match(sql, @"^\s*SELECT\s+(.+)$", RegexOptions.IgnoreCase ||| RegexOptions.Singleline)
                if not m.Success then failwith "could not parse SELECT"
                m.Groups.[1].Value.Trim(), ""

        // Peel OFFSET … from the end
        let offsetOpt, rest1 =
            let om = Regex.Match(rest, @"(?is)\s+OFFSET\s+(\d+)\s*$")
            if om.Success then
                Some (int64 om.Groups.[1].Value),
                rest.Substring(0, om.Index).Trim()
            else None, rest

        // Peel LIMIT … from the end.
        // SQLite semantics: LIMIT -1 means "all rows", which we model as None (no limit).
        let limitOpt, rest2 =
            let lm = Regex.Match(rest1, @"(?is)\s+LIMIT\s+(-?\d+)\s*$")
            if lm.Success then
                let n = int64 lm.Groups.[1].Value
                // -1 (or any negative) = no limit in SQLite
                let limitVal = if n < 0L then None else Some n
                limitVal, rest1.Substring(0, lm.Index).Trim()
            else None, rest1

        // Peel ORDER BY … from the end
        let orderKeys, rest3 =
            let om = Regex.Match(rest2, @"(?is)\s+ORDER\s+BY\s+(.+)$")
            if om.Success then
                parseSortKeys om.Groups.[1].Value,
                rest2.Substring(0, om.Index).Trim()
            else [], rest2

        // Peel HAVING … from the end
        let havingOpt, rest4 =
            let hm = Regex.Match(rest3, @"(?is)\s+HAVING\s+(.+)$")
            if hm.Success then
                Some (parseExprText hm.Groups.[1].Value),
                rest3.Substring(0, hm.Index).Trim()
            else None, rest3

        // Peel GROUP BY … from the end
        let groupByExprs, rest5 =
            let gm = Regex.Match(rest4, @"(?is)\s+GROUP\s+BY\s+(.+)$")
            if gm.Success then
                let exprs = SqlText.splitTopLevel ',' gm.Groups.[1].Value |> List.map parseExprText
                exprs, rest4.Substring(0, gm.Index).Trim()
            else [], rest4

        // Peel WHERE … from the end (careful: WHERE must precede JOIN ON)
        let whereOpt, rest6 =
            // Find WHERE that's not inside a JOIN ON clause
            let wm = Regex.Match(rest5, @"(?is)^(.*?)\s+WHERE\s+(.+)$")
            if wm.Success then
                Some (parseExprText wm.Groups.[2].Value),
                wm.Groups.[1].Value.Trim()
            else None, rest5

        // Parse FROM + JOIN clauses from rest6
        // Split on JOIN keywords to get FROM and each join
        let fromAndJoins = Regex.Split(rest6, @"\s+(?:INNER\s+|LEFT\s+|RIGHT\s+|FULL\s+|CROSS\s+)?JOIN\s+", RegexOptions.IgnoreCase)
        let joinKeywordsRe = Regex(@"\s+((?:INNER|LEFT|RIGHT|FULL|CROSS)\s+)?JOIN\s+", RegexOptions.IgnoreCase)
        let joinKindMatches = joinKeywordsRe.Matches(rest6)

        let fromList =
            let fromPart = fromAndJoins.[0].Trim()
            if fromPart = "" then []
            else
            // Can be  "table [AS alias], table2 [AS alias2]"
            SqlText.splitTopLevel ',' fromPart
            |> List.map (fun t ->
                let parts = t.Trim().Split([|' '; '\t'|], StringSplitOptions.RemoveEmptyEntries)
                if parts.Length >= 2 && ci parts.[1] "AS" && parts.Length >= 3 then
                    parts.[0], Some parts.[2]
                elif parts.Length >= 2 && not (ci parts.[1] "AS") then
                    parts.[0], Some parts.[1]
                else
                    parts.[0], None)

        let joins =
            if fromAndJoins.Length <= 1 then []
            else
                [ for i in 1 .. fromAndJoins.Length - 1 do
                    let joinPart = fromAndJoins.[i].Trim()
                    // Parse: tableName [AS alias] ON condition
                    let onM = Regex.Match(joinPart, @"^(\S+)(?:\s+(?:AS\s+)?(\S+))?\s+ON\s+(.+)$", RegexOptions.IgnoreCase ||| RegexOptions.Singleline)
                    let crossM = Regex.Match(joinPart, @"^(\S+)(?:\s+(?:AS\s+)?(\S+))?\s*$", RegexOptions.IgnoreCase)
                    let joinKind =
                        if i - 1 < joinKindMatches.Count then
                            let kindText = (joinKindMatches.[i - 1].Groups.[1].Value.Trim()).ToUpperInvariant()
                            match kindText with
                            | k when k.StartsWith("LEFT") -> JoinKind.Left
                            | k when k.StartsWith("RIGHT") -> JoinKind.Right
                            | k when k.StartsWith("FULL") -> JoinKind.Full
                            | k when k.StartsWith("CROSS") -> JoinKind.Cross
                            | _ -> JoinKind.Inner
                        else JoinKind.Inner
                    if onM.Success then
                        let table = onM.Groups.[1].Value
                        let alias = if onM.Groups.[2].Success then Some onM.Groups.[2].Value else None
                        let cond = parseExprText onM.Groups.[3].Value
                        yield { Kind = joinKind; Table = table; Alias = alias; On = Some cond }
                    elif crossM.Success then
                        let table = crossM.Groups.[1].Value
                        let alias = if crossM.Groups.[2].Success then Some crossM.Groups.[2].Value else None
                        yield { Kind = joinKind; Table = table; Alias = alias; On = None } ]

        // Parse output columns; handle DISTINCT
        let distinct = Regex.IsMatch(colsText, @"^\s*DISTINCT\s+", RegexOptions.IgnoreCase)
        let colsText2 = if distinct then Regex.Replace(colsText, @"^\s*DISTINCT\s+", "", RegexOptions.IgnoreCase) else colsText

        let outputCols =
            SqlText.splitTopLevel ',' colsText2
            |> List.map parseOutputColumn

        // ── ORDER BY with non-projected columns ────────────────────────────
        // SQLite allows ORDER BY to reference source table columns that are not
        // in the SELECT list.  The VM sorts by output column name, so we must
        // include those extra columns in the projection.
        //
        // For each sort key column (simple Expr.Column) not already present in
        // the SELECT list, we add a hidden extra output column named
        // "__sort_N__" (N = 0-based index).  The Level1Engine.execute function
        // strips these hidden columns from the final QueryResult.
        //
        // We only do this for non-aggregate, non-star queries.
        let isStar = outputCols |> List.exists (fun oc -> match oc with OutputColumn.Star -> true | _ -> false)

        let colNamesInSelect =
            if isStar then Set.empty
            else
                outputCols
                |> List.choose (fun oc ->
                    match oc with
                    | OutputColumn.Expr(_, Some alias) -> Some (alias.ToLowerInvariant())
                    | OutputColumn.Expr(Expr.Column(_, c), None) -> Some (c.ToLowerInvariant())
                    | _ -> None)
                |> Set.ofList

        // Augment the SELECT with hidden sort columns (named __sort_N__) and
        // rewrite the corresponding sort key expressions to reference the hidden
        // alias, so the VM's output-column-based sort can find the value.
        // Level1Engine.execute strips these hidden columns from the final output.
        let augmentedCols, remappedOrderKeys =
            if isStar || orderKeys.IsEmpty then
                outputCols, orderKeys
            else
                let extras = ResizeArray<OutputColumn>()
                let newKeys =
                    orderKeys |> List.mapi (fun i key ->
                        match key.KeyExpr with
                        | Expr.Column(_, c) when not (colNamesInSelect.Contains(c.ToLowerInvariant())) ->
                            let hiddenName = $"__sort_{i}__"
                            extras.Add(OutputColumn.Expr(key.KeyExpr, Some hiddenName))
                            // Rewrite the sort key to reference the hidden output column.
                            { key with KeyExpr = Expr.Column(None, hiddenName) }
                        | _ -> key)
                outputCols @ (extras |> Seq.toList), newKeys

        // ── Function-argument source columns ──────────────────────────────
        // Scalar function calls in the SELECT list (e.g. LOWER(word)) need the
        // source column values at evaluation time.  The VM only emits projected
        // columns; it stubs FuncCall with Null.  We post-process in Level1Engine
        // using FuncEval.evalExpr with a row-value map, but the source column
        // ("word") must be present in that map.
        //
        // Solution: add hidden columns __farg_COLNAME__ = COLNAME for any column
        // that appears inside a function call argument but is not already
        // visible in the SELECT output.  Level1Engine strips these before returning.
        // The rowMap in applyFuncCalls maps "__farg_COLNAME__" → "COLNAME" so
        // evalExpr can look up the column by its original name.
        let rec collectColsInFuncArgs (e: Expr) : string list =
            match e with
            | Expr.FuncCall(_, args) ->
                args |> List.collect (fun a ->
                    match a with
                    | Expr.Column(_, c) -> [c]
                    | other -> collectColsInFuncArgs other)
            | Expr.BinaryOp(_, l, r) -> collectColsInFuncArgs l @ collectColsInFuncArgs r
            | Expr.UnaryOp(_, inner) -> collectColsInFuncArgs inner
            | _ -> []

        // Names already in the output after sort augmentation.
        let colNamesAfterSort =
            augmentedCols
            |> List.choose (fun oc ->
                match oc with
                | OutputColumn.Expr(_, Some alias) -> Some (alias.ToLowerInvariant())
                | OutputColumn.Expr(Expr.Column(_, c), None) -> Some (c.ToLowerInvariant())
                | _ -> None)
            |> Set.ofList

        let finalCols =
            if isStar || fromList.IsEmpty then
                augmentedCols
            else
                let needed =
                    outputCols
                    |> List.collect (fun oc ->
                        match oc with
                        | OutputColumn.Expr(e, _) -> collectColsInFuncArgs e
                        | _ -> [])
                    |> List.distinct
                    |> List.filter (fun c -> not (colNamesAfterSort.Contains(c.ToLowerInvariant())))

                let extras =
                    needed |> List.map (fun c ->
                        let hiddenName = $"__farg_{c.ToLowerInvariant()}__"
                        OutputColumn.Expr(Expr.Column(None, c), Some hiddenName))

                augmentedCols @ extras

        let limitClause =
            match limitOpt, offsetOpt with
            | None, None -> None
            | _ -> Some { Count = limitOpt; Offset = offsetOpt }

        Statement.Select
            { Distinct = distinct
              Columns  = finalCols
              From     = fromList
              Joins    = joins
              Where    = whereOpt
              GroupBy  = groupByExprs
              Having   = havingOpt
              OrderBy  = remappedOrderKeys
              Limit    = limitClause }

    /// Parse a SQL statement from pre-processed (parameter-bound) text.
    let parse (sql: string) : Statement =
        let trimmed = SqlText.trim sql
        match SqlText.firstKeyword trimmed with
        | "CREATE" -> parseCreate trimmed
        | "DROP"   -> parseDrop trimmed
        | "INSERT" -> parseInsert trimmed
        | "UPDATE" -> parseUpdate trimmed
        | "DELETE" -> parseDelete trimmed
        | "SELECT" -> parseSelect trimmed
        | other    -> failwithf "unsupported SQL statement: %s" other

// ── Schema provider from InMemoryBackend ──────────────────────────────────
//
// The Planner needs to know the columns of each table to resolve
// column references.  We wrap the InMemoryBackend to provide this.

module private Schema =
    /// Build an ISchemaProvider (SqlPlanner's version) from an InMemoryBackend.
    let fromBackend (backend: InMemoryBackend) : PlannerSchemaProvider =
        { new PlannerSchemaProvider with
            member _.Columns(table: string) =
                try
                    let cols = backend.Columns(table) |> Seq.map (fun c -> c.Name) |> Seq.toList
                    Ok cols
                with
                | :? TableNotFound -> Error (PlanError.UnknownTable table)
                | ex -> Error (PlanError.InternalError ex.Message) }

// ── Scalar function evaluator ──────────────────────────────────────────────
//
// The codegen stubs FuncCall nodes to NULL.  For Level 1 we handle scalar
// functions by rewriting the OptimizedPlan: FuncCall nodes that can be
// evaluated row-independently (constant arguments or column-only arguments)
// are left for the VM's evalExpr; the VM's evalExpr currently returns NULL
// for all FuncCall nodes, so we need to intercept before the VM.
//
// Strategy: we don't modify the vm.  Instead, for queries that contain
// scalar function calls, we use a "function-resolving VM" — a small
// additional post-processor that walks QueryResult rows and re-evaluates
// function calls using the original plan's FuncCall nodes.
//
// Actually, the cleanest approach for Level 1 is to lower FuncCall nodes
// to built-in IL before they reach the codegen.  We add a "function
// lowering" pass that runs on the OptimizedPlan and replaces known
// FuncCall nodes with their equivalent using existing expression forms.
//
// For functions that cannot be lowered (column-dependent at plan time),
// we emit them inline in the scan body by tracking them through a special
// FuncCallRewriter that substitutes column values at runtime.
//
// Implementation: since F# sql-codegen stubs FuncCall → NULL, we need to
// evaluate FuncCall at the point where we have the actual row values.
// The simplest way to support this in Level 1 without modifying sql-codegen
// is to intercept the OptimizedPlan and add a "row transformer" step that
// patches output columns.
//
// However, this becomes complex.  A pragmatic Level 1 approach:
//   For constant-argument calls (no column refs): evaluate at plan time.
//   For column-dependent calls: replace with a FuncCall placeholder that
//     the MiniSqlite facade evaluates after the VM returns results.
//
// We implement a two-pass approach:
//   1. FuncFolder — walks the OptimizedPlan and evaluates constant FuncCalls
//      into Expr.Literal, passes column-dependent ones through unchanged.
//   2. FuncApplier — after the VM runs, for any output column whose plan
//      expression is a FuncCall (directly or nested), we re-evaluate it
//      against each row using a mini expression evaluator.
//
// To make FuncApplier work we need to track which output columns are
// function-call expressions.  We do this by maintaining a parallel list
// of (column_name, expr_or_none) where expr is present when that column
// needs function evaluation.
//
// For Level 1 we support:
//   LENGTH(x), UPPER(x), LOWER(x), SUBSTR(x,n[,len]), TRIM(x), LTRIM(x),
//   RTRIM(x), REPLACE(x,y,z), ABS(x), ROUND(x[,n]),
//   __CONCAT__(x, y)  (the || operator)
//
// Column-dependent function calls in WHERE/HAVING are not yet supported
// at this Level and will return NULL (consistent with codegen stub behaviour).
// SELECT-projection function calls over column values ARE supported.

module private FuncEval =
    /// Evaluate a SqlValue list as function arguments and compute the result.
    /// Returns SqlValue.Null for unknown functions or wrong arg types.
    let evalBuiltin (name: string) (args: SqlValue list) : SqlValue =
        let ci (a: string) (b: string) = String.Compare(a, b, StringComparison.OrdinalIgnoreCase) = 0
        match name.ToUpperInvariant(), args with
        | "LENGTH", [SqlValue.Text s] -> SqlValue.Integer (int64 s.Length)
        | "LENGTH", [SqlValue.Null]   -> SqlValue.Null
        | "LENGTH", [_]               -> SqlValue.Null

        | "UPPER",  [SqlValue.Text s] -> SqlValue.Text (s.ToUpperInvariant())
        | "UPPER",  [SqlValue.Null]   -> SqlValue.Null
        | "LOWER",  [SqlValue.Text s] -> SqlValue.Text (s.ToLowerInvariant())
        | "LOWER",  [SqlValue.Null]   -> SqlValue.Null

        | "TRIM",   [SqlValue.Text s] -> SqlValue.Text (s.Trim())
        | "LTRIM",  [SqlValue.Text s] -> SqlValue.Text (s.TrimStart())
        | "RTRIM",  [SqlValue.Text s] -> SqlValue.Text (s.TrimEnd())
        | ("TRIM"|"LTRIM"|"RTRIM"), [SqlValue.Null] -> SqlValue.Null

        | "SUBSTR", [SqlValue.Text s; SqlValue.Integer start] ->
            // 1-indexed; negative start counts from end
            let len = s.Length
            let s1 =
                if start > 0L then int start - 1
                elif start < 0L then max 0 (len + int start)
                else 0
            if s1 >= len then SqlValue.Text ""
            else SqlValue.Text (s.Substring(s1))
        | "SUBSTR", [SqlValue.Text s; SqlValue.Integer start; SqlValue.Integer length] ->
            let len = s.Length
            let s1 =
                if start > 0L then int start - 1
                elif start < 0L then max 0 (len + int start)
                else 0
            let take = max 0 (int length)
            if s1 >= len then SqlValue.Text ""
            else SqlValue.Text (s.Substring(s1, min take (len - s1)))
        | "SUBSTR", _args when _args |> List.exists (fun a -> a = SqlValue.Null) -> SqlValue.Null

        | "REPLACE", [SqlValue.Text s; SqlValue.Text oldV; SqlValue.Text newV] ->
            SqlValue.Text (s.Replace(oldV, newV))
        | "REPLACE", _ -> SqlValue.Null

        | "ABS", [SqlValue.Integer i] -> SqlValue.Integer (abs i)
        | "ABS", [SqlValue.Real r]    -> SqlValue.Real (abs r)
        | "ABS", [SqlValue.Null]      -> SqlValue.Null
        | "ABS", _ -> SqlValue.Null

        | "ROUND", [SqlValue.Real r] ->
            // SQLite rounds half away from zero
            let rounded = Math.Round(r, MidpointRounding.AwayFromZero)
            SqlValue.Real rounded
        | "ROUND", [SqlValue.Integer i] -> SqlValue.Real (float i)
        | "ROUND", [SqlValue.Real r; SqlValue.Integer digits] ->
            let d = int digits
            let factor = Math.Pow(10.0, float d)
            let rounded = Math.Round(r * factor, MidpointRounding.AwayFromZero) / factor
            SqlValue.Real rounded
        | "ROUND", [SqlValue.Integer i; SqlValue.Integer _] -> SqlValue.Real (float i)
        | "ROUND", _ -> SqlValue.Null

        | "COALESCE", args ->
            args |> List.tryFind (fun a -> a <> SqlValue.Null) |> Option.defaultValue SqlValue.Null

        | "IFNULL", [a; b] ->
            if a <> SqlValue.Null then a else b

        | "__CONCAT__", [a; b] ->
            // || operator
            let toStr v = match v with SqlValue.Text s -> s | SqlValue.Integer i -> string i | SqlValue.Real r -> r.ToString(CultureInfo.InvariantCulture) | _ -> ""
            match a, b with
            | SqlValue.Null, _ | _, SqlValue.Null -> SqlValue.Null
            | _ -> SqlValue.Text (toStr a + toStr b)

        | _ -> SqlValue.Null

    // ── LIKE pattern matching — iterative, no Regex ───────────────────────
    //
    // Using Regex.Replace("%", ".*") triggers catastrophic backtracking on
    // adversarial patterns like 'a%b%c%d%' against a long mismatch string
    // (ReDoS).  The iterative implementation below is O(n×m) in the worst
    // case — the same algorithm used in SqlVm.LikeMatch.

    let rec private likeMatchRec (s: string) (si: int) (p: string) (pi: int) : bool =
        if pi = p.Length then
            si = s.Length
        elif p.[pi] = '%' then
            // Skip consecutive % wildcards — they collapse to one.
            let mutable pi' = pi + 1
            while pi' < p.Length && p.[pi'] = '%' do
                pi' <- pi' + 1
            if pi' = p.Length then true  // trailing % matches any suffix
            else
                let mutable i = si
                let mutable found = false
                while i <= s.Length && not found do
                    found <- likeMatchRec s i p pi'
                    i <- i + 1
                found
        elif si = s.Length then
            false
        elif p.[pi] = '_' || Char.ToUpperInvariant(p.[pi]) = Char.ToUpperInvariant(s.[si]) then
            likeMatchRec s (si + 1) p (pi + 1)
        else
            false

    let likeMatch (value: string) (pattern: string) : bool =
        likeMatchRec value 0 pattern 0

    /// Compare two SqlValues; returns -1/0/1 using SQLite ordering rules
    /// (NULL < everything; integers and reals are compared numerically).
    let cmpValues (a: SqlValue) (b: SqlValue) : int =
        match a, b with
        | SqlValue.Null, SqlValue.Null -> 0
        | SqlValue.Null, _             -> -1
        | _, SqlValue.Null             ->  1
        | SqlValue.Bool x, SqlValue.Bool y ->
            compare (if x then 1 else 0) (if y then 1 else 0)
        | SqlValue.Integer x, SqlValue.Integer y -> compare x y
        | SqlValue.Integer x, SqlValue.Real y    -> compare (float x) y
        | SqlValue.Real x,    SqlValue.Integer y -> compare x (float y)
        | SqlValue.Real x,    SqlValue.Real y    -> compare x y
        | SqlValue.Text x,    SqlValue.Text y    -> String.Compare(x, y, StringComparison.OrdinalIgnoreCase)
        | _ -> 0

    /// Recursively evaluate an expression against a row (column values by name).
    /// Handles comparisons, arithmetic, AND/OR, IS NULL, etc.
    let rec evalExpr (row: Map<string, SqlValue>) (expr: Expr) : SqlValue =
        match expr with
        | Expr.Literal v -> v
        | Expr.Column(_, col) ->
            match Map.tryFind (col.ToLowerInvariant()) row with
            | Some v -> v
            | None   ->
                // Case-insensitive fallback
                row |> Map.tryFindKey (fun k _ -> String.Compare(k, col, StringComparison.OrdinalIgnoreCase) = 0)
                    |> Option.bind (fun k -> Map.tryFind k row)
                    |> Option.defaultValue SqlValue.Null
        | Expr.FuncCall(name, args) ->
            let argVals = args |> List.map (evalExpr row)
            evalBuiltin name argVals
        | Expr.IsNull e ->
            SqlValue.Bool (evalExpr row e = SqlValue.Null)
        | Expr.IsNotNull e ->
            SqlValue.Bool (evalExpr row e <> SqlValue.Null)
        | Expr.BinaryOp(op, l, r) ->
            let lv = evalExpr row l
            let rv = evalExpr row r
            match op with
            // Arithmetic
            | BinaryOperator.Add ->
                match lv, rv with
                | SqlValue.Null, _ | _, SqlValue.Null -> SqlValue.Null
                | SqlValue.Integer a, SqlValue.Integer b -> SqlValue.Integer (a + b)
                | SqlValue.Real a, SqlValue.Integer b    -> SqlValue.Real (a + float b)
                | SqlValue.Integer a, SqlValue.Real b    -> SqlValue.Real (float a + b)
                | SqlValue.Real a, SqlValue.Real b       -> SqlValue.Real (a + b)
                | _ -> SqlValue.Null
            | BinaryOperator.Sub ->
                match lv, rv with
                | SqlValue.Null, _ | _, SqlValue.Null -> SqlValue.Null
                | SqlValue.Integer a, SqlValue.Integer b -> SqlValue.Integer (a - b)
                | SqlValue.Real a, SqlValue.Integer b    -> SqlValue.Real (a - float b)
                | SqlValue.Integer a, SqlValue.Real b    -> SqlValue.Real (float a - b)
                | SqlValue.Real a, SqlValue.Real b       -> SqlValue.Real (a - b)
                | _ -> SqlValue.Null
            | BinaryOperator.Mul ->
                match lv, rv with
                | SqlValue.Null, _ | _, SqlValue.Null -> SqlValue.Null
                | SqlValue.Integer a, SqlValue.Integer b -> SqlValue.Integer (a * b)
                | SqlValue.Real a, SqlValue.Integer b    -> SqlValue.Real (a * float b)
                | SqlValue.Integer a, SqlValue.Real b    -> SqlValue.Real (float a * b)
                | SqlValue.Real a, SqlValue.Real b       -> SqlValue.Real (a * b)
                | _ -> SqlValue.Null
            | BinaryOperator.Div ->
                match lv, rv with
                | SqlValue.Null, _ | _, SqlValue.Null -> SqlValue.Null
                | _, SqlValue.Integer 0L | _, SqlValue.Real 0.0 -> SqlValue.Null
                | SqlValue.Integer a, SqlValue.Integer b -> SqlValue.Integer (a / b)
                | SqlValue.Real a, SqlValue.Integer b    -> SqlValue.Real (a / float b)
                | SqlValue.Integer a, SqlValue.Real b    -> SqlValue.Real (float a / b)
                | SqlValue.Real a, SqlValue.Real b       -> SqlValue.Real (a / b)
                | _ -> SqlValue.Null
            // Comparisons (propagate NULL)
            | BinaryOperator.Eq ->
                if lv = SqlValue.Null || rv = SqlValue.Null then SqlValue.Null
                else SqlValue.Bool (cmpValues lv rv = 0)
            | BinaryOperator.NotEq ->
                if lv = SqlValue.Null || rv = SqlValue.Null then SqlValue.Null
                else SqlValue.Bool (cmpValues lv rv <> 0)
            | BinaryOperator.Lt ->
                if lv = SqlValue.Null || rv = SqlValue.Null then SqlValue.Null
                else SqlValue.Bool (cmpValues lv rv < 0)
            | BinaryOperator.Lte ->
                if lv = SqlValue.Null || rv = SqlValue.Null then SqlValue.Null
                else SqlValue.Bool (cmpValues lv rv <= 0)
            | BinaryOperator.Gt ->
                if lv = SqlValue.Null || rv = SqlValue.Null then SqlValue.Null
                else SqlValue.Bool (cmpValues lv rv > 0)
            | BinaryOperator.Gte ->
                if lv = SqlValue.Null || rv = SqlValue.Null then SqlValue.Null
                else SqlValue.Bool (cmpValues lv rv >= 0)
            // Logical (three-valued)
            | BinaryOperator.And ->
                match lv, rv with
                | SqlValue.Bool false, _ | _, SqlValue.Bool false -> SqlValue.Bool false
                | SqlValue.Null, _ | _, SqlValue.Null -> SqlValue.Null
                | _ -> SqlValue.Bool true
            | BinaryOperator.Or ->
                match lv, rv with
                | SqlValue.Bool true,  _ | _, SqlValue.Bool true  -> SqlValue.Bool true
                | SqlValue.Null, _ | _, SqlValue.Null -> SqlValue.Null
                | _ -> SqlValue.Bool false
            | BinaryOperator.Mod ->
                match lv, rv with
                | SqlValue.Null, _ | _, SqlValue.Null -> SqlValue.Null
                | SqlValue.Integer a, SqlValue.Integer b -> if b = 0L then SqlValue.Null else SqlValue.Integer (a % b)
                | _ -> SqlValue.Null
        | Expr.UnaryOp(UnaryOperator.Neg, e) ->
            match evalExpr row e with
            | SqlValue.Integer i -> SqlValue.Integer -i
            | SqlValue.Real r -> SqlValue.Real -r
            | _ -> SqlValue.Null
        | Expr.UnaryOp(UnaryOperator.Not, e) ->
            match evalExpr row e with
            | SqlValue.Bool b -> SqlValue.Bool (not b)
            | SqlValue.Null -> SqlValue.Null
            | _ -> SqlValue.Null
        | Expr.Between(v, lo, hi) ->
            let vv = evalExpr row v
            let lv = evalExpr row lo
            let hv = evalExpr row hi
            if vv = SqlValue.Null then SqlValue.Null
            else SqlValue.Bool (cmpValues lv vv <= 0 && cmpValues vv hv <= 0)
        | Expr.In(v, items) ->
            let vv = evalExpr row v
            if vv = SqlValue.Null then SqlValue.Null
            else SqlValue.Bool (items |> List.exists (fun i -> evalExpr row i = vv))
        | Expr.NotIn(v, items) ->
            let vv = evalExpr row v
            if vv = SqlValue.Null then SqlValue.Null
            else SqlValue.Bool (items |> List.forall (fun i -> evalExpr row i <> vv))
        | Expr.Like(v, pat) ->
            match evalExpr row v with
            | SqlValue.Text s ->
                // Use iterative matcher to avoid ReDoS from Regex with '.*'
                // for '%' wildcards.  Same algorithm as SqlVm.LikeMatch.
                SqlValue.Bool (likeMatch s pat)
            | SqlValue.Null -> SqlValue.Null
            | _ -> SqlValue.Bool false
        | Expr.NotLike(v, pat) ->
            match evalExpr row v with
            | SqlValue.Text s ->
                SqlValue.Bool (not (likeMatch s pat))
            | SqlValue.Null -> SqlValue.Null
            | _ -> SqlValue.Bool true
        | _ -> SqlValue.Null

// ── SqlValue → obj conversion for result rows ─────────────────────────────

module private SqlValueConv =
    /// Convert a pipeline SqlValue to a .NET obj for the facade's row arrays.
    let toObj (v: SqlValue) : obj =
        match v with
        | SqlValue.Null      -> null
        | SqlValue.Integer i -> box i
        | SqlValue.Real    r -> box r
        | SqlValue.Text    s -> box s
        | SqlValue.Bool    b -> box b

// ── Level 1 execute engine ─────────────────────────────────────────────────
//
// The Level 1 engine runs the five-stage pipeline for each SQL statement.
// Transaction control commands (BEGIN, COMMIT, ROLLBACK) bypass the pipeline
// and are forwarded directly to the InMemoryBackend transaction API.

module private Level1Engine =

    /// Build a schema provider for the current backend state.
    let private schemaOf (backend: InMemoryBackend) = Schema.fromBackend backend

    /// Check whether an expression contains any FuncCall nodes.
    let rec private hasFuncCall (e: Expr) : bool =
        match e with
        | Expr.FuncCall _          -> true
        | Expr.BinaryOp(_, l, r)  -> hasFuncCall l || hasFuncCall r
        | Expr.UnaryOp(_, e2)     -> hasFuncCall e2
        | Expr.IsNull e2           -> hasFuncCall e2
        | Expr.IsNotNull e2        -> hasFuncCall e2
        | Expr.Between(v, lo, hi)  -> hasFuncCall v || hasFuncCall lo || hasFuncCall hi
        | Expr.In(v, items)        -> hasFuncCall v || List.exists hasFuncCall items
        | Expr.NotIn(v, items)     -> hasFuncCall v || List.exists hasFuncCall items
        | Expr.Like(v, _)          -> hasFuncCall v
        | Expr.NotLike(v, _)       -> hasFuncCall v
        | Expr.AggExpr(_, AggArg.Expr e2, _) -> hasFuncCall e2
        | _ -> false

    /// Check whether a plan contains any FuncCall nodes in projection columns.
    let rec private planHasFuncCallProjection (plan: OptimizedPlan) : bool =
        match plan with
        | OptimizedPlan.Project(_, cols) ->
            cols |> List.exists (fun c ->
                match c with
                | OutputColumn.Expr(e, _) -> hasFuncCall e
                | _ -> false)
        | _ -> false

    /// Collect SELECT output column (name, expr) pairs for function rewriting.
    let private collectProjectionExprs (plan: OptimizedPlan) : (string * Expr) list option =
        // Peel Sort/Limit/Distinct wrappers to find the innermost Project node.
        let rec peelWrappers p =
            match p with
            | OptimizedPlan.Sort(inner, _)   -> peelWrappers inner
            | OptimizedPlan.Limit(inner, _,_)-> peelWrappers inner
            | OptimizedPlan.Distinct(inner)  -> peelWrappers inner
            | other -> other
        let corePlan = peelWrappers plan
        match corePlan with
        | OptimizedPlan.Project(_, cols) ->
            // We need the column names that the VM will emit.
            // OutputColumn carries name/alias.
            let pairs =
                cols |> List.choose (fun col ->
                    match col with
                    | OutputColumn.Star -> None
                    | OutputColumn.Expr(e, aliasOpt) ->
                        let name =
                            match aliasOpt with
                            | Some a -> a
                            | None ->
                                match e with
                                | Expr.Column(_, c) -> c
                                | Expr.FuncCall(fn, _) -> fn.ToLowerInvariant()
                                | _ -> "?"
                        if hasFuncCall e then Some (name.ToLowerInvariant(), e)
                        else None)
            if pairs.IsEmpty then None else Some pairs
        | _ -> None

    /// Apply SELECT-projection function calls to a result row.
    /// `colExprs` = (colName.lower, expr) for columns that need function eval.
    /// `resultCols` = column names from the VM result (in order).
    /// `row` = the VM-produced row values (in column order).
    let private applyFuncCalls
        (colExprs: (string * Expr) list)
        (resultCols: string list)
        (row: SqlValue list)
        : SqlValue list =

        // Build a map from column name to VM value for use as row context.
        // Hidden function-argument columns __farg_X__ are also added under
        // the original name X, so FuncEval.evalExpr can look up Expr.Column(_, "X").
        let rowMap =
            List.zip resultCols row
            |> List.collect (fun (c, v) ->
                let lower = c.ToLowerInvariant()
                if lower.StartsWith("__farg_") && lower.EndsWith("__") then
                    let origName = lower.[7..lower.Length - 3]  // strip __farg_ and __
                    [(lower, v); (origName, v)]
                else
                    [(lower, v)])
            |> Map.ofList

        // Replace values for columns that have function expressions.
        row |> List.mapi (fun i v ->
            let colName = if i < resultCols.Length then resultCols.[i].ToLowerInvariant() else ""
            match colExprs |> List.tryFind (fun (n, _) -> n = colName) with
            | Some (_, expr) -> FuncEval.evalExpr rowMap expr
            | None           -> v)

    /// Whether a SqlValue is truthy (for WHERE / HAVING / CASE).
    let private isTruthySqlValue (v: SqlValue) : bool =
        match v with
        | SqlValue.Bool b  -> b
        | SqlValue.Integer i -> i <> 0L
        | SqlValue.Real r    -> r <> 0.0
        | SqlValue.Text t    -> t <> ""
        | SqlValue.Null      -> false

    /// Scan a table and return rows as Map<colname, SqlValue>, properly converting obj values.
    let private scanTable (backend: InMemoryBackend) (tableName: string) (alias: string) : Map<string, SqlValue> list =
        let iter = backend.Scan(tableName)
        let result = ResizeArray<Map<string, SqlValue>>()
        let mutable current = iter.Next()
        while not (obj.ReferenceEquals(current, null)) do
            let keys = current.Keys |> Seq.toList
            let rowMap =
                keys
                |> List.collect (fun colName ->
                    let v = current.[colName]
                    let sqlVal =
                        match v with
                        | :? int64 as i -> SqlValue.Integer i
                        | :? int32 as i -> SqlValue.Integer (int64 i)
                        | :? double as r -> SqlValue.Real r
                        | :? single as f -> SqlValue.Real (float f)
                        | :? string as s -> SqlValue.Text s
                        | :? bool as b -> SqlValue.Bool b
                        | null -> SqlValue.Null
                        | other -> SqlValue.Text (string other)
                    let lo = colName.ToLowerInvariant()
                    // Register under both bare name and alias-qualified name
                    [(lo, sqlVal); ($"{alias.ToLowerInvariant()}.{lo}", sqlVal)])
                |> Map.ofList
            result.Add(rowMap)
            current <- iter.Next()
        iter.Close()
        result |> Seq.toList

    /// Compute aggregates for a GROUP BY query entirely in memory.
    /// Handles COUNT(*), COUNT(expr), SUM, AVG, MIN, MAX with optional DISTINCT.
    // Maximum rows materialized by the in-memory GROUP BY path.  This mirrors
    // the backend row-count ceiling that the VM enforces for its scan loop.
    // Without this guard the in-memory path could buffer the full table twice
    // (flat list + per-group sub-lists), exhausting heap on large tables.
    [<Literal>]
    let private MaxInMemoryRows = 1_000_000

    let private executeGroupByInMemory (backend: InMemoryBackend) (sel: SelectStmt) : ExecutionResult =
        // Step 1: Scan source table(s).
        // For Level 1 we support a single FROM table (no JOINs).
        let allRows =
            sel.From
            |> List.collect (fun (tblName, aliasOpt) ->
                let alias = aliasOpt |> Option.defaultValue tblName
                scanTable backend tblName alias)

        // Guard against unbounded in-memory materialisation.
        if allRows.Length > MaxInMemoryRows then
            raise (MiniSqliteException("OperationalError",
                sprintf "query scans more than %d rows; split the query or add a WHERE filter" MaxInMemoryRows))

        // Step 2: Apply WHERE filter.
        let filteredRows =
            match sel.Where with
            | None -> allRows
            | Some pred ->
                allRows |> List.filter (fun row ->
                    isTruthySqlValue (FuncEval.evalExpr row pred))

        // Step 3: Compute GROUP BY key for each row (empty key = no GROUP BY = one group).
        let keyOf (row: Map<string, SqlValue>) : SqlValue list =
            sel.GroupBy |> List.map (FuncEval.evalExpr row)

        // Step 4: Group rows by key.
        let groups =
            if sel.GroupBy.IsEmpty then
                [filteredRows]
            else
                filteredRows
                |> List.groupBy keyOf
                |> List.map snd

        // Step 5: For each group, compute aggregate column values.
        // We need to evaluate each SELECT output column expression over the group.
        // Non-aggregate columns are taken from the first row.
        let computeGroupRow (groupRows: Map<string, SqlValue> list) : (string * SqlValue) list =
            let firstRow = if groupRows.IsEmpty then Map.empty else groupRows.[0]
            // Collect actual output columns (excluding hidden __sort_N__ and __farg_X__).
            sel.Columns
            |> List.choose (fun oc ->
                match oc with
                | OutputColumn.Star -> None
                | OutputColumn.Expr(expr, aliasOpt) ->
                    // Determine display name.
                    let name =
                        match aliasOpt with
                        | Some a -> a
                        | None ->
                            match expr with
                            | Expr.Column(_, c) -> c
                            | Expr.AggExpr(fn, _, _) ->
                                match fn with
                                | AggFunction.Count -> "count(*)"
                                | AggFunction.Sum   -> "sum"
                                | AggFunction.Avg   -> "avg"
                                | AggFunction.Min   -> "min"
                                | AggFunction.Max   -> "max"
                            | _ -> "?"
                    // Skip hidden columns.
                    let lo = name.ToLowerInvariant()
                    if (lo.StartsWith("__sort_") || lo.StartsWith("__farg_")) && lo.EndsWith("__") then None
                    else
                    // Evaluate the expression.
                    let value =
                        match expr with
                        | Expr.AggExpr(fn, arg, distinct) ->
                            // Evaluate each row's column value for this aggregate.
                            let rawValues =
                                match arg with
                                | AggArg.Star -> groupRows |> List.map (fun _ -> SqlValue.Integer 1L)
                                | AggArg.Expr e -> groupRows |> List.map (fun r -> FuncEval.evalExpr r e)
                            let nonNulls = rawValues |> List.filter (fun v -> v <> SqlValue.Null)
                            let effectiveValues =
                                if distinct then
                                    nonNulls |> List.distinctBy (fun v ->
                                        match v with
                                        | SqlValue.Integer i -> box i
                                        | SqlValue.Real r -> box r
                                        | SqlValue.Text s -> box s
                                        | _ -> box "")
                                else nonNulls
                            match fn with
                            | AggFunction.Count ->
                                match arg with
                                | AggArg.Star ->
                                    if distinct then SqlValue.Integer (int64 effectiveValues.Length)
                                    else SqlValue.Integer (int64 groupRows.Length)
                                | _ -> SqlValue.Integer (int64 effectiveValues.Length)
                            | AggFunction.Sum ->
                                effectiveValues
                                |> List.fold (fun acc v ->
                                    match acc, v with
                                    | SqlValue.Null, _ -> v
                                    | _, SqlValue.Null -> acc
                                    | SqlValue.Integer a, SqlValue.Integer b ->
                                        // Checked add: on overflow promote to Real
                                        // (mirrors SQLite's silent int→real promotion).
                                        try SqlValue.Integer (Checked.(+) a b)
                                        with :? System.OverflowException -> SqlValue.Real (float a + float b)
                                    | SqlValue.Real a, SqlValue.Integer b -> SqlValue.Real (a + float b)
                                    | SqlValue.Integer a, SqlValue.Real b -> SqlValue.Real (float a + b)
                                    | SqlValue.Real a, SqlValue.Real b -> SqlValue.Real (a + b)
                                    | _ -> acc) SqlValue.Null
                            | AggFunction.Min ->
                                effectiveValues |> List.fold (fun acc v ->
                                    match acc with
                                    | SqlValue.Null -> v
                                    | a when FuncEval.cmpValues v a < 0 -> v
                                    | a -> a) SqlValue.Null
                            | AggFunction.Max ->
                                effectiveValues |> List.fold (fun acc v ->
                                    match acc with
                                    | SqlValue.Null -> v
                                    | a when FuncEval.cmpValues v a > 0 -> v
                                    | a -> a) SqlValue.Null
                            | AggFunction.Avg ->
                                if effectiveValues.IsEmpty then SqlValue.Null
                                else
                                    let sum =
                                        effectiveValues |> List.fold (fun acc v ->
                                            match v with
                                            | SqlValue.Integer i -> acc + float i
                                            | SqlValue.Real r -> acc + r
                                            | _ -> acc) 0.0
                                    SqlValue.Real (sum / float effectiveValues.Length)
                        | other -> FuncEval.evalExpr firstRow other
                    Some (name, value))

        // Step 6: Build HAVING evaluator.
        // For HAVING, map AggExpr in the predicate to the computed aggregate values.
        let aggResultsForHaving (groupRow: (string * SqlValue) list) (pred: Expr) : bool =
            // Build a row map from output column names to values.
            let rowMap = groupRow |> Map.ofList
            // Evaluate predicate using evalExpr but with AggExpr → column name mapping.
            // We need a modified eval that replaces AggExpr with its computed value.
            // Since FuncEval.evalExpr doesn't handle AggExpr, we pre-substitute:
            // AggExpr → the value already computed in `groupRow`.
            // The approach: rewrite the pred by replacing AggExpr with Literal.
            let rec rewriteAgg (e: Expr) : Expr =
                match e with
                | Expr.AggExpr(fn, arg, distinct) ->
                    // Find the computed value by matching against the output columns.
                    let matchingName =
                        groupRow |> List.tryFind (fun (colName, _) ->
                            // Match by function name and argument if possible.
                            // Since GROUP BY aggregates appear in sel.Columns, scan those.
                            sel.Columns |> List.exists (fun oc ->
                                match oc with
                                | OutputColumn.Expr(Expr.AggExpr(fn2, arg2, d2), Some alias) ->
                                    fn = fn2 && arg = arg2 && distinct = d2 && alias.ToLowerInvariant() = colName.ToLowerInvariant()
                                | OutputColumn.Expr(Expr.AggExpr(fn2, arg2, d2), None) ->
                                    fn = fn2 && arg = arg2 && distinct = d2
                                | _ -> false))
                    match matchingName with
                    | Some (_, v) -> Expr.Literal v
                    | None ->
                        // Last resort: find any aggregate column in groupRow.
                        // Re-compute the aggregate value on the fly.
                        // This handles the HAVING case where the pred agg is not in SELECT.
                        // We need the group rows, but we don't have them here.
                        // For now, return Null (aggregate not found).
                        Expr.Literal SqlValue.Null
                | Expr.BinaryOp(op, l, r) -> Expr.BinaryOp(op, rewriteAgg l, rewriteAgg r)
                | Expr.UnaryOp(op, e2) -> Expr.UnaryOp(op, rewriteAgg e2)
                | other -> other
            let rewritten = rewriteAgg pred
            isTruthySqlValue (FuncEval.evalExpr rowMap rewritten)

        // Re-compute HAVING evaluation WITH access to group rows for aggregate re-computation.
        let evalHavingWithGroupRows (groupRows: Map<string, SqlValue> list) (groupRow: (string * SqlValue) list) (pred: Expr) : bool =
            let firstRow = if groupRows.IsEmpty then Map.empty else groupRows.[0]
            // Rewrite AggExpr nodes to their actual computed values.
            let rec rewriteAgg (e: Expr) : Expr =
                match e with
                | Expr.AggExpr(fn, arg, distinct) ->
                    // Compute this aggregate fresh from groupRows.
                    let rawValues =
                        match arg with
                        | AggArg.Star -> groupRows |> List.map (fun _ -> SqlValue.Integer 1L)
                        | AggArg.Expr ex -> groupRows |> List.map (fun r -> FuncEval.evalExpr r ex)
                    let nonNulls = rawValues |> List.filter (fun v -> v <> SqlValue.Null)
                    let effectiveValues =
                        if distinct then
                            nonNulls |> List.distinctBy (fun v ->
                                match v with
                                | SqlValue.Integer i -> box i
                                | SqlValue.Real r -> box r
                                | SqlValue.Text s -> box s
                                | _ -> box "")
                        else nonNulls
                    let computed =
                        match fn with
                        | AggFunction.Count ->
                            match arg with
                            | AggArg.Star ->
                                if distinct then SqlValue.Integer (int64 effectiveValues.Length)
                                else SqlValue.Integer (int64 groupRows.Length)
                            | _ -> SqlValue.Integer (int64 effectiveValues.Length)
                        | AggFunction.Sum ->
                            effectiveValues |> List.fold (fun acc v ->
                                match acc, v with
                                | SqlValue.Null, _ -> v
                                | _, SqlValue.Null -> acc
                                | SqlValue.Integer a, SqlValue.Integer b ->
                                    try SqlValue.Integer (Checked.(+) a b)
                                    with :? System.OverflowException -> SqlValue.Real (float a + float b)
                                | SqlValue.Real a, SqlValue.Integer b -> SqlValue.Real (a + float b)
                                | SqlValue.Integer a, SqlValue.Real b -> SqlValue.Real (float a + b)
                                | SqlValue.Real a, SqlValue.Real b -> SqlValue.Real (a + b)
                                | _ -> acc) SqlValue.Null
                        | AggFunction.Min ->
                            effectiveValues |> List.fold (fun acc v ->
                                match acc with
                                | SqlValue.Null -> v
                                | a when FuncEval.cmpValues v a < 0 -> v
                                | a -> a) SqlValue.Null
                        | AggFunction.Max ->
                            effectiveValues |> List.fold (fun acc v ->
                                match acc with
                                | SqlValue.Null -> v
                                | a when FuncEval.cmpValues v a > 0 -> v
                                | a -> a) SqlValue.Null
                        | AggFunction.Avg ->
                            if effectiveValues.IsEmpty then SqlValue.Null
                            else
                                let sum = effectiveValues |> List.fold (fun acc v ->
                                    match v with
                                    | SqlValue.Integer i -> acc + float i
                                    | SqlValue.Real r -> acc + r
                                    | _ -> acc) 0.0
                                SqlValue.Real (sum / float effectiveValues.Length)
                    Expr.Literal computed
                | Expr.BinaryOp(op, l, r) -> Expr.BinaryOp(op, rewriteAgg l, rewriteAgg r)
                | Expr.UnaryOp(op, e2) -> Expr.UnaryOp(op, rewriteAgg e2)
                | other -> other
            let rewritten = rewriteAgg pred
            let baseMap = groupRow |> Map.ofList
            let rowMap =
                if groupRows.IsEmpty then baseMap
                else
                    groupRows.[0] |> Map.fold (fun acc k v ->
                        if Map.containsKey k acc then acc else Map.add k v acc) baseMap
            isTruthySqlValue (FuncEval.evalExpr rowMap rewritten)

        // Step 7: Compute one (name, value) list per group and apply HAVING.
        let groupResults =
            groups
            |> List.map (fun groupRows -> computeGroupRow groupRows, groupRows)
            |> List.choose (fun (groupRow, groupRows) ->
                match sel.Having with
                | None -> Some groupRow
                | Some pred ->
                    if evalHavingWithGroupRows groupRows groupRow pred then Some groupRow
                    else None)

        // Step 8: Build output columns (names) from groupResults.
        let outputColNames =
            match groupResults with
            | row :: _ -> row |> List.map fst
            | [] ->
                // Empty result — derive column names from statement.
                sel.Columns
                |> List.choose (fun oc ->
                    match oc with
                    | OutputColumn.Star -> None
                    | OutputColumn.Expr(e, aliasOpt) ->
                        let name =
                            match aliasOpt with
                            | Some a -> a
                            | None ->
                                match e with
                                | Expr.Column(_, c) -> c
                                | Expr.AggExpr(fn, _, _) ->
                                    match fn with
                                    | AggFunction.Count -> "count(*)"
                                    | _ -> "?"
                                | _ -> "?"
                        let lo = name.ToLowerInvariant()
                        if (lo.StartsWith("__sort_") || lo.StartsWith("__farg_")) && lo.EndsWith("__") then None
                        else Some name)

        // Step 9: Build rows as SqlValue lists.
        let rawRows =
            groupResults
            |> List.map (fun row ->
                row |> List.map snd)

        // Step 10: Apply ORDER BY in memory.
        let sortedRows =
            if sel.OrderBy.IsEmpty then rawRows
            else
                rawRows |> List.sortWith (fun a b ->
                    let mutable result = 0
                    let mutable ki = 0
                    while result = 0 && ki < sel.OrderBy.Length do
                        let key = sel.OrderBy.[ki]
                        // Find the column index for the sort key.
                        let colIdx =
                            match key.KeyExpr with
                            | Expr.Column(_, c) ->
                                outputColNames |> List.tryFindIndex (fun n ->
                                    String.Compare(n, c, StringComparison.OrdinalIgnoreCase) = 0)
                            | _ -> None
                        let av = match colIdx with Some i when i < a.Length -> a.[i] | _ -> SqlValue.Null
                        let bv = match colIdx with Some i when i < b.Length -> b.[i] | _ -> SqlValue.Null
                        // NullOrder specifies where NULLs appear in the final
                        // output, independent of direction.  Only non-null
                        // comparisons are flipped for DESC.
                        result <-
                            match av, bv with
                            | SqlValue.Null, SqlValue.Null -> 0
                            | SqlValue.Null, _ ->
                                match key.NullOrder with NullOrder.NullsFirst -> -1 | NullOrder.NullsLast -> 1
                            | _, SqlValue.Null ->
                                match key.NullOrder with NullOrder.NullsFirst -> 1 | NullOrder.NullsLast -> -1
                            | l, r ->
                                let rawCmp = FuncEval.cmpValues l r
                                match key.Direction with SortDir.Asc -> rawCmp | SortDir.Desc -> -rawCmp
                        ki <- ki + 1
                    result)

        // Step 11: Apply LIMIT / OFFSET.
        // Clamp int64 LIMIT/OFFSET values to valid int range to avoid silent
        // wrap-around on values > Int32.MaxValue (e.g. LIMIT 2147483648).
        let toSafeInt (v: int64) =
            if v < 0L then 0
            elif v > int64 System.Int32.MaxValue then System.Int32.MaxValue
            else int v
        let limitedRows =
            match sel.Limit with
            | None -> sortedRows
            | Some lc ->
                let offset = match lc.Offset with Some o -> toSafeInt o | None -> 0
                let count  = match lc.Count  with Some c -> toSafeInt c | None -> System.Int32.MaxValue
                sortedRows |> List.skip (min offset sortedRows.Length)
                           |> List.truncate count

        // Step 12: Convert to IReadOnlyList<obj> rows.
        let finalRows =
            limitedRows
            |> List.map (fun row ->
                row |> List.map SqlValueConv.toObj :> IReadOnlyList<obj>)

        { Columns = outputColNames
          Rows    = finalRows
          RowCount = -1
          LastRowId = box (null: string) }

    /// Execute a single bound SQL statement against the backend.
    /// Returns ExecutionResult.
    let execute (backend: InMemoryBackend) (sql: string) (autocommit: bool) (txHandle: TransactionHandle option ref) : ExecutionResult =
        let trimmed = SqlText.trim sql

        try
            match SqlText.firstKeyword trimmed with
            | "BEGIN" ->
                // Start a transaction.
                if txHandle.Value.IsNone then
                    txHandle.Value <- Some (backend.BeginTransaction())
                ExecutionResult.empty 0

            | "COMMIT" ->
                match txHandle.Value with
                | Some h ->
                    backend.Commit(h)
                    txHandle.Value <- None
                | None -> ()
                ExecutionResult.empty 0

            | "ROLLBACK" ->
                match txHandle.Value with
                | Some h ->
                    backend.Rollback(h)
                    txHandle.Value <- None
                | None -> ()
                ExecutionResult.empty 0

            | _ ->
                // Parse the SQL text into a Statement DU.
                let stmt = SqlStatementParser.parse trimmed

                // ── FROM-less SELECT ──────────────────────────────────────────
                // SELECT expr [AS alias] [, …] without a FROM clause.
                // The planner rejects these as UnsupportedStatement.  We handle
                // them directly by evaluating the projection expressions against
                // an empty row map.  Supports constant expressions, function
                // calls, and || concatenation.
                let isFromlessSelect =
                    match stmt with
                    | Statement.Select sel when sel.From.IsEmpty -> true
                    | _ -> false

                // ── GROUP BY SELECT ───────────────────────────────────────────
                // SELECT with GROUP BY is handled entirely in memory because the
                // VM's compileAggregateQuery only produces one result row (it
                // accumulates all rows into a single slot set and emits once).
                // Per-group accumulation requires Group-Change detection that the
                // current VM does not implement.  We bypass the pipeline entirely
                // and use FuncEval to evaluate expressions row-by-row.
                let isGroupBySelect =
                    match stmt with
                    | Statement.Select sel when not sel.From.IsEmpty && sel.GroupBy <> [] -> true
                    | _ -> false

                // Also handle aggregate queries without GROUP BY (COUNT(*), SUM, etc.)
                // using in-memory evaluation to avoid the codegen issues.
                let rec containsAggExpr (e: Expr) : bool =
                    match e with
                    | Expr.AggExpr _ -> true
                    | Expr.BinaryOp(_, l, r) -> containsAggExpr l || containsAggExpr r
                    | Expr.FuncCall(_, args) -> List.exists containsAggExpr args
                    | _ -> false
                let isAggSelect =
                    not isGroupBySelect &&
                    (match stmt with
                     | Statement.Select sel when not sel.From.IsEmpty ->
                         sel.Columns |> List.exists (fun oc ->
                             match oc with
                             | OutputColumn.Expr(e, _) -> containsAggExpr e
                             | _ -> false)
                     | _ -> false)

                if isFromlessSelect then
                    let sel =
                        match stmt with
                        | Statement.Select s -> s
                        | _ -> failwith "impossible"
                    let emptyRow = Map.empty<string, SqlValue>
                    let cols, vals =
                        sel.Columns
                        |> List.map (fun oc ->
                            match oc with
                            | OutputColumn.Star -> ("*", SqlValue.Null)
                            | OutputColumn.Expr(e, aliasOpt) ->
                                let v = FuncEval.evalExpr emptyRow e
                                let name =
                                    match aliasOpt with
                                    | Some a -> a
                                    | None ->
                                        match e with
                                        | Expr.Literal (SqlValue.Text s) -> s
                                        | Expr.FuncCall(fn, _) -> fn.ToLowerInvariant()
                                        | Expr.Column(_, c) -> c
                                        | _ -> "?"
                                name, v)
                        |> List.unzip
                    let row = vals |> List.map SqlValueConv.toObj :> IReadOnlyList<obj>
                    { Columns = cols; Rows = [ row ]; RowCount = -1; LastRowId = box (null: string) }

                elif isGroupBySelect || isAggSelect then
                    // In-memory GROUP BY (or aggregate-without-GROUP-BY) path.
                    let sel =
                        match stmt with
                        | Statement.Select s -> s
                        | _ -> failwith "impossible"
                    executeGroupByInMemory backend sel

                else

                // For INSERT/UPDATE/DELETE/DDL, ensure a transaction is open
                // if not in autocommit mode.
                match stmt with
                | Statement.Insert _ | Statement.Update _ | Statement.Delete _
                | Statement.CreateTable _ | Statement.DropTable _ when not autocommit ->
                    if txHandle.Value.IsNone then
                        txHandle.Value <- Some (backend.BeginTransaction())
                | _ -> ()

                // Build schema provider for the planner.
                let schema = schemaOf backend

                // Plan → Optimize → Codegen → VM
                let logicalPlan =
                    match Planner.plan schema stmt with
                    | Ok plan -> plan
                    | Error (PlanError.UnknownTable t) ->
                        raise (MiniSqliteException("OperationalError", $"no such table: {t}"))
                    | Error (PlanError.UnknownColumn(Some t, c)) ->
                        raise (MiniSqliteException("OperationalError", $"no such column: {t}.{c}"))
                    | Error (PlanError.UnknownColumn(None, c)) ->
                        raise (MiniSqliteException("OperationalError", $"no such column: {c}"))
                    | Error (PlanError.AmbiguousColumn(c, _)) ->
                        raise (MiniSqliteException("OperationalError", $"ambiguous column: {c}"))
                    | Error (PlanError.InvalidAggregate msg) ->
                        raise (MiniSqliteException("OperationalError", msg))
                    | Error (PlanError.UnsupportedStatement k) ->
                        raise (MiniSqliteException("OperationalError", $"unsupported statement: {k}"))
                    | Error (PlanError.InternalError msg) ->
                        raise (MiniSqliteException("OperationalError", msg))

                let optimizedPlan = SqlOptimizer.optimize logicalPlan

                // Detect projection function calls before codegen swallows them.
                let projFuncExprs = collectProjectionExprs optimizedPlan

                let program     = SqlCodegen.compile optimizedPlan
                let queryResult = SqlVm.execute program backend

                // Apply function calls to result rows if needed.
                let finalRows =
                    match projFuncExprs with
                    | None -> queryResult.Rows
                    | Some colExprs ->
                        queryResult.Rows
                        |> List.map (applyFuncCalls colExprs queryResult.Columns)

                // Convert to ExecutionResult.
                // Use the statement type, not queryResult.Columns.IsEmpty, to distinguish
                // SELECT from DML: the VM only records column names on the first EmitRow,
                // so an empty SELECT result (LIMIT 0, past-end OFFSET, filter removes all)
                // returns Columns = [] which would be mistaken for DML.
                let isSelectStmt =
                    match stmt with Statement.Select _ -> true | _ -> false

                if not isSelectStmt then
                    // DML / DDL
                    let rowCount = queryResult.RowsAffected

                    // For INSERT, last row ID is not exposed by the InMemoryBackend.
                    // Return a synthetic row count (the backend tracks row order).
                    let lastRowId : obj =
                        match stmt with
                        | Statement.Insert i ->
                            try
                                let rows2 = backend.Scan(i.Table)
                                let mutable count = 0L
                                let mutable r2 = rows2.Next()
                                while not (obj.ReferenceEquals(r2, null)) do
                                    count <- count + 1L
                                    r2 <- rows2.Next()
                                rows2.Close()
                                box count
                            with _ -> box (null: string)
                        | _ -> box (null: string)

                    { ExecutionResult.empty rowCount with LastRowId = lastRowId }
                else
                    // SELECT result.
                    // Strip hidden sort columns (__sort_N__) that were added
                    // to the projection so that ORDER BY could reference
                    // non-projected source columns.
                    let rawCols, stripIdxSet =
                        let idxs = ResizeArray<int>()
                        let vc =
                            queryResult.Columns
                            |> List.mapi (fun i c ->
                                let lo = c.ToLowerInvariant()
                                if (lo.StartsWith("__sort_") || lo.StartsWith("__farg_")) && lo.EndsWith("__") then
                                    idxs.Add(i); None
                                else Some c)
                            |> List.choose id
                        vc, Set.ofSeq idxs

                    let filterRow (row: SqlValue list) =
                        row
                        |> List.mapi (fun i v -> if stripIdxSet.Contains(i) then None else Some v)
                        |> List.choose id

                    // Derive user-visible column names from the SELECT clause.
                    // This is the authoritative source, even when queryResult.Columns
                    // is empty (the VM only records column names on the first EmitRow;
                    // for empty result sets it returns []).
                    //
                    // The planner's collectAggregates assigns _agg0, _agg1, …
                    // but the outer Project node carries the user alias (e.g. AS n).
                    // The codegen discards those outer aliases for aggregates, so we
                    // restore them here by position-matching.
                    let visibleCols =
                        match stmt with
                        | Statement.Select sel ->
                            // User-visible column aliases (excluding hidden __sort_N__ extras).
                            let userAliases =
                                sel.Columns
                                |> List.choose (fun oc ->
                                    match oc with
                                    | OutputColumn.Star -> None
                                    | OutputColumn.Expr(e, aliasOpt) -> Some (e, aliasOpt))
                                |> List.filter (fun (_, ao) ->
                                    match ao with
                                    | Some a when (a.StartsWith("__sort_") || a.StartsWith("__farg_")) && a.EndsWith("__") -> false
                                    | _ -> true)

                            if rawCols.IsEmpty then
                                // VM returned no column info (empty result set).
                                // Derive the column names from the statement directly.
                                userAliases
                                |> List.map (fun (e, aliasOpt) ->
                                    match aliasOpt with
                                    | Some alias -> alias
                                    | None ->
                                        match e with
                                        | Expr.Column(_, c) -> c
                                        | Expr.AggExpr _ -> "?"
                                        | _ -> "?")
                            else
                                // Map each position: if the VM gave us `_aggN` and the user
                                // provided an explicit alias at the same position, use that alias.
                                let aliasOpts = userAliases |> List.map snd
                                List.mapi2 (fun _ (vmCol: string) (userAliasOpt: string option) ->
                                    match userAliasOpt with
                                    | Some alias when vmCol.StartsWith("_agg") -> alias
                                    | _ -> vmCol)
                                    rawCols aliasOpts
                        | _ -> rawCols

                    let rows =
                        finalRows
                        |> List.map (fun row ->
                            filterRow row |> List.map SqlValueConv.toObj :> IReadOnlyList<obj>)
                    { Columns = visibleCols
                      Rows = rows
                      RowCount = -1
                      LastRowId = box (null: string) }

        with
        | :? MiniSqliteException -> reraise ()
        | :? TableNotFound as ex ->
            raise (MiniSqliteException("OperationalError", $"no such table: {ex.Table}"))
        | :? TableAlreadyExists as ex ->
            raise (MiniSqliteException("OperationalError", $"table already exists: {ex.Table}"))
        | :? ConstraintViolation as ex ->
            raise (MiniSqliteException("OperationalError", ex.Message))
        | ex ->
            raise (MiniSqliteException("OperationalError", ex.Message))

// ── Connection and Cursor ──────────────────────────────────────────────────

/// A Level 1 connection that routes SQL through the five-stage pipeline.
type Connection(options: ConnectionOptions) as this =
    let autocommit  = options.Autocommit
    let backend     = InMemoryBackend()
    let mutable closed = false

    // Transaction handle managed through the backend.
    // In non-autocommit mode, the first mutating statement opens a transaction.
    let txHandle : TransactionHandle option ref = ref None

    let assertOpen () =
        if closed then
            raise (MiniSqliteException("ProgrammingError", "connection is closed"))

    member _.Cursor() =
        assertOpen ()
        new Cursor(this)

    member this.Execute(sql: string, [<ParamArray>] parameters: obj[]) =
        this.Cursor().Execute(sql, parameters :> IReadOnlyList<obj>)

    member this.Execute(sql: string, parameters: IReadOnlyList<obj>) =
        this.Cursor().Execute(sql, parameters)

    member this.ExecuteMany(sql: string, parameterSets: seq<IReadOnlyList<obj>>) =
        this.Cursor().ExecuteMany(sql, parameterSets)

    member _.Commit() =
        assertOpen ()
        match txHandle.Value with
        | Some h ->
            backend.Commit(h)
            txHandle.Value <- None
        | None -> ()

    member _.Rollback() =
        assertOpen ()
        match txHandle.Value with
        | Some h ->
            backend.Rollback(h)
            txHandle.Value <- None
        | None -> ()

    member internal _.ExecuteBound(sql: string, parameters: IReadOnlyList<obj>) =
        assertOpen ()
        if sql.Length > 1_048_576 then
            raise (MiniSqliteException("ProgrammingError", "SQL statement exceeds maximum length of 1 MB"))
        let bound = SqlText.bindParameters sql parameters
        Level1Engine.execute backend bound autocommit txHandle

    interface IDisposable with
        member _.Dispose() =
            if not closed then
                // Rollback uncommitted changes on close.
                match txHandle.Value with
                | Some h ->
                    try backend.Rollback(h) with _ -> ()
                    txHandle.Value <- None
                | None -> ()
                closed <- true

and Cursor internal (connection: Connection) =
    let mutable rows: IReadOnlyList<obj> array = [||]
    let mutable offset = 0
    let mutable description: Column array = [||]
    let mutable rowCount = -1
    let mutable lastRowId: obj = box (null: string)
    let mutable arraySize = 1
    let mutable closed = false

    let assertOpen () =
        if closed then
            raise (MiniSqliteException("ProgrammingError", "cursor is closed"))

    member _.Description = description :> IReadOnlyList<Column>
    member _.RowCount = rowCount
    member _.LastRowId = lastRowId

    member _.ArraySize
        with get ()  = arraySize
        and  set v   = arraySize <- v

    member this.Execute(sql: string, [<ParamArray>] parameters: obj[]) =
        this.Execute(sql, parameters :> IReadOnlyList<obj>)

    member this.Execute(sql: string, parameters: IReadOnlyList<obj>) =
        assertOpen ()
        let result = connection.ExecuteBound(sql, parameters)
        rows        <- result.Rows |> List.toArray
        offset      <- 0
        description <- result.Columns |> List.map (fun name -> { Name = name }) |> List.toArray
        rowCount    <- result.RowCount
        lastRowId   <- result.LastRowId
        this

    member this.ExecuteMany(sql: string, parameterSets: seq<IReadOnlyList<obj>>) =
        let mutable last: Cursor = this
        for parameters in parameterSets do
            last <- this.Execute(sql, parameters)
        last

    member _.FetchOne() : IReadOnlyList<obj> =
        assertOpen ()
        if offset >= rows.Length then null
        else
            let row = rows.[offset]
            offset <- offset + 1
            row

    member this.FetchMany() = this.FetchMany(arraySize)

    member _.FetchMany(size: int) : IReadOnlyList<IReadOnlyList<obj>> =
        assertOpen ()
        let limit = max 0 size
        let output = ResizeArray<IReadOnlyList<obj>>()
        let mutable consumed = 0
        while consumed < limit && offset < rows.Length do
            output.Add(rows.[offset])
            offset <- offset + 1
            consumed <- consumed + 1
        output :> IReadOnlyList<IReadOnlyList<obj>>

    member _.FetchAll() : IReadOnlyList<IReadOnlyList<obj>> =
        assertOpen ()
        let output = ResizeArray<IReadOnlyList<obj>>()
        while offset < rows.Length do
            output.Add(rows.[offset])
            offset <- offset + 1
        output :> IReadOnlyList<IReadOnlyList<obj>>

    interface IDisposable with
        member _.Dispose() = closed <- true

/// Entry point for the Level 1 mini-sqlite facade.
type MiniSqlite =
    static member ApiLevel   = "2.0"
    static member ThreadSafety = 1
    static member ParamStyle = "qmark"

    static member Connect(database: string, ?options: ConnectionOptions) =
        if database <> ":memory:" then
            raise (MiniSqliteException("NotSupportedError",
                "F# mini-sqlite supports only :memory: at Level 1"))
        new Connection(defaultArg options ConnectionOptions.Default)
