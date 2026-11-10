using System.Diagnostics;
using System.Text;
using CodingAdventures.Lz77;

namespace CodingAdventures.Lz77.Tests;

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

    [Fact]
    public void MaxCountHeaderOnly() =>
        Assert.Empty(Quick(() => Lz77.Decompress(B(255, 255, 255, 255))));

    [Fact]
    public void CountLargerThanRemainingBytes_YieldsPresentTokens()
    {
        var d = B(255, 255, 255, 255, 0, 0, 0, 104, 0, 0, 0, 105);
        Assert.Equal(Str("hi"), Quick(() => Lz77.Decompress(d)));
        Assert.Equal(Str("hi"), Lz77.Decompress(Cat(d, B(0, 0, 0))));
    }

    [Fact]
    public void CountSmallerThanRemainingBytes_StopsAtCount() =>
        Assert.Equal(Str("h"), Lz77.Decompress(B(0, 0, 0, 1, 0, 0, 0, 104, 0, 0, 0, 105)));

    [Fact]
    public void InvalidOffsets_AreRejected()
    {
        Assert.Throws<InvalidOperationException>(() => Lz77.Decompress(B(0, 0, 0, 1, 0, 5, 3, 120)));
        Assert.Throws<InvalidOperationException>(() => Lz77.Decompress(B(0, 0, 0, 1, 0, 0, 3, 120)));
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
            Assert.Equal(d, Lz77.Decompress(Lz77.Compress(d)));
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

            Assert.Equal(d, Lz77.Decompress(Lz77.Compress(d)));
        }
    }
}
