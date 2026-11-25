using System.Collections;
using System.Numerics;
using CodingAdventures.TestBootstrap.CorpusJson;
using CodingAdventures.TestBootstrap.Pbt;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>Canonical witness rendering: stable shapes and total behaviour on hostile values.</summary>
public sealed class CanonTests : HookedTest
{
    private sealed class Odd
    {
        public override string ToString() => "odd\n\"thing\"";
    }

    private sealed class ThrowingToString
    {
        public override string ToString() => throw new InvalidOperationException("nope");
    }

    private sealed class NullToString
    {
        public override string? ToString() => null;
    }

    private sealed class ThrowingTuple : System.Runtime.CompilerServices.ITuple
    {
        public int Length => 2;

        public object? this[int index] => throw new InvalidOperationException("indexer");
    }

    private sealed class ThrowingDictionary : IDictionary
    {
        public object? this[object key] { get => null; set { } }

        public ICollection Keys => Array.Empty<object>();

        public ICollection Values => Array.Empty<object>();

        public bool IsReadOnly => true;

        public bool IsFixedSize => true;

        public int Count => 0;

        public bool IsSynchronized => false;

        public object SyncRoot => this;

        public void Add(object key, object? value) { }

        public void Clear() { }

        public bool Contains(object key) => false;

        public void CopyTo(Array array, int index) { }

        public IDictionaryEnumerator GetEnumerator() => throw new InvalidOperationException("dict");

        public void Remove(object key) { }

        IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
    }

    [Fact]
    public void RendersCommonShapes()
    {
        Assert.Equal("true", Canon.Render(true));
        Assert.Equal("false", Canon.Render(false));
        Assert.Equal("nil", Canon.Render(null));
        Assert.Equal("-12", Canon.Render(new BigInteger(-12)));
        Assert.Equal("7", Canon.Render(7));
        Assert.Equal("255", Canon.Render((byte)255));
        Assert.Equal("18446744073709551615", Canon.Render(ulong.MaxValue));
        Assert.Equal("170141183460469231731687303715884105727", Canon.Render(Int128.MaxValue));
        Assert.Equal("1.5", Canon.Render(1.5));
        Assert.Equal("2.5", Canon.Render(2.5f));
        Assert.Equal("NaN", Canon.Render(double.NaN));
        Assert.Equal("3.25", Canon.Render(3.25m));
        Assert.Equal("0x00ff", Canon.Render(new byte[] { 0, 255 }));
        Assert.Equal("0x", Canon.Render(Array.Empty<byte>()));
        Assert.Equal("[1,2]", Canon.Render(new List<int> { 1, 2 }));
        Assert.Equal("[1,2]", Canon.Render(new[] { 1, 2 }));
        Assert.Equal("[]", Canon.Render(new List<int>()));
        Assert.Equal("{1,2}", Canon.Render(new Pair<int, int>(1, 2)));
        Assert.Equal("{1,true,\"x\"}", Canon.Render(new Triple<int, bool, string>(1, true, "x")));
        Assert.Equal("{1,2}", Canon.Render((1, 2))); // ValueTuple is an ITuple too
        Assert.Equal("\"a\\\"b\\\\c\\n\\r\\t\\u0001\\u007f\"", Canon.Render("a\"b\\c\n\r\t\u0001\u007f"));
        Assert.Equal("\"x\"", Canon.Render('x'));
        Assert.Equal("[[1],[2,3]]", Canon.Render(new List<List<int>> { new() { 1 }, new() { 2, 3 } }));
    }

    [Fact]
    public void DictionariesRenderSortedByRenderedKey()
    {
        Dictionary<string, int> d = new() { ["b"] = 2, ["a"] = 1 };
        Assert.Equal("{\"a\":1,\"b\":2}", Canon.Render(d));
        Assert.Equal("{}", Canon.Render(new Dictionary<string, int>()));
    }

    [Fact]
    public void OtherObjectsRenderAsQuotedToStringAndNeverThrow()
    {
        Assert.Equal("\"odd\\n\\\"thing\\\"\"", Canon.Render(new Odd()));
        Assert.Equal("\"<" + typeof(ThrowingToString).FullName + ">\"", Canon.Render(new ThrowingToString()));
        Assert.Equal("\"\"", Canon.Render(new NullToString()));
        Assert.Equal(Canon.Unrenderable, Canon.Render(new ThrowingTuple()));
        Assert.Equal(Canon.Unrenderable, Canon.Render(new ThrowingDictionary()));
        Assert.Equal(Canon.Unrenderable, Canon.Render(new object[] { 1, new ThrowingTuple() }));
        Assert.Equal(200 + 3 + 2, Canon.Render(new LongToString()).Length); // clipped to 200 + "..." + quotes
    }

    private sealed class LongToString
    {
        public override string ToString() => new('z', 5000);
    }

    [Fact]
    public void LongStringsAndHugeIntegersAreBounded()
    {
        Assert.True(Canon.Render(new string('s', 1_000_000)).Length < 10_100);
        BigInteger huge = BigInteger.One << 200_000;
        Assert.Equal("<bigint 200001 bits>", Canon.Render(huge));
        BigInteger edge = (BigInteger.One << Canon.MaxBigIntBits) - 1;
        Assert.DoesNotContain("<bigint", Canon.Render(edge));
        Assert.EndsWith("...", Canon.Render(new byte[200_000]));
    }

    [Fact]
    public void DepthCapAppliesToEveryContainerKind()
    {
        object nested = 1;
        for (int i = 0; i < 20; i++)
        {
            nested = new Pair<object, int>(nested, 0);
        }

        Assert.Contains("...", Canon.Render(nested));
        object dictNest = 1;
        for (int i = 0; i < 20; i++)
        {
            dictNest = new Dictionary<string, object> { ["k"] = dictNest };
        }

        Assert.Contains("...", Canon.Render(dictNest));
        object arrNest = 1;
        for (int i = 0; i < 20; i++)
        {
            arrNest = new[] { arrNest };
        }

        Assert.Contains("...", Canon.Render(arrNest));
        object listNest = 1;
        for (int i = 0; i < 20; i++)
        {
            listNest = new List<object> { listNest };
        }

        Assert.Contains("...", Canon.Render(listNest));
    }

    [Fact]
    public void NodeBudgetStopsDictionariesAndTuples()
    {
        Dictionary<int, int> big = Enumerable.Range(0, 30_000).ToDictionary(i => i, i => i);
        string r = Canon.Render(big);
        Assert.True(r.Length < 200_000);
        List<Pair<int, int>> pairs = Enumerable.Range(0, 30_000).Select(i => new Pair<int, int>(i, i)).ToList();
        Assert.EndsWith("...]", Canon.Render(pairs));
    }

    [Fact]
    public void SafeStringNeverThrows()
    {
        Assert.Equal("null", Safe.SafeString(null));
        Assert.Equal("hello", Safe.SafeString("hello"));
        Assert.Equal(typeof(ThrowingToString).FullName, Safe.SafeString(new ThrowingToString()));
        Assert.Equal(203, Safe.SafeString(new string('q', 1000)).Length);
        Assert.Equal("null", Safe.SafeString(new NullToString()));
    }

    [Fact]
    public void ClipNeverSplitsASurrogatePair()
    {
        string s = new string('a', 9) + "\U0001F600" + "tail";
        Assert.Equal(new string('a', 9) + "...", Clip.Apply(s, 10));
        Assert.Equal(s, Clip.Apply(s, 100));
        Assert.Equal("...", Clip.Apply("abc", 0));
        Assert.Equal("ab...", Clip.Apply("abcdef", 2));
    }
}
