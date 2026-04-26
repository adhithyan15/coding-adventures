namespace CodingAdventures.BarcodeLayout1D.FSharp

open System
open System.Collections.Generic
open CodingAdventures.PaintInstructions

type Barcode1DRunColor =
    | Bar
    | Space
    member this.AsString =
        match this with
        | Bar -> "bar"
        | Space -> "space"

type Barcode1DRunRole =
    | Data
    | Start
    | Stop
    | Guard
    | Check
    | InterCharacterGap
    member this.AsString =
        match this with
        | Data -> "data"
        | Start -> "start"
        | Stop -> "stop"
        | Guard -> "guard"
        | Check -> "check"
        | InterCharacterGap -> "inter-character-gap"

type Barcode1DSymbolRole =
    | SymbolData
    | SymbolStart
    | SymbolStop
    | SymbolGuard
    | SymbolCheck
    member this.AsString =
        match this with
        | SymbolData -> "data"
        | SymbolStart -> "start"
        | SymbolStop -> "stop"
        | SymbolGuard -> "guard"
        | SymbolCheck -> "check"

type Barcode1DLayoutTarget =
    | NativePaintVm
    | CanvasPaintVm
    | DomPaintVm
    member this.AsString =
        match this with
        | NativePaintVm -> "native-paint-vm"
        | CanvasPaintVm -> "canvas-paint-vm"
        | DomPaintVm -> "dom-paint-vm"

type Barcode1DRun =
    {
        Color: Barcode1DRunColor
        Modules: uint32
        SourceLabel: string
        SourceIndex: int
        Role: Barcode1DRunRole
    }

type Barcode1DSymbolLayout =
    {
        Label: string
        StartModule: uint32
        EndModule: uint32
        SourceIndex: int
        Role: Barcode1DSymbolRole
    }

type Barcode1DLayout =
    {
        LeftQuietZoneModules: uint32
        RightQuietZoneModules: uint32
        ContentModules: uint32
        TotalModules: uint32
        SymbolLayouts: Barcode1DSymbolLayout list
    }

type Barcode1DSymbolDescriptor =
    {
        Label: string
        Modules: uint32
        SourceIndex: int
        Role: Barcode1DSymbolRole
    }

type Barcode1DRenderConfig =
    {
        LayoutTarget: Barcode1DLayoutTarget
        ModuleWidth: float
        BarHeight: float
        QuietZoneModules: uint32
        IncludeHumanReadableText: bool
        TextFontSize: float
        TextMargin: float
        Foreground: string
        Background: string
    }

type PaintBarcode1DOptions =
    {
        RenderConfig: Barcode1DRenderConfig
        HumanReadableText: string option
        Metadata: Metadata
        Label: string option
        Symbols: Barcode1DSymbolDescriptor list option
    }

type RunsFromBinaryPatternOptions =
    {
        SourceLabel: string
        SourceIndex: int
        Role: Barcode1DRunRole
    }

type RunsFromWidthPatternOptions =
    {
        SourceLabel: string
        SourceIndex: int
        Role: Barcode1DRunRole
        NarrowModules: uint32
        WideModules: uint32
        NarrowMarker: char
        WideMarker: char
        StartingColor: Barcode1DRunColor
    }

[<RequireQualifiedAccess>]
module BarcodeLayout1D =
    [<Literal>]
    let VERSION = "0.1.0"

    let emptyMetadata : Metadata = Dictionary<string, obj>() :> Metadata

    let defaultRenderConfig =
        {
            LayoutTarget = NativePaintVm
            ModuleWidth = 4.0
            BarHeight = 120.0
            QuietZoneModules = 10u
            IncludeHumanReadableText = false
            TextFontSize = 16.0
            TextMargin = 8.0
            Foreground = "#000000"
            Background = "#ffffff"
        }

    let defaultPaintOptions =
        {
            RenderConfig = defaultRenderConfig
            HumanReadableText = None
            Metadata = emptyMetadata
            Label = None
            Symbols = None
        }

    let defaultWidthPatternOptions sourceLabel sourceIndex role =
        {
            SourceLabel = sourceLabel
            SourceIndex = sourceIndex
            Role = role
            NarrowModules = 1u
            WideModules = 3u
            NarrowMarker = 'N'
            WideMarker = 'W'
            StartingColor = Bar
        }

    let totalModules (runs: seq<Barcode1DRun>) =
        if isNull (box runs) then nullArg "runs"
        runs |> Seq.sumBy _.Modules

    let private validateRuns (runs: Barcode1DRun list) =
        runs
        |> List.iteri (fun index run ->
            if run.Modules = 0u then
                invalidArg "runs" $"runs[{index}].modules must be greater than zero."
            if index > 0 && runs.[index - 1].Color = run.Color then
                invalidArg "runs" "Runs must alternate between bars and spaces.")

    let private toSymbolRole role =
        match role with
        | Data -> Some SymbolData
        | Start -> Some SymbolStart
        | Stop -> Some SymbolStop
        | Guard -> Some SymbolGuard
        | Check -> Some SymbolCheck
        | InterCharacterGap -> None

    let private inferSymbolLayouts (runs: Barcode1DRun list) =
        let layouts = ResizeArray<Barcode1DSymbolLayout>()
        let mutable cursor = 0u
        let mutable currentStart = 0u
        let mutable currentLabel: string option = None
        let mutable currentSourceIndex = 0
        let mutable currentRole: Barcode1DSymbolRole option = None

        let flush () =
            match currentLabel, currentRole with
            | Some label, Some role ->
                layouts.Add
                    {
                        Label = label
                        StartModule = currentStart
                        EndModule = cursor
                        SourceIndex = currentSourceIndex
                        Role = role
                    }
            | _ -> ()

        for run in runs do
            match toSymbolRole run.Role with
            | Some symbolRole ->
                let sameSymbol =
                    currentLabel = Some run.SourceLabel
                    && currentSourceIndex = run.SourceIndex
                    && currentRole = Some symbolRole

                if not sameSymbol then
                    flush ()
                    currentStart <- cursor
                    currentLabel <- Some run.SourceLabel
                    currentSourceIndex <- run.SourceIndex
                    currentRole <- Some symbolRole
            | None -> ()

            cursor <- cursor + run.Modules

        flush ()
        List.ofSeq layouts

    let private layoutExplicitSymbols (symbols: Barcode1DSymbolDescriptor list) contentModules =
        let layouts = ResizeArray<Barcode1DSymbolLayout>()
        let mutable cursor = 0u

        for symbol in symbols do
            if symbol.Modules = 0u then
                invalidArg "symbols" $"Symbol '{symbol.Label}' modules must be greater than zero."

            layouts.Add
                {
                    Label = symbol.Label
                    StartModule = cursor
                    EndModule = cursor + symbol.Modules
                    SourceIndex = symbol.SourceIndex
                    Role = symbol.Role
                }
            cursor <- cursor + symbol.Modules

        if cursor <> contentModules then
            invalidArg "symbols" "Symbol descriptors must add up to the same total width as the run stream."

        List.ofSeq layouts

    let computeBarcode1DLayout (runs: Barcode1DRun list) quietZoneModules (symbols: Barcode1DSymbolDescriptor list option) =
        if isNull (box runs) then nullArg "runs"
        validateRuns runs
        if quietZoneModules = 0u then
            invalidArg "quietZoneModules" "quiet_zone_modules must be greater than zero."

        let contentModules = totalModules runs
        let symbolLayouts =
            match symbols with
            | Some values -> layoutExplicitSymbols values contentModules
            | None -> inferSymbolLayouts runs

        {
            LeftQuietZoneModules = quietZoneModules
            RightQuietZoneModules = quietZoneModules
            ContentModules = contentModules
            TotalModules = quietZoneModules + contentModules + quietZoneModules
            SymbolLayouts = symbolLayouts
        }

    let runsFromBinaryPattern (pattern: string) (options: RunsFromBinaryPatternOptions) =
        if isNull pattern then nullArg "pattern"
        if pattern.Length = 0 then
            invalidArg "pattern" "Binary pattern must not be empty."
        if pattern |> Seq.exists (fun bit -> bit <> '0' && bit <> '1') then
            invalidArg "pattern" "Binary pattern must contain only 0 or 1."

        let runs = ResizeArray<Barcode1DRun>()
        let mutable currentBit = pattern.[0]
        let mutable width = 1u

        let addRun bit modules =
            runs.Add
                {
                    Color = if bit = '1' then Bar else Space
                    Modules = modules
                    SourceLabel = options.SourceLabel
                    SourceIndex = options.SourceIndex
                    Role = options.Role
                }

        for index in 1 .. pattern.Length - 1 do
            if pattern.[index] = currentBit then
                width <- width + 1u
            else
                addRun currentBit width
                currentBit <- pattern.[index]
                width <- 1u

        addRun currentBit width
        List.ofSeq runs

    let runsFromWidthPattern (pattern: string) (options: RunsFromWidthPatternOptions) =
        if isNull pattern then nullArg "pattern"
        if pattern.Length = 0 then
            invalidArg "pattern" "Width pattern must not be empty."
        if options.NarrowModules = 0u || options.WideModules = 0u then
            invalidArg "options" "Narrow and wide module counts must be greater than zero."

        let runs = ResizeArray<Barcode1DRun>()
        let mutable color = options.StartingColor

        for marker in pattern do
            let modules =
                if marker = options.NarrowMarker then options.NarrowModules
                elif marker = options.WideMarker then options.WideModules
                else invalidArg "pattern" $"Unknown width marker '{marker}'."

            runs.Add
                {
                    Color = color
                    Modules = modules
                    SourceLabel = options.SourceLabel
                    SourceIndex = options.SourceIndex
                    Role = options.Role
                }
            color <- if color = Bar then Space else Bar

        List.ofSeq runs

    let private validatePositive name value =
        if not (Double.IsFinite value) || value <= 0.0 then
            invalidArg name $"{name} must be a positive number."

    let private validateRenderConfig config =
        validatePositive "ModuleWidth" config.ModuleWidth
        validatePositive "BarHeight" config.BarHeight
        validatePositive "TextFontSize" config.TextFontSize
        if config.QuietZoneModules = 0u then
            invalidArg "RenderConfig" "Quiet zone modules must be greater than zero."
        if not (Double.IsFinite config.TextMargin) || config.TextMargin < 0.0 then
            invalidArg "RenderConfig" "Text margin must be zero or greater."

    let private copyMetadata (metadata: Metadata) =
        let result = Dictionary<string, obj>()
        for pair in metadata do
            result.[pair.Key] <- pair.Value
        result

    let layoutBarcode1D (runs: Barcode1DRun list) (options: PaintBarcode1DOptions option) =
        if isNull (box runs) then nullArg "runs"
        let options = defaultArg options defaultPaintOptions
        validateRenderConfig options.RenderConfig

        if options.RenderConfig.IncludeHumanReadableText then
            raise (NotSupportedException "Human-readable text shaping is not wired for dotnet barcode-layout-1d yet.")

        let layout = computeBarcode1DLayout runs options.RenderConfig.QuietZoneModules options.Symbols
        let instructions = ResizeArray<PaintInstruction>()
        let mutable moduleCursor = layout.LeftQuietZoneModules

        for run in runs do
            let x = float moduleCursor * options.RenderConfig.ModuleWidth
            let width = float run.Modules * options.RenderConfig.ModuleWidth
            if run.Color = Bar then
                let metadata =
                    let values = Dictionary<string, obj>()
                    values.["sourceLabel"] <- box run.SourceLabel
                    values.["sourceIndex"] <- box run.SourceIndex
                    values.["role"] <- box run.Role.AsString
                    values.["moduleStart"] <- box moduleCursor
                    values.["moduleEnd"] <- box (moduleCursor + run.Modules)
                    values :> Metadata

                instructions.Add(
                    PaintInstructions.paintRectWith
                        { PaintInstructions.defaultPaintRectOptions with
                            Fill = Some options.RenderConfig.Foreground
                            Metadata = Some metadata }
                        x
                        0.0
                        width
                        options.RenderConfig.BarHeight
                )

            moduleCursor <- moduleCursor + run.Modules

        let sceneWidth = float layout.TotalModules * options.RenderConfig.ModuleWidth
        let sceneHeight = options.RenderConfig.BarHeight
        let metadata = copyMetadata options.Metadata
        metadata.["label"] <- box (defaultArg options.Label "1D barcode")
        metadata.["leftQuietZoneModules"] <- box layout.LeftQuietZoneModules
        metadata.["rightQuietZoneModules"] <- box layout.RightQuietZoneModules
        metadata.["contentModules"] <- box layout.ContentModules
        metadata.["totalModules"] <- box layout.TotalModules
        metadata.["moduleWidthPx"] <- box options.RenderConfig.ModuleWidth
        metadata.["barHeightPx"] <- box options.RenderConfig.BarHeight
        metadata.["sceneWidthPx"] <- box sceneWidth
        metadata.["sceneHeightPx"] <- box sceneHeight
        metadata.["symbolCount"] <- box layout.SymbolLayouts.Length
        metadata.["layoutTarget"] <- box options.RenderConfig.LayoutTarget.AsString

        match options.HumanReadableText with
        | Some text -> metadata.["humanReadableText"] <- box text
        | None -> ()

        PaintInstructions.paintSceneWith
            { PaintInstructions.defaultSceneOptions with Metadata = Some (metadata :> Metadata) }
            sceneWidth
            sceneHeight
            options.RenderConfig.Background
            (List.ofSeq instructions)
