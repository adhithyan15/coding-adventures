namespace CodingAdventures.SqlExecutionEngine.FSharp

open System
open System.Collections.Generic
open System.Globalization
open System.Text.RegularExpressions

type SqlExecutionException(message: string, ?inner: exn) =
    inherit Exception(message, defaultArg inner null)

type QueryResult =
    { Columns: string array
      Rows: obj array array }

type ExecutionResult =
    { Ok: bool
      Result: QueryResult option
      Error: string option }

type IDataSource =
    abstract Schema: tableName: string -> string array
    abstract Scan: tableName: string -> IReadOnlyDictionary<string, obj> array

type InMemoryDataSource() =
    let schemas = Dictionary<string, string array>(StringComparer.Ordinal)
    let tables = Dictionary<string, IReadOnlyDictionary<string, obj> array>(StringComparer.Ordinal)

    let copyRow (row: IReadOnlyDictionary<string, obj>) =
        let copied = Dictionary<string, obj>(StringComparer.Ordinal)
        for entry in row do
            copied[entry.Key] <- entry.Value
        copied :> IReadOnlyDictionary<string, obj>

    member this.AddTable(name: string, schema: seq<string>, rows: seq<IReadOnlyDictionary<string, obj>>) =
        schemas[name] <- schema |> Seq.toArray
        tables[name] <- rows |> Seq.map copyRow |> Seq.toArray
        this

    interface IDataSource with
        member _.Schema(tableName: string) =
            match schemas.TryGetValue tableName with
            | true, schema -> Array.copy schema
            | false, _ -> raise (SqlExecutionException("table not found: " + tableName))

        member _.Scan(tableName: string) =
            match tables.TryGetValue tableName with
            | true, rows -> rows |> Array.map copyRow
            | false, _ -> raise (SqlExecutionException("table not found: " + tableName))

type private TokenKind =
    | Ident
    | Keyword
    | Number
    | String
    | Symbol
    | Eof

type private Token = { Kind: TokenKind; Value: string }

type private Expr =
    | Literal of obj
    | Null
    | Column of string option * string
    | Star
    | Unary of string * Expr
    | Binary of string * Expr * Expr
    | IsNullExpr of Expr * bool
    | Between of Expr * Expr * Expr * bool
    | In of Expr * Expr list * bool
    | Like of Expr * Expr * bool
    | Function of string * Expr list

type private SelectItem = { Expression: Expr; Alias: string option }
type private TableRef = { Name: string; Alias: string }
type private JoinDef = { JoinType: string; Table: TableRef; On: Expr option }
type private OrderItem = { Expression: Expr; Descending: bool }

type private SelectStatement =
    { Distinct: bool
      SelectItems: SelectItem list
      From: TableRef
      Joins: JoinDef list
      Where: Expr option
      GroupBy: Expr list
      Having: Expr option
      OrderBy: OrderItem list
      Limit: int option
      Offset: int option }

type private RowContext = { Values: Dictionary<string, obj> }
type private RowFrame = { Row: RowContext; GroupRows: RowContext list option }

[<RequireQualifiedAccess>]
module SqlExecutionEngine =
    let private keywords =
        set
            [ "SELECT"; "FROM"; "WHERE"; "GROUP"; "BY"; "HAVING"; "ORDER"; "LIMIT"; "OFFSET"
              "DISTINCT"; "ALL"; "JOIN"; "INNER"; "LEFT"; "RIGHT"; "FULL"; "OUTER"; "CROSS"
              "ON"; "AS"; "AND"; "OR"; "NOT"; "IS"; "NULL"; "IN"; "BETWEEN"; "LIKE"; "TRUE"
              "FALSE"; "ASC"; "DESC"; "COUNT"; "SUM"; "AVG"; "MIN"; "MAX"; "UPPER"; "LOWER"; "LENGTH" ]

    let private isIdentStart ch = Char.IsLetter(ch) || ch = '_'
    let private isIdentPart ch = Char.IsLetterOrDigit(ch) || ch = '_'

    let private tokenize (sql: string) =
        let tokens = ResizeArray<Token>()
        let mutable index = 0

        while index < sql.Length do
            let ch = sql[index]

            if Char.IsWhiteSpace ch then
                index <- index + 1
            elif ch = '-' && index + 1 < sql.Length && sql[index + 1] = '-' then
                index <- index + 2
                while index < sql.Length && sql[index] <> '\n' do
                    index <- index + 1
            elif ch = '\'' then
                let value = Text.StringBuilder()
                index <- index + 1
                let mutable doneString = false
                while index < sql.Length && not doneString do
                    let current = sql[index]
                    if current = '\'' && index + 1 < sql.Length && sql[index + 1] = '\'' then
                        value.Append('\'') |> ignore
                        index <- index + 2
                    elif current = '\'' then
                        index <- index + 1
                        doneString <- true
                    else
                        value.Append(current) |> ignore
                        index <- index + 1
                tokens.Add({ Kind = String; Value = value.ToString() })
            elif Char.IsDigit ch || (ch = '.' && index + 1 < sql.Length && Char.IsDigit(sql[index + 1])) then
                let start = index
                index <- index + 1
                while index < sql.Length && (Char.IsDigit(sql[index]) || sql[index] = '.') do
                    index <- index + 1
                tokens.Add({ Kind = Number; Value = sql.Substring(start, index - start) })
            elif isIdentStart ch then
                let start = index
                index <- index + 1
                while index < sql.Length && isIdentPart sql[index] do
                    index <- index + 1
                let value = sql.Substring(start, index - start)
                let upper = value.ToUpperInvariant()
                tokens.Add({ Kind = (if keywords.Contains upper then Keyword else Ident); Value = value })
            elif ch = '"' || ch = '`' then
                let quote = ch
                let start = index + 1
                index <- start
                while index < sql.Length && sql[index] <> quote do
                    index <- index + 1
                tokens.Add({ Kind = Ident; Value = sql.Substring(start, index - start) })
                if index < sql.Length then
                    index <- index + 1
            else
                if index + 1 < sql.Length then
                    let two = sql.Substring(index, 2)
                    match two with
                    | "!=" | "<>" | "<=" | ">=" ->
                        tokens.Add({ Kind = Symbol; Value = two })
                        index <- index + 2
                    | _ ->
                        if "=<>+-*/%(),.;".Contains(ch) then
                            tokens.Add({ Kind = Symbol; Value = string ch })
                        index <- index + 1
                else
                    if "=<>+-*/%(),.;".Contains(ch) then
                        tokens.Add({ Kind = Symbol; Value = string ch })
                    index <- index + 1

        tokens.Add({ Kind = Eof; Value = "" })
        tokens |> Seq.toArray

    type private Parser(tokens: Token array) =
        let mutable position = 0

        member private _.Peek = tokens[position]

        member private _.Advance() =
            let token = tokens[position]
            if token.Kind <> Eof then
                position <- position + 1
            token

        member private this.Error(message: string) =
            SqlExecutionException(sprintf "%s near token %d" message position)

        member private this.MatchKeyword(value: string) =
            let token = this.Peek
            if token.Kind = Keyword && token.Value.Equals(value, StringComparison.OrdinalIgnoreCase) then
                this.Advance() |> ignore
                true
            else
                false

        member private this.ExpectKeyword(value: string) =
            if not (this.MatchKeyword value) then
                raise (this.Error("expected " + value))

        member private this.MatchSymbol(value: string) =
            let token = this.Peek
            if token.Kind = Symbol && token.Value = value then
                this.Advance() |> ignore
                true
            else
                false

        member private this.ExpectSymbol(value: string) =
            if not (this.MatchSymbol value) then
                raise (this.Error("expected " + value))

        member private this.Expect(kind: TokenKind) =
            let token = this.Advance()
            if token.Kind <> kind then
                raise (this.Error(sprintf "expected %A, got %A" kind token.Kind))

        member private this.ExpectIdentifier() =
            let token = this.Advance()
            if token.Kind = Ident || token.Kind = Keyword then
                token.Value
            else
                raise (this.Error "expected identifier")

        member private this.NumberAsInt(token: Token) =
            if token.Kind <> Number then
                raise (this.Error "expected number")
            Int32.Parse(token.Value, CultureInfo.InvariantCulture)

        member this.ParseStatement() =
            this.ExpectKeyword "SELECT"
            let distinct = this.MatchKeyword "DISTINCT"
            this.MatchKeyword "ALL" |> ignore
            let selectItems = this.ParseSelectList()
            this.ExpectKeyword "FROM"
            let from = this.ParseTableRef()
            let joins = this.ParseJoins()
            let where = if this.MatchKeyword "WHERE" then Some(this.ParseExpression()) else None
            let groupBy =
                if this.MatchKeyword "GROUP" then
                    this.ExpectKeyword "BY"
                    this.ParseExpressionList()
                else
                    []
            let having = if this.MatchKeyword "HAVING" then Some(this.ParseExpression()) else None
            let orderBy =
                if this.MatchKeyword "ORDER" then
                    this.ExpectKeyword "BY"
                    this.ParseOrderList()
                else
                    []
            let limit = if this.MatchKeyword "LIMIT" then Some(this.NumberAsInt(this.Advance())) else None
            let offset = if this.MatchKeyword "OFFSET" then Some(this.NumberAsInt(this.Advance())) else None
            this.MatchSymbol ";" |> ignore
            this.Expect Eof
            { Distinct = distinct
              SelectItems = selectItems
              From = from
              Joins = joins
              Where = where
              GroupBy = groupBy
              Having = having
              OrderBy = orderBy
              Limit = limit
              Offset = offset }

        member private this.ParseSelectList() =
            let items = ResizeArray<SelectItem>()
            let mutable keepGoing = true
            while keepGoing do
                if this.MatchSymbol "*" then
                    items.Add({ Expression = Star; Alias = None })
                else
                    let expression = this.ParseExpression()
                    let alias =
                        if this.MatchKeyword "AS" then
                            Some(this.ExpectIdentifier())
                        elif this.Peek.Kind = Ident then
                            Some((this.Advance()).Value)
                        else
                            None
                    items.Add({ Expression = expression; Alias = alias })
                keepGoing <- this.MatchSymbol ","
            items |> Seq.toList

        member private this.ParseTableRef() =
            let name = this.ExpectIdentifier()
            let alias =
                if this.MatchKeyword "AS" then
                    this.ExpectIdentifier()
                elif this.Peek.Kind = Ident then
                    (this.Advance()).Value
                else
                    name
            { Name = name; Alias = alias }

        member private this.ParseJoins() =
            let joins = ResizeArray<JoinDef>()
            let mutable parsing = true
            while parsing do
                let mutable joinType = None
                if this.MatchKeyword "INNER" then
                    joinType <- Some "INNER"
                    this.ExpectKeyword "JOIN"
                elif this.MatchKeyword "LEFT" then
                    joinType <- Some "LEFT"
                    this.MatchKeyword "OUTER" |> ignore
                    this.ExpectKeyword "JOIN"
                elif this.MatchKeyword "CROSS" then
                    joinType <- Some "CROSS"
                    this.ExpectKeyword "JOIN"
                elif this.MatchKeyword "JOIN" then
                    joinType <- Some "INNER"

                match joinType with
                | None -> parsing <- false
                | Some kind ->
                    let table = this.ParseTableRef()
                    let on =
                        if kind = "CROSS" then
                            None
                        else
                            this.ExpectKeyword "ON"
                            Some(this.ParseExpression())
                    joins.Add({ JoinType = kind; Table = table; On = on })
            joins |> Seq.toList

        member private this.ParseExpressionList() =
            let expressions = ResizeArray<Expr>()
            let mutable keepGoing = true
            while keepGoing do
                expressions.Add(this.ParseExpression())
                keepGoing <- this.MatchSymbol ","
            expressions |> Seq.toList

        member private this.ParseOrderList() =
            let items = ResizeArray<OrderItem>()
            let mutable keepGoing = true
            while keepGoing do
                let expression = this.ParseExpression()
                let descending =
                    if this.MatchKeyword "ASC" then false
                    elif this.MatchKeyword "DESC" then true
                    else false
                items.Add({ Expression = expression; Descending = descending })
                keepGoing <- this.MatchSymbol ","
            items |> Seq.toList

        member private this.ParseExpression() = this.ParseOr()

        member private this.ParseOr() =
            let mutable left = this.ParseAnd()
            while this.MatchKeyword "OR" do
                left <- Binary("OR", left, this.ParseAnd())
            left

        member private this.ParseAnd() =
            let mutable left = this.ParseNot()
            while this.MatchKeyword "AND" do
                left <- Binary("AND", left, this.ParseNot())
            left

        member private this.ParseNot() =
            if this.MatchKeyword "NOT" then
                Unary("NOT", this.ParseNot())
            else
                this.ParseComparison()

        member private this.ParseComparison() =
            let left = this.ParseAdditive()
            if this.MatchKeyword "IS" then
                let negated = this.MatchKeyword "NOT"
                this.ExpectKeyword "NULL"
                IsNullExpr(left, negated)
            elif this.MatchKeyword "NOT" then
                if this.MatchKeyword "BETWEEN" then
                    let lower = this.ParseAdditive()
                    this.ExpectKeyword "AND"
                    Between(left, lower, this.ParseAdditive(), true)
                elif this.MatchKeyword "IN" then
                    In(left, this.ParseInValues(), true)
                elif this.MatchKeyword "LIKE" then
                    Like(left, this.ParseAdditive(), true)
                else
                    raise (this.Error "expected BETWEEN, IN, or LIKE after NOT")
            elif this.MatchKeyword "BETWEEN" then
                let lower = this.ParseAdditive()
                this.ExpectKeyword "AND"
                Between(left, lower, this.ParseAdditive(), false)
            elif this.MatchKeyword "IN" then
                In(left, this.ParseInValues(), false)
            elif this.MatchKeyword "LIKE" then
                Like(left, this.ParseAdditive(), false)
            elif this.Peek.Kind = Symbol then
                match this.Peek.Value with
                | "=" | "!=" | "<>" | "<" | ">" | "<=" | ">=" ->
                    let operator = (this.Advance()).Value
                    Binary(operator, left, this.ParseAdditive())
                | _ -> left
            else
                left

        member private this.ParseInValues() =
            this.ExpectSymbol "("
            let values = this.ParseExpressionList()
            this.ExpectSymbol ")"
            values

        member private this.ParseAdditive() =
            let mutable left = this.ParseMultiplicative()
            while this.Peek.Kind = Symbol && (this.Peek.Value = "+" || this.Peek.Value = "-") do
                let operator = (this.Advance()).Value
                left <- Binary(operator, left, this.ParseMultiplicative())
            left

        member private this.ParseMultiplicative() =
            let mutable left = this.ParseUnary()
            while this.Peek.Kind = Symbol && (this.Peek.Value = "*" || this.Peek.Value = "/" || this.Peek.Value = "%") do
                let operator = (this.Advance()).Value
                left <- Binary(operator, left, this.ParseUnary())
            left

        member private this.ParseUnary() =
            if this.MatchSymbol "-" then
                Unary("-", this.ParseUnary())
            else
                this.ParsePrimary()

        member private this.ParsePrimary() =
            let token = this.Peek
            if this.MatchSymbol "(" then
                let expression = this.ParseExpression()
                this.ExpectSymbol ")"
                expression
            elif token.Kind = Number then
                this.Advance() |> ignore
                if token.Value.Contains(".") then
                    Literal(box (Double.Parse(token.Value, CultureInfo.InvariantCulture)))
                else
                    Literal(box (Int64.Parse(token.Value, CultureInfo.InvariantCulture)))
            elif token.Kind = String then
                this.Advance() |> ignore
                Literal(box token.Value)
            elif this.MatchKeyword "NULL" then
                Null
            elif this.MatchKeyword "TRUE" then
                Literal(box true)
            elif this.MatchKeyword "FALSE" then
                Literal(box false)
            elif this.MatchSymbol "*" then
                Star
            elif token.Kind = Ident || token.Kind = Keyword then
                let name = (this.Advance()).Value
                if this.MatchSymbol "(" then
                    let args = ResizeArray<Expr>()
                    if not (this.MatchSymbol ")") then
                        if this.MatchSymbol "*" then
                            args.Add Star
                        else
                            args.Add(this.ParseExpression())
                            while this.MatchSymbol "," do
                                args.Add(this.ParseExpression())
                        this.ExpectSymbol ")"
                    Function(name, args |> Seq.toList)
                elif this.MatchSymbol "." then
                    Column(Some name, this.ExpectIdentifier())
                else
                    Column(None, name)
            else
                raise (this.Error("unexpected token: " + token.Value))

    let private isNumber (value: obj) =
        match value with
        | :? byte | :? int16 | :? int | :? int64 | :? single | :? double | :? decimal -> true
        | _ -> false

    let private asDouble (value: obj) =
        match value with
        | :? byte as v -> double v
        | :? int16 as v -> double v
        | :? int as v -> double v
        | :? int64 as v -> double v
        | :? single as v -> double v
        | :? double as v -> v
        | :? decimal as v -> double v
        | _ -> Double.Parse(Convert.ToString(value, CultureInfo.InvariantCulture), CultureInfo.InvariantCulture)

    let private rank (value: obj) =
        if isNull value then 0
        elif value :? bool then 1
        elif isNumber value then 2
        elif value :? string then 3
        else 4

    let private compareSql (left: obj) (right: obj) =
        let rankCompare = compare (rank left) (rank right)
        if rankCompare <> 0 then
            rankCompare
        elif isNull left then
            0
        elif isNumber left && isNumber right then
            compare (asDouble left) (asDouble right)
        else
            String.Compare(Convert.ToString left, Convert.ToString right, StringComparison.Ordinal)

    let private sqlEquals (left: obj) (right: obj) =
        if isNull left || isNull right then
            obj.ReferenceEquals(left, right)
        elif isNumber left && isNumber right then
            compare (asDouble left) (asDouble right) = 0
        else
            Object.Equals(left, right)

    let private truthy (value: obj) =
        if isNull value then false
        else
            match value with
            | :? bool as v -> v
            | _ when isNumber value -> asDouble value <> 0.0
            | :? string as v -> v.Length > 0
            | _ -> true

    let private like (value: string) (pattern: string) =
        let regex =
            pattern
            |> Seq.map (fun ch ->
                match ch with
                | '%' -> ".*"
                | '_' -> "."
                | _ -> Regex.Escape(string ch))
            |> String.concat ""
        Regex.IsMatch(value, "^" + regex + "$")

    let private serializeValue (value: obj) =
        if isNull value then
            "<NULL>"
        else
            sprintf "%s:%O" (value.GetType().FullName) value

    let private serializeRow (row: obj array) =
        row |> Array.map serializeValue |> String.concat "\u001f"

    let private expressionLabel expression =
        match expression with
        | Column(_, name) -> name
        | Function(name, [ Star ]) -> name.ToUpperInvariant() + "(*)"
        | Function(name, _) -> name.ToUpperInvariant() + "(...)"
        | Literal value -> Convert.ToString value
        | _ -> "?"

    let private scanTable (dataSource: IDataSource) (tableName: string) (alias: string) =
        let schema = dataSource.Schema tableName
        let rawRows = dataSource.Scan tableName
        rawRows
        |> Array.map (fun raw ->
            let values = Dictionary<string, obj>(StringComparer.Ordinal)
            for column in schema do
                let value =
                    match raw.TryGetValue column with
                    | true, v -> v
                    | false, _ -> null
                values[column] <- value
                values[alias + "." + column] <- value
                values[tableName + "." + column] <- value
            { Values = values })
        |> Array.toList

    let private mergeRows left right =
        let values = Dictionary<string, obj>(left.Values, StringComparer.Ordinal)
        for entry in right.Values do
            values[entry.Key] <- entry.Value
        { Values = values }

    let rec private eval expression (row: Dictionary<string, obj>) (groupRows: RowContext list option) : obj =
        match expression with
        | Literal value -> value
        | Null -> null
        | Column(table, name) ->
            match table with
            | Some tableName ->
                match row.TryGetValue(tableName + "." + name) with
                | true, value -> value
                | false, _ -> null
            | None ->
                match row.TryGetValue name with
                | true, value -> value
                | false, _ ->
                    row
                    |> Seq.tryFind (fun entry -> entry.Key.EndsWith("." + name, StringComparison.Ordinal))
                    |> Option.map (fun entry -> entry.Value)
                    |> Option.defaultValue null
        | Unary(operator, inner) ->
            let value = eval inner row groupRows
            match operator with
            | "NOT" -> if isNull value then null else box (not (truthy value))
            | "-" -> if isNull value then null else box (-asDouble value)
            | _ -> raise (SqlExecutionException("unknown unary operator: " + operator))
        | Binary(operator, left, right) -> evalBinary operator left right row groupRows
        | IsNullExpr(inner, negated) ->
            let result = isNull (eval inner row groupRows)
            box (if negated then not result else result)
        | Between(inner, lower, upper, negated) ->
            let value = eval inner row groupRows
            let lo = eval lower row groupRows
            let hi = eval upper row groupRows
            if isNull value || isNull lo || isNull hi then
                null
            else
                let result = compareSql value lo >= 0 && compareSql value hi <= 0
                box (if negated then not result else result)
        | In(inner, values, negated) ->
            let value = eval inner row groupRows
            if isNull value then
                null
            else
                let found =
                    values
                    |> List.exists (fun option ->
                        let optionValue = eval option row groupRows
                        not (isNull optionValue) && sqlEquals value optionValue)
                box (if negated then not found else found)
        | Like(inner, pattern, negated) ->
            let value = eval inner row groupRows
            let patternValue = eval pattern row groupRows
            if isNull value || isNull patternValue then
                null
            else
                let result = like (Convert.ToString value) (Convert.ToString patternValue)
                box (if negated then not result else result)
        | Function(name, args) -> evalFunction name args row groupRows
        | Star -> box row

    and private evalBinary operator left right row groupRows =
        match operator with
        | "AND" ->
            let leftValue = eval left row groupRows
            if not (isNull leftValue) && not (truthy leftValue) then
                box false
            else
                let rightValue = eval right row groupRows
                if not (isNull rightValue) && not (truthy rightValue) then
                    box false
                elif isNull leftValue || isNull rightValue then
                    null
                else
                    box true
        | "OR" ->
            let leftValue = eval left row groupRows
            if not (isNull leftValue) && truthy leftValue then
                box true
            else
                let rightValue = eval right row groupRows
                if not (isNull rightValue) && truthy rightValue then
                    box true
                elif isNull leftValue || isNull rightValue then
                    null
                else
                    box false
        | _ ->
            let leftValue = eval left row groupRows
            let rightValue = eval right row groupRows
            if isNull leftValue || isNull rightValue then
                null
            else
                match operator with
                | "+" -> box (asDouble leftValue + asDouble rightValue)
                | "-" -> box (asDouble leftValue - asDouble rightValue)
                | "*" -> box (asDouble leftValue * asDouble rightValue)
                | "/" -> box (asDouble leftValue / asDouble rightValue)
                | "%" -> box (asDouble leftValue % asDouble rightValue)
                | "=" -> box (sqlEquals leftValue rightValue)
                | "!=" | "<>" -> box (not (sqlEquals leftValue rightValue))
                | "<" -> box (compareSql leftValue rightValue < 0)
                | ">" -> box (compareSql leftValue rightValue > 0)
                | "<=" -> box (compareSql leftValue rightValue <= 0)
                | ">=" -> box (compareSql leftValue rightValue >= 0)
                | _ -> raise (SqlExecutionException("unknown operator: " + operator))

    and private evalFunction name args row groupRows =
        let upper = name.ToUpperInvariant()
        if Set.contains upper (set [ "COUNT"; "SUM"; "AVG"; "MIN"; "MAX" ]) then
            match groupRows with
            | None -> raise (SqlExecutionException("aggregate used outside grouped context: " + upper))
            | Some rows ->
                if upper = "COUNT" then
                    match args with
                    | [ Star ] -> box rows.Length
                    | first :: _ ->
                        rows
                        |> List.filter (fun groupRow -> not (isNull (eval first groupRow.Values None)))
                        |> List.length
                        |> box
                    | [] -> box rows.Length
                else
                    let first =
                        match args with
                        | arg :: _ -> arg
                        | [] -> raise (SqlExecutionException("aggregate requires an argument: " + upper))
                    let values =
                        rows
                        |> List.map (fun groupRow -> eval first groupRow.Values None)
                        |> List.filter (fun value -> not (isNull value))
                    if List.isEmpty values then
                        null
                    else
                        match upper with
                        | "SUM" -> values |> List.sumBy asDouble |> box
                        | "AVG" -> values |> List.averageBy asDouble |> box
                        | "MIN" -> values |> List.reduce (fun best next -> if compareSql next best < 0 then next else best)
                        | "MAX" -> values |> List.reduce (fun best next -> if compareSql next best > 0 then next else best)
                        | _ -> raise (SqlExecutionException("unknown aggregate: " + upper))
        else
            let value =
                match args with
                | first :: _ -> eval first row groupRows
                | [] -> null
            if isNull value then
                null
            else
                match upper with
                | "UPPER" -> box ((Convert.ToString value).ToUpperInvariant())
                | "LOWER" -> box ((Convert.ToString value).ToLowerInvariant())
                | "LENGTH" -> box ((Convert.ToString value).Length)
                | _ -> raise (SqlExecutionException("unknown function: " + upper))

    let rec private hasAggregate expression =
        match expression with
        | Function(name, _) when Set.contains (name.ToUpperInvariant()) (set [ "COUNT"; "SUM"; "AVG"; "MIN"; "MAX" ]) -> true
        | Binary(_, left, right) -> hasAggregate left || hasAggregate right
        | Unary(_, inner) -> hasAggregate inner
        | IsNullExpr(inner, _) -> hasAggregate inner
        | Between(value, lower, upper, _) -> hasAggregate value || hasAggregate lower || hasAggregate upper
        | In(value, values, _) -> hasAggregate value || List.exists hasAggregate values
        | Like(value, pattern, _) -> hasAggregate value || hasAggregate pattern
        | Function(_, args) -> List.exists hasAggregate args
        | _ -> false

    let private applyJoin leftRows rightRows join =
        if join.JoinType = "CROSS" then
            [ for left in leftRows do
                for right in rightRows do
                    yield mergeRows left right ]
        else
            [ for left in leftRows do
                let mutable matched = false
                for right in rightRows do
                    let merged = mergeRows left right
                    let keep =
                        match join.On with
                        | None -> true
                        | Some expression -> truthy (eval expression merged.Values None)
                    if keep then
                        matched <- true
                        yield merged
                if not matched && join.JoinType = "LEFT" then
                    yield left ]

    let private makeFrames rows statement =
        let grouped = not (List.isEmpty statement.GroupBy)
        let aggregated =
            statement.SelectItems |> List.exists (fun item -> hasAggregate item.Expression)
            || (statement.Having |> Option.exists hasAggregate)

        if not grouped && not aggregated then
            rows |> List.map (fun row -> { Row = row; GroupRows = None })
        elif not grouped then
            let row =
                match rows with
                | first :: _ -> first
                | [] -> { Values = Dictionary<string, obj>(StringComparer.Ordinal) }
            [ { Row = row; GroupRows = Some rows } ]
        else
            let groups = Dictionary<string, ResizeArray<RowContext>>(StringComparer.Ordinal)
            for row in rows do
                let key = statement.GroupBy |> List.map (fun expr -> eval expr row.Values None) |> List.toArray |> serializeRow
                match groups.TryGetValue key with
                | true, bucket -> bucket.Add row
                | false, _ ->
                    let bucket = ResizeArray<RowContext>()
                    bucket.Add row
                    groups[key] <- bucket

            groups.Values
            |> Seq.map (fun bucket ->
                let groupRows = bucket |> Seq.toList
                { Row = groupRows.Head; GroupRows = Some groupRows })
            |> Seq.toList

    let private compareOrder (left: RowFrame) (right: RowFrame) (orderBy: OrderItem array) =
        let mutable result = 0
        let mutable index = 0
        while result = 0 && index < orderBy.Length do
            let item = orderBy[index]
            let cmp =
                compareSql
                    (eval item.Expression left.Row.Values left.GroupRows)
                    (eval item.Expression right.Row.Values right.GroupRows)
            result <- if item.Descending then -cmp else cmp
            index <- index + 1
        result

    let private project frames statement =
        match statement.SelectItems with
        | [ { Expression = Star } ] ->
            let columns =
                match frames with
                | [] -> [||]
                | first :: _ ->
                    first.Row.Values.Keys
                    |> Seq.filter (fun key -> not (key.Contains(".", StringComparison.Ordinal)))
                    |> Seq.sort
                    |> Seq.toArray
            let rows =
                frames
                |> List.map (fun frame -> columns |> Array.map (fun column -> frame.Row.Values[column]))
                |> List.toArray
            columns, rows
        | items ->
            let columns =
                items
                |> List.map (fun item -> item.Alias |> Option.defaultValue (expressionLabel item.Expression))
                |> List.toArray
            let rows =
                frames
                |> List.map (fun frame ->
                    items
                    |> List.map (fun item -> eval item.Expression frame.Row.Values frame.GroupRows)
                    |> List.toArray)
                |> List.toArray
            columns, rows

    let private executeSelect statement dataSource =
        let mutable rows = scanTable dataSource statement.From.Name statement.From.Alias
        for join in statement.Joins do
            let right = scanTable dataSource join.Table.Name join.Table.Alias
            rows <- applyJoin rows right join

        match statement.Where with
        | Some where -> rows <- rows |> List.filter (fun row -> truthy (eval where row.Values None))
        | None -> ()

        let mutable frames = makeFrames rows statement
        match statement.Having with
        | Some having -> frames <- frames |> List.filter (fun frame -> truthy (eval having frame.Row.Values frame.GroupRows))
        | None -> ()

        if not (List.isEmpty statement.OrderBy) then
            frames <- frames |> List.sortWith (fun left right -> compareOrder left right (statement.OrderBy |> List.toArray))

        let columns, projected = project frames statement
        let mutable rowsOut = projected

        if statement.Distinct then
            let seen = HashSet<string>(StringComparer.Ordinal)
            rowsOut <-
                rowsOut
                |> Array.filter (fun row ->
                    let key = serializeRow row
                    if seen.Contains key then
                        false
                    else
                        seen.Add key |> ignore
                        true)

        let offset = statement.Offset |> Option.defaultValue 0 |> max 0
        let limit = statement.Limit |> Option.map (max 0)
        rowsOut <-
            if offset >= rowsOut.Length then
                [||]
            else
                let count =
                    match limit with
                    | Some value -> min value (rowsOut.Length - offset)
                    | None -> rowsOut.Length - offset
                rowsOut[offset .. offset + count - 1]

        { Columns = columns; Rows = rowsOut }

    let execute (sql: string) (dataSource: IDataSource) =
        try
            let parser = Parser(tokenize sql)
            executeSelect (parser.ParseStatement()) dataSource
        with
        | :? SqlExecutionException as ex -> raise ex
        | ex -> raise (SqlExecutionException(ex.Message, ex))

    let tryExecute (sql: string) (dataSource: IDataSource) =
        try
            { Ok = true
              Result = Some(execute sql dataSource)
              Error = None }
        with ex ->
            { Ok = false
              Result = None
              Error = Some ex.Message }
