using System.Text;
using Sha1Algorithm = CodingAdventures.Sha1.Sha1;

namespace CodingAdventures.Sha1.Tests;

public sealed class Sha1Tests
{
    [Theory]
    [InlineData("", "da39a3ee5e6b4b0d3255bfef95601890afd80709")]
    [InlineData("abc", "a9993e364706816aba3e25717850c26c9cd0d89d")]
    [InlineData("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq", "84983e441c3bd26ebaae4aa1f95129e5e54670f1")]
    public void HashMatchesFipsVectors(string input, string expected)
    {
        var data = Encoding.ASCII.GetBytes(input);

        Assert.Equal(Sha1Algorithm.DigestLength, Sha1Algorithm.Hash(data).Length);
        Assert.Equal(expected, Sha1Algorithm.HashHex(data));
    }

    [Fact]
    public void HashHandlesLargeInput()
    {
        var data = Enumerable.Repeat((byte)'a', 1_000_000).ToArray();

        Assert.Equal("34aa973cd4c4daa4f61eeb2bdbad27316534016f", Sha1Algorithm.HashHex(data));
    }

    [Fact]
    public void HexDigestIsLowercase()
    {
        var hex = Sha1Algorithm.HashHex(Encoding.ASCII.GetBytes("abc"));

        Assert.Equal(40, hex.Length);
        Assert.Equal(hex.ToLowerInvariant(), hex);
    }

    [Fact]
    public void HashRejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() => Sha1Algorithm.Hash(null!));
        Assert.Throws<ArgumentNullException>(() => Sha1Algorithm.HashHex(null!));
    }

    [Fact]
    public void StreamingMatchesOneShotAcrossChunking()
    {
        var data = Enumerable.Range(0, 200).Select(value => (byte)value).ToArray();

        foreach (var chunkSize in new[] { 1, 7, 13, 32, 63, 64, 65, 100, 200 })
        {
            var hasher = new Sha1Hasher();
            for (var offset = 0; offset < data.Length; offset += chunkSize)
            {
                hasher.Update(data.Skip(offset).Take(chunkSize).ToArray());
            }

            Assert.Equal(Sha1Algorithm.Hash(data), hasher.Digest());
        }
    }

    [Fact]
    public void StreamingDigestIsNonDestructiveAndChainable()
    {
        var hasher = new Sha1Hasher();

        var result = hasher.Update("a"u8.ToArray()).Update("b"u8.ToArray()).Update("c"u8.ToArray());

        Assert.Same(hasher, result);
        Assert.Equal(hasher.Digest(), hasher.Digest());
        Assert.Equal(Sha1Algorithm.HashHex("abc"u8.ToArray()), hasher.HexDigest());
    }

    [Fact]
    public void StreamingCanContinueAfterDigest()
    {
        var hasher = new Sha1Hasher();

        hasher.Update("ab"u8.ToArray());
        _ = hasher.Digest();
        hasher.Update("c"u8.ToArray());

        Assert.Equal(Sha1Algorithm.Hash("abc"u8.ToArray()), hasher.Digest());
    }

    [Fact]
    public void CopyIsIndependent()
    {
        var baseHasher = new Sha1Hasher();
        baseHasher.Update("ab"u8.ToArray());

        var copy = baseHasher.Copy();
        copy.Update("c"u8.ToArray());
        baseHasher.Update("x"u8.ToArray());

        Assert.Equal(Sha1Algorithm.Hash("abc"u8.ToArray()), copy.Digest());
        Assert.Equal(Sha1Algorithm.Hash("abx"u8.ToArray()), baseHasher.Digest());
    }

    [Fact]
    public void UpdateRejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() => new Sha1Hasher().Update(null!));
    }
}
