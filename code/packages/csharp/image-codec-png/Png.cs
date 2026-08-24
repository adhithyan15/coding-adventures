using System.Buffers.Binary;
using System.Collections.ObjectModel;
using System.Diagnostics.CodeAnalysis;
using System.Text;
using CodingAdventures.PixelContainer;
using CodingAdventures.Zip;
using PixelBuffer = CodingAdventures.PixelContainer.PixelContainer;

namespace CodingAdventures.ImageCodecPng;

/// <summary>A stable, payload-blind IC18 PNG failure.</summary>
public sealed class PngError : Exception
{
    public PngError(string code) : base(code) => Code = code;

    /// <summary>The stable portable failure identifier.</summary>
    public string Code { get; }
}

/// <summary>An <see cref="IImageCodec"/> adapter for the bounded PNG profile.</summary>
public sealed class PngCodec : IImageCodec
{
    private readonly double? _maxPixels;

    public PngCodec(double? maxPixels = null)
    {
        Png.ValidateMaxPixels(maxPixels);
        _maxPixels = maxPixels;
    }

    public string MimeType => "image/png";

    public byte[] Encode(PixelBuffer pixels) => Png.EncodePng(pixels);

    public PixelBuffer Decode(byte[] bytes) => Png.DecodePng(bytes, _maxPixels);
}

/// <summary>
/// Pure in-memory IC18 PNG framing, zlib wrapping, filtering, encoding, and
/// decoding. RFC 1951 and CRC-32 are delegated to the sibling ZIP package.
/// </summary>
public static class Png
{
    public const int MaxDimension = 16_384;
    public const int DefaultMaxPixels = 32 * 1024 * 1024;

    private const uint AdlerMod = 65_521;

    private static readonly byte[] Signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

    private static readonly string[] ErrorCodeValues =
    [
        "invalid-max-pixels",
        "invalid-image-dimensions",
        "invalid-pixel-data-length",
        "file-too-short",
        "invalid-signature",
        "truncated-chunk",
        "invalid-chunk-type",
        "chunk-crc-mismatch",
        "chunk-before-ihdr",
        "duplicate-ihdr",
        "invalid-ihdr-length",
        "invalid-dimensions",
        "dimension-limit",
        "pixel-limit",
        "unsupported-feature",
        "invalid-plte",
        "invalid-trns",
        "nonconsecutive-idat",
        "invalid-iend",
        "trailing-data",
        "unknown-critical-chunk",
        "missing-required-chunk",
        "invalid-zlib-header",
        "preset-dictionary",
        "inflate-failed",
        "inflated-length-mismatch",
        "idat-cavity",
        "adler-mismatch",
        "invalid-filter",
    ];

    /// <summary>The closed IC18 error taxonomy in normative order.</summary>
    public static IReadOnlyList<string> ErrorCodes => new ReadOnlyCollection<string>([.. ErrorCodeValues]);

    /// <summary>Compute the RFC 1950 Adler-32 checksum.</summary>
    public static uint Adler32(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        uint a = 1;
        uint b = 0;
        for (var start = 0; start < data.Length; start += 5552)
        {
            var end = Math.Min(start + 5552, data.Length);
            for (var index = start; index < end; index++)
            {
                a += data[index];
                b += a;
            }
            a %= AdlerMod;
            b %= AdlerMod;
        }
        return (b << 16) | a;
    }

    /// <summary>Encode RGBA8 pixels as a bounded, non-interlaced PNG.</summary>
    public static byte[] EncodePng(PixelBuffer pixels)
    {
        if (pixels is null || pixels.Width <= 0 || pixels.Height <= 0 ||
            pixels.Width > MaxDimension || pixels.Height > MaxDimension)
            Fail("invalid-image-dimensions");

        var pixelCount = checked((long)pixels.Width * pixels.Height);
        if (pixelCount > DefaultMaxPixels)
            Fail("invalid-image-dimensions");
        if (pixels.Data is null || pixels.Data.LongLength != checked(pixelCount * 4))
            Fail("invalid-pixel-data-length");

        using var output = new MemoryStream();
        output.Write(Signature);
        var ihdr = new byte[13];
        BinaryPrimitives.WriteUInt32BigEndian(ihdr, (uint)pixels.Width);
        BinaryPrimitives.WriteUInt32BigEndian(ihdr.AsSpan(4), (uint)pixels.Height);
        ihdr[8] = 8;
        ihdr[9] = 6;
        WriteChunk(output, "IHDR", ihdr);

        var stride = checked(pixels.Width * 4);
        var filtered = new byte[checked(pixels.Height * (stride + 1))];
        var prior = new byte[stride];
        var scratch = new byte[stride];
        var best = new byte[stride];
        for (var rowIndex = 0; rowIndex < pixels.Height; rowIndex++)
        {
            var raw = pixels.Data.AsSpan(rowIndex * stride, stride);
            var destination = rowIndex * (stride + 1);
            filtered[destination] = ChooseFilter(raw, prior, 4, scratch, best);
            best.CopyTo(filtered.AsSpan(destination + 1, stride));
            raw.CopyTo(prior);
        }

        var deflated = RawRfc1951.RawDeflate(filtered);
        var idat = new byte[checked(deflated.Length + 6)];
        idat[0] = 0x78;
        idat[1] = 0x9c;
        deflated.CopyTo(idat, 2);
        BinaryPrimitives.WriteUInt32BigEndian(idat.AsSpan(idat.Length - 4), Adler32(filtered));
        WriteChunk(output, "IDAT", idat);
        WriteChunk(output, "IEND", []);
        return output.ToArray();
    }

    /// <summary>Decode the bounded, non-interlaced 8-bit IC18 PNG profile.</summary>
    public static PixelBuffer DecodePng(byte[] data, double? maxPixels = null)
    {
        var activeLimit = ValidateMaxPixels(maxPixels);
        ArgumentNullException.ThrowIfNull(data);
        if (data.Length < Signature.Length)
            Fail("file-too-short");
        if (!data.AsSpan(0, Signature.Length).SequenceEqual(Signature))
            Fail("invalid-signature");

        var width = 0;
        var height = 0;
        byte bitDepth = 0;
        byte colourType = 0;
        var sawIhdr = false;
        var sawIend = false;
        var sawPlte = false;
        var sawTrns = false;
        var inIdat = false;
        var idatEnded = false;
        byte? transparentGrey = null;
        (byte Red, byte Green, byte Blue)? transparentRgb = null;
        var idatParts = new List<byte[]>();

        for (var position = Signature.Length; position < data.Length;)
        {
            if (data.Length - position < 8)
                Fail("truncated-chunk");
            var length = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(position, 4));
            var chunkEnd = (long)position + 12 + length;
            if (chunkEnd > data.LongLength)
                Fail("truncated-chunk");

            var typeStart = position + 4;
            var dataStart = position + 8;
            var dataEnd = checked((int)((long)dataStart + length));
            var typeBytes = data.AsSpan(typeStart, 4);
            if (!ValidChunkType(typeBytes))
                Fail("invalid-chunk-type");
            var declaredCrc = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(dataEnd, 4));
            if (RawRfc1951.Crc32(data[typeStart..dataEnd]) != declaredCrc)
                Fail("chunk-crc-mismatch");

            var chunkType = Encoding.ASCII.GetString(typeBytes);
            var chunkData = data.AsSpan(dataStart, checked((int)length));
            if (!sawIhdr && chunkType != "IHDR")
                Fail("chunk-before-ihdr");

            switch (chunkType)
            {
                case "IHDR":
                    if (sawIhdr) Fail("duplicate-ihdr");
                    if (length != 13) Fail("invalid-ihdr-length");
                    var widthRaw = BinaryPrimitives.ReadUInt32BigEndian(chunkData);
                    var heightRaw = BinaryPrimitives.ReadUInt32BigEndian(chunkData[4..]);
                    bitDepth = chunkData[8];
                    colourType = chunkData[9];
                    if (widthRaw == 0 || heightRaw == 0) Fail("invalid-dimensions");
                    if (widthRaw > MaxDimension || heightRaw > MaxDimension) Fail("dimension-limit");
                    width = (int)widthRaw;
                    height = (int)heightRaw;
                    if (checked((long)width * height) > activeLimit) Fail("pixel-limit");
                    if (chunkData[10] != 0 || chunkData[11] != 0 || chunkData[12] != 0 ||
                        bitDepth != 8 || colourType is not (0 or 2 or 4 or 6))
                        Fail("unsupported-feature");
                    sawIhdr = true;
                    break;

                case "PLTE":
                    if (sawPlte || idatParts.Count > 0 || sawTrns || colourType is not (2 or 6) ||
                        length is < 3 or > 768 || length % 3 != 0)
                        Fail("invalid-plte");
                    sawPlte = true;
                    break;

                case "tRNS":
                    if (sawTrns || idatParts.Count > 0) Fail("invalid-trns");
                    if (colourType == 0)
                    {
                        if (length != 2) Fail("invalid-trns");
                        var value = BinaryPrimitives.ReadUInt16BigEndian(chunkData);
                        if (value > byte.MaxValue) Fail("invalid-trns");
                        transparentGrey = (byte)value;
                    }
                    else if (colourType == 2)
                    {
                        if (length != 6) Fail("invalid-trns");
                        var red = BinaryPrimitives.ReadUInt16BigEndian(chunkData);
                        var green = BinaryPrimitives.ReadUInt16BigEndian(chunkData[2..]);
                        var blue = BinaryPrimitives.ReadUInt16BigEndian(chunkData[4..]);
                        if (red > byte.MaxValue || green > byte.MaxValue || blue > byte.MaxValue)
                            Fail("invalid-trns");
                        transparentRgb = ((byte)red, (byte)green, (byte)blue);
                    }
                    else
                    {
                        Fail("invalid-trns");
                    }
                    sawTrns = true;
                    break;

                case "IDAT":
                    if (idatEnded) Fail("nonconsecutive-idat");
                    idatParts.Add(chunkData.ToArray());
                    inIdat = true;
                    break;

                case "IEND":
                    if (length != 0) Fail("invalid-iend");
                    if (chunkEnd != data.LongLength) Fail("trailing-data");
                    sawIend = true;
                    position = checked((int)chunkEnd);
                    continue;

                case "acTL":
                case "fcTL":
                case "fdAT":
                    Fail("unsupported-feature");
                    break;

                default:
                    if ((typeBytes[0] & 0x20) == 0) Fail("unknown-critical-chunk");
                    break;
            }

            if (chunkType != "IDAT" && inIdat)
            {
                inIdat = false;
                idatEnded = true;
            }
            position = checked((int)chunkEnd);
        }

        if (!sawIhdr || !sawIend || idatParts.Count == 0)
            Fail("missing-required-chunk");

        long zlibLength = 0;
        foreach (var part in idatParts)
            zlibLength = checked(zlibLength + part.LongLength);
        if (zlibLength > data.LongLength || zlibLength > int.MaxValue)
            Fail("truncated-chunk");
        var zlibData = new byte[(int)zlibLength];
        var copied = 0;
        foreach (var part in idatParts)
        {
            part.CopyTo(zlibData, copied);
            copied += part.Length;
        }
        if (zlibData.Length < 6) Fail("invalid-zlib-header");
        var cmf = zlibData[0];
        var flg = zlibData[1];
        if ((cmf & 0x0f) != 8 || (cmf >> 4) > 7 || (((cmf << 8) | flg) % 31) != 0)
            Fail("invalid-zlib-header");
        if ((flg & 0x20) != 0) Fail("preset-dictionary");

        var channels = colourType switch { 0 => 1, 2 => 3, 4 => 2, _ => 4 };
        var strideLong = checked((long)width * channels);
        var expectedLong = checked((long)height * (strideLong + 1));
        var expected = checked((int)expectedLong);
        var deflateData = zlibData[2..^4];
        RawInflateResult inflated;
        try
        {
            inflated = RawRfc1951.RawInflateCounted(deflateData, expected);
        }
        catch (RawInflateError error)
        {
            Fail(error.Code == "output-limit-exceeded" ? "inflated-length-mismatch" : "inflate-failed");
            throw;
        }
        if (inflated.Output.Length != expected) Fail("inflated-length-mismatch");
        if (inflated.BytesConsumed != deflateData.Length) Fail("idat-cavity");
        if (Adler32(inflated.Output) != BinaryPrimitives.ReadUInt32BigEndian(zlibData.AsSpan(zlibData.Length - 4)))
            Fail("adler-mismatch");

        var stride = (int)strideLong;
        var rowSize = stride + 1;
        for (var rowIndex = 0; rowIndex < height; rowIndex++)
            if (inflated.Output[rowIndex * rowSize] > 4) Fail("invalid-filter");

        var container = new PixelBuffer(width, height);
        var prior = new byte[stride];
        for (var rowIndex = 0; rowIndex < height; rowIndex++)
        {
            var at = rowIndex * rowSize;
            var filter = inflated.Output[at];
            var row = inflated.Output.AsSpan(at + 1, stride);
            UndoFilter(filter, row, prior, channels);
            var destinationRow = rowIndex * width * 4;
            for (var x = 0; x < width; x++)
            {
                var source = x * channels;
                var destination = destinationRow + x * 4;
                switch (channels)
                {
                    case 1:
                        var grey = row[source];
                        container.Data[destination] = grey;
                        container.Data[destination + 1] = grey;
                        container.Data[destination + 2] = grey;
                        container.Data[destination + 3] = transparentGrey == grey ? (byte)0 : (byte)255;
                        break;
                    case 2:
                        var greyAlpha = row[source];
                        container.Data[destination] = greyAlpha;
                        container.Data[destination + 1] = greyAlpha;
                        container.Data[destination + 2] = greyAlpha;
                        container.Data[destination + 3] = row[source + 1];
                        break;
                    case 3:
                        var red = row[source];
                        var green = row[source + 1];
                        var blue = row[source + 2];
                        container.Data[destination] = red;
                        container.Data[destination + 1] = green;
                        container.Data[destination + 2] = blue;
                        container.Data[destination + 3] = transparentRgb is { } key && key == (red, green, blue) ? (byte)0 : (byte)255;
                        break;
                    default:
                        row.Slice(source, 4).CopyTo(container.Data.AsSpan(destination, 4));
                        break;
                }
            }
            row.CopyTo(prior);
        }
        return container;
    }

    internal static int ValidateMaxPixels(double? value)
    {
        if (value is null) return DefaultMaxPixels;
        if (!double.IsFinite(value.Value) || Math.Truncate(value.Value) != value.Value ||
            value.Value <= 0 || value.Value > DefaultMaxPixels)
            Fail("invalid-max-pixels");
        return checked((int)value.Value);
    }

    private static bool ValidChunkType(ReadOnlySpan<byte> chunkType)
    {
        if (chunkType.Length != 4 || (chunkType[2] & 0x20) != 0) return false;
        foreach (var value in chunkType)
            if (!((value >= (byte)'A' && value <= (byte)'Z') || (value >= (byte)'a' && value <= (byte)'z')))
                return false;
        return true;
    }

    private static void WriteChunk(Stream output, string type, byte[] data)
    {
        Span<byte> length = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32BigEndian(length, checked((uint)data.Length));
        output.Write(length);
        var typeBytes = Encoding.ASCII.GetBytes(type);
        output.Write(typeBytes);
        output.Write(data);
        var crc = RawRfc1951.Crc32(typeBytes);
        crc = RawRfc1951.Crc32(data, crc);
        Span<byte> checksum = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32BigEndian(checksum, crc);
        output.Write(checksum);
    }

    private static byte ChooseFilter(
        ReadOnlySpan<byte> raw,
        ReadOnlySpan<byte> prior,
        int bytesPerPixel,
        Span<byte> scratch,
        Span<byte> best)
    {
        byte bestFilter = 0;
        var bestScore = int.MaxValue;
        for (byte filter = 0; filter <= 4; filter++)
        {
            ApplyFilter(filter, raw, prior, bytesPerPixel, scratch);
            var score = 0;
            foreach (var value in scratch)
                score += value < 128 ? value : 256 - value;
            if (score < bestScore)
            {
                bestScore = score;
                bestFilter = filter;
                scratch.CopyTo(best);
            }
        }
        return bestFilter;
    }

    private static void ApplyFilter(
        byte filter,
        ReadOnlySpan<byte> raw,
        ReadOnlySpan<byte> prior,
        int bytesPerPixel,
        Span<byte> output)
    {
        for (var index = 0; index < raw.Length; index++)
        {
            byte left = 0;
            byte aboveLeft = 0;
            if (index >= bytesPerPixel)
            {
                left = raw[index - bytesPerPixel];
                aboveLeft = prior[index - bytesPerPixel];
            }
            var predicted = filter switch
            {
                1 => left,
                2 => prior[index],
                3 => (byte)((left + prior[index]) / 2),
                4 => Paeth(left, prior[index], aboveLeft),
                _ => (byte)0,
            };
            output[index] = unchecked((byte)(raw[index] - predicted));
        }
    }

    private static void UndoFilter(byte filter, Span<byte> row, ReadOnlySpan<byte> prior, int bytesPerPixel)
    {
        switch (filter)
        {
            case 0:
                return;
            case 1:
                for (var index = bytesPerPixel; index < row.Length; index++)
                    row[index] = unchecked((byte)(row[index] + row[index - bytesPerPixel]));
                return;
            case 2:
                for (var index = 0; index < row.Length; index++)
                    row[index] = unchecked((byte)(row[index] + prior[index]));
                return;
            case 3:
                for (var index = 0; index < row.Length; index++)
                {
                    var left = index >= bytesPerPixel ? row[index - bytesPerPixel] : (byte)0;
                    row[index] = unchecked((byte)(row[index] + (left + prior[index]) / 2));
                }
                return;
            case 4:
                for (var index = 0; index < row.Length; index++)
                {
                    var left = index >= bytesPerPixel ? row[index - bytesPerPixel] : (byte)0;
                    var aboveLeft = index >= bytesPerPixel ? prior[index - bytesPerPixel] : (byte)0;
                    row[index] = unchecked((byte)(row[index] + Paeth(left, prior[index], aboveLeft)));
                }
                return;
            default:
                Fail("invalid-filter");
                return;
        }
    }

    private static byte Paeth(byte a, byte b, byte c)
    {
        var p = a + b - c;
        var pa = Math.Abs(p - a);
        var pb = Math.Abs(p - b);
        var pc = Math.Abs(p - c);
        if (pa <= pb && pa <= pc) return a;
        return pb <= pc ? b : c;
    }

    [DoesNotReturn]
    private static void Fail(string code) => throw new PngError(code);
}
