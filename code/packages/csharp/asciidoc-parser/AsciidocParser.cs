using System.Text;
using CodingAdventures.DocumentAst;

namespace CodingAdventures.AsciidocParser;

/// <summary>Parses a portable AsciiDoc subset into the shared document AST.</summary>
public static class AsciidocParser
{
    private enum BlockState
    {
        Normal,
        Paragraph,
        Code,
        Literal,
        Passthrough,
        Quote,
        UnorderedList,
        OrderedList,
    }

    private readonly record struct ListEntry(int Level, string Text);

    /// <summary>Parse a complete AsciiDoc source string.</summary>
    public static DocumentNode Parse(string source)
    {
        ArgumentNullException.ThrowIfNull(source);
        var normalized = source.Replace("\r\n", "\n", StringComparison.Ordinal).Replace('\r', '\n');
        var lines = normalized.Split('\n').ToList();
        if (normalized.EndsWith('\n'))
        {
            lines.RemoveAt(lines.Count - 1);
        }

        var blocks = new List<IBlockNode>();
        var paragraphLines = new List<string>();
        var delimitedLines = new List<string>();
        var listEntries = new List<ListEntry>();
        var state = BlockState.Normal;
        string? pendingLanguage = null;
        var listOrdered = false;

        void FlushParagraph()
        {
            if (paragraphLines.Count == 0)
            {
                return;
            }

            blocks.Add(new ParagraphNode(ParseInline(string.Join('\n', paragraphLines))));
            paragraphLines.Clear();
        }

        void FlushList()
        {
            if (listEntries.Count == 0)
            {
                return;
            }

            blocks.Add(BuildList(listEntries, listOrdered));
            listEntries.Clear();
        }

        foreach (var rawLine in lines)
        {
            var line = rawLine.TrimEnd();

            if (state is BlockState.Code or BlockState.Literal or BlockState.Passthrough or BlockState.Quote)
            {
                var delimiter = state switch
                {
                    BlockState.Code => '-',
                    BlockState.Literal => '.',
                    BlockState.Passthrough => '+',
                    _ => '_',
                };

                if (IsDelimiter(line, delimiter, 4))
                {
                    EmitDelimited(blocks, state, delimitedLines, pendingLanguage);
                    if (state == BlockState.Code)
                    {
                        pendingLanguage = null;
                    }

                    delimitedLines.Clear();
                    state = BlockState.Normal;
                }
                else
                {
                    delimitedLines.Add(rawLine);
                }

                continue;
            }

            if (state == BlockState.Paragraph)
            {
                if (line.Length == 0)
                {
                    FlushParagraph();
                    state = BlockState.Normal;
                    continue;
                }

                if (StartsNewBlock(line))
                {
                    FlushParagraph();
                    state = BlockState.Normal;
                }
                else
                {
                    paragraphLines.Add(line);
                    continue;
                }
            }

            if (state is BlockState.UnorderedList or BlockState.OrderedList)
            {
                if (line.Length == 0)
                {
                    FlushList();
                    state = BlockState.Normal;
                    continue;
                }

                var marker = state == BlockState.OrderedList ? '.' : '*';
                if (TryListItem(line, marker, out var entry))
                {
                    listEntries.Add(entry);
                    continue;
                }

                FlushList();
                state = BlockState.Normal;
            }

            if (line.Length == 0 || line.StartsWith("//", StringComparison.Ordinal))
            {
                continue;
            }

            if (TrySourceAttribute(line, out var language))
            {
                pendingLanguage = language;
                continue;
            }

            if (TryHeading(line, out var level, out var headingText))
            {
                blocks.Add(new HeadingNode(level, ParseInline(headingText)));
                continue;
            }

            if (IsDelimiter(line, '\'', 3))
            {
                blocks.Add(new ThematicBreakNode());
                continue;
            }

            if (IsDelimiter(line, '-', 4))
            {
                state = BlockState.Code;
                continue;
            }

            if (IsDelimiter(line, '.', 4))
            {
                state = BlockState.Literal;
                continue;
            }

            if (IsDelimiter(line, '+', 4))
            {
                state = BlockState.Passthrough;
                continue;
            }

            if (IsDelimiter(line, '_', 4))
            {
                state = BlockState.Quote;
                continue;
            }

            if (TryListItem(line, '*', out var unordered))
            {
                listOrdered = false;
                listEntries.Add(unordered);
                state = BlockState.UnorderedList;
                continue;
            }

            if (TryListItem(line, '.', out var ordered))
            {
                listOrdered = true;
                listEntries.Add(ordered);
                state = BlockState.OrderedList;
                continue;
            }

            paragraphLines.Add(line);
            state = BlockState.Paragraph;
        }

        FlushParagraph();
        FlushList();
        if (delimitedLines.Count > 0 && state is BlockState.Code or BlockState.Literal or BlockState.Passthrough or BlockState.Quote)
        {
            EmitDelimited(blocks, state, delimitedLines, pendingLanguage);
        }

        return new DocumentNode(blocks);
    }

    /// <summary>Parse inline AsciiDoc markup into shared inline AST nodes.</summary>
    public static IReadOnlyList<IInlineNode> ParseInline(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        var output = new List<IInlineNode>();
        var plain = new StringBuilder();
        var index = 0;

        void FlushText()
        {
            if (plain.Length == 0)
            {
                return;
            }

            output.Add(new TextNode(plain.ToString()));
            plain.Clear();
        }

        while (index < text.Length)
        {
            if (StartsWith(text, index, "  \n"))
            {
                FlushText();
                output.Add(new HardBreakNode());
                index += 3;
                continue;
            }

            if (StartsWith(text, index, "\\\n"))
            {
                FlushText();
                output.Add(new HardBreakNode());
                index += 2;
                continue;
            }

            if (text[index] == '\n')
            {
                FlushText();
                output.Add(new SoftBreakNode());
                index++;
                continue;
            }

            if (TryDelimitedInline(text, ref index, "`", value => new CodeSpanNode(value), output, FlushText) ||
                TryDelimitedInline(text, ref index, "**", value => new StrongNode(ParseInline(value)), output, FlushText) ||
                TryDelimitedInline(text, ref index, "__", value => new EmphasisNode(ParseInline(value)), output, FlushText) ||
                TryDelimitedInline(text, ref index, "*", value => new StrongNode(ParseInline(value)), output, FlushText) ||
                TryDelimitedInline(text, ref index, "_", value => new EmphasisNode(ParseInline(value)), output, FlushText))
            {
                continue;
            }

            if (TryMacro(text, ref index, "link:", false, output, FlushText) ||
                TryMacro(text, ref index, "image:", true, output, FlushText) ||
                TryCrossReference(text, ref index, output, FlushText) ||
                TryUrl(text, ref index, output, FlushText))
            {
                continue;
            }

            plain.Append(text[index]);
            index++;
        }

        FlushText();
        return output;
    }

    private static bool TryDelimitedInline(
        string text,
        ref int index,
        string delimiter,
        Func<string, IInlineNode> create,
        List<IInlineNode> output,
        Action flush)
    {
        if (!StartsWith(text, index, delimiter))
        {
            return false;
        }

        var closing = text.IndexOf(delimiter, index + delimiter.Length, StringComparison.Ordinal);
        if (closing < 0)
        {
            return false;
        }

        flush();
        output.Add(create(text[(index + delimiter.Length)..closing]));
        index = closing + delimiter.Length;
        return true;
    }

    private static bool TryMacro(string text, ref int index, string prefix, bool image, List<IInlineNode> output, Action flush)
    {
        if (!StartsWith(text, index, prefix))
        {
            return false;
        }

        var open = text.IndexOf('[', index + prefix.Length);
        if (open < 0)
        {
            return false;
        }

        var close = text.IndexOf(']', open + 1);
        if (close < 0)
        {
            return false;
        }

        var destination = text[(index + prefix.Length)..open];
        var label = text[(open + 1)..close];
        flush();
        output.Add(image
            ? new ImageNode(destination, null, label)
            : new LinkNode(destination, null, [new TextNode(label.Length == 0 ? destination : label)]));
        index = close + 1;
        return true;
    }

    private static bool TryCrossReference(string text, ref int index, List<IInlineNode> output, Action flush)
    {
        if (!StartsWith(text, index, "<<"))
        {
            return false;
        }

        var close = text.IndexOf(">>", index + 2, StringComparison.Ordinal);
        if (close < 0)
        {
            return false;
        }

        var parts = text[(index + 2)..close].Split(',', 2);
        var anchor = parts[0].Trim();
        var label = parts.Length == 2 ? parts[1].Trim() : anchor;
        flush();
        output.Add(new LinkNode($"#{anchor}", null, [new TextNode(label)]));
        index = close + 2;
        return true;
    }

    private static bool TryUrl(string text, ref int index, List<IInlineNode> output, Action flush)
    {
        var schemeLength = StartsWith(text, index, "https://") ? 8 : StartsWith(text, index, "http://") ? 7 : 0;
        if (schemeLength == 0)
        {
            return false;
        }

        var end = index + schemeLength;
        while (end < text.Length && !char.IsWhiteSpace(text[end]) && text[end] is not '[' and not ']')
        {
            end++;
        }

        var url = text[index..end];
        flush();
        if (end < text.Length && text[end] == '[')
        {
            var close = text.IndexOf(']', end + 1);
            if (close >= 0)
            {
                var label = text[(end + 1)..close];
                output.Add(new LinkNode(url, null, [new TextNode(label.Length == 0 ? url : label)]));
                index = close + 1;
                return true;
            }
        }

        output.Add(new AutolinkNode(url, false));
        index = end;
        return true;
    }

    private static ListNode BuildList(IReadOnlyList<ListEntry> entries, bool ordered)
    {
        var index = 0;
        return BuildListLevel(entries[0].Level, entries, ordered, ref index);
    }

    private static ListNode BuildListLevel(int level, IReadOnlyList<ListEntry> entries, bool ordered, ref int index)
    {
        var children = new List<IListChildNode>();
        while (index < entries.Count && entries[index].Level >= level)
        {
            if (entries[index].Level > level)
            {
                break;
            }

            var entry = entries[index++];
            var itemBlocks = new List<IBlockNode> { new ParagraphNode(ParseInline(entry.Text)) };
            while (index < entries.Count && entries[index].Level > level)
            {
                itemBlocks.Add(BuildListLevel(entries[index].Level, entries, ordered, ref index));
            }

            children.Add(new ListItemNode(itemBlocks));
        }

        return new ListNode(ordered, ordered ? 1 : null, true, children);
    }

    private static void EmitDelimited(List<IBlockNode> blocks, BlockState state, List<string> lines, string? language)
    {
        var value = lines.Count == 0 ? string.Empty : string.Join('\n', lines) + "\n";
        switch (state)
        {
            case BlockState.Code:
                blocks.Add(new CodeBlockNode(language, value));
                break;
            case BlockState.Literal:
                blocks.Add(new CodeBlockNode(null, value));
                break;
            case BlockState.Passthrough:
                blocks.Add(new RawBlockNode("html", string.Join('\n', lines)));
                break;
            case BlockState.Quote:
                blocks.Add(new BlockquoteNode(Parse(string.Join('\n', lines)).Children));
                break;
        }
    }

    private static bool StartsNewBlock(string line) =>
        TryHeading(line, out _, out _) ||
        TrySourceAttribute(line, out _) ||
        TryListItem(line, '*', out _) ||
        TryListItem(line, '.', out _) ||
        line.StartsWith("//", StringComparison.Ordinal) ||
        IsDelimiter(line, '\'', 3) ||
        IsDelimiter(line, '-', 4) ||
        IsDelimiter(line, '.', 4) ||
        IsDelimiter(line, '+', 4) ||
        IsDelimiter(line, '_', 4);

    private static bool TryHeading(string line, out int level, out string text)
    {
        var count = 0;
        while (count < line.Length && line[count] == '=')
        {
            count++;
        }

        if (count > 0 && count < line.Length && char.IsWhiteSpace(line[count]))
        {
            level = Math.Min(count, 6);
            text = line[count..].TrimStart();
            return true;
        }

        level = 0;
        text = string.Empty;
        return false;
    }

    private static bool TrySourceAttribute(string line, out string language)
    {
        if (line.StartsWith("[source", StringComparison.OrdinalIgnoreCase) && line.EndsWith(']'))
        {
            var comma = line.IndexOf(',');
            if (comma >= 0)
            {
                language = line[(comma + 1)..^1].Trim();
                return language.Length > 0;
            }
        }

        language = string.Empty;
        return false;
    }

    private static bool TryListItem(string line, char marker, out ListEntry entry)
    {
        var count = 0;
        while (count < line.Length && line[count] == marker)
        {
            count++;
        }

        if (count > 0 && count < line.Length && line[count] == ' ')
        {
            entry = new ListEntry(count, line[(count + 1)..].Trim());
            return true;
        }

        entry = default;
        return false;
    }

    private static bool IsDelimiter(string line, char value, int minimum) =>
        line.Length >= minimum && line.All(character => character == value);

    private static bool StartsWith(string text, int index, string value) =>
        index + value.Length <= text.Length && text.AsSpan(index, value.Length).SequenceEqual(value);
}
