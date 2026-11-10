using System.Diagnostics;
using System.Text;
using CodingAdventures.Lz78;

namespace CodingAdventures.Lz78.Tests;

/// <summary>
/// Security regression tests: hostile headers must not drive work or memory.
/// Each case is a few bytes whose header claims an enormous count or length;
/// they must finish quickly with the data actually present.  Also round-trips
/// every input of length &lt;= 2 and a seeded random sample.
/// </summary>
public sealed class HostileHeadersTests
{
    private static byte[] B(params int[] v) => v.Select(x => (byte)x).ToArray();

    private static byte[] Cat(params byte[][] parts) => parts.SelectMany(p => p).ToArray();

    private static byte[] U32(uint v) => B((int)(v >> 24) & 255, (int)(v >> 16) & 255, (int)(v >> 8) & 255, (int)v & 255);

    private static byte[] Str(string s) => Encoding.Latin1.GetBytes(s);

    private static T Quick<T>(Func<T> body)
    {
        var sw = Stopwatch.StartNew();
        var result = body();
        Assert.True(sw.Elapsed < TimeSpan.FromSeconds(10), "hostile input took too long");
        return result;
    }

    private static byte[] Hdr(uint len, uint count) => Cat(U32(len), U32(count));

    [Fact]
    public void MaxHeaderOnly() =>
        Assert.Empty(Quick(() => Lz78.Decompress(Enumerable.Repeat((byte)255, 8).ToArray())));

    [Fact]
    public void MaxLengthZeroTokens() =>
        Assert.Empty(Quick(() => Lz78.Decompress(Hdr(0xFFFFFFFF, 0))));

    [Fact]
    public void CountLargerThanRemainingBytes_YieldsPresentTokens() =>
        Assert.Equal(Str("hi"), Quick(() => Lz78.Decompress(Cat(Hdr(2, 0xFFFFFFFF), B(0, 0, 104, 0, 0, 0, 105, 0)))));

    [Fact]
    public void LengthLargerThanTokensSupply() =>
        Assert.Equal(Str("h"), Quick(() => Lz78.Decompress(Cat(Hdr(0xFFFFFFFF, 1), B(0, 0, 104, 0)))));

    [Fact]
    public void DanglingDictIndex_IsRejected() =>
        Assert.Throws<InvalidOperationException>(() =>
            Lz78.Decompress(Cat(Hdr(10, 2), B(0, 0, 97, 0, 0x27, 0x0F, 98, 0))));

    [Fact]
    public void ChainTokens_StopAtDeclaredLength()
    {
        const int n = 20000;
        var body = new List<byte>();
        for (var i = 0; i < n; i++)
        {
            body.AddRange(B(i >> 8, i & 255, 97, 0));
        }

        var d = Cat(Hdr(1000, n), body.ToArray());
        Assert.Equal(1000, Quick(() => Lz78.Decompress(d)).Length);
    }

    [Fact]
    public void RoundTrip_EveryInputUpToLength2()
    {
        var inputs = new List<byte[]> { Array.Empty<byte>() };
        for (var a = 0; a < 256; a++)
        {
            inputs.Add(B(a));
            for (var b = 0; b < 256; b++)
            {
                inputs.Add(B(a, b));
            }
        }

        foreach (var d in inputs)
        {
            Assert.Equal(d, Lz78.Decompress(Lz78.Compress(d)));
        }
    }

    [Fact]
    public void RoundTrip_SeededRandomInputs()
    {
        var rng = new Random(0x5EED);
        int[] alphabets = [2, 4, 16, 256];
        for (var i = 0; i < 300; i++)
        {
            var d = new byte[3 + rng.Next(597)];
            for (var j = 0; j < d.Length; j++)
            {
                d[j] = (byte)rng.Next(alphabets[i % 4]);
            }

            Assert.Equal(d, Lz78.Decompress(Lz78.Compress(d)));
        }
    }
}
