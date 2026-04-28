namespace CodingAdventures.Code39.FSharp

open System
open System.Collections.Generic
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.PaintInstructions

exception InvalidCharacterException of string

type EncodedCharacter =
    { Char: string
      IsStartStop: bool
      Pattern: string }

[<RequireQualifiedAccess>]
module Code39 =
    [<Literal>]
    let VERSION = "0.1.0"

    let patterns =
        Map.ofList [
            "0", "bwbWBwBwb"; "1", "BwbWbwbwB"; "2", "bwBWbwbwB"; "3", "BwBWbwbwb"
            "4", "bwbWBwbwB"; "5", "BwbWBwbwb"; "6", "bwBWBwbwb"; "7", "bwbWbwBwB"
            "8", "BwbWbwBwb"; "9", "bwBWbwBwb"; "A", "BwbwbWbwB"; "B", "bwBwbWbwB"
            "C", "BwBwbWbwb"; "D", "bwbwBWbwB"; "E", "BwbwBWbwb"; "F", "bwBwBWbwb"
            "G", "bwbwbWBwB"; "H", "BwbwbWBwb"; "I", "bwBwbWBwb"; "J", "bwbwBWBwb"
            "K", "BwbwbwbWB"; "L", "bwBwbwbWB"; "M", "BwBwbwbWb"; "N", "bwbwBwbWB"
            "O", "BwbwBwbWb"; "P", "bwBwBwbWb"; "Q", "bwbwbwBWB"; "R", "BwbwbwBWb"
            "S", "bwBwbwBWb"; "T", "bwbwBwBWb"; "U", "BWbwbwbwB"; "V", "bWBwbwbwB"
            "W", "BWBwbwbwb"; "X", "bWbwBwbwB"; "Y", "BWbwBwbwb"; "Z", "bWBwBwbwb"
            "-", "bWbwbwBwB"; ".", "BWbwbwBwb"; " ", "bWBwbwBwb"; "$", "bWbWbWbwb"
            "/", "bWbWbwbWb"; "+", "bWbwbWbWb"; "%", "bwbWbWbWb"; "*", "bWbwBwBwb"
        ]

    let defaultRenderConfig = BarcodeLayout1D.defaultRenderConfig

    let private invalidCharacter message =
        raise (InvalidCharacterException message)

    let private widthPattern (pattern: string) =
        pattern
        |> Seq.map (fun part -> if Char.IsUpper part then 'W' else 'N')
        |> Array.ofSeq
        |> String

    let normalizeCode39 (data: string) =
        if isNull data then
            nullArg (nameof data)

        let normalized = data.ToUpperInvariant()
        for ch in normalized do
            let value = string ch
            if value = "*" then
                invalidCharacter "input must not contain \"*\" because it is reserved for start/stop"
            if not (patterns.ContainsKey value) then
                invalidCharacter $"invalid character: \"{value}\" is not supported by Code 39"

        normalized

    let encodeCode39Char (value: string) =
        if isNull value then
            nullArg (nameof value)

        match patterns.TryFind value with
        | Some pattern ->
            { Char = value
              IsStartStop = value = "*"
              Pattern = widthPattern pattern }
        | None -> invalidCharacter $"invalid character: \"{value}\" is not supported by Code 39"

    let encodeCode39 data =
        let normalized = normalizeCode39 data
        $"*{normalized}*"
        |> Seq.map (string >> encodeCode39Char)
        |> Seq.toList

    let private runRoleFor sourceIndex encodedLength encodedChar =
        if not encodedChar.IsStartStop then
            Data
        elif sourceIndex = 0 then
            Start
        elif sourceIndex = encodedLength - 1 then
            Stop
        else
            Guard

    let expandCode39Runs data =
        let encoded = encodeCode39 data
        let runs = ResizeArray<Barcode1DRun>()

        encoded
        |> List.iteri (fun sourceIndex encodedChar ->
            let role = runRoleFor sourceIndex encoded.Length encodedChar
            let charRuns =
                BarcodeLayout1D.runsFromWidthPattern
                    encodedChar.Pattern
                    (BarcodeLayout1D.defaultWidthPatternOptions encodedChar.Char sourceIndex role)

            runs.AddRange(charRuns)

            if sourceIndex < encoded.Length - 1 then
                runs.Add
                    { Color = Space
                      Modules = 1u
                      SourceLabel = encodedChar.Char
                      SourceIndex = sourceIndex
                      Role = InterCharacterGap })

        List.ofSeq runs

    let layoutCode39 data options =
        let normalized = normalizeCode39 data
        let runs = expandCode39Runs normalized
        let options = defaultArg options BarcodeLayout1D.defaultPaintOptions
        let metadata = Dictionary<string, obj>()

        for pair in options.Metadata do
            metadata.[pair.Key] <- pair.Value

        metadata.["symbology"] <- box "code39"
        metadata.["encodedText"] <- box normalized

        let layoutOptions =
            { options with
                Label =
                    options.Label
                    |> Option.orElse (Some(if normalized.Length = 0 then "Code 39 barcode" else $"Code 39 barcode for {normalized}"))
                HumanReadableText = options.HumanReadableText |> Option.orElse (Some normalized)
                Metadata = metadata :> Metadata }

        BarcodeLayout1D.layoutBarcode1D runs (Some layoutOptions)

    let drawCode39 data options =
        layoutCode39 data options
