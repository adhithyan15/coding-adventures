namespace CodingAdventures.MiniSqlite.FSharp

open System
open System.Collections.Generic
open System.Globalization
open System.Text
open System.Text.RegularExpressions

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
          LastRowId = null }

type private Table =
    { Columns: string list
      Rows: ResizeArray<Dictionary<string, obj>>
      mutable NextRowId: int64 }

type private Database = { Tables: Dictionary<string, Table> }

module private Database =
    let empty () =
        { Tables = Dictionary<string, Table>(StringComparer.OrdinalIgnoreCase) }

    let private copyRow (row: Dictionary<string, obj>) =
        Dictionary<string, obj>(row, StringComparer.OrdinalIgnoreCase)

    let copy (db: Database) =
        let tables = Dictionary<string, Table>(StringComparer.OrdinalIgnoreCase)
        for KeyValue(name, table) in db.Tables do
            tables[name] <-
                { Columns = table.Columns
                  Rows = ResizeArray(table.Rows |> Seq.map copyRow)
                  NextRowId = table.NextRowId }

        { Tables = tables }

    let requireTable name (db: Database) =
        match db.Tables.TryGetValue(name) with
        | true, table -> table
        | _ -> raise (MiniSqliteException("OperationalError", $"no such table: {name}"))

module private SqlText =
    let private invariant = CultureInfo.InvariantCulture

    let trim (sql: string) =
        let trimmed = sql.Trim()
        if trimmed.EndsWith(";", StringComparison.Ordinal) then
            trimmed.Substring(0, trimmed.Length - 1).Trim()
        else
            trimmed

    let firstKeyword (sql: string) =
        let m = Regex.Match(sql.TrimStart(), "^[A-Za-z]+")
        if m.Success then m.Value.ToUpperInvariant() else ""

    let private isIdentifierChar ch =
        Char.IsLetterOrDigit(ch) || ch = '_'

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
                    if i + 1 < text.Length && text[i + 1] = quote then
                        i <- i + 1
                    else
                        quote <- '\000'
            else
                match ch with
                | '\''
                | '"' -> quote <- ch
                | '(' -> depth <- depth + 1
                | ')' when depth > 0 -> depth <- depth - 1
                | _ when ch = separator && depth = 0 ->
                    parts.Add(text.Substring(start, i - start).Trim())
                    start <- i + 1
                | _ -> ()

            i <- i + 1

        parts.Add(text.Substring(start).Trim())
        parts |> Seq.filter (String.IsNullOrWhiteSpace >> not) |> Seq.toList

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
                    if i + 1 < text.Length && text[i + 1] = quote then
                        i <- i + 1
                    else
                        quote <- '\000'
            else
                match ch with
                | '\''
                | '"' -> quote <- ch
                | '(' -> depth <- depth + 1
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
        | :? string as text -> "'" + text.Replace("'", "''") + "'"
        | :? bool as flag -> if flag then "TRUE" else "FALSE"
        | :? IFormattable as formattable -> formattable.ToString(null, invariant)
        | other -> "'" + other.ToString().Replace("'", "''") + "'"

    let bindParameters (sql: string) (parameters: IReadOnlyList<obj>) =
        let output = StringBuilder()
        let mutable parameterIndex = 0
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
                    else
                        quote <- '\000'
            elif ch = '\'' || ch = '"' then
                quote <- ch
                output.Append(ch) |> ignore
            elif ch = '-' && i + 1 < sql.Length && sql[i + 1] = '-' then
                while i < sql.Length && sql[i] <> '\n' do
                    output.Append(sql[i]) |> ignore
                    i <- i + 1
                if i < sql.Length then
                    output.Append(sql[i]) |> ignore
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
                if parameterIndex >= parameters.Count then
                    raise (MiniSqliteException("ProgrammingError", "not enough query parameters"))

                output.Append(formatParameter parameters[parameterIndex]) |> ignore
                parameterIndex <- parameterIndex + 1
            else
                output.Append(ch) |> ignore

            i <- i + 1

        if parameterIndex <> parameters.Count then
            raise (MiniSqliteException("ProgrammingError", "too many query parameters"))

        output.ToString()

module private SqlValue =
    let isNull (value: obj) =
        obj.ReferenceEquals(value, null) || value :? DBNull

    let private tryDecimal (value: obj) =
        match value with
        | null -> None
        | :? byte as v -> Some(decimal v)
        | :? int16 as v -> Some(decimal v)
        | :? int as v -> Some(decimal v)
        | :? int64 as v -> Some(decimal v)
        | :? sbyte as v -> Some(decimal v)
        | :? uint16 as v -> Some(decimal v)
        | :? uint32 as v -> Some(decimal v)
        | :? uint64 as v -> Some(decimal v)
        | :? single as v when Single.IsFinite(v) -> Some(decimal v)
        | :? double as v when Double.IsFinite(v) -> Some(decimal v)
        | :? decimal as v -> Some(v)
        | _ -> None

    let private unquote (text: string) =
        if text.Length >= 2 && ((text[0] = '\'' && text[text.Length - 1] = '\'') || (text[0] = '"' && text[text.Length - 1] = '"')) then
            let quote = text[0].ToString()
            text.Substring(1, text.Length - 2).Replace(quote + quote, quote) :> obj
        else
            null

    let parseLiteral (token: string) =
        let text = token.Trim()
        let quoted = unquote text

        if not (isNull quoted) then quoted
        elif text.Equals("NULL", StringComparison.OrdinalIgnoreCase) then null
        elif text.Equals("TRUE", StringComparison.OrdinalIgnoreCase) then box true
        elif text.Equals("FALSE", StringComparison.OrdinalIgnoreCase) then box false
        else
            let mutable integer = 0L
            let mutable floating = 0.0

            if Int64.TryParse(text, NumberStyles.Integer, CultureInfo.InvariantCulture, &integer) then
                box integer
            elif Double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, &floating) then
                box floating
            else
                text :> obj

    let resolve (row: Dictionary<string, obj>) (token: string) =
        let text = token.Trim()
        match row.TryGetValue(text) with
        | true, value -> value
        | _ -> parseLiteral text

    let equals left right =
        if isNull left || isNull right then
            isNull left && isNull right
        else
            match tryDecimal left, tryDecimal right with
            | Some a, Some b -> a = b
            | _ -> obj.Equals(left, right)

    let compare left right =
        if equals left right then
            0
        elif isNull left then
            -1
        elif isNull right then
            1
        else
            match tryDecimal left, tryDecimal right with
            | Some a, Some b -> compare a b
            | _ -> StringComparer.Ordinal.Compare(string left, string right)

module private Conditions =
    let private comparisonPattern = Regex(@"(?is)^\s*(.+?)\s*(=|!=|<>|<=|>=|<|>)\s*(.+?)\s*$", RegexOptions.Compiled)

    let rec matches (whereSql: string option) (row: Dictionary<string, obj>) =
        match whereSql with
        | None -> true
        | Some sql when String.IsNullOrWhiteSpace sql -> true
        | Some sql ->
            SqlText.splitByKeyword "OR" sql
            |> List.exists (fun disjunct ->
                SqlText.splitByKeyword "AND" disjunct
                |> List.forall (fun atom -> matchesAtom atom row))

    and private matchesAtom atom row =
        let text = atom.Trim()
        let isNullMatch = Regex.Match(text, @"(?is)^(.+?)\s+IS\s+(NOT\s+)?NULL$")

        if isNullMatch.Success then
            let value = SqlValue.resolve row isNullMatch.Groups[1].Value
            let negate = isNullMatch.Groups[2].Success
            if negate then not (SqlValue.isNull value) else SqlValue.isNull value
        else
            let inMatch = Regex.Match(text, @"(?is)^(.+?)\s+IN\s*\((.*)\)$")

            if inMatch.Success then
                let left = SqlValue.resolve row inMatch.Groups[1].Value
                SqlText.splitTopLevel ',' inMatch.Groups[2].Value
                |> List.exists (fun valueSql -> SqlValue.equals left (SqlValue.resolve row valueSql))
            else
                let comparison = comparisonPattern.Match(text)

                if not comparison.Success then
                    let value = SqlValue.resolve row text
                    match value with
                    | :? bool as flag -> flag
                    | _ -> not (SqlValue.isNull value)
                else
                    let left = SqlValue.resolve row comparison.Groups[1].Value
                    let right = SqlValue.resolve row comparison.Groups[3].Value

                    match comparison.Groups[2].Value with
                    | "=" -> SqlValue.equals left right
                    | "!="
                    | "<>" -> not (SqlValue.equals left right)
                    | "<" -> SqlValue.compare left right < 0
                    | "<=" -> SqlValue.compare left right <= 0
                    | ">" -> SqlValue.compare left right > 0
                    | ">=" -> SqlValue.compare left right >= 0
                    | _ -> false

module private Statements =
    type CreateStatement = { TableName: string; Columns: string list }
    type InsertStatement = { TableName: string; Columns: string list option; Values: string list }
    type UpdateStatement = { TableName: string; Assignments: (string * string) list; Where: string option }
    type DeleteStatement = { TableName: string; Where: string option }
    type SelectStatement = { TableName: string; Projection: string list; Where: string option; OrderBy: string option }

    let private requireMatch pattern sql =
        let m = Regex.Match(SqlText.trim sql, pattern, RegexOptions.IgnoreCase ||| RegexOptions.Singleline)
        if m.Success then m else raise (ArgumentException("could not parse SQL statement"))

    let private identifierFromColumn (columnSql: string) =
        let parts = Regex.Split(columnSql.Trim(), @"\s+") |> Array.filter (String.IsNullOrWhiteSpace >> not)
        if parts.Length = 0 then
            raise (ArgumentException("empty column definition"))
        parts[0].Trim('"', '\'')

    let parseCreate sql =
        let m = requireMatch @"^\s*CREATE\s+TABLE\s+([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)\s*$" sql
        { TableName = m.Groups[1].Value
          Columns = SqlText.splitTopLevel ',' m.Groups[2].Value |> List.map identifierFromColumn }

    let parseDrop sql =
        let m = requireMatch @"^\s*DROP\s+TABLE\s+([A-Za-z_][A-Za-z0-9_]*)\s*$" sql
        m.Groups[1].Value

    let parseInsert sql =
        let m = requireMatch @"^\s*INSERT\s+INTO\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*?)\))?\s+VALUES\s*\((.*)\)\s*$" sql
        let columns =
            if m.Groups[2].Success then
                Some(SqlText.splitTopLevel ',' m.Groups[2].Value |> List.map identifierFromColumn)
            else
                None

        { TableName = m.Groups[1].Value
          Columns = columns
          Values = SqlText.splitTopLevel ',' m.Groups[3].Value }

    let parseUpdate sql =
        let m = requireMatch @"^\s*UPDATE\s+([A-Za-z_][A-Za-z0-9_]*)\s+SET\s+(.+?)(?:\s+WHERE\s+(.+))?\s*$" sql
        let assignments =
            SqlText.splitTopLevel ',' m.Groups[2].Value
            |> List.map (fun assignment ->
                let pieces = assignment.Split([| '=' |], 2)
                if pieces.Length <> 2 then
                    raise (ArgumentException("invalid assignment"))
                identifierFromColumn pieces[0], pieces[1].Trim())

        { TableName = m.Groups[1].Value
          Assignments = assignments
          Where = if m.Groups[3].Success then Some m.Groups[3].Value else None }

    let parseDelete sql =
        let m = requireMatch @"^\s*DELETE\s+FROM\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s+WHERE\s+(.+))?\s*$" sql
        { TableName = m.Groups[1].Value
          Where = if m.Groups[2].Success then Some m.Groups[2].Value else None }

    let parseSelect sql =
        let m = requireMatch @"^\s*SELECT\s+(.+?)\s+FROM\s+([A-Za-z_][A-Za-z0-9_]*)\s*(.*)\s*$" sql
        let rest = m.Groups[3].Value.Trim()
        let whereSql, orderSql =
            let withWhereAndOrder = Regex.Match(rest, @"(?is)^WHERE\s+(.+?)\s+ORDER\s+BY\s+(.+)$")
            let withWhere = Regex.Match(rest, @"(?is)^WHERE\s+(.+)$")
            let withOrder = Regex.Match(rest, @"(?is)^ORDER\s+BY\s+(.+)$")

            if withWhereAndOrder.Success then
                Some withWhereAndOrder.Groups[1].Value, Some withWhereAndOrder.Groups[2].Value
            elif withWhere.Success then
                Some withWhere.Groups[1].Value, None
            elif withOrder.Success then
                None, Some withOrder.Groups[1].Value
            elif String.IsNullOrWhiteSpace rest then
                None, None
            else
                raise (ArgumentException("could not parse SELECT suffix"))

        { TableName = m.Groups[2].Value
          Projection = SqlText.splitTopLevel ',' m.Groups[1].Value |> List.map identifierFromColumn
          Where = whereSql
          OrderBy = orderSql }

module private Engine =
    let create (statement: Statements.CreateStatement) db =
        if db.Tables.ContainsKey(statement.TableName) then
            raise (MiniSqliteException("OperationalError", $"table already exists: {statement.TableName}"))

        db.Tables.Add(
            statement.TableName,
            { Columns = statement.Columns
              Rows = ResizeArray()
              NextRowId = 1L }
        )
        ExecutionResult.empty 0

    let drop tableName db =
        if not (db.Tables.Remove(tableName)) then
            raise (MiniSqliteException("OperationalError", $"no such table: {tableName}"))
        ExecutionResult.empty 0

    let insert (statement: Statements.InsertStatement) db =
        let table = Database.requireTable statement.TableName db
        let columns = defaultArg statement.Columns table.Columns

        if columns.Length <> statement.Values.Length then
            raise (MiniSqliteException("ProgrammingError", "column/value count mismatch"))

        let row = Dictionary<string, obj>(StringComparer.OrdinalIgnoreCase)
        for column in table.Columns do
            row[column] <- null

        for column, valueSql in List.zip columns statement.Values do
            row[column] <- SqlValue.parseLiteral valueSql

        let rowId = table.NextRowId
        table.NextRowId <- table.NextRowId + 1L
        table.Rows.Add(row)
        { ExecutionResult.empty 1 with LastRowId = box rowId }

    let update (statement: Statements.UpdateStatement) db =
        let table = Database.requireTable statement.TableName db
        let mutable count = 0

        for row in table.Rows do
            if Conditions.matches statement.Where row then
                for column, valueSql in statement.Assignments do
                    row[column] <- SqlValue.parseLiteral valueSql
                count <- count + 1

        ExecutionResult.empty count

    let delete (statement: Statements.DeleteStatement) db =
        let table = Database.requireTable statement.TableName db
        let mutable count = 0

        for i in table.Rows.Count - 1 .. -1 .. 0 do
            if Conditions.matches statement.Where table.Rows[i] then
                table.Rows.RemoveAt(i)
                count <- count + 1

        ExecutionResult.empty count

    let private applyOrder (order: string option) (rows: Dictionary<string, obj> list) =
        match order with
        | None -> rows
        | Some orderSql ->
            let parts = Regex.Split(orderSql.Trim(), @"\s+") |> Array.filter (String.IsNullOrWhiteSpace >> not)
            if parts.Length = 0 then
                rows
            else
                let column = parts[0]
                let descending = parts.Length > 1 && parts[1].Equals("DESC", StringComparison.OrdinalIgnoreCase)
                let sorted = rows |> List.sortWith (fun left right -> SqlValue.compare (SqlValue.resolve left column) (SqlValue.resolve right column))
                if descending then List.rev sorted else sorted

    let select (statement: Statements.SelectStatement) db =
        let table = Database.requireTable statement.TableName db
        let projection =
            match statement.Projection with
            | [ "*" ] -> table.Columns
            | columns -> columns

        let rows =
            table.Rows
            |> Seq.filter (Conditions.matches statement.Where)
            |> Seq.toList
            |> applyOrder statement.OrderBy
            |> List.map (fun row ->
                projection
                |> List.map (fun column ->
                    match row.TryGetValue(column) with
                    | true, value -> value
                    | _ -> null)
                :> IReadOnlyList<obj>)

        { Columns = projection
          Rows = rows
          RowCount = -1
          LastRowId = null }

type Connection(options: ConnectionOptions) as this =
    let autocommit = options.Autocommit
    let mutable db = Database.empty()
    let mutable snapshot: Database option = None
    let mutable closed = false

    let assertOpen () =
        if closed then
            raise (MiniSqliteException("ProgrammingError", "connection is closed"))

    let ensureSnapshot () =
        if not autocommit && snapshot.IsNone then
            snapshot <- Some(Database.copy db)

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
        snapshot <- None

    member _.Rollback() =
        assertOpen ()
        match snapshot with
        | Some original ->
            db <- Database.copy original
            snapshot <- None
        | None -> ()

    member internal _.ExecuteBound(sql: string, parameters: IReadOnlyList<obj>) =
        assertOpen ()
        let bound = SqlText.bindParameters sql parameters

        try
            match SqlText.firstKeyword bound with
            | "BEGIN" ->
                ensureSnapshot ()
                ExecutionResult.empty 0
            | "COMMIT" ->
                snapshot <- None
                ExecutionResult.empty 0
            | "ROLLBACK" ->
                match snapshot with
                | Some original -> db <- Database.copy original
                | None -> ()
                snapshot <- None
                ExecutionResult.empty 0
            | "SELECT" -> Engine.select (Statements.parseSelect bound) db
            | "CREATE" ->
                ensureSnapshot ()
                Engine.create (Statements.parseCreate bound) db
            | "DROP" ->
                ensureSnapshot ()
                Engine.drop (Statements.parseDrop bound) db
            | "INSERT" ->
                ensureSnapshot ()
                Engine.insert (Statements.parseInsert bound) db
            | "UPDATE" ->
                ensureSnapshot ()
                Engine.update (Statements.parseUpdate bound) db
            | "DELETE" ->
                ensureSnapshot ()
                Engine.delete (Statements.parseDelete bound) db
            | _ -> raise (ArgumentException("unsupported SQL statement"))
        with
        | :? MiniSqliteException -> reraise ()
        | ex -> raise (MiniSqliteException("OperationalError", ex.Message))

    interface IDisposable with
        member _.Dispose() =
            if not closed then
                match snapshot with
                | Some original -> db <- Database.copy original
                | None -> ()

                snapshot <- None
                closed <- true

and Cursor internal (connection: Connection) =
    let mutable rows: IReadOnlyList<obj> array = [||]
    let mutable offset = 0
    let mutable description: Column array = [||]
    let mutable rowCount = -1
    let mutable lastRowId: obj = null
    let mutable arraySize = 1
    let mutable closed = false

    let assertOpen () =
        if closed then
            raise (MiniSqliteException("ProgrammingError", "cursor is closed"))

    member _.Description = description :> IReadOnlyList<Column>
    member _.RowCount = rowCount
    member _.LastRowId = lastRowId

    member _.ArraySize
        with get () = arraySize
        and set value = arraySize <- value

    member this.Execute(sql: string, [<ParamArray>] parameters: obj[]) =
        this.Execute(sql, parameters :> IReadOnlyList<obj>)

    member this.Execute(sql: string, parameters: IReadOnlyList<obj>) =
        assertOpen ()
        let result = connection.ExecuteBound(sql, parameters)
        rows <- result.Rows |> List.toArray
        offset <- 0
        description <- result.Columns |> List.map (fun name -> { Name = name }) |> List.toArray
        rowCount <- result.RowCount
        lastRowId <- result.LastRowId
        this

    member this.ExecuteMany(sql: string, parameterSets: seq<IReadOnlyList<obj>>) =
        let mutable last: Cursor = this
        for parameters in parameterSets do
            last <- this.Execute(sql, parameters)
        last

    member _.FetchOne() : IReadOnlyList<obj> =
        assertOpen ()
        if offset >= rows.Length then
            null
        else
            let row = rows[offset]
            offset <- offset + 1
            row

    member this.FetchMany() = this.FetchMany(arraySize)

    member _.FetchMany(size: int) : IReadOnlyList<IReadOnlyList<obj>> =
        assertOpen ()
        let limit = max 0 size
        let output = ResizeArray<IReadOnlyList<obj>>()
        let mutable consumed = 0

        while consumed < limit && offset < rows.Length do
            output.Add(rows[offset])
            offset <- offset + 1
            consumed <- consumed + 1

        output :> IReadOnlyList<IReadOnlyList<obj>>

    member _.FetchAll() : IReadOnlyList<IReadOnlyList<obj>> =
        assertOpen ()
        let output = ResizeArray<IReadOnlyList<obj>>()

        while offset < rows.Length do
            output.Add(rows[offset])
            offset <- offset + 1

        output :> IReadOnlyList<IReadOnlyList<obj>>

    interface IDisposable with
        member _.Dispose() = closed <- true

/// Entry point for the Level 0 in-memory mini-sqlite facade.
type MiniSqlite =
    static member ApiLevel = "2.0"
    static member ThreadSafety = 1
    static member ParamStyle = "qmark"

    static member Connect(database: string, ?options: ConnectionOptions) =
        if database <> ":memory:" then
            raise (MiniSqliteException("NotSupportedError", "F# mini-sqlite supports only :memory: in Level 0"))

        new Connection(defaultArg options ConnectionOptions.Default)
