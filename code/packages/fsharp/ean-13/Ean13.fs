namespace CodingAdventures.Ean13.FSharp

open System
open System.Collections.Generic
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.PaintInstructions

exception InvalidEan13InputException of string
exception InvalidEan13CheckDigitException of string

type EncodedDigit =
    { Digit: string
      Encoding: string
      Pattern: string
      SourceIndex: int
      Role: Barcode1DRunRole }

[<RequireQualifiedAccess>]
module Ean13 =
    [<Literal>]
    let VERSION = "0.1.0"

    let private sideGuard = "101"
    let private centerGuard = "01010"

    let private leftPatterns =
        [| "0001101"; "0011001"; "0010011"; "0111101"; "0100011"
           "0110001"; "0101111"; "0111011"; "0110111"; "0001011" |]

    let private gPatterns =
        [| "0100111"; "0110011"; "0011011"; "0100001"; "0011101"
           "0111001"; "0000101"; "0010001"; "0001001"; "0010111" |]

    let private rightPatterns =
        [| "1110010"; "1100110"; "1101100"; "1000010"; "1011100"
           "1001110"; "1010000"; "1000100"; "1001000"; "1110100" |]

    let private leftParityPatterns =
        [| "LLLLLL"; "LLGLGG"; "LLGGLG"; "LLGGGL"; "LGLLGG"
           "LGGLLG"; "LGGGLL"; "LGLGLG"; "LGLGGL"; "LGGLGL" |]

    let defaultRenderConfig = BarcodeLayout1D.defaultRenderConfig

    let private invalidInput message =
        raise (InvalidEan13InputException message)

    let private invalidCheckDigit message =
        raise (InvalidEan13CheckDigitException message)

    let private digitValue (digit: char) =
        int digit - int '0'

    let private assertDigits (data: string) expectedLengths =
        if isNull data then
            nullArg (nameof data)

        if data |> Seq.exists (fun ch -> ch < '0' || ch > '9') then
            invalidInput "EAN-13 input must contain digits only"

        if not (expectedLengths |> List.contains data.Length) then
            invalidInput "EAN-13 input must contain 12 digits or 13 digits"

    let computeEan13CheckDigit (payload12: string) =
        assertDigits payload12 [ 12 ]

        let total =
            payload12
            |> Seq.rev
            |> Seq.mapi (fun index digit -> digitValue digit * if index % 2 = 0 then 3 else 1)
            |> Seq.sum

        string ((10 - (total % 10)) % 10)

    let normalizeEan13 (data: string) =
        assertDigits data [ 12; 13 ]

        if data.Length = 12 then
            $"{data}{computeEan13CheckDigit data}"
        else
            let expected = computeEan13CheckDigit data[0..11]
            let actual = string data[12]
            if expected <> actual then
                invalidCheckDigit $"invalid EAN-13 check digit: expected {expected} but received {actual}"

            data

    let leftParityPattern data =
        let normalized = normalizeEan13 data
        leftParityPatterns[digitValue normalized[0]]

    let encodeEan13 data =
        let normalized = normalizeEan13 data
        let parity = leftParityPatterns[digitValue normalized[0]]

        let left =
            [ for offset in 0 .. 5 do
                let digit = normalized[offset + 1]
                let encoding = string parity[offset]
                let pattern = if encoding = "L" then leftPatterns[digitValue digit] else gPatterns[digitValue digit]
                { Digit = string digit
                  Encoding = encoding
                  Pattern = pattern
                  SourceIndex = offset + 1
                  Role = Data } ]

        let right =
            [ for offset in 0 .. 5 do
                let digit = normalized[offset + 7]
                { Digit = string digit
                  Encoding = "R"
                  Pattern = rightPatterns[digitValue digit]
                  SourceIndex = offset + 7
                  Role = if offset = 5 then Check else Data } ]

        left @ right

    let private symbolFor digit =
        { Label = digit.Digit
          Modules = 7u
          SourceIndex = digit.SourceIndex
          Role = if digit.Role = Check then SymbolCheck else SymbolData }

    let private buildSymbols encoded =
        [ yield { Label = "start"; Modules = 3u; SourceIndex = -1; Role = SymbolGuard }
          yield! encoded |> List.take 6 |> List.map symbolFor
          yield { Label = "center"; Modules = 5u; SourceIndex = -2; Role = SymbolGuard }
          yield! encoded |> List.skip 6 |> List.map symbolFor
          yield { Label = "end"; Modules = 3u; SourceIndex = -3; Role = SymbolGuard } ]

    let private addPatternRuns (runs: ResizeArray<Barcode1DRun>) pattern sourceLabel sourceIndex role =
        let patternRuns =
            BarcodeLayout1D.runsFromBinaryPattern
                pattern
                { SourceLabel = sourceLabel
                  SourceIndex = sourceIndex
                  Role = role }

        runs.AddRange(patternRuns)

    let expandEan13Runs data =
        let encoded = encodeEan13 data
        let runs = ResizeArray<Barcode1DRun>()

        addPatternRuns runs sideGuard "start" -1 Guard
        encoded |> List.take 6 |> List.iter (fun digit -> addPatternRuns runs digit.Pattern digit.Digit digit.SourceIndex digit.Role)
        addPatternRuns runs centerGuard "center" -2 Guard
        encoded |> List.skip 6 |> List.iter (fun digit -> addPatternRuns runs digit.Pattern digit.Digit digit.SourceIndex digit.Role)
        addPatternRuns runs sideGuard "end" -3 Guard

        List.ofSeq runs

    let layoutEan13 data options =
        let normalized = normalizeEan13 data
        let encoded = encodeEan13 normalized
        let runs = expandEan13Runs normalized
        let options = defaultArg options BarcodeLayout1D.defaultPaintOptions
        let metadata = Dictionary<string, obj>()

        for pair in options.Metadata do
            metadata.[pair.Key] <- pair.Value

        metadata.["symbology"] <- box "ean-13"
        metadata.["leadingDigit"] <- box (string normalized[0])
        metadata.["leftParity"] <- box leftParityPatterns[digitValue normalized[0]]

        let layoutOptions =
            { options with
                Label = options.Label |> Option.orElse (Some $"EAN-13 barcode for {normalized}")
                HumanReadableText = options.HumanReadableText |> Option.orElse (Some normalized)
                Metadata = metadata :> Metadata
                Symbols = Some(buildSymbols encoded) }

        BarcodeLayout1D.layoutBarcode1D runs (Some layoutOptions)

    let drawEan13 data options =
        layoutEan13 data options
