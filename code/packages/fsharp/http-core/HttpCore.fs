namespace CodingAdventures.HttpCore.FSharp

open System
open System.Globalization

type Header = { Name: string; Value: string }

[<Struct>]
type HttpVersion =
    { Major: uint16
      Minor: uint16 }

    static member Parse(text: string) =
        if not (text.StartsWith("HTTP/", StringComparison.Ordinal)) then
            raise (FormatException($"invalid HTTP version: {text}"))

        let rest = text.Substring(5)
        let dot = rest.IndexOf('.')
        let mutable major = 0us
        let mutable minor = 0us

        if dot < 0
           || not (UInt16.TryParse(rest.Substring(0, dot), NumberStyles.None, CultureInfo.InvariantCulture, &major))
           || not (UInt16.TryParse(rest.Substring(dot + 1), NumberStyles.None, CultureInfo.InvariantCulture, &minor)) then
            raise (FormatException($"invalid HTTP version: {text}"))

        { Major = major; Minor = minor }

    override this.ToString() = $"HTTP/{this.Major}.{this.Minor}"

type BodyKind =
    { Mode: string
      Length: int option }

    static member None() = { Mode = "none"; Length = None }

    static member ContentLength(length: int) = { Mode = "content-length"; Length = Some length }

    static member UntilEof() = { Mode = "until-eof"; Length = None }

    static member Chunked() = { Mode = "chunked"; Length = None }

type RouteSegment =
    | Literal of string
    | Param of string

type RoutePattern =
    { Segments: RouteSegment list }

    static member Parse(pattern: string) =
        { Segments =
            HttpCore.splitPathSegments pattern
            |> List.map (fun (segment: string) ->
                if segment.StartsWith(":", StringComparison.Ordinal) then
                    Param(segment.Substring(1))
                else
                    Literal segment) }

    member this.MatchPath(path: string) =
        let pathSegments = HttpCore.splitPathSegments path
        if pathSegments.Length <> this.Segments.Length then
            None
        else
            let mutable failed = false
            let parameters = ResizeArray<string * string>()

            for segment, actual in List.zip this.Segments pathSegments do
                match segment with
                | Literal expected when expected = actual -> ()
                | Literal _ -> failed <- true
                | Param name -> parameters.Add(name, actual)

            if failed then None else Some(List.ofSeq parameters)

and RequestHead =
    { Method: string
      Target: string
      Version: HttpVersion
      Headers: Header list }

    member this.Header(name: string) = HttpCore.findHeader this.Headers name

    member this.ContentLength() = HttpCore.parseContentLength this.Headers

    member this.ContentType() = HttpCore.parseContentType this.Headers

and ResponseHead =
    { Version: HttpVersion
      Status: uint16
      Reason: string
      Headers: Header list }

    member this.Header(name: string) = HttpCore.findHeader this.Headers name

    member this.ContentLength() = HttpCore.parseContentLength this.Headers

    member this.ContentType() = HttpCore.parseContentType this.Headers

and [<RequireQualifiedAccess>] HttpCore =
    static member VERSION = "0.1.0"

    static member findHeader (headers: Header list) (name: string) =
        headers
        |> List.tryFind (fun header -> header.Name.Equals(name, StringComparison.OrdinalIgnoreCase))
        |> Option.map _.Value

    static member parseContentLength headers =
        match HttpCore.findHeader headers "Content-Length" with
        | None -> None
        | Some value ->
            let mutable length = 0
            if Int32.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, &length) && length >= 0 then
                Some length
            else
                None

    static member parseContentType headers =
        match HttpCore.findHeader headers "Content-Type" with
        | None -> None
        | Some value ->
            let pieces = value.Split(';', StringSplitOptions.TrimEntries)
            let mediaType = if pieces.Length = 0 then "" else pieces[0]

            if mediaType.Length = 0 then
                None
            else
                let charset =
                    pieces
                    |> Array.skip 1
                    |> Array.tryPick (fun piece ->
                        let pair = piece.Split('=', 2, StringSplitOptions.TrimEntries)
                        if pair.Length = 2 && pair[0].Equals("charset", StringComparison.OrdinalIgnoreCase) then
                            Some(pair[1].Trim('"'))
                        else
                            None)

                Some(mediaType, charset)

    static member splitPathSegments(path: string) =
        if isNull path then nullArg "path"
        if path = "/" then
            []
        else
            path.Split('/', StringSplitOptions.RemoveEmptyEntries) |> Array.toList
