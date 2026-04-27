using System.Net;
using System.Text;
using System.Text.RegularExpressions;
using CodingAdventures.DocumentAst;

namespace CodingAdventures.CommonmarkParser;

public static class CommonmarkParser
{
    public const string VERSION = "0.1.0";
    public const string COMMONMARK_VERSION = "0.31.2";

    public static DocumentNode Parse(string markdown) => MarkdownParser.Parse(markdown, enableGfm: false);
}

public static class MarkdownParser
{
    internal const int MaxInputLength = 100_000;
    internal const int MaxParseDepth = 64;

    public static DocumentNode Parse(string markdown, bool enableGfm = false)
        => Parse(markdown, enableGfm, depth: 0);

    internal static DocumentNode Parse(string markdown, bool enableGfm, int depth)
    {
        var normalized = markdown ?? string.Empty;
        EnsureWithinLimits(normalized.Length, depth);
        var parser = new BlockParser(normalized, enableGfm, depth);
        return parser.ParseDocument();
    }

    internal static IReadOnlyList<IInlineNode> ParseInlines(string text, bool enableGfm, int depth)
    {
        var normalized = text ?? string.Empty;
        EnsureWithinLimits(normalized.Length, depth);
        return InlineParser.Parse(normalized, enableGfm, depth);
    }

    internal static void EnsureWithinLimits(int inputLength, int depth)
    {
        if (inputLength > MaxInputLength)
        {
            throw new InvalidOperationException($"Markdown input exceeds the supported size limit of {MaxInputLength} characters.");
        }

        if (depth > MaxParseDepth)
        {
            throw new InvalidOperationException($"Markdown nesting exceeds the supported depth limit of {MaxParseDepth}.");
        }
    }
}

internal sealed partial class BlockParser
{
    private static readonly Regex FenceRegex = new(@"^\s{0,3}(?<fence>`{3,}|~{3,})(?<info>.*)$", RegexOptions.Compiled);
    private static readonly Regex AtxHeadingRegex = new(@"^\s{0,3}(?<marks>#{1,6})[ \t]+(?<text>.+?)\s*#*\s*$", RegexOptions.Compiled);
    private static readonly Regex OrderedListRegex = new(@"^\s{0,3}(?<num>\d+)[.)][ \t]+(?<text>.*)$", RegexOptions.Compiled);
    private static readonly Regex BulletListRegex = new(@"^\s{0,3}[-+*][ \t]+(?<text>.*)$", RegexOptions.Compiled);
    private static readonly Regex TaskMarkerRegex = new(@"^\[(?<mark>[ xX])\][ \t]+(?<text>.*)$", RegexOptions.Compiled);
    private static readonly Regex TableDelimiterCellRegex = new(@"^:?-{3,}:?$", RegexOptions.Compiled);

    private readonly string[] _lines;
    private readonly bool _enableGfm;
    private readonly int _depth;
    private int _index;

    public BlockParser(string markdown, bool enableGfm, int depth)
    {
        _lines = Normalize(markdown).Split('\n');
        _enableGfm = enableGfm;
        _depth = depth;
    }

    public DocumentNode ParseDocument() => new(ParseBlocks());

    private List<IBlockNode> ParseBlocks()
    {
        var nodes = new List<IBlockNode>();

        while (_index < _lines.Length)
        {
            if (IsBlank(Current))
            {
                _index++;
                continue;
            }

            if (TryParseFence(out var codeBlock))
            {
                nodes.Add(codeBlock);
                continue;
            }

            if (_enableGfm && TryParseTable(out var table))
            {
                nodes.Add(table);
                continue;
            }

            if (TryParseHeading(out var heading))
            {
                nodes.Add(heading);
                continue;
            }

            if (IsThematicBreak(Current))
            {
                nodes.Add(new ThematicBreakNode());
                _index++;
                continue;
            }

            if (TryParseBlockquote(out var quote))
            {
                nodes.Add(quote);
                continue;
            }

            if (TryParseList(out var list))
            {
                nodes.Add(list);
                continue;
            }

            if (TryParseRawHtmlBlock(out var rawBlock))
            {
                nodes.Add(rawBlock);
                continue;
            }

            nodes.Add(ParseParagraphOrSetextHeading());
        }

        return nodes;
    }

    private IBlockNode ParseParagraphOrSetextHeading()
    {
        var paragraphLines = new List<string>();

        while (_index < _lines.Length && !IsBlank(Current) && !StartsOtherBlock(Current))
        {
            if (_index + 1 < _lines.Length && IsSetextUnderline(_lines[_index + 1], out var headingLevel))
            {
                var headingText = string.Join("\n", paragraphLines.Append(Current)).Trim();
                _index += 2;
                return new HeadingNode(headingLevel, MarkdownParser.ParseInlines(headingText, _enableGfm, _depth));
            }

            paragraphLines.Add(Current);
            _index++;
        }

        var paragraphText = string.Join("\n", paragraphLines).Trim();
        return new ParagraphNode(MarkdownParser.ParseInlines(paragraphText, _enableGfm, _depth));
    }

    private bool TryParseFence(out CodeBlockNode node)
    {
        var match = FenceRegex.Match(Current);
        if (!match.Success)
        {
            node = null!;
            return false;
        }

        var fence = match.Groups["fence"].Value;
        var fenceChar = fence[0];
        var info = match.Groups["info"].Value.Trim();
        var language = info.Length == 0 ? null : info.Split(' ', StringSplitOptions.RemoveEmptyEntries)[0];
        var content = new List<string>();
        _index++;

        while (_index < _lines.Length)
        {
            var line = Current;
            var trimmed = line.TrimStart();
            if (trimmed.Length >= fence.Length && trimmed.All(ch => ch == fenceChar))
            {
                _index++;
                break;
            }

            content.Add(line);
            _index++;
        }

        node = new CodeBlockNode(language, string.Join("\n", content) + "\n");
        return true;
    }

    private bool TryParseHeading(out HeadingNode node)
    {
        var match = AtxHeadingRegex.Match(Current);
        if (!match.Success)
        {
            node = null!;
            return false;
        }

        var level = match.Groups["marks"].Value.Length;
        var text = match.Groups["text"].Value.Trim();
        node = new HeadingNode(level, MarkdownParser.ParseInlines(text, _enableGfm, _depth));
        _index++;
        return true;
    }

    private bool TryParseBlockquote(out BlockquoteNode node)
    {
        if (!IsBlockquoteLine(Current))
        {
            node = null!;
            return false;
        }

        var innerLines = new List<string>();
        while (_index < _lines.Length && (IsBlank(Current) || IsBlockquoteLine(Current)))
        {
            innerLines.Add(IsBlank(Current) ? string.Empty : StripBlockquoteMarker(Current));
            _index++;
        }

        var innerDocument = MarkdownParser.Parse(string.Join("\n", innerLines), _enableGfm, _depth + 1);
        node = new BlockquoteNode(innerDocument.Children);
        return true;
    }

    private bool TryParseList(out ListNode node)
    {
        if (!TryParseListItemStart(Current, out var ordered, out var start, out var firstText))
        {
            node = null!;
            return false;
        }

        var children = new List<IListChildNode>();
        var tight = true;

        while (_index < _lines.Length)
        {
            if (!TryParseListItemStart(Current, out var currentOrdered, out _, out var itemText) || currentOrdered != ordered)
            {
                break;
            }

            _index++;
            var lines = new List<string> { itemText };

            while (_index < _lines.Length)
            {
                if (IsBlank(Current))
                {
                    tight = false;
                    lines.Add(string.Empty);
                    _index++;

                    if (_index < _lines.Length && TryParseListItemStart(Current, out var nextOrdered, out _, out _) && nextOrdered == ordered)
                    {
                        break;
                    }

                    continue;
                }

                if (TryParseListItemStart(Current, out var nextItemOrdered, out _, out _) && nextItemOrdered == ordered)
                {
                    break;
                }

                if (StartsOtherBlock(Current))
                {
                    break;
                }

                lines.Add(StripContinuationIndent(Current));
                _index++;
            }

            var firstLine = lines[0];
            var isTaskItem = false;
            var isChecked = false;
            if (_enableGfm && TryStripTaskMarker(firstLine, out isChecked, out var stripped))
            {
                isTaskItem = true;
                lines[0] = stripped;
            }

            var childDocument = MarkdownParser.Parse(string.Join("\n", lines).TrimEnd('\n'), _enableGfm, _depth + 1);
            children.Add(
                isTaskItem
                    ? new TaskItemNode(isChecked, childDocument.Children)
                    : new ListItemNode(childDocument.Children));
        }

        node = new ListNode(ordered, ordered ? start : null, tight, children);
        return true;
    }

    private bool TryParseRawHtmlBlock(out RawBlockNode node)
    {
        var trimmed = Current.Trim();
        if (LooksLikeRawHtmlBlock(trimmed))
        {
            node = new RawBlockNode("html", trimmed);
            _index++;
            return true;
        }

        node = null!;
        return false;
    }

    private bool TryParseTable(out TableNode node)
    {
        node = null!;
        if (_index + 1 >= _lines.Length)
        {
            return false;
        }

        if (!TrySplitTableRow(Current, out var headerCells) ||
            !TryParseDelimiterRow(_lines[_index + 1], out var alignment) ||
            headerCells.Count != alignment.Count)
        {
            return false;
        }

        var rows = new List<TableRowNode>
        {
            new(true, headerCells.Select(cell => new TableCellNode(MarkdownParser.ParseInlines(cell, enableGfm: true, _depth))).ToArray())
        };

        _index += 2;
        while (_index < _lines.Length && TrySplitTableRow(Current, out var bodyCells))
        {
            if (bodyCells.Count != alignment.Count)
            {
                break;
            }

            rows.Add(new TableRowNode(false, bodyCells.Select(cell => new TableCellNode(MarkdownParser.ParseInlines(cell, enableGfm: true, _depth))).ToArray()));
            _index++;
        }

        node = new TableNode(alignment, rows);
        return true;
    }

    private string Current => _lines[_index];

    private static string Normalize(string markdown) => markdown.Replace("\r\n", "\n", StringComparison.Ordinal).Replace('\r', '\n');

    private static bool IsBlank(string line) => string.IsNullOrWhiteSpace(line);

    private bool StartsOtherBlock(string line)
    {
        if (FenceRegex.IsMatch(line) || AtxHeadingRegex.IsMatch(line) || IsThematicBreak(line) || IsBlockquoteLine(line))
        {
            return true;
        }

        if (TryParseListItemStart(line, out _, out _, out _))
        {
            return true;
        }

        if (_enableGfm && _index + 1 < _lines.Length && TrySplitTableRow(line, out _) && TryParseDelimiterRow(_lines[_index + 1], out _))
        {
            return true;
        }

        return LooksLikeRawHtmlBlock(line.Trim());
    }

    private static bool IsSetextUnderline(string line, out int level)
    {
        var trimmed = line.Trim();
        if (trimmed.Length >= 3 && trimmed.All(ch => ch == '='))
        {
            level = 1;
            return true;
        }

        if (trimmed.Length >= 3 && trimmed.All(ch => ch == '-'))
        {
            level = 2;
            return true;
        }

        level = 0;
        return false;
    }

    private static bool IsThematicBreak(string line)
    {
        var trimmed = string.Concat(line.Where(ch => !char.IsWhiteSpace(ch)));
        if (trimmed.Length < 3)
        {
            return false;
        }

        var first = trimmed[0];
        return (first == '-' || first == '*' || first == '_') && trimmed.All(ch => ch == first);
    }

    private static bool IsBlockquoteLine(string line)
    {
        var trimmed = line.TrimStart();
        return trimmed.StartsWith(">", StringComparison.Ordinal);
    }

    private static string StripBlockquoteMarker(string line)
    {
        var trimmed = line.TrimStart();
        return trimmed.Length > 1 && trimmed[1] == ' ' ? trimmed[2..] : trimmed[1..];
    }

    private static bool TryParseListItemStart(string line, out bool ordered, out int start, out string text)
    {
        var orderedMatch = OrderedListRegex.Match(line);
        if (orderedMatch.Success)
        {
            if (!int.TryParse(orderedMatch.Groups["num"].Value, out var parsedStart))
            {
                ordered = false;
                start = 0;
                text = string.Empty;
                return false;
            }

            ordered = true;
            start = parsedStart;
            text = orderedMatch.Groups["text"].Value;
            return true;
        }

        var bulletMatch = BulletListRegex.Match(line);
        if (bulletMatch.Success)
        {
            ordered = false;
            start = 0;
            text = bulletMatch.Groups["text"].Value;
            return true;
        }

        ordered = false;
        start = 0;
        text = string.Empty;
        return false;
    }

    private static string StripContinuationIndent(string line)
    {
        var spaces = 0;
        while (spaces < line.Length && spaces < 4 && line[spaces] == ' ')
        {
            spaces++;
        }

        return line[spaces..];
    }

    private static bool TryStripTaskMarker(string text, out bool isChecked, out string remaining)
    {
        var match = TaskMarkerRegex.Match(text);
        if (!match.Success)
        {
            isChecked = false;
            remaining = text;
            return false;
        }

        isChecked = match.Groups["mark"].Value.Equals("x", StringComparison.OrdinalIgnoreCase);
        remaining = match.Groups["text"].Value;
        return true;
    }

    private static bool TrySplitTableRow(string line, out List<string> cells)
    {
        var trimmed = line.Trim();
        cells = new List<string>();
        if (!trimmed.Contains('|', StringComparison.Ordinal))
        {
            return false;
        }

        if (trimmed.StartsWith("|", StringComparison.Ordinal))
        {
            trimmed = trimmed[1..];
        }

        if (trimmed.EndsWith("|", StringComparison.Ordinal))
        {
            trimmed = trimmed[..^1];
        }

        cells.AddRange(trimmed.Split('|').Select(cell => cell.Trim()));
        return cells.Count > 0;
    }

    private static bool TryParseDelimiterRow(string line, out IReadOnlyList<TableAlignment?> alignment)
    {
        alignment = Array.Empty<TableAlignment?>();
        if (!TrySplitTableRow(line, out var cells))
        {
            return false;
        }

        var parsed = new List<TableAlignment?>();
        foreach (var cell in cells)
        {
            if (!TableDelimiterCellRegex.IsMatch(cell))
            {
                return false;
            }

            var left = cell.StartsWith(':');
            var right = cell.EndsWith(':');
            parsed.Add(left && right ? TableAlignment.Center : left ? TableAlignment.Left : right ? TableAlignment.Right : null);
        }

        alignment = parsed;
        return parsed.Count > 0;
    }

    private static bool LooksLikeRawHtmlBlock(string trimmed)
    {
        if (!trimmed.StartsWith("<", StringComparison.Ordinal) || !trimmed.EndsWith(">", StringComparison.Ordinal))
        {
            return false;
        }

        var inner = trimmed[1..^1].Trim();
        if (inner.Length == 0)
        {
            return false;
        }

        if (inner.StartsWith("http://", StringComparison.OrdinalIgnoreCase) ||
            inner.StartsWith("https://", StringComparison.OrdinalIgnoreCase) ||
            inner.StartsWith("mailto:", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        if (inner.Contains('@') && !inner.Contains('/') && !inner.Contains(' '))
        {
            return false;
        }

        var first = inner[0];
        return char.IsLetter(first) || first == '/' || first == '!' || first == '?';
    }
}

internal static class InlineParser
{
    private const int MaxInlineSearchWindow = 4_096;

    public static IReadOnlyList<IInlineNode> Parse(string text, bool enableGfm, int depth)
    {
        MarkdownParser.EnsureWithinLimits(text.Length, depth);
        var nodes = new List<IInlineNode>();
        var buffer = new StringBuilder();
        var index = 0;

        while (index < text.Length)
        {
            if (text[index] == '\\' && index + 1 < text.Length && text[index + 1] == '\n')
            {
                FlushText(buffer, nodes);
                nodes.Add(new HardBreakNode());
                index += 2;
                continue;
            }

            if (text[index] == '\n')
            {
                var hardBreak = RemoveTrailingBreakSpaces(buffer);
                FlushText(buffer, nodes);
                nodes.Add(hardBreak ? new HardBreakNode() : new SoftBreakNode());
                index++;
                continue;
            }

            if (TryParseImage(text, ref index, enableGfm, depth, nodes, buffer) ||
                TryParseLink(text, ref index, enableGfm, depth, nodes, buffer) ||
                TryParseCodeSpan(text, ref index, nodes, buffer) ||
                TryParseDelimited(text, ref index, "**", content => new StrongNode(Parse(content, enableGfm, depth + 1)), nodes, buffer) ||
                TryParseDelimited(text, ref index, "__", content => new StrongNode(Parse(content, enableGfm, depth + 1)), nodes, buffer) ||
                TryParseDelimited(text, ref index, "*", content => new EmphasisNode(Parse(content, enableGfm, depth + 1)), nodes, buffer) ||
                TryParseDelimited(text, ref index, "_", content => new EmphasisNode(Parse(content, enableGfm, depth + 1)), nodes, buffer) ||
                (enableGfm && TryParseDelimited(text, ref index, "~~", content => new StrikethroughNode(Parse(content, enableGfm, depth + 1)), nodes, buffer)) ||
                TryParseAngle(text, ref index, nodes, buffer))
            {
                continue;
            }

            if (TryDecodeEntity(text, ref index, out var decoded))
            {
                buffer.Append(decoded);
                continue;
            }

            buffer.Append(text[index]);
            index++;
        }

        FlushText(buffer, nodes);
        return nodes;
    }

    private static bool TryParseDelimited(string text, ref int index, string delimiter, Func<string, IInlineNode> factory, List<IInlineNode> nodes, StringBuilder buffer)
    {
        if (!text.AsSpan(index).StartsWith(delimiter, StringComparison.Ordinal))
        {
            return false;
        }

        var start = index + delimiter.Length;
        var end = FindStringWithinWindow(text, delimiter, start);
        if (end < 0 || end == start)
        {
            return false;
        }

        FlushText(buffer, nodes);
        nodes.Add(factory(text[start..end]));
        index = end + delimiter.Length;
        return true;
    }

    private static bool TryParseCodeSpan(string text, ref int index, List<IInlineNode> nodes, StringBuilder buffer)
    {
        if (text[index] != '`')
        {
            return false;
        }

        var tickCount = 1;
        while (index + tickCount < text.Length && text[index + tickCount] == '`')
        {
            tickCount++;
        }

        var delimiter = new string('`', tickCount);
        var end = FindStringWithinWindow(text, delimiter, index + tickCount);
        if (end < 0)
        {
            return false;
        }

        var content = text.Substring(index + tickCount, end - index - tickCount);
        if (content.Length > 1 && content.StartsWith(' ') && content.EndsWith(' '))
        {
            content = content[1..^1];
        }

        FlushText(buffer, nodes);
        nodes.Add(new CodeSpanNode(content));
        index = end + tickCount;
        return true;
    }

    private static bool TryParseLink(string text, ref int index, bool enableGfm, int depth, List<IInlineNode> nodes, StringBuilder buffer)
    {
        if (text[index] != '[')
        {
            return false;
        }

        var closingBracket = FindClosingBracket(text, index + 1);
        if (closingBracket < 0 || closingBracket + 1 >= text.Length || text[closingBracket + 1] != '(')
        {
            return false;
        }

        var closingParen = FindClosingParen(text, closingBracket + 2);
        if (closingParen < 0)
        {
            return false;
        }

        var label = text[(index + 1)..closingBracket];
        var target = text.Substring(closingBracket + 2, closingParen - closingBracket - 2);
        if (!TryParseDestinationAndTitle(target, out var destination, out var title))
        {
            return false;
        }

        FlushText(buffer, nodes);
        nodes.Add(new LinkNode(destination, title, Parse(label, enableGfm, depth + 1)));
        index = closingParen + 1;
        return true;
    }

    private static bool TryParseImage(string text, ref int index, bool enableGfm, int depth, List<IInlineNode> nodes, StringBuilder buffer)
    {
        if (text[index] != '!' || index + 1 >= text.Length || text[index + 1] != '[')
        {
            return false;
        }

        var imageIndex = index + 1;
        var closingBracket = FindClosingBracket(text, imageIndex + 1);
        if (closingBracket < 0 || closingBracket + 1 >= text.Length || text[closingBracket + 1] != '(')
        {
            return false;
        }

        var closingParen = FindClosingParen(text, closingBracket + 2);
        if (closingParen < 0)
        {
            return false;
        }

        var label = text[(imageIndex + 1)..closingBracket];
        var target = text.Substring(closingBracket + 2, closingParen - closingBracket - 2);
        if (!TryParseDestinationAndTitle(target, out var destination, out var title))
        {
            return false;
        }

        FlushText(buffer, nodes);
        var alt = ToPlainText(Parse(label, enableGfm, depth + 1));
        nodes.Add(new ImageNode(destination, title, alt));
        index = closingParen + 1;
        return true;
    }

    private static bool TryParseAngle(string text, ref int index, List<IInlineNode> nodes, StringBuilder buffer)
    {
        if (text[index] != '<')
        {
            return false;
        }

        var end = FindCharWithinWindow(text, '>', index + 1);
        if (end < 0)
        {
            return false;
        }

        var inner = text[(index + 1)..end];
        if (inner.Length == 0 || inner.Contains(' '))
        {
            return false;
        }

        FlushText(buffer, nodes);

        if (inner.StartsWith("http://", StringComparison.OrdinalIgnoreCase) || inner.StartsWith("https://", StringComparison.OrdinalIgnoreCase))
        {
            nodes.Add(new AutolinkNode(inner, false));
        }
        else if (inner.StartsWith("mailto:", StringComparison.OrdinalIgnoreCase))
        {
            nodes.Add(new AutolinkNode(inner["mailto:".Length..], true));
        }
        else if (inner.Contains('@'))
        {
            nodes.Add(new AutolinkNode(inner, true));
        }
        else
        {
            nodes.Add(new RawInlineNode("html", $"<{inner}>"));
        }

        index = end + 1;
        return true;
    }

    private static bool TryDecodeEntity(string text, ref int index, out string decoded)
    {
        decoded = string.Empty;
        if (text[index] != '&')
        {
            return false;
        }

        var end = FindCharWithinWindow(text, ';', index + 1, 16);
        if (end < 0 || end - index > 16)
        {
            return false;
        }

        var candidate = text[index..(end + 1)];
        var resolved = WebUtility.HtmlDecode(candidate);
        if (resolved == candidate)
        {
            return false;
        }

        decoded = resolved;
        index = end + 1;
        return true;
    }

    private static bool TryParseDestinationAndTitle(string target, out string destination, out string? title)
    {
        target = target.Trim();
        destination = string.Empty;
        title = null;
        if (target.Length == 0)
        {
            return false;
        }

        if (target.StartsWith("<", StringComparison.Ordinal) && target.EndsWith(">", StringComparison.Ordinal))
        {
            destination = target[1..^1];
            return destination.Length > 0;
        }

        var match = Regex.Match(target, @"^(?<dest>\S+)(?:\s+[""'](?<title>.*)[""'])?$");
        if (!match.Success)
        {
            return false;
        }

        destination = match.Groups["dest"].Value;
        title = match.Groups["title"].Success ? match.Groups["title"].Value : null;
        return destination.Length > 0;
    }

    private static int FindClosingBracket(string text, int start)
    {
        var depth = 0;
        var limit = Math.Min(text.Length, start + MaxInlineSearchWindow);
        for (var i = start; i < limit; i++)
        {
            if (text[i] == '[')
            {
                depth++;
            }
            else if (text[i] == ']')
            {
                if (depth == 0)
                {
                    return i;
                }

                depth--;
            }
        }

        return -1;
    }

    private static int FindClosingParen(string text, int start)
    {
        var depth = 1;
        var limit = Math.Min(text.Length, start + MaxInlineSearchWindow);
        for (var i = start; i < limit; i++)
        {
            if (text[i] == '(')
            {
                depth++;
            }
            else if (text[i] == ')')
            {
                depth--;
                if (depth == 0)
                {
                    return i;
                }
            }
        }

        return -1;
    }

    private static int FindStringWithinWindow(string text, string value, int start)
    {
        var searchLength = Math.Min(text.Length - start, MaxInlineSearchWindow);
        if (searchLength <= 0)
        {
            return -1;
        }

        return text.IndexOf(value, start, searchLength, StringComparison.Ordinal);
    }

    private static int FindCharWithinWindow(string text, char value, int start, int? explicitWindow = null)
    {
        var searchLength = Math.Min(text.Length - start, explicitWindow ?? MaxInlineSearchWindow);
        if (searchLength <= 0)
        {
            return -1;
        }

        return text.IndexOf(value, start, searchLength);
    }

    private static bool RemoveTrailingBreakSpaces(StringBuilder buffer)
    {
        var count = 0;
        for (var i = buffer.Length - 1; i >= 0 && buffer[i] == ' '; i--)
        {
            count++;
        }

        if (count < 2)
        {
            return false;
        }

        buffer.Length -= 2;
        return true;
    }

    private static string ToPlainText(IReadOnlyList<IInlineNode> nodes)
    {
        var builder = new StringBuilder();
        foreach (var node in nodes)
        {
            switch (node)
            {
                case TextNode text:
                    builder.Append(text.Value);
                    break;
                case EmphasisNode emphasis:
                    builder.Append(ToPlainText(emphasis.Children));
                    break;
                case StrongNode strong:
                    builder.Append(ToPlainText(strong.Children));
                    break;
                case StrikethroughNode strike:
                    builder.Append(ToPlainText(strike.Children));
                    break;
                case LinkNode link:
                    builder.Append(ToPlainText(link.Children));
                    break;
                case ImageNode image:
                    builder.Append(image.Alt);
                    break;
                case CodeSpanNode code:
                    builder.Append(code.Value);
                    break;
                case AutolinkNode autolink:
                    builder.Append(autolink.Destination);
                    break;
                case RawInlineNode raw:
                    builder.Append(raw.Value);
                    break;
                case HardBreakNode:
                case SoftBreakNode:
                    builder.Append(' ');
                    break;
            }
        }

        return builder.ToString();
    }

    private static void FlushText(StringBuilder buffer, List<IInlineNode> nodes)
    {
        if (buffer.Length == 0)
        {
            return;
        }

        var value = WebUtility.HtmlDecode(buffer.ToString());
        if (value.Length > 0)
        {
            nodes.Add(new TextNode(value));
        }

        buffer.Clear();
    }
}
