using System.Buffers.Binary;
using System.Text;
using CodingAdventures.Lzw;

namespace CodingAdventures.Lzw.Tests;

public sealed class ConstantTests
{
    [Fact]
    public void ConstantsMatchCmp03()
    {
        Assert.Equal(256, Lzw.CLEAR_CODE);
        Assert.Equal(257, Lzw.STOP_CODE);
        Assert.Equal(258, Lzw.INITIAL_NEXT_CODE);
        Assert.Equal(9, Lzw.INITIAL_CODE_SIZE);
        Assert.Equal(16, Lzw.MAX_CODE_SIZE);
    }
}

public sealed class BitIoTests
{
    [Fact]
    public void SingleNineBitCodeRoundTrips()
    {
        var writer = new BitWriter();
        writer.Write(256, 9);
        writer.Flush();

        var reader = new BitReader(writer.ToArray());
        Assert.Equal(256, reader.Read(9));
    }

    [Fact]
    public void MultipleNineBitCodesRoundTrip()
    {
        var writer = new BitWriter();
        foreach (var code in new[] { Lzw.CLEAR_CODE, 65, 66, 258, Lzw.STOP_CODE })
        {
            writer.Write(code, 9);
        }

        writer.Flush();
        var reader = new BitReader(writer.ToArray());

        Assert.Equal(Lzw.CLEAR_CODE, reader.Read(9));
        Assert.Equal(65, reader.Read(9));
        Assert.Equal(66, reader.Read(9));
        Assert.Equal(258, reader.Read(9));
        Assert.Equal(Lzw.STOP_CODE, reader.Read(9));
    }

    [Fact]
    public void ExhaustedReaderThrows()
    {
        var reader = new BitReader([]);
        Assert.Throws<InvalidOperationException>(() => reader.Read(9));
    }
}

public sealed class EncodeDecodeCodeTests
{
    private static byte[] Bytes(string value) => Encoding.UTF8.GetBytes(value);

    [Fact]
    public void EmptyInputEncodesToClearAndStop()
    {
        var (codes, originalLength) = Lzw.EncodeCodes([]);
        Assert.Equal(0, originalLength);
        Assert.Equal([Lzw.CLEAR_CODE, Lzw.STOP_CODE], codes);
    }

    [Fact]
    public void AbEncodesToExpectedVector()
    {
        var (codes, _) = Lzw.EncodeCodes(Bytes("AB"));
        Assert.Equal([Lzw.CLEAR_CODE, 65, 66, Lzw.STOP_CODE], codes);
    }

    [Fact]
    public void AbababEncodesToExpectedVector()
    {
        var (codes, _) = Lzw.EncodeCodes(Bytes("ABABAB"));
        Assert.Equal([Lzw.CLEAR_CODE, 65, 66, 258, 258, Lzw.STOP_CODE], codes);
    }

    [Fact]
    public void AaaaaaaEncodesToTrickyTokenVector()
    {
        var (codes, _) = Lzw.EncodeCodes(Bytes("AAAAAAA"));
        Assert.Equal([Lzw.CLEAR_CODE, 65, 258, 259, 65, Lzw.STOP_CODE], codes);
    }

    [Fact]
    public void AbababDecodesFromExpectedVector()
    {
        var decoded = Lzw.DecodeCodes([Lzw.CLEAR_CODE, 65, 66, 258, 258, Lzw.STOP_CODE]);
        Assert.Equal(Bytes("ABABAB"), decoded);
    }

    [Fact]
    public void TrickyTokenVectorDecodes()
    {
        var decoded = Lzw.DecodeCodes([Lzw.CLEAR_CODE, 65, 258, 259, 65, Lzw.STOP_CODE]);
        Assert.Equal(Bytes("AAAAAAA"), decoded);
    }

    [Fact]
    public void ClearResetsDictionary()
    {
        var decoded = Lzw.DecodeCodes([Lzw.CLEAR_CODE, 65, Lzw.CLEAR_CODE, 66, Lzw.STOP_CODE]);
        Assert.Equal(Bytes("AB"), decoded);
    }

    [Fact]
    public void InvalidCodeThrows()
    {
        var error = Assert.Throws<InvalidOperationException>(() => Lzw.DecodeCodes([Lzw.CLEAR_CODE, 9999, 65, Lzw.STOP_CODE]));
        Assert.Contains("invalid LZW code", error.Message);
    }
}

public sealed class PackUnpackTests
{
    [Fact]
    public void HeaderStoresOriginalLengthBigEndian()
    {
        var packed = Lzw.PackCodes([Lzw.CLEAR_CODE, Lzw.STOP_CODE], 42);
        Assert.Equal(42u, BinaryPrimitives.ReadUInt32BigEndian(packed.AsSpan(0, 4)));
    }

    [Fact]
    public void AbababCodesRoundTripThroughWireFormat()
    {
        var codes = new List<int> { Lzw.CLEAR_CODE, 65, 66, 258, 258, Lzw.STOP_CODE };
        var packed = Lzw.PackCodes(codes, 6);
        var (unpacked, originalLength) = Lzw.UnpackCodes(packed);

        Assert.Equal(6, originalLength);
        Assert.Equal(codes, unpacked);
    }

    [Fact]
    public void TruncatedDataDoesNotCrashUnpack()
    {
        var (codes, originalLength) = Lzw.UnpackCodes([0, 0]);
        Assert.Empty(codes);
        Assert.Equal(0, originalLength);
    }
}

public sealed class CompressDecompressTests
{
    [Theory]
    [InlineData("")]
    [InlineData("A")]
    [InlineData("AB")]
    [InlineData("ABABAB")]
    [InlineData("AAAAAAA")]
    [InlineData("AABABC")]
    public void StringVectorsRoundTrip(string value)
    {
        var bytes = Bytes(value);
        Assert.Equal(bytes, Lzw.Decompress(Lzw.Compress(bytes)));
    }

    [Fact]
    public void RepetitiveTextRoundTrips()
    {
        var value = Bytes(string.Concat(Enumerable.Repeat("the quick brown fox jumps over the lazy dog ", 20)));
        Assert.Equal(value, Lzw.Decompress(Lzw.Compress(value)));
    }

    [Fact]
    public void BinaryDataRoundTrips()
    {
        var value = new byte[1024];
        for (var index = 0; index < value.Length; index++)
        {
            value[index] = (byte)(index % 256);
        }

        Assert.Equal(value, Lzw.Decompress(Lzw.Compress(value)));
    }

    [Fact]
    public void HeaderContainsOriginalLength()
    {
        var input = Bytes("hello world");
        var compressed = Lzw.Compress(input);
        Assert.Equal((uint)input.Length, BinaryPrimitives.ReadUInt32BigEndian(compressed.AsSpan(0, 4)));
    }

    private static byte[] Bytes(string value) => Encoding.UTF8.GetBytes(value);
}
