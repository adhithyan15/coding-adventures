namespace CodingAdventures.DerTlv.FSharp.Tests

open System
open System.IO
open System.Text.Json
open System.Text.Json.Nodes
open CodingAdventures.DerTlv.FSharp
open Xunit

module PortableConformanceTests =
    let private requiredString (element: JsonElement) (name: string) =
        match element.GetProperty(name).GetString() |> Option.ofObj with
        | Some value -> value
        | None -> raise (InvalidDataException(sprintf "null %s" name))

    let private findFixture () =
        let rec search (directory: DirectoryInfo option) =
            match directory with
            | None -> raise (FileNotFoundException("der-tlv-v1/cases.json"))
            | Some current ->
                let path = Path.Combine(current.FullName, "code", "specs", "fixtures", "der-tlv-v1", "cases.json")
                if File.Exists(path) then path else search (current.Parent |> Option.ofObj)
        search (Some(DirectoryInfo(AppContext.BaseDirectory)))

    let private materialize (segments: JsonElement) =
        use output = new MemoryStream()
        for segment in segments.EnumerateArray() do
            let mutable value = Unchecked.defaultof<JsonElement>
            let hex =
                if segment.TryGetProperty("hex", &value) then requiredString segment "hex"
                else requiredString segment "repeat_hex"
            let bytes = Convert.FromHexString(hex)
            let count = if segment.TryGetProperty("count", &value) then value.GetInt32() else 1
            for _ in 1 .. count do output.Write(bytes)
        output.ToArray()

    let private limits (defaults: JsonElement) (testCase: JsonElement) =
        let mutable overrides = Unchecked.defaultof<JsonElement>
        let hasOverrides = testCase.TryGetProperty("limits", &overrides)
        let read (name: string) =
            let mutable value = Unchecked.defaultof<JsonElement>
            if not hasOverrides || not (overrides.TryGetProperty(name, &value)) then value <- defaults.GetProperty(name)
            if value.ValueKind = JsonValueKind.String then Int64.MaxValue else value.GetInt64()
        DerLimits(read "max_input_len", read "max_value_len", read "max_elements", uint32 (read "max_tag_number"))

    let private errorProjection (error: DerError) =
        let value = JsonObject()
        value["outcome"] <- JsonValue.Create("error")
        value["error_id"] <- JsonValue.Create(error.Id)
        value["offset"] <- JsonValue.Create(error.Offset)
        value

    let private elementProjection (element: DerElement) (offset: int) =
        let tag = JsonObject()
        tag["class"] <- JsonValue.Create(element.Tag.Class)
        tag["constructed"] <- JsonValue.Create(element.Tag.Constructed)
        tag["number"] <- JsonValue.Create(element.Tag.Number)
        let value = JsonObject()
        value["outcome"] <- JsonValue.Create("element")
        value["element_offset"] <- JsonValue.Create(offset)
        value["tag"] <- tag
        value["header_len"] <- JsonValue.Create(element.Header.Length)
        value["encoded_len"] <- JsonValue.Create(element.Encoded.Length)
        value["remainder_offset"] <- JsonValue.Create(offset + element.Encoded.Length)
        value

    let private assertViews (input: ReadOnlyMemory<byte>) (element: DerElement) (remainder: ReadOnlyMemory<byte>) =
        let encodedLength = element.Encoded.Length
        let headerLength = element.Header.Length
        Assert.True(input.Slice(0, encodedLength).Span.SequenceEqual(element.Encoded.Span))
        Assert.True(input.Slice(0, headerLength).Span.SequenceEqual(element.Header.Span))
        Assert.True(input.Slice(headerLength, encodedLength - headerLength).Span.SequenceEqual(element.Value.Span))
        Assert.True(input.Slice(encodedLength).Span.SequenceEqual(remainder.Span))

    let private runDecode input limits exact =
        try
            let element, remainder =
                if exact then DerTlv.decodeExactWith limits input, ReadOnlyMemory<byte>.Empty
                else let decoded = DerTlv.decodeOneWith limits input in decoded.Element, decoded.Remainder
            assertViews input element remainder
            elementProjection element 0 :> JsonNode
        with :? DerError as error -> errorProjection error :> JsonNode

    let private runCursor (testCase: JsonElement) input limits =
        let cursor = DerCursor(input, limits)
        let events = JsonArray()
        for actionNode in testCase.GetProperty("actions").EnumerateArray() do
            let action =
                actionNode.GetString()
                |> Option.ofObj
                |> Option.defaultWith (fun () -> raise (InvalidDataException("null cursor action")))
            match action with
            | "finish" ->
                try cursor.Finish(); let value = JsonObject() in value["outcome"] <- JsonValue.Create("finished"); events.Add(value)
                with :? DerError as error -> events.Add(errorProjection error)
            | "read" ->
                let offset = input.Length - cursor.Remaining.Length
                let before = cursor.Remaining
                try
                    match cursor.Read() with
                    | None -> let value = JsonObject() in value["outcome"] <- JsonValue.Create("end"); events.Add(value)
                    | Some element -> assertViews before element cursor.Remaining; events.Add(elementProjection element offset)
                with :? DerError as error -> events.Add(errorProjection error)
            | _ -> raise (InvalidDataException(sprintf "unknown cursor action %A" action))
        let value = JsonObject()
        value["events"] <- events
        value["elements_read"] <- JsonValue.Create(cursor.ElementsRead)
        value["remaining_offset"] <- JsonValue.Create(input.Length - cursor.Remaining.Length)
        value :> JsonNode

    [<Fact>]
    let ``all portable fixture cases match`` () =
        use document = JsonDocument.Parse(File.ReadAllText(findFixture ()))
        let root = document.RootElement
        Assert.Equal(1, root.GetProperty("schema_version").GetInt32())
        Assert.Equal("x690-der-tlv-framing-v1", root.GetProperty("profile").GetString())
        Assert.Equal(17, root.GetProperty("error_ids").GetArrayLength())
        Assert.Equal(54, root.GetProperty("cases").GetArrayLength())
        let defaults = root.GetProperty("defaults")
        for testCase in root.GetProperty("cases").EnumerateArray() do
            let inputBytes = materialize (testCase.GetProperty("input"))
            let input = ReadOnlyMemory<byte>(inputBytes)
            let configured = limits defaults testCase
            let actual =
                match requiredString testCase "operation" with
                | "decode-one" -> runDecode input configured false
                | "decode-exact" -> runDecode input configured true
                | "cursor" -> runCursor testCase input configured
                | operation -> raise (InvalidDataException(sprintf "unknown operation %s" operation))
            let expected = JsonNode.Parse(testCase.GetProperty("expected").GetRawText())
            Assert.True(JsonNode.DeepEquals(expected, actual), requiredString testCase "id")
            let mutable hostile = Unchecked.defaultof<JsonElement>
            if testCase.TryGetProperty("redacted_input_hex", &hostile) then
                Assert.DoesNotContain(requiredString testCase "redacted_input_hex", actual.ToJsonString(), StringComparison.OrdinalIgnoreCase)

    [<Fact>]
    let ``views share caller memory and errors are payload blind`` () =
        let input = [| 4uy; 1uy; 42uy |]
        let element = DerTlv.decodeExact (ReadOnlyMemory<byte>(input))
        input[2] <- 127uy
        Assert.True(([| 127uy |]).AsSpan().SequenceEqual(element.Value.Span))
        Assert.Equal(4096L, DerLimits.Default.MaxElements)
        let error = Assert.Throws<DerError>(Action(fun () -> DerTlv.decodeExact (ReadOnlyMemory<byte>([| 0xdeuy; 0xaduy; 0xbeuy; 0xefuy |])) |> ignore))
        Assert.Equal("length-too-wide", error.Id)
        Assert.Equal(1, error.Offset)
        Assert.DoesNotContain("deadbeef", error.ToString(), StringComparison.OrdinalIgnoreCase)

    [<Fact>]
    let ``limits and native width are checked`` () =
        Assert.Throws<ArgumentOutOfRangeException>(Action(fun () -> DerLimits(-1L, 1L, 1L, 1u) |> ignore)) |> ignore
        let wide = DerLimits(Int64.MaxValue, Int64.MaxValue, 1L, UInt32.MaxValue)
        let encoded = ReadOnlyMemory<byte>([| 4uy; 136uy; 127uy; 255uy; 255uy; 255uy; 255uy; 255uy; 255uy; 255uy |])
        let error = Assert.Throws<DerError>(Action(fun () -> DerTlv.decodeOneWith wide encoded |> ignore))
        Assert.Equal("length-host-overflow", error.Id)
        let narrow = DerLimits(1L, 1L, 1L, 1u)
        Assert.Throws<DerError>(Action(fun () -> DerCursor(ReadOnlyMemory<byte>([| 5uy; 0uy |]), narrow) |> ignore)) |> ignore
