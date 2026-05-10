using System.Globalization;

namespace CodingAdventures.SqlBackend;

/// <summary>Opaque transaction handle issued by a backend.</summary>
public readonly record struct TransactionHandle(int Value);

/// <summary>Helpers for the portable SQL value set.</summary>
public static class SqlValues
{
    public static bool IsSqlValue(object? value)
        => value is null or bool or string or byte[] || IsInteger(value) || IsReal(value);

    public static string TypeName(object? value)
    {
        if (value is null)
        {
            return "NULL";
        }

        if (value is bool)
        {
            return "BOOLEAN";
        }

        if (IsInteger(value))
        {
            return "INTEGER";
        }

        if (IsReal(value))
        {
            return "REAL";
        }

        if (value is string)
        {
            return "TEXT";
        }

        if (value is byte[])
        {
            return "BLOB";
        }

        throw new ArgumentException($"not a SQL value: {value.GetType().Name}", nameof(value));
    }

    internal static int Compare(object? left, object? right)
    {
        var rank = Rank(left).CompareTo(Rank(right));
        if (rank != 0)
        {
            return rank;
        }

        return left switch
        {
            null => 0,
            bool leftBool => leftBool.CompareTo((bool)right!),
            string leftText => string.CompareOrdinal(leftText, (string)right!),
            byte[] leftBytes => CompareBytes(leftBytes, (byte[])right!),
            _ when IsNumeric(left) && IsNumeric(right)
                => Convert.ToDouble(left, CultureInfo.InvariantCulture)
                    .CompareTo(Convert.ToDouble(right, CultureInfo.InvariantCulture)),
            _ => string.CompareOrdinal(left.ToString(), right?.ToString()),
        };
    }

    private static int Rank(object? value)
    {
        if (value is null)
        {
            return 0;
        }

        if (value is bool)
        {
            return 1;
        }

        if (IsNumeric(value))
        {
            return 2;
        }

        if (value is string)
        {
            return 3;
        }

        if (value is byte[])
        {
            return 4;
        }

        return 5;
    }

    private static int CompareBytes(IReadOnlyList<byte> left, IReadOnlyList<byte> right)
    {
        var length = Math.Min(left.Count, right.Count);
        for (var i = 0; i < length; i++)
        {
            var cmp = left[i].CompareTo(right[i]);
            if (cmp != 0)
            {
                return cmp;
            }
        }

        return left.Count.CompareTo(right.Count);
    }

    private static bool IsNumeric(object? value) => IsInteger(value) || IsReal(value);

    private static bool IsInteger(object? value)
        => value is byte or sbyte or short or ushort or int or uint or long or ulong;

    private static bool IsReal(object? value) => value is float or double;
}

/// <summary>A mutable row keyed by column name.</summary>
public sealed class Row : Dictionary<string, object?>
{
    public Row()
        : base(StringComparer.OrdinalIgnoreCase)
    {
    }

    public Row(IEnumerable<KeyValuePair<string, object?>> values)
        : this()
    {
        foreach (var (key, value) in values)
        {
            this[key] = value;
        }
    }

    public Row Copy() => new(this);
}

/// <summary>Lazy iterator over backend rows.</summary>
public interface IRowIterator
{
    Row? Next();
    void Close();
}

/// <summary>Iterator that remembers the current row for positioned DML.</summary>
public interface ICursor : IRowIterator
{
    Row? CurrentRow { get; }
}

/// <summary>Row iterator backed by a materialized list.</summary>
public sealed class ListRowIterator(IEnumerable<Row> rows) : IRowIterator
{
    private readonly IReadOnlyList<Row> _rows = rows.ToArray();
    private int _index;
    private bool _closed;

    public Row? Next()
    {
        if (_closed || _index >= _rows.Count)
        {
            return null;
        }

        return _rows[_index++].Copy();
    }

    public void Close() => _closed = true;
}

/// <summary>List-backed cursor used by the in-memory backend.</summary>
public sealed class ListCursor(IList<Row> rows) : ICursor
{
    private readonly IList<Row> _rows = rows;
    private int _index = -1;
    private Row? _current;
    private bool _closed;

    public Row? CurrentRow => _current?.Copy();

    internal int CurrentIndex => _index;

    public Row? Next()
    {
        if (_closed)
        {
            return null;
        }

        _index++;
        if (_index >= _rows.Count)
        {
            _current = null;
            return null;
        }

        _current = _rows[_index];
        return _current.Copy();
    }

    public void Close() => _closed = true;

    internal bool IsBackedBy(IList<Row> rows) => ReferenceEquals(_rows, rows);

    internal void AdjustAfterDelete()
    {
        _index--;
        _current = null;
    }
}

/// <summary>One column in a table schema.</summary>
public sealed record ColumnDef(
    string Name,
    string TypeName,
    bool NotNull = false,
    bool PrimaryKey = false,
    bool Unique = false,
    bool Autoincrement = false,
    object? DefaultValue = null,
    bool HasDefault = false,
    object? CheckExpression = null,
    object? ForeignKey = null)
{
    public bool EffectiveNotNull => NotNull || PrimaryKey;
    public bool EffectiveUnique => Unique || PrimaryKey;

    public static ColumnDef WithDefault(
        string name,
        string typeName,
        object? defaultValue,
        bool notNull = false,
        bool primaryKey = false,
        bool unique = false,
        bool autoincrement = false)
        => new(name, typeName, notNull, primaryKey, unique, autoincrement, defaultValue, HasDefault: true);
}

/// <summary>Definition of a trigger stored by a backend.</summary>
public sealed record TriggerDef(string Name, string Table, string Timing, string Event, string Body);

/// <summary>Definition of a backend-managed index.</summary>
public sealed record IndexDef
{
    public IndexDef(string name, string table, IReadOnlyList<string>? columns = null, bool unique = false, bool auto = false)
    {
        Name = name;
        Table = table;
        Columns = columns?.ToArray() ?? Array.Empty<string>();
        Unique = unique;
        Auto = auto;
    }

    public string Name { get; init; }
    public string Table { get; init; }
    public IReadOnlyList<string> Columns { get; init; }
    public bool Unique { get; init; }
    public bool Auto { get; init; }
}

public abstract class BackendError(string message) : Exception(message);

public sealed class TableNotFound(string table) : BackendError($"table not found: '{table}'")
{
    public string Table { get; } = table;
}

public sealed class TableAlreadyExists(string table) : BackendError($"table already exists: '{table}'")
{
    public string Table { get; } = table;
}

public sealed class ColumnNotFound(string table, string column) : BackendError($"column not found: '{table}.{column}'")
{
    public string Table { get; } = table;
    public string Column { get; } = column;
}

public sealed class ColumnAlreadyExists(string table, string column) : BackendError($"column already exists: '{table}.{column}'")
{
    public string Table { get; } = table;
    public string Column { get; } = column;
}

public sealed class ConstraintViolation(string table, string column, string detail)
    : BackendError(detail)
{
    public string Table { get; } = table;
    public string Column { get; } = column;
}

public sealed class Unsupported(string operation) : BackendError($"unsupported operation: {operation}")
{
    public string Operation { get; } = operation;
}

public sealed class Internal(string detail) : BackendError(detail);

public sealed class IndexAlreadyExists(string index) : BackendError($"index already exists: '{index}'")
{
    public string Index { get; } = index;
}

public sealed class IndexNotFound(string index) : BackendError($"index not found: '{index}'")
{
    public string Index { get; } = index;
}

public sealed class TriggerAlreadyExists(string trigger) : BackendError($"trigger already exists: '{trigger}'")
{
    public string Trigger { get; } = trigger;
}

public sealed class TriggerNotFound(string trigger) : BackendError($"trigger not found: '{trigger}'")
{
    public string Trigger { get; } = trigger;
}

/// <summary>Pluggable interface every SQL data source implements.</summary>
public abstract class Backend
{
    public abstract IReadOnlyList<string> Tables();
    public abstract IReadOnlyList<ColumnDef> Columns(string table);
    public abstract IRowIterator Scan(string table);
    public abstract void Insert(string table, Row row);
    public abstract void Update(string table, ICursor cursor, IReadOnlyDictionary<string, object?> assignments);
    public abstract void Delete(string table, ICursor cursor);
    public abstract void CreateTable(string table, IReadOnlyList<ColumnDef> columns, bool ifNotExists);
    public abstract void DropTable(string table, bool ifExists);
    public abstract void AddColumn(string table, ColumnDef column);
    public abstract void CreateIndex(IndexDef index);
    public abstract void DropIndex(string name, bool ifExists = false);
    public abstract IReadOnlyList<IndexDef> ListIndexes(string? table = null);
    public abstract IEnumerable<int> ScanIndex(
        string indexName,
        IReadOnlyList<object?>? lo,
        IReadOnlyList<object?>? hi,
        bool loInclusive = true,
        bool hiInclusive = true);

    public abstract IRowIterator ScanByRowIds(string table, IReadOnlyList<int> rowids);
    public abstract TransactionHandle BeginTransaction();
    public abstract void Commit(TransactionHandle handle);
    public abstract void Rollback(TransactionHandle handle);

    public virtual TransactionHandle? CurrentTransaction() => null;
    public virtual void CreateSavepoint(string name) => throw new Unsupported("savepoints");
    public virtual void ReleaseSavepoint(string name) => throw new Unsupported("savepoints");
    public virtual void RollbackToSavepoint(string name) => throw new Unsupported("savepoints");
    public virtual void CreateTrigger(TriggerDef defn) => throw new Unsupported("triggers");
    public virtual void DropTrigger(string name, bool ifExists = false) => throw new Unsupported("triggers");
    public virtual IReadOnlyList<TriggerDef> ListTriggers(string table) => Array.Empty<TriggerDef>();
}

public interface ISchemaProvider
{
    IReadOnlyList<string> Columns(string table);
}

public static class BackendAdapters
{
    public static ISchemaProvider AsSchemaProvider(Backend backend) => new BackendSchemaProvider(backend);

    private sealed class BackendSchemaProvider(Backend backend) : ISchemaProvider
    {
        public IReadOnlyList<string> Columns(string table) => backend.Columns(table).Select(column => column.Name).ToArray();
    }
}

/// <summary>Reference in-memory backend with DDL, DML, indexes, and transactions.</summary>
public sealed class InMemoryBackend : Backend
{
    private readonly Dictionary<string, TableState> _tables = new(StringComparer.OrdinalIgnoreCase);
    private readonly Dictionary<string, IndexDef> _indexes = new(StringComparer.OrdinalIgnoreCase);
    private Snapshot? _snapshot;
    private int _nextHandle = 1;
    private TransactionHandle? _activeHandle;

    public static InMemoryBackend FromTables(
        IReadOnlyDictionary<string, (IReadOnlyList<ColumnDef> Columns, IReadOnlyList<Row> Rows)> tables)
    {
        var backend = new InMemoryBackend();
        foreach (var (name, table) in tables)
        {
            backend._tables[name] = new TableState(table.Columns, table.Rows.Select(row => row.Copy()));
        }

        return backend;
    }

    public override IReadOnlyList<string> Tables() => _tables.Keys.ToArray();

    public override IReadOnlyList<ColumnDef> Columns(string table) => RequireTable(table).Columns.ToArray();

    public override IRowIterator Scan(string table) => new ListRowIterator(RequireTable(table).Rows);

    public ListCursor OpenCursor(string table) => new(RequireTable(table).Rows);

    public override void Insert(string table, Row row)
    {
        var state = RequireTable(table);
        var normalized = ApplyDefaults(table, state, row);
        CheckUnknownColumns(table, state, normalized);
        CheckNotNull(table, state, normalized);
        CheckUnique(table, state, normalized, ignoreIndex: null);
        state.Rows.Add(normalized);
    }

    public override void Update(string table, ICursor cursor, IReadOnlyDictionary<string, object?> assignments)
    {
        var state = RequireTable(table);
        var listCursor = RequireListCursor(table, state, cursor);
        var index = listCursor.CurrentIndex;
        if (index < 0 || index >= state.Rows.Count)
        {
            throw new Unsupported("cursor has no current row");
        }

        var updated = state.Rows[index].Copy();
        foreach (var (column, value) in assignments)
        {
            updated[CanonicalColumn(table, state, column)] = value;
        }

        CheckNotNull(table, state, updated);
        CheckUnique(table, state, updated, ignoreIndex: index);
        state.Rows[index] = updated;
    }

    public override void Delete(string table, ICursor cursor)
    {
        var state = RequireTable(table);
        var listCursor = RequireListCursor(table, state, cursor);
        var index = listCursor.CurrentIndex;
        if (index < 0 || index >= state.Rows.Count)
        {
            throw new Unsupported("cursor has no current row");
        }

        state.Rows.RemoveAt(index);
        listCursor.AdjustAfterDelete();
    }

    public override void CreateTable(string table, IReadOnlyList<ColumnDef> columns, bool ifNotExists)
    {
        if (_tables.ContainsKey(table))
        {
            if (ifNotExists)
            {
                return;
            }

            throw new TableAlreadyExists(table);
        }

        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var column in columns)
        {
            if (!seen.Add(column.Name))
            {
                throw new ColumnAlreadyExists(table, column.Name);
            }
        }

        _tables[table] = new TableState(columns, Array.Empty<Row>());
    }

    public override void DropTable(string table, bool ifExists)
    {
        if (!_tables.Remove(table))
        {
            if (ifExists)
            {
                return;
            }

            throw new TableNotFound(table);
        }

        foreach (var index in _indexes.Values.Where(index => Same(index.Table, table)).Select(index => index.Name).ToArray())
        {
            _indexes.Remove(index);
        }
    }

    public override void AddColumn(string table, ColumnDef column)
    {
        var state = RequireTable(table);
        if (state.Columns.Any(existing => Same(existing.Name, column.Name)))
        {
            throw new ColumnAlreadyExists(table, column.Name);
        }

        if (column.EffectiveNotNull && !column.HasDefault)
        {
            throw new ConstraintViolation(table, column.Name, $"NOT NULL constraint failed: {table}.{column.Name}");
        }

        state.Columns.Add(column);
        foreach (var row in state.Rows)
        {
            row[column.Name] = column.HasDefault ? column.DefaultValue : null;
        }
    }

    public override void CreateIndex(IndexDef index)
    {
        if (_indexes.ContainsKey(index.Name))
        {
            throw new IndexAlreadyExists(index.Name);
        }

        var state = RequireTable(index.Table);
        foreach (var column in index.Columns)
        {
            _ = CanonicalColumn(index.Table, state, column);
        }

        if (index.Unique)
        {
            var seen = new HashSet<string>(StringComparer.Ordinal);
            foreach (var row in state.Rows)
            {
                var key = IndexKey(state, row, index.Columns);
                if (key.Any(value => value is null))
                {
                    continue;
                }

                if (!seen.Add(SerializeKey(key)))
                {
                    throw new ConstraintViolation(index.Table, string.Join(",", index.Columns), $"UNIQUE constraint failed: {index.Name}");
                }
            }
        }

        _indexes[index.Name] = CloneIndex(index);
    }

    public override void DropIndex(string name, bool ifExists = false)
    {
        if (_indexes.Remove(name))
        {
            return;
        }

        if (!ifExists)
        {
            throw new IndexNotFound(name);
        }
    }

    public override IReadOnlyList<IndexDef> ListIndexes(string? table = null)
        => _indexes.Values
            .Where(index => table is null || Same(index.Table, table))
            .Select(CloneIndex)
            .ToArray();

    public override IEnumerable<int> ScanIndex(
        string indexName,
        IReadOnlyList<object?>? lo,
        IReadOnlyList<object?>? hi,
        bool loInclusive = true,
        bool hiInclusive = true)
    {
        if (!_indexes.TryGetValue(indexName, out var index))
        {
            throw new IndexNotFound(indexName);
        }

        var state = RequireTable(index.Table);
        var keyed = state.Rows
            .Select((row, rowid) => (Key: IndexKey(state, row, index.Columns), Rowid: rowid))
            .OrderBy(item => item.Key, KeyComparer.Instance)
            .ThenBy(item => item.Rowid)
            .ToArray();

        foreach (var item in keyed)
        {
            if (lo is not null)
            {
                var cmp = ComparePrefix(item.Key, lo);
                if (cmp < 0 || (cmp == 0 && !loInclusive))
                {
                    continue;
                }
            }

            if (hi is not null)
            {
                var cmp = ComparePrefix(item.Key, hi);
                if (cmp > 0 || (cmp == 0 && !hiInclusive))
                {
                    yield break;
                }
            }

            yield return item.Rowid;
        }
    }

    public override IRowIterator ScanByRowIds(string table, IReadOnlyList<int> rowids)
    {
        var state = RequireTable(table);
        var rows = rowids.Where(rowid => rowid >= 0 && rowid < state.Rows.Count).Select(rowid => state.Rows[rowid]);
        return new ListRowIterator(rows);
    }

    public override TransactionHandle BeginTransaction()
    {
        if (_activeHandle is not null)
        {
            throw new Unsupported("nested transactions");
        }

        var handle = new TransactionHandle(_nextHandle++);
        _snapshot = Capture();
        _activeHandle = handle;
        return handle;
    }

    public override void Commit(TransactionHandle handle)
    {
        RequireActive(handle);
        _snapshot = null;
        _activeHandle = null;
    }

    public override void Rollback(TransactionHandle handle)
    {
        RequireActive(handle);
        if (_snapshot is not null)
        {
            Restore(_snapshot);
        }

        _snapshot = null;
        _activeHandle = null;
    }

    public override TransactionHandle? CurrentTransaction() => _activeHandle;

    private TableState RequireTable(string table)
    {
        if (!_tables.TryGetValue(table, out var state))
        {
            throw new TableNotFound(table);
        }

        return state;
    }

    private static ListCursor RequireListCursor(string table, TableState state, ICursor cursor)
    {
        if (cursor is not ListCursor listCursor || !listCursor.IsBackedBy(state.Rows))
        {
            throw new Unsupported($"foreign cursor for table {table}");
        }

        return listCursor;
    }

    private static Row ApplyDefaults(string table, TableState state, Row row)
    {
        var normalized = row.Copy();
        foreach (var column in state.Columns)
        {
            if (!normalized.ContainsKey(column.Name))
            {
                normalized[column.Name] = column.HasDefault ? column.DefaultValue : null;
            }
        }

        CheckUnknownColumns(table, state, normalized);
        return normalized;
    }

    private static void CheckUnknownColumns(string table, TableState state, Row row)
    {
        foreach (var column in row.Keys)
        {
            if (!state.Columns.Any(existing => Same(existing.Name, column)))
            {
                throw new ColumnNotFound(table, column);
            }
        }
    }

    private static void CheckNotNull(string table, TableState state, Row row)
    {
        foreach (var column in state.Columns.Where(column => column.EffectiveNotNull))
        {
            if (!row.TryGetValue(column.Name, out var value) || value is null)
            {
                throw new ConstraintViolation(table, column.Name, $"NOT NULL constraint failed: {table}.{column.Name}");
            }
        }
    }

    private static void CheckUnique(string table, TableState state, Row row, int? ignoreIndex)
    {
        foreach (var column in state.Columns.Where(column => column.EffectiveUnique))
        {
            if (!row.TryGetValue(column.Name, out var value) || value is null)
            {
                continue;
            }

            for (var i = 0; i < state.Rows.Count; i++)
            {
                if (ignoreIndex == i)
                {
                    continue;
                }

                if (state.Rows[i].TryGetValue(column.Name, out var existing) && Equals(existing, value))
                {
                    var label = column.PrimaryKey ? "PRIMARY KEY" : "UNIQUE";
                    throw new ConstraintViolation(table, column.Name, $"{label} constraint failed: {table}.{column.Name}");
                }
            }
        }
    }

    private static string CanonicalColumn(string table, TableState state, string column)
    {
        foreach (var candidate in state.Columns)
        {
            if (Same(candidate.Name, column))
            {
                return candidate.Name;
            }
        }

        throw new ColumnNotFound(table, column);
    }

    private static IReadOnlyList<object?> IndexKey(TableState state, Row row, IReadOnlyList<string> columns)
        => columns.Select(column => row.GetValueOrDefault(CanonicalColumn("", state, column))).ToArray();

    private static int ComparePrefix(IReadOnlyList<object?> key, IReadOnlyList<object?> bound)
    {
        for (var i = 0; i < bound.Count; i++)
        {
            var cmp = SqlValues.Compare(i < key.Count ? key[i] : null, bound[i]);
            if (cmp != 0)
            {
                return cmp;
            }
        }

        return 0;
    }

    private static string SerializeKey(IEnumerable<object?> key)
        => string.Join("\u001f", key.Select(value => value switch
        {
            null => "NULL",
            byte[] bytes => Convert.ToBase64String(bytes),
            _ => Convert.ToString(value, CultureInfo.InvariantCulture) ?? "",
        }));

    private Snapshot Capture()
        => new(
            _tables.ToDictionary(pair => pair.Key, pair => pair.Value.Clone(), StringComparer.OrdinalIgnoreCase),
            _indexes.ToDictionary(pair => pair.Key, pair => CloneIndex(pair.Value), StringComparer.OrdinalIgnoreCase));

    private void Restore(Snapshot snapshot)
    {
        _tables.Clear();
        foreach (var (name, table) in snapshot.Tables)
        {
            _tables[name] = table.Clone();
        }

        _indexes.Clear();
        foreach (var (name, index) in snapshot.Indexes)
        {
            _indexes[name] = CloneIndex(index);
        }
    }

    private void RequireActive(TransactionHandle handle)
    {
        if (_activeHandle is null)
        {
            throw new Unsupported("no active transaction");
        }

        if (_activeHandle.Value != handle)
        {
            throw new Unsupported("stale transaction handle");
        }
    }

    private static bool Same(string left, string right) => string.Equals(left, right, StringComparison.OrdinalIgnoreCase);

    private static IndexDef CloneIndex(IndexDef index)
        => new(index.Name, index.Table, index.Columns.ToArray(), index.Unique, index.Auto);

    private sealed class TableState
    {
        public TableState(IReadOnlyList<ColumnDef> columns, IEnumerable<Row> rows)
        {
            Columns = columns.ToList();
            Rows = rows.Select(row => row.Copy()).ToList();
        }

        public List<ColumnDef> Columns { get; }
        public List<Row> Rows { get; }

        public TableState Clone() => new(Columns.ToArray(), Rows.Select(row => row.Copy()));
    }

    private sealed record Snapshot(
        Dictionary<string, TableState> Tables,
        Dictionary<string, IndexDef> Indexes);

    private sealed class KeyComparer : IComparer<IReadOnlyList<object?>>
    {
        public static readonly KeyComparer Instance = new();

        public int Compare(IReadOnlyList<object?>? x, IReadOnlyList<object?>? y)
        {
            if (x is null || y is null)
            {
                return x is null ? y is null ? 0 : -1 : 1;
            }

            var length = Math.Min(x.Count, y.Count);
            for (var i = 0; i < length; i++)
            {
                var cmp = SqlValues.Compare(x[i], y[i]);
                if (cmp != 0)
                {
                    return cmp;
                }
            }

            return x.Count.CompareTo(y.Count);
        }
    }
}
