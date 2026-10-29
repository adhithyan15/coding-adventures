using System.Text;
using Sha512Algorithm = CodingAdventures.Sha512.Sha512;

namespace CodingAdventures.Sha512.Tests;

public sealed class Sha512Tests
{
    private static string[] FixtureLines()
    {
        const string path = "../../../../../../../../specs/fixtures/sha512-v1/cases.txt";
        var info = new FileInfo(path);
        Assert.InRange(info.Length, 1, 4096);
        var lines = File.ReadAllLines(path).Where(line => line.Length > 0 && line[0] != '#').ToArray();
        Assert.InRange(lines.Length, 1, 64);
        return lines;
    }

    [Fact]
    public void PortableCorpusMatchesOneShotAndStreaming()
    {
        foreach (var line in FixtureLines().Where(line => line.StartsWith("V|", StringComparison.Ordinal)))
        {
            var fields = line.Split('|');
            Assert.Equal(5, fields.Length);
            Assert.InRange(fields[2].Length, 0, 8192);
            Assert.Equal(128, fields[4].Length);
            var data = Convert.FromHexString(fields[2]);
            Assert.InRange(data.Length, 0, 4096);
            Assert.Equal(fields[4], Sha512Algorithm.HashHex(data));

            var hasher = new Sha512Hasher();
            var offset = 0;
            var chunks = fields[3].Length == 0 ? Array.Empty<int>() : fields[3].Split(',').Select(int.Parse).ToArray();
            Assert.InRange(chunks.Length, 0, 64);
            foreach (var size in chunks)
            {
                Assert.True(size > 0 && offset + size <= data.Length);
                hasher.Update(data.AsSpan(offset, size).ToArray());
                offset += size;
            }
            Assert.Equal(data.Length, offset);
            Assert.Equal(fields[4], hasher.HexDigest());
        }
    }

    [Fact]
    public void PortableLengthCorpusExercisesFull128BitField()
    {
        foreach (var line in FixtureLines().Where(line => line.StartsWith("L|", StringComparison.Ordinal)))
        {
            var fields = line.Split('|');
            var high = Convert.ToUInt64(fields[2], 16);
            var low = Convert.ToUInt64(fields[3], 16);
            if (fields[4] == "ERROR")
            {
                Assert.Throws<ArgumentOutOfRangeException>(() => Sha512Algorithm.EncodeBitLength(high, low));
            }
            else
            {
                Assert.Equal(fields[4], Convert.ToHexString(Sha512Algorithm.EncodeBitLength(high, low)).ToLowerInvariant());
            }
        }
    }

    [Theory]
    [InlineData("", "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e")]
    [InlineData("abc", "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f")]
    [InlineData("abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu", "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909")]
    public void HashMatchesFipsVectors(string input, string expected)
    {
        var data = Encoding.ASCII.GetBytes(input);

        Assert.Equal(Sha512Algorithm.DigestLength, Sha512Algorithm.Hash(data).Length);
        Assert.Equal(expected, Sha512Algorithm.HashHex(data));
    }

    [Fact]
    public void HashHandlesLargeInput()
    {
        var data = Enumerable.Repeat((byte)'a', 1_000_000).ToArray();

        Assert.Equal(
            "e718483d0ce769644e2e42c7bc15b4638e1f98b13b2044285632a803afa973ebde0ff244877ea60a4cb0432ce577c31beb009c5c2c49aa2e4eadb217ad8cc09b",
            Sha512Algorithm.HashHex(data));
    }

    [Fact]
    public void HexDigestIsLowercase()
    {
        var hex = Sha512Algorithm.HashHex(Encoding.ASCII.GetBytes("abc"));

        Assert.Equal(128, hex.Length);
        Assert.Equal(hex.ToLowerInvariant(), hex);
    }

    [Fact]
    public void HashRejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() => Sha512Algorithm.Hash(null!));
        Assert.Throws<ArgumentNullException>(() => Sha512Algorithm.HashHex(null!));
    }

    [Fact]
    public void StreamingMatchesOneShotAcrossChunking()
    {
        var data = Enumerable.Range(0, 256).Select(value => (byte)value).ToArray();

        foreach (var chunkSize in new[] { 1, 7, 13, 32, 63, 64, 65, 100, 128, 256 })
        {
            var hasher = new Sha512Hasher();
            for (var offset = 0; offset < data.Length; offset += chunkSize)
            {
                hasher.Update(data.Skip(offset).Take(chunkSize).ToArray());
            }

            Assert.Equal(Sha512Algorithm.Hash(data), hasher.Digest());
        }
    }

    [Fact]
    public void StreamingDigestIsNonDestructiveAndChainable()
    {
        var hasher = new Sha512Hasher();

        var result = hasher.Update("a"u8.ToArray()).Update("b"u8.ToArray()).Update("c"u8.ToArray());

        Assert.Same(hasher, result);
        Assert.Equal(hasher.Digest(), hasher.Digest());
        Assert.Equal(Sha512Algorithm.HashHex("abc"u8.ToArray()), hasher.HexDigest());
    }

    [Fact]
    public void StreamingCanContinueAfterDigest()
    {
        var hasher = new Sha512Hasher();

        hasher.Update("ab"u8.ToArray());
        _ = hasher.Digest();
        hasher.Update("c"u8.ToArray());

        Assert.Equal(Sha512Algorithm.Hash("abc"u8.ToArray()), hasher.Digest());
    }

    [Fact]
    public void CopyIsIndependent()
    {
        var baseHasher = new Sha512Hasher();
        baseHasher.Update("ab"u8.ToArray());

        var copy = baseHasher.Copy();
        copy.Update("c"u8.ToArray());
        baseHasher.Update("x"u8.ToArray());

        Assert.Equal(Sha512Algorithm.Hash("abc"u8.ToArray()), copy.Digest());
        Assert.Equal(Sha512Algorithm.Hash("abx"u8.ToArray()), baseHasher.Digest());
    }

    [Fact]
    public void UpdateRejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() => new Sha512Hasher().Update(null!));
    }

    [Fact]
    public void UpdateConsumesInputWithoutAliasing()
    {
        var input = "abc"u8.ToArray();
        var hasher = new Sha512Hasher().Update(input);
        input[0] = (byte)'z';
        Assert.Equal(Sha512Algorithm.Hash("abc"u8.ToArray()), hasher.Digest());
    }
}
