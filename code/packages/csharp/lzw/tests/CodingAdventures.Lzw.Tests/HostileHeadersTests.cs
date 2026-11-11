using System.Diagnostics;
using System.Text;
using CodingAdventures.Lzw;

namespace CodingAdventures.Lzw.Tests;

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
    public void MaxHeaderOnly()
    {
        foreach (var n in new[] { 4, 8, 64 })
        {
            try
            {
                Quick(() => Lzw.Decompress(Enumerable.Repeat((byte)255, n).ToArray()));
            }
            catch (InvalidOperationException)
            {
            }
        }
    }

    [Fact]
    public void MaxDeclaredLength_OnSmallStream()
    {
        var c = Lzw.Compress(Str("hello hello hello"));
        U32(0xFFFFFFFF).CopyTo(c, 0);
        Assert.Equal(Str("hello hello hello"), Quick(() => Lzw.Decompress(c)));
    }

    [Fact]
    public void DeclaredLength_TruncatesOutput()
    {
        var c = Lzw.Compress(Str("hello hello hello"));
        U32(5).CopyTo(c, 0);
        Assert.Equal(Str("hello"), Quick(() => Lzw.Decompress(c)));
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
            Assert.Equal(d, Lzw.Decompress(Lzw.Compress(d)));
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

            Assert.Equal(d, Lzw.Decompress(Lzw.Compress(d)));
        }
    }
}
