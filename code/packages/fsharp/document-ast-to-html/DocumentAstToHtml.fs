namespace CodingAdventures.DocumentAstToHtml.FSharp

open System
open System.Globalization
open System.Text
open CodingAdventures.DocumentAst.FSharp

/// Options for the mechanical AST-to-HTML rendering pass.
type RenderOptions =
    { /// Omit raw nodes whose format is HTML instead of passing them through.
      SuppressRawHtml: bool }

/// Converts the format-independent TE00 document tree into a bounded HTML fragment.
[<RequireQualifiedAccess>]
module DocumentAstToHtml =
    [<Literal>]
    let private MaxRenderDepth = 256

    [<Literal>]
    let private MaxRenderNodes = 100_000

    [<Literal>]
    let private MaxOutputCharacters = 1_000_000

    /// The default policy preserves raw HTML nodes.
    let defaultOptions = { SuppressRawHtml = false }

    let private checkValue argumentName (value: string) =
        if isNull value then
            nullArg argumentName

    let private isIgnoredUrlCharacter codePoint =
        (codePoint >= 0x0000 && codePoint <= 0x001F)
        || (codePoint >= 0x007F && codePoint <= 0x009F)
        || codePoint = 0x200B
        || codePoint = 0x200C
        || codePoint = 0x200D
        || codePoint = 0x2060
        || codePoint = 0xFEFF

    let private isAllowedUrlRune (rune: Rune) =
        let value = rune.Value

        (value >= int 'a' && value <= int 'z')
        || (value >= int 'A' && value <= int 'Z')
        || (value >= int '0' && value <= int '9')
        || match value with
           | 0x2D // -
           | 0x5F // _
           | 0x2E // .
           | 0x7E // ~
           | 0x3A // :
           | 0x2F // /
           | 0x3F // ?
           | 0x23 // #
           | 0x40 // @
           | 0x21 // !
           | 0x24 // $
           | 0x26 // &
           | 0x27 // '
           | 0x28 // (
           | 0x29 // )
           | 0x2A // *
           | 0x2B // +
           | 0x2C // ,
           | 0x3B // ;
           | 0x3D // =
           | 0x25 -> true // %
           | _ -> false

    let private sanitizeUrl (value: string) =
        checkValue "value" value

        if value.Length > MaxOutputCharacters then
            raise (InvalidOperationException($"URL exceeds the supported limit of {MaxOutputCharacters} characters."))

        let stripped = StringBuilder(value.Length)

        for rune in value.EnumerateRunes() do
            if not (isIgnoredUrlCharacter rune.Value) then
                stripped.Append(rune.ToString()) |> ignore

        let result = stripped.ToString()
        let candidate = result.TrimStart(' ')

        if candidate.StartsWith("javascript:", StringComparison.OrdinalIgnoreCase)
           || candidate.StartsWith("vbscript:", StringComparison.OrdinalIgnoreCase)
           || candidate.StartsWith("data:", StringComparison.OrdinalIgnoreCase)
           || candidate.StartsWith("blob:", StringComparison.OrdinalIgnoreCase) then
            ""
        else
            result

    let private normalizeAutolink (value: string) =
        let output = StringBuilder(value.Length)
        let bytes = Array.zeroCreate<byte> 4

        for rune in value.EnumerateRunes() do
            if isAllowedUrlRune rune then
                output.Append(rune.ToString()) |> ignore
            else
                let length = rune.EncodeToUtf8(bytes.AsSpan())

                for index in 0 .. length - 1 do
                    if output.Length > MaxOutputCharacters - 3 then
                        raise (
                            InvalidOperationException(
                                $"Normalized URL exceeds the supported limit of {MaxOutputCharacters} characters."
                            )
                        )

                    output.Append('%') |> ignore
                    output.Append(bytes[index].ToString("X2", CultureInfo.InvariantCulture)) |> ignore

        output.ToString()

    type private RenderContext(options: RenderOptions) =
        let output = StringBuilder()
        let mutable nodeCount = 0

        member private _.Append(value: string) =
            checkValue "value" value

            if value.Length > MaxOutputCharacters - output.Length then
                raise (
                    InvalidOperationException(
                        $"HTML output exceeds the supported limit of {MaxOutputCharacters} characters."
                    )
                )

            output.Append(value) |> ignore

        member private this.AppendEscaped(value: string) =
            checkValue "value" value

            for character in value do
                match character with
                | '&' -> this.Append("&amp;")
                | '<' -> this.Append("&lt;")
                | '>' -> this.Append("&gt;")
                | '"' -> this.Append("&quot;")
                | _ -> this.Append(character.ToString())

        member private _.CheckDepth(depth: int) =
            if depth > MaxRenderDepth then
                raise (
                    InvalidOperationException(
                        $"Document tree exceeds the supported depth limit of {MaxRenderDepth}."
                    )
                )

        member private this.CountNode(depth: int) =
            this.CheckDepth(depth)
            nodeCount <- nodeCount + 1

            if nodeCount > MaxRenderNodes then
                raise (
                    InvalidOperationException(
                        $"Document tree exceeds the supported limit of {MaxRenderNodes} nodes."
                    )
                )

        member private this.AppendTitle(title: string option) =
            match title with
            | None -> ()
            | Some value ->
                this.Append(" title=\"")
                this.AppendEscaped(value)
                this.Append("\"")

        member private this.RenderInlines(nodes: InlineNode list, depth: int) =
            this.CheckDepth(depth)

            for node in nodes do
                this.RenderInline(node, depth + 1)

        member private this.RenderInline(node: InlineNode, depth: int) =
            this.CountNode(depth)

            match node with
            | TextNode value -> this.AppendEscaped(value)
            | EmphasisNode children ->
                this.Append("<em>")
                this.RenderInlines(children, depth)
                this.Append("</em>")
            | StrongNode children ->
                this.Append("<strong>")
                this.RenderInlines(children, depth)
                this.Append("</strong>")
            | StrikethroughNode children ->
                this.Append("<del>")
                this.RenderInlines(children, depth)
                this.Append("</del>")
            | CodeSpanNode value ->
                this.Append("<code>")
                this.AppendEscaped(value)
                this.Append("</code>")
            | LinkNode(destination, title, children) ->
                this.Append("<a href=\"")
                this.AppendEscaped(sanitizeUrl destination)
                this.Append("\"")
                this.AppendTitle(title)
                this.Append(">")
                this.RenderInlines(children, depth)
                this.Append("</a>")
            | ImageNode(destination, title, alt) ->
                this.Append("<img src=\"")
                this.AppendEscaped(sanitizeUrl destination)
                this.Append("\" alt=\"")
                this.AppendEscaped(alt)
                this.Append("\"")
                this.AppendTitle(title)
                this.Append(" />")
            | AutolinkNode(destination, isEmail) ->
                let safeDestination = sanitizeUrl destination
                let href = if isEmail then "mailto:" + safeDestination else normalizeAutolink safeDestination
                this.Append("<a href=\"")
                this.AppendEscaped(href)
                this.Append("\">")
                this.AppendEscaped(destination)
                this.Append("</a>")
            | RawInlineNode(format, value) ->
                if not options.SuppressRawHtml && format = "html" then
                    this.Append(value)
            | HardBreakNode -> this.Append("<br />\n")
            | SoftBreakNode -> this.Append("\n")

        member private this.RenderTableCell(
            cell: TableCellNode,
            header: bool,
            alignment: TableAlignment option,
            depth: int
        ) =
            this.CountNode(depth)
            let tag = if header then "th" else "td"
            this.Append("<")
            this.Append(tag)

            match alignment with
            | None -> ()
            | Some value ->
                this.Append(" align=\"")

                match value with
                | Left -> this.Append("left")
                | Right -> this.Append("right")
                | Center -> this.Append("center")

                this.Append("\"")

            this.Append(">")
            this.RenderInlines(cell.Children, depth)
            this.Append("</")
            this.Append(tag)
            this.Append(">\n")

        member private this.RenderTableRow(
            row: TableRowNode,
            alignments: TableAlignment option array,
            depth: int
        ) =
            this.CountNode(depth)
            this.Append("<tr>\n")

            row.Children
            |> List.iteri (fun index cell ->
                let alignment = if index < alignments.Length then alignments[index] else None
                this.RenderTableCell(cell, row.IsHeader, alignment, depth + 1))

            this.Append("</tr>\n")

        member private this.RenderTable(alignments: TableAlignment option list, rows: TableRowNode list, depth: int) =
            let mutable rowCount = 0

            for _ in rows do
                rowCount <- rowCount + 1

                if rowCount > MaxRenderNodes - nodeCount then
                    raise (
                        InvalidOperationException(
                            $"Document tree exceeds the supported limit of {MaxRenderNodes} nodes."
                        )
                    )

            if alignments.Length > MaxRenderNodes then
                raise (
                    InvalidOperationException(
                        $"Table alignment metadata exceeds the supported limit of {MaxRenderNodes} entries."
                    )
                )

            let alignmentArray = List.toArray alignments
            this.Append("<table>\n")

            if List.exists (fun row -> row.IsHeader) rows then
                this.Append("<thead>\n")

                for row in rows do
                    if row.IsHeader then
                        this.RenderTableRow(row, alignmentArray, depth + 1)

                this.Append("</thead>\n")

            if List.exists (fun row -> not row.IsHeader) rows then
                this.Append("<tbody>\n")

                for row in rows do
                    if not row.IsHeader then
                        this.RenderTableRow(row, alignmentArray, depth + 1)

                this.Append("</tbody>\n")

            this.Append("</table>\n")

        member private this.RenderListItem(
            children: BlockNode list,
            tight: bool,
            depth: int,
            checkbox: bool option
        ) =
            this.Append("<li>")

            match checkbox with
            | Some true -> this.Append("<input type=\"checkbox\" disabled=\"\" checked=\"\" /> ")
            | Some false -> this.Append("<input type=\"checkbox\" disabled=\"\" /> ")
            | None -> ()

            match children with
            | [] -> this.Append("</li>\n")
            | ParagraphNode firstChildren :: rest when tight ->
                this.CountNode(depth + 1)
                this.RenderInlines(firstChildren, depth + 1)

                match rest with
                | [] -> this.Append("</li>\n")
                | _ ->
                    this.Append("\n")

                    for block in rest do
                        this.RenderBlock(block, tight, depth + 1)

                    this.Append("</li>\n")
            | _ ->
                this.Append("\n")
                this.RenderBlocks(children, tight, depth)

                match checkbox, tight, List.tryLast children with
                | None, true, Some(ParagraphNode _) when output.Length > 0 && output[output.Length - 1] = '\n' ->
                    output.Length <- output.Length - 1
                | _ -> ()

                this.Append("</li>\n")

        member private this.RenderList(
            ordered: bool,
            start: int option,
            tight: bool,
            children: ListChildNode list,
            depth: int
        ) =
            let tag = if ordered then "ol" else "ul"
            this.Append("<")
            this.Append(tag)

            match ordered, start with
            | true, Some value when value <> 1 ->
                this.Append(" start=\"")
                this.Append(value.ToString(CultureInfo.InvariantCulture))
                this.Append("\"")
            | _ -> ()

            this.Append(">\n")

            for child in children do
                this.CountNode(depth + 1)

                match child with
                | ListItemNode itemChildren -> this.RenderListItem(itemChildren, tight, depth + 1, None)
                | TaskItemNode(checked', itemChildren) ->
                    this.RenderListItem(itemChildren, tight, depth + 1, Some checked')

            this.Append("</")
            this.Append(tag)
            this.Append(">\n")

        member private this.RenderCodeBlock(language: string option, value: string) =
            this.Append("<pre><code")

            match language with
            | Some nonEmpty when nonEmpty.Length > 0 ->
                this.Append(" class=\"language-")
                this.AppendEscaped(nonEmpty)
                this.Append("\"")
            | _ -> ()

            this.Append(">")
            this.AppendEscaped(value)
            this.Append("</code></pre>\n")

        member private this.RenderBlock(block: BlockNode, tight: bool, depth: int) =
            this.CountNode(depth)

            match block with
            | HeadingNode(level, children) ->
                if level < 1 || level > 6 then
                    raise (InvalidOperationException("Heading levels must be between 1 and 6."))

                let levelText = level.ToString(CultureInfo.InvariantCulture)
                this.Append("<h")
                this.Append(levelText)
                this.Append(">")
                this.RenderInlines(children, depth)
                this.Append("</h")
                this.Append(levelText)
                this.Append(">\n")
            | ParagraphNode children ->
                if not tight then
                    this.Append("<p>")

                this.RenderInlines(children, depth)
                this.Append(if tight then "\n" else "</p>\n")
            | CodeBlockNode(language, value) -> this.RenderCodeBlock(language, value)
            | BlockquoteNode children ->
                this.Append("<blockquote>\n")
                this.RenderBlocks(children, false, depth)
                this.Append("</blockquote>\n")
            | ListNode(ordered, start, isTight, children) ->
                this.RenderList(ordered, start, isTight, children, depth)
            | ThematicBreakNode -> this.Append("<hr />\n")
            | RawBlockNode(format, value) ->
                if not options.SuppressRawHtml && format = "html" then
                    this.Append(value)
            | TableNode(alignments, rows) -> this.RenderTable(alignments, rows, depth)

        member this.RenderBlocks(blocks: BlockNode list, tight: bool, depth: int) =
            this.CheckDepth(depth)

            for block in blocks do
                this.RenderBlock(block, tight, depth + 1)

        member _.Finish() = output.ToString()

    /// Escape text for either HTML text or a double-quoted attribute.
    let escapeHtml (value: string) =
        checkValue "value" value

        if value.Length > MaxOutputCharacters then
            raise (
                InvalidOperationException(
                    $"Escaped HTML exceeds the supported limit of {MaxOutputCharacters} characters."
                )
            )

        let output = StringBuilder(value.Length)

        for character in value do
            let encoded =
                match character with
                | '&' -> "&amp;"
                | '<' -> "&lt;"
                | '>' -> "&gt;"
                | '"' -> "&quot;"
                | _ -> character.ToString()

            if encoded.Length > MaxOutputCharacters - output.Length then
                raise (
                    InvalidOperationException(
                        $"Escaped HTML exceeds the supported limit of {MaxOutputCharacters} characters."
                    )
                )

            output.Append(encoded) |> ignore

        output.ToString()

    /// Render a document using explicit raw-HTML policy.
    let renderWithOptions (options: RenderOptions) (document: DocumentNode) =
        if obj.ReferenceEquals(document, null) then
            nullArg "document"

        if obj.ReferenceEquals(options, null) then
            nullArg "options"

        let renderer = RenderContext(options)
        renderer.RenderBlocks(document.Children, false, 0)
        renderer.Finish()

    /// Render a document using the default policy.
    let render (document: DocumentNode) = renderWithOptions defaultOptions document
