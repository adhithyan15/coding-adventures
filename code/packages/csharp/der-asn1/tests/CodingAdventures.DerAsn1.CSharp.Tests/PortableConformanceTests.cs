using System.Collections;
using System.Globalization;
using System.Text.Json;
using System.Text.Json.Nodes;
using DerAsn1 = CodingAdventures.DerAsn1.CSharp.DerAsn1;
using TlvApi = CodingAdventures.DerTlv.CSharp.DerTlv;

namespace CodingAdventures.DerAsn1.CSharp.Tests;

public sealed class PortableConformanceTests
{
    private static readonly JsonObject Fixture = ReadFixture("der-asn1-v1");
    private static readonly JsonObject Upstream = ReadFixture("der-tlv-v1");

    public static IEnumerable<object[]> Cases =>
        Fixture["cases"]!.AsArray().Select(testCase => new object[] { testCase!["id"]!.GetValue<string>() });

    [Theory]
    [MemberData(nameof(Cases))]
    public void MatchesPortableFixture(string id)
    {
        JsonObject testCase = Fixture["cases"]!.AsArray()
            .Select(node => node!.AsObject())
            .Single(candidate => candidate["id"]!.GetValue<string>() == id);
        object actual = RunCase(testCase);
        JsonNode normalized = JsonSerializer.SerializeToNode(actual)!;
        Assert.True(JsonNode.DeepEquals(testCase["expected"], normalized),
            $"{id}\nexpected={testCase["expected"]}\nactual={normalized}");
        if (testCase["redacted_input_hex"] is JsonValue hostile)
        {
            Assert.DoesNotContain(hostile.GetValue<string>(), normalized.ToJsonString(), StringComparison.OrdinalIgnoreCase);
        }
    }

    [Fact]
    public void FixtureRosterAndDelegationAreClosed()
    {
        JsonArray cases = Fixture["cases"]!.AsArray();
        Assert.Equal(109, cases.Count);
        Assert.Equal(22, Fixture["error_ids"]!.AsArray().Count);
        string[] references = cases
            .Where(node => node!.AsObject().ContainsKey("der_tlv_case_id"))
            .Select(node => node!["der_tlv_case_id"]!.GetValue<string>())
            .ToArray();
        Assert.Equal(46, references.Length);
        Assert.Equal(46, references.Distinct(StringComparer.Ordinal).Count());
    }

    [Fact]
    public void ValidatedValuesAreUnforgeableAndDefensivelyImmutable()
    {
        Assert.Empty(typeof(DerAsn1.Asn1Element).GetConstructors());
        Assert.Empty(typeof(DerAsn1.Asn1Cursor).GetConstructors());
        Assert.Empty(typeof(DerAsn1.DerInteger).GetConstructors());
        Assert.Empty(typeof(DerAsn1.DerBitString).GetConstructors());
        Assert.Empty(typeof(DerAsn1.ObjectIdentifier).GetConstructors());
        Assert.Empty(typeof(DerAsn1.Error).GetConstructors());

        byte[] input = Convert.FromHexString("060B2A81FFFFFFFFFFFFFFFF7F");
        DerAsn1.Asn1Element element = new DerAsn1.Asn1Decoder().DecodeExact(input);
        DerAsn1.ObjectIdentifier oid = DerAsn1.DecodeObjectIdentifier(element);
        input[^1] = 1;
        Assert.Equal(0x7f, element.Value.Span[^1]);
        byte[] exposed = element.Value.ToArray();
        exposed[^1] = 2;
        Assert.Equal(0x7f, element.Value.Span[^1]);
        Assert.Equal(new ulong[] { 1, 2, ulong.MaxValue }, oid.Arcs);
        ICollection<ulong> arcs = Assert.IsAssignableFrom<ICollection<ulong>>(oid.Arcs);
        Assert.True(arcs.IsReadOnly);
        Assert.Throws<NotSupportedException>(() => arcs.Clear());

        DerAsn1.DerInteger integer = DerAsn1.DecodeInteger(
            new DerAsn1.Asn1Decoder().DecodeExact(Convert.FromHexString("020101")));
        byte[] signed = integer.SignedBytes.ToArray();
        signed[0] = 0xff;
        Assert.False(integer.IsNegative);
        Assert.Equal(1UL, integer.ToUInt64());

        DerAsn1.DerBitString bits = DerAsn1.DecodeBitString(
            new DerAsn1.Asn1Decoder().DecodeExact(Convert.FromHexString("03020180")));
        byte[] bitBytes = bits.Bytes.ToArray();
        bitBytes[0] = 0x81;
        Assert.Equal(new byte[] { 0x80 }, bits.Bytes.ToArray());
    }

    [Fact]
    public void LimitsAreValidatedAndStructurallyCompared()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new DerAsn1.Asn1Limits(TlvApi.Limits.Default, -1, 1, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => new DerAsn1.Asn1Limits(TlvApi.Limits.Default, 1, -1, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => new DerAsn1.Asn1Limits(TlvApi.Limits.Default, 1, 1, -1));

        DerAsn1.Asn1Decoder first = new();
        DerAsn1.Asn1Cursor cursor = first.Sequence(first.DecodeExact(Convert.FromHexString("30020500")));
        DerAsn1.Asn1Decoder second = new(DerAsn1.Asn1Limits.Default);
        Assert.NotNull(cursor.Read(second));
        Assert.Equal(1, second.ElementsRead);
        Assert.Empty(cursor.Remaining.ToArray());
    }

    [Fact]
    public void PublicErrorsRemainPayloadBlind()
    {
        DerAsn1.Asn1Element hostile = new DerAsn1.Asn1Decoder().DecodeExact(Convert.FromHexString("160261FF"));
        DerAsn1.Error error = Assert.Throws<DerAsn1.Error>(() => DerAsn1.DecodeIa5String(hostile));
        Assert.DoesNotContain("61ff", error.ToString(), StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("255", error.ToString(), StringComparison.OrdinalIgnoreCase);
    }

    private static object RunCase(JsonObject testCase)
    {
        if (testCase["der_tlv_case_id"] is JsonValue reference)
        {
            return VerifyUpstream(reference.GetValue<string>());
        }
        DerAsn1.Asn1Limits limits = Limits(testCase);
        DerAsn1.Asn1Decoder decoder = new(limits);
        string operation = testCase["operation"]!.GetValue<string>();
        try
        {
            DerAsn1.Asn1Element root = decoder.DecodeExact(Materialize(testCase["input"]!.AsArray()));
            return operation switch
            {
                "decode-exact" => Map(
                    ("outcome", "value"),
                    ("tag", Tag(root)),
                    ("header_hex", Hex(root.Header)),
                    ("value_hex", Hex(root.Value)),
                    ("encoded_hex", Hex(root.Encoded)),
                    ("depth", root.Depth),
                    ("elements_read", decoder.ElementsRead)),
                "cursor-script" => CursorResult(testCase, decoder, root),
                "sequence" or "set" => ConstructedResult(operation, decoder, root),
                "explicit" => ExplicitResult(testCase, decoder, root),
                _ => PrimitiveResult(operation, root, limits, testCase["tag_number"]?.GetValue<uint>() ?? 0),
            };
        }
        catch (DerAsn1.Error error)
        {
            string scope = operation == "explicit" && error.Kind == DerAsn1.Asn1ErrorKind.Framing
                ? "container-value"
                : "operation-input";
            return Error(error, scope);
        }
    }

    private static object VerifyUpstream(string id)
    {
        JsonObject referenced = Upstream["cases"]!.AsArray()
            .Select(node => node!.AsObject())
            .Single(candidate => candidate["id"]!.GetValue<string>() == id);
        TlvApi.Limits derLimits = DerLimits(Upstream["defaults"]!.AsObject(), referenced["limits"] as JsonObject);
        DerAsn1.Asn1Decoder decoder = new(new DerAsn1.Asn1Limits(derLimits, 32, 16_384, 128));
        JsonObject expected = referenced["expected"]!.AsObject();
        byte[] input = Materialize(referenced["input"]!.AsArray());
        try
        {
            DerAsn1.Asn1Element element = decoder.DecodeExact(input);
            Assert.Equal("element", expected["outcome"]!.GetValue<string>());
            Assert.Equal(expected["tag"]!["class"]!.GetValue<string>(), element.Tag.Class);
            Assert.Equal(expected["tag"]!["constructed"]!.GetValue<bool>(), element.Tag.Constructed);
            Assert.Equal(expected["tag"]!["number"]!.GetValue<uint>(), element.Tag.Number);
            Assert.Equal(expected["header_len"]!.GetValue<int>(), element.Header.Length);
            Assert.Equal(expected["encoded_len"]!.GetValue<int>(), element.Encoded.Length);
            Assert.Equal(input.Length, element.Encoded.Length);
        }
        catch (DerAsn1.Error error)
        {
            Assert.Equal("error", expected["outcome"]!.GetValue<string>());
            Assert.Equal(DerAsn1.Asn1ErrorKind.Framing, error.Kind);
            Assert.Equal(expected["error_id"]!.GetValue<string>(), error.FramingKind);
            Assert.Equal(expected["offset"]!.GetValue<int>(), error.Offset);
        }
        return Map(("outcome", "upstream"));
    }

    private static object ConstructedResult(
        string operation,
        DerAsn1.Asn1Decoder decoder,
        DerAsn1.Asn1Element root)
    {
        DerAsn1.Asn1Cursor cursor = operation == "sequence" ? decoder.Sequence(root) : decoder.Set(root);
        return Map(
            ("outcome", "value"),
            ("elements_read", decoder.ElementsRead),
            ("remaining_offset", root.Value.Length - cursor.Remaining.Length));
    }

    private static object ExplicitResult(
        JsonObject testCase,
        DerAsn1.Asn1Decoder decoder,
        DerAsn1.Asn1Element root)
    {
        DerAsn1.Asn1Element child = decoder.Explicit(root, testCase["tag_number"]!.GetValue<uint>());
        return Map(
            ("outcome", "value"),
            ("tag", Tag(child)),
            ("value_hex", Hex(child.Value)),
            ("depth", child.Depth),
            ("elements_read", decoder.ElementsRead));
    }

    private static object PrimitiveResult(
        string operation,
        DerAsn1.Asn1Element element,
        DerAsn1.Asn1Limits limits,
        uint tagNumber)
    {
        switch (operation)
        {
            case "decode-boolean":
                return Map(("outcome", "value"), ("boolean", DerAsn1.DecodeBoolean(element)), ("elements_read", 1));
            case "decode-integer":
            case "integer-to-u64":
                DerAsn1.DerInteger integer = DerAsn1.DecodeInteger(element);
                Dictionary<string, object?> integerResult = Map(
                    ("outcome", "value"),
                    ("signed_hex", Hex(integer.SignedBytes)),
                    ("negative", integer.IsNegative));
                if (operation == "integer-to-u64")
                {
                    integerResult["u64_decimal"] = integer.ToUInt64().ToString(CultureInfo.InvariantCulture);
                }
                return integerResult;
            case "decode-bit-string":
                DerAsn1.DerBitString bits = DerAsn1.DecodeBitString(element);
                return Map(
                    ("outcome", "value"),
                    ("bytes_hex", Hex(bits.Bytes)),
                    ("unused_bits", bits.UnusedBits),
                    ("bit_length", bits.BitLength));
            case "decode-octet-string":
                return Map(("outcome", "value"), ("bytes_hex", Hex(DerAsn1.DecodeOctetString(element))));
            case "decode-implicit-octet-string":
                return Map(("outcome", "value"),
                    ("bytes_hex", Hex(DerAsn1.DecodeImplicitOctetString(element, tagNumber))));
            case "decode-ia5-string":
                return Map(("outcome", "value"), ("text", DerAsn1.DecodeIa5String(element)));
            case "decode-implicit-ia5-string":
                return Map(("outcome", "value"), ("text", DerAsn1.DecodeImplicitIa5String(element, tagNumber)));
            case "decode-null":
                DerAsn1.DecodeNull(element);
                return Map(("outcome", "value"));
            case "decode-object-identifier":
            case "decode-implicit-object-identifier":
                DerAsn1.ObjectIdentifier oid = operation == "decode-object-identifier"
                    ? DerAsn1.DecodeObjectIdentifier(element, limits)
                    : DerAsn1.DecodeImplicitObjectIdentifier(element, tagNumber, limits);
                return Map(
                    ("outcome", "value"),
                    ("bytes_hex", Hex(oid.Encoded)),
                    ("arcs_decimal", oid.Arcs.Select(arc => arc.ToString(CultureInfo.InvariantCulture)).ToArray()),
                    ("arc_count", oid.ArcCount));
            default:
                throw new InvalidDataException($"unsupported operation {operation}");
        }
    }

    private static object CursorResult(
        JsonObject testCase,
        DerAsn1.Asn1Decoder decoder,
        DerAsn1.Asn1Element root)
    {
        DerAsn1.Asn1Cursor cursor = decoder.Sequence(root);
        int total = cursor.Remaining.Length;
        List<Dictionary<string, object?>> events = [];
        foreach (JsonNode? raw in testCase["actions"]!.AsArray())
        {
            string action = raw!.GetValue<string>();
            if (action == "finish")
            {
                try
                {
                    cursor.Finish();
                    events.Add(Map(("outcome", "finished")));
                }
                catch (DerAsn1.Error error)
                {
                    events.Add(Error(error, "container-value"));
                }
                continue;
            }
            if (action is not ("read" or "read-with-different-limits"))
            {
                throw new InvalidDataException($"unsupported cursor action {action}");
            }
            DerAsn1.Asn1Decoder active = decoder;
            if (action == "read-with-different-limits")
            {
                DerAsn1.Asn1Limits current = decoder.Limits;
                active = new(new DerAsn1.Asn1Limits(
                    current.Der,
                    current.MaxDepth,
                    current.MaxTotalElements + 1,
                    current.MaxOidArcs));
            }
            try
            {
                DerAsn1.Asn1Element? child = cursor.Read(active);
                events.Add(child is null
                    ? Map(("outcome", "end"))
                    : Map(("outcome", "value"), ("tag", Tag(child)), ("depth", child.Depth)));
            }
            catch (DerAsn1.Error error)
            {
                events.Add(Error(error, "container-value"));
            }
        }
        return Map(
            ("outcome", "value"),
            ("elements_read", decoder.ElementsRead),
            ("remaining_offset", total - cursor.Remaining.Length),
            ("events", events));
    }

    private static DerAsn1.Asn1Limits Limits(JsonObject testCase)
    {
        JsonObject defaults = Fixture["defaults"]!.AsObject();
        JsonObject? overrides = testCase["limits"] as JsonObject;
        return new(
            DerLimits(defaults["der"]!.AsObject(), overrides?["der"] as JsonObject),
            IntegerLimit(defaults, overrides, "max_depth"),
            IntegerLimit(defaults, overrides, "max_total_elements"),
            IntegerLimit(defaults, overrides, "max_oid_arcs"));
    }

    private static TlvApi.Limits DerLimits(JsonObject defaults, JsonObject? overrides) => new(
        LongLimit(defaults, overrides, "max_input_len"),
        LongLimit(defaults, overrides, "max_value_len"),
        LongLimit(defaults, overrides, "max_elements"),
        checked((uint)LongLimit(defaults, overrides, "max_tag_number")));

    private static int IntegerLimit(JsonObject defaults, JsonObject? overrides, string name) =>
        checked((int)LongLimit(defaults, overrides, name));

    private static long LongLimit(JsonObject defaults, JsonObject? overrides, string name)
    {
        JsonNode value = overrides?[name] ?? defaults[name]!;
        return value is JsonValue json && json.TryGetValue<string>(out _) ? long.MaxValue : value.GetValue<long>();
    }

    private static Dictionary<string, object?> Tag(DerAsn1.Asn1Element element) => Map(
        ("class", element.Tag.Class),
        ("constructed", element.Tag.Constructed),
        ("number", element.Tag.Number));

    private static Dictionary<string, object?> Error(DerAsn1.Error error, string scope)
    {
        Dictionary<string, object?> result = Map(
            ("outcome", "error"),
            ("error_id", error.KindId),
            ("offset", error.Offset),
            ("offset_scope", scope));
        if (error.FramingKind is not null)
        {
            result["framing_error_id"] = error.FramingKind;
        }
        return result;
    }

    private static Dictionary<string, object?> Map(params (string Key, object? Value)[] pairs)
    {
        Dictionary<string, object?> result = new(StringComparer.Ordinal);
        foreach ((string key, object? value) in pairs)
        {
            result[key] = value;
        }
        return result;
    }

    private static byte[] Materialize(JsonArray segments)
    {
        using MemoryStream output = new();
        foreach (JsonNode? node in segments)
        {
            JsonObject segment = node!.AsObject();
            byte[] value = Convert.FromHexString((segment["hex"] ?? segment["repeat_hex"])!.GetValue<string>());
            int count = segment["count"]?.GetValue<int>() ?? 1;
            for (int index = 0; index < count; index++)
            {
                output.Write(value);
            }
        }
        return output.ToArray();
    }

    private static string Hex(ReadOnlyMemory<byte> value) => Convert.ToHexString(value.Span).ToLowerInvariant();

    private static JsonObject ReadFixture(string name)
    {
        DirectoryInfo? directory = new(AppContext.BaseDirectory);
        while (directory is not null)
        {
            string path = Path.Combine(directory.FullName, "code", "specs", "fixtures", name, "cases.json");
            if (File.Exists(path))
            {
                return JsonNode.Parse(File.ReadAllText(path))!.AsObject();
            }
            directory = directory.Parent;
        }
        throw new FileNotFoundException($"{name}/cases.json");
    }
}
