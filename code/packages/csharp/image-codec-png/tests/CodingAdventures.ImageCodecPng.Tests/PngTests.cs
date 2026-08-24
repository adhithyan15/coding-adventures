using System.Buffers.Binary;
using System.Runtime.CompilerServices;
using CodingAdventures.PixelContainer;
using CodingAdventures.Zip;
using PixelBuffer = CodingAdventures.PixelContainer.PixelContainer;

namespace CodingAdventures.ImageCodecPng.Tests;

public sealed class PngTests
{
    [Fact]
    public void CodecImplementsTheImageContractAndRoundTrips()
    {
        IImageCodec codec = new PngCodec();
        Assert.Equal("image/png", codec.MimeType);
        var pixels = new PixelBuffer(2, 1, [1, 2, 3, 4, 250, 240, 230, 220]);
        var decoded = codec.Decode(codec.Encode(pixels));
        Assert.Equal(2, decoded.Width);
        Assert.Equal(1, decoded.Height);
        Assert.Equal(pixels.Data, decoded.Data);

        IImageCodec limited = new PngCodec(2);
        Assert.Equal(pixels.Data, limited.Decode(limited.Encode(pixels)).Data);
    }

    public static TheoryData<double> InvalidLimits => new()
    {
        0,
        -1,
        1.5,
        Png.DefaultMaxPixels + 1,
        double.NaN,
        double.PositiveInfinity,
        double.NegativeInfinity,
    };

    [Theory]
    [MemberData(nameof(InvalidLimits))]
    public void CallerPixelLimitMustBeAPositiveLoweringInteger(double value)
    {
        var constructorError = Assert.Throws<PngError>(() => new PngCodec(value));
        Assert.Equal("invalid-max-pixels", constructorError.Code);

        var decodeError = Assert.Throws<PngError>(() => Png.DecodePng([], value));
        Assert.Equal("invalid-max-pixels", decodeError.Code);
    }

    [Fact]
    public void ErrorTaxonomyIsClosedAndReturnedAsAReadOnlyCopy()
    {
        var first = Png.ErrorCodes;
        var second = Png.ErrorCodes;
        Assert.Equal(29, first.Count);
        Assert.NotSame(first, second);
        Assert.Throws<NotSupportedException>(() => ((IList<string>)first)[0] = "changed");
        Assert.Equal("invalid-max-pixels", second[0]);
    }

    [Fact]
    public void EncoderRejectsTypedShapeAndResourceFailuresBeforeFiltering()
    {
        AssertCode("invalid-image-dimensions", () => Png.EncodePng(null!));
        AssertCode("invalid-image-dimensions", () => Png.EncodePng(new PixelBuffer(0, 1)));
        AssertCode("invalid-image-dimensions", () => Png.EncodePng(MalformedContainer(Png.MaxDimension + 1, 1, [])));
        AssertCode("invalid-image-dimensions", () => Png.EncodePng(MalformedContainer(8192, 4097, [])));
        AssertCode("invalid-pixel-data-length", () => Png.EncodePng(MalformedContainer(1, 1, [1, 2, 3])));
    }

    [Fact]
    public void APNGNamesAreRejectedAfterOrdinaryCRCValidation()
    {
        var encoded = Png.EncodePng(new PixelBuffer(1, 1));
        foreach (var type in new[] { "acTL", "fcTL", "fdAT" })
        {
            AssertCode("unsupported-feature", () => Png.DecodePng(InsertAfterIhdr(encoded, type, validCrc: true)));
            AssertCode("chunk-crc-mismatch", () => Png.DecodePng(InsertAfterIhdr(encoded, type, validCrc: false)));
        }
    }

    [Fact]
    public void Adler32MatchesThePublishedVectorAndChunkBoundary()
    {
        Assert.Equal(0x11e60398u, Png.Adler32("Wikipedia"u8.ToArray()));
        var data = Enumerable.Range(0, 6000).Select(i => (byte)(i * 31)).ToArray();
        using var stream = new MemoryStream();
        using (var zlib = new System.IO.Compression.ZLibStream(stream, System.IO.Compression.CompressionLevel.SmallestSize, leaveOpen: true))
            zlib.Write(data);
        var wrapped = stream.ToArray();
        Assert.Equal(Png.Adler32(data), BinaryPrimitives.ReadUInt32BigEndian(wrapped.AsSpan(wrapped.Length - 4)));
    }

    private static void AssertCode(string expected, Action action)
    {
        var error = Assert.Throws<PngError>(action);
        Assert.Equal(expected, error.Code);
        Assert.Equal(expected, error.Message);
        Assert.Empty(error.Data);
    }

    private static PixelBuffer MalformedContainer(int width, int height, byte[] data)
    {
        var result = (PixelBuffer)RuntimeHelpers.GetUninitializedObject(typeof(PixelBuffer));
        typeof(PixelBuffer).GetField("<Width>k__BackingField", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!.SetValue(result, width);
        typeof(PixelBuffer).GetField("<Height>k__BackingField", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!.SetValue(result, height);
        typeof(PixelBuffer).GetField("<Data>k__BackingField", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!.SetValue(result, data);
        return result;
    }

    private static byte[] InsertAfterIhdr(byte[] png, string type, bool validCrc)
    {
        const int ihdrEnd = 8 + 12 + 13;
        var typeBytes = System.Text.Encoding.ASCII.GetBytes(type);
        var chunk = new byte[12];
        typeBytes.CopyTo(chunk, 4);
        var crc = RawRfc1951.Crc32(typeBytes);
        BinaryPrimitives.WriteUInt32BigEndian(chunk.AsSpan(8), validCrc ? crc : crc ^ 1);
        return [.. png[..ihdrEnd], .. chunk, .. png[ihdrEnd..]];
    }
}
