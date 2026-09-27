namespace CodingAdventures.Itf.Tests

open System
open System.IO
open System.Security.Cryptography
open System.Text
open System.Text.Json
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.Itf.FSharp
open Xunit

module ItfTests =
    let private findFixture () =
        let rec search (directory: DirectoryInfo) =
            if isNull directory then
                raise (FileNotFoundException("barcode-symbologies-v1 cases.json was not found"))
            let candidate = Path.Combine(directory.FullName, "code", "specs", "fixtures", "barcode-symbologies-v1", "cases.json")
            if File.Exists candidate then candidate else search directory.Parent
        search (DirectoryInfo(AppContext.BaseDirectory))

    let private sha256 (value: string) =
        value
        |> Encoding.UTF8.GetBytes
        |> SHA256.HashData
        |> Convert.ToHexString
        |> fun value -> value.ToLowerInvariant()

    let private runLengths (modules: string) =
        modules
        |> Seq.fold (fun (previous, lengths) bit ->
            match previous, lengths with
            | Some prior, head :: tail when prior = bit -> Some bit, (head + 1) :: tail
            | _ -> Some bit, 1 :: lengths) (None, [])
        |> snd
        |> List.rev

    [<Fact>]
    let ``shared ITF corpus conforms`` () =
        use document = JsonDocument.Parse(File.ReadAllText(findFixture ()))
        document.RootElement.GetProperty("cases").EnumerateArray()
        |> Seq.filter (fun testCase -> testCase.GetProperty("symbology").GetString() = "itf")
        |> Seq.iter (fun testCase ->
            let inputSpec = testCase.GetProperty("input")
            let mutable text = Unchecked.defaultof<JsonElement>
            let input =
                if inputSpec.TryGetProperty("text", &text) then text.GetString()
                else
                    let repeat = inputSpec.GetProperty("repeat")
                    String.replicate (repeat.GetProperty("count").GetInt32()) (repeat.GetProperty("text").GetString())
            let expected = testCase.GetProperty("expected")
            let mutable error = Unchecked.defaultof<JsonElement>
            if expected.TryGetProperty("error", &error) then
                let caught = Assert.Throws<InvalidItfInputException>(fun () -> Itf.normalizeItf input |> ignore)
                Assert.Equal(Some(error.GetString()), Itf.errorId (caught :> exn))
            else
                let normalized = Itf.normalizeItf input
                let modules = "1010" + (Itf.encodeItf input |> List.map _.BinaryPattern |> String.concat "") + "11101"
                let runs = runLengths modules
                let mutable normalizedValue = Unchecked.defaultof<JsonElement>
                if expected.TryGetProperty("normalized", &normalizedValue) then
                    Assert.Equal(normalizedValue.GetString(), normalized)
                    Assert.Equal(expected.GetProperty("modules").GetString(), modules)
                    Assert.Equal<int list>(expected.GetProperty("run_lengths").EnumerateArray() |> Seq.map _.GetInt32() |> List.ofSeq, runs)
                else
                    Assert.Equal(expected.GetProperty("normalized_sha256").GetString(), sha256 normalized)
                    Assert.Equal(expected.GetProperty("run_count").GetInt32(), runs.Length)
                    Assert.Equal(expected.GetProperty("run_lengths_sha256").GetString(), sha256 (JsonSerializer.Serialize runs))
                Assert.Equal(expected.GetProperty("module_count").GetInt32(), modules.Length)
                Assert.Equal(expected.GetProperty("module_sha256").GetString(), sha256 modules))

    [<Fact>]
    let ``version exists`` () =
        Assert.Equal("0.1.0", Itf.VERSION)

    [<Fact>]
    let ``normalize accepts even length digit strings`` () =
        Assert.Equal("123456", Itf.normalizeItf "123456")

    [<Fact>]
    let ``normalize rejects invalid input`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Itf.normalizeItf Unchecked.defaultof<string> |> ignore) |> ignore
        Assert.Throws<InvalidItfInputException>(fun () -> Itf.normalizeItf "" |> ignore) |> ignore
        Assert.Throws<InvalidItfInputException>(fun () -> Itf.normalizeItf "12345" |> ignore) |> ignore
        Assert.Throws<InvalidItfInputException>(fun () -> Itf.normalizeItf "12A4" |> ignore) |> ignore

    [<Fact>]
    let ``encode encodes digit pairs`` () =
        let encoded = Itf.encodeItf "123456"

        Assert.Equal(3, encoded.Length)
        Assert.Equal("12", encoded[0].Pair)
        Assert.Equal("10001", encoded[0].BarPattern)
        Assert.Equal("01001", encoded[0].SpacePattern)
        Assert.Equal(0, encoded[0].SourceIndex)
        Assert.NotEmpty(encoded[0].BinaryPattern)

    [<Fact>]
    let ``expand runs includes start and stop patterns`` () =
        let runs = Itf.expandItfRuns "123456"

        Assert.Equal("start", runs[0].SourceLabel)
        Assert.Equal(Start, runs[0].Role)
        Assert.Equal("stop", runs[runs.Length - 1].SourceLabel)
        Assert.Equal(Stop, runs[runs.Length - 1].Role)
        Assert.Contains(runs, fun run -> run.Role = Data && run.SourceLabel = "12")

    [<Fact>]
    let ``draw returns barcode scene`` () =
        let scene = Itf.drawItf "123456" None

        Assert.Equal(box "itf", scene.Metadata.Value["symbology"])
        Assert.Equal(box 3, scene.Metadata.Value["pairCount"])
        Assert.True(scene.Width > 0.0)
        Assert.Equal(Itf.defaultRenderConfig.BarHeight, scene.Height)

    [<Fact>]
    let ``invalid layout config is rejected`` () =
        let badOptions =
            { BarcodeLayout1D.defaultPaintOptions with
                RenderConfig = { BarcodeLayout1D.defaultRenderConfig with ModuleWidth = 0.0 } }

        Assert.Throws<ArgumentException>(fun () -> Itf.layoutItf "123456" (Some badOptions) |> ignore) |> ignore
