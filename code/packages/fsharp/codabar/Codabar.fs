namespace CodingAdventures.Codabar.FSharp

open System
open System.Collections.Generic
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.PaintInstructions

exception InvalidCodabarInputException of string

type EncodedCodabarSymbol =
    { Char: string
      Pattern: string
      SourceIndex: int
      Role: Barcode1DRunRole }

[<RequireQualifiedAccess>]
module Codabar =
    [<Literal>]
    let VERSION = "0.1.0"

    let private guards = Set.ofList [ "A"; "B"; "C"; "D" ]

    let patterns =
        Map.ofList [
            "0", "101010011"
            "1", "101011001"
            "2", "101001011"
            "3", "110010101"
            "4", "101101001"
            "5", "110101001"
            "6", "100101011"
            "7", "100101101"
            "8", "100110101"
            "9", "110100101"
            "-", "101001101"
            "$", "101100101"
            ":", "1101011011"
            "/", "1101101011"
            ".", "1101101101"
            "+", "1011011011"
            "A", "1011001001"
            "B", "1001001011"
            "C", "1010010011"
            "D", "1010011001"
        ]

    let defaultRenderConfig = BarcodeLayout1D.defaultRenderConfig

    let private invalidInput message =
        raise (InvalidCodabarInputException message)

    let private isGuard value =
        guards.Contains value

    let private validateGuard paramName (value: string) =
        if isNull value then
            nullArg paramName

        let normalized = value.ToUpperInvariant()
        if not (isGuard normalized) then
            invalidInput $"Codabar {paramName} guard must be one of A, B, C, or D"

    let private assertBodyChars (body: string) =
        for ch in body do
            let value = string ch
            if not (patterns.ContainsKey value) || isGuard value then
                invalidInput $"invalid Codabar body character \"{value}\""

    let normalizeCodabar (data: string) (start: string option) (stop: string option) =
        if isNull data then
            nullArg (nameof data)

        let start = defaultArg start "A"
        let stop = defaultArg stop "A"
        let normalized = data.ToUpperInvariant()

        if normalized.Length >= 2 then
            let first = string normalized[0]
            let last = string normalized[normalized.Length - 1]
            if isGuard first && isGuard last then
                assertBodyChars normalized[1 .. normalized.Length - 2]
                normalized
            else
                validateGuard "start" start
                validateGuard "stop" stop
                assertBodyChars normalized
                $"{start.ToUpperInvariant()}{normalized}{stop.ToUpperInvariant()}"
        else
            validateGuard "start" start
            validateGuard "stop" stop
            assertBodyChars normalized
            $"{start.ToUpperInvariant()}{normalized}{stop.ToUpperInvariant()}"

    let encodeCodabar data start stop =
        let normalized = normalizeCodabar data start stop
        normalized
        |> Seq.mapi (fun index ch ->
            let value = string ch
            { Char = value
              Pattern = patterns[value]
              SourceIndex = index
              Role =
                if index = 0 then Start
                elif index = normalized.Length - 1 then Stop
                else Data })
        |> Seq.toList

    let expandCodabarRuns data start stop =
        let encoded = encodeCodabar data start stop
        let runs = ResizeArray<Barcode1DRun>()

        encoded
        |> List.iteri (fun index symbol ->
            let symbolRuns =
                BarcodeLayout1D.runsFromBinaryPattern
                    symbol.Pattern
                    { SourceLabel = symbol.Char
                      SourceIndex = symbol.SourceIndex
                      Role = symbol.Role }

            runs.AddRange(symbolRuns)

            if index < encoded.Length - 1 then
                runs.Add
                    { Color = Space
                      Modules = 1u
                      SourceLabel = symbol.Char
                      SourceIndex = symbol.SourceIndex
                      Role = InterCharacterGap })

        List.ofSeq runs

    let layoutCodabar data options start stop =
        let normalized = normalizeCodabar data start stop
        let runs = expandCodabarRuns normalized None None
        let options = defaultArg options BarcodeLayout1D.defaultPaintOptions
        let metadata = Dictionary<string, obj>()

        for pair in options.Metadata do
            metadata.[pair.Key] <- pair.Value

        metadata.["symbology"] <- box "codabar"
        metadata.["start"] <- box (string normalized[0])
        metadata.["stop"] <- box (string normalized[normalized.Length - 1])

        let layoutOptions =
            { options with
                Label = options.Label |> Option.orElse (Some $"Codabar barcode for {normalized}")
                HumanReadableText = options.HumanReadableText |> Option.orElse (Some normalized)
                Metadata = metadata :> Metadata }

        BarcodeLayout1D.layoutBarcode1D runs (Some layoutOptions)

    let drawCodabar data options start stop =
        layoutCodabar data options start stop
