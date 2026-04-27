namespace CodingAdventures.CsvParser

open System
open System.Collections.Generic
open System.Text

// CsvParser.fs -- CSV needs state because commas do not always mean "next field"
// =============================================================================
//
// A comma outside quotes separates columns.
// A comma inside quotes is ordinary text.
//
// That single rule is why CSV parsing becomes a state machine instead of a
// string split. The parser must remember which mode it is in at every step.

/// Raised when the input ends while the parser is still inside a quoted field.
type UnclosedQuoteError() =
    inherit Exception("unclosed quoted field: EOF reached inside a quoted field")

[<RequireQualifiedAccess>]
module CsvParser =
    type private ParseState =
        | FieldStart
        | InUnquotedField
        | InQuotedField
        | InQuotedMaybeEnd

    let private isNewlineStart ch =
        ch = '\n' || ch = '\r'

    let private consumeNewline (source: string) index =
        if source[index] = '\r' && index + 1 < source.Length && source[index + 1] = '\n' then
            index + 2
        else
            index + 1

    let private buildRowMap (header: string array) (row: string array) =
        let map = Dictionary<string, string>(header.Length, StringComparer.Ordinal)

        for index in 0 .. header.Length - 1 do
            map[header[index]] <-
                if index < row.Length then
                    row[index]
                else
                    String.Empty

        map

    let private tokeniseRows (source: string) delimiter =
        let rows = ResizeArray<string array>()
        let mutable currentRow = ResizeArray<string>()
        let fieldBuffer = StringBuilder()
        let mutable state = ParseState.FieldStart
        let mutable index = 0

        while index < source.Length do
            let ch = source[index]
            let mutable advance = true

            match state with
            | ParseState.FieldStart ->
                if ch = '"' then
                    state <- ParseState.InQuotedField
                elif ch = delimiter then
                    currentRow.Add(String.Empty)
                elif isNewlineStart ch then
                    if currentRow.Count > 0 then
                        currentRow.Add(String.Empty)

                    rows.Add(currentRow |> Seq.toArray)
                    currentRow <- ResizeArray<string>()
                    index <- consumeNewline source index
                    advance <- false
                else
                    fieldBuffer.Append(ch) |> ignore
                    state <- ParseState.InUnquotedField

            | ParseState.InUnquotedField ->
                if ch = delimiter then
                    currentRow.Add(fieldBuffer.ToString())
                    fieldBuffer.Clear() |> ignore
                    state <- ParseState.FieldStart
                elif isNewlineStart ch then
                    currentRow.Add(fieldBuffer.ToString())
                    fieldBuffer.Clear() |> ignore
                    rows.Add(currentRow |> Seq.toArray)
                    currentRow <- ResizeArray<string>()
                    state <- ParseState.FieldStart
                    index <- consumeNewline source index
                    advance <- false
                else
                    fieldBuffer.Append(ch) |> ignore

            | ParseState.InQuotedField ->
                if ch = '"' then
                    state <- ParseState.InQuotedMaybeEnd
                else
                    fieldBuffer.Append(ch) |> ignore

            | ParseState.InQuotedMaybeEnd ->
                if ch = '"' then
                    fieldBuffer.Append('"') |> ignore
                    state <- ParseState.InQuotedField
                elif ch = delimiter then
                    currentRow.Add(fieldBuffer.ToString())
                    fieldBuffer.Clear() |> ignore
                    state <- ParseState.FieldStart
                elif isNewlineStart ch then
                    currentRow.Add(fieldBuffer.ToString())
                    fieldBuffer.Clear() |> ignore
                    rows.Add(currentRow |> Seq.toArray)
                    currentRow <- ResizeArray<string>()
                    state <- ParseState.FieldStart
                    index <- consumeNewline source index
                    advance <- false
                else
                    fieldBuffer.Append(ch) |> ignore
                    state <- ParseState.InUnquotedField

            if advance then
                index <- index + 1

        match state with
        | ParseState.FieldStart ->
            if currentRow.Count > 0 then
                currentRow.Add(String.Empty)
                rows.Add(currentRow |> Seq.toArray)

        | ParseState.InUnquotedField ->
            currentRow.Add(fieldBuffer.ToString())
            rows.Add(currentRow |> Seq.toArray)

        | ParseState.InQuotedField ->
            raise (UnclosedQuoteError())

        | ParseState.InQuotedMaybeEnd ->
            currentRow.Add(fieldBuffer.ToString())
            rows.Add(currentRow |> Seq.toArray)

        rows |> Seq.toArray

    /// Parse CSV text using a comma as the delimiter.
    let rec parseCsv (source: string) =
        parseCsvWithDelimiter source ','

    /// Parse CSV text using a custom delimiter.
    and parseCsvWithDelimiter (source: string) (delimiter: char) =
        if isNull source then
            nullArg "source"

        if delimiter = '"' then
            invalidArg "delimiter" "Delimiter cannot be a double quote."

        let rawRows = tokeniseRows source delimiter

        if rawRows.Length = 0 then
            [||]
        elif rawRows.Length = 1 then
            [||]
        else
            let header = rawRows[0]
            rawRows[1..] |> Array.map (buildRowMap header)

    /// Parse CSV text using a single-character delimiter string.
    let parseCsvWithDelimiterString (source: string) (delimiter: string) =
        if isNull delimiter then
            nullArg "delimiter"

        if delimiter.Length <> 1 then
            invalidArg "delimiter" "Delimiter must be a single character."

        parseCsvWithDelimiter source delimiter[0]
