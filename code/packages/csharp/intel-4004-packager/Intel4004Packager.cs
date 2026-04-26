using System.Globalization;
using System.Text;

namespace CodingAdventures.Intel4004Packager;

public static class Intel4004Packager
{
    public const string Version = "0.1.0";
    private const int BytesPerRecord = 16;
    private const byte RecordTypeData = 0x00;
    private const byte RecordTypeEof = 0x01;
    private const int MaxImageSize = 0x1000;

    public static string EncodeHex(ReadOnlySpan<byte> binary, int origin = 0)
    {
        if (binary.IsEmpty)
        {
            throw new ArgumentException("binary must be non-empty", nameof(binary));
        }

        if (origin < 0 || origin > 0xFFFF)
        {
            throw new ArgumentOutOfRangeException(nameof(origin), "origin must be 0-65535");
        }

        if (origin + binary.Length > 0x10000)
        {
            throw new ArgumentException("image overflows 16-bit address space", nameof(binary));
        }

        var builder = new StringBuilder();
        for (var offset = 0; offset < binary.Length; offset += BytesPerRecord)
        {
            var count = Math.Min(BytesPerRecord, binary.Length - offset);
            builder.Append(DataRecord(origin + offset, binary.Slice(offset, count)));
        }

        builder.Append(":00000001FF\n");
        return builder.ToString();
    }

    public static DecodedHex DecodeHex(string text)
    {
        ArgumentNullException.ThrowIfNull(text);

        var segments = new SortedDictionary<int, byte[]>();
        var normalized = text.Replace("\r\n", "\n", StringComparison.Ordinal);
        var lines = normalized.Split('\n');

        for (var index = 0; index < lines.Length; index++)
        {
            var line = lines[index].Trim();
            if (line.Length == 0)
            {
                continue;
            }

            var lineNumber = index + 1;
            if (!line.StartsWith(':'))
            {
                throw new FormatException($"line {lineNumber}: expected ':'");
            }

            var record = DecodeHexBytes(line[1..], lineNumber);
            if (record.Length < 5)
            {
                throw new FormatException($"line {lineNumber}: record too short");
            }

            var byteCount = record[0];
            var address = (record[1] << 8) | record[2];
            var recordType = record[3];
            var expectedLength = 4 + byteCount + 1;
            if (record.Length < expectedLength)
            {
                throw new FormatException($"line {lineNumber}: truncated record");
            }

            var computedChecksum = Checksum(record.AsSpan(0, 4 + byteCount));
            var storedChecksum = record[4 + byteCount];
            if (computedChecksum != storedChecksum)
            {
                throw new FormatException($"line {lineNumber}: checksum mismatch");
            }

            if (recordType == RecordTypeEof)
            {
                break;
            }

            if (recordType != RecordTypeData)
            {
                throw new FormatException($"line {lineNumber}: unsupported record type");
            }

            segments[address] = record.AsSpan(4, byteCount).ToArray();
        }

        if (segments.Count == 0)
        {
            return new DecodedHex(0, []);
        }

        var origin = segments.Keys.First();
        var end = segments.Max(pair => pair.Key + pair.Value.Length);
        if (end - origin > MaxImageSize)
        {
            throw new FormatException("decoded image too large");
        }

        var binary = new byte[end - origin];
        foreach (var (address, data) in segments)
        {
            data.CopyTo(binary.AsSpan(address - origin));
        }

        return new DecodedHex(origin, binary);
    }

    private static string DataRecord(int address, ReadOnlySpan<byte> chunk)
    {
        var fields = new byte[4 + chunk.Length];
        fields[0] = (byte)chunk.Length;
        fields[1] = (byte)((address >> 8) & 0xFF);
        fields[2] = (byte)(address & 0xFF);
        fields[3] = RecordTypeData;
        chunk.CopyTo(fields.AsSpan(4));

        return $":{chunk.Length:X2}{address:X4}00{Convert.ToHexString(chunk)}{Checksum(fields):X2}\n";
    }

    private static byte Checksum(ReadOnlySpan<byte> fields)
    {
        var total = 0;
        foreach (var field in fields)
        {
            total += field;
        }

        return (byte)((0x100 - (total % 0x100)) % 0x100);
    }

    private static byte[] DecodeHexBytes(string hex, int lineNumber)
    {
        try
        {
            return Convert.FromHexString(hex);
        }
        catch (FormatException ex)
        {
            throw new FormatException($"line {lineNumber}: invalid hex", ex);
        }
    }
}

public sealed record DecodedHex(int Origin, byte[] Binary);
