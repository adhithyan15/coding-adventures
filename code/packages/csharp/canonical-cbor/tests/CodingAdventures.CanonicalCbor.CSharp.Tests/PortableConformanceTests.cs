using System.Text.Json;

using CodingAdventures.CanonicalCbor.CSharp;

namespace CodingAdventures.CanonicalCbor.CSharp.Tests;

/// <summary>
/// Executes the complete language-neutral CBR01 corpus. The test deliberately
/// rebuilds generated values instead of copying another implementation's
/// output, so the shared JSON remains the only byte oracle.
/// </summary>
public sealed class PortableConformanceTests
{
    [Fact]
    public void ExactPortableBytesMatchSharedOracle()
    {
        using JsonDocument document = JsonDocument.Parse(File.ReadAllText(FindFixture()));
        JsonElement root = document.RootElement;
        Assert.Equal(1, root.GetProperty("schema_version").GetInt32());
        Assert.Equal("rfc8949-section-4.2.3-length-first", root.GetProperty("profile").GetString());
        Assert.Equal(CanonicalCbor.MaxNestingDepth,
            root.GetProperty("limits").GetProperty("max_nesting_depth").GetInt32());
        Assert.Equal(CanonicalCbor.MaxEncodedBytes,
            root.GetProperty("limits").GetProperty("max_encoded_bytes").GetInt32());
        Assert.Equal(55, root.GetProperty("cases").GetArrayLength());
        string[] expectedErrors =
        [
            "unexpected-eof", "trailing-bytes", "reserved", "indefinite",
            "non-minimal-integer", "invalid-utf8", "non-canonical-map-order",
            "unsupported-simple", "float-not-supported", "too-deep",
            "length-too-large", "duplicate-map-key", "encode-too-deep", "encode-too-large",
        ];
        Assert.Equal(expectedErrors,
            root.GetProperty("error_ids").EnumerateArray().Select(item => item.GetString()).ToArray());

        foreach (JsonElement testCase in root.GetProperty("cases").EnumerateArray())
        {
            string id = RequiredString(testCase, "id");
            string operation = RequiredString(testCase, "operation");
            string input = RequiredString(testCase, "input");
            string expected = RequiredString(testCase, "expected");

            switch (operation)
            {
                case "round-trip":
                    Assert.Equal(FromHex(expected), CanonicalCbor.EncodeChecked(CanonicalCbor.Decode(FromHex(input))));
                    break;
                case "decode-error":
                    byte[] wire = input.StartsWith("nested-array-wire:", StringComparison.Ordinal)
                        ? NestedArrayWire(int.Parse(input[(input.LastIndexOf(':') + 1)..],
                            System.Globalization.CultureInfo.InvariantCulture))
                        : FromHex(input);
                    AssertError(expected, () => CanonicalCbor.Decode(wire), id);
                    break;
                case "encode-map":
                    Assert.Equal(FromHex(expected), CanonicalCbor.EncodeChecked(MapValue(input)));
                    break;
                case "generated-round-trip":
                    Assert.Equal(GeneratedWire(expected), CanonicalCbor.EncodeChecked(GeneratedValue(input)));
                    break;
                case "encode-error":
                    CborValue value = input == "duplicate-map-key"
                        ? MapValue("6161=00;6161=01")
                        : GeneratedValue(input);
                    AssertError(expected, () => CanonicalCbor.EncodeChecked(value), id);
                    using (MemoryStream destination = new())
                    {
                        destination.WriteByte(0xaa);
                        AssertError(expected, () => CanonicalCbor.EncodeIntoChecked(value, destination), id);
                        Assert.Equal(FromHex("aa"), destination.ToArray());
                    }
                    break;
                default:
                    throw new InvalidOperationException($"unknown fixture operation: {operation}");
            }
        }
    }

    [Fact]
    public void UnsignedMaximumUsesAllEightArgumentBytes()
    {
        CborValue value = new CborUnsigned(ulong.MaxValue);
        Assert.Equal(FromHex("1bffffffffffffffff"), CanonicalCbor.EncodeChecked(value));
        Assert.Equal(value, CanonicalCbor.Decode(FromHex("1bffffffffffffffff")));
    }

    [Fact]
    public void PublicValuesDefendMutableBytesAndCheckedAppendIsAtomic()
    {
        byte[] source = [1, 2, 3];
        CborByteString value = new(source);
        source[0] = 9;
        Assert.Equal(new byte[] { 1, 2, 3 }, value.Value);
        byte[] exposed = value.Value;
        exposed[1] = 9;
        Assert.Equal(new byte[] { 1, 2, 3 }, value.Value);
        Assert.Equal(new CborByteString([1, 2, 3]), value);
        Assert.Equal(new CborByteString([1, 2, 3]).GetHashCode(), value.GetHashCode());
        Assert.Equal("CborByteString(Length=3)", value.ToString());

        using MemoryStream destination = new();
        destination.WriteByte(0xaa);
        destination.Position = 0;
        CanonicalCbor.EncodeIntoChecked(new CborUnsigned(24), destination);
        Assert.Equal(FromHex("aa1818"), destination.ToArray());
    }

    [Fact]
    public void TextRejectsUnpairedSurrogatesBeforeUtf8ReplacementCanOccur()
    {
        Assert.Throws<ArgumentException>(() => new CborText("\ud800"));
        Assert.Throws<ArgumentException>(() => new CborText("\udc00"));
        Assert.Equal(FromHex("64f09f9880"), CanonicalCbor.EncodeChecked(new CborText("\ud83d\ude00")));
    }

    [Fact]
    public void OversizedUtf8FailsBeforePublishingAndErrorsStayPayloadBlind()
    {
        CborText value = new(new string('\u0800', 400_000));
        AssertError("encode-too-large", () => CanonicalCbor.EncodeChecked(value), "large-text");
        using MemoryStream destination = new();
        destination.WriteByte(0xaa);
        AssertError("encode-too-large", () => CanonicalCbor.EncodeIntoChecked(value, destination), "append");
        Assert.Equal(FromHex("aa"), destination.ToArray());

        CborException error = Assert.Throws<CborException>(() => CanonicalCbor.Decode(FromHex("63e298")));
        Assert.Equal("length-too-large", error.Id);
        Assert.StartsWith("canonical-cbor:", error.Message, StringComparison.Ordinal);
        Assert.DoesNotContain("e298", error.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void FullUnsignedDomainsAndHostileContainerLengthsAreHandled()
    {
        Assert.Equal(FromHex("3bffffffffffffffff"),
            CanonicalCbor.EncodeChecked(new CborNegative(ulong.MaxValue)));
        Assert.Equal(FromHex("dbffffffffffffffff00"),
            CanonicalCbor.EncodeChecked(new CborTag(ulong.MaxValue, new CborUnsigned(0))));

        foreach (string wire in new[]
        {
            "5bffffffffffffffff", "7bffffffffffffffff",
            "9bffffffffffffffff", "bbffffffffffffffff",
        })
        {
            AssertError("length-too-large", () => CanonicalCbor.Decode(FromHex(wire)), "hostile");
        }
    }

    [Fact]
    public void StrictUtf8AndCanonicalMapOrderingCoverAdversarialEdges()
    {
        Assert.Throws<ArgumentException>(() => new CborText("\ud800x"));
        foreach (string wire in new[] { "6180", "62e298", "64f4908080" })
        {
            AssertError("invalid-utf8", () => CanonicalCbor.Decode(FromHex(wire)), "utf8");
        }

        CborMap map = new(
        [
            new(new CborByteString([0x80]), new CborUnsigned(1)),
            new(new CborByteString([0x7f]), new CborUnsigned(0)),
        ]);
        Assert.Equal(FromHex("a2417f00418001"), CanonicalCbor.EncodeChecked(map));
        AssertError("duplicate-map-key", () => CanonicalCbor.EncodeChecked(new CborMap(
        [
            new(new CborByteString([1]), CborNull.Instance),
            new(new CborByteString([1]), CborNull.Instance),
        ])), "secret-key-payload");
    }

    [Fact]
    public void CollectionsAreValuesAndTheExactEncodingCapIsEnforced()
    {
        List<CborValue> source = [new CborUnsigned(1)];
        CborArray array = new(source);
        source.Add(new CborUnsigned(2));
        Assert.Single(array.Values);
        Assert.Equal(new CborArray([new CborUnsigned(1)]), array);
        Assert.Equal(new CborArray([new CborUnsigned(1)]).GetHashCode(), array.GetHashCode());
        Assert.Throws<ArgumentException>(() => new CborArray([null!]));
        Assert.Throws<ArgumentException>(() => new CborMap([null!]));

        CborMap mapValue = new([new(new CborText("a"), new CborUnsigned(1))]);
        CborMap equalMap = new([new(new CborText("a"), new CborUnsigned(1))]);
        Assert.Equal(equalMap, mapValue);
        Assert.Equal(equalMap.GetHashCode(), mapValue.GetHashCode());

        Assert.Equal(FromHex("6161"), CanonicalCbor.EncodeChecked(new CborText("a")));
        Assert.Equal(FromHex("62c3a9"), CanonicalCbor.EncodeChecked(new CborText("é")));
        foreach (int count in new[] { 24, 256, 65_536 })
        {
            Assert.Equal(count, ((CborArray)CanonicalCbor.Decode(CanonicalCbor.EncodeChecked(
                new CborArray(Enumerable.Repeat<CborValue>(CborNull.Instance, count))))).Values.Count);
        }
        AssertError("encode-too-large", () => CanonicalCbor.EncodeChecked(new CborArray(
            Enumerable.Repeat<CborValue>(CborNull.Instance, CanonicalCbor.MaxEncodedBytes))), "array-bound");

        string exactText = new string('\u0800', 349_523) + "aa";
        byte[] exact = CanonicalCbor.EncodeChecked(new CborText(exactText));
        Assert.Equal(CanonicalCbor.MaxEncodedBytes, exact.Length);
        AssertError("encode-too-large",
            () => CanonicalCbor.EncodeChecked(new CborText(exactText + "a")), "cap");

        byte[] oversizedWire = ByteStringWire(CanonicalCbor.MaxEncodedBytes, 0x5a);
        CborValue decoded = CanonicalCbor.Decode(oversizedWire);
        AssertError("encode-too-large", () => CanonicalCbor.EncodeChecked(decoded), "decode-cap");

        byte[] firstKey = new byte[524_284];
        byte[] secondKey = new byte[524_284];
        secondKey[^1] = 1;
        CborMap retainedKeys = new(
        [
            new(new CborByteString(firstKey), CborNull.Instance),
            new(new CborByteString(secondKey), CborNull.Instance),
        ]);
        AssertError("encode-too-large", () => CanonicalCbor.EncodeChecked(retainedKeys), "retained-keys");
    }

    private static string RequiredString(JsonElement element, string property) =>
        element.GetProperty(property).GetString()
        ?? throw new InvalidDataException($"fixture property {property} is null");

    private static void AssertError(string id, Action action, string caseId)
    {
        CborException error = Assert.Throws<CborException>(action);
        Assert.Equal(id, error.Id);
        Assert.StartsWith("canonical-cbor:", error.Message, StringComparison.Ordinal);
        Assert.DoesNotContain(caseId, error.Message, StringComparison.Ordinal);
    }

    private static CborMap MapValue(string specification)
    {
        List<CborMapEntry> entries = [];
        foreach (string fragment in specification.Split(';'))
        {
            string[] pair = fragment.Split('=', 2);
            entries.Add(new CborMapEntry(
                CanonicalCbor.Decode(FromHex(pair[0])),
                CanonicalCbor.Decode(FromHex(pair[1]))));
        }
        return new CborMap(entries);
    }

    private static CborValue GeneratedValue(string specification)
    {
        if (specification.StartsWith("nested-array:", StringComparison.Ordinal))
        {
            string[] nestedParts = specification.Split(':');
            if (nestedParts.Length != 2)
            {
                throw new InvalidDataException("invalid generated nested-array grammar");
            }
            int depth = ParseNatural(nestedParts[1]);
            CborValue value = CborNull.Instance;
            for (int index = 0; index < depth; index++)
            {
                value = new CborArray([value]);
            }
            return value;
        }

        if (!specification.StartsWith("bytes-repeat:", StringComparison.Ordinal))
        {
            throw new InvalidDataException("unknown generated value grammar");
        }
        string[] parts = specification.Split(':');
        if (parts.Length != 3)
        {
            throw new InvalidDataException("invalid generated byte-string grammar");
        }
        int length = ParseNatural(parts[1]);
        byte repeated = ParseSingleByte(parts[2]);
        return new CborByteString(Enumerable.Repeat(repeated, length).ToArray());
    }

    private static byte[] GeneratedWire(string specification)
    {
        if (specification.StartsWith("wire:nested-array:", StringComparison.Ordinal))
        {
            string[] nestedParts = specification.Split(':');
            if (nestedParts.Length != 3)
            {
                throw new InvalidDataException("invalid generated nested-array wire grammar");
            }
            return NestedArrayWire(ParseNatural(nestedParts[2]));
        }

        if (!specification.StartsWith("wire:bytes-repeat:", StringComparison.Ordinal))
        {
            throw new InvalidDataException("unknown generated wire grammar");
        }
        string[] parts = specification.Split(':');
        if (parts.Length != 4)
        {
            throw new InvalidDataException("invalid generated wire grammar");
        }
        int length = ParseNatural(parts[2]);
        using MemoryStream output = new(length + 9);
        if (length <= 23)
        {
            output.WriteByte((byte)(0x40 | length));
        }
        else if (length <= byte.MaxValue)
        {
            output.WriteByte(0x58);
            output.WriteByte((byte)length);
        }
        else if (length <= ushort.MaxValue)
        {
            output.WriteByte(0x59);
            output.WriteByte((byte)(length >> 8));
            output.WriteByte((byte)length);
        }
        else
        {
            output.WriteByte(0x5a);
            output.WriteByte((byte)(length >> 24));
            output.WriteByte((byte)(length >> 16));
            output.WriteByte((byte)(length >> 8));
            output.WriteByte((byte)length);
        }
        byte repeated = ParseSingleByte(parts[3]);
        for (int index = 0; index < length; index++)
        {
            output.WriteByte(repeated);
        }
        return output.ToArray();
    }

    private static byte[] NestedArrayWire(int depth)
    {
        byte[] wire = new byte[depth + 1];
        Array.Fill(wire, (byte)0x81, 0, depth);
        wire[depth] = 0xf6;
        return wire;
    }

    private static byte[] ByteStringWire(int length, byte repeated)
    {
        byte[] wire = new byte[length + 5];
        wire[0] = 0x5a;
        wire[1] = (byte)(length >> 24);
        wire[2] = (byte)(length >> 16);
        wire[3] = (byte)(length >> 8);
        wire[4] = (byte)length;
        Array.Fill(wire, repeated, 5, length);
        return wire;
    }

    private static byte[] FromHex(string value) => Convert.FromHexString(value);

    private static int ParseNatural(string value)
    {
        if (!int.TryParse(value, System.Globalization.NumberStyles.None,
            System.Globalization.CultureInfo.InvariantCulture, out int result))
        {
            throw new InvalidDataException("generated length is not a bounded decimal natural");
        }
        return result;
    }

    private static byte ParseSingleByte(string value)
    {
        byte[] decoded = FromHex(value);
        if (decoded.Length != 1)
        {
            throw new InvalidDataException("generated byte token must decode to exactly one byte");
        }
        return decoded[0];
    }

    private static string FindFixture()
    {
        DirectoryInfo? directory = new(Directory.GetCurrentDirectory());
        while (directory is not null)
        {
            string candidate = Path.Combine(directory.FullName,
                "code", "specs", "fixtures", "canonical-cbor-v1", "cases.json");
            if (File.Exists(candidate))
            {
                return candidate;
            }
            directory = directory.Parent;
        }
        throw new FileNotFoundException("canonical-cbor-v1 fixture not found");
    }
}
