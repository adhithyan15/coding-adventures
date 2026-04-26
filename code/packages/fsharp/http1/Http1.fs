namespace CodingAdventures.Http1.FSharp

open System
open System.Globalization
open System.Text
open CodingAdventures.HttpCore.FSharp

exception Http1ParseException of string

type ParsedRequestHead =
    { Head: RequestHead
      BodyOffset: int
      BodyKind: BodyKind }

type ParsedResponseHead =
    { Head: ResponseHead
      BodyOffset: int
      BodyKind: BodyKind }

module Http1 =
    [<Literal>]
    let VERSION = "0.1.0"

    let private raiseParse message =
        raise (Http1ParseException message)

    let private startsWithAt (input: byte array) index (pattern: byte array) =
        if index + pattern.Length > input.Length then
            false
        else
            let mutable same = true
            let mutable offset = 0
            while offset < pattern.Length && same do
                if input[index + offset] <> pattern[offset] then
                    same <- false
                offset <- offset + 1
            same

    let private splitHeadLines (input: byte array) =
        let crlf = [| 13uy; 10uy |]
        let mutable index = 0

        while index < input.Length && (startsWithAt input index crlf || input[index] = 10uy) do
            if startsWithAt input index crlf then
                index <- index + 2
            else
                index <- index + 1

        let lines = ResizeArray<string>()
        let mutable finished = false
        let mutable bodyOffset = 0

        while not finished do
            if index >= input.Length then
                raiseParse "incomplete HTTP/1 head"

            let lineStart = index

            while index < input.Length && input[index] <> 10uy do
                index <- index + 1

            if index >= input.Length then
                raiseParse "incomplete HTTP/1 head"

            let lineEnd =
                if index > lineStart && input[index - 1] = 13uy then
                    index - 1
                else
                    index

            let line = Encoding.Latin1.GetString(input, lineStart, lineEnd - lineStart)
            index <- index + 1

            if line.Length = 0 then
                finished <- true
                bodyOffset <- index
            else
                lines.Add(line)

        List.ofSeq lines, bodyOffset

    let private splitWhitespace (line: string) =
        line.Split([| ' '; '\t' |], StringSplitOptions.RemoveEmptyEntries)

    let private parseHeaders lines =
        lines
        |> List.map (fun (line: string) ->
            let colon = line.IndexOf(':')

            if colon < 0 then
                raiseParse $"invalid HTTP/1 header: {line}"

            let name = line.Substring(0, colon).Trim()

            if name.Length = 0 then
                raiseParse $"invalid HTTP/1 header: {line}"

            { Name = name
              Value = line.Substring(colon + 1).Trim([| ' '; '\t' |]) })

    let private declaredContentLength headers =
        headers
        |> List.tryFind (fun header -> header.Name.Equals("Content-Length", StringComparison.OrdinalIgnoreCase))
        |> Option.map (fun header ->
            let mutable length = 0

            if not (Int32.TryParse(header.Value, NumberStyles.None, CultureInfo.InvariantCulture, &length)) || length < 0 then
                raiseParse $"invalid Content-Length: {header.Value}"

            length)

    let private hasChunkedTransferEncoding headers =
        headers
        |> List.filter (fun header -> header.Name.Equals("Transfer-Encoding", StringComparison.OrdinalIgnoreCase))
        |> List.exists (fun header ->
            header.Value.Split(',')
            |> Array.exists (fun piece -> piece.Trim().Equals("chunked", StringComparison.OrdinalIgnoreCase)))

    let private requestBodyKind headers =
        if hasChunkedTransferEncoding headers then
            BodyKind.Chunked()
        else
            match declaredContentLength headers with
            | None
            | Some 0 -> BodyKind.None()
            | Some length -> BodyKind.ContentLength length

    let private responseBodyKind status headers =
        if (status >= 100us && status < 200us) || status = 204us || status = 304us then
            BodyKind.None()
        elif hasChunkedTransferEncoding headers then
            BodyKind.Chunked()
        else
            match declaredContentLength headers with
            | None -> BodyKind.UntilEof()
            | Some 0 -> BodyKind.None()
            | Some length -> BodyKind.ContentLength length

    let parseRequestHead (input: byte array) =
        let lines, bodyOffset = splitHeadLines input

        match lines with
        | [] -> raiseParse "invalid HTTP/1 start line"
        | startLine :: headerLines ->
            let parts = splitWhitespace startLine

            if parts.Length <> 3 then
                raiseParse $"invalid HTTP/1 start line: {startLine}"

            let version =
                try
                    HttpVersion.Parse(parts[2])
                with :? FormatException ->
                    raiseParse $"invalid HTTP version: {parts[2]}"

            let headers = parseHeaders headerLines

            let head: RequestHead =
                { Method = parts[0]
                  Target = parts[1]
                  Version = version
                  Headers = headers }

            let result: ParsedRequestHead =
                { Head = head
                  BodyOffset = bodyOffset
                  BodyKind = requestBodyKind headers }

            result

    let parseResponseHead (input: byte array) =
        let lines, bodyOffset = splitHeadLines input

        match lines with
        | [] -> raiseParse "invalid HTTP/1 status line"
        | statusLine :: headerLines ->
            let parts = splitWhitespace statusLine

            if parts.Length < 2 then
                raiseParse $"invalid HTTP/1 status line: {statusLine}"

            let version =
                try
                    HttpVersion.Parse(parts[0])
                with :? FormatException ->
                    raiseParse $"invalid HTTP version: {parts[0]}"

            let mutable status = 0us
            if not (UInt16.TryParse(parts[1], NumberStyles.None, CultureInfo.InvariantCulture, &status)) then
                raiseParse $"invalid HTTP status: {parts[1]}"

            let reason =
                if parts.Length > 2 then
                    String.Join(" ", parts[2..])
                else
                    String.Empty

            let headers = parseHeaders headerLines

            let head: ResponseHead =
                { Version = version
                  Status = status
                  Reason = reason
                  Headers = headers }

            let result: ParsedResponseHead =
                { Head = head
                  BodyOffset = bodyOffset
                  BodyKind = responseBodyKind status headers }

            result
