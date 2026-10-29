using System.Text;
using System.Text.RegularExpressions;
using Sha512Algorithm = CodingAdventures.Sha512.Sha512;

namespace CodingAdventures.Sha512.Tests;

public sealed class Sha512Tests
{
    private static readonly Regex IdPattern = new("^[a-z0-9]+(?:-[a-z0-9]+)*$", RegexOptions.CultureInvariant);
    private static readonly Regex EvenHexPattern = new("^(?:[0-9a-f]{2})*$", RegexOptions.CultureInvariant);
    private static readonly Regex DigestPattern = new("^[0-9a-f]{128}$", RegexOptions.CultureInvariant);
    private static readonly Regex WordPattern = new("^[0-9a-f]{16}$", RegexOptions.CultureInvariant);
    private static readonly Regex ChunksPattern = new("^[1-9][0-9]*(?:,[1-9][0-9]*)*$", RegexOptions.CultureInvariant);

    private static string[] ParseFixture(byte[] raw)
    {
        if (raw.Length is < 1 or > 4096) throw new InvalidDataException("fixture byte size outside bounds");
        var text = new UTF8Encoding(false, true).GetString(raw);
        var lines = text.Split('\n').Select(line => line.TrimEnd('\r'))
            .Where(line => line.Length > 0 && line[0] != '#').ToArray();
        if (lines.Length is < 1 or > 64) throw new InvalidDataException("fixture row count outside bounds");
        foreach (var line in lines) ValidateFixtureRow(line);
        return lines;
    }

    private static string[] FixtureLines(string path = "../../../../../../../../specs/fixtures/sha512-v1/cases.txt")
    {
        using var stream = File.OpenRead(path);
        var raw = new byte[4097];
        var length = 0;
        while (length < raw.Length)
        {
            var count = stream.Read(raw, length, raw.Length - length);
            if (count == 0) break;
            length += count;
        }
        var lines = ParseFixture(raw[..length]);
        Assert.Equal(9, lines.Count(line => line.StartsWith("V|", StringComparison.Ordinal)));
        Assert.Equal(6, lines.Count(line => line.StartsWith("L|", StringComparison.Ordinal)));
        return lines;
    }

    [Fact]
    public void FixtureReaderRejectsOversizedFile()
    {
        var path = Path.GetTempFileName();
        try
        {
            File.WriteAllBytes(path, new byte[4097]);
            Assert.Throws<InvalidDataException>(() => FixtureLines(path));
        }
        finally
        {
            File.Delete(path);
        }
    }

    private static void ValidateFixtureRow(string line)
    {
        var fields = line.Split('|');
        if (fields.Length != 5 || !IdPattern.IsMatch(fields[1])) throw new InvalidDataException("malformed fixture row");
        if (fields[0] == "V")
        {
            if (fields[2].Length > 8192 || !EvenHexPattern.IsMatch(fields[2]) || !DigestPattern.IsMatch(fields[4]))
                throw new InvalidDataException("malformed vector row");
            var byteCount = fields[2].Length / 2;
            if (byteCount > 4096 || (byteCount == 0) != (fields[3].Length == 0)) throw new InvalidDataException("malformed vector size");
            var chunks = fields[3].Length == 0 ? Array.Empty<int>() : fields[3].Split(',').Select(value =>
                int.TryParse(value, out var size) && size <= 4096 ? size : throw new InvalidDataException("malformed chunk")).ToArray();
            if (chunks.Length > 64 || (fields[3].Length > 0 && !ChunksPattern.IsMatch(fields[3])) || chunks.Sum() != byteCount)
                throw new InvalidDataException("malformed chunks");
        }
        else if (fields[0] == "L")
        {
            if (!WordPattern.IsMatch(fields[2]) || !WordPattern.IsMatch(fields[3]) ||
                (fields[4] != "ERROR" && !Regex.IsMatch(fields[4], "^[0-9a-f]{32}$", RegexOptions.CultureInvariant)))
                throw new InvalidDataException("malformed length row");
        }
        else throw new InvalidDataException("unknown fixture row tag");
    }

    [Fact]
    public void FixtureReaderRejectsMalformedRows()
    {
        foreach (var row in new[] { "X|id|||", "V|UPPER|00|1|" + new string('0', 128),
                     "V|bad|0|1|" + new string('0', 128), "V|bad|00|2|" + new string('0', 128),
                     "L|bad|000000000000000G|0000000000000000|ERROR", "L|bad|0000000000000000|0000000000000000|ABC" })
            Assert.Throws<InvalidDataException>(() => ParseFixture(Encoding.ASCII.GetBytes(row)));
    }

    [Fact]
    public void PortableCorpusMatchesOneShotAndStreaming()
    {
        foreach (var line in FixtureLines().Where(line => line.StartsWith("V|", StringComparison.Ordinal)))
        {
            var fields = line.Split('|');
            var data = Convert.FromHexString(fields[2]);
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
