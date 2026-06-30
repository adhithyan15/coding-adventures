// SqlValueDiscriminatedUnionTests.cs — unit tests for the SqlValue discriminated union.
//
// SqlValue is a closed, typed representation of a SQL primitive. This is distinct
// from the portable object?-based row values; SqlValue provides richer type safety
// for callers that want explicit variant handling rather than runtime type checks.

using CodingAdventures.SqlBackend;

namespace CodingAdventures.SqlBackend.Tests;

public sealed class SqlValueDiscriminatedUnionTests
{
    // ── Null ─────────────────────────────────────────────────────────────────

    [Fact]
    public void Null_IsSingleton()
    {
        Assert.Same(SqlValue.Null, SqlValue.Null);
    }

    [Fact]
    public void Null_HasCorrectKind()
    {
        Assert.Equal(SqlValueKind.Null, SqlValue.Null.Kind);
        Assert.True(SqlValue.Null.IsNull);
    }

    [Fact]
    public void Null_ToString_ReturnsNULL()
    {
        Assert.Equal("NULL", SqlValue.Null.ToString());
    }

    [Fact]
    public void Null_EqualsNull()
    {
        Assert.Equal(SqlValue.Null, SqlValue.Null);
    }

    [Fact]
    public void Null_DoesNotEqualInteger()
    {
        Assert.NotEqual(SqlValue.Null, SqlValue.Integer(0));
    }

    // ── Integer ──────────────────────────────────────────────────────────────

    [Fact]
    public void Integer_HasCorrectKind()
    {
        var v = SqlValue.Integer(42L);
        Assert.Equal(SqlValueKind.Integer, v.Kind);
        Assert.False(v.IsNull);
    }

    [Theory]
    [InlineData(0L)]
    [InlineData(1L)]
    [InlineData(-1L)]
    [InlineData(long.MaxValue)]
    [InlineData(long.MinValue)]
    public void Integer_RoundTrips(long n)
    {
        Assert.Equal(n, SqlValue.Integer(n).AsInteger());
    }

    [Fact]
    public void Integer_AsReal_Throws()
    {
        Assert.Throws<InvalidCastException>(() => SqlValue.Integer(1).AsReal());
    }

    [Fact]
    public void Integer_Equality()
    {
        Assert.Equal(SqlValue.Integer(7), SqlValue.Integer(7));
        Assert.NotEqual(SqlValue.Integer(7), SqlValue.Integer(8));
    }

    [Fact]
    public void Integer_ToString()
    {
        Assert.Equal("42", SqlValue.Integer(42).ToString());
        Assert.Equal("-1", SqlValue.Integer(-1).ToString());
    }

    // ── Real ─────────────────────────────────────────────────────────────────

    [Fact]
    public void Real_HasCorrectKind()
    {
        Assert.Equal(SqlValueKind.Real, SqlValue.Real(3.14).Kind);
    }

    [Theory]
    [InlineData(0.0)]
    [InlineData(3.14)]
    [InlineData(-1.5)]
    public void Real_RoundTrips(double d)
    {
        Assert.Equal(d, SqlValue.Real(d).AsReal());
    }

    [Fact]
    public void Real_AsInteger_Throws()
    {
        Assert.Throws<InvalidCastException>(() => SqlValue.Real(1.0).AsInteger());
    }

    [Fact]
    public void Real_Equality()
    {
        Assert.Equal(SqlValue.Real(1.5), SqlValue.Real(1.5));
        Assert.NotEqual(SqlValue.Real(1.5), SqlValue.Real(2.5));
    }

    // ── Text ─────────────────────────────────────────────────────────────────

    [Fact]
    public void Text_HasCorrectKind()
    {
        Assert.Equal(SqlValueKind.Text, SqlValue.Text("hello").Kind);
    }

    [Fact]
    public void Text_RoundTrips()
    {
        Assert.Equal("hello", SqlValue.Text("hello").AsText());
    }

    [Fact]
    public void Text_NullArgument_Throws()
    {
        Assert.Throws<ArgumentNullException>(() => SqlValue.Text(null!));
    }

    [Fact]
    public void Text_Equality()
    {
        Assert.Equal(SqlValue.Text("a"), SqlValue.Text("a"));
        Assert.NotEqual(SqlValue.Text("a"), SqlValue.Text("b"));
    }

    [Fact]
    public void Text_ToString_ReturnsRawString()
    {
        Assert.Equal("hello", SqlValue.Text("hello").ToString());
    }

    // ── Boolean ──────────────────────────────────────────────────────────────

    [Fact]
    public void Boolean_HasCorrectKind()
    {
        Assert.Equal(SqlValueKind.Boolean, SqlValue.Boolean(true).Kind);
    }

    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public void Boolean_RoundTrips(bool b)
    {
        Assert.Equal(b, SqlValue.Boolean(b).AsBoolean());
    }

    [Fact]
    public void Boolean_Equality()
    {
        Assert.Equal(SqlValue.Boolean(true), SqlValue.Boolean(true));
        Assert.NotEqual(SqlValue.Boolean(true), SqlValue.Boolean(false));
    }

    [Fact]
    public void Boolean_ToString()
    {
        Assert.Equal("TRUE", SqlValue.Boolean(true).ToString());
        Assert.Equal("FALSE", SqlValue.Boolean(false).ToString());
    }

    // ── Blob ─────────────────────────────────────────────────────────────────

    [Fact]
    public void Blob_HasCorrectKind()
    {
        Assert.Equal(SqlValueKind.Blob, SqlValue.Blob(new byte[] { 1, 2, 3 }).Kind);
    }

    [Fact]
    public void Blob_RoundTrips()
    {
        var bytes = new byte[] { 0xFF, 0x00, 0xAB };
        Assert.Equal(bytes, SqlValue.Blob(bytes).AsBlob());
    }

    [Fact]
    public void Blob_NullArgument_Throws()
    {
        Assert.Throws<ArgumentNullException>(() => SqlValue.Blob(null!));
    }

    [Fact]
    public void Blob_ContentEquality()
    {
        var a = SqlValue.Blob(new byte[] { 1, 2, 3 });
        var b = SqlValue.Blob(new byte[] { 1, 2, 3 });
        Assert.Equal(a, b);
    }

    [Fact]
    public void Blob_ToString_ContainsLength()
    {
        Assert.Contains("3B", SqlValue.Blob(new byte[] { 1, 2, 3 }).ToString());
    }

    // ── SqlTypeName ───────────────────────────────────────────────────────────

    [Theory]
    [InlineData(SqlValueKind.Null,    "NULL")]
    [InlineData(SqlValueKind.Integer, "INTEGER")]
    [InlineData(SqlValueKind.Real,    "REAL")]
    [InlineData(SqlValueKind.Text,    "TEXT")]
    [InlineData(SqlValueKind.Boolean, "BOOLEAN")]
    [InlineData(SqlValueKind.Blob,    "BLOB")]
    public void SqlTypeName_ReturnsCorrectName(SqlValueKind kind, string expected)
    {
        SqlValue v = kind switch
        {
            SqlValueKind.Null    => SqlValue.Null,
            SqlValueKind.Integer => SqlValue.Integer(0),
            SqlValueKind.Real    => SqlValue.Real(0),
            SqlValueKind.Text    => SqlValue.Text(""),
            SqlValueKind.Boolean => SqlValue.Boolean(false),
            SqlValueKind.Blob    => SqlValue.Blob(Array.Empty<byte>()),
            _ => throw new ArgumentOutOfRangeException()
        };
        Assert.Equal(expected, v.SqlTypeName());
    }

    // ── Cross-type inequality ─────────────────────────────────────────────────

    [Fact]
    public void DifferentKinds_AreNotEqual()
    {
        var vals = new[]
        {
            SqlValue.Null,
            SqlValue.Integer(0),
            SqlValue.Real(0.0),
            SqlValue.Text(""),
            SqlValue.Boolean(false),
            SqlValue.Blob(Array.Empty<byte>()),
        };
        for (var i = 0; i < vals.Length; i++)
        for (var j = 0; j < vals.Length; j++)
        {
            if (i != j)
                Assert.NotEqual(vals[i], vals[j]);
        }
    }

    // ── Operator == / != ─────────────────────────────────────────────────────

    [Fact]
    public void OperatorEquals_Works()
    {
        Assert.True(SqlValue.Integer(1) == SqlValue.Integer(1));
        Assert.False(SqlValue.Integer(1) == SqlValue.Integer(2));
        Assert.True(SqlValue.Integer(1) != SqlValue.Integer(2));
    }

    // ── Wrong-variant accessor throws ─────────────────────────────────────────

    [Fact]
    public void AsText_Throws_WhenKindIsNotText()
    {
        Assert.Throws<InvalidCastException>(() => SqlValue.Integer(1).AsText());
    }

    [Fact]
    public void AsBoolean_Throws_WhenKindIsNotBoolean()
    {
        Assert.Throws<InvalidCastException>(() => SqlValue.Null.AsBoolean());
    }

    [Fact]
    public void AsBlob_Throws_WhenKindIsNotBlob()
    {
        Assert.Throws<InvalidCastException>(() => SqlValue.Text("x").AsBlob());
    }
}
