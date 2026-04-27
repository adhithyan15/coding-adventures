using System.Buffers.Binary;
using System.Globalization;
using System.Text;
using System.Text.RegularExpressions;
using CsprngAlgorithm = CodingAdventures.Csprng.Csprng;
using Md5Algorithm = CodingAdventures.Md5.Md5;
using Sha1Algorithm = CodingAdventures.Sha1.Sha1;

namespace CodingAdventures.Uuid;

/// <summary>
/// A 128-bit universally unique identifier per RFC 4122 and RFC 9562.
/// </summary>
public readonly record struct Uuid(ulong Msb, ulong Lsb) : IComparable<Uuid>
{
    /// <summary>
    /// Package version.
    /// </summary>
    public const string VERSION = "0.1.0";

    private const ulong GregorianOffset = 122_192_928_000_000_000UL;
    private static readonly Regex UuidPattern = new(
        "^\\s*(?:urn:uuid:)?\\{?"
        + "([0-9a-fA-F]{8})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{12})"
        + "\\}?\\s*$",
        RegexOptions.Compiled | RegexOptions.CultureInvariant | RegexOptions.IgnoreCase);

    private static readonly int ClockSequence = CreateClockSequence();

    /// <summary>
    /// The nil UUID: all 128 bits are zero.
    /// </summary>
    public static readonly Uuid NIL = new(0UL, 0UL);

    /// <summary>
    /// The max UUID: all 128 bits are one.
    /// </summary>
    public static readonly Uuid MAX = new(ulong.MaxValue, ulong.MaxValue);

    /// <summary>
    /// RFC 4122 namespace for fully-qualified domain names.
    /// </summary>
    public static readonly Uuid NAMESPACE_DNS = FromString("6ba7b810-9dad-11d1-80b4-00c04fd430c8");

    /// <summary>
    /// RFC 4122 namespace for URLs.
    /// </summary>
    public static readonly Uuid NAMESPACE_URL = FromString("6ba7b811-9dad-11d1-80b4-00c04fd430c8");

    /// <summary>
    /// RFC 4122 namespace for ISO OIDs.
    /// </summary>
    public static readonly Uuid NAMESPACE_OID = FromString("6ba7b812-9dad-11d1-80b4-00c04fd430c8");

    /// <summary>
    /// RFC 4122 namespace for X.500 distinguished names.
    /// </summary>
    public static readonly Uuid NAMESPACE_X500 = FromString("6ba7b814-9dad-11d1-80b4-00c04fd430c8");

    /// <summary>
    /// Version field stored in the high nibble of byte 6.
    /// </summary>
    public int Version => (int)((Msb >> 12) & 0xFUL);

    /// <summary>
    /// Variant field rendered as ncs, rfc4122, microsoft, or reserved.
    /// </summary>
    public string Variant
    {
        get
        {
            var top = (int)((Lsb >> 62) & 0x3UL);
            if (top <= 1)
            {
                return "ncs";
            }

            if (top == 2)
            {
                return "rfc4122";
            }

            return ((Lsb >> 61) & 0x7UL) == 7UL ? "reserved" : "microsoft";
        }
    }

    /// <summary>
    /// True when all bits are zero.
    /// </summary>
    public bool IsNil => Msb == 0UL && Lsb == 0UL;

    /// <summary>
    /// True when all bits are one.
    /// </summary>
    public bool IsMax => Msb == ulong.MaxValue && Lsb == ulong.MaxValue;

    /// <summary>
    /// Parse a UUID from standard, compact, braced, or URN text.
    /// </summary>
    public static Uuid FromString(string? text)
    {
        if (text is null)
        {
            throw new UuidException("UUID string must not be null");
        }

        var match = UuidPattern.Match(text.Trim());
        if (!match.Success)
        {
            throw new UuidException($"Invalid UUID string: '{text}'");
        }

        var hex = string.Concat(
            match.Groups[1].Value,
            match.Groups[2].Value,
            match.Groups[3].Value,
            match.Groups[4].Value,
            match.Groups[5].Value);
        var msb = ulong.Parse(hex[..16], NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture);
        var lsb = ulong.Parse(hex[16..], NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture);
        return new Uuid(msb, lsb);
    }

    /// <summary>
    /// Construct a UUID from exactly 16 bytes in network byte order.
    /// </summary>
    public static Uuid FromBytes(byte[]? bytes)
    {
        if (bytes is null)
        {
            throw new UuidException("UUID bytes must be exactly 16, got null");
        }

        return FromBytes((ReadOnlySpan<byte>)bytes);
    }

    /// <summary>
    /// Construct a UUID from exactly 16 bytes in network byte order.
    /// </summary>
    public static Uuid FromBytes(ReadOnlySpan<byte> bytes)
    {
        if (bytes.Length != 16)
        {
            throw new UuidException($"UUID bytes must be exactly 16, got {bytes.Length}");
        }

        return new Uuid(
            BinaryPrimitives.ReadUInt64BigEndian(bytes[..8]),
            BinaryPrimitives.ReadUInt64BigEndian(bytes[8..]));
    }

    /// <summary>
    /// Return true when the text is a UUID in one of the supported forms.
    /// </summary>
    public static bool IsValid(string? text) => text is not null && UuidPattern.IsMatch(text.Trim());

    /// <summary>
    /// Return the 16 bytes of this UUID in network byte order.
    /// </summary>
    public byte[] ToBytes()
    {
        var bytes = new byte[16];
        BinaryPrimitives.WriteUInt64BigEndian(bytes.AsSpan(0, 8), Msb);
        BinaryPrimitives.WriteUInt64BigEndian(bytes.AsSpan(8, 8), Lsb);
        return bytes;
    }

    /// <summary>
    /// Return the standard lowercase 8-4-4-4-12 UUID string.
    /// </summary>
    public override string ToString()
    {
        var hex = string.Create(CultureInfo.InvariantCulture, $"{Msb:x16}{Lsb:x16}");
        return $"{hex[..8]}-{hex[8..12]}-{hex[12..16]}-{hex[16..20]}-{hex[20..]}";
    }

    /// <summary>
    /// Compare UUIDs by unsigned byte order.
    /// </summary>
    public int CompareTo(Uuid other)
    {
        var compareMsb = Msb.CompareTo(other.Msb);
        return compareMsb != 0 ? compareMsb : Lsb.CompareTo(other.Lsb);
    }

    /// <summary>
    /// Generate a random UUID v4.
    /// </summary>
    public static Uuid V4()
    {
        var bytes = CsprngAlgorithm.RandomBytes(16);
        return StampVersionVariant(bytes, 4);
    }

    /// <summary>
    /// Generate a time-ordered random UUID v7.
    /// </summary>
    public static Uuid V7()
    {
        var timestampMs = (ulong)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        var random = CsprngAlgorithm.RandomBytes(10);
        var raw = new byte[16];
        raw[0] = (byte)((timestampMs >> 40) & 0xFFUL);
        raw[1] = (byte)((timestampMs >> 32) & 0xFFUL);
        raw[2] = (byte)((timestampMs >> 24) & 0xFFUL);
        raw[3] = (byte)((timestampMs >> 16) & 0xFFUL);
        raw[4] = (byte)((timestampMs >> 8) & 0xFFUL);
        raw[5] = (byte)(timestampMs & 0xFFUL);
        raw[6] = (byte)(0x70 | (random[0] & 0x0F));
        raw[7] = random[1];
        raw[8] = (byte)(0x80 | (random[2] & 0x3F));
        random.AsSpan(3, 7).CopyTo(raw.AsSpan(9));
        return FromBytes(raw);
    }

    /// <summary>
    /// Generate a time-based UUID v1 with a random node id.
    /// </summary>
    public static Uuid V1()
    {
        var timestamp = ((ulong)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() * 10_000UL) + GregorianOffset;
        var timeLow = timestamp & 0xFFFFFFFFUL;
        var timeMid = (timestamp >> 32) & 0xFFFFUL;
        var timeHi = (timestamp >> 48) & 0x0FFFUL;
        var msb = (timeLow << 32) | (timeMid << 16) | (0x1000UL | timeHi);

        var clockSeqHi = 0x80UL | ((uint)ClockSequence >> 8);
        var clockSeqLow = (uint)ClockSequence & 0xFFU;
        var nodeBytes = CsprngAlgorithm.RandomBytes(6);
        nodeBytes[0] = (byte)(nodeBytes[0] | 0x01);

        var node = 0UL;
        foreach (var value in nodeBytes)
        {
            node = (node << 8) | value;
        }

        var lsb = (clockSeqHi << 56) | ((ulong)clockSeqLow << 48) | node;
        return new Uuid(msb, lsb);
    }

    /// <summary>
    /// Generate a name-based UUID v5 using SHA-1.
    /// </summary>
    public static Uuid V5(Uuid namespaceId, string name)
    {
        ArgumentNullException.ThrowIfNull(name);
        var digest = Sha1Algorithm.Hash(Concat(namespaceId.ToBytes(), Encoding.UTF8.GetBytes(name)));
        var raw = new byte[16];
        digest.AsSpan(0, 16).CopyTo(raw);
        return StampVersionVariant(raw, 5);
    }

    /// <summary>
    /// Generate a name-based UUID v3 using MD5.
    /// </summary>
    public static Uuid V3(Uuid namespaceId, string name)
    {
        ArgumentNullException.ThrowIfNull(name);
        return StampVersionVariant(Md5Algorithm.SumMd5(Concat(namespaceId.ToBytes(), Encoding.UTF8.GetBytes(name))), 3);
    }

    private static Uuid StampVersionVariant(byte[] bytes, int version)
    {
        bytes[6] = (byte)((bytes[6] & 0x0F) | (version << 4));
        bytes[8] = (byte)((bytes[8] & 0x3F) | 0x80);
        return FromBytes(bytes);
    }

    private static byte[] Concat(byte[] left, byte[] right)
    {
        var result = new byte[left.Length + right.Length];
        Buffer.BlockCopy(left, 0, result, 0, left.Length);
        Buffer.BlockCopy(right, 0, result, left.Length, right.Length);
        return result;
    }

    private static int CreateClockSequence()
    {
        var bytes = CsprngAlgorithm.RandomBytes(2);
        return (((bytes[0] << 8) | bytes[1]) & 0x3FFF);
    }
}
