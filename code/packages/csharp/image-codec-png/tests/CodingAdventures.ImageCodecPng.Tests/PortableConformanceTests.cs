using System.Buffers.Binary;
using System.IO.Compression;
using System.Text;
using System.Text.Json;
using CodingAdventures.PixelContainer;
using PixelBuffer = CodingAdventures.PixelContainer.PixelContainer;

namespace CodingAdventures.ImageCodecPng.Tests;

public sealed class PortableConformanceTests
{
    private sealed record Chunk(string Type, byte[] Data);

    [Fact]
    public void ClosedPortableCorpusPassesThroughThePublicAPI()
    {
        using var document = JsonDocument.Parse(File.ReadAllText(FindFixture()));
        var root = document.RootElement;
        Assert.Equal(1, root.GetProperty("schema_version").GetInt32());
        Assert.Equal("image-codec-png-v1", root.GetProperty("profile").GetString());
        Assert.Equal(Png.MaxDimension, root.GetProperty("limits").GetProperty("max_dimension").GetInt32());
        Assert.Equal(Png.DefaultMaxPixels, root.GetProperty("limits").GetProperty("default_max_pixels").GetInt32());
        Assert.Equal(Png.ErrorCodes, root.GetProperty("error_ids").EnumerateArray().Select(value => value.GetString()!).ToArray());

        var cases = root.GetProperty("cases").EnumerateArray().ToArray();
        Assert.Equal(85, cases.Length);
        foreach (var testCase in cases)
        {
            var id = testCase.GetProperty("id").GetString()!;
            var operation = testCase.GetProperty("operation").GetString()!;
            switch (operation)
            {
                case "decode":
                    AssertDecode(testCase, id);
                    break;
                case "decode-error":
                    AssertDecodeError(testCase, id);
                    break;
                case "encode":
                    AssertEncode(testCase, id);
                    break;
                case "encode-error":
                    AssertEncodeError(testCase, id);
                    break;
                case "adler32":
                    Assert.Equal(
                        testCase.GetProperty("expected").GetProperty("adler32_hex").GetString(),
                        Png.Adler32(FromHex(testCase.GetProperty("input_hex").GetString()!)).ToString("x8"));
                    break;
                default:
                    throw new Xunit.Sdk.XunitException($"{id}: unknown operation {operation}");
            }
        }
    }

    private static void AssertDecode(JsonElement testCase, string id)
    {
        var decoded = DecodeFixture(testCase);
        var expected = testCase.GetProperty("expected");
        Assert.Equal(expected.GetProperty("width").GetInt32(), decoded.Width);
        Assert.Equal(expected.GetProperty("height").GetInt32(), decoded.Height);
        Assert.True(decoded.Data.SequenceEqual(FromHex(expected.GetProperty("rgba_hex").GetString()!)), id);
    }

    private static void AssertDecodeError(JsonElement testCase, string id)
    {
        var error = Assert.Throws<PngError>(() => DecodeFixture(testCase));
        Assert.Equal(testCase.GetProperty("expected").GetProperty("error_id").GetString(), error.Code);
        Assert.Equal(error.Code, error.Message);
        Assert.True(error.Message.Length <= 40, id);
    }

    private static PixelBuffer DecodeFixture(JsonElement testCase)
    {
        var bytes = FromHex(testCase.GetProperty("png_hex").GetString()!);
        return testCase.TryGetProperty("options", out var options)
            ? Png.DecodePng(bytes, options.GetProperty("max_pixels").GetDouble())
            : Png.DecodePng(bytes);
    }

    private static void AssertEncode(JsonElement testCase, string id)
    {
        var input = testCase.GetProperty("input");
        var pixels = FixturePixels(input);
        var encoded = Png.EncodePng(pixels);
        var expected = testCase.GetProperty("expected");
        var chunks = ParseChunks(encoded);
        Assert.Equal(expected.GetProperty("chunk_types").EnumerateArray().Select(value => value.GetString()), chunks.Select(chunk => chunk.Type));
        Assert.Equal(expected.GetProperty("bit_depth").GetByte(), encoded[24]);
        Assert.Equal(expected.GetProperty("colour_type").GetByte(), encoded[25]);
        Assert.Equal(expected.GetProperty("interlace").GetByte(), encoded[28]);

        var idat = chunks.Where(chunk => chunk.Type == "IDAT").SelectMany(chunk => chunk.Data).ToArray();
        using var source = new MemoryStream(idat);
        using var zlib = new ZLibStream(source, CompressionMode.Decompress);
        using var filtered = new MemoryStream();
        zlib.CopyTo(filtered);
        var filteredBytes = filtered.ToArray();
        var stride = pixels.Width * 4;
        var actualFilters = Enumerable.Range(0, pixels.Height).Select(row => filteredBytes[row * (stride + 1)]).ToArray();
        var expectedFilters = expected.GetProperty("filter_types").EnumerateArray().Select(value => value.GetByte()).ToArray();
        Assert.Equal(expectedFilters, actualFilters);

        var decoded = Png.DecodePng(encoded);
        Assert.Equal(pixels.Data, decoded.Data);
        Assert.Equal(source.Length, source.Position);
        Assert.True(filteredBytes.Length == pixels.Height * (stride + 1), id);
    }

    private static void AssertEncodeError(JsonElement testCase, string id)
    {
        var error = Assert.Throws<PngError>(() => FixturePixelsAndEncode(testCase.GetProperty("input")));
        Assert.Equal(testCase.GetProperty("expected").GetProperty("error_id").GetString(), error.Code);
        Assert.True(error.Message.Length <= 40, id);
    }

    private static byte[] FixturePixelsAndEncode(JsonElement input) => Png.EncodePng(FixturePixels(input));

    private static PixelBuffer FixturePixels(JsonElement input)
    {
        var width = input.GetProperty("width").GetDouble();
        var height = input.GetProperty("height").GetDouble();
        if (!double.IsFinite(width) || !double.IsFinite(height) || Math.Truncate(width) != width || Math.Truncate(height) != height ||
            width < 0 || height < 0 || width > int.MaxValue || height > int.MaxValue)
            throw new PngError("invalid-image-dimensions");
        var data = FromHex(input.GetProperty("rgba_hex").GetString()!);
        var expected = checked((long)width * (long)height * 4);
        if (data.LongLength != expected)
            throw new PngError("invalid-pixel-data-length");
        return new PixelBuffer((int)width, (int)height, data);
    }

    private static IReadOnlyList<Chunk> ParseChunks(byte[] encoded)
    {
        var chunks = new List<Chunk>();
        for (var offset = 8; offset < encoded.Length;)
        {
            var length = checked((int)BinaryPrimitives.ReadUInt32BigEndian(encoded.AsSpan(offset, 4)));
            chunks.Add(new Chunk(Encoding.ASCII.GetString(encoded, offset + 4, 4), encoded[(offset + 8)..(offset + 8 + length)]));
            offset = checked(offset + length + 12);
        }
        return chunks.AsReadOnly();
    }

    private static byte[] FromHex(string value) => Convert.FromHexString(value);

    private static string FindFixture()
    {
        for (var directory = new DirectoryInfo(AppContext.BaseDirectory); directory is not null; directory = directory.Parent)
        {
            var candidate = Path.Combine(directory.FullName, "code", "specs", "fixtures", "image-codec-png-v1", "cases.json");
            if (File.Exists(candidate)) return candidate;
        }
        throw new FileNotFoundException("image-codec-png-v1 fixture not found");
    }
}
