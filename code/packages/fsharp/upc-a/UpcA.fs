namespace CodingAdventures.UpcA.FSharp

open System
open System.Collections.Generic
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.PaintInstructions

exception InvalidUpcAInputException of string
exception InvalidUpcACheckDigitException of string

type EncodedDigit =
    { Digit: string
      Encoding: string
      Pattern: string
      SourceIndex: int
      Role: Barcode1DRunRole }

[<RequireQualifiedAccess>]
module UpcA =
    [<Literal>]
    let VERSION = "0.1.0"

    let private sideGuard = "101"
    let private centerGuard = "01010"

    let private leftPatterns =
        [| "0001101"; "0011001"; "0010011"; "0111101"; "0100011"
           "0110001"; "0101111"; "0111011"; "0110111"; "0001011" |]

    let private rightPatterns =
        [| "1110010"; "1100110"; "1101100"; "1000010"; "1011100"
           "1001110"; "1010000"; "1000100"; "1001000"; "1110100" |]

    let defaultRenderConfig = BarcodeLayout1D.defaultRenderConfig

    let private invalidInput message =
        raise (InvalidUpcAInputException message)

    let private invalidCheckDigit message =
        raise (InvalidUpcACheckDigitException message)

    let private digitValue (digit: char) =
        int digit - int '0'

    let private assertDigits (data: string) expectedLengths =
        if isNull data then
            nullArg (nameof data)

        if data |> Seq.exists (fun ch -> ch < '0' || ch > '9') then
            invalidInput "UPC-A input must contain digits only"

        if not (expectedLengths |> List.contains data.Length) then
            invalidInput "UPC-A input must contain 11 digits or 12 digits"

    let computeUpcACheckDigit (payload11: string) =
        assertDigits payload11 [ 11 ]

        let mutable oddSum = 0
        let mutable evenSum = 0
        for index in 0 .. payload11.Length - 1 do
            let value = digitValue payload11[index]
            if index % 2 = 0 then
                oddSum <- oddSum + value
            else
                evenSum <- evenSum + value

        string ((10 - ((oddSum * 3 + evenSum) % 10)) % 10)

    let normalizeUpcA (data: string) =
        assertDigits data [ 11; 12 ]

        if data.Length = 11 then
            $"{data}{computeUpcACheckDigit data}"
        else
            let expected = computeUpcACheckDigit data[0..10]
            let actual = string data[11]
            if expected <> actual then
                invalidCheckDigit $"invalid UPC-A check digit: expected {expected} but received {actual}"

            data

    let encodeUpcA data =
        let normalized = normalizeUpcA data
        [ for index in 0 .. normalized.Length - 1 do
            let digit = normalized[index]
            { Digit = string digit
              Encoding = if index < 6 then "L" else "R"
              Pattern = if index < 6 then leftPatterns[digitValue digit] else rightPatterns[digitValue digit]
              SourceIndex = index
              Role = if index = 11 then Check else Data } ]

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

    let expandUpcARuns data =
        let encoded = encodeUpcA data
        let runs = ResizeArray<Barcode1DRun>()

        addPatternRuns runs sideGuard "start" -1 Guard
        encoded |> List.take 6 |> List.iter (fun digit -> addPatternRuns runs digit.Pattern digit.Digit digit.SourceIndex digit.Role)
        addPatternRuns runs centerGuard "center" -2 Guard
        encoded |> List.skip 6 |> List.iter (fun digit -> addPatternRuns runs digit.Pattern digit.Digit digit.SourceIndex digit.Role)
        addPatternRuns runs sideGuard "end" -3 Guard

        List.ofSeq runs

    let layoutUpcA data options =
        let normalized = normalizeUpcA data
        let encoded = encodeUpcA normalized
        let runs = expandUpcARuns normalized
        let options = defaultArg options BarcodeLayout1D.defaultPaintOptions
        let metadata = Dictionary<string, obj>()

        for pair in options.Metadata do
            metadata.[pair.Key] <- pair.Value

        metadata.["symbology"] <- box "upc-a"

        let layoutOptions =
            { options with
                Label = options.Label |> Option.orElse (Some $"UPC-A barcode for {normalized}")
                HumanReadableText = options.HumanReadableText |> Option.orElse (Some normalized)
                Metadata = metadata :> Metadata
                Symbols = Some(buildSymbols encoded) }

        BarcodeLayout1D.layoutBarcode1D runs (Some layoutOptions)

    let drawUpcA data options =
        layoutUpcA data options
