using System.Text.Json;
using System.Text.Json.Nodes;
using CodingAdventures.DerTlv.CSharp;

public sealed class PortableConformanceTests
{
    private static readonly JsonObject Fixture = LoadFixture();

    [Fact]
    public void MatchesPortableFixture()
    {
        Assert.Equal(1, Fixture["schema_version"]!.GetValue<int>());
        Assert.Equal("x690-der-tlv-framing-v1", Fixture["profile"]!.GetValue<string>());
        Assert.Equal(17, Fixture["error_ids"]!.AsArray().Count);
        JsonArray cases = Fixture["cases"]!.AsArray();
        Assert.Equal(54, cases.Count);
        foreach (JsonNode? node in cases)
        {
            JsonObject testCase = node!.AsObject();
            byte[] input = Materialize(testCase["input"]!.AsArray());
            JsonNode actual = testCase["operation"]!.GetValue<string>() switch
            {
                "decode-one" => RunDecode(input, Limits(testCase), false),
                "decode-exact" => RunDecode(input, Limits(testCase), true),
                "cursor" => RunCursor(testCase, input, Limits(testCase)),
                string operation => throw new InvalidDataException($"unknown operation {operation}"),
            };
            Assert.True(JsonNode.DeepEquals(testCase["expected"], actual), testCase["id"]!.GetValue<string>());
            if (testCase["redacted_input_hex"] is JsonValue hostile)
            {
                Assert.DoesNotContain(hostile.GetValue<string>(), actual.ToJsonString(), StringComparison.OrdinalIgnoreCase);
            }
        }
    }

    [Fact]
    public void ViewsShareCallerMemoryAndErrorsArePayloadBlind()
    {
        byte[] input = [4, 1, 42];
        DerTlv.Element element = DerTlv.DecodeExact(input);
        input[2] = 127;
        Assert.Equal(new byte[] { 127 }, element.Value.ToArray());
        Assert.Equal(4096, DerTlv.Limits.Default.MaxElements);
        DerTlv.Error error = Assert.Throws<DerTlv.Error>(() => DerTlv.DecodeExact(new byte[] { 0xde, 0xad, 0xbe, 0xef }));
        Assert.Equal("length-too-wide", error.Kind);
        Assert.Equal(1, error.Offset);
        Assert.DoesNotContain("deadbeef", error.ToString(), StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void RejectsInvalidLimitsAndCoversNativeWidthBoundary()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new DerTlv.Limits(-1, 1, 1, 1));
        DerTlv.Limits wide = new(long.MaxValue, long.MaxValue, 1, uint.MaxValue);
        DerTlv.Error error = Assert.Throws<DerTlv.Error>(() => DerTlv.DecodeOne(new byte[] { 4, 136, 127, 255, 255, 255, 255, 255, 255, 255 }, wide));
        Assert.Equal("length-host-overflow", error.Kind);
        Assert.Throws<DerTlv.Error>(() => new DerTlv.Cursor(new byte[] { 5, 0 }, new DerTlv.Limits(1, 1, 1, 1)));
    }

    private static JsonNode RunDecode(byte[] input, DerTlv.Limits limits, bool exact)
    {
        try
        {
            DerTlv.Element element;
            ReadOnlyMemory<byte> remainder;
            if (exact)
            {
                element = DerTlv.DecodeExact(input, limits);
                remainder = ReadOnlyMemory<byte>.Empty;
            }
            else
            {
                DerTlv.Decoded decoded = DerTlv.DecodeOne(input, limits);
                element = decoded.Element;
                remainder = decoded.Remainder;
            }
            AssertViews(input, element, remainder);
            return ElementProjection(element, 0);
        }
        catch (DerTlv.Error error)
        {
            return ErrorProjection(error);
        }
    }

    private static JsonNode RunCursor(JsonObject testCase, byte[] input, DerTlv.Limits limits)
    {
        DerTlv.Cursor cursor = new(input, limits);
        JsonArray events = [];
        foreach (JsonNode? actionNode in testCase["actions"]!.AsArray())
        {
            string action = actionNode!.GetValue<string>();
            if (action == "finish")
            {
                try { cursor.Finish(); events.Add(new JsonObject { ["outcome"] = "finished" }); }
                catch (DerTlv.Error error) { events.Add(ErrorProjection(error)); }
            }
            else if (action == "read")
            {
                int offset = input.Length - cursor.Remaining.Length;
                ReadOnlyMemory<byte> before = cursor.Remaining;
                try
                {
                    DerTlv.Element? element = cursor.Read();
                    if (element is null) events.Add(new JsonObject { ["outcome"] = "end" });
                    else { AssertViews(before, element, cursor.Remaining); events.Add(ElementProjection(element, offset)); }
                }
                catch (DerTlv.Error error) { events.Add(ErrorProjection(error)); }
            }
            else throw new InvalidDataException($"unknown cursor action {action}");
        }
        return new JsonObject
        {
            ["events"] = events,
            ["elements_read"] = cursor.ElementsRead,
            ["remaining_offset"] = input.Length - cursor.Remaining.Length,
        };
    }

    private static void AssertViews(ReadOnlyMemory<byte> input, DerTlv.Element element, ReadOnlyMemory<byte> remainder)
    {
        int encoded = element.Encoded.Length;
        int header = element.Header.Length;
        Assert.True(input[..encoded].Span.SequenceEqual(element.Encoded.Span));
        Assert.True(input[..header].Span.SequenceEqual(element.Header.Span));
        Assert.True(input.Slice(header, encoded - header).Span.SequenceEqual(element.Value.Span));
        Assert.True(input[encoded..].Span.SequenceEqual(remainder.Span));
    }

    private static JsonObject ElementProjection(DerTlv.Element element, int offset) => new()
    {
        ["outcome"] = "element",
        ["element_offset"] = offset,
        ["tag"] = new JsonObject { ["class"] = element.Tag.Class, ["constructed"] = element.Tag.Constructed, ["number"] = element.Tag.Number },
        ["header_len"] = element.Header.Length,
        ["encoded_len"] = element.Encoded.Length,
        ["remainder_offset"] = offset + element.Encoded.Length,
    };

    private static JsonObject ErrorProjection(DerTlv.Error error) => new()
    {
        ["outcome"] = "error", ["error_id"] = error.Kind, ["offset"] = error.Offset,
    };

    private static DerTlv.Limits Limits(JsonObject testCase)
    {
        JsonObject defaults = Fixture["defaults"]!.AsObject();
        JsonObject? overrides = testCase["limits"] as JsonObject;
        long ReadLong(string name)
        {
            JsonNode value = overrides?[name] ?? defaults[name]!;
            return value is JsonValue json && json.TryGetValue<string>(out _) ? long.MaxValue : value.GetValue<long>();
        }
        return new(ReadLong("max_input_len"), ReadLong("max_value_len"), ReadLong("max_elements"), (uint)ReadLong("max_tag_number"));
    }

    private static byte[] Materialize(JsonArray segments)
    {
        using MemoryStream output = new();
        foreach (JsonNode? node in segments)
        {
            JsonObject segment = node!.AsObject();
            byte[] value = Convert.FromHexString((segment["hex"] ?? segment["repeat_hex"])!.GetValue<string>());
            int count = segment["count"]?.GetValue<int>() ?? 1;
            for (int index = 0; index < count; index++) output.Write(value);
        }
        return output.ToArray();
    }

    private static JsonObject LoadFixture()
    {
        DirectoryInfo? directory = new(AppContext.BaseDirectory);
        while (directory is not null)
        {
            string path = Path.Combine(directory.FullName, "code", "specs", "fixtures", "der-tlv-v1", "cases.json");
            if (File.Exists(path)) return JsonNode.Parse(File.ReadAllText(path))!.AsObject();
            directory = directory.Parent;
        }
        throw new FileNotFoundException("der-tlv-v1/cases.json");
    }
}
