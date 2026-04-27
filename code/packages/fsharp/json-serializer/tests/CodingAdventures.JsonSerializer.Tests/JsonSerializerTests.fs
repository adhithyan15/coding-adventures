namespace CodingAdventures.JsonSerializer.FSharp.Tests

open System.Collections.Generic
open CodingAdventures.JsonSerializer.FSharp
open CodingAdventures.JsonValue.FSharp
open Xunit

type JsonSerializerTests() =
    [<Fact>]
    member _.``Serialize produces compact json``() =
        let value =
            JsonValue.JsonObject(
                [
                    "name", JsonValue.JsonString("Alice")
                    "age", JsonValue.JsonNumber(30.0, true)
                ])

        let text = JsonSerializer.Serialize(value)

        Assert.Equal("{\"name\":\"Alice\",\"age\":30}", text)

    [<Fact>]
    member _.``SerializePretty honors formatting options``() =
        let value =
            JsonValue.JsonObject(
                [
                    "b", JsonValue.JsonNumber(2.0, true)
                    "a", JsonValue.JsonNumber(1.0, true)
                ])

        let text = JsonSerializer.SerializePretty(value, SerializerConfig(indentSize = 4, sortKeys = true, trailingNewline = true))

        Assert.Equal("{\n    \"a\": 1,\n    \"b\": 2\n}\n", text)

    [<Fact>]
    member _.``Stringify converts native values through json value``() =
        let native = Dictionary<string, obj>()
        native.["name"] <- box "Alice"
        native.["active"] <- box true

        let text = JsonSerializer.Stringify(box native)

        Assert.Equal("{\"name\":\"Alice\",\"active\":true}", text)

    [<Fact>]
    member _.``Serialize escapes control characters``() =
        let value = JsonValue.JsonString("line 1\nline 2\t\"quoted\"")

        let text = JsonSerializer.Serialize(value)

        Assert.Equal("\"line 1\\nline 2\\t\\\"quoted\\\"\"", text)

    [<Fact>]
    member _.``Serialize handles empty collections``() =
        Assert.Equal("[]", JsonSerializer.Serialize(JsonValue.JsonArray()))
        Assert.Equal("{}", JsonSerializer.Serialize(JsonValue.JsonObject()))

    [<Fact>]
    member _.``SerializePretty formats nested arrays``() =
        let value = JsonValue.JsonArray([ JsonValue.JsonNumber(1.0, true); JsonValue.JsonArray([ JsonValue.JsonString("two") ]) ])

        let text = JsonSerializer.SerializePretty(value)

        Assert.Equal("[\n  1,\n  [\n    \"two\"\n  ]\n]", text)

    [<Fact>]
    member _.``Serialize rejects non finite numbers``() =
        let ex = Assert.Throws<JsonSerializerError>(fun () -> JsonSerializer.Serialize(JsonNumberValue(System.Double.NaN, false)) |> ignore)

        Assert.Contains("NaN or infinity", ex.Message)

    [<Fact>]
    member _.``StringifyPretty formats native objects``() =
        let native = Dictionary<string, obj>()
        native.["name"] <- box "Alice"
        native.["items"] <- box [| 1; 2 |]

        let text = JsonSerializer.StringifyPretty(box native)

        Assert.Equal("{\n  \"name\": \"Alice\",\n  \"items\": [\n    1,\n    2\n  ]\n}", text)

    [<Fact>]
    member _.``SerializePretty keeps empty objects compact``() =
        let text = JsonSerializer.SerializePretty(JsonValue.JsonObject())

        Assert.Equal("{}", text)
