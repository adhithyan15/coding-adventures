namespace CodingAdventures.DerAsn1.FSharp.Tests

open System
open System.Collections.Generic
open System.Globalization
open System.IO
open System.Text.Json
open System.Text.Json.Nodes
open CodingAdventures.DerTlv.FSharp
open CodingAdventures.DerAsn1.FSharp
open Xunit

module PortableConformanceTests =
    let private requiredString (element: JsonElement) (name: string) =
        match element.GetProperty(name).GetString() |> Option.ofObj with
        | Some value -> value
        | None -> raise (InvalidDataException(sprintf "null %s" name))

    let private findFixture name =
        let rec search (directory: DirectoryInfo option) =
            match directory with
            | None -> raise (FileNotFoundException(sprintf "%s/cases.json" name))
            | Some current ->
                let path = Path.Combine(current.FullName, "code", "specs", "fixtures", name, "cases.json")
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

    let private readInt64 (defaults: JsonElement) (overrides: JsonElement option) (name: string) =
        let mutable value = Unchecked.defaultof<JsonElement>
        let selected =
            match overrides with
            | Some candidate when candidate.TryGetProperty(name, &value) -> value
            | _ -> defaults.GetProperty(name)
        if selected.ValueKind = JsonValueKind.String then Int64.MaxValue else selected.GetInt64()

    let private derLimits (defaults: JsonElement) (overrides: JsonElement option) =
        DerLimits(
            readInt64 defaults overrides "max_input_len",
            readInt64 defaults overrides "max_value_len",
            readInt64 defaults overrides "max_elements",
            uint32 (readInt64 defaults overrides "max_tag_number"))

    let private limits (fixture: JsonElement) (testCase: JsonElement) =
        let defaults = fixture.GetProperty("defaults")
        let mutable outerOverride = Unchecked.defaultof<JsonElement>
        let outer = if testCase.TryGetProperty("limits", &outerOverride) then Some outerOverride else None
        let mutable derOverride = Unchecked.defaultof<JsonElement>
        let der =
            match outer with
            | Some candidate when candidate.TryGetProperty("der", &derOverride) -> Some derOverride
            | _ -> None
        Asn1Limits(
            derLimits (defaults.GetProperty("der")) der,
            int (readInt64 defaults outer "max_depth"),
            readInt64 defaults outer "max_total_elements",
            int (readInt64 defaults outer "max_oid_arcs"))

    let private addString (value: JsonObject) (name: string) (content: string) = value[name] <- JsonValue.Create(content)
    let private addInt (value: JsonObject) (name: string) (content: int) = value[name] <- JsonValue.Create(content)
    let private addInt64 (value: JsonObject) (name: string) (content: int64) = value[name] <- JsonValue.Create(content)
    let private addUInt32 (value: JsonObject) (name: string) (content: uint32) = value[name] <- JsonValue.Create(content)
    let private addBool (value: JsonObject) (name: string) (content: bool) = value[name] <- JsonValue.Create(content)

    let private tagProjection (element: Asn1Element) =
        let tag = JsonObject()
        addString tag "class" element.Tag.Class
        addBool tag "constructed" element.Tag.Constructed
        addUInt32 tag "number" element.Tag.Number
        tag

    let private errorProjection (error: Asn1Error) scope =
        let value = JsonObject()
        addString value "outcome" "error"
        addString value "error_id" error.KindId
        addInt value "offset" error.Offset
        addString value "offset_scope" scope
        match error.FramingKind with
        | Some kind -> addString value "framing_error_id" kind
        | None -> ()
        value

    let private verifyUpstream (upstream: JsonElement) reference =
        let referenced =
            upstream.GetProperty("cases").EnumerateArray()
            |> Seq.find (fun candidate -> requiredString candidate "id" = reference)
        let mutable overrideNode = Unchecked.defaultof<JsonElement>
        let overrides = if referenced.TryGetProperty("limits", &overrideNode) then Some overrideNode else None
        let framingLimits = derLimits (upstream.GetProperty("defaults")) overrides
        let decoder = Asn1Decoder(Asn1Limits(framingLimits, 32, 16_384L, 128))
        let expected = referenced.GetProperty("expected")
        try
            let element = decoder.DecodeExact(ReadOnlyMemory<byte>(materialize (referenced.GetProperty("input"))))
            Assert.Equal("element", requiredString expected "outcome")
            Assert.Equal(requiredString (expected.GetProperty("tag")) "class", element.Tag.Class)
            Assert.Equal(expected.GetProperty("tag").GetProperty("constructed").GetBoolean(), element.Tag.Constructed)
            Assert.Equal(expected.GetProperty("tag").GetProperty("number").GetUInt32(), element.Tag.Number)
            Assert.Equal(expected.GetProperty("header_len").GetInt32(), element.Header.Length)
            Assert.Equal(expected.GetProperty("encoded_len").GetInt32(), element.Encoded.Length)
        with :? Asn1Error as error ->
            Assert.Equal("error", requiredString expected "outcome")
            Assert.Equal("framing", error.KindId)
            Assert.Equal(requiredString expected "error_id", error.FramingKind.Value)
            Assert.Equal(expected.GetProperty("offset").GetInt32(), error.Offset)
        let value = JsonObject()
        addString value "outcome" "upstream"
        value :> JsonNode

    let private primitiveResult operation (element: Asn1Element) configured tagNumber =
        let value = JsonObject()
        addString value "outcome" "value"
        match operation with
        | "decode-boolean" -> addBool value "boolean" (DerAsn1.decodeBoolean element)
        | "decode-integer"
        | "integer-to-u64" ->
            let integer = DerAsn1.decodeInteger element
            addString value "signed_hex" (Convert.ToHexString(integer.SignedBytes.Span).ToLowerInvariant())
            addBool value "negative" integer.IsNegative
            if operation = "integer-to-u64" then
                addString value "u64_decimal" (integer.ToUInt64().ToString(CultureInfo.InvariantCulture))
        | "decode-bit-string" ->
            let bits = DerAsn1.decodeBitString element
            addString value "bytes_hex" (Convert.ToHexString(bits.Bytes.Span).ToLowerInvariant())
            addInt value "unused_bits" (int bits.UnusedBits)
            addInt64 value "bit_length" bits.BitLength
        | "decode-octet-string" ->
            addString value "bytes_hex" (Convert.ToHexString((DerAsn1.decodeOctetString element).Span).ToLowerInvariant())
        | "decode-implicit-octet-string" ->
            addString value "bytes_hex" (Convert.ToHexString((DerAsn1.decodeImplicitOctetString element tagNumber).Span).ToLowerInvariant())
        | "decode-ia5-string" -> addString value "text" (DerAsn1.decodeIa5String element)
        | "decode-implicit-ia5-string" -> addString value "text" (DerAsn1.decodeImplicitIa5String element tagNumber)
        | "decode-null" -> DerAsn1.decodeNull element
        | "decode-object-identifier"
        | "decode-implicit-object-identifier" ->
            let oid =
                if operation = "decode-object-identifier" then DerAsn1.decodeObjectIdentifierWith configured element
                else DerAsn1.decodeImplicitObjectIdentifierWith configured element tagNumber
            addString value "bytes_hex" (Convert.ToHexString(oid.Encoded.Span).ToLowerInvariant())
            let arcs = JsonArray()
            oid.Arcs |> Seq.iter (fun arc -> arcs.Add(JsonValue.Create(arc.ToString(CultureInfo.InvariantCulture))))
            value["arcs_decimal"] <- arcs
            addInt value "arc_count" oid.ArcCount
        | _ -> raise (InvalidDataException(sprintf "unsupported operation %s" operation))
        value

    let private cursorResult (testCase: JsonElement) (decoder: Asn1Decoder) (root: Asn1Element) =
        let cursor = decoder.Sequence(root)
        let total = cursor.Remaining.Length
        let events = JsonArray()
        for raw in testCase.GetProperty("actions").EnumerateArray() do
            let action = raw.GetString() |> Option.ofObj |> Option.defaultWith (fun () -> raise (InvalidDataException("null action")))
            if action = "finish" then
                try
                    cursor.Finish()
                    let event = JsonObject()
                    addString event "outcome" "finished"
                    events.Add(event)
                with :? Asn1Error as error -> events.Add(errorProjection error "container-value")
            else
                let active =
                    if action = "read-with-different-limits" then
                        let current = decoder.Limits
                        Asn1Decoder(Asn1Limits(current.Der, current.MaxDepth, current.MaxTotalElements + 1L, current.MaxOidArcs))
                    else decoder
                try
                    match cursor.Read(active) with
                    | None ->
                        let event = JsonObject()
                        addString event "outcome" "end"
                        events.Add(event)
                    | Some child when action = "read-nested-sequence" ->
                        let nested = decoder.Sequence(child)
                        let grandchild = nested.Read(decoder) |> Option.defaultWith (fun () -> raise (InvalidDataException("nested child required")))
                        nested.Finish()
                        let event = JsonObject()
                        addString event "outcome" "value"
                        event["tag"] <- tagProjection grandchild
                        addInt event "depth" grandchild.Depth
                        events.Add(event)
                    | Some child ->
                        let event = JsonObject()
                        addString event "outcome" "value"
                        event["tag"] <- tagProjection child
                        addInt event "depth" child.Depth
                        events.Add(event)
                with :? Asn1Error as error -> events.Add(errorProjection error "container-value")
        let value = JsonObject()
        addString value "outcome" "value"
        addInt64 value "elements_read" decoder.ElementsRead
        addInt value "remaining_offset" (total - cursor.Remaining.Length)
        value["events"] <- events
        value

    let private runCase (fixture: JsonElement) (upstream: JsonElement) (testCase: JsonElement) =
        let mutable reference = Unchecked.defaultof<JsonElement>
        if testCase.TryGetProperty("der_tlv_case_id", &reference) then
            verifyUpstream upstream (requiredString testCase "der_tlv_case_id")
        else
            let configured = limits fixture testCase
            let decoder = Asn1Decoder(configured)
            let operation = requiredString testCase "operation"
            try
                let root = decoder.DecodeExact(ReadOnlyMemory<byte>(materialize (testCase.GetProperty("input"))))
                let value =
                    match operation with
                    | "decode-exact" ->
                        let result = JsonObject()
                        addString result "outcome" "value"
                        result["tag"] <- tagProjection root
                        addString result "header_hex" (Convert.ToHexString(root.Header.Span).ToLowerInvariant())
                        addString result "value_hex" (Convert.ToHexString(root.Value.Span).ToLowerInvariant())
                        addString result "encoded_hex" (Convert.ToHexString(root.Encoded.Span).ToLowerInvariant())
                        addInt result "depth" root.Depth
                        addInt64 result "elements_read" decoder.ElementsRead
                        result
                    | "cursor-script" -> cursorResult testCase decoder root
                    | "sequence"
                    | "set" ->
                        let cursor = if operation = "sequence" then decoder.Sequence(root) else decoder.Set(root)
                        let result = JsonObject()
                        addString result "outcome" "value"
                        addInt64 result "elements_read" decoder.ElementsRead
                        addInt result "remaining_offset" (root.Value.Length - cursor.Remaining.Length)
                        result
                    | "explicit" ->
                        let child = decoder.Explicit(root, testCase.GetProperty("tag_number").GetUInt32())
                        let result = JsonObject()
                        addString result "outcome" "value"
                        result["tag"] <- tagProjection child
                        addString result "value_hex" (Convert.ToHexString(child.Value.Span).ToLowerInvariant())
                        addInt result "depth" child.Depth
                        addInt64 result "elements_read" decoder.ElementsRead
                        result
                    | _ ->
                        let tagNumber =
                            let mutable node = Unchecked.defaultof<JsonElement>
                            if testCase.TryGetProperty("tag_number", &node) then node.GetUInt32() else 0u
                        let result = primitiveResult operation root configured tagNumber
                        let mutable count = Unchecked.defaultof<JsonElement>
                        if testCase.GetProperty("expected").TryGetProperty("elements_read", &count) then
                            addInt64 result "elements_read" decoder.ElementsRead
                        result
                value :> JsonNode
            with :? Asn1Error as error ->
                let scope = if operation = "explicit" && error.KindId = "framing" then "container-value" else "operation-input"
                errorProjection error scope :> JsonNode

    [<Fact>]
    let ``all portable fixture cases match`` () =
        use fixtureDocument = JsonDocument.Parse(File.ReadAllText(findFixture "der-asn1-v1"))
        use upstreamDocument = JsonDocument.Parse(File.ReadAllText(findFixture "der-tlv-v1"))
        let fixture = fixtureDocument.RootElement
        let upstream = upstreamDocument.RootElement
        Assert.Equal(122, fixture.GetProperty("cases").GetArrayLength())
        Assert.Equal(22, fixture.GetProperty("error_ids").GetArrayLength())
        let references =
            fixture.GetProperty("cases").EnumerateArray()
            |> Seq.choose (fun testCase ->
                let mutable value = Unchecked.defaultof<JsonElement>
                if testCase.TryGetProperty("der_tlv_case_id", &value) then Some(requiredString testCase "der_tlv_case_id") else None)
            |> Seq.distinct
            |> Seq.length
        Assert.Equal(46, references)
        for testCase in fixture.GetProperty("cases").EnumerateArray() do
            let actual = runCase fixture upstream testCase
            let expected = JsonNode.Parse(testCase.GetProperty("expected").GetRawText())
            Assert.True(JsonNode.DeepEquals(expected, actual), sprintf "%s\nexpected=%O\nactual=%O" (requiredString testCase "id") expected actual)
            let mutable hostile = Unchecked.defaultof<JsonElement>
            if testCase.TryGetProperty("redacted_input_hex", &hostile) then
                Assert.DoesNotContain(requiredString testCase "redacted_input_hex", actual.ToJsonString(), StringComparison.OrdinalIgnoreCase)

    [<Fact>]
    let ``validated values are private immutable and payload blind`` () =
        Assert.Empty(typeof<Asn1Element>.GetConstructors())
        Assert.Empty(typeof<Asn1Cursor>.GetConstructors())
        Assert.Empty(typeof<DerInteger>.GetConstructors())
        Assert.Empty(typeof<DerBitString>.GetConstructors())
        Assert.Empty(typeof<ObjectIdentifier>.GetConstructors())
        Assert.Empty(typeof<Asn1Error>.GetConstructors())
        let input = Convert.FromHexString("060B2A81FFFFFFFFFFFFFFFF7F")
        let element = Asn1Decoder().DecodeExact(ReadOnlyMemory<byte>(input))
        let oid = DerAsn1.decodeObjectIdentifier element
        input[input.Length - 1] <- 1uy
        Assert.Equal(0x7fuy, element.Value.Span[element.Value.Length - 1])
        Assert.Equal<uint64>([| 1UL; 2UL; UInt64.MaxValue |], oid.Arcs)
        Assert.True(oid.EqualsArcs([| 1UL; 2UL; UInt64.MaxValue |]))
        Assert.False(oid.EqualsArcs([| 1UL; 2UL |]))
        let overlong = seq { yield 1UL; yield 2UL; yield UInt64.MaxValue; yield 4UL; failwith "over-enumerated" }
        Assert.False(oid.EqualsArcs(overlong))
        let nullArcs = Unchecked.defaultof<IEnumerable<uint64>>
        Assert.Throws<ArgumentNullException>(Action(fun () -> oid.EqualsArcs(nullArcs) |> ignore)) |> ignore
        let hostile = Asn1Decoder().DecodeExact(ReadOnlyMemory<byte>(Convert.FromHexString("160261FF")))
        let error = Assert.Throws<Asn1Error>(Action(fun () -> DerAsn1.decodeIa5String hostile |> ignore))
        Assert.DoesNotContain("61ff", error.ToString(), StringComparison.OrdinalIgnoreCase)
        Assert.DoesNotContain("255", error.ToString(), StringComparison.OrdinalIgnoreCase)

    [<Fact>]
    let ``limits cursor state and value helpers are checked`` () =
        Assert.Throws<ArgumentOutOfRangeException>(Action(fun () -> Asn1Limits(DerLimits.Default, -1, 1L, 1) |> ignore)) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(Action(fun () -> Asn1Limits(DerLimits.Default, 1, -1L, 1) |> ignore)) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(Action(fun () -> Asn1Limits(DerLimits.Default, 1, 1L, -1) |> ignore)) |> ignore
        let decoder = Asn1Decoder()
        let cursor = decoder.Sequence(decoder.DecodeExact(ReadOnlyMemory<byte>(Convert.FromHexString("30020500"))))
        Assert.NotNull(cursor.Read(Asn1Decoder(Asn1Limits.Default)))
        Assert.True(cursor.Remaining.IsEmpty)
        let integer = DerAsn1.decodeInteger (Asn1Decoder().DecodeExact(ReadOnlyMemory<byte>(Convert.FromHexString("020101"))))
        Assert.False(integer.IsNegative)
        Assert.Equal(1UL, integer.ToUInt64())
        let bits = DerAsn1.decodeBitString (Asn1Decoder().DecodeExact(ReadOnlyMemory<byte>(Convert.FromHexString("03020180"))))
        Assert.Equal<byte>([| 0x80uy |], bits.Bytes.ToArray())
