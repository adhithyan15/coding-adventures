namespace CodingAdventures.SqlBackend.FSharp

open System
open System.Collections.Generic
open System.Globalization

[<Struct>]
type TransactionHandle = { Value: int }

module SqlValues =
    let private isNull (value: obj) = obj.ReferenceEquals(value, null)

    let private isInteger (value: obj) =
        match value with
        | :? byte
        | :? sbyte
        | :? int16
        | :? uint16
        | :? int
        | :? uint32
        | :? int64
        | :? uint64 -> true
        | _ -> false

    let private isReal (value: obj) =
        match value with
        | :? single
        | :? double -> true
        | _ -> false

    let isSqlValue (value: obj) =
        isNull value
        || value :? bool
        || value :? string
        || value :? byte[]
        || isInteger value
        || isReal value

    let typeName (value: obj) =
        if isNull value then "NULL"
        elif value :? bool then "BOOLEAN"
        elif isInteger value then "INTEGER"
        elif isReal value then "REAL"
        elif value :? string then "TEXT"
        elif value :? byte[] then "BLOB"
        else invalidArg "value" $"not a SQL value: {value.GetType().Name}"

    let private rank (value: obj) =
        if isNull value then 0
        elif value :? bool then 1
        elif isInteger value || isReal value then 2
        elif value :? string then 3
        elif value :? byte[] then 4
        else 5

    let private compareBytes (left: byte[]) (right: byte[]) =
        let length = min left.Length right.Length
        let mutable i = 0
        let mutable cmp = 0

        while cmp = 0 && i < length do
            cmp <- compare left[i] right[i]
            i <- i + 1

        if cmp <> 0 then cmp else compare left.Length right.Length

    let compareValues (left: obj) (right: obj) =
        let rankCompare = compare (rank left) (rank right)

        if rankCompare <> 0 then
            rankCompare
        else
            match left, right with
            | null, null -> 0
            | :? bool as l, (:? bool as r) -> compare l r
            | :? string as l, (:? string as r) -> String.CompareOrdinal(l, r)
            | :? (byte[]) as l, (:? (byte[]) as r) -> compareBytes l r
            | _ when (isInteger left || isReal left) && (isInteger right || isReal right) ->
                compare (Convert.ToDouble(left, CultureInfo.InvariantCulture)) (Convert.ToDouble(right, CultureInfo.InvariantCulture))
            | _ -> String.CompareOrdinal(string left, string right)

[<AllowNullLiteral>]
type Row() =
    inherit Dictionary<string, obj>(StringComparer.OrdinalIgnoreCase)

    new(values: seq<KeyValuePair<string, obj>>) as this =
        Row()
        then
            for KeyValue(key, value) in values do
                this[key] <- value

    member this.Copy() = Row(this :> seq<KeyValuePair<string, obj>>)

type IRowIterator =
    abstract Next: unit -> Row
    abstract Close: unit -> unit

type ICursor =
    inherit IRowIterator
    abstract CurrentRow: Row

type ListRowIterator(rows: seq<Row>) =
    let rows = rows |> Seq.map (fun row -> row.Copy()) |> Seq.toArray
    let mutable index = 0
    let mutable closed = false

    interface IRowIterator with
        member _.Next() =
            if closed || index >= rows.Length then
                null
            else
                let row = rows[index].Copy()
                index <- index + 1
                row

        member _.Close() = closed <- true

type ListCursor(rows: ResizeArray<Row>) =
    let mutable index = -1
    let mutable current: Row = null
    let mutable closed = false

    member internal _.CurrentIndex = index
    member internal _.IsBackedBy(otherRows: ResizeArray<Row>) = obj.ReferenceEquals(rows, otherRows)

    member internal _.AdjustAfterDelete() =
        index <- index - 1
        current <- null

    interface ICursor with
        member _.CurrentRow =
            if obj.ReferenceEquals(current, null) then null else current.Copy()

        member _.Next() =
            if closed then
                null
            else
                index <- index + 1

                if index >= rows.Count then
                    current <- null
                    null
                else
                    current <- rows[index]
                    current.Copy()

        member _.Close() = closed <- true

type ColumnDef
    (
        name: string,
        typeName: string,
        ?notNull: bool,
        ?primaryKey: bool,
        ?unique: bool,
        ?autoincrement: bool,
        ?defaultValue: obj,
        ?hasDefault: bool,
        ?checkExpression: obj,
        ?foreignKey: obj
    ) =
    member _.Name = name
    member _.TypeName = typeName
    member _.NotNull = defaultArg notNull false
    member _.PrimaryKey = defaultArg primaryKey false
    member _.Unique = defaultArg unique false
    member _.Autoincrement = defaultArg autoincrement false
    member _.DefaultValue = defaultArg defaultValue null
    member _.HasDefault = defaultArg hasDefault false
    member _.CheckExpression = defaultArg checkExpression null
    member _.ForeignKey = defaultArg foreignKey null
    member this.EffectiveNotNull = this.NotNull || this.PrimaryKey
    member this.EffectiveUnique = this.Unique || this.PrimaryKey

    static member WithDefault(name: string, typeName: string, defaultValue: obj, ?notNull: bool, ?primaryKey: bool, ?unique: bool, ?autoincrement: bool) =
        ColumnDef(
            name,
            typeName,
            ?notNull = notNull,
            ?primaryKey = primaryKey,
            ?unique = unique,
            ?autoincrement = autoincrement,
            defaultValue = defaultValue,
            hasDefault = true
        )

type TriggerDef(name: string, table: string, timing: string, event: string, body: string) =
    member _.Name = name
    member _.Table = table
    member _.Timing = timing
    member _.Event = event
    member _.Body = body

type IndexDef(name: string, table: string, ?columns: seq<string>, ?unique: bool, ?auto: bool) =
    member _.Name = name
    member _.Table = table
    member _.Columns = defaultArg columns Seq.empty |> Seq.toArray :> IReadOnlyList<string>
    member _.Unique = defaultArg unique false
    member _.Auto = defaultArg auto false

type BackendError(message: string) =
    inherit Exception(message)

type TableNotFound(table: string) =
    inherit BackendError($"table not found: '{table}'")
    member _.Table = table

type TableAlreadyExists(table: string) =
    inherit BackendError($"table already exists: '{table}'")
    member _.Table = table

type ColumnNotFound(table: string, column: string) =
    inherit BackendError($"column not found: '{table}.{column}'")
    member _.Table = table
    member _.Column = column

type ColumnAlreadyExists(table: string, column: string) =
    inherit BackendError($"column already exists: '{table}.{column}'")
    member _.Table = table
    member _.Column = column

type ConstraintViolation(table: string, column: string, detail: string) =
    inherit BackendError(detail)
    member _.Table = table
    member _.Column = column

type Unsupported(operation: string) =
    inherit BackendError($"unsupported operation: {operation}")
    member _.Operation = operation

type Internal(detail: string) =
    inherit BackendError(detail)

type IndexAlreadyExists(index: string) =
    inherit BackendError($"index already exists: '{index}'")
    member _.Index = index

type IndexNotFound(index: string) =
    inherit BackendError($"index not found: '{index}'")
    member _.Index = index

type TriggerAlreadyExists(trigger: string) =
    inherit BackendError($"trigger already exists: '{trigger}'")
    member _.Trigger = trigger

type TriggerNotFound(trigger: string) =
    inherit BackendError($"trigger not found: '{trigger}'")
    member _.Trigger = trigger

[<AbstractClass>]
type Backend() =
    abstract Tables: unit -> IReadOnlyList<string>
    abstract Columns: table: string -> IReadOnlyList<ColumnDef>
    abstract Scan: table: string -> IRowIterator
    abstract Insert: table: string * row: Row -> unit
    abstract Update: table: string * cursor: ICursor * assignments: IReadOnlyDictionary<string, obj> -> unit
    abstract Delete: table: string * cursor: ICursor -> unit
    abstract CreateTable: table: string * columns: IReadOnlyList<ColumnDef> * ifNotExists: bool -> unit
    abstract DropTable: table: string * ifExists: bool -> unit
    abstract AddColumn: table: string * column: ColumnDef -> unit
    abstract CreateIndex: index: IndexDef -> unit
    abstract DropIndex: name: string * ifExists: bool -> unit
    abstract ListIndexes: table: string option -> IReadOnlyList<IndexDef>
    abstract ScanIndex: indexName: string * lo: IReadOnlyList<obj> option * hi: IReadOnlyList<obj> option * loInclusive: bool * hiInclusive: bool -> seq<int>
    abstract ScanByRowIds: table: string * rowids: IReadOnlyList<int> -> IRowIterator
    abstract BeginTransaction: unit -> TransactionHandle
    abstract Commit: handle: TransactionHandle -> unit
    abstract Rollback: handle: TransactionHandle -> unit
    abstract CurrentTransaction: unit -> TransactionHandle option

    default _.CurrentTransaction() = None
    abstract CreateSavepoint: name: string -> unit
    default _.CreateSavepoint(_name: string) = raise (Unsupported("savepoints"))
    abstract ReleaseSavepoint: name: string -> unit
    default _.ReleaseSavepoint(_name: string) = raise (Unsupported("savepoints"))
    abstract RollbackToSavepoint: name: string -> unit
    default _.RollbackToSavepoint(_name: string) = raise (Unsupported("savepoints"))
    abstract CreateTrigger: defn: TriggerDef -> unit
    default _.CreateTrigger(_defn: TriggerDef) = raise (Unsupported("triggers"))
    abstract DropTrigger: name: string * ifExists: bool -> unit
    default _.DropTrigger(_name: string, _ifExists: bool) = raise (Unsupported("triggers"))
    abstract ListTriggers: table: string -> IReadOnlyList<TriggerDef>
    default _.ListTriggers(_table: string) = Array.empty<TriggerDef> :> IReadOnlyList<TriggerDef>

type ISchemaProvider =
    abstract Columns: table: string -> IReadOnlyList<string>

module BackendAdapters =
    let asSchemaProvider (backend: Backend) =
        { new ISchemaProvider with
            member _.Columns(table: string) =
                backend.Columns(table)
                |> Seq.map (fun column -> column.Name)
                |> Seq.toArray
                :> IReadOnlyList<string> }

type private TableState(columns: seq<ColumnDef>, rows: seq<Row>) =
    member val Columns = ResizeArray<ColumnDef>(columns)
    member val Rows = ResizeArray<Row>(rows |> Seq.map (fun row -> row.Copy()))
    member this.Clone() = TableState(this.Columns, this.Rows |> Seq.map (fun row -> row.Copy()))

type private Snapshot =
    { Tables: Dictionary<string, TableState>
      Indexes: Dictionary<string, IndexDef> }

type private KeyComparer() =
    interface IComparer<IReadOnlyList<obj>> with
        member _.Compare(left, right) =
            let length = min left.Count right.Count
            let mutable i = 0
            let mutable cmp = 0

            while cmp = 0 && i < length do
                cmp <- SqlValues.compareValues left[i] right[i]
                i <- i + 1

            if cmp <> 0 then cmp else compare left.Count right.Count

type InMemoryBackend() =
    inherit Backend()

    let tables = Dictionary<string, TableState>(StringComparer.OrdinalIgnoreCase)
    let indexes = Dictionary<string, IndexDef>(StringComparer.OrdinalIgnoreCase)
    let mutable snapshot: Snapshot option = None
    let mutable nextHandle = 1
    let mutable activeHandle: TransactionHandle option = None

    let same left right = String.Equals(left, right, StringComparison.OrdinalIgnoreCase)

    let cloneIndex (index: IndexDef) =
        IndexDef(index.Name, index.Table, columns = index.Columns, unique = index.Unique, auto = index.Auto)

    let requireTable table =
        match tables.TryGetValue(table) with
        | true, state -> state
        | _ -> raise (TableNotFound(table))

    let canonicalColumn table (state: TableState) column =
        state.Columns
        |> Seq.tryFind (fun candidate -> same candidate.Name column)
        |> function
            | Some candidate -> candidate.Name
            | None -> raise (ColumnNotFound(table, column))

    let checkUnknownColumns table (state: TableState) (row: Row) =
        for key in row.Keys do
            if state.Columns |> Seq.exists (fun column -> same column.Name key) |> not then
                raise (ColumnNotFound(table, key))

    let checkNotNull table (state: TableState) (row: Row) =
        for column in state.Columns do
            if column.EffectiveNotNull then
                match row.TryGetValue(column.Name) with
                | true, value when not (obj.ReferenceEquals(value, null)) -> ()
                | _ -> raise (ConstraintViolation(table, column.Name, $"NOT NULL constraint failed: {table}.{column.Name}"))

    let checkUnique table (state: TableState) (row: Row) (ignoreIndex: int option) =
        for column in state.Columns do
            if column.EffectiveUnique then
                match row.TryGetValue(column.Name) with
                | true, value when not (obj.ReferenceEquals(value, null)) ->
                    for i in 0 .. state.Rows.Count - 1 do
                        if ignoreIndex <> Some i then
                            match state.Rows[i].TryGetValue(column.Name) with
                            | true, existing when Object.Equals(existing, value) ->
                                let label = if column.PrimaryKey then "PRIMARY KEY" else "UNIQUE"
                                raise (ConstraintViolation(table, column.Name, $"{label} constraint failed: {table}.{column.Name}"))
                            | _ -> ()
                | _ -> ()

    let applyDefaults table (state: TableState) (row: Row) =
        let normalized = row.Copy()

        for column in state.Columns do
            if not (normalized.ContainsKey(column.Name)) then
                normalized[column.Name] <- if column.HasDefault then column.DefaultValue else null

        checkUnknownColumns table state normalized
        normalized

    let requireListCursor table (state: TableState) (cursor: ICursor) =
        match cursor with
        | :? ListCursor as listCursor when listCursor.IsBackedBy(state.Rows) -> listCursor
        | _ -> raise (Unsupported($"foreign cursor for table {table}"))

    let indexKey (state: TableState) (row: Row) (columns: IReadOnlyList<string>) =
        columns
        |> Seq.map (fun column ->
            let canonical = canonicalColumn "" state column
            match row.TryGetValue(canonical) with
            | true, value -> value
            | _ -> null)
        |> Seq.toArray
        :> IReadOnlyList<obj>

    let comparePrefix (key: IReadOnlyList<obj>) (bound: IReadOnlyList<obj>) =
        let mutable i = 0
        let mutable cmp = 0

        while cmp = 0 && i < bound.Count do
            let value = if i < key.Count then key[i] else null
            cmp <- SqlValues.compareValues value bound[i]
            i <- i + 1

        cmp

    let serializeKey (key: seq<obj>) =
        key
        |> Seq.map (fun value ->
            if obj.ReferenceEquals(value, null) then "NULL"
            else
                match value with
                | :? (byte[]) as bytes -> Convert.ToBase64String(bytes)
                | _ -> Convert.ToString(value, CultureInfo.InvariantCulture))
        |> String.concat "\u001f"

    let capture () =
        let tableCopy = Dictionary<string, TableState>(StringComparer.OrdinalIgnoreCase)
        for KeyValue(name, table) in tables do
            tableCopy[name] <- table.Clone()

        let indexCopy = Dictionary<string, IndexDef>(StringComparer.OrdinalIgnoreCase)
        for KeyValue(name, index) in indexes do
            indexCopy[name] <- cloneIndex index

        { Tables = tableCopy; Indexes = indexCopy }

    let restore snap =
        tables.Clear()
        for KeyValue(name, table) in snap.Tables do
            tables[name] <- table.Clone()

        indexes.Clear()
        for KeyValue(name, index) in snap.Indexes do
            indexes[name] <- cloneIndex index

    let requireActive handle =
        match activeHandle with
        | None -> raise (Unsupported("no active transaction"))
        | Some active when active <> handle -> raise (Unsupported("stale transaction handle"))
        | Some _ -> ()

    static member FromTables(source: seq<string * seq<ColumnDef> * seq<Row>>) =
        let backend = InMemoryBackend()
        for table, columns, rows in source do
            backend.CreateTable(table, columns |> Seq.toArray, false)
            for row in rows do
                backend.Insert(table, row.Copy())
        backend

    override _.Tables() = tables.Keys |> Seq.toArray :> IReadOnlyList<string>

    override _.Columns(table: string) = (requireTable table).Columns |> Seq.toArray :> IReadOnlyList<ColumnDef>

    override _.Scan(table: string) = ListRowIterator((requireTable table).Rows) :> IRowIterator

    member _.OpenCursor(table: string) = ListCursor((requireTable table).Rows)

    override _.Insert(table: string, row: Row) =
        let state = requireTable table
        let normalized = applyDefaults table state row
        checkNotNull table state normalized
        checkUnique table state normalized None
        state.Rows.Add(normalized)

    override _.Update(table: string, cursor: ICursor, assignments: IReadOnlyDictionary<string, obj>) =
        let state = requireTable table
        let listCursor = requireListCursor table state cursor
        let index = listCursor.CurrentIndex

        if index < 0 || index >= state.Rows.Count then
            raise (Unsupported("cursor has no current row"))

        let updated = state.Rows[index].Copy()
        for KeyValue(column, value) in assignments do
            updated[canonicalColumn table state column] <- value

        checkNotNull table state updated
        checkUnique table state updated (Some index)
        state.Rows[index] <- updated

    override _.Delete(table: string, cursor: ICursor) =
        let state = requireTable table
        let listCursor = requireListCursor table state cursor
        let index = listCursor.CurrentIndex

        if index < 0 || index >= state.Rows.Count then
            raise (Unsupported("cursor has no current row"))

        state.Rows.RemoveAt(index)
        listCursor.AdjustAfterDelete()

    override _.CreateTable(table: string, columns: IReadOnlyList<ColumnDef>, ifNotExists: bool) =
        if tables.ContainsKey(table) then
            if not ifNotExists then
                raise (TableAlreadyExists(table))
        else
            let seen = HashSet<string>(StringComparer.OrdinalIgnoreCase)
            for column in columns do
                if not (seen.Add(column.Name)) then
                    raise (ColumnAlreadyExists(table, column.Name))

            tables[table] <- TableState(columns, Seq.empty)

    override _.DropTable(table: string, ifExists: bool) =
        if tables.Remove(table) then
            indexes.Values
            |> Seq.filter (fun index -> same index.Table table)
            |> Seq.map (fun index -> index.Name)
            |> Seq.toArray
            |> Array.iter (fun name -> indexes.Remove(name) |> ignore)
        elif not ifExists then
            raise (TableNotFound(table))

    override _.AddColumn(table: string, column: ColumnDef) =
        let state = requireTable table

        if state.Columns |> Seq.exists (fun existing -> same existing.Name column.Name) then
            raise (ColumnAlreadyExists(table, column.Name))

        if column.EffectiveNotNull && not column.HasDefault then
            raise (ConstraintViolation(table, column.Name, $"NOT NULL constraint failed: {table}.{column.Name}"))

        state.Columns.Add(column)
        for row in state.Rows do
            row[column.Name] <- if column.HasDefault then column.DefaultValue else null

    override _.CreateIndex(index: IndexDef) =
        if indexes.ContainsKey(index.Name) then
            raise (IndexAlreadyExists(index.Name))

        let state = requireTable index.Table
        for column in index.Columns do
            canonicalColumn index.Table state column |> ignore

        if index.Unique then
            let seen = HashSet<string>(StringComparer.Ordinal)
            for row in state.Rows do
                let key = indexKey state row index.Columns
                if key |> Seq.exists (fun value -> obj.ReferenceEquals(value, null)) |> not then
                    if not (seen.Add(serializeKey key)) then
                        raise (ConstraintViolation(index.Table, String.concat "," index.Columns, $"UNIQUE constraint failed: {index.Name}"))

        indexes[index.Name] <- cloneIndex index

    override _.DropIndex(name: string, ifExists: bool) =
        if not (indexes.Remove(name)) && not ifExists then
            raise (IndexNotFound(name))

    override _.ListIndexes(table: string option) =
        indexes.Values
        |> Seq.filter (fun index -> table |> Option.map (same index.Table) |> Option.defaultValue true)
        |> Seq.map cloneIndex
        |> Seq.toArray
        :> IReadOnlyList<IndexDef>

    override _.ScanIndex(indexName: string, lo: IReadOnlyList<obj> option, hi: IReadOnlyList<obj> option, loInclusive: bool, hiInclusive: bool) =
        seq {
            match indexes.TryGetValue(indexName) with
            | false, _ -> raise (IndexNotFound(indexName))
            | true, index ->
                let state = requireTable index.Table

                let keyed =
                    state.Rows
                    |> Seq.mapi (fun rowid row -> indexKey state row index.Columns, rowid)
                    |> Seq.sortWith (fun (leftKey, leftRowid) (rightKey, rightRowid) ->
                        let cmp = (KeyComparer() :> IComparer<IReadOnlyList<obj>>).Compare(leftKey, rightKey)
                        if cmp <> 0 then cmp else compare leftRowid rightRowid)
                    |> Seq.toArray

                for key, rowid in keyed do
                    let aboveLo =
                        match lo with
                        | None -> true
                        | Some bound ->
                            let cmp = comparePrefix key bound
                            cmp > 0 || (cmp = 0 && loInclusive)

                    let belowHi =
                        match hi with
                        | None -> true
                        | Some bound ->
                            let cmp = comparePrefix key bound
                            cmp < 0 || (cmp = 0 && hiInclusive)

                    if aboveLo && belowHi then
                        yield rowid
        }

    override _.ScanByRowIds(table: string, rowids: IReadOnlyList<int>) =
        let state = requireTable table
        rowids
        |> Seq.filter (fun rowid -> rowid >= 0 && rowid < state.Rows.Count)
        |> Seq.map (fun rowid -> state.Rows[rowid])
        |> fun rows -> ListRowIterator(rows) :> IRowIterator

    override _.BeginTransaction() =
        if activeHandle.IsSome then
            raise (Unsupported("nested transactions"))

        let handle = { Value = nextHandle }
        nextHandle <- nextHandle + 1
        snapshot <- Some(capture ())
        activeHandle <- Some handle
        handle

    override _.Commit(handle: TransactionHandle) =
        requireActive handle
        snapshot <- None
        activeHandle <- None

    override _.Rollback(handle: TransactionHandle) =
        requireActive handle
        snapshot |> Option.iter restore
        snapshot <- None
        activeHandle <- None

    override _.CurrentTransaction() = activeHandle
