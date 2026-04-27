namespace CodingAdventures.Itf.FSharp

open System
open System.Collections.Generic
open System.Text
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.PaintInstructions

exception InvalidItfInputException of string

type EncodedPair =
    { Pair: string
      BarPattern: string
      SpacePattern: string
      BinaryPattern: string
      SourceIndex: int }

[<RequireQualifiedAccess>]
module Itf =
    [<Literal>]
    let VERSION = "0.1.0"

    let private startPattern = "1010"
    let private stopPattern = "11101"

    let private digitPatterns =
        [|
            "00110"
            "10001"
            "01001"
            "11000"
            "00101"
            "10100"
            "01100"
            "00011"
            "10010"
            "01010"
        |]

    let defaultRenderConfig = BarcodeLayout1D.defaultRenderConfig

    let private invalidInput message =
        raise (InvalidItfInputException message)

    let normalizeItf (data: string) =
        if isNull data then
            nullArg (nameof data)

        if data.Length = 0 || data.Length % 2 <> 0 then
            invalidInput "ITF input must contain an even number of digits"

        if data |> Seq.exists (fun ch -> not (Char.IsDigit ch)) then
            invalidInput "ITF input must contain digits only"

        data

    let private encodePair (pair: string) sourceIndex =
        let barPattern = digitPatterns[int pair[0] - int '0']
        let spacePattern = digitPatterns[int pair[1] - int '0']
        let builder = StringBuilder()

        for index in 0 .. barPattern.Length - 1 do
            builder.Append(if barPattern[index] = '1' then "111" else "1") |> ignore
            builder.Append(if spacePattern[index] = '1' then "000" else "0") |> ignore

        { Pair = pair
          BarPattern = barPattern
          SpacePattern = spacePattern
          BinaryPattern = builder.ToString()
          SourceIndex = sourceIndex }

    let encodeItf data =
        let normalized = normalizeItf data

        [ for index in 0 .. 2 .. normalized.Length - 1 ->
              encodePair (normalized.Substring(index, 2)) (index / 2) ]

    let expandItfRuns data =
        let encodedPairs = encodeItf data
        let runs = ResizeArray<Barcode1DRun>()

        runs.AddRange(
            BarcodeLayout1D.runsFromBinaryPattern
                startPattern
                { SourceLabel = "start"; SourceIndex = -1; Role = Start }
        )

        for pair in encodedPairs do
            runs.AddRange(
                BarcodeLayout1D.runsFromBinaryPattern
                    pair.BinaryPattern
                    { SourceLabel = pair.Pair; SourceIndex = pair.SourceIndex; Role = Data }
            )

        runs.AddRange(
            BarcodeLayout1D.runsFromBinaryPattern
                stopPattern
                { SourceLabel = "stop"; SourceIndex = -2; Role = Stop }
        )

        List.ofSeq runs

    let layoutItf data options =
        let normalized = normalizeItf data
        let runs = expandItfRuns normalized
        let options = defaultArg options BarcodeLayout1D.defaultPaintOptions
        let metadata = Dictionary<string, obj>()

        for pair in options.Metadata do
            metadata[pair.Key] <- pair.Value

        metadata["symbology"] <- box "itf"
        metadata["pairCount"] <- box (normalized.Length / 2)

        BarcodeLayout1D.layoutBarcode1D runs (Some { options with Metadata = metadata :> Metadata })

    let drawItf data options =
        layoutItf data options
