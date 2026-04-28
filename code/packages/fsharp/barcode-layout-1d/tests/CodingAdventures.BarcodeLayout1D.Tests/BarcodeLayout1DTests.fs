namespace CodingAdventures.BarcodeLayout1D.Tests

open System
open Xunit
open CodingAdventures.PaintInstructions
open CodingAdventures.BarcodeLayout1D.FSharp

module BarcodeLayout1DTests =
    [<Fact>]
    let ``binary pattern expands to runs`` () =
        let runs =
            BarcodeLayout1D.runsFromBinaryPattern
                "11001"
                { SourceLabel = "start"; SourceIndex = -1; Role = Guard }

        Assert.Equal(3, runs.Length)
        Assert.Equal(Bar, runs.[0].Color)
        Assert.Equal(2u, runs.[0].Modules)
        Assert.Equal(Space, runs.[1].Color)
        Assert.Equal(1u, runs.[2].Modules)
        Assert.Throws<ArgumentException>(fun () ->
            BarcodeLayout1D.runsFromBinaryPattern "10x" { SourceLabel = "bad"; SourceIndex = 0; Role = Data } |> ignore)
        |> ignore

    [<Fact>]
    let ``width pattern expands to runs`` () =
        let runs =
            BarcodeLayout1D.runsFromWidthPattern
                "NWN"
                (BarcodeLayout1D.defaultWidthPatternOptions "A" 0 Data)

        Assert.Equal(3, runs.Length)
        Assert.Equal(1u, runs.[0].Modules)
        Assert.Equal(3u, runs.[1].Modules)
        Assert.Equal(Bar, runs.[2].Color)
        Assert.Throws<ArgumentException>(fun () ->
            BarcodeLayout1D.runsFromWidthPattern "NX" (BarcodeLayout1D.defaultWidthPatternOptions "A" 0 Data) |> ignore)
        |> ignore

    [<Fact>]
    let ``computes quiet zone aware layout`` () =
        let runs =
            [
                { Color = Bar; Modules = 1u; SourceLabel = "*"; SourceIndex = 0; Role = Start }
                { Color = Space; Modules = 1u; SourceLabel = "*"; SourceIndex = 0; Role = InterCharacterGap }
                { Color = Bar; Modules = 2u; SourceLabel = "A"; SourceIndex = 1; Role = Data }
            ]

        let layout = BarcodeLayout1D.computeBarcode1DLayout runs 10u None

        Assert.Equal(4u, layout.ContentModules)
        Assert.Equal(24u, layout.TotalModules)
        Assert.Equal(2, layout.SymbolLayouts.Length)
        Assert.Equal("*", layout.SymbolLayouts.[0].Label)
        Assert.Equal(2u, layout.SymbolLayouts.[0].EndModule)

    [<Fact>]
    let ``explicit symbols must match run width`` () =
        let runs = BarcodeLayout1D.runsFromBinaryPattern "101" { SourceLabel = "demo"; SourceIndex = 0; Role = Guard }
        let symbols = [ { Label = "demo"; Modules = 3u; SourceIndex = 0; Role = SymbolGuard } ]

        let layout = BarcodeLayout1D.computeBarcode1DLayout runs 10u (Some symbols)
        Assert.Single(layout.SymbolLayouts) |> ignore
        Assert.Throws<ArgumentException>(fun () ->
            BarcodeLayout1D.computeBarcode1DLayout runs 10u (Some [ { Label = "bad"; Modules = 2u; SourceIndex = 0; Role = SymbolData } ])
            |> ignore)
        |> ignore

    [<Fact>]
    let ``lays out runs into paint scene`` () =
        let runs = BarcodeLayout1D.runsFromBinaryPattern "101" { SourceLabel = "demo"; SourceIndex = 0; Role = Guard }
        let scene =
            BarcodeLayout1D.layoutBarcode1D
                runs
                (Some { BarcodeLayout1D.defaultPaintOptions with Label = Some "Demo barcode" })

        Assert.Equal("#ffffff", scene.Background)
        Assert.Equal(2, scene.Instructions.Length)
        Assert.All<PaintInstruction>(scene.Instructions, fun instruction ->
            match instruction with
            | Rect _ -> ()
            | _ -> failwith "expected rect")

        Assert.Equal(box "Demo barcode", scene.Metadata.Value.["label"])
        Assert.Equal(box 23u, scene.Metadata.Value.["totalModules"])

    [<Fact>]
    let ``validates layout and render configuration`` () =
        Assert.Throws<ArgumentException>(fun () ->
            BarcodeLayout1D.computeBarcode1DLayout
                [
                    { Color = Bar; Modules = 1u; SourceLabel = "a"; SourceIndex = 0; Role = Data }
                    { Color = Bar; Modules = 1u; SourceLabel = "b"; SourceIndex = 1; Role = Data }
                ]
                10u
                None
            |> ignore)
        |> ignore

        let runs = BarcodeLayout1D.runsFromBinaryPattern "101" { SourceLabel = "demo"; SourceIndex = 0; Role = Guard }

        Assert.Throws<NotSupportedException>(fun () ->
            BarcodeLayout1D.layoutBarcode1D
                runs
                (Some
                    { BarcodeLayout1D.defaultPaintOptions with
                        RenderConfig = { BarcodeLayout1D.defaultRenderConfig with IncludeHumanReadableText = true }
                        HumanReadableText = Some "demo" })
            |> ignore)
        |> ignore

        Assert.Throws<ArgumentException>(fun () ->
            BarcodeLayout1D.layoutBarcode1D
                runs
                (Some
                    { BarcodeLayout1D.defaultPaintOptions with
                        RenderConfig = { BarcodeLayout1D.defaultRenderConfig with ModuleWidth = 0.0 } })
            |> ignore)
        |> ignore
