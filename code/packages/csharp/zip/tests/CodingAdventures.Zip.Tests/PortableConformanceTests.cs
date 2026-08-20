using System.Diagnostics;
using System.Text;
using System.Text.Json;

namespace CodingAdventures.Zip.Tests;

public sealed class PortableConformanceTests
{
    private static readonly string[] ExpectedErrorCodes =
    [
        "invalid-output-limit",
        "unexpected-eof",
        "reserved-block-type",
        "stored-length-mismatch",
        "huffman-oversubscribed",
        "incomplete-code-length-tree",
        "incomplete-literal-length-tree",
        "incomplete-distance-tree",
        "repeat-without-previous",
        "repeat-overrun",
        "invalid-literal-length-symbol",
        "reserved-distance-symbol",
        "invalid-back-reference",
        "output-limit-exceeded",
    ];

    [Fact]
    public void ClosedPortableCorpusPasses()
    {
        using var fixture = JsonDocument.Parse(File.ReadAllText(FindFixture()));
        var root = fixture.RootElement;
        Assert.Equal(1, root.GetProperty("schema_version").GetInt32());
        Assert.Equal("zip-owned-raw-rfc1951-v1", root.GetProperty("profile").GetString());
        Assert.Equal(RawRfc1951.MaxOutput, root.GetProperty("limits").GetProperty("default_max_output").GetInt32());
        Assert.Equal(RawRfc1951.MaxOutput, root.GetProperty("limits").GetProperty("hard_max_output").GetInt32());
        Assert.Equal(ExpectedErrorCodes, RawRfc1951.ErrorCodes);
        Assert.Equal(ExpectedErrorCodes, root.GetProperty("error_ids").EnumerateArray().Select(value => value.GetString()));

        var cases = root.GetProperty("cases").EnumerateArray().ToArray();
        Assert.Equal(34, cases.Length);
        foreach (var testCase in cases)
        {
            var id = testCase.GetProperty("id").GetString()!;
            var operation = testCase.GetProperty("operation").GetString()!;
            var limit = testCase.TryGetProperty("max_output", out var maxOutput)
                ? maxOutput.GetInt32()
                : RawRfc1951.MaxOutput;

            switch (operation)
            {
                case "inflate":
                    {
                        var input = Convert.FromHexString(testCase.GetProperty("input_hex").GetString()!);
                        var expected = Materialize(testCase.GetProperty("expected").GetProperty("output"));
                        var result = RawRfc1951.RawInflateCounted(input, limit);
                        Assert.True(result.Output.SequenceEqual(expected), id);
                        Assert.Equal(testCase.GetProperty("expected").GetProperty("bytes_consumed").GetInt32(), result.BytesConsumed);
                        Assert.True(RawRfc1951.RawInflate(input, limit).SequenceEqual(expected), id);
                        break;
                    }
                case "inflate-error":
                    {
                        var input = Convert.FromHexString(testCase.GetProperty("input_hex").GetString()!);
                        var expected = testCase.GetProperty("expected").GetProperty("error_id").GetString();
                        var error = Assert.Throws<RawInflateError>(() => RawRfc1951.RawInflateCounted(input, limit));
                        Assert.Equal(expected, error.Code);
                        Assert.Equal(expected, error.Message);
                        Assert.Empty(error.Data);
                        break;
                    }
                case "deflate-interoperability":
                    {
                        var input = Convert.FromHexString(testCase.GetProperty("input_hex").GetString()!);
                        var expected = Materialize(testCase.GetProperty("expected").GetProperty("output"));
                        Assert.True(PythonCodec("decompress", RawRfc1951.RawDeflate(input)).SequenceEqual(expected), id);
                        break;
                    }
                case "crc32":
                    {
                        uint checksum = testCase.TryGetProperty("initial_crc32_hex", out var initial)
                            ? Convert.ToUInt32(initial.GetString(), 16)
                            : 0;
                        foreach (var chunk in testCase.GetProperty("chunks_hex").EnumerateArray())
                            checksum = RawRfc1951.Crc32(Convert.FromHexString(chunk.GetString()!), checksum);
                        Assert.Equal(testCase.GetProperty("expected").GetProperty("crc32_hex").GetString(), checksum.ToString("x8"));
                        break;
                    }
                default:
                    throw new Xunit.Sdk.XunitException($"{id}: unknown operation {operation}");
            }
        }
    }

    [Fact]
    public void ForeignFullWindowStreamAndRawCodecPass()
    {
        var prefix = Enumerable.Range(0, 32_768)
            .Select(index => (byte)((index * 73 + index / 251) & 0xff))
            .ToArray();
        var expected = prefix.Concat(prefix).ToArray();
        var foreign = PythonCodec("compress", expected);
        Assert.Equal(expected, RawRfc1951.RawInflate(foreign, expected.Length));

        var historical = Encoding.UTF8.GetBytes("historical wrapper compatibility");
        Assert.Equal(historical, RawRfc1951.RawInflate(RawRfc1951.RawDeflate(historical)));
    }

    [Fact]
    public void ZipReaderRequiresExactCompressedAndUncompressedSizes()
    {
        var compressed = Convert.FromHexString("0dc28911c0200c03b0d8f97028ec3f6ed129cab7dd96a0c2445bdb93809663a5d303f6b265e20c2b79ea03379d227e");
        var plain = Convert.FromHexString("0406030b000e070909010906010a04070007000000000501010908030108050302030401000401000207090009020a0a020605020d060c01020b020302090201");
        Assert.Equal(plain, new ZipReader(RawZip("dynamic.bin", compressed, plain, plain.Length)).ReadByName("dynamic.bin"));

        var cavity = compressed.Concat(new byte[] { 0xde, 0xad }).ToArray();
        var suffixError = Assert.Throws<InvalidDataException>(() =>
            new ZipReader(RawZip("cavity.bin", cavity, plain, plain.Length)).ReadByName("cavity.bin"));
        Assert.Equal("zip: compressed payload contains trailing bytes", suffixError.Message);

        var sizeError = Assert.Throws<InvalidDataException>(() =>
            new ZipReader(RawZip("size.bin", compressed, plain, plain.Length + 1)).ReadByName("size.bin"));
        Assert.Equal("zip: uncompressed size does not match the directory", sizeError.Message);

        var storedSizeError = Assert.Throws<InvalidDataException>(() =>
            new ZipReader(RawZip("stored.bin", plain, plain, plain.Length + 1, method: 0)).ReadByName("stored.bin"));
        Assert.Equal("zip: stored entry sizes do not match", storedSizeError.Message);

        var malformedError = Assert.Throws<InvalidDataException>(() =>
            new ZipReader(RawZip("malformed.bin", new byte[] { 0x07 }, Array.Empty<byte>(), 0)).ReadByName("malformed.bin"));
        Assert.Equal("zip: raw inflate failed: reserved-block-type", malformedError.Message);
    }

    private static byte[] Materialize(JsonElement output)
    {
        if (output.TryGetProperty("hex", out var hex))
            return Convert.FromHexString(hex.GetString()!);
        var unit = Convert.FromHexString(output.GetProperty("repeat_hex").GetString()!);
        return Enumerable.Repeat(unit[0], output.GetProperty("count").GetInt32()).ToArray();
    }

    private static string FindFixture()
    {
        for (var directory = new DirectoryInfo(AppContext.BaseDirectory); directory is not null; directory = directory.Parent)
        {
            var candidate = Path.Combine(directory.FullName, "code", "specs", "fixtures", "zip-raw-rfc1951-v1", "cases.json");
            if (File.Exists(candidate)) return candidate;
        }
        throw new FileNotFoundException("zip-raw-rfc1951-v1 fixture not found");
    }

    private static byte[] PythonCodec(string mode, byte[] input)
    {
        var start = new ProcessStartInfo
        {
            FileName = OperatingSystem.IsWindows() ? "python" : "python3",
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true,
        };
        start.ArgumentList.Add("-c");
        start.ArgumentList.Add("import sys,zlib;m=sys.argv[1];d=sys.stdin.buffer.read();c=zlib.compressobj(9,zlib.DEFLATED,-15);r=(c.compress(d)+c.flush()) if m=='compress' else zlib.decompress(d,-15);sys.stdout.buffer.write(r)");
        start.ArgumentList.Add(mode);
        using var process = Process.Start(start) ?? throw new InvalidOperationException("python oracle did not start");
        process.StandardInput.BaseStream.Write(input);
        process.StandardInput.Close();
        using var output = new MemoryStream();
        process.StandardOutput.BaseStream.CopyTo(output);
        var diagnostic = process.StandardError.ReadToEnd();
        process.WaitForExit();
        if (process.ExitCode != 0) throw new InvalidOperationException($"python oracle failed: {diagnostic}");
        return output.ToArray();
    }

    private static byte[] RawZip(string name, byte[] compressed, byte[] plain, int declaredSize, ushort method = 8)
    {
        using var archive = new MemoryStream();
        using var writer = new BinaryWriter(archive, Encoding.UTF8, leaveOpen: true);
        var nameBytes = Encoding.UTF8.GetBytes(name);
        var checksum = RawRfc1951.Crc32(plain);
        writer.Write(0x04034b50u); writer.Write((ushort)20); writer.Write((ushort)0x0800); writer.Write(method);
        writer.Write((ushort)0); writer.Write((ushort)0); writer.Write(checksum); writer.Write((uint)compressed.Length);
        writer.Write((uint)declaredSize); writer.Write((ushort)nameBytes.Length); writer.Write((ushort)0); writer.Write(nameBytes); writer.Write(compressed);
        var centralOffset = (uint)archive.Length;
        writer.Write(0x02014b50u); writer.Write((ushort)0x031e); writer.Write((ushort)20); writer.Write((ushort)0x0800); writer.Write(method);
        writer.Write((ushort)0); writer.Write((ushort)0); writer.Write(checksum); writer.Write((uint)compressed.Length); writer.Write((uint)declaredSize);
        writer.Write((ushort)nameBytes.Length); writer.Write((ushort)0); writer.Write((ushort)0); writer.Write((ushort)0); writer.Write((ushort)0);
        writer.Write(0u); writer.Write(0u); writer.Write(nameBytes);
        var centralSize = (uint)archive.Length - centralOffset;
        writer.Write(0x06054b50u); writer.Write((ushort)0); writer.Write((ushort)0); writer.Write((ushort)1); writer.Write((ushort)1);
        writer.Write(centralSize); writer.Write(centralOffset); writer.Write((ushort)0);
        return archive.ToArray();
    }
}
