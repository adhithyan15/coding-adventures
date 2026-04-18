namespace CodingAdventures.CommonmarkParser.FSharp

open System
open System.Net
open System.Text
open System.Text.RegularExpressions
open System.Collections.Generic
open CodingAdventures.DocumentAst.FSharp

module private Common =
    let fenceRegex = Regex(@"^\s{0,3}(?<fence>`{3,}|~{3,})(?<info>.*)$", RegexOptions.Compiled)
    let atxHeadingRegex = Regex(@"^\s{0,3}(?<marks>#{1,6})[ \t]+(?<text>.+?)\s*#*\s*$", RegexOptions.Compiled)
    let orderedListRegex = Regex(@"^\s{0,3}(?<num>\d+)[.)][ \t]+(?<text>.*)$", RegexOptions.Compiled)
    let bulletListRegex = Regex(@"^\s{0,3}[-+*][ \t]+(?<text>.*)$", RegexOptions.Compiled)
    let taskMarkerRegex = Regex(@"^\[(?<mark>[ xX])\][ \t]+(?<text>.*)$", RegexOptions.Compiled)
    let tableDelimiterCellRegex = Regex(@"^:?-{3,}:?$", RegexOptions.Compiled)
    let destinationWithTitleRegex = Regex(@"^(?<dest>\S+)(?:\s+[""'](?<title>.*)[""'])?$", RegexOptions.Compiled)

    let normalize (markdown: string) = markdown.Replace("\r\n", "\n").Replace('\r', '\n')
    let isBlank (line: string) = String.IsNullOrWhiteSpace line

    let isThematicBreak (line: string) =
        let trimmed = line |> Seq.filter (fun ch -> not (Char.IsWhiteSpace ch)) |> Seq.toArray |> String
        if trimmed.Length < 3 then
            false
        else
            let first = trimmed[0]
            (first = '-' || first = '*' || first = '_')
            && (trimmed |> Seq.forall ((=) first))

    let isBlockquoteLine (line: string) =
        line.TrimStart().StartsWith(">", StringComparison.Ordinal)

    let stripBlockquoteMarker (line: string) =
        let trimmed = line.TrimStart()
        if trimmed.Length > 1 && trimmed[1] = ' ' then trimmed.Substring(2) else trimmed.Substring(1)

    let tryParseListItemStart (line: string) =
        let orderedMatch = orderedListRegex.Match(line)
        if orderedMatch.Success then
            match Int32.TryParse orderedMatch.Groups["num"].Value with
            | true, parsedStart -> Some(true, parsedStart, orderedMatch.Groups["text"].Value)
            | false, _ -> None
        else
            let bulletMatch = bulletListRegex.Match(line)
            if bulletMatch.Success then
                Some(false, 0, bulletMatch.Groups["text"].Value)
            else
                None

    let stripContinuationIndent (line: string) =
        let mutable spaces = 0
        while spaces < line.Length && spaces < 4 && line[spaces] = ' ' do
            spaces <- spaces + 1
        line.Substring(spaces)

    let tryStripTaskMarker (text: string) =
        let match' = taskMarkerRegex.Match(text)
        if match'.Success then
            Some(match'.Groups["mark"].Value.Equals("x", StringComparison.OrdinalIgnoreCase), match'.Groups["text"].Value)
        else
            None

    let isSetextUnderline (line: string) =
        let trimmed = line.Trim()
        if trimmed.Length >= 3 && trimmed |> Seq.forall ((=) '=') then Some 1
        elif trimmed.Length >= 3 && trimmed |> Seq.forall ((=) '-') then Some 2
        else None

    let trySplitTableRow (line: string) =
        let mutable trimmed = line.Trim()
        if not (trimmed.Contains("|", StringComparison.Ordinal)) then
            None
        else
            if trimmed.StartsWith("|", StringComparison.Ordinal) then
                trimmed <- trimmed.Substring(1)
            if trimmed.EndsWith("|", StringComparison.Ordinal) then
                trimmed <- trimmed.Substring(0, trimmed.Length - 1)
            Some(trimmed.Split('|') |> Array.map (fun cell -> cell.Trim()) |> Array.toList)

    let tryParseDelimiterRow (line: string) =
        match trySplitTableRow line with
        | None -> None
        | Some cells ->
            let mutable valid = true
            let parsed =
                cells
                |> List.map (fun cell ->
                    if not (tableDelimiterCellRegex.IsMatch cell) then
                        valid <- false
                        None
                    else
                        let left = cell.StartsWith(":")
                        let right = cell.EndsWith(":")
                        if left && right then Some Center
                        elif left then Some Left
                        elif right then Some Right
                        else None)
            if valid then Some parsed else None

    let looksLikeRawHtmlBlock (line: string) =
        let trimmed = line.Trim()
        if not (trimmed.StartsWith("<", StringComparison.Ordinal) && trimmed.EndsWith(">", StringComparison.Ordinal)) then
            false
        else
            let inner = trimmed.Substring(1, trimmed.Length - 2).Trim()
            if inner.Length = 0 then
                false
            elif inner.StartsWith("http://", StringComparison.OrdinalIgnoreCase)
                || inner.StartsWith("https://", StringComparison.OrdinalIgnoreCase)
                || inner.StartsWith("mailto:", StringComparison.OrdinalIgnoreCase) then
                false
            elif inner.Contains('@') && not (inner.Contains('/')) && not (inner.Contains(' ')) then
                false
            else
                let first = inner[0]
                Char.IsLetter(first) || first = '/' || first = '!' || first = '?'

module private ParserLimits =
    let maxInputLength = 100_000
    let maxParseDepth = 64

    let ensureWithinLimits (inputLength: int) (depth: int) =
        if inputLength > maxInputLength then
            invalidOp $"Markdown input exceeds the supported size limit of {maxInputLength} characters."
        elif depth > maxParseDepth then
            invalidOp $"Markdown nesting exceeds the supported depth limit of {maxParseDepth}."

module private InlineParsing =
    let private maxInlineSearchWindow = 4096

    let private startsWithAt (text: string) index (value: string) =
        index + value.Length <= text.Length && text.Substring(index, value.Length).Equals(value, StringComparison.Ordinal)

    let private findStringWithinWindow (text: string) (value: string) start =
        let searchLength = min (text.Length - start) maxInlineSearchWindow
        if searchLength <= 0 then
            -1
        else
            text.IndexOf(value, start, searchLength, StringComparison.Ordinal)

    let private findCharWithinWindow (text: string) (value: char) start window =
        let searchLength = min (text.Length - start) window
        if searchLength <= 0 then
            -1
        else
            text.IndexOf(value, start, searchLength)

    let private removeTrailingBreakSpaces (buffer: StringBuilder) =
        let mutable count = 0
        let mutable i = buffer.Length - 1
        while i >= 0 && buffer[i] = ' ' do
            count <- count + 1
            i <- i - 1
        if count >= 2 then
            buffer.Length <- buffer.Length - 2
            true
        else
            false

    let private flushText (buffer: StringBuilder) (nodes: ResizeArray<InlineNode>) =
        if buffer.Length > 0 then
            let value = WebUtility.HtmlDecode(buffer.ToString())
            if value.Length > 0 then
                nodes.Add(TextNode value)
            buffer.Clear() |> ignore

    let rec private toPlainText nodes =
        nodes
        |> List.map (function
            | TextNode value -> value
            | EmphasisNode children -> toPlainText children
            | StrongNode children -> toPlainText children
            | StrikethroughNode children -> toPlainText children
            | LinkNode (_, _, children) -> toPlainText children
            | ImageNode (_, _, alt) -> alt
            | CodeSpanNode value -> value
            | AutolinkNode (destination, _) -> destination
            | RawInlineNode (_, value) -> value
            | HardBreakNode
            | SoftBreakNode -> " ")
        |> String.concat ""

    let private tryDecodeEntity (text: string) (index: byref<int>) =
        if text[index] <> '&' then
            None
        else
            let end' = findCharWithinWindow text ';' (index + 1) 16
            if end' < 0 || end' - index > 16 then
                None
            else
                let candidate = text.Substring(index, end' - index + 1)
                let resolved = WebUtility.HtmlDecode(candidate)
                if resolved = candidate then
                    None
                else
                    index <- end' + 1
                    Some resolved

    let private findClosingBracket (text: string) start =
        let mutable depth = 0
        let mutable result = -1
        let mutable i = start
        let limit = min text.Length (start + maxInlineSearchWindow)
        while i < limit && result < 0 do
            if text[i] = '[' then depth <- depth + 1
            elif text[i] = ']' then
                if depth = 0 then result <- i else depth <- depth - 1
            i <- i + 1
        result

    let private findClosingParen (text: string) start =
        let mutable depth = 1
        let mutable result = -1
        let mutable i = start
        let limit = min text.Length (start + maxInlineSearchWindow)
        while i < limit && result < 0 do
            if text[i] = '(' then depth <- depth + 1
            elif text[i] = ')' then
                depth <- depth - 1
                if depth = 0 then result <- i
            i <- i + 1
        result

    let private tryParseDestinationAndTitle (target: string) =
        let trimmed = target.Trim()
        if trimmed.Length = 0 then
            None
        elif trimmed.StartsWith("<", StringComparison.Ordinal) && trimmed.EndsWith(">", StringComparison.Ordinal) then
            let destination = trimmed.Substring(1, trimmed.Length - 2)
            if destination.Length = 0 then None else Some(destination, None)
        else
            let match' = Common.destinationWithTitleRegex.Match(trimmed)
            if not match'.Success then
                None
            else
                let destination = match'.Groups["dest"].Value
                if destination.Length = 0 then
                    None
                else
                    let title =
                        if match'.Groups["title"].Success then Some match'.Groups["title"].Value else None
                    Some(destination, title)

    let rec parseAtDepth (depth: int) (text: string) (enableGfm: bool) =
        ParserLimits.ensureWithinLimits text.Length depth
        let nodes = ResizeArray<InlineNode>()
        let buffer = StringBuilder()
        let mutable index = 0

        let tryDelimited delimiter factory =
            if not (startsWithAt text index delimiter) then
                false
            else
                let start = index + delimiter.Length
                let end' = findStringWithinWindow text delimiter start
                if end' < 0 || end' = start then
                    false
                else
                    flushText buffer nodes
                    nodes.Add(factory (text.Substring(start, end' - start)))
                    index <- end' + delimiter.Length
                    true

        let tryCodeSpan () =
            if text[index] <> '`' then
                false
            else
                let mutable tickCount = 1
                while index + tickCount < text.Length && text[index + tickCount] = '`' do
                    tickCount <- tickCount + 1
                let delimiter = String('`', tickCount)
                let end' = findStringWithinWindow text delimiter (index + tickCount)
                if end' < 0 then
                    false
                else
                    let mutable content = text.Substring(index + tickCount, end' - index - tickCount)
                    if content.Length > 1 && content.StartsWith(" ") && content.EndsWith(" ") then
                        content <- content.Substring(1, content.Length - 2)
                    flushText buffer nodes
                    nodes.Add(CodeSpanNode content)
                    index <- end' + tickCount
                    true

        let tryLink () =
            if text[index] <> '[' then
                false
            else
                let closingBracket = findClosingBracket text (index + 1)
                if closingBracket < 0 || closingBracket + 1 >= text.Length || text[closingBracket + 1] <> '(' then
                    false
                else
                    let closingParen = findClosingParen text (closingBracket + 2)
                    if closingParen < 0 then
                        false
                    else
                        let label = text.Substring(index + 1, closingBracket - index - 1)
                        let target = text.Substring(closingBracket + 2, closingParen - closingBracket - 2)
                        match tryParseDestinationAndTitle target with
                        | None -> false
                        | Some(destination, title) ->
                            flushText buffer nodes
                            nodes.Add(LinkNode(destination, title, parseAtDepth (depth + 1) label enableGfm))
                            index <- closingParen + 1
                            true

        let tryImage () =
            if text[index] <> '!' || index + 1 >= text.Length || text[index + 1] <> '[' then
                false
            else
                let imageIndex = index + 1
                let closingBracket = findClosingBracket text (imageIndex + 1)
                if closingBracket < 0 || closingBracket + 1 >= text.Length || text[closingBracket + 1] <> '(' then
                    false
                else
                    let closingParen = findClosingParen text (closingBracket + 2)
                    if closingParen < 0 then
                        false
                    else
                        let label = text.Substring(imageIndex + 1, closingBracket - imageIndex - 1)
                        let target = text.Substring(closingBracket + 2, closingParen - closingBracket - 2)
                        match tryParseDestinationAndTitle target with
                        | None -> false
                        | Some(destination, title) ->
                            flushText buffer nodes
                            nodes.Add(ImageNode(destination, title, toPlainText (parseAtDepth (depth + 1) label enableGfm)))
                            index <- closingParen + 1
                            true

        let tryAngle () =
            if text[index] <> '<' then
                false
            else
                let end' = findCharWithinWindow text '>' (index + 1) maxInlineSearchWindow
                if end' < 0 then
                    false
                else
                    let inner = text.Substring(index + 1, end' - index - 1)
                    if inner.Length = 0 || inner.Contains(' ') then
                        false
                    else
                        flushText buffer nodes
                        if inner.StartsWith("http://", StringComparison.OrdinalIgnoreCase)
                            || inner.StartsWith("https://", StringComparison.OrdinalIgnoreCase) then
                            nodes.Add(AutolinkNode(inner, false))
                        elif inner.StartsWith("mailto:", StringComparison.OrdinalIgnoreCase) then
                            nodes.Add(AutolinkNode(inner.Substring("mailto:".Length), true))
                        elif inner.Contains('@') then
                            nodes.Add(AutolinkNode(inner, true))
                        else
                            nodes.Add(RawInlineNode("html", $"<{inner}>"))
                        index <- end' + 1
                        true

        while index < text.Length do
            if text[index] = '\\' && index + 1 < text.Length && text[index + 1] = '\n' then
                flushText buffer nodes
                nodes.Add HardBreakNode
                index <- index + 2
            elif text[index] = '\n' then
                let hardBreak = removeTrailingBreakSpaces buffer
                flushText buffer nodes
                nodes.Add(if hardBreak then HardBreakNode else SoftBreakNode)
                index <- index + 1
            elif
                tryImage ()
                || tryLink ()
                || tryCodeSpan ()
                || tryDelimited "**" (fun content -> StrongNode(parseAtDepth (depth + 1) content enableGfm))
                || tryDelimited "__" (fun content -> StrongNode(parseAtDepth (depth + 1) content enableGfm))
                || tryDelimited "*" (fun content -> EmphasisNode(parseAtDepth (depth + 1) content enableGfm))
                || tryDelimited "_" (fun content -> EmphasisNode(parseAtDepth (depth + 1) content enableGfm))
                || (enableGfm && tryDelimited "~~" (fun content -> StrikethroughNode(parseAtDepth (depth + 1) content enableGfm)))
                || tryAngle ()
            then
                ()
            else
                match tryDecodeEntity text &index with
                | Some decoded -> buffer.Append(decoded) |> ignore
                | None ->
                    buffer.Append(text[index]) |> ignore
                    index <- index + 1

        flushText buffer nodes
        List.ofSeq nodes

    let parse (text: string) (enableGfm: bool) = parseAtDepth 0 text enableGfm

type private BlockParser(markdown: string, enableGfm: bool, depth: int) =
    let lines = Common.normalize markdown |> fun text -> text.Split('\n')
    let mutable index = 0

    member private _.Current = lines[index]

    static member ParseMarkdown(markdown: string, enableGfm: bool, depth: int) : DocumentNode =
        let normalized = if isNull markdown then "" else markdown
        ParserLimits.ensureWithinLimits normalized.Length depth
        let parser = BlockParser(normalized, enableGfm, depth)
        parser.ParseDocument()

    static member ParseMarkdown(markdown: string, enableGfm: bool) : DocumentNode =
        BlockParser.ParseMarkdown(markdown, enableGfm, 0)

    member this.ParseDocument() : DocumentNode =
        { Children = this.ParseBlocks() }

    member private this.ParseBlocks() =
        let nodes = ResizeArray<BlockNode>()

        let rec parseNext () =
            match this.TryParseFence() with
            | Some node -> nodes.Add node
            | None ->
                match if enableGfm then this.TryParseTable() else None with
                | Some node -> nodes.Add node
                | None ->
                    match this.TryParseHeading() with
                    | Some node -> nodes.Add node
                    | None when Common.isThematicBreak this.Current ->
                        nodes.Add ThematicBreakNode
                        index <- index + 1
                    | None ->
                        match this.TryParseBlockquote() with
                        | Some node -> nodes.Add node
                        | None ->
                            match this.TryParseList() with
                            | Some node -> nodes.Add node
                            | None ->
                                match this.TryParseRawHtmlBlock() with
                                | Some node -> nodes.Add node
                                | None -> nodes.Add(this.ParseParagraphOrSetextHeading())

        while index < lines.Length do
            if Common.isBlank this.Current then
                index <- index + 1
            else
                parseNext ()

        List.ofSeq nodes

    member private this.ParseParagraphOrSetextHeading() =
        let paragraphLines = ResizeArray<string>()
        let mutable headingResult : BlockNode option = None

        while index < lines.Length && headingResult.IsNone && not (Common.isBlank this.Current) && not (this.StartsOtherBlock this.Current) do
            match
                if index + 1 < lines.Length then Common.isSetextUnderline lines[index + 1] else None
            with
            | Some headingLevel ->
                let headingText = Seq.append paragraphLines [ this.Current ] |> String.concat "\n" |> fun s -> s.Trim()
                index <- index + 2
                headingResult <- Some(HeadingNode(headingLevel, InlineParsing.parseAtDepth depth headingText enableGfm))
            | None ->
                paragraphLines.Add this.Current
                index <- index + 1

        match headingResult with
        | Some node -> node
        | None when paragraphLines.Count > 0 ->
            ParagraphNode(paragraphLines |> Seq.toList |> String.concat "\n" |> fun s -> s.Trim() |> fun s -> InlineParsing.parseAtDepth depth s enableGfm)
        | None -> ParagraphNode []

    member private this.TryParseFence() =
        let match' = Common.fenceRegex.Match(this.Current)
        if not match'.Success then
            None
        else
            let fence = match'.Groups["fence"].Value
            let fenceChar = fence[0]
            let info = match'.Groups["info"].Value.Trim()
            let language =
                if info.Length = 0 then None
                else info.Split(' ', StringSplitOptions.RemoveEmptyEntries) |> Array.tryHead
            let content = ResizeArray<string>()
            index <- index + 1
            let mutable closed = false
            while index < lines.Length && not closed do
                let line = this.Current
                let trimmed = line.TrimStart()
                if trimmed.Length >= fence.Length && trimmed |> Seq.forall ((=) fenceChar) then
                    index <- index + 1
                    closed <- true
                else
                    content.Add line
                    index <- index + 1
            Some(CodeBlockNode(language, String.concat "\n" content + "\n"))

    member private this.TryParseHeading() =
        let match' = Common.atxHeadingRegex.Match(this.Current)
        if match'.Success then
            let level = match'.Groups["marks"].Value.Length
            let text = match'.Groups["text"].Value.Trim()
            index <- index + 1
            Some(HeadingNode(level, InlineParsing.parseAtDepth depth text enableGfm))
        else
            None

    member private this.TryParseBlockquote() =
        if not (Common.isBlockquoteLine this.Current) then
            None
        else
            let innerLines = ResizeArray<string>()
            while index < lines.Length && (Common.isBlank this.Current || Common.isBlockquoteLine this.Current) do
                innerLines.Add(if Common.isBlank this.Current then "" else Common.stripBlockquoteMarker this.Current)
                index <- index + 1
            let innerDocument : DocumentNode = BlockParser.ParseMarkdown(String.concat "\n" innerLines, enableGfm, depth + 1)
            Some(BlockquoteNode innerDocument.Children)

    member private this.TryParseList() =
        match Common.tryParseListItemStart this.Current with
        | None -> None
        | Some(ordered, start, _) ->
            let children = ResizeArray<ListChildNode>()
            let mutable tight = true
            let mutable continueItems = true

            while index < lines.Length && continueItems do
                match Common.tryParseListItemStart this.Current with
                | Some(currentOrdered, _, itemText) when currentOrdered = ordered ->
                    index <- index + 1
                    let lines' = ResizeArray<string>()
                    lines'.Add itemText
                    let mutable collecting = true

                    while index < lines.Length && collecting do
                        if Common.isBlank this.Current then
                            tight <- false
                            lines'.Add ""
                            index <- index + 1
                            if index < lines.Length then
                                match Common.tryParseListItemStart this.Current with
                                | Some(nextOrdered, _, _) when nextOrdered = ordered -> collecting <- false
                                | _ -> ()
                            else
                                collecting <- false
                        else
                            match Common.tryParseListItemStart this.Current with
                            | Some(nextOrdered, _, _) when nextOrdered = ordered -> collecting <- false
                            | _ ->
                                if this.StartsOtherBlock this.Current then
                                    collecting <- false
                                else
                                    lines'.Add(Common.stripContinuationIndent this.Current)
                                    index <- index + 1

                    let mutable taskState = None
                    match Common.tryStripTaskMarker lines'[0] with
                    | Some(checkedState, stripped) when enableGfm ->
                        taskState <- Some checkedState
                        lines'[0] <- stripped
                    | _ -> ()

                    let childDocument : DocumentNode = BlockParser.ParseMarkdown(String.concat "\n" lines', enableGfm, depth + 1)
                    children.Add(
                        match taskState with
                        | Some checkedState -> TaskItemNode(checkedState, childDocument.Children)
                        | None -> ListItemNode childDocument.Children)
                | _ -> continueItems <- false

            let startValue : int option = if ordered then Some start else None
            let childNodes : ListChildNode list = List.ofSeq children
            Some(ListNode(ordered, startValue, tight, childNodes))

    member private this.TryParseRawHtmlBlock() =
        let trimmed = this.Current.Trim()
        if Common.looksLikeRawHtmlBlock trimmed then
            index <- index + 1
            Some(RawBlockNode("html", trimmed))
        else
            None

    member private this.TryParseTable() =
        if index + 1 >= lines.Length then
            None
        else
            match Common.trySplitTableRow this.Current, Common.tryParseDelimiterRow lines[index + 1] with
            | Some headerCells, Some alignment when List.length headerCells = List.length alignment ->
                let rows = ResizeArray<TableRowNode>()
                rows.Add { IsHeader = true; Children = headerCells |> List.map (fun cell -> { Children = InlineParsing.parseAtDepth depth cell true }) }
                index <- index + 2
                let mutable collecting = true
                while index < lines.Length && collecting do
                    match Common.trySplitTableRow this.Current with
                    | Some bodyCells when List.length bodyCells = List.length alignment ->
                        rows.Add { IsHeader = false; Children = bodyCells |> List.map (fun cell -> { Children = InlineParsing.parseAtDepth depth cell true }) }
                        index <- index + 1
                    | _ -> collecting <- false
                Some(TableNode(alignment, List.ofSeq rows))
            | _ -> None

    member private this.StartsOtherBlock(line: string) =
        Common.fenceRegex.IsMatch line
        || Common.atxHeadingRegex.IsMatch line
        || Common.isThematicBreak line
        || Common.isBlockquoteLine line
        || (Common.tryParseListItemStart line |> Option.isSome)
        || (enableGfm
            && index + 1 < lines.Length
            && (Common.trySplitTableRow line |> Option.isSome)
            && (Common.tryParseDelimiterRow lines[index + 1] |> Option.isSome))
        || Common.looksLikeRawHtmlBlock line

[<AbstractClass; Sealed>]
type MarkdownParser =
    static member Parse(markdown: string, ?enableGfm: bool) =
        BlockParser.ParseMarkdown((if isNull markdown then "" else markdown), defaultArg enableGfm false)

[<AbstractClass; Sealed>]
type CommonmarkParser =
    static member Parse(markdown: string) = MarkdownParser.Parse(markdown, enableGfm = false)
    static member Version = "0.1.0"
    static member CommonmarkVersion = "0.31.2"
