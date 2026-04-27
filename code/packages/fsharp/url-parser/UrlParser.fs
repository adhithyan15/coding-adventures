namespace CodingAdventures.UrlParser.FSharp

open System
open System.Globalization
open System.Text

[<RequireQualifiedAccess>]
type UrlErrorKind =
    | MissingScheme
    | InvalidScheme
    | InvalidPort
    | InvalidPercentEncoding
    | EmptyHost
    | RelativeWithoutBase

exception UrlParseException of UrlErrorKind * string

module private Helpers =
    let strictUtf8 = UTF8Encoding(false, true)

    let fail kind message = raise (UrlParseException(kind, message))

    let isAsciiLower c = c >= 'a' && c <= 'z'

    let isAsciiLetter c = (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')

    let isSchemeChar c = isAsciiLetter c || Char.IsAsciiDigit c || c = '+' || c = '-' || c = '.'

    let validateScheme (scheme: string) =
        if scheme.Length = 0 || not (isAsciiLower scheme[0]) then
            fail UrlErrorKind.InvalidScheme "scheme must start with a letter"

        for c in scheme do
            if not (isAsciiLower c || Char.IsAsciiDigit c || c = '+' || c = '-' || c = '.') then
                fail UrlErrorKind.InvalidScheme "scheme contains invalid characters"

    let splitFirst (delimiter: char) (input: string) =
        let index = input.IndexOf(delimiter)
        if index >= 0 then
            input.Substring(0, index), Some(input.Substring(index + 1))
        else
            input, None

    let splitFragment input = splitFirst '#' input

    let splitQuery input = splitFirst '?' input

    let parsePort (text: string) =
        let mutable value = 0us
        if UInt16.TryParse(text, NumberStyles.None, CultureInfo.InvariantCulture, &value) then
            value
        else
            fail UrlErrorKind.InvalidPort "port must be between 0 and 65535"

    let defaultPort scheme =
        match scheme with
        | "http" -> Some 80us
        | "https" -> Some 443us
        | "ftp" -> Some 21us
        | _ -> None

    let splitHostAndPort (hostPort: string) =
        if hostPort.StartsWith("[", StringComparison.Ordinal) then
            let bracket = hostPort.IndexOf(']')
            if bracket >= 0 then
                let host = hostPort.Substring(0, bracket + 1)
                let afterBracket = hostPort.Substring(bracket + 1)
                let port =
                    if afterBracket.StartsWith(":", StringComparison.Ordinal) then
                        Some(parsePort (afterBracket.Substring(1)))
                    else
                        None
                host, port
            else
                hostPort, None
        else
            let colon = hostPort.LastIndexOf(':')
            if colon >= 0 then
                let maybePort = hostPort.Substring(colon + 1)
                if maybePort.Length > 0 && maybePort |> Seq.forall Char.IsAsciiDigit then
                    hostPort.Substring(0, colon), Some(parsePort maybePort)
                else
                    hostPort, None
            else
                hostPort, None

    let startsWithScheme (input: string) =
        let colon = input.IndexOf(':')
        if colon <= 0 || input.StartsWith("/", StringComparison.Ordinal) then
            false
        else
            let candidate = input.Substring(0, colon)
            isAsciiLetter candidate[0] && (candidate |> Seq.forall isSchemeChar)

    let mergePaths (basePath: string) (relativePath: string) =
        let slash = basePath.LastIndexOf('/')
        if slash >= 0 then
            basePath.Substring(0, slash + 1) + relativePath
        else
            "/" + relativePath

    let removeDotSegments (path: string) =
        let output = ResizeArray<string>()
        for segment in path.Split('/') do
            match segment with
            | "." -> ()
            | ".." ->
                if output.Count > 0 then
                    output.RemoveAt(output.Count - 1)
            | _ -> output.Add segment

        let result = String.Join("/", output)
        if path.StartsWith("/", StringComparison.Ordinal) && not (result.StartsWith("/", StringComparison.Ordinal)) then
            "/" + result
        else
            result

    let isUnreserved (b: byte) =
        Char.IsAsciiLetterOrDigit(char b)
        || b = byte '-'
        || b = byte '_'
        || b = byte '.'
        || b = byte '~'
        || b = byte '/'

    let percentEncode (input: string) =
        if isNull input then nullArg "input"

        let builder = StringBuilder(input.Length)
        for b in Encoding.UTF8.GetBytes(input) do
            if isUnreserved b then
                builder.Append(char b) |> ignore
            else
                builder.Append('%') |> ignore
                builder.Append(b.ToString("X2", CultureInfo.InvariantCulture)) |> ignore

        builder.ToString()

    let hexDigit c =
        match c with
        | c when c >= '0' && c <= '9' -> byte (int c - int '0')
        | c when c >= 'a' && c <= 'f' -> byte (int c - int 'a' + 10)
        | c when c >= 'A' && c <= 'F' -> byte (int c - int 'A' + 10)
        | _ -> fail UrlErrorKind.InvalidPercentEncoding "invalid hex digit"

    let percentDecode (input: string) =
        if isNull input then nullArg "input"

        let bytes = ResizeArray<byte>(input.Length)
        let mutable i = 0
        while i < input.Length do
            if input[i] = '%' then
                if i + 2 >= input.Length then
                    fail UrlErrorKind.InvalidPercentEncoding "truncated percent escape"

                let hi = hexDigit input[i + 1]
                let lo = hexDigit input[i + 2]
                bytes.Add(byte ((int hi <<< 4) ||| int lo))
                i <- i + 3
            elif Char.IsHighSurrogate(input[i]) && i + 1 < input.Length && Char.IsLowSurrogate(input[i + 1]) then
                bytes.AddRange(Encoding.UTF8.GetBytes(input.Substring(i, 2)))
                i <- i + 2
            else
                bytes.AddRange(Encoding.UTF8.GetBytes(input[i].ToString()))
                i <- i + 1

        try
            strictUtf8.GetString(bytes.ToArray())
        with :? DecoderFallbackException as ex ->
            fail UrlErrorKind.InvalidPercentEncoding ex.Message

type Url =
    { Scheme: string
      Userinfo: string option
      Host: string option
      Port: uint16 option
      Path: string
      Query: string option
      Fragment: string option
      Raw: string }

    static member Parse(input: string) =
        if isNull input then nullArg "input"

        let raw = input
        let input = input.Trim()
        let separator = input.IndexOf("://", StringComparison.Ordinal)

        let scheme, afterScheme, hasAuthority =
            if separator >= 0 then
                input.Substring(0, separator).ToLowerInvariant(), input.Substring(separator + 3), true
            else
                let colon = input.IndexOf(':')
                if colon <= 0 || input.Substring(0, colon).Contains('/') then
                    Helpers.fail UrlErrorKind.MissingScheme "missing scheme"

                input.Substring(0, colon).ToLowerInvariant(), input.Substring(colon + 1), false

        Helpers.validateScheme scheme

        let withoutFragment, fragment = Helpers.splitFragment afterScheme
        let withoutQuery, query = Helpers.splitQuery withoutFragment

        if not hasAuthority then
            { Scheme = scheme
              Userinfo = None
              Host = None
              Port = None
              Path = withoutQuery
              Query = query
              Fragment = fragment
              Raw = raw }
        else
            let slash = withoutQuery.IndexOf('/')
            let authority, path =
                if slash >= 0 then
                    withoutQuery.Substring(0, slash), withoutQuery.Substring(slash)
                else
                    withoutQuery, "/"

            let at = authority.LastIndexOf('@')
            let userinfo, hostPort =
                if at >= 0 then
                    Some(authority.Substring(0, at)), authority.Substring(at + 1)
                else
                    None, authority

            let hostText, port = Helpers.splitHostAndPort hostPort
            let host =
                if String.IsNullOrEmpty hostText then
                    None
                else
                    Some(hostText.ToLowerInvariant())

            { Scheme = scheme
              Userinfo = userinfo
              Host = host
              Port = port
              Path = path
              Query = query
              Fragment = fragment
              Raw = raw }

    member this.Resolve(relative: string) =
        if isNull relative then nullArg "relative"

        let relative = relative.Trim()

        if relative.Length = 0 then
            let result = { this with Fragment = None }
            { result with Raw = result.ToUrlString() }
        elif relative.StartsWith("#", StringComparison.Ordinal) then
            let result = { this with Fragment = Some(relative.Substring(1)) }
            { result with Raw = result.ToUrlString() }
        elif Helpers.startsWithScheme relative then
            Url.Parse relative
        elif relative.StartsWith("//", StringComparison.Ordinal) then
            Url.Parse(this.Scheme + ":" + relative)
        else
            let relativeWithoutFragment, fragment = Helpers.splitFragment relative
            let relativePath, query = Helpers.splitQuery relativeWithoutFragment

            if relativePath.StartsWith("/", StringComparison.Ordinal) then
                let result =
                    { this with
                        Path = Helpers.removeDotSegments relativePath
                        Query = query
                        Fragment = fragment }

                { result with Raw = result.ToUrlString() }
            else
                let merged = Helpers.mergePaths this.Path relativePath
                let result =
                    { this with
                        Path = Helpers.removeDotSegments merged
                        Query = query
                        Fragment = fragment }

                { result with Raw = result.ToUrlString() }

    member this.EffectivePort() =
        match this.Port with
        | Some port -> Some port
        | None -> Helpers.defaultPort this.Scheme

    member this.Authority() =
        let builder = StringBuilder()

        match this.Userinfo with
        | Some userinfo ->
            builder.Append(userinfo) |> ignore
            builder.Append('@') |> ignore
        | None -> ()

        match this.Host with
        | Some host -> builder.Append(host) |> ignore
        | None -> ()

        match this.Port with
        | Some port ->
            builder.Append(':') |> ignore
            builder.Append(port.ToString(CultureInfo.InvariantCulture)) |> ignore
        | None -> ()

        builder.ToString()

    member this.ToUrlString() =
        let builder = StringBuilder()
        builder.Append(this.Scheme) |> ignore

        match this.Host with
        | Some _ ->
            builder.Append("://") |> ignore
            builder.Append(this.Authority()) |> ignore
        | None -> builder.Append(':') |> ignore

        builder.Append(this.Path) |> ignore

        match this.Query with
        | Some query ->
            builder.Append('?') |> ignore
            builder.Append(query) |> ignore
        | None -> ()

        match this.Fragment with
        | Some fragment ->
            builder.Append('#') |> ignore
            builder.Append(fragment) |> ignore
        | None -> ()

        builder.ToString()

    override this.ToString() = this.ToUrlString()

[<RequireQualifiedAccess>]
module UrlParser =
    [<Literal>]
    let VERSION = "0.1.0"

    let parse input = Url.Parse input

    let resolve relative (baseUrl: Url) = baseUrl.Resolve relative

    let effectivePort (url: Url) = url.EffectivePort()

    let authority (url: Url) = url.Authority()

    let toUrlString (url: Url) = url.ToUrlString()

    let percentEncode input = Helpers.percentEncode input

    let percentDecode input = Helpers.percentDecode input
