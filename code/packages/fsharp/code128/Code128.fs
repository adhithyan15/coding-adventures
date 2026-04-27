namespace CodingAdventures.Code128.FSharp

open System
open System.Collections.Generic
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.PaintInstructions

exception InvalidCode128InputException of string

type EncodedCode128Symbol =
    { Label: string
      Value: int
      Pattern: string
      SourceIndex: int
      Role: Barcode1DRunRole }

[<RequireQualifiedAccess>]
module Code128 =
    [<Literal>]
    let VERSION = "0.1.0"

    let private startB = 104
    let private stop = 106

    let private patterns =
        [| "11011001100"; "11001101100"; "11001100110"; "10010011000"; "10010001100"
           "10001001100"; "10011001000"; "10011000100"; "10001100100"; "11001001000"
           "11001000100"; "11000100100"; "10110011100"; "10011011100"; "10011001110"
           "10111001100"; "10011101100"; "10011100110"; "11001110010"; "11001011100"
           "11001001110"; "11011100100"; "11001110100"; "11101101110"; "11101001100"
           "11100101100"; "11100100110"; "11101100100"; "11100110100"; "11100110010"
           "11011011000"; "11011000110"; "11000110110"; "10100011000"; "10001011000"
           "10001000110"; "10110001000"; "10001101000"; "10001100010"; "11010001000"
           "11000101000"; "11000100010"; "10110111000"; "10110001110"; "10001101110"
           "10111011000"; "10111000110"; "10001110110"; "11101110110"; "11010001110"
           "11000101110"; "11011101000"; "11011100010"; "11011101110"; "11101011000"
           "11101000110"; "11100010110"; "11101101000"; "11101100010"; "11100011010"
           "11101111010"; "11001000010"; "11110001010"; "10100110000"; "10100001100"
           "10010110000"; "10010000110"; "10000101100"; "10000100110"; "10110010000"
           "10110000100"; "10011010000"; "10011000010"; "10000110100"; "10000110010"
           "11000010010"; "11001010000"; "11110111010"; "11000010100"; "10001111010"
           "10100111100"; "10010111100"; "10010011110"; "10111100100"; "10011110100"
           "10011110010"; "11110100100"; "11110010100"; "11110010010"; "11011011110"
           "11011110110"; "11110110110"; "10101111000"; "10100011110"; "10001011110"
           "10111101000"; "10111100010"; "11110101000"; "11110100010"; "10111011110"
           "10111101110"; "11101011110"; "11110101110"; "11010000100"; "11010010000"
           "11010011100"; "1100011101011" |]

    let defaultRenderConfig = BarcodeLayout1D.defaultRenderConfig

    let private invalidInput message =
        raise (InvalidCode128InputException message)

    let normalizeCode128B (data: string) =
        if isNull data then
            nullArg (nameof data)

        for ch in data do
            if ch < ' ' || ch > '~' then
                invalidInput "Code 128 Code Set B supports printable ASCII characters only"

        data

    let private valueForCode128BChar ch =
        int ch - 32

    let computeCode128Checksum (values: int list) =
        if isNull (box values) then
            nullArg (nameof values)

        values
        |> List.mapi (fun index value -> value * (index + 1))
        |> List.sum
        |> (+) startB
        |> fun total -> total % 103

    let encodeCode128B data =
        let normalized = normalizeCode128B data
        let dataSymbols =
            normalized
            |> Seq.mapi (fun index ch ->
                let value = valueForCode128BChar ch
                { Label = string ch
                  Value = value
                  Pattern = patterns[value]
                  SourceIndex = index
                  Role = Data })
            |> Seq.toList
        let checksum = dataSymbols |> List.map _.Value |> computeCode128Checksum

        [ yield { Label = "Start B"; Value = startB; Pattern = patterns[startB]; SourceIndex = -1; Role = Start }
          yield! dataSymbols
          yield { Label = $"Checksum {checksum}"; Value = checksum; Pattern = patterns[checksum]; SourceIndex = normalized.Length; Role = Check }
          yield { Label = "Stop"; Value = stop; Pattern = patterns[stop]; SourceIndex = normalized.Length + 1; Role = Stop } ]

    let private symbolFor symbol =
        { Label = symbol.Label
          Modules = if symbol.Role = Stop then 13u else 11u
          SourceIndex = symbol.SourceIndex
          Role =
            match symbol.Role with
            | Start -> SymbolStart
            | Check -> SymbolCheck
            | Stop -> SymbolStop
            | _ -> SymbolData }

    let expandCode128Runs data =
        let encoded = encodeCode128B data
        let runs = ResizeArray<Barcode1DRun>()

        for symbol in encoded do
            let symbolRuns =
                BarcodeLayout1D.runsFromBinaryPattern
                    symbol.Pattern
                    { SourceLabel = symbol.Label
                      SourceIndex = symbol.SourceIndex
                      Role = symbol.Role }

            runs.AddRange(symbolRuns)

        List.ofSeq runs

    let layoutCode128 data options =
        let normalized = normalizeCode128B data
        let encoded = encodeCode128B normalized
        let checksum = encoded[encoded.Length - 2].Value
        let runs = expandCode128Runs normalized
        let options = defaultArg options BarcodeLayout1D.defaultPaintOptions
        let metadata = Dictionary<string, obj>()

        for pair in options.Metadata do
            metadata.[pair.Key] <- pair.Value

        metadata.["symbology"] <- box "code128"
        metadata.["codeSet"] <- box "B"
        metadata.["checksum"] <- box checksum

        let layoutOptions =
            { options with
                Label = options.Label |> Option.orElse (Some $"Code 128 barcode for {normalized}")
                HumanReadableText = options.HumanReadableText |> Option.orElse (Some normalized)
                Metadata = metadata :> Metadata
                Symbols = Some(encoded |> List.map symbolFor) }

        BarcodeLayout1D.layoutBarcode1D runs (Some layoutOptions)

    let drawCode128 data options =
        layoutCode128 data options
