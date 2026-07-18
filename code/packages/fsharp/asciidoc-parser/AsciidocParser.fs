namespace CodingAdventures.AsciidocParser

open System
open System.Collections.Generic
open System.Text
open CodingAdventures.DocumentAst.FSharp

type private BlockState =
    | Normal
    | Paragraph
    | Code
    | Literal
    | Passthrough
    | Quote
    | UnorderedList
    | OrderedList

type private ListEntry = { Level: int; Text: string }

/// Parses a portable AsciiDoc subset into the shared document AST.
[<RequireQualifiedAccess>]
module AsciidocParser =
    let private startsAt (text: string) index (value: string) =
        index >= 0
        && index + value.Length <= text.Length
        && String.CompareOrdinal(text, index, value, 0, value.Length) = 0

    /// Parse inline AsciiDoc markup into shared inline AST nodes.
    let rec parseInline (text: string) =
        nullArgCheck "text" text |> ignore
        let output = ResizeArray<InlineNode>()
        let plain = StringBuilder()
        let mutable index = 0

        let flush () =
            if plain.Length > 0 then
                output.Add(TextNode(plain.ToString()))
                plain.Clear() |> ignore

        let tryDelimited delimiter create =
            if not (startsAt text index delimiter) then
                false
            else
                let closing = text.IndexOf(delimiter, index + delimiter.Length, StringComparison.Ordinal)
                if closing < 0 then
                    false
                else
                    flush ()
                    output.Add(create text[(index + delimiter.Length) .. closing - 1])
                    index <- closing + delimiter.Length
                    true

        let tryMacro prefix image =
            if not (startsAt text index prefix) then
                false
            else
                let openBracket = text.IndexOf('[', index + prefix.Length)
                let closeBracket = if openBracket < 0 then -1 else text.IndexOf(']', openBracket + 1)
                if openBracket < 0 || closeBracket < 0 then
                    false
                else
                    let destination = text[(index + prefix.Length) .. openBracket - 1]
                    let label = text[(openBracket + 1) .. closeBracket - 1]
                    flush ()
                    if image then
                        output.Add(ImageNode(destination, None, label))
                    else
                        let display = if label.Length = 0 then destination else label
                        output.Add(LinkNode(destination, None, [ TextNode display ]))
                    index <- closeBracket + 1
                    true

        let tryCrossReference () =
            if not (startsAt text index "<<") then
                false
            else
                let closing = text.IndexOf(">>", index + 2, StringComparison.Ordinal)
                if closing < 0 then
                    false
                else
                    let content = text[(index + 2) .. closing - 1]
                    let parts = content.Split(',', 2)
                    let anchor = parts.[0].Trim()
                    let label = if parts.Length = 2 then parts.[1].Trim() else anchor
                    flush ()
                    output.Add(LinkNode("#" + anchor, None, [ TextNode label ]))
                    index <- closing + 2
                    true

        let tryUrl () =
            let schemeLength =
                if startsAt text index "https://" then 8
                elif startsAt text index "http://" then 7
                else 0

            if schemeLength = 0 then
                false
            else
                let mutable ending = index + schemeLength
                while ending < text.Length
                      && not (Char.IsWhiteSpace text.[ending])
                      && text.[ending] <> '['
                      && text.[ending] <> ']' do
                    ending <- ending + 1

                let url = text[index .. ending - 1]
                flush ()
                if ending < text.Length && text.[ending] = '[' then
                    let closeBracket = text.IndexOf(']', ending + 1)
                    if closeBracket >= 0 then
                        let label = text[(ending + 1) .. closeBracket - 1]
                        let display = if label.Length = 0 then url else label
                        output.Add(LinkNode(url, None, [ TextNode display ]))
                        index <- closeBracket + 1
                    else
                        output.Add(AutolinkNode(url, false))
                        index <- ending
                else
                    output.Add(AutolinkNode(url, false))
                    index <- ending
                true

        while index < text.Length do
            if startsAt text index "  \n" then
                flush ()
                output.Add(HardBreakNode)
                index <- index + 3
            elif startsAt text index "\\\n" then
                flush ()
                output.Add(HardBreakNode)
                index <- index + 2
            elif text.[index] = '\n' then
                flush ()
                output.Add(SoftBreakNode)
                index <- index + 1
            elif tryDelimited "`" CodeSpanNode then ()
            elif tryDelimited "**" (parseInline >> StrongNode) then ()
            elif tryDelimited "__" (parseInline >> EmphasisNode) then ()
            elif tryDelimited "*" (parseInline >> StrongNode) then ()
            elif tryDelimited "_" (parseInline >> EmphasisNode) then ()
            elif tryMacro "link:" false then ()
            elif tryMacro "image:" true then ()
            elif tryCrossReference () then ()
            elif tryUrl () then ()
            else
                plain.Append(text.[index]) |> ignore
                index <- index + 1

        flush ()
        List.ofSeq output

    let private isDelimiter (line: string) value minimum =
        line.Length >= minimum && line |> Seq.forall ((=) value)

    let private tryHeading (line: string) =
        let mutable count = 0
        while count < line.Length && line.[count] = '=' do
            count <- count + 1
        if count > 0 && count < line.Length && Char.IsWhiteSpace line.[count] then
            Some(min count 6, line[count ..].TrimStart())
        else
            None

    let private trySourceAttribute (line: string) =
        if line.StartsWith("[source", StringComparison.OrdinalIgnoreCase) && line.EndsWith(']') then
            let comma = line.IndexOf(',')
            if comma >= 0 then
                let language = line[(comma + 1) .. line.Length - 2].Trim()
                if language.Length > 0 then Some language else None
            else
                None
        else
            None

    let private tryListItem (line: string) marker =
        let mutable count = 0
        while count < line.Length && line.[count] = marker do
            count <- count + 1
        if count > 0 && count < line.Length && line.[count] = ' ' then
            Some { Level = count; Text = line[(count + 1) ..].Trim() }
        else
            None

    let private startsNewBlock line =
        tryHeading line |> Option.isSome
        || trySourceAttribute line |> Option.isSome
        || tryListItem line '*' |> Option.isSome
        || tryListItem line '.' |> Option.isSome
        || line.StartsWith("//", StringComparison.Ordinal)
        || isDelimiter line '\'' 3
        || isDelimiter line '-' 4
        || isDelimiter line '.' 4
        || isDelimiter line '+' 4
        || isDelimiter line '_' 4

    let private buildList (entries: IReadOnlyList<ListEntry>) ordered =
        let rec buildLevel level startIndex =
            let children = ResizeArray<ListChildNode>()
            let mutable index = startIndex
            let mutable keepGoing = true

            while index < entries.Count && entries.[index].Level >= level && keepGoing do
                if entries.[index].Level > level then
                    keepGoing <- false
                else
                    let entry = entries.[index]
                    index <- index + 1
                    let blocks = ResizeArray<BlockNode>()
                    blocks.Add(ParagraphNode(parseInline entry.Text))

                    while index < entries.Count && entries.[index].Level > level do
                        let sublist, nextIndex = buildLevel entries.[index].Level index
                        blocks.Add(sublist)
                        index <- nextIndex

                    children.Add(ListItemNode(List.ofSeq blocks))

            ListNode(ordered, (if ordered then Some 1 else None), true, List.ofSeq children), index

        buildLevel entries.[0].Level 0 |> fst

    /// Parse a complete AsciiDoc source string.
    let rec parse (source: string) : DocumentNode =
        nullArgCheck "source" source |> ignore
        let normalized = source.Replace("\r\n", "\n", StringComparison.Ordinal).Replace('\r', '\n')
        let lines = ResizeArray<string>(normalized.Split('\n'))
        if normalized.EndsWith('\n') then
            lines.RemoveAt(lines.Count - 1)

        let blocks = ResizeArray<BlockNode>()
        let paragraphLines = ResizeArray<string>()
        let delimitedLines = ResizeArray<string>()
        let listEntries = ResizeArray<ListEntry>()
        let mutable state = Normal
        let mutable pendingLanguage: string option = None
        let mutable listOrdered = false

        let flushParagraph () =
            if paragraphLines.Count > 0 then
                blocks.Add(ParagraphNode(parseInline (String.Join('\n', paragraphLines))))
                paragraphLines.Clear()

        let flushList () =
            if listEntries.Count > 0 then
                blocks.Add(buildList listEntries listOrdered)
                listEntries.Clear()

        let emitDelimited activeState =
            let value = if delimitedLines.Count = 0 then String.Empty else String.Join('\n', delimitedLines) + "\n"
            match activeState with
            | Code -> blocks.Add(CodeBlockNode(pendingLanguage, value))
            | Literal -> blocks.Add(CodeBlockNode(None, value))
            | Passthrough -> blocks.Add(RawBlockNode("html", String.Join('\n', delimitedLines)))
            | Quote -> blocks.Add(BlockquoteNode((parse (String.Join('\n', delimitedLines))).Children))
            | _ -> ()

        for rawLine in lines do
            let line = rawLine.TrimEnd()
            let mutable handled = false

            match state with
            | Code
            | Literal
            | Passthrough
            | Quote ->
                let delimiter =
                    match state with
                    | Code -> '-'
                    | Literal -> '.'
                    | Passthrough -> '+'
                    | _ -> '_'

                if isDelimiter line delimiter 4 then
                    let completedState = state
                    emitDelimited completedState
                    if completedState = Code then pendingLanguage <- None
                    delimitedLines.Clear()
                    state <- Normal
                else
                    delimitedLines.Add(rawLine)
                handled <- true
            | Paragraph ->
                if line.Length = 0 then
                    flushParagraph ()
                    state <- Normal
                    handled <- true
                elif startsNewBlock line then
                    flushParagraph ()
                    state <- Normal
                else
                    paragraphLines.Add(line)
                    handled <- true
            | UnorderedList
            | OrderedList ->
                if line.Length = 0 then
                    flushList ()
                    state <- Normal
                    handled <- true
                else
                    let marker = if state = OrderedList then '.' else '*'
                    match tryListItem line marker with
                    | Some entry ->
                        listEntries.Add(entry)
                        handled <- true
                    | None ->
                        flushList ()
                        state <- Normal
            | Normal -> ()

            if not handled then
                if line.Length = 0 || line.StartsWith("//", StringComparison.Ordinal) then
                    ()
                else
                    match trySourceAttribute line with
                    | Some language -> pendingLanguage <- Some language
                    | None ->
                        match tryHeading line with
                        | Some(level, headingText) -> blocks.Add(HeadingNode(level, parseInline headingText))
                        | None when isDelimiter line '\'' 3 -> blocks.Add(ThematicBreakNode)
                        | None when isDelimiter line '-' 4 -> state <- Code
                        | None when isDelimiter line '.' 4 -> state <- Literal
                        | None when isDelimiter line '+' 4 -> state <- Passthrough
                        | None when isDelimiter line '_' 4 -> state <- Quote
                        | None ->
                            match tryListItem line '*' with
                            | Some entry ->
                                listOrdered <- false
                                listEntries.Add(entry)
                                state <- UnorderedList
                            | None ->
                                match tryListItem line '.' with
                                | Some entry ->
                                    listOrdered <- true
                                    listEntries.Add(entry)
                                    state <- OrderedList
                                | None ->
                                    paragraphLines.Add(line)
                                    state <- Paragraph

        flushParagraph ()
        flushList ()
        if delimitedLines.Count > 0 then emitDelimited state
        { Children = List.ofSeq blocks }
