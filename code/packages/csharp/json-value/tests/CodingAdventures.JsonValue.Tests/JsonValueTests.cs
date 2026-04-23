using System.Collections;
using CodingAdventures.JsonValue;

namespace CodingAdventures.JsonValue.Tests;

public sealed class JsonValueTests
{
    [Fact]
    public void ParseBuildsTypedTreeForObjectsAndArrays()
    {
        var value = JsonValue.Parse("{\"name\":\"Alice\",\"scores\":[95,87],\"active\":true}");

        var root = Assert.IsType<JsonObjectValue>(value);
        Assert.IsType<JsonStringValue>(root.Pairs["name"]);
        var scores = Assert.IsType<JsonArrayValue>(root.Pairs["scores"]);
        Assert.Equal(2, scores.Elements.Count);
        Assert.True(Assert.IsType<JsonBooleanValue>(root.Pairs["active"]).Value);
    }

    [Fact]
    public void ParseNativeReturnsPlainDotNetValues()
    {
        var native = Assert.IsType<Dictionary<string, object?>>(JsonValue.ParseNative("{\"count\":2,\"name\":\"demo\"}"));

        Assert.Equal(2L, native["count"]);
        Assert.Equal("demo", native["name"]);
    }

    [Fact]
    public void FromNativeAcceptsAnonymousObjectsAndLists()
    {
        var value = JsonValue.FromNative(new { Name = "Alice", Scores = new[] { 1, 2, 3 } });

        var root = Assert.IsType<JsonObjectValue>(value);
        Assert.Equal("Alice", Assert.IsType<JsonStringValue>(root.Pairs["Name"]).Value);
        var scores = Assert.IsType<JsonArrayValue>(root.Pairs["Scores"]);
        Assert.Equal(3, scores.Elements.Count);
    }

    [Fact]
    public void ParsePreservesIntegerAndFloatDistinction()
    {
        var value = Assert.IsType<JsonArrayValue>(JsonValue.Parse("[1,1.5,2e4]"));

        Assert.True(Assert.IsType<JsonNumberValue>(value.Elements[0]).IsInteger);
        Assert.False(Assert.IsType<JsonNumberValue>(value.Elements[1]).IsInteger);
        Assert.False(Assert.IsType<JsonNumberValue>(value.Elements[2]).IsInteger);
    }

    [Fact]
    public void FromNativeRejectsUnsupportedSystemObjects()
    {
        var exception = Assert.Throws<JsonValueError>(() => JsonValue.FromNative(DateTime.UnixEpoch));

        Assert.Contains("Convert them to strings first", exception.Message);
    }

    [Fact]
    public void ToNativeRebuildsNestedCollections()
    {
        var value = JsonValue.JsonObject(
        [
            KeyValuePair.Create<string, JsonValue>("title", JsonValue.JsonString("demo")),
            KeyValuePair.Create<string, JsonValue>("items", JsonValue.JsonArray([JsonValue.JsonBool(true), JsonValue.JsonNull()])),
        ]);

        var native = Assert.IsType<Dictionary<string, object?>>(value.ToNative());
        var items = Assert.IsType<List<object?>>(native["items"]);

        Assert.Equal("demo", native["title"]);
        Assert.Equal([true, null], items);
    }

    [Fact]
    public void FromNativeAcceptsUntypedDictionary()
    {
        IDictionary native = new Dictionary<string, object?> { ["enabled"] = true, ["count"] = 3 };

        var value = Assert.IsType<JsonObjectValue>(JsonValue.FromNative(native));

        Assert.True(Assert.IsType<JsonBooleanValue>(value.Pairs["enabled"]).Value);
        Assert.True(Assert.IsType<JsonNumberValue>(value.Pairs["count"]).IsInteger);
    }

    [Fact]
    public void FromNativeRejectsDictionaryWithNonStringKeys()
    {
        IDictionary native = new Hashtable { [1] = "value" };

        var exception = Assert.Throws<JsonValueError>(() => JsonValue.FromNative(native));

        Assert.Contains("keys must be strings", exception.Message);
    }

    [Fact]
    public void FromNativeRejectsDelegates()
    {
        var exception = Assert.Throws<JsonValueError>(() => JsonValue.FromNative(() => "demo"));

        Assert.Contains("Delegates", exception.Message);
    }

    [Fact]
    public void ParseRejectsInvalidJson()
    {
        var exception = Assert.Throws<JsonValueError>(() => JsonValue.Parse("{oops"));

        Assert.Contains("Failed to parse JSON", exception.Message);
    }

    [Fact]
    public void JsonNumberRejectsNonFiniteValues()
    {
        var exception = Assert.Throws<JsonValueError>(() => JsonValue.JsonNumber(double.PositiveInfinity));

        Assert.Contains("cannot be NaN or infinity", exception.Message);
    }

    [Fact]
    public void FromNativeConvertsCharAndDecimalValues()
    {
        var value = Assert.IsType<JsonArrayValue>(JsonValue.FromNative(new object[] { 'x', 1.25m }));

        Assert.Equal("x", Assert.IsType<JsonStringValue>(value.Elements[0]).Value);
        Assert.False(Assert.IsType<JsonNumberValue>(value.Elements[1]).IsInteger);
    }
}
