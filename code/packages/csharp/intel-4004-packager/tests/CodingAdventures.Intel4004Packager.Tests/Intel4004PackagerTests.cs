namespace CodingAdventures.Intel4004Packager.Tests;

public sealed class Intel4004PackagerTests
{
    [Fact]
    public void VersionIsExposed()
    {
        Assert.Equal("0.1.0", Intel4004Packager.Version);
    }

    [Fact]
    public void EncodesRecordsWithValidChecksumsAndEof()
    {
        var hex = Intel4004Packager.EncodeHex(Enumerable.Range(0, 17).Select(i => (byte)i).ToArray());
        var lines = hex.Split('\n', StringSplitOptions.RemoveEmptyEntries);

        Assert.Equal(3, lines.Length);
        Assert.Equal("10", lines[0][1..3]);
        Assert.Equal("01", lines[1][1..3]);
        Assert.Equal(":00000001FF", lines[^1]);
        Assert.All(lines, VerifyChecksum);
    }

    [Fact]
    public void AppliesOriginAndRoundTrips()
    {
        var binary = new byte[] { 0xD5, 0x01, 0xC0 };
        var hex = Intel4004Packager.EncodeHex(binary, 0x0300);
        Assert.Equal("0300", hex.Split('\n')[0][3..7]);

        var decoded = Intel4004Packager.DecodeHex(hex);

        Assert.Equal(0x0300, decoded.Origin);
        Assert.Equal<byte>(binary, decoded.Binary);
    }

    [Fact]
    public void DecodesSparseSegmentsWithZeroFill()
    {
        var first = Intel4004Packager.EncodeHex([0xAA], 0x0010).Split('\n')[0];
        var second = Intel4004Packager.EncodeHex([0xBB], 0x0012).Split('\n')[0];

        var decoded = Intel4004Packager.DecodeHex(first + "\n" + second + "\n:00000001FF\n");

        Assert.Equal(0x0010, decoded.Origin);
        Assert.Equal<byte>([0xAA, 0x00, 0xBB], decoded.Binary);
    }

    [Fact]
    public void RejectsInvalidEncodeInputs()
    {
        Assert.Throws<ArgumentException>(() => Intel4004Packager.EncodeHex([]));
        Assert.Throws<ArgumentOutOfRangeException>(() => Intel4004Packager.EncodeHex([0x00], -1));
        Assert.Throws<ArgumentOutOfRangeException>(() => Intel4004Packager.EncodeHex([0x00], 0x10000));
        Assert.Throws<ArgumentException>(() => Intel4004Packager.EncodeHex(new byte[100], 0xFFFF));
    }

    [Fact]
    public void RejectsMalformedHex()
    {
        Assert.Throws<FormatException>(() => Intel4004Packager.DecodeHex("020000000000D5012A\n"));
        Assert.Throws<FormatException>(() => Intel4004Packager.DecodeHex(":0Z000000D5\n"));
        Assert.Throws<FormatException>(() => Intel4004Packager.DecodeHex(":01000000D500\n:00000001FF\n"));
        Assert.Throws<FormatException>(() => Intel4004Packager.DecodeHex(":10000002D5AA\n"));
        Assert.Throws<FormatException>(() => Intel4004Packager.DecodeHex(":10000000D5\n"));
    }

    [Fact]
    public void EmptyDecodeReturnsEmptyBinary()
    {
        var decoded = Intel4004Packager.DecodeHex("");

        Assert.Equal(0, decoded.Origin);
        Assert.Empty(decoded.Binary);
    }

    private static void VerifyChecksum(string line)
    {
        var bytes = Convert.FromHexString(line[1..]);
        Assert.Equal(0, bytes.Sum(b => b) % 256);
    }
}
