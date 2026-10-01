namespace CodingAdventures.BarcodeLayout1D.FSharp

open System
open System.Collections.Generic
open System.Globalization
open System.Text
open CodingAdventures.PaintInstructions

[<Sealed>]
type Barcode1DV1Exception(errorId: string) =
    inherit ArgumentException(errorId)
    member _.ErrorId = errorId

type Barcode1DV1Run =
    { Color: Barcode1DRunColor
      Modules: int64
      SourceLabel: string
      SourceIndex: int64
      Role: Barcode1DRunRole }

type Barcode1DV1SymbolDescriptor =
    { Label: string
      Modules: int64
      SourceIndex: int64
      Role: Barcode1DSymbolRole }

type Barcode1DV1SymbolLayout =
    { Label: string
      StartModule: int64
      EndModule: int64
      SourceIndex: int
      Role: Barcode1DSymbolRole }

type Barcode1DV1Layout =
    { LeftQuietZoneModules: int64
      RightQuietZoneModules: int64
      ContentModules: int64
      TotalModules: int64
      SymbolLayouts: Barcode1DV1SymbolLayout list }

type Barcode1DV1BinaryOptions =
    { SourceLabel: string
      SourceIndex: int64
      Role: Barcode1DRunRole }

type Barcode1DV1WidthOptions =
    { SourceLabel: string
      SourceIndex: int64
      Role: Barcode1DRunRole
      NarrowMarker: string
      WideMarker: string
      NarrowModules: int64
      WideModules: int64
      StartingColor: Barcode1DRunColor }

type Barcode1DV1RenderConfig =
    { ModuleWidth: int64
      BarHeight: int64
      Foreground: string
      Background: string
      IncludeHumanReadableText: bool }

type Barcode1DV1SceneOptions =
    { QuietZoneModules: int64
      RenderConfig: Barcode1DV1RenderConfig
      HumanReadableText: string option
      Label: string
      Metadata: IReadOnlyDictionary<string, string>
      Symbols: Barcode1DV1SymbolDescriptor list option }

[<RequireQualifiedAccess>]
module BarcodeLayout1DV1 =
    let private maxPatternScalars = 65567
    let private maxRuns = 40979
    let private maxContentModules = 65567L
    let private maxQuietZoneModules = 4096L
    let private maxSymbols = 40979
    let private maxLabelScalars = 4096
    let private maxMetadataEntries = 64
    let private maxMetadataKeyScalars = 128
    let private maxMetadataValueScalars = 4096
    let private maxMetadataBytes = 65536L
    let private maxRenderDimension = 8192L
    let private maxColorScalars = 128

    let private fail id = raise (Barcode1DV1Exception id)

    let private decodeScalars error limit limitError (value: string) =
        if isNull value then fail error
        let result = ResizeArray<Rune>()
        let mutable offset = 0
        while offset < value.Length do
            let mutable rune = Unchecked.defaultof<Rune>
            if not (Rune.TryGetRuneAt(value, offset, &rune)) then fail error
            result.Add rune
            if result.Count > limit then fail limitError
            offset <- offset + rune.Utf16SequenceLength
        List.ofSeq result

    let private scalarCount error limit limitError value =
        decodeScalars error limit limitError value |> List.length

    let private singleScalar value =
        try
            match decodeScalars "invalid-marker-configuration" 1 "invalid-marker-configuration" value with
            | [ rune ] -> Some rune
            | _ -> None
        with :? Barcode1DV1Exception -> None

    let private validateSource label sourceIndex =
        scalarCount "invalid-source-attribution" maxLabelScalars "invalid-source-attribution" label |> ignore
        if sourceIndex < int64 Int32.MinValue || sourceIndex > int64 Int32.MaxValue then
            fail "invalid-source-attribution"

    let private validateColor color =
        match color with
        | Bar | Space -> ()

    let private validateRunRole role =
        match role with
        | Data | Start | Stop | Guard | Check | InterCharacterGap -> ()

    let private checkedAdd error left right =
        if right > 0L && left > Int64.MaxValue - right then fail error
        if right < 0L && left < Int64.MinValue - right then fail error
        left + right

    let private checkedMultiply error left right =
        if left <> 0L && right > Int64.MaxValue / left then fail error
        left * right

    let private decimal (value: int64) = value.ToString(CultureInfo.InvariantCulture)

    let defaultWidthOptions sourceLabel sourceIndex role =
        { SourceLabel = sourceLabel
          SourceIndex = sourceIndex
          Role = role
          NarrowMarker = "N"
          WideMarker = "W"
          NarrowModules = 1L
          WideModules = 3L
          StartingColor = Bar }

    let defaultRenderConfig =
        { ModuleWidth = 4L
          BarHeight = 120L
          Foreground = "#000000"
          Background = "#ffffff"
          IncludeHumanReadableText = false }

    let defaultSceneOptions () =
        { QuietZoneModules = 10L
          RenderConfig = defaultRenderConfig
          HumanReadableText = None
          Label = "1D barcode"
          Metadata = Dictionary<string, string>() :> IReadOnlyDictionary<string, string>
          Symbols = None }

    let expandBinary (pattern: string) (options: Barcode1DV1BinaryOptions) =
        if isNull (box options) then nullArg "options"
        let scalars = decodeScalars "invalid-binary-token" maxPatternScalars "pattern-too-long" pattern
        if scalars.IsEmpty then fail "empty-pattern"
        validateSource options.SourceLabel options.SourceIndex
        validateRunRole options.Role
        let result = ResizeArray<Barcode1DV1Run>()
        let mutable current = scalars.Head
        let mutable width = 1L
        let append (token: Rune) (modules: int64) =
            let color =
                if token.Value = int '1' then Bar
                elif token.Value = int '0' then Space
                else fail "invalid-binary-token"
            if result.Count = maxRuns then fail "too-many-runs"
            result.Add
                { Color = color; Modules = modules; SourceLabel = options.SourceLabel
                  SourceIndex = options.SourceIndex; Role = options.Role }
        for token in scalars.Tail do
            if token = current then width <- checkedAdd "content-too-wide" width 1L
            else
                append current width
                current <- token
                width <- 1L
        append current width
        List.ofSeq result

    let expandWidth (pattern: string) (options: Barcode1DV1WidthOptions) =
        if isNull (box options) then nullArg "options"
        let scalars = decodeScalars "invalid-width-token" maxPatternScalars "pattern-too-long" pattern
        if scalars.IsEmpty then fail "empty-pattern"
        let narrow = singleScalar options.NarrowMarker
        let wide = singleScalar options.WideMarker
        if narrow.IsNone || wide.IsNone || narrow = wide then fail "invalid-marker-configuration"
        if options.NarrowModules <= 0L || options.WideModules <= 0L then fail "invalid-module-count"
        validateSource options.SourceLabel options.SourceIndex
        validateRunRole options.Role
        validateColor options.StartingColor
        let result = ResizeArray<Barcode1DV1Run>()
        let mutable color = options.StartingColor
        let mutable content = 0L
        for marker in scalars do
            let modules =
                if Some marker = narrow then options.NarrowModules
                elif Some marker = wide then options.WideModules
                else fail "invalid-width-token"
            content <- checkedAdd "content-too-wide" content modules
            if content > maxContentModules then fail "content-too-wide"
            if result.Count = maxRuns then fail "too-many-runs"
            result.Add
                { Color = color; Modules = modules; SourceLabel = options.SourceLabel
                  SourceIndex = options.SourceIndex; Role = options.Role }
            color <- if color = Bar then Space else Bar
        List.ofSeq result

    let private validateRuns (runs: Barcode1DV1Run list) =
        if List.length runs > maxRuns then fail "too-many-runs"
        let mutable content = 0L
        runs
        |> List.iteri (fun index run ->
            validateSource run.SourceLabel run.SourceIndex
            if run.Modules <= 0L then fail "invalid-module-count"
            validateColor run.Color
            validateRunRole run.Role
            if index > 0 && runs[index - 1].Color = run.Color then fail "non-alternating-runs"
            content <- checkedAdd "content-too-wide" content run.Modules
            if content > maxContentModules then fail "content-too-wide")
        content

    let private symbolRole role =
        match role with
        | Data -> Some SymbolData | Start -> Some SymbolStart | Stop -> Some SymbolStop
        | Guard -> Some SymbolGuard | Check -> Some SymbolCheck | InterCharacterGap -> None

    let private inferSymbols (runs: Barcode1DV1Run list) =
        let result = ResizeArray<Barcode1DV1SymbolLayout>()
        let mutable cursor = 0L
        let mutable current: (string * int * Barcode1DSymbolRole * int64) option = None
        let flush ending =
            match current with
            | Some (label, sourceIndex, role, starting) ->
                result.Add { Label = label; StartModule = starting; EndModule = ending
                             SourceIndex = sourceIndex; Role = role }
                if result.Count > maxSymbols then fail "too-many-symbols"
            | None -> ()
        for run in runs do
            match symbolRole run.Role with
            | Some role ->
                let key = run.SourceLabel, int run.SourceIndex, role
                match current with
                | Some (label, index, currentRole, _) when (label, index, currentRole) = key -> ()
                | _ ->
                    flush cursor
                    current <- Some (run.SourceLabel, int run.SourceIndex, role, cursor)
            | None -> ()
            cursor <- checkedAdd "content-too-wide" cursor run.Modules
        flush cursor
        List.ofSeq result

    let private layoutSymbols (symbols: Barcode1DV1SymbolDescriptor list) content =
        if List.length symbols > maxSymbols then fail "too-many-symbols"
        let result = ResizeArray<Barcode1DV1SymbolLayout>()
        let mutable cursor = 0L
        for symbol in symbols do
            if symbol.Modules <= 0L then fail "invalid-module-count"
            validateSource symbol.Label symbol.SourceIndex
            let ending = checkedAdd "symbol-width-mismatch" cursor symbol.Modules
            result.Add { Label = symbol.Label; StartModule = cursor; EndModule = ending
                         SourceIndex = int symbol.SourceIndex; Role = symbol.Role }
            cursor <- ending
        if cursor <> content then fail "symbol-width-mismatch"
        List.ofSeq result

    let computeLayout (runs: Barcode1DV1Run list) quietZoneModules (symbols: Barcode1DV1SymbolDescriptor list option) =
        if isNull (box runs) then nullArg "runs"
        let content = validateRuns runs
        if quietZoneModules < 1L || quietZoneModules > maxQuietZoneModules then fail "invalid-quiet-zone"
        let total = checkedAdd "content-too-wide" (checkedAdd "content-too-wide" content quietZoneModules) quietZoneModules
        let layouts =
            match symbols with
            | Some values -> layoutSymbols values content
            | None -> inferSymbols runs
        { LeftQuietZoneModules = quietZoneModules; RightQuietZoneModules = quietZoneModules
          ContentModules = content; TotalModules = total; SymbolLayouts = layouts }

    let private validateMetadata (metadata: IReadOnlyDictionary<string, string>) label =
        if isNull (box metadata) then nullArg "metadata"
        if metadata.Count > maxMetadataEntries then fail "metadata-too-large"
        scalarCount "metadata-too-large" maxLabelScalars "metadata-too-large" label |> ignore
        let mutable bytes = 0L
        for pair in metadata do
            scalarCount "metadata-too-large" maxMetadataKeyScalars "metadata-too-large" pair.Key |> ignore
            scalarCount "metadata-too-large" maxMetadataValueScalars "metadata-too-large" pair.Value |> ignore
            bytes <- bytes + int64 (Encoding.UTF8.GetByteCount pair.Key) + int64 (Encoding.UTF8.GetByteCount pair.Value)
            if bytes > maxMetadataBytes then fail "metadata-too-large"

    let projectSceneWithProbeForTests (runs: Barcode1DV1Run list) (options: Barcode1DV1SceneOptions option) (forbiddenNativeTextResolver: Action) =
        if isNull (box runs) then nullArg "runs"
        if isNull forbiddenNativeTextResolver then nullArg "forbiddenNativeTextResolver"
        let options = defaultArg options (defaultSceneOptions ())
        if options.RenderConfig.IncludeHumanReadableText || options.HumanReadableText.IsSome then
            fail "human-readable-text-unsupported"
        let config = options.RenderConfig
        if config.ModuleWidth < 1L || config.ModuleWidth > maxRenderDimension
           || config.BarHeight < 1L || config.BarHeight > maxRenderDimension then
            fail "invalid-render-config"
        scalarCount "invalid-render-config" maxColorScalars "invalid-render-config" config.Foreground |> ignore
        scalarCount "invalid-render-config" maxColorScalars "invalid-render-config" config.Background |> ignore
        let layout = computeLayout runs options.QuietZoneModules options.Symbols
        validateMetadata options.Metadata options.Label
        let instructions = ResizeArray<PaintInstruction>()
        let mutable cursor = layout.LeftQuietZoneModules
        for run in runs do
            let ending = checkedAdd "content-too-wide" cursor run.Modules
            if run.Color = Bar then
                let metadata = Dictionary<string, obj>()
                metadata["sourceLabel"] <- box run.SourceLabel
                metadata["sourceIndex"] <- box (decimal run.SourceIndex)
                metadata["role"] <- box run.Role.AsString
                metadata["moduleStart"] <- box (decimal cursor)
                metadata["moduleEnd"] <- box (decimal ending)
                instructions.Add(
                    PaintInstructions.paintRectWith
                        { PaintInstructions.defaultPaintRectOptions with
                            Fill = Some config.Foreground
                            Metadata = Some (metadata :> Metadata) }
                        (float (checkedMultiply "invalid-render-config" cursor config.ModuleWidth)) 0.0
                        (float (checkedMultiply "invalid-render-config" run.Modules config.ModuleWidth)) (float config.BarHeight))
            cursor <- ending
        let sceneWidth = checkedMultiply "invalid-render-config" layout.TotalModules config.ModuleWidth
        let metadata = Dictionary<string, obj>()
        for pair in options.Metadata do metadata[pair.Key] <- box pair.Value
        metadata["label"] <- box options.Label
        metadata["leftQuietZoneModules"] <- box (decimal layout.LeftQuietZoneModules)
        metadata["rightQuietZoneModules"] <- box (decimal layout.RightQuietZoneModules)
        metadata["contentModules"] <- box (decimal layout.ContentModules)
        metadata["totalModules"] <- box (decimal layout.TotalModules)
        metadata["moduleWidthPx"] <- box (decimal config.ModuleWidth)
        metadata["barHeightPx"] <- box (decimal config.BarHeight)
        metadata["sceneWidthPx"] <- box (decimal sceneWidth)
        metadata["sceneHeightPx"] <- box (decimal config.BarHeight)
        metadata["symbolCount"] <- box (decimal layout.SymbolLayouts.Length)
        PaintInstructions.paintSceneWith
            { PaintInstructions.defaultSceneOptions with Metadata = Some (metadata :> Metadata) }
            (float sceneWidth) (float config.BarHeight) config.Background (List.ofSeq instructions)

    let projectScene runs options =
        projectSceneWithProbeForTests runs options (Action(fun () -> invalidOp "native text resolution is forbidden"))
