namespace CodingAdventures.X509Extension.FSharp.Tests

open System
open System.Globalization
open System.IO
open System.Text.Json
open System.Text.Json.Nodes
open CodingAdventures.DerTlv.FSharp
open CodingAdventures.DerAsn1.FSharp
open CodingAdventures.X509Extension.FSharp
open Xunit

module PortableConformanceTests =
    let private requiredString (element: JsonElement) (name: string) =
        element.GetProperty(name).GetString()
        |> Option.ofObj
        |> Option.defaultWith (fun () -> raise (InvalidDataException(sprintf "null %s" name)))

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

    let private limits (upstream: JsonElement) (testCase: JsonElement) =
        let defaults = upstream.GetProperty("defaults")
        let mutable outerNode = Unchecked.defaultof<JsonElement>
        let outer = if testCase.TryGetProperty("limits", &outerNode) then Some outerNode else None
        let mutable derNode = Unchecked.defaultof<JsonElement>
        let derOverride =
            match outer with
            | Some candidate when candidate.TryGetProperty("der", &derNode) -> Some derNode
            | _ -> None
        let derDefaults = defaults.GetProperty("der")
        let der = DerLimits(
            readInt64 derDefaults derOverride "max_input_len",
            readInt64 derDefaults derOverride "max_value_len",
            readInt64 derDefaults derOverride "max_elements",
            uint32 (readInt64 derDefaults derOverride "max_tag_number"))
        Asn1Limits(
            der,
            int (readInt64 defaults outer "max_depth"),
            readInt64 defaults outer "max_total_elements",
            int (readInt64 defaults outer "max_oid_arcs"))

    let private addString (value: JsonObject) (name: string) (content: string) = value[name] <- JsonValue.Create(content)
    let private addInt (value: JsonObject) (name: string) (content: int) = value[name] <- JsonValue.Create(content)
    let private addInt64 (value: JsonObject) (name: string) (content: int64) = value[name] <- JsonValue.Create(content)
    let private addBool (value: JsonObject) (name: string) (content: bool) = value[name] <- JsonValue.Create(content)

    let private attempt (decoder: Asn1Decoder) (root: Asn1Element) =
        try
            let extension = X509Extension.decodeX509Extension decoder root
            let result = JsonObject()
            addString result "outcome" "value"
            let arcs = JsonArray()
            extension.ExtensionId.Arcs
            |> Seq.iter (fun arc -> arcs.Add(JsonValue.Create(arc.ToString(CultureInfo.InvariantCulture))))
            result["extension_id_arcs_decimal"] <- arcs
            addBool result "critical" extension.Critical
            addString result "extension_value_hex" (Convert.ToHexString(extension.ExtensionValue.Span).ToLowerInvariant())
            addInt64 result "elements_read" decoder.ElementsRead
            result
        with :? X509ExtensionError as error ->
            let result = JsonObject()
            addString result "outcome" "error"
            addString result "error_id" error.KindId
            addInt result "offset" error.Offset
            addString result "offset_scope" "extension-element"
            addInt64 result "elements_read" decoder.ElementsRead
            match error.Asn1Kind with
            | Some kind -> addString result "asn1_error_id" (DerAsn1.errorId kind)
            | None -> ()
            match error.FramingKind with
            | Some kind -> addString result "framing_error_id" kind
            | None -> ()
            result

    [<Fact>]
    let ``all portable x509 extension cases match`` () =
        use contractDocument = JsonDocument.Parse(File.ReadAllText(findFixture "x509-extension-v1"))
        use upstreamDocument = JsonDocument.Parse(File.ReadAllText(findFixture "der-asn1-v1"))
        let contract = contractDocument.RootElement
        let upstream = upstreamDocument.RootElement
        Assert.Equal(48, contract.GetProperty("cases").GetArrayLength())
        Assert.Equal(8, contract.GetProperty("error_ids").GetArrayLength())
        for testCase in contract.GetProperty("cases").EnumerateArray() do
            let decoder = Asn1Decoder(limits upstream testCase)
            let root = decoder.DecodeExact(ReadOnlyMemory<byte>(materialize (testCase.GetProperty("input"))))
            let actual : JsonNode =
                if requiredString testCase "operation" = "extension-script" then
                    let result = JsonObject()
                    addString result "outcome" "script"
                    let events = JsonArray()
                    for _ in testCase.GetProperty("actions").EnumerateArray() do events.Add(attempt decoder root)
                    result["events"] <- events
                    result
                else attempt decoder root
            let expected = JsonNode.Parse(testCase.GetProperty("expected").GetRawText())
            Assert.True(
                JsonNode.DeepEquals(expected, actual),
                sprintf "%s\nexpected=%O\nactual=%O" (requiredString testCase "id") expected actual)
            let mutable hostile = Unchecked.defaultof<JsonElement>
            if testCase.TryGetProperty("redacted_input_hex", &hostile) then
                Assert.DoesNotContain(
                    requiredString testCase "redacted_input_hex",
                    actual.ToJsonString(),
                    StringComparison.OrdinalIgnoreCase)

    [<Fact>]
    let ``validated values are private immutable and payload blind`` () =
        Assert.Empty(typeof<X509ExtensionValue>.GetConstructors())
        Assert.Empty(typeof<X509ExtensionError>.GetConstructors())
        let input = Convert.FromHexString("30090603551D1104023000")
        let decoder = Asn1Decoder()
        let extension =
            decoder.DecodeExact(ReadOnlyMemory<byte>(input))
            |> X509Extension.decodeX509Extension decoder
        input[9] <- 0xffuy
        Assert.Equal("3000", Convert.ToHexString(extension.ExtensionValue.Span).ToLowerInvariant())
        let exposed = extension.ExtensionValue.ToArray()
        exposed[0] <- 0xffuy
        Assert.Equal("3000", Convert.ToHexString(extension.ExtensionValue.Span).ToLowerInvariant())

        let hostileDecoder = Asn1Decoder()
        let hostile = hostileDecoder.DecodeExact(ReadOnlyMemory<byte>(Convert.FromHexString("30080601800403DEADBE")))
        let error = Assert.Throws<X509ExtensionError>(Action(fun () ->
            X509Extension.decodeX509Extension hostileDecoder hostile |> ignore))
        Assert.Equal(InvalidExtensionId, error.Kind)
        Assert.Equal(Some NonMinimalObjectIdentifier, error.Asn1Kind)
        Assert.Equal(4, error.Offset)
        Assert.Equal(2L, hostileDecoder.ElementsRead)
        Assert.DoesNotContain("deadbe", error.ToString(), StringComparison.OrdinalIgnoreCase)
