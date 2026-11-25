using System.Numerics;
using CodingAdventures.TestBootstrap.CorpusJson;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>Strict case-value decoding and kind-sensitive equality.</summary>
public sealed class ValuesTests : HookedTest
{
    private static Dictionary<string, object?> IntNode(string text) => new() { ["kind"] = "int", ["value"] = text };

    private static Dictionary<string, object?> BytesNode(string hex) => new() { ["kind"] = "bytes", ["hex"] = hex };

    [Fact]
    public void CanonicalIntAndHex()
    {
        foreach (string ok in new[] { "0", "7", "-7", "10", "123456789012345678901234567890" })
        {
            Assert.True(Values.IsCanonicalInt(ok), ok);
        }

        foreach (string bad in new[] { string.Empty, "-", "-0", "00", "01", "-01", "+1", "1_0", " 1", "1 ", "0x1", "1.0", "١" })
        {
            Assert.False(Values.IsCanonicalInt(bad), bad);
        }

        foreach (string ok in new[] { string.Empty, "00", "ff00", "0123456789abcdef" })
        {
            Assert.True(Values.IsLowercaseHex(ok), ok);
        }

        foreach (string bad in new[] { "0", "FF", "gg", "f f", "0x" })
        {
            Assert.False(Values.IsLowercaseHex(bad), bad);
        }
    }

    [Fact]
    public void DecodesEveryKind()
    {
        Assert.Equal(true, Values.Decode(true));
        Assert.Equal("text", Values.Decode("text"));
        Assert.Equal(new BigInteger(-12), Values.Decode(IntNode("-12")));
        Assert.Equal(new byte[] { 0, 255 }, (byte[])Values.Decode(BytesNode("00ff"))!);
        Assert.Empty((byte[])Values.Decode(BytesNode(string.Empty))!);
        List<object?> list = (List<object?>)Values.Decode(new List<object?> { true, IntNode("3"), new List<object?>() })!;
        Assert.Equal(3, list.Count);
        Assert.Equal(new BigInteger(3), list[1]);
    }

    [Fact]
    public void RefusesMalformedNodes()
    {
        object?[] bad =
        {
            null,
            new JsonNumber("1"),
            IntNode("007"),
            IntNode("1.5"),
            new Dictionary<string, object?> { ["kind"] = "int", ["value"] = true },
            new Dictionary<string, object?> { ["kind"] = "int" },
            new Dictionary<string, object?> { ["kind"] = "int", ["value"] = "1", ["extra"] = 1 },
            BytesNode("FF"),
            BytesNode("f"),
            new Dictionary<string, object?> { ["kind"] = "bytes", ["hex"] = 5 },
            new Dictionary<string, object?> { ["kind"] = "float", ["value"] = "1" },
            new Dictionary<string, object?>(),
            new Dictionary<string, object?> { ["value"] = "1" },
        };
        foreach (object? node in bad)
        {
            Assert.Throws<CaseException>(() => Values.Decode(node));
        }
    }

    [Fact]
    public void SameDoesNotConflateKinds()
    {
        Assert.True(Values.Same(true, true));
        Assert.False(Values.Same(true, new BigInteger(1)));
        Assert.False(Values.Same(new BigInteger(1), true));
        Assert.False(Values.Same("1", new BigInteger(1)));
        Assert.True(Values.Same("a", "a"));
        Assert.False(Values.Same("a", "b"));
        Assert.True(Values.Same(new BigInteger(5), new BigInteger(5)));
        Assert.False(Values.Same(new BigInteger(5), new BigInteger(6)));
        Assert.True(Values.Same(new byte[] { 1, 2 }, new byte[] { 1, 2 }));
        Assert.False(Values.Same(new byte[] { 1, 2 }, new byte[] { 1, 3 }));
        Assert.False(Values.Same(new byte[] { 1, 2 }, new List<object?> { new BigInteger(1), new BigInteger(2) }));
        Assert.False(Values.Same(new List<object?> { new BigInteger(1) }, new byte[] { 1 }));
        Assert.True(Values.Same(new List<object?> { true, new List<object?>() }, new List<object?> { true, new List<object?>() }));
        Assert.False(Values.Same(new List<object?> { true }, new List<object?> { true, true }));
        Assert.False(Values.Same(new List<object?> { true }, new List<object?> { false }));
        Assert.False(Values.Same(null, null));
        Assert.False(Values.Same(1, 1)); // an int is not a decoded value kind
        Assert.False(Values.Same("x", null));
    }

    [Fact]
    public void DeepHandBuiltTreesAreNotTheSameAndNotAStackOverflow()
    {
        object? a = new BigInteger(1);
        object? b = new BigInteger(1);
        for (int i = 0; i < 100_000; i++)
        {
            a = new List<object?> { a };
            b = new List<object?> { b };
        }

        // Decode refuses them; Same declines to call them equal rather than recursing forever.
        Assert.Throws<CaseException>(() => Values.Decode(a));
        Assert.False(Values.Same(a, b));
    }

    [Fact]
    public void ShowIsBounded()
    {
        Assert.True(Values.Show(new string('x', 10_000)).Length <= Values.MaxFragmentLen + 3);
        Assert.Equal(Values.MaxProblemLen + 3, new CaseException(new string('y', 1000)).Message.Length);
    }
}
