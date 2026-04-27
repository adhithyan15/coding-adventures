namespace CodingAdventures.JsonValue.FSharp.Tests

open System
open System.Collections
open System.Collections.Generic
open CodingAdventures.JsonValue.FSharp
open Xunit

type JsonValueTests() =
    [<Fact>]
    member _.``Parse builds typed tree for objects and arrays``() =
        let value = JsonValue.Parse("""{"name":"Alice","scores":[95,87],"active":true}""")

        let root = Assert.IsType<JsonObjectValue>(value)
        Assert.IsType<JsonStringValue>(root.Pairs.["name"]) |> ignore
        let scores = Assert.IsType<JsonArrayValue>(root.Pairs.["scores"])
        Assert.Equal(2, scores.Elements.Count)
        Assert.True((Assert.IsType<JsonBooleanValue>(root.Pairs.["active"])).Value)

    [<Fact>]
    member _.``ParseNative returns plain dotnet values``() =
        let native = Assert.IsType<Dictionary<string, obj>>(JsonValue.ParseNative("""{"count":2,"name":"demo"}"""))

        Assert.Equal(box 2L, native.["count"])
        Assert.Equal(box "demo", native.["name"])

    [<Fact>]
    member _.``FromNative accepts anonymous objects and lists``() =
        let value = JsonValue.FromNative(box {| Name = "Alice"; Scores = [| 1; 2; 3 |] |})

        let root = Assert.IsType<JsonObjectValue>(value)
        Assert.Equal("Alice", (Assert.IsType<JsonStringValue>(root.Pairs.["Name"])).Value)
        let scores = Assert.IsType<JsonArrayValue>(root.Pairs.["Scores"])
        Assert.Equal(3, scores.Elements.Count)

    [<Fact>]
    member _.``Parse preserves integer and float distinction``() =
        let value = Assert.IsType<JsonArrayValue>(JsonValue.Parse("[1,1.5,2e4]"))

        Assert.True((Assert.IsType<JsonNumberValue>(value.Elements.[0])).IsInteger)
        Assert.False((Assert.IsType<JsonNumberValue>(value.Elements.[1])).IsInteger)
        Assert.False((Assert.IsType<JsonNumberValue>(value.Elements.[2])).IsInteger)

    [<Fact>]
    member _.``FromNative rejects unsupported system objects``() =
        let ex = Assert.Throws<JsonValueError>(fun () -> JsonValue.FromNative(box DateTime.UnixEpoch) |> ignore)

        Assert.Contains("Convert them to strings first", ex.Message)

    [<Fact>]
    member _.``ToNative rebuilds nested collections``() =
        let value =
            JsonValue.JsonObject(
                [
                    "title", JsonValue.JsonString("demo")
                    "items", JsonValue.JsonArray([ JsonValue.JsonBool(true); JsonValue.JsonNull() ])
                ])

        let native = Assert.IsType<Dictionary<string, obj>>(value.ToNative())
        let items = Assert.IsType<ResizeArray<obj>>(native.["items"])

        Assert.Equal(box "demo", native.["title"])
        Assert.Equal<obj array>([| box true; null |], items.ToArray())

    [<Fact>]
    member _.``FromNative accepts untyped dictionary``() =
        let native = Dictionary<string, obj>()
        native.["enabled"] <- box true
        native.["count"] <- box 3

        let value = Assert.IsType<JsonObjectValue>(JsonValue.FromNative(box native))

        Assert.True((Assert.IsType<JsonBooleanValue>(value.Pairs.["enabled"])).Value)
        Assert.True((Assert.IsType<JsonNumberValue>(value.Pairs.["count"])).IsInteger)

    [<Fact>]
    member _.``FromNative rejects dictionary with non string keys``() =
        let native = Hashtable()
        native.[1] <- "value"

        let ex = Assert.Throws<JsonValueError>(fun () -> JsonValue.FromNative(box native) |> ignore)

        Assert.Contains("keys must be strings", ex.Message)

    [<Fact>]
    member _.``FromNative rejects delegates``() =
        let callback = Func<string>(fun () -> "demo")
        let ex = Assert.Throws<JsonValueError>(fun () -> JsonValue.FromNative(box callback) |> ignore)

        Assert.Contains("Delegates", ex.Message)

    [<Fact>]
    member _.``Parse rejects invalid json``() =
        let ex = Assert.Throws<JsonValueError>(fun () -> JsonValue.Parse("{oops") |> ignore)

        Assert.Contains("Failed to parse JSON", ex.Message)

    [<Fact>]
    member _.``JsonNumber rejects non finite values``() =
        let ex = Assert.Throws<JsonValueError>(fun () -> JsonValue.JsonNumber(Double.PositiveInfinity) |> ignore)

        Assert.Contains("cannot be NaN or infinity", ex.Message)

    [<Fact>]
    member _.``FromNative converts char and decimal values``() =
        let value = Assert.IsType<JsonArrayValue>(JsonValue.FromNative(box [| box 'x'; box 1.25m |]))

        Assert.Equal("x", (Assert.IsType<JsonStringValue>(value.Elements.[0])).Value)
        Assert.False((Assert.IsType<JsonNumberValue>(value.Elements.[1])).IsInteger)
