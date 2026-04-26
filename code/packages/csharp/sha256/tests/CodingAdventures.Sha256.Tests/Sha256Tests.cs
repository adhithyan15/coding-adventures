using System.Text;
using Sha256Algorithm = CodingAdventures.Sha256.Sha256;

namespace CodingAdventures.Sha256.Tests;

public sealed class Sha256Tests
{
    [Theory]
    [InlineData("", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")]
    [InlineData("abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")]
    [InlineData("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq", "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1")]
    public void HashMatchesFipsVectors(string input, string expected)
    {
        var data = Encoding.ASCII.GetBytes(input);

        Assert.Equal(Sha256Algorithm.DigestLength, Sha256Algorithm.Hash(data).Length);
        Assert.Equal(expected, Sha256Algorithm.HashHex(data));
    }

    [Fact]
    public void HashHandlesLargeInput()
    {
        var data = Enumerable.Repeat((byte)'a', 1_000_000).ToArray();

        Assert.Equal(
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
            Sha256Algorithm.HashHex(data));
    }

    [Fact]
    public void HexDigestIsLowercase()
    {
        var hex = Sha256Algorithm.HashHex(Encoding.ASCII.GetBytes("abc"));

        Assert.Equal(64, hex.Length);
        Assert.Equal(hex.ToLowerInvariant(), hex);
    }

    [Fact]
    public void HashRejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() => Sha256Algorithm.Hash(null!));
        Assert.Throws<ArgumentNullException>(() => Sha256Algorithm.HashHex(null!));
    }

    [Fact]
    public void StreamingMatchesOneShotAcrossChunking()
    {
        var data = Enumerable.Range(0, 200).Select(value => (byte)value).ToArray();

        foreach (var chunkSize in new[] { 1, 7, 13, 32, 63, 64, 65, 100, 200 })
        {
            var hasher = new Sha256Hasher();
            for (var offset = 0; offset < data.Length; offset += chunkSize)
            {
                hasher.Update(data.Skip(offset).Take(chunkSize).ToArray());
            }

            Assert.Equal(Sha256Algorithm.Hash(data), hasher.Digest());
        }
    }

    [Fact]
    public void StreamingDigestIsNonDestructiveAndChainable()
    {
        var hasher = new Sha256Hasher();

        var result = hasher.Update("a"u8.ToArray()).Update("b"u8.ToArray()).Update("c"u8.ToArray());

        Assert.Same(hasher, result);
        Assert.Equal(hasher.Digest(), hasher.Digest());
        Assert.Equal(Sha256Algorithm.HashHex("abc"u8.ToArray()), hasher.HexDigest());
    }

    [Fact]
    public void StreamingCanContinueAfterDigest()
    {
        var hasher = new Sha256Hasher();

        hasher.Update("ab"u8.ToArray());
        _ = hasher.Digest();
        hasher.Update("c"u8.ToArray());

        Assert.Equal(Sha256Algorithm.Hash("abc"u8.ToArray()), hasher.Digest());
    }

    [Fact]
    public void CopyIsIndependent()
    {
        var baseHasher = new Sha256Hasher();
        baseHasher.Update("ab"u8.ToArray());

        var copy = baseHasher.Copy();
        copy.Update("c"u8.ToArray());
        baseHasher.Update("x"u8.ToArray());

        Assert.Equal(Sha256Algorithm.Hash("abc"u8.ToArray()), copy.Digest());
        Assert.Equal(Sha256Algorithm.Hash("abx"u8.ToArray()), baseHasher.Digest());
    }

    [Fact]
    public void UpdateRejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() => new Sha256Hasher().Update(null!));
    }
}
