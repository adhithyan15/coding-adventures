using System.Globalization;
using System.Text;
using CodingAdventures.DocumentAst;

namespace CodingAdventures.DocumentAstToHtml;

/// <summary>
/// Rendering options intentionally stay smaller than a sanitization policy.
/// Suppressing raw HTML is useful at a trust boundary, while callers needing
/// per-node policy should sanitize the AST before it reaches this renderer.
/// </summary>
public sealed record RenderOptions(bool SuppressRawHtml = false);

/// <summary>
/// Converts the format-independent TE00 document tree into an HTML fragment.
///
/// <para>
/// Think of this class as the final, mechanical stage of a compiler: the
/// parser has already decided what every node means, so rendering is a bounded
/// walk that chooses one HTML spelling for each node.  The walk never mutates
/// the borrowed tree and owns only its output buffer.
/// </para>
/// </summary>
public static class DocumentAstToHtml
{
    private const int MaxRenderDepth = 256;
    private const int MaxRenderNodes = 100_000;
    private const int MaxOutputCharacters = 1_000_000;

    /// <summary>Render a document to a newly owned HTML string.</summary>
    public static string ToHtml(DocumentNode document, RenderOptions? options = null)
    {
        ArgumentNullException.ThrowIfNull(document);
        var context = new RenderContext(options ?? new RenderOptions());
        context.RenderBlocks(document.Children, tight: false, depth: 0);
        return context.Finish();
    }

    /// <summary>Escape text for either HTML text or a double-quoted attribute.</summary>
    public static string EscapeHtml(string value)
    {
        ArgumentNullException.ThrowIfNull(value);
        if (value.Length > MaxOutputCharacters)
        {
            throw new InvalidOperationException($"Escaped HTML exceeds the supported limit of {MaxOutputCharacters} characters.");
        }

        var builder = new StringBuilder(value.Length);
        foreach (var character in value)
        {
            var encoded = character switch
            {
                '&' => "&amp;",
                '<' => "&lt;",
                '>' => "&gt;",
                '"' => "&quot;",
                _ => character.ToString(),
            };
            if (encoded.Length > MaxOutputCharacters - builder.Length)
            {
                throw new InvalidOperationException($"Escaped HTML exceeds the supported limit of {MaxOutputCharacters} characters.");
            }

            builder.Append(encoded);
        }

        return builder.ToString();
    }

    /// <summary>
    /// One context owns the limits and output for a render.  Centralising every
    /// append here means a leaf cannot accidentally bypass the output budget.
    /// </summary>
    private sealed class RenderContext(RenderOptions options)
    {
        private readonly StringBuilder _output = new();
        private int _nodes;

        public string Finish() => _output.ToString();

        public void RenderBlocks(IReadOnlyList<IBlockNode> blocks, bool tight, int depth)
        {
            CheckDepth(depth);
            foreach (var block in blocks)
            {
                RenderBlock(block, tight, depth + 1);
            }
        }

        private void RenderBlock(IBlockNode block, bool tight, int depth)
        {
            CountNode(depth);
            switch (block)
            {
                case HeadingNode heading:
                    if (heading.Level is < 1 or > 6)
                    {
                        throw new InvalidOperationException("Heading levels must be between 1 and 6.");
                    }

                    Append("<h");
                    Append(heading.Level.ToString(CultureInfo.InvariantCulture));
                    Append(">");
                    RenderInlines(heading.Children, depth);
                    Append("</h");
                    Append(heading.Level.ToString(CultureInfo.InvariantCulture));
                    Append(">\n");
                    break;

                case ParagraphNode paragraph:
                    if (!tight)
                    {
                        Append("<p>");
                    }

                    RenderInlines(paragraph.Children, depth);
                    Append(tight ? "\n" : "</p>\n");
                    break;

                case CodeBlockNode code:
                    RenderCodeBlock(code);
                    break;

                case BlockquoteNode quote:
                    Append("<blockquote>\n");
                    RenderBlocks(quote.Children, tight: false, depth);
                    Append("</blockquote>\n");
                    break;

                case ListNode list:
                    RenderList(list, depth);
                    break;

                case ListItemNode item:
                    RenderListItem(item.Children, tight, depth, checkbox: null);
                    break;

                case TaskItemNode task:
                    RenderListItem(task.Children, tight, depth, task.Checked);
                    break;

                case ThematicBreakNode:
                    Append("<hr />\n");
                    break;

                case RawBlockNode raw:
                    if (!options.SuppressRawHtml && raw.Format == "html")
                    {
                        Append(raw.Value);
                    }

                    break;

                case TableNode table:
                    RenderTable(table, depth);
                    break;

                default:
                    throw new InvalidOperationException($"Unsupported block node type: {block.GetType().FullName}.");
            }
        }

        private void RenderCodeBlock(CodeBlockNode code)
        {
            Append("<pre><code");
            if (!string.IsNullOrEmpty(code.Language))
            {
                Append(" class=\"language-");
                AppendEscaped(code.Language);
                Append("\"");
            }

            Append(">");
            AppendEscaped(code.Value);
            Append("</code></pre>\n");
        }

        private void RenderList(ListNode list, int depth)
        {
            var tag = list.Ordered ? "ol" : "ul";
            Append("<");
            Append(tag);
            if (list.Ordered && list.Start is not null and not 1)
            {
                Append(" start=\"");
                Append(list.Start.Value.ToString(CultureInfo.InvariantCulture));
                Append("\"");
            }

            Append(">\n");
            foreach (var item in list.Children)
            {
                CountNode(depth);
                switch (item)
                {
                    case TaskItemNode task:
                        RenderListItem(task.Children, list.Tight, depth, task.Checked);
                        break;
                    case ListItemNode ordinary:
                        RenderListItem(ordinary.Children, list.Tight, depth, checkbox: null);
                        break;
                    default:
                        throw new InvalidOperationException($"Unsupported list child node type: {item.GetType().FullName}.");
                }
            }

            Append("</");
            Append(tag);
            Append(">\n");
        }

        private void RenderListItem(
            IReadOnlyList<IBlockNode> children,
            bool tight,
            int depth,
            bool? checkbox)
        {
            Append("<li>");
            if (checkbox is not null)
            {
                Append(checkbox.Value
                    ? "<input type=\"checkbox\" disabled=\"\" checked=\"\" /> "
                    : "<input type=\"checkbox\" disabled=\"\" /> ");
            }

            if (children.Count == 0)
            {
                Append("</li>\n");
                return;
            }

            if (tight && children[0] is ParagraphNode first)
            {
                CountNode(depth + 1);
                RenderInlines(first.Children, depth + 1);
                if (children.Count == 1)
                {
                    Append("</li>\n");
                    return;
                }

                Append("\n");
                for (var index = 1; index < children.Count; index++)
                {
                    RenderBlock(children[index], tight, depth + 1);
                }

                Append("</li>\n");
                return;
            }

            Append("\n");
            RenderBlocks(children, tight, depth);
            if (checkbox is null && tight && children[^1] is ParagraphNode && _output.Length > 0 && _output[^1] == '\n')
            {
                _output.Length--;
            }

            Append("</li>\n");
        }

        private void RenderTable(TableNode table, int depth)
        {
            if (table.Children.Count > MaxRenderNodes - _nodes)
            {
                throw new InvalidOperationException($"Document tree exceeds the supported limit of {MaxRenderNodes} nodes.");
            }

            Append("<table>\n");
            if (table.Children.Any(row => row.IsHeader))
            {
                Append("<thead>\n");
                foreach (var row in table.Children)
                {
                    if (row.IsHeader)
                    {
                        RenderTableRow(row, table.Align, depth + 1);
                    }
                }

                Append("</thead>\n");
            }

            if (table.Children.Any(row => !row.IsHeader))
            {
                Append("<tbody>\n");
                foreach (var row in table.Children)
                {
                    if (!row.IsHeader)
                    {
                        RenderTableRow(row, table.Align, depth + 1);
                    }
                }

                Append("</tbody>\n");
            }

            Append("</table>\n");
        }

        private void RenderTableRow(TableRowNode row, IReadOnlyList<TableAlignment?> alignments, int depth)
        {
            CountNode(depth);
            Append("<tr>\n");
            for (var index = 0; index < row.Children.Count; index++)
            {
                var alignment = index < alignments.Count ? alignments[index] : null;
                RenderTableCell(row.Children[index], row.IsHeader, alignment, depth + 1);
            }

            Append("</tr>\n");
        }

        private void RenderTableCell(TableCellNode cell, bool header, TableAlignment? alignment, int depth)
        {
            CountNode(depth);
            var tag = header ? "th" : "td";
            Append("<");
            Append(tag);
            if (alignment is not null)
            {
                Append(" align=\"");
                Append(alignment.Value switch
                {
                    TableAlignment.Left => "left",
                    TableAlignment.Right => "right",
                    TableAlignment.Center => "center",
                    _ => throw new InvalidOperationException($"Unsupported table alignment value: {alignment.Value}."),
                });
                Append("\"");
            }

            Append(">");
            RenderInlines(cell.Children, depth);
            Append("</");
            Append(tag);
            Append(">\n");
        }

        private void RenderInlines(IReadOnlyList<IInlineNode> nodes, int depth)
        {
            CheckDepth(depth);
            foreach (var node in nodes)
            {
                RenderInline(node, depth + 1);
            }
        }

        private void RenderInline(IInlineNode node, int depth)
        {
            CountNode(depth);
            switch (node)
            {
                case TextNode text:
                    AppendEscaped(text.Value);
                    break;
                case EmphasisNode emphasis:
                    Append("<em>");
                    RenderInlines(emphasis.Children, depth);
                    Append("</em>");
                    break;
                case StrongNode strong:
                    Append("<strong>");
                    RenderInlines(strong.Children, depth);
                    Append("</strong>");
                    break;
                case StrikethroughNode strike:
                    Append("<del>");
                    RenderInlines(strike.Children, depth);
                    Append("</del>");
                    break;
                case CodeSpanNode code:
                    Append("<code>");
                    AppendEscaped(code.Value);
                    Append("</code>");
                    break;
                case LinkNode link:
                    Append("<a href=\"");
                    AppendEscaped(SanitizeUrl(link.Destination));
                    Append("\"");
                    AppendTitle(link.Title);
                    Append(">");
                    RenderInlines(link.Children, depth);
                    Append("</a>");
                    break;
                case ImageNode image:
                    Append("<img src=\"");
                    AppendEscaped(SanitizeUrl(image.Destination));
                    Append("\" alt=\"");
                    AppendEscaped(image.Alt);
                    Append("\"");
                    AppendTitle(image.Title);
                    Append(" />");
                    break;
                case AutolinkNode autolink:
                    var destination = SanitizeUrl(autolink.Destination);
                    Append("<a href=\"");
                    AppendEscaped(autolink.IsEmail ? "mailto:" + destination : NormalizeAutolink(destination));
                    Append("\">");
                    AppendEscaped(autolink.Destination);
                    Append("</a>");
                    break;
                case RawInlineNode raw:
                    if (!options.SuppressRawHtml && raw.Format == "html")
                    {
                        Append(raw.Value);
                    }

                    break;
                case HardBreakNode:
                    Append("<br />\n");
                    break;
                case SoftBreakNode:
                    Append("\n");
                    break;

                default:
                    throw new InvalidOperationException($"Unsupported inline node type: {node.GetType().FullName}.");
            }
        }

        private void AppendTitle(string? title)
        {
            if (title is null)
            {
                return;
            }

            Append(" title=\"");
            AppendEscaped(title);
            Append("\"");
        }

        private void AppendEscaped(string value)
        {
            foreach (var character in value)
            {
                Append(character switch
                {
                    '&' => "&amp;",
                    '<' => "&lt;",
                    '>' => "&gt;",
                    '"' => "&quot;",
                    _ => character.ToString(),
                });
            }
        }

        private void Append(string value)
        {
            if (value.Length > MaxOutputCharacters - _output.Length)
            {
                throw new InvalidOperationException($"HTML output exceeds the supported limit of {MaxOutputCharacters} characters.");
            }

            _output.Append(value);
        }

        private void CountNode(int depth)
        {
            CheckDepth(depth);
            if (++_nodes > MaxRenderNodes)
            {
                throw new InvalidOperationException($"Document tree exceeds the supported limit of {MaxRenderNodes} nodes.");
            }
        }

        private static void CheckDepth(int depth)
        {
            if (depth > MaxRenderDepth)
            {
                throw new InvalidOperationException($"Document tree exceeds the supported depth limit of {MaxRenderDepth}.");
            }
        }
    }

    private static string SanitizeUrl(string value)
    {
        if (value.Length > MaxOutputCharacters)
        {
            throw new InvalidOperationException($"URL exceeds the supported limit of {MaxOutputCharacters} characters.");
        }

        var stripped = new StringBuilder(value.Length);
        foreach (var rune in value.EnumerateRunes())
        {
            if (IsIgnoredUrlCharacter(rune.Value))
            {
                continue;
            }

            stripped.Append(rune.ToString());
        }

        var result = stripped.ToString();
        // Browsers ignore leading ASCII spaces before scheme parsing.  Keep
        // safe relative URLs byte-for-byte, but inspect the browser-visible
        // candidate so " javascript:" cannot bypass the blocklist.
        var schemeCandidate = result.TrimStart(' ');
        return schemeCandidate.StartsWith("javascript:", StringComparison.OrdinalIgnoreCase) ||
               schemeCandidate.StartsWith("vbscript:", StringComparison.OrdinalIgnoreCase) ||
               schemeCandidate.StartsWith("data:", StringComparison.OrdinalIgnoreCase) ||
               schemeCandidate.StartsWith("blob:", StringComparison.OrdinalIgnoreCase)
            ? string.Empty
            : result;
    }

    private static bool IsIgnoredUrlCharacter(int codePoint)
        => codePoint is >= 0x0000 and <= 0x001F or >= 0x007F and <= 0x009F or
           0x200B or 0x200C or 0x200D or 0x2060 or 0xFEFF;

    private static string NormalizeAutolink(string value)
    {
        var output = new StringBuilder(value.Length);
        Span<byte> bytes = stackalloc byte[4];
        foreach (var rune in value.EnumerateRunes())
        {
            if (IsAllowedUrlRune(rune))
            {
                output.Append(rune.ToString());
                continue;
            }

            var length = rune.EncodeToUtf8(bytes);
            for (var index = 0; index < length; index++)
            {
                if (output.Length > MaxOutputCharacters - 3)
                {
                    throw new InvalidOperationException($"Normalized URL exceeds the supported limit of {MaxOutputCharacters} characters.");
                }

                output.Append('%');
                output.Append(bytes[index].ToString("X2", CultureInfo.InvariantCulture));
            }
        }

        return output.ToString();
    }

    private static bool IsAllowedUrlRune(Rune rune)
    {
        var value = rune.Value;
        if (value is >= 'a' and <= 'z' or >= 'A' and <= 'Z' or >= '0' and <= '9')
        {
            return true;
        }

        return rune.Value switch
        {
            '-' or '_' or '.' or '~' or ':' or '/' or '?' or '#' or '@' or '!' or '$' or '&' or
            '\'' or '(' or ')' or '*' or '+' or ',' or ';' or '=' or '%' => true,
            _ => false,
        };
    }
}
