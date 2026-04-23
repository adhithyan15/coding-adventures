using CodingAdventures.JsonValue;
using JsonNode = CodingAdventures.JsonValue.JsonValue;

namespace CodingAdventures.JsonSerializer.Tests;

public sealed class JsonSerializerTests
{
    [Fact]
    public void SerializeProducesCompactJson()
    {
        var value = JsonNode.JsonObject(
        [
            KeyValuePair.Create<string, JsonNode>("name", JsonNode.JsonString("Alice")),
            KeyValuePair.Create<string, JsonNode>("age", JsonNode.JsonNumber(30, isInteger: true)),
        ]);

        var text = JsonSerializer.Serialize(value);

        Assert.Equal("{\"name\":\"Alice\",\"age\":30}", text);
    }

    [Fact]
    public void SerializePrettyHonorsFormattingOptions()
    {
        var value = JsonNode.JsonObject(
        [
            KeyValuePair.Create<string, JsonNode>("b", JsonNode.JsonNumber(2, true)),
            KeyValuePair.Create<string, JsonNode>("a", JsonNode.JsonNumber(1, true)),
        ]);

        var text = JsonSerializer.SerializePretty(value, new SerializerConfig(IndentSize: 4, SortKeys: true, TrailingNewline: true));

        Assert.Equal("{\n    \"a\": 1,\n    \"b\": 2\n}\n", text);
    }

    [Fact]
    public void StringifyConvertsNativeValuesThroughJsonValue()
    {
        var text = JsonSerializer.Stringify(new Dictionary<string, object?> { ["name"] = "Alice", ["active"] = true });

        Assert.Equal("{\"name\":\"Alice\",\"active\":true}", text);
    }

    [Fact]
    public void SerializeEscapesControlCharacters()
    {
        var value = JsonNode.JsonString("line 1\nline 2\t\"quoted\"");

        var text = JsonSerializer.Serialize(value);

        Assert.Equal("\"line 1\\nline 2\\t\\\"quoted\\\"\"", text);
    }

    [Fact]
    public void SerializeHandlesEmptyCollections()
    {
        Assert.Equal("[]", JsonSerializer.Serialize(JsonNode.JsonArray()));
        Assert.Equal("{}", JsonSerializer.Serialize(JsonNode.JsonObject()));
    }

    [Fact]
    public void SerializePrettyFormatsNestedArrays()
    {
        var value = JsonNode.JsonArray([JsonNode.JsonNumber(1, true), JsonNode.JsonArray([JsonNode.JsonString("two")])]);

        var text = JsonSerializer.SerializePretty(value);

        Assert.Equal("[\n  1,\n  [\n    \"two\"\n  ]\n]", text);
    }

    [Fact]
    public void SerializeRejectsNonFiniteNumbers()
    {
        var exception = Assert.Throws<JsonSerializerError>(() => JsonSerializer.Serialize(new JsonNumberValue(double.NaN, false)));

        Assert.Contains("NaN or infinity", exception.Message);
    }

    [Fact]
    public void StringifyPrettyFormatsNativeObjects()
    {
        var text = JsonSerializer.StringifyPretty(new Dictionary<string, object?> { ["name"] = "Alice", ["items"] = new[] { 1, 2 } });

        Assert.Equal("{\n  \"name\": \"Alice\",\n  \"items\": [\n    1,\n    2\n  ]\n}", text);
    }

    [Fact]
    public void SerializePrettyKeepsEmptyObjectsCompact()
    {
        var text = JsonSerializer.SerializePretty(JsonNode.JsonObject());

        Assert.Equal("{}", text);
    }
}
