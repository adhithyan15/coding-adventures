namespace CodingAdventures.BarcodeLayout1D.Tests

open System
open System.Collections.Generic
open System.IO
open System.Security.Cryptography
open System.Text
open System.Text.Encodings.Web
open System.Text.Json
open System.Text.Json.Nodes
open Xunit
open CodingAdventures.PaintInstructions
open CodingAdventures.BarcodeLayout1D.FSharp

module BarcodeLayout1DConformanceTests =
    let corpusSha = "be95aa0381041ef3bd729b36bb4292f7a20692e139b53adca97af4e157cb7388"

    let findCorpus () =
        let rec search (directory: DirectoryInfo) =
            if isNull directory then raise (FileNotFoundException "barcode-layout-1d-v1/cases.json")
            let candidate = Path.Combine(directory.FullName, "code", "specs", "fixtures", "barcode-layout-1d-v1", "cases.json")
            if File.Exists candidate then candidate else search directory.Parent
        search (DirectoryInfo AppContext.BaseDirectory)

    let tryProperty (name: string) (value: JsonElement) =
        let mutable found = Unchecked.defaultof<JsonElement>
        if value.TryGetProperty(name, &found) then Some found else None

    let requiredString (name: string) (value: JsonElement) = value.GetProperty(name).GetString()
    let requiredInt (name: string) (value: JsonElement) = value.GetProperty(name).GetInt64()
    let optionalString name fallback value =
        tryProperty name value |> Option.map _.GetString() |> Option.defaultValue fallback
    let optionalInt name fallback value =
        tryProperty name value |> Option.map _.GetInt64() |> Option.defaultValue fallback

    let fixtureCount (name: string) (limit: int) (errorId: string) (value: JsonElement) =
        let property = value.GetProperty name
        let mutable count = 0L
        if property.ValueKind <> JsonValueKind.Number || not (property.TryGetInt64(&count)) || count < 0L then
            raise (InvalidDataException "fixture-invalid-count")
        if count > int64 limit then raise (Barcode1DV1Exception errorId)
        int count

    let color value = if value = "bar" then Bar else Space
    let toggle value = if value = Bar then Space else Bar
    let runRole value =
        match value with
        | "data" -> Data | "start" -> Start | "stop" -> Stop | "guard" -> Guard
        | "check" -> Check | _ -> InterCharacterGap
    let symbolRole value =
        match value with
        | "data" -> SymbolData | "start" -> SymbolStart | "stop" -> SymbolStop
        | "guard" -> SymbolGuard | _ -> SymbolCheck

    let pattern input =
        match tryProperty "pattern" input with
        | Some value -> value.GetString()
        | None ->
            let repeat = input.GetProperty "repeat"
            let token = requiredString "token" repeat
            let suffix = optionalString "suffix" "" repeat
            let count = fixtureCount "count" 65568 "pattern-too-long" repeat
            let tokenCount = token.EnumerateRunes() |> Seq.length
            let suffixCount = suffix.EnumerateRunes() |> Seq.length
            if tokenCount > 0 && count > (65568 - suffixCount) / tokenCount then
                raise (Barcode1DV1Exception "pattern-too-long")
            let scalarCount = tokenCount * count + suffixCount
            if scalarCount > 65567 then raise (Barcode1DV1Exception "pattern-too-long")
            String.replicate count token + suffix

    let run (value: JsonElement) : Barcode1DV1Run =
        { Color = color (requiredString "color" value)
          Modules = requiredInt "modules" value
          SourceLabel = requiredString "sourceLabel" value
          SourceIndex = requiredInt "sourceIndex" value
          Role = runRole (requiredString "role" value) }

    let runs input =
        match tryProperty "runs" input with
        | Some values ->
            if values.ValueKind <> JsonValueKind.Array then raise (InvalidDataException "fixture-invalid-type")
            if values.GetArrayLength() > 40979 then raise (Barcode1DV1Exception "too-many-runs")
            values.EnumerateArray() |> Seq.map run |> List.ofSeq
        | None ->
            let repeat = input.GetProperty "repeatRuns"
            let count = fixtureCount "count" 40979 "too-many-runs" repeat
            let first = color (requiredString "firstColor" repeat)
            [ for index in 0 .. count - 1 ->
                { Color = if index % 2 = 0 then first else toggle first
                  Modules = requiredInt "modules" repeat
                  SourceLabel = requiredString "sourceLabel" repeat
                  SourceIndex = requiredInt "sourceIndex" repeat
                  Role = runRole (requiredString "role" repeat) } ]

    let symbol (value: JsonElement) : Barcode1DV1SymbolDescriptor =
        { Label = requiredString "label" value
          Modules = requiredInt "modules" value
          SourceIndex = requiredInt "sourceIndex" value
          Role = symbolRole (requiredString "role" value) }

    let symbols (input: JsonElement) : Barcode1DV1SymbolDescriptor list option =
        match tryProperty "symbols" input, tryProperty "repeatSymbols" input with
        | Some values, _ ->
            if values.ValueKind <> JsonValueKind.Array then raise (InvalidDataException "fixture-invalid-type")
            if values.GetArrayLength() > 40979 then raise (Barcode1DV1Exception "too-many-symbols")
            values.EnumerateArray() |> Seq.map symbol |> List.ofSeq |> Some
        | None, Some repeat ->
            let count = fixtureCount "count" 40979 "too-many-symbols" repeat
            [ for index in 0 .. count - 1 ->
                ({ Label = requiredString "label" repeat
                   Modules = requiredInt "modules" repeat
                   SourceIndex = int64 index
                   Role = symbolRole (requiredString "role" repeat) } : Barcode1DV1SymbolDescriptor) ] |> Some
        | _ -> None

    let widthOptions input =
        { BarcodeLayout1DV1.defaultWidthOptions
            (requiredString "sourceLabel" input) (requiredInt "sourceIndex" input)
            (runRole (requiredString "role" input)) with
            NarrowMarker = optionalString "narrowMarker" "N" input
            WideMarker = optionalString "wideMarker" "W" input
            NarrowModules = optionalInt "narrowModules" 1L input
            WideModules = optionalInt "wideModules" 3L input
            StartingColor = color (optionalString "startingColor" "bar" input) }

    let sceneOptions input =
        let render = tryProperty "renderConfig" input
        let metadata = Dictionary<string, string>()
        match tryProperty "metadata" input with
        | Some values -> for pair in values.EnumerateObject() do metadata[pair.Name] <- pair.Value.GetString()
        | None -> ()
        let renderConfig =
            match render with
            | Some value ->
                { ModuleWidth = optionalInt "moduleWidth" 4L value
                  BarHeight = optionalInt "barHeight" 120L value
                  Foreground = optionalString "foreground" "#000000" value
                  Background = optionalString "background" "#ffffff" value
                  IncludeHumanReadableText =
                    tryProperty "includeHumanReadableText" value
                    |> Option.map _.GetBoolean() |> Option.defaultValue false }
            | None -> BarcodeLayout1DV1.defaultRenderConfig
        { QuietZoneModules = requiredInt "quietZoneModules" input
          RenderConfig = renderConfig
          HumanReadableText =
            match tryProperty "humanReadableText" input with
            | Some value when value.ValueKind <> JsonValueKind.Null -> Some (value.GetString())
            | _ -> None
          Label = optionalString "label" "1D barcode" input
          Metadata = metadata :> IReadOnlyDictionary<string, string>
          Symbols = symbols input }

    let dict pairs =
        let result = Dictionary<string, obj>()
        for key, value in pairs do result[key] <- value
        result

    let runObject (value: Barcode1DV1Run) =
        dict [ "color", box value.Color.AsString; "modules", box value.Modules
               "sourceLabel", box value.SourceLabel; "sourceIndex", box value.SourceIndex
               "role", box value.Role.AsString ]

    let runResult values =
        if List.length values <= 1000 then
            dict [ "runs", box (values |> List.map runObject) ]
        else
            let canonical =
                values
                |> List.map (fun value ->
                    let item = SortedDictionary<string, obj>(StringComparer.Ordinal)
                    item["color"] <- box value.Color.AsString
                    item["modules"] <- box value.Modules
                    item["role"] <- box value.Role.AsString
                    item["sourceIndex"] <- box value.SourceIndex
                    item["sourceLabel"] <- box value.SourceLabel
                    item)
            let options = JsonSerializerOptions(Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping)
            let encoded = JsonSerializer.SerializeToUtf8Bytes(canonical, options)
            dict [ "runDigest", box (dict [
                "runCount", box values.Length
                "contentModules", box (values |> List.sumBy _.Modules)
                "firstRun", box (runObject values.Head)
                "lastRun", box (runObject values[values.Length - 1])
                "runsSha256", box (Convert.ToHexString(SHA256.HashData encoded).ToLowerInvariant()) ]) ]

    let layoutObject (layout: Barcode1DV1Layout) =
        dict [ "leftQuietZoneModules", box layout.LeftQuietZoneModules
               "rightQuietZoneModules", box layout.RightQuietZoneModules
               "contentModules", box layout.ContentModules
               "totalModules", box layout.TotalModules
               "symbolLayouts", box (layout.SymbolLayouts |> List.map (fun symbol ->
                   dict [ "label", box symbol.Label; "startModule", box symbol.StartModule
                          "endModule", box symbol.EndModule; "sourceIndex", box symbol.SourceIndex
                          "role", box symbol.Role.AsString ])) ]

    let metadataObject (metadata: Metadata) =
        let result = SortedDictionary<string, obj>(StringComparer.Ordinal)
        for pair in metadata do result[pair.Key] <- box (Assert.IsType<string> pair.Value)
        result

    let sceneObject (scene: PaintScene) =
        let rectangles =
            scene.Instructions
            |> List.map (fun instruction ->
                match instruction with
                | Rect rect ->
                    dict [ "x", box (int64 rect.X); "y", box (int64 rect.Y)
                           "width", box (int64 rect.Width); "height", box (int64 rect.Height)
                           "fill", box rect.Fill.Value
                           "metadata", box (metadataObject rect.Base.Metadata.Value) ]
                | _ -> failwith "expected rect")
        dict [ "width", box (int64 scene.Width); "height", box (int64 scene.Height)
               "background", box scene.Background; "rectangles", box rectangles
               "metadata", box (metadataObject scene.Metadata.Value) ]

    let dispatch (item: JsonElement) =
        let input = item.GetProperty "input"
        match item.GetProperty("operation").GetString() with
        | "expand-binary" ->
            let values = BarcodeLayout1DV1.expandBinary (pattern input)
                            { SourceLabel = requiredString "sourceLabel" input
                              SourceIndex = requiredInt "sourceIndex" input
                              Role = runRole (requiredString "role" input) }
            runResult values
        | "expand-width" -> BarcodeLayout1DV1.expandWidth (pattern input) (widthOptions input) |> runResult
        | "compute-layout" ->
            dict [ "layout", box (BarcodeLayout1DV1.computeLayout (runs input)
                                    (requiredInt "quietZoneModules" input) (symbols input) |> layoutObject) ]
        | "project-scene" ->
            dict [ "scene", box (BarcodeLayout1DV1.projectScene (runs input) (Some (sceneOptions input)) |> sceneObject) ]
        | operation -> invalidOp operation

    let readBoundedFixture path =
        use stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read)
        if stream.Length < 1L || stream.Length > 131072L then
            raise (InvalidDataException "fixture-size-limit")
        let encoded = Array.zeroCreate<byte> (int stream.Length)
        stream.ReadExactly encoded
        encoded

    let validateScalarString (value: string) =
        let mutable remaining = value.AsSpan()
        while not remaining.IsEmpty do
            let mutable rune = Unchecked.defaultof<Rune>
            let mutable consumed = 0
            let status = Rune.DecodeFromUtf16(remaining, &rune, &consumed)
            if status <> System.Buffers.OperationStatus.Done then raise (InvalidDataException "fixture-invalid-scalar")
            remaining <- remaining.Slice consumed

    let rec validateFixtureElement depth (value: JsonElement) =
        if depth > 8 then raise (InvalidDataException "fixture-depth-limit")
        match value.ValueKind with
        | JsonValueKind.Object ->
            let names = HashSet<string>(StringComparer.Ordinal)
            for property in value.EnumerateObject() do
                validateScalarString property.Name
                if not (names.Add property.Name) then raise (InvalidDataException "fixture-duplicate-key")
                validateFixtureElement (depth + 1) property.Value
        | JsonValueKind.Array ->
            for item in value.EnumerateArray() do validateFixtureElement (depth + 1) item
        | JsonValueKind.String -> validateScalarString (value.GetString())
        | JsonValueKind.Number ->
            let mutable number = 0L
            if not (value.TryGetInt64(&number)) then raise (InvalidDataException "fixture-invalid-number")
        | JsonValueKind.True | JsonValueKind.False | JsonValueKind.Null -> ()
        | _ -> raise (InvalidDataException "fixture-invalid-type")

    [<Fact>]
    let ``executes every v1 corpus case`` () =
        let encoded = readBoundedFixture (findCorpus())
        Assert.Equal(corpusSha, Convert.ToHexString(SHA256.HashData encoded).ToLowerInvariant())
        use document = JsonDocument.Parse(encoded, JsonDocumentOptions(MaxDepth = 8))
        validateFixtureElement 0 document.RootElement
        Assert.Equal(1L, requiredInt "schema_version" document.RootElement)
        Assert.Equal("barcode-layout-1d-v1", requiredString "profile" document.RootElement)
        let cases = document.RootElement.GetProperty("cases").EnumerateArray() |> Seq.toArray
        Assert.Equal(56, cases.Length)
        for item in cases do
            let id = requiredString "id" item
            let expected = item.GetProperty "expected"
            try
                let actual = dispatch item |> JsonSerializer.SerializeToNode
                Assert.True((tryProperty "error" expected).IsNone, $"{id}: expected error")
                let expectedNode = JsonNode.Parse(expected.GetRawText())
                Assert.True(JsonNode.DeepEquals(expectedNode, actual), $"{id}\nexpected: {expectedNode}\nactual: {actual}")
            with
            | :? Barcode1DV1Exception as error ->
                match tryProperty "error" expected with
                | Some expectedError -> Assert.Equal(expectedError.GetString(), error.ErrorId)
                | None -> Assert.Fail($"{id}: unexpected {error.ErrorId}")

    [<Fact>]
    let ``text requests fail before native resolution`` () =
        let values: Barcode1DV1Run list =
            [ { Color = Bar; Modules = 1L; SourceLabel = "A"; SourceIndex = 0L; Role = Data } ]
        let mutable calls = 0
        let options =
            [ { BarcodeLayout1DV1.defaultSceneOptions () with HumanReadableText = Some "A" }
              { BarcodeLayout1DV1.defaultSceneOptions () with
                    RenderConfig = { BarcodeLayout1DV1.defaultRenderConfig with IncludeHumanReadableText = true; ModuleWidth = 0L } } ]
        for item in options do
            let error = Assert.Throws<Barcode1DV1Exception>(fun () ->
                BarcodeLayout1DV1.projectSceneWithProbeForTests values (Some item) (Action(fun () -> calls <- calls + 1)) |> ignore)
            Assert.Equal("human-readable-text-unsupported", error.ErrorId)
        Assert.Equal(0, calls)

    [<Fact>]
    let ``results and metadata are deep owned`` () =
        let caller = Dictionary<string, string>()
        caller["owner"] <- "caller"
        let values = BarcodeLayout1DV1.expandBinary "101" { SourceLabel = "A"; SourceIndex = 0L; Role = Data }
        let first = BarcodeLayout1DV1.projectScene values
                        (Some { BarcodeLayout1DV1.defaultSceneOptions () with Metadata = caller })
        caller["owner"] <- "changed"
        Assert.Equal(box "caller", first.Metadata.Value["owner"])
        let second = BarcodeLayout1DV1.projectScene values
                        (Some { BarcodeLayout1DV1.defaultSceneOptions () with
                                    Metadata = Dictionary<string, string>(dict [ "owner", box "caller" ] |> Seq.map (fun p -> KeyValuePair(p.Key, unbox<string> p.Value))) })
        Assert.NotSame(first.Metadata.Value, second.Metadata.Value)
        Assert.NotSame(first.Instructions, second.Instructions)

    [<Fact>]
    let ``default scene options own fresh metadata`` () =
        let first = BarcodeLayout1DV1.defaultSceneOptions ()
        let second = BarcodeLayout1DV1.defaultSceneOptions ()
        Assert.NotSame(first.Metadata, second.Metadata)

    [<Fact>]
    let ``pattern scanner stops at the normative limit`` () =
        let pattern = String('1', 65568) + "\uD800"
        let error = Assert.Throws<Barcode1DV1Exception>(fun () ->
            BarcodeLayout1DV1.expandBinary pattern
                { SourceLabel = "A"; SourceIndex = 0L; Role = Data } |> ignore)
        Assert.Equal("pattern-too-long", error.ErrorId)

    [<Fact>]
    let ``fixture materializers reject invalid repeat counts before allocation`` () =
        use negative = JsonDocument.Parse("{\"repeat\":{\"token\":\"1\",\"count\":-1}}")
        Assert.Throws<InvalidDataException>(fun () -> pattern negative.RootElement |> ignore) |> ignore
        use tooManyRuns = JsonDocument.Parse("{\"repeatRuns\":{\"count\":40980}}")
        let runError = Assert.Throws<Barcode1DV1Exception>(fun () -> runs tooManyRuns.RootElement |> ignore)
        Assert.Equal("too-many-runs", runError.ErrorId)
        use tooManySymbols = JsonDocument.Parse("{\"repeatSymbols\":{\"count\":40980}}")
        let symbolError = Assert.Throws<Barcode1DV1Exception>(fun () -> symbols tooManySymbols.RootElement |> ignore)
        Assert.Equal("too-many-symbols", symbolError.ErrorId)

    [<Fact>]
    let ``fixture validator rejects duplicate keys`` () =
        use duplicate = JsonDocument.Parse("{\"a\":1,\"a\":2}")
        Assert.Throws<InvalidDataException>(fun () -> validateFixtureElement 0 duplicate.RootElement) |> ignore
