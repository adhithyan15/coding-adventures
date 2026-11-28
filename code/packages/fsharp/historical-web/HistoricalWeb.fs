namespace CodingAdventures.HistoricalWeb.FSharp

open System
open System.Collections
open System.Collections.Generic
open System.Text

[<RequireQualifiedAccess>]
type HistoricalWebErrorKind =
    | InvalidAddress
    | InvalidRequest
    | InvalidResponse
    | InvalidUnicode

type HistoricalWebException(kind: HistoricalWebErrorKind, offset: int option, message: string) =
    inherit FormatException(message)

    member _.Kind = kind
    member _.Offset = offset

[<RequireQualifiedAccess>]
type HtmlNodeKind =
    | Text
    | Element

[<Struct>]
type HtmlNode =
    { Kind: HtmlNodeKind
      ParentIndex: int
      FirstChildIndex: int
      NextSiblingIndex: int
      Tag: string
      Text: string
      FirstAttributeIndex: int
      AttributeCount: int }

[<Struct>]
type HtmlAttribute =
    { Name: string
      Value: string option }

type private ReadOnlyArrayView<'T>(items: 'T array) =
    interface IReadOnlyList<'T> with
        member _.Item
            with get index = items[index]

    interface IReadOnlyCollection<'T> with
        member _.Count = items.Length

    interface IEnumerable<'T> with
        member _.GetEnumerator() = (items :> IEnumerable<'T>).GetEnumerator()

    interface IEnumerable with
        member _.GetEnumerator() = (items :> IEnumerable).GetEnumerator()

[<Sealed>]
type HtmlDocument internal (nodes: HtmlNode array, attributes: HtmlAttribute array, rootCount: int) =
    let nodeView = ReadOnlyArrayView(nodes) :> IReadOnlyList<HtmlNode>
    let attributeView = ReadOnlyArrayView(attributes) :> IReadOnlyList<HtmlAttribute>

    member _.Nodes = nodeView
    member _.Attributes = attributeView
    member _.FirstRoot = if nodes.Length = 0 then -1 else 0
    member _.RootCount = rootCount
    member _.NodeCount = nodes.Length
    member _.AttributeCount = attributes.Length

    member _.NodeAt(index: int) =
        if index < 0 || index >= nodes.Length then
            raise (ArgumentOutOfRangeException(nameof index))

        nodes[index]

    member _.AttributeAt(index: int) =
        if index < 0 || index >= attributes.Length then
            raise (ArgumentOutOfRangeException(nameof index))

        attributes[index]

[<RequireQualifiedAccess>]
module Http09 =
    let private prefixLength = 4

    let private prefixByte index =
        match index with
        | 0 -> byte 'G'
        | 1 -> byte 'E'
        | 2 -> byte 'T'
        | _ -> byte ' '

    let private invalidRequest offset =
        HistoricalWebException(
            HistoricalWebErrorKind.InvalidRequest,
            Some offset,
            "Malformed HTTP/0.9 request.")

    let encodeRequest (address: ReadOnlySpan<byte>) =
        if address.IsEmpty then
            raise (HistoricalWebException(HistoricalWebErrorKind.InvalidAddress, None, "HTTP/0.9 address must not be empty."))

        for index in 0 .. address.Length - 1 do
            if address[index] < 0x21uy || address[index] > 0x7euy then
                raise (HistoricalWebException(HistoricalWebErrorKind.InvalidAddress, Some index, "HTTP/0.9 address must contain printable ASCII bytes."))

        // Check both additions before allocating or forming output slices.
        if address.Length > Array.MaxLength - 6 then
            raise (OverflowException("HTTP/0.9 request is too large for a managed array."))

        let addressEnd = Checked.(+) prefixLength address.Length
        let requestLength = Checked.(+) addressEnd 2
        let request = Array.zeroCreate<byte> requestLength
        request[0] <- prefixByte 0
        request[1] <- prefixByte 1
        request[2] <- prefixByte 2
        request[3] <- prefixByte 3
        address.CopyTo(request.AsSpan(prefixLength, address.Length))
        request[addressEnd] <- 13uy
        request[addressEnd + 1] <- 10uy
        request

    let parseRequest (request: ReadOnlySpan<byte>) =
        let compared = min request.Length prefixLength

        for index in 0 .. compared - 1 do
            if request[index] <> prefixByte index then
                raise (invalidRequest index)

        if request.Length < prefixLength then
            raise (invalidRequest request.Length)

        let framedLength =
            if request.Length >= 2 && request[request.Length - 2] = 13uy && request[request.Length - 1] = 10uy then
                request.Length - 2
            elif request.Length >= 1 && request[request.Length - 1] = 10uy then
                request.Length - 1
            else
                raise (invalidRequest request.Length)

        if framedLength <= prefixLength then
            raise (invalidRequest prefixLength)

        let addressLength = framedLength - prefixLength
        let address = request.Slice(prefixLength, addressLength)

        for index in 0 .. address.Length - 1 do
            if address[index] < 0x21uy || address[index] > 0x7euy then
                raise (invalidRequest (prefixLength + index))

        address.ToArray()

    let decodeResponse (body: ReadOnlySpan<byte>) =
        for index in 0 .. body.Length - 1 do
            if body[index] > 0x7fuy then
                raise (HistoricalWebException(HistoricalWebErrorKind.InvalidResponse, Some index, "HTTP/0.9 response body must contain ASCII bytes."))

        if body.IsEmpty then
            String.Empty
        else
            Encoding.ASCII.GetString(body)

[<RequireQualifiedAccess>]
module HtmlTags1992 =
    [<Literal>]
    let private noIndex = -1

    type private Tag =
        | Unknown = 0
        | Title = 1
        | Address = 2
        | Anchor = 3
        | H1 = 4
        | H2 = 5
        | H3 = 6
        | H4 = 7
        | H5 = 8
        | H6 = 9
        | Ul = 10
        | Menu = 11
        | Dir = 12
        | Li = 13
        | Dl = 14
        | Dt = 15
        | Dd = 16
        | Paragraph = 17
        | Isindex = 18
        | Xmp = 19
        | Listing = 20
        | Plaintext = 21

    let private tagNames =
        [| ""; "title"; "address"; "a"; "h1"; "h2"; "h3"; "h4"; "h5"; "h6"
           "ul"; "menu"; "dir"; "li"; "dl"; "dt"; "dd"; "p"; "isindex"
           "xmp"; "listing"; "plaintext" |]

    let private entities = [| "&lt;", '<'; "&gt;", '>'; "&amp;", '&' |]

    let private tagIndex tag = int tag

    let private isEmpty tag = tag = Tag.Paragraph || tag = Tag.Isindex

    let private isHeading tag = tag >= Tag.H1 && tag <= Tag.H6

    let private isAsciiAlpha value =
        (value >= 'A' && value <= 'Z') || (value >= 'a' && value <= 'z')

    let private isNameCharacter value =
        isAsciiAlpha value || (value >= '0' && value <= '9') || value = '-'

    let private isTagSpace value =
        value = ' ' || value = '\t' || value = '\n' || value = '\r' || value = '\f'

    let private lowerAscii value =
        if value >= 'A' && value <= 'Z' then char (int value + int 'a' - int 'A') else value

    let private equalsAsciiIgnoreCase (source: string) start (needle: string) =
        if start < 0 || needle.Length > source.Length - start then
            false
        else
            let mutable matches = true
            let mutable index = 0
            while matches && index < needle.Length do
                if lowerAscii source[start + index] <> needle[index] then
                    matches <- false
                index <- index + 1
            matches

    let private identifyTag (source: string) start length =
        let mutable found = Tag.Unknown
        let mutable index = 1
        while found = Tag.Unknown && index < tagNames.Length do
            if length = tagNames[index].Length && equalsAsciiIgnoreCase source start tagNames[index] then
                found <- enum<Tag> index
            index <- index + 1
        found

    let private validateUnicode (source: string) =
        let mutable index = 0
        while index < source.Length do
            let current = source[index]
            if Char.IsHighSurrogate(current) then
                if index + 1 >= source.Length || not (Char.IsLowSurrogate(source[index + 1])) then
                    raise (HistoricalWebException(HistoricalWebErrorKind.InvalidUnicode, Some index, "HTML source contains an unpaired UTF-16 high surrogate."))
                index <- index + 2
            elif Char.IsLowSurrogate(current) then
                raise (HistoricalWebException(HistoricalWebErrorKind.InvalidUnicode, Some index, "HTML source contains an unpaired UTF-16 low surrogate."))
            else
                index <- index + 1

    let private normalizeCrLf (source: string) =
        if not (source.Contains("\r\n", StringComparison.Ordinal)) then
            source
        else
            let builder = StringBuilder(source.Length)
            let mutable index = 0
            while index < source.Length do
                if source[index] = '\r' && index + 1 < source.Length && source[index + 1] = '\n' then
                    builder.Append('\n') |> ignore
                    index <- index + 2
                else
                    builder.Append(source[index]) |> ignore
                    index <- index + 1
            builder.ToString()

    type private ParsedTag =
        { Tag: Tag
          Closing: bool
          AttributesStart: int
          AttributesLength: int
          End: int }

    let private tryParseTag (source: string) start =
        let mutable cursor = start + 1
        let closing = cursor < source.Length && source[cursor] = '/'
        if closing then cursor <- cursor + 1

        if cursor >= source.Length || not (isAsciiAlpha source[cursor]) then
            None
        else
            let nameStart = cursor
            cursor <- cursor + 1
            while cursor < source.Length && isNameCharacter source[cursor] do
                cursor <- cursor + 1
            let tag = identifyTag source nameStart (cursor - nameStart)

            if cursor >= source.Length || source[cursor] = '<' then
                None
            elif source[cursor] = '>' then
                Some { Tag = tag; Closing = closing; AttributesStart = cursor; AttributesLength = 0; End = cursor + 1 }
            elif not (isTagSpace source[cursor]) then
                None
            else
                let attributesStart = cursor
                let mutable result = None
                let mutable failed = false

                while cursor < source.Length && result.IsNone && not failed do
                    let current = source[cursor]
                    if current = '>' then
                        result <- Some { Tag = tag; Closing = closing; AttributesStart = attributesStart; AttributesLength = cursor - attributesStart; End = cursor + 1 }
                    elif current = '<' then
                        failed <- true
                    elif current = '\'' || current = '"' then
                        let quote = current
                        cursor <- cursor + 1
                        while cursor < source.Length && source[cursor] <> quote && source[cursor] <> '<' do
                            cursor <- cursor + 1
                        if cursor >= source.Length || source[cursor] <> quote then
                            failed <- true
                        else
                            cursor <- cursor + 1
                    else
                        cursor <- cursor + 1

                if failed then None else result

    type private ParsedAttribute =
        { Name: string
          Value: string option
          Next: int }

    let private tryParseAttribute (source: string) start length position =
        let limit = start + length
        let mutable cursor = position
        while cursor < limit && isTagSpace source[cursor] do cursor <- cursor + 1

        if cursor = position || cursor >= limit || not (isAsciiAlpha source[cursor]) then
            None
        else
            let nameStart = cursor
            cursor <- cursor + 1
            while cursor < limit && isNameCharacter source[cursor] do cursor <- cursor + 1
            let name = source.Substring(nameStart, cursor - nameStart)
            let valueStart = cursor
            while cursor < limit && isTagSpace source[cursor] do cursor <- cursor + 1

            if cursor >= limit || source[cursor] <> '=' then
                Some { Name = name; Value = None; Next = valueStart }
            else
                cursor <- cursor + 1
                while cursor < limit && isTagSpace source[cursor] do cursor <- cursor + 1

                if cursor >= limit then
                    Some { Name = name; Value = None; Next = valueStart }
                elif source[cursor] = '\'' || source[cursor] = '"' then
                    let quote = source[cursor]
                    cursor <- cursor + 1
                    let valueStart = cursor
                    while cursor < limit && source[cursor] <> quote do cursor <- cursor + 1
                    if cursor >= limit then
                        None
                    else
                        Some { Name = name; Value = Some(source.Substring(valueStart, cursor - valueStart)); Next = cursor + 1 }
                else
                    let unquotedStart = cursor
                    while cursor < limit && not (isTagSpace source[cursor]) &&
                          source[cursor] <> '"' && source[cursor] <> '\'' && source[cursor] <> '=' &&
                          source[cursor] <> '<' && source[cursor] <> '>' do
                        cursor <- cursor + 1
                    if cursor = unquotedStart then
                        Some { Name = name; Value = None; Next = valueStart }
                    else
                        Some { Name = name; Value = Some(source.Substring(unquotedStart, cursor - unquotedStart)); Next = cursor }

    let private appendDecoded (builder: StringBuilder) (source: string) start length =
        let finish = start + length
        let mutable cursor = start
        while cursor < finish do
            let mutable matched = false
            if source[cursor] = '&' then
                let mutable entity = 0
                while not matched && entity < entities.Length do
                    let name, value = entities[entity]
                    if name.Length <= finish - cursor && String.CompareOrdinal(source, cursor, name, 0, name.Length) = 0 then
                        builder.Append(value) |> ignore
                        cursor <- cursor + name.Length
                        matched <- true
                    entity <- entity + 1
            if not matched then
                builder.Append(source[cursor]) |> ignore
                cursor <- cursor + 1

    type private NodeMeta =
        { PreviousOpen: int
          StackSlot: int
          LastChild: int
          Tag: Tag }

    type private Parser(source: string) =
        let nodes = ResizeArray<HtmlNode>()
        let attributes = ResizeArray<HtmlAttribute>()
        let metadata = ResizeArray<NodeMeta>()
        let openStack = ResizeArray<int>()
        let latestOpen = Array.create tagNames.Length noIndex
        let pendingText = StringBuilder()
        let mutable rootCount = 0
        let mutable lastRoot = noIndex
        let mutable rawTag = Tag.Unknown

        let appendNode (node: HtmlNode) tag =
            let index = nodes.Count
            nodes.Add(node)
            metadata.Add({ PreviousOpen = latestOpen[tagIndex tag]; StackSlot = openStack.Count; LastChild = noIndex; Tag = tag })

            if node.ParentIndex <> noIndex then
                let parent = nodes[node.ParentIndex]
                if parent.FirstChildIndex = noIndex then
                    nodes[node.ParentIndex] <- { parent with FirstChildIndex = index }
                else
                    let previousIndex = metadata[node.ParentIndex].LastChild
                    let previous = nodes[previousIndex]
                    nodes[previousIndex] <- { previous with NextSiblingIndex = index }
                metadata[node.ParentIndex] <- { metadata[node.ParentIndex] with LastChild = index }
            else
                if lastRoot <> noIndex then
                    let previous = nodes[lastRoot]
                    nodes[lastRoot] <- { previous with NextSiblingIndex = index }
                lastRoot <- index
                rootCount <- rootCount + 1

            if tag <> Tag.Unknown && not (isEmpty tag) then
                latestOpen[tagIndex tag] <- index
            index

        let appendText (start: int) (length: int) decodeEntities =
            if length > 0 then
                if decodeEntities then appendDecoded pendingText source start length
                else pendingText.Append(source, start, length) |> ignore

        let flushText () =
            if pendingText.Length > 0 then
                let parent = if openStack.Count = 0 then noIndex else openStack[openStack.Count - 1]
                let node =
                    { Kind = HtmlNodeKind.Text
                      ParentIndex = parent
                      FirstChildIndex = noIndex
                      NextSiblingIndex = noIndex
                      Tag = String.Empty
                      Text = pendingText.ToString()
                      FirstAttributeIndex = noIndex
                      AttributeCount = 0 }
                appendNode node Tag.Unknown |> ignore
                pendingText.Clear() |> ignore

        let matching tag boundaries =
            let candidate = latestOpen[tagIndex tag]
            if candidate = noIndex then
                noIndex
            else
                // A missing tag is a sentinel, not a stack position.
                let mutable newestBoundary = noIndex
                for boundary in boundaries do
                    let openBoundary = latestOpen[tagIndex boundary]
                    if openBoundary <> noIndex &&
                       (newestBoundary = noIndex || metadata[openBoundary].StackSlot > metadata[newestBoundary].StackSlot) then
                        newestBoundary <- openBoundary
                if newestBoundary <> noIndex && metadata[candidate].StackSlot <= metadata[newestBoundary].StackSlot then
                    noIndex
                else
                    metadata[candidate].StackSlot

        let closeTo stackSlot =
            while openStack.Count > stackSlot do
                let last = openStack[openStack.Count - 1]
                openStack.RemoveAt(openStack.Count - 1)
                latestOpen[tagIndex metadata[last].Tag] <- metadata[last].PreviousOpen

        let decodeAttribute (value: string) =
            let builder = StringBuilder(value.Length)
            appendDecoded builder value 0 value.Length
            builder.ToString()

        let parseAnchorAttributes start length =
            let result = ResizeArray<HtmlAttribute>()
            let seen = Array.create 3 false
            // `cursor`, ParsedAttribute.Next, and this range use absolute
            // source indexes; only `finish` bounds the attribute scanner.
            let mutable cursor = start
            let finish = start + length
            let mutable continueParsing = true
            while cursor < finish && continueParsing do
                match tryParseAttribute source start length cursor with
                | None -> continueParsing <- false
                | Some parsed ->
                    let key =
                        if String.Equals(parsed.Name, "href", StringComparison.OrdinalIgnoreCase) then 0
                        elif String.Equals(parsed.Name, "name", StringComparison.OrdinalIgnoreCase) then 1
                        elif String.Equals(parsed.Name, "type", StringComparison.OrdinalIgnoreCase) then 2
                        else -1
                    if key >= 0 && not seen[key] then
                        seen[key] <- true
                        let name = if key = 0 then "href" elif key = 1 then "name" else "type"
                        let value = parsed.Value |> Option.map decodeAttribute
                        result.Add({ Name = name; Value = value })
                    if parsed.Next <= cursor then continueParsing <- false
                    else cursor <- parsed.Next
            result.ToArray()

        let handleTag tag closing attributesStart attributesLength =
            let mutable closeSlot = noIndex
            if closing then
                closeSlot <- matching tag []
            elif tag = Tag.Li then
                closeSlot <- matching Tag.Li [ Tag.Ul; Tag.Menu; Tag.Dir ]
            elif tag = Tag.Dt || tag = Tag.Dd then
                let term = matching Tag.Dt [ Tag.Dl ]
                let definition = matching Tag.Dd [ Tag.Dl ]
                closeSlot <-
                    if term = noIndex then definition
                    elif definition = noIndex then term
                    else max term definition
            elif isHeading tag then
                for heading in [ Tag.H1; Tag.H2; Tag.H3; Tag.H4; Tag.H5; Tag.H6 ] do
                    let candidate = matching heading []
                    if candidate <> noIndex && (closeSlot = noIndex || candidate > closeSlot) then
                        closeSlot <- candidate

            if closing && closeSlot = noIndex then
                false
            else
                flushText()
                if closeSlot <> noIndex then closeTo closeSlot
                if closing then
                    true
                else
                    let parent = if openStack.Count = 0 then noIndex else openStack[openStack.Count - 1]
                    let parsedAttributes =
                        if tag = Tag.Anchor then parseAnchorAttributes attributesStart attributesLength
                        else Array.empty<HtmlAttribute>
                    let firstAttribute = if parsedAttributes.Length = 0 then noIndex else attributes.Count
                    for attribute in parsedAttributes do attributes.Add(attribute)
                    let node =
                        { Kind = HtmlNodeKind.Element
                          ParentIndex = parent
                          FirstChildIndex = noIndex
                          NextSiblingIndex = noIndex
                          Tag = tagNames[tagIndex tag]
                          Text = String.Empty
                          FirstAttributeIndex = firstAttribute
                          AttributeCount = parsedAttributes.Length }
                    let index = appendNode node (if isEmpty tag then Tag.Unknown else tag)
                    if not (isEmpty tag) then openStack.Add(index)
                    true

        let findRawEnd cursor tag =
            let needle = "</" + tagNames[tagIndex tag] + ">"
            let mutable search = cursor
            let mutable found = None
            while found.IsNone do
                let start = source.IndexOf("</", search, StringComparison.Ordinal)
                if start < 0 then
                    found <- Some None
                elif needle.Length <= source.Length - start && equalsAsciiIgnoreCase source start needle then
                    found <- Some(Some(start, start + needle.Length))
                else
                    search <- start + 2
            found.Value

        member _.Run() =
            let mutable cursor = 0
            while cursor < source.Length do
                if rawTag <> Tag.Unknown then
                    if rawTag = Tag.Plaintext then
                        appendText cursor (source.Length - cursor) false
                        cursor <- source.Length
                    else
                        match findRawEnd cursor rawTag with
                        | None ->
                            appendText cursor (source.Length - cursor) false
                            cursor <- source.Length
                        | Some(start, finish) ->
                            appendText cursor (start - cursor) false
                            cursor <- finish
                            let ended = rawTag
                            rawTag <- Tag.Unknown
                            handleTag ended true 0 0 |> ignore
                else
                    let start = source.IndexOf('<', cursor)
                    if start < 0 then
                        appendText cursor (source.Length - cursor) true
                        cursor <- source.Length
                    else
                        appendText cursor (start - cursor) true
                        if source.AsSpan(start).StartsWith("<!--".AsSpan(), StringComparison.Ordinal) then
                            let commentEnd = source.IndexOf("-->", start + 4, StringComparison.Ordinal)
                            if commentEnd < 0 then cursor <- source.Length
                            else cursor <- commentEnd + 3
                        else
                            match tryParseTag source start with
                            | None ->
                                appendText start 1 true
                                cursor <- start + 1
                            | Some parsed ->
                                cursor <- parsed.End
                                if parsed.Tag <> Tag.Unknown && not (parsed.Closing && isEmpty parsed.Tag) then
                                    handleTag parsed.Tag parsed.Closing parsed.AttributesStart parsed.AttributesLength |> ignore
                                    if not parsed.Closing && parsed.Tag = Tag.Plaintext then
                                        rawTag <- Tag.Plaintext
                                    elif not parsed.Closing && (parsed.Tag = Tag.Xmp || parsed.Tag = Tag.Listing) then
                                        rawTag <- parsed.Tag

            flushText()
            HtmlDocument(nodes.ToArray(), attributes.ToArray(), rootCount)

    let parse (source: string) =
        if isNull source then nullArg (nameof source)
        validateUnicode source
        let normalized = normalizeCrLf source
        Parser(normalized).Run()
