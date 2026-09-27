using System.Globalization;
using System.Text.Json;
using System.Text.Json.Nodes;
using CodingAdventures.X509Extension.CSharp;
using Asn1Api = CodingAdventures.DerAsn1.CSharp.DerAsn1;
using TlvApi = CodingAdventures.DerTlv.CSharp.DerTlv;

namespace CodingAdventures.X509Extension.CSharp.Tests;

public sealed class PortableConformanceTests
{
    private static readonly JsonObject Contract = ReadFixture("x509-extension-v1");
    private static readonly JsonObject Upstream = ReadFixture("der-asn1-v1");

    public static IEnumerable<object[]> Cases() => Contract["cases"]!.AsArray()
        .Select(node => new object[] { node!["id"]!.GetValue<string>() });

    [Theory]
    [MemberData(nameof(Cases))]
    public void ConsumesEveryPortableFixture(string id)
    {
        JsonObject testCase = Contract["cases"]!.AsArray()
            .Select(node => node!.AsObject())
            .Single(candidate => candidate["id"]!.GetValue<string>() == id);
        Asn1Api.Asn1Decoder decoder = new(Limits(testCase));
        Asn1Api.Asn1Element root = decoder.DecodeExact(Materialize(testCase["input"]!.AsArray()));
        object actual;
        if (testCase["operation"]!.GetValue<string>() == "extension-script")
        {
            List<Dictionary<string, object?>> events = [];
            foreach (JsonNode? _ in testCase["actions"]!.AsArray())
                events.Add(Attempt(decoder, root));
            actual = Map(("outcome", "script"), ("events", events));
        }
        else
        {
            actual = Attempt(decoder, root);
        }

        JsonNode normalized = JsonSerializer.SerializeToNode(actual)!;
        Assert.True(JsonNode.DeepEquals(testCase["expected"], normalized),
            $"{id}\nexpected={testCase["expected"]}\nactual={normalized}");
        if (testCase["redacted_input_hex"] is JsonValue hostile)
        {
            Assert.DoesNotContain(
                hostile.GetValue<string>(),
                normalized.ToJsonString(),
                StringComparison.OrdinalIgnoreCase);
        }
    }

    [Fact]
    public void FixtureRosterIsClosed()
    {
        Assert.Equal(48, Contract["cases"]!.AsArray().Count);
        Assert.Equal(8, Contract["error_ids"]!.AsArray().Count);
    }

    [Fact]
    public void ValidatedValuesAreUnforgeableImmutableAndRedacted()
    {
        Assert.Empty(typeof(X509ExtensionValue).GetConstructors());
        Assert.Empty(typeof(X509ExtensionError).GetConstructors());

        byte[] input = Convert.FromHexString("30090603551D1104023000");
        Asn1Api.Asn1Decoder decoder = new();
        X509ExtensionValue value = X509Extension.DecodeX509Extension(
            decoder, decoder.DecodeExact(input));
        input[9] = 0xff;
        Assert.Equal("3000", Hex(value.ExtensionValue));
        byte[] exposed = value.ExtensionValue.ToArray();
        exposed[0] = 0xff;
        Assert.Equal("3000", Hex(value.ExtensionValue));

        Asn1Api.Asn1Decoder hostileDecoder = new();
        Asn1Api.Asn1Element hostile = hostileDecoder.DecodeExact(
            Convert.FromHexString("30080601800403DEADBE"));
        X509ExtensionError error = Assert.Throws<X509ExtensionError>(() =>
            X509Extension.DecodeX509Extension(hostileDecoder, hostile));
        Assert.Equal(X509ExtensionErrorKind.InvalidExtensionId, error.Kind);
        Assert.Equal(Asn1Api.Asn1ErrorKind.NonMinimalObjectIdentifier, error.Asn1Kind);
        Assert.Equal(4, error.Offset);
        Assert.Equal(2, hostileDecoder.ElementsRead);
        Assert.DoesNotContain("deadbe", error.ToString(), StringComparison.OrdinalIgnoreCase);
    }

    private static Dictionary<string, object?> Attempt(
        Asn1Api.Asn1Decoder decoder,
        Asn1Api.Asn1Element root)
    {
        try
        {
            X509ExtensionValue value = X509Extension.DecodeX509Extension(decoder, root);
            return Map(
                ("outcome", "value"),
                ("extension_id_arcs_decimal", value.ExtensionId.Arcs
                    .Select(arc => arc.ToString(CultureInfo.InvariantCulture)).ToArray()),
                ("critical", value.Critical),
                ("extension_value_hex", Hex(value.ExtensionValue)),
                ("elements_read", decoder.ElementsRead));
        }
        catch (X509ExtensionError error)
        {
            Dictionary<string, object?> result = Map(
                ("outcome", "error"),
                ("error_id", error.KindId),
                ("offset", error.Offset),
                ("offset_scope", "extension-element"),
                ("elements_read", decoder.ElementsRead));
            if (error.Asn1Kind is not null)
                result["asn1_error_id"] = Asn1Api.ErrorId(error.Asn1Kind.Value);
            if (error.FramingKind is not null)
                result["framing_error_id"] = error.FramingKind;
            return result;
        }
    }

    private static Asn1Api.Asn1Limits Limits(JsonObject testCase)
    {
        JsonObject defaults = Upstream["defaults"]!.AsObject();
        JsonObject? overrides = testCase["limits"] as JsonObject;
        return new Asn1Api.Asn1Limits(
            DerLimits(defaults["der"]!.AsObject(), overrides?["der"] as JsonObject),
            checked((int)LongLimit(defaults, overrides, "max_depth")),
            LongLimit(defaults, overrides, "max_total_elements"),
            checked((int)LongLimit(defaults, overrides, "max_oid_arcs")));
    }

    private static TlvApi.Limits DerLimits(JsonObject defaults, JsonObject? overrides) => new(
        LongLimit(defaults, overrides, "max_input_len"),
        LongLimit(defaults, overrides, "max_value_len"),
        LongLimit(defaults, overrides, "max_elements"),
        checked((uint)LongLimit(defaults, overrides, "max_tag_number")));

    private static long LongLimit(JsonObject defaults, JsonObject? overrides, string name)
    {
        JsonNode value = overrides?[name] ?? defaults[name]!;
        return value is JsonValue json && json.TryGetValue<string>(out _)
            ? long.MaxValue
            : value.GetValue<long>();
    }

    private static Dictionary<string, object?> Map(params (string Key, object? Value)[] pairs)
    {
        Dictionary<string, object?> result = new(StringComparer.Ordinal);
        foreach ((string key, object? value) in pairs)
            result[key] = value;
        return result;
    }

    private static byte[] Materialize(JsonArray segments)
    {
        using MemoryStream output = new();
        foreach (JsonNode? node in segments)
        {
            JsonObject segment = node!.AsObject();
            byte[] value = Convert.FromHexString(
                (segment["hex"] ?? segment["repeat_hex"])!.GetValue<string>());
            int count = segment["count"]?.GetValue<int>() ?? 1;
            for (int index = 0; index < count; index++)
                output.Write(value);
        }
        return output.ToArray();
    }

    private static string Hex(ReadOnlyMemory<byte> value) =>
        Convert.ToHexString(value.Span).ToLowerInvariant();

    private static JsonObject ReadFixture(string name)
    {
        DirectoryInfo? directory = new(AppContext.BaseDirectory);
        while (directory is not null)
        {
            string path = Path.Combine(
                directory.FullName, "code", "specs", "fixtures", name, "cases.json");
            if (File.Exists(path))
                return JsonNode.Parse(File.ReadAllText(path))!.AsObject();
            directory = directory.Parent;
        }
        throw new FileNotFoundException($"{name}/cases.json");
    }
}
