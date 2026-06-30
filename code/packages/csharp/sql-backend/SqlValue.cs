// SqlValue.cs — the six-variant SQL primitive value type.
//
// SQL recognises six primitive value types: NULL, INTEGER (64-bit signed),
// REAL (64-bit IEEE-754 float), TEXT (UTF-8 string), BOOLEAN, and BLOB
// (raw bytes). Every column value in the backend is one of these six.
//
// Design: a sealed class with a private discriminated-union representation.
// Factory methods construct values; the Kind property selects the variant;
// typed accessors (AsInteger, AsText, …) retrieve the payload safely.
//
//   var n = SqlValue.Null;
//   var i = SqlValue.Integer(42L);
//   var s = SqlValue.Text("hello");
//
//   switch (v.Kind)
//   {
//       case SqlValueKind.Integer: Console.WriteLine(v.AsInteger()); break;
//       case SqlValueKind.Null:    Console.WriteLine("NULL"); break;
//   }

namespace CodingAdventures.SqlBackend;

/// <summary>Discriminates the six SQL value types.</summary>
public enum SqlValueKind
{
    /// <summary>SQL NULL — the absence of a value.</summary>
    Null,
    /// <summary>A 64-bit signed integer (SQL INTEGER, INT, BIGINT, …).</summary>
    Integer,
    /// <summary>A 64-bit IEEE-754 double (SQL REAL, FLOAT, DOUBLE).</summary>
    Real,
    /// <summary>A UTF-8 string (SQL TEXT, VARCHAR, CHAR, …).</summary>
    Text,
    /// <summary>A boolean (SQL BOOLEAN, BOOL).</summary>
    Boolean,
    /// <summary>Raw bytes (SQL BLOB).</summary>
    Blob,
}

/// <summary>
/// An immutable SQL primitive value.
///
/// <para>
/// Use the static factory methods to construct values:
/// <code>
/// SqlValue.Null            // SQL NULL (singleton)
/// SqlValue.Integer(1L)     // INTEGER
/// SqlValue.Real(3.14)      // REAL
/// SqlValue.Text("hi")      // TEXT
/// SqlValue.Boolean(true)   // BOOLEAN
/// SqlValue.Blob(bytes)     // BLOB
/// </code>
/// </para>
/// <para>
/// Use <see cref="Kind"/> and the typed accessors to inspect values:
/// <code>
/// if (v.Kind == SqlValueKind.Integer)
///     Console.WriteLine(v.AsInteger());
/// </code>
/// </para>
/// </summary>
public sealed class SqlValue : IEquatable<SqlValue>
{
    // ── Singleton ───────────────────────────────────────────────────────────

    /// <summary>The SQL NULL value. A shared singleton — never allocates.</summary>
    public static readonly SqlValue Null = new(SqlValueKind.Null, null);

    // ── Factory methods ─────────────────────────────────────────────────────

    /// <summary>Wrap a 64-bit integer as a SQL INTEGER value.</summary>
    public static SqlValue Integer(long value) => new(SqlValueKind.Integer, value);

    /// <summary>Wrap a 64-bit float as a SQL REAL value.</summary>
    public static SqlValue Real(double value) => new(SqlValueKind.Real, value);

    /// <summary>Wrap a string as a SQL TEXT value.</summary>
    /// <exception cref="ArgumentNullException">When <paramref name="value"/> is null.</exception>
    public static SqlValue Text(string value) =>
        new(SqlValueKind.Text, value ?? throw new ArgumentNullException(nameof(value)));

    /// <summary>Wrap a boolean as a SQL BOOLEAN value.</summary>
    public static SqlValue Boolean(bool value) => new(SqlValueKind.Boolean, value);

    /// <summary>Wrap a byte array as a SQL BLOB value.</summary>
    /// <exception cref="ArgumentNullException">When <paramref name="value"/> is null.</exception>
    public static SqlValue Blob(byte[] value) =>
        new(SqlValueKind.Blob, value ?? throw new ArgumentNullException(nameof(value)));

    // ── Kind ────────────────────────────────────────────────────────────────

    /// <summary>Identifies which of the six SQL types this instance carries.</summary>
    public SqlValueKind Kind { get; }

    // ── Internal storage (one object slot for any variant; null for Null) ──

    private readonly object? _raw;

    private SqlValue(SqlValueKind kind, object? raw) { Kind = kind; _raw = raw; }

    // ── Accessors ───────────────────────────────────────────────────────────

    /// <summary>Returns the integer payload. Throws <see cref="InvalidCastException"/> if Kind != Integer.</summary>
    public long AsInteger() => Kind == SqlValueKind.Integer
        ? (long)_raw!
        : throw new InvalidCastException($"SqlValue is {Kind}, not Integer");

    /// <summary>Returns the real payload. Throws <see cref="InvalidCastException"/> if Kind != Real.</summary>
    public double AsReal() => Kind == SqlValueKind.Real
        ? (double)_raw!
        : throw new InvalidCastException($"SqlValue is {Kind}, not Real");

    /// <summary>Returns the text payload. Throws <see cref="InvalidCastException"/> if Kind != Text.</summary>
    public string AsText() => Kind == SqlValueKind.Text
        ? (string)_raw!
        : throw new InvalidCastException($"SqlValue is {Kind}, not Text");

    /// <summary>Returns the boolean payload. Throws <see cref="InvalidCastException"/> if Kind != Boolean.</summary>
    public bool AsBoolean() => Kind == SqlValueKind.Boolean
        ? (bool)_raw!
        : throw new InvalidCastException($"SqlValue is {Kind}, not Boolean");

    /// <summary>Returns the blob payload. Throws <see cref="InvalidCastException"/> if Kind != Blob.</summary>
    public byte[] AsBlob() => Kind == SqlValueKind.Blob
        ? (byte[])_raw!
        : throw new InvalidCastException($"SqlValue is {Kind}, not Blob");

    /// <summary>True when this value is SQL NULL.</summary>
    public bool IsNull => Kind == SqlValueKind.Null;

    // ── Equality ────────────────────────────────────────────────────────────

    /// <inheritdoc/>
    public bool Equals(SqlValue? other)
    {
        if (other is null) return false;
        if (Kind != other.Kind) return false;
        return Kind switch
        {
            SqlValueKind.Null    => true,
            SqlValueKind.Integer => (long)_raw!   == (long)other._raw!,
            SqlValueKind.Real    => (double)_raw!  == (double)other._raw!,
            SqlValueKind.Text    => (string)_raw!  == (string)other._raw!,
            SqlValueKind.Boolean => (bool)_raw!    == (bool)other._raw!,
            SqlValueKind.Blob    =>
                ((byte[])_raw!).AsSpan().SequenceEqual(((byte[])other._raw!).AsSpan()),
            _ => false,
        };
    }

    /// <inheritdoc/>
    public override bool Equals(object? obj) => obj is SqlValue v && Equals(v);

    /// <inheritdoc/>
    public override int GetHashCode() => Kind switch
    {
        SqlValueKind.Null    => 0,
        SqlValueKind.Integer => HashCode.Combine(Kind, (long)_raw!),
        SqlValueKind.Real    => HashCode.Combine(Kind, (double)_raw!),
        SqlValueKind.Text    => HashCode.Combine(Kind, (string)_raw!),
        SqlValueKind.Boolean => HashCode.Combine(Kind, (bool)_raw!),
        // Blob: hash the array reference (not content) to stay consistent with GetHashCode contract
        SqlValueKind.Blob    => HashCode.Combine(Kind, _raw!.GetHashCode()),
        _                    => 0,
    };

    /// <summary>Value equality.</summary>
    public static bool operator ==(SqlValue? a, SqlValue? b) =>
        ReferenceEquals(a, b) || (a is not null && a.Equals(b));

    /// <summary>Value inequality.</summary>
    public static bool operator !=(SqlValue? a, SqlValue? b) => !(a == b);

    // ── Display ─────────────────────────────────────────────────────────────

    /// <summary>Returns a human-readable SQL representation of this value.</summary>
    public override string ToString() => Kind switch
    {
        SqlValueKind.Null    => "NULL",
        SqlValueKind.Integer => ((long)_raw!).ToString(),
        SqlValueKind.Real    => ((double)_raw!).ToString("G"),
        SqlValueKind.Text    => (string)_raw!,
        SqlValueKind.Boolean => (bool)_raw! ? "TRUE" : "FALSE",
        SqlValueKind.Blob    => $"<blob:{((byte[])_raw!).Length}B>",
        _                    => "?",
    };

    /// <summary>Returns the SQL type-affinity name for this value ("NULL", "INTEGER", etc.).</summary>
    public string SqlTypeName() => Kind switch
    {
        SqlValueKind.Null    => "NULL",
        SqlValueKind.Integer => "INTEGER",
        SqlValueKind.Real    => "REAL",
        SqlValueKind.Text    => "TEXT",
        SqlValueKind.Boolean => "BOOLEAN",
        SqlValueKind.Blob    => "BLOB",
        _                    => "UNKNOWN",
    };
}
