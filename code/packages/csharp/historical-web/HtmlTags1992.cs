using System.Text;

namespace CodingAdventures.HistoricalWeb;

/// <summary>Tree builder for the archived November 1992 HTML Tags profile.</summary>
public static class HtmlTags1992
{
    private const int NoIndex = -1;

    private enum Tag
    {
        Unknown,
        Title,
        Address,
        Anchor,
        H1,
        H2,
        H3,
        H4,
        H5,
        H6,
        Ul,
        Menu,
        Dir,
        Li,
        Dl,
        Dt,
        Dd,
        Paragraph,
        Isindex,
        Xmp,
        Listing,
        Plaintext,
        Count,
    }

    private static readonly string[] TagNames =
    [
        "", "title", "address", "a", "h1", "h2", "h3", "h4", "h5", "h6",
        "ul", "menu", "dir", "li", "dl", "dt", "dd", "p", "isindex",
        "xmp", "listing", "plaintext",
    ];

    private static readonly string[] EntityNames = ["&lt;", "&gt;", "&amp;"];
    private static readonly char[] EntityValues = ['<', '>', '&'];

    /// <summary>Parses source into an owned, flat arena using HWEB00 recovery rules.</summary>
    public static HtmlDocument Parse(string? source)
    {
        ArgumentNullException.ThrowIfNull(source);
        ValidateUnicode(source);
        return new Parser(NormalizeCrLf(source)).Run();
    }

    private static void ValidateUnicode(string source)
    {
        for (var i = 0; i < source.Length; i++)
        {
            var current = source[i];
            if (char.IsHighSurrogate(current))
            {
                if (i + 1 >= source.Length || !char.IsLowSurrogate(source[i + 1]))
                {
                    throw new HistoricalWebException(
                        HistoricalWebErrorKind.InvalidUnicode,
                        i,
                        "HTML source contains an unpaired UTF-16 high surrogate.");
                }

                i++;
            }
            else if (char.IsLowSurrogate(current))
            {
                throw new HistoricalWebException(
                    HistoricalWebErrorKind.InvalidUnicode,
                    i,
                    "HTML source contains an unpaired UTF-16 low surrogate.");
            }
        }
    }

    private static string NormalizeCrLf(string source)
    {
        if (!source.Contains("\r\n", StringComparison.Ordinal))
        {
            return source;
        }

        var normalized = new StringBuilder(source.Length);
        for (var i = 0; i < source.Length; i++)
        {
            if (source[i] == '\r' && i + 1 < source.Length && source[i + 1] == '\n')
            {
                normalized.Append('\n');
                i++;
            }
            else
            {
                normalized.Append(source[i]);
            }
        }

        return normalized.ToString();
    }

    private sealed class Parser
    {
        private readonly string _source;
        private readonly List<HtmlNode> _nodes = [];
        private readonly List<HtmlAttribute> _attributes = [];
        private readonly List<NodeMeta> _metadata = [];
        private readonly List<int> _openStack = [];
        private readonly int[] _latestOpen = new int[(int)Tag.Count];
        private readonly StringBuilder _pendingText = new();
        private int _rootCount;
        private int _lastRoot = NoIndex;
        private Tag _rawTag;

        public Parser(string source)
        {
            _source = source;
            Array.Fill(_latestOpen, NoIndex);
        }

        public HtmlDocument Run()
        {
            var cursor = 0;
            while (cursor < _source.Length)
            {
                if (_rawTag != Tag.Unknown)
                {
                    if (_rawTag == Tag.Plaintext)
                    {
                        AppendText(_source.AsSpan(cursor), decodeEntities: false);
                        break;
                    }

                    var rawEnd = FindRawEnd(cursor, _rawTag);
                    if (rawEnd is null)
                    {
                        AppendText(_source.AsSpan(cursor), decodeEntities: false);
                        break;
                    }

                    AppendText(_source.AsSpan(cursor, rawEnd.Value.Start - cursor), decodeEntities: false);
                    cursor = rawEnd.Value.End;
                    var endedTag = _rawTag;
                    _rawTag = Tag.Unknown;
                    HandleTag(endedTag, closing: true, ReadOnlySpan<char>.Empty);
                    continue;
                }

                var start = _source.IndexOf('<', cursor);
                if (start < 0)
                {
                    AppendText(_source.AsSpan(cursor), decodeEntities: true);
                    break;
                }

                if (start > cursor)
                {
                    AppendText(_source.AsSpan(cursor, start - cursor), decodeEntities: true);
                }

                if (_source.AsSpan(start).StartsWith("<!--", StringComparison.Ordinal))
                {
                    var commentEnd = _source.IndexOf("-->", start + 4, StringComparison.Ordinal);
                    if (commentEnd < 0)
                    {
                        break;
                    }

                    cursor = commentEnd + 3;
                    continue;
                }

                if (!TryParseTag(_source, start, out var parsed))
                {
                    AppendText("<".AsSpan(), decodeEntities: true);
                    cursor = start + 1;
                    continue;
                }

                cursor = parsed.End;
                if (parsed.Tag == Tag.Unknown || (parsed.Closing && IsEmpty(parsed.Tag)))
                {
                    continue;
                }

                HandleTag(parsed.Tag, parsed.Closing, parsed.Attributes.Span);
                if (!parsed.Closing && parsed.Tag == Tag.Plaintext)
                {
                    _rawTag = Tag.Plaintext;
                }
                else if (!parsed.Closing && parsed.Tag is Tag.Xmp or Tag.Listing)
                {
                    _rawTag = parsed.Tag;
                }
            }

            FlushText();
            return new HtmlDocument(_nodes.ToArray(), _attributes.ToArray(), _rootCount);
        }

        private bool IsEmpty(Tag tag) => tag is Tag.Paragraph or Tag.Isindex;

        private static bool IsHeading(Tag tag) => tag is >= Tag.H1 and <= Tag.H6;

        private static bool IsAsciiAlpha(char value) =>
            value is >= 'A' and <= 'Z' or >= 'a' and <= 'z';

        private static bool IsNameCharacter(char value) =>
            IsAsciiAlpha(value) || value is >= '0' and <= '9' or '-';

        private static bool IsTagSpace(char value) => value is ' ' or '\t' or '\n' or '\r' or '\f';

        private static char LowerAscii(char value) =>
            value is >= 'A' and <= 'Z' ? (char)(value + ('a' - 'A')) : value;

        private static bool EqualsAsciiIgnoreCase(ReadOnlySpan<char> left, ReadOnlySpan<char> right)
        {
            if (left.Length != right.Length)
            {
                return false;
            }

            for (var i = 0; i < left.Length; i++)
            {
                if (LowerAscii(left[i]) != right[i])
                {
                    return false;
                }
            }

            return true;
        }

        private static Tag IdentifyTag(ReadOnlySpan<char> name)
        {
            for (var i = 1; i < TagNames.Length; i++)
            {
                if (EqualsAsciiIgnoreCase(name, TagNames[i]))
                {
                    return (Tag)i;
                }
            }

            return Tag.Unknown;
        }

        private static bool TryParseTag(string source, int start, out ParsedTag parsed)
        {
            var cursor = start + 1;
            var closing = cursor < source.Length && source[cursor] == '/';
            if (closing)
            {
                cursor++;
            }

            if (cursor >= source.Length || !IsAsciiAlpha(source[cursor]))
            {
                parsed = default;
                return false;
            }

            var nameStart = cursor++;
            while (cursor < source.Length && IsNameCharacter(source[cursor]))
            {
                cursor++;
            }

            var tag = IdentifyTag(source.AsSpan(nameStart, cursor - nameStart));
            if (cursor >= source.Length || source[cursor] == '<')
            {
                parsed = default;
                return false;
            }

            if (source[cursor] == '>')
            {
                parsed = new ParsedTag(tag, closing, ReadOnlyMemory<char>.Empty, cursor + 1);
                return true;
            }

            if (!IsTagSpace(source[cursor]))
            {
                parsed = default;
                return false;
            }

            var attributesStart = cursor;
            while (cursor < source.Length)
            {
                var current = source[cursor];
                if (current == '>')
                {
                    parsed = new ParsedTag(
                        tag,
                        closing,
                        source.AsMemory(attributesStart, cursor - attributesStart),
                        cursor + 1);
                    return true;
                }

                if (current == '<')
                {
                    parsed = default;
                    return false;
                }

                if (current is '\'' or '"')
                {
                    var quote = current;
                    cursor++;
                    while (cursor < source.Length && source[cursor] != quote && source[cursor] != '<')
                    {
                        cursor++;
                    }

                    if (cursor >= source.Length || source[cursor] != quote)
                    {
                        parsed = default;
                        return false;
                    }
                }

                cursor++;
            }

            parsed = default;
            return false;
        }

        private void AppendText(ReadOnlySpan<char> text, bool decodeEntities)
        {
            if (text.IsEmpty)
            {
                return;
            }

            if (!decodeEntities)
            {
                _pendingText.Append(text);
                return;
            }

            AppendDecoded(_pendingText, text);
        }

        private static void AppendDecoded(StringBuilder destination, ReadOnlySpan<char> text)
        {
            var cursor = 0;
            while (cursor < text.Length)
            {
                if (text[cursor] == '&')
                {
                    var matched = false;
                    for (var entity = 0; entity < EntityNames.Length; entity++)
                    {
                        if (text[cursor..].StartsWith(EntityNames[entity], StringComparison.Ordinal))
                        {
                            destination.Append(EntityValues[entity]);
                            cursor += EntityNames[entity].Length;
                            matched = true;
                            break;
                        }
                    }

                    if (matched)
                    {
                        continue;
                    }
                }

                destination.Append(text[cursor]);
                cursor++;
            }
        }

        private void FlushText()
        {
            if (_pendingText.Length == 0)
            {
                return;
            }

            var parent = _openStack.Count == 0 ? NoIndex : _openStack[^1];
            // Unknown markup never flushes this buffer. A flush occurs only
            // at an effective element transition, so adjacent text is already
            // coalesced here and can be appended to the arena once.
            var text = new HtmlNode(
                HtmlNodeKind.Text,
                parent,
                NoIndex,
                NoIndex,
                string.Empty,
                _pendingText.ToString(),
                NoIndex,
                0);
            AppendNode(text, Tag.Unknown);
            _pendingText.Clear();
        }

        private int AppendNode(HtmlNode node, Tag tag)
        {
            var index = _nodes.Count;
            var parent = node.ParentIndex;
            _nodes.Add(node);
            _metadata.Add(new NodeMeta(_latestOpen[(int)tag], _openStack.Count, NoIndex, tag));

            if (parent != NoIndex)
            {
                var parentNode = _nodes[parent];
                if (parentNode.FirstChildIndex == NoIndex)
                {
                    _nodes[parent] = parentNode with { FirstChildIndex = index };
                }
                else
                {
                    var last = _metadata[parent].LastChild;
                    var lastNode = _nodes[last];
                    _nodes[last] = lastNode with { NextSiblingIndex = index };
                }

                _metadata[parent] = _metadata[parent] with { LastChild = index };
            }
            else
            {
                if (_lastRoot != NoIndex)
                {
                    var previousRoot = _nodes[_lastRoot];
                    _nodes[_lastRoot] = previousRoot with { NextSiblingIndex = index };
                }

                _lastRoot = index;
                _rootCount++;
            }

            if (tag != Tag.Unknown && !IsEmpty(tag))
            {
                _latestOpen[(int)tag] = index;
            }

            return index;
        }

        private int Matching(Tag tag, params Tag[] boundaries)
        {
            var candidate = _latestOpen[(int)tag];
            if (candidate == NoIndex)
            {
                return NoIndex;
            }

            // Absent tags are sentinels, not stack positions; select the
            // newest present boundary before deciding whether the match fits.
            var newestBoundary = NoIndex;
            foreach (var boundary in boundaries)
            {
                var boundaryNode = _latestOpen[(int)boundary];
                if (boundaryNode != NoIndex &&
                    (newestBoundary == NoIndex || _metadata[boundaryNode].StackSlot > _metadata[newestBoundary].StackSlot))
                {
                    newestBoundary = boundaryNode;
                }
            }

            if (newestBoundary != NoIndex &&
                _metadata[candidate].StackSlot <= _metadata[newestBoundary].StackSlot)
            {
                return NoIndex;
            }

            return _metadata[candidate].StackSlot;
        }

        private void CloseTo(int stackSlot)
        {
            while (_openStack.Count > stackSlot)
            {
                var index = _openStack[^1];
                _openStack.RemoveAt(_openStack.Count - 1);
                _latestOpen[(int)_metadata[index].Tag] = _metadata[index].PreviousOpen;
            }
        }

        private void HandleTag(Tag tag, bool closing, ReadOnlySpan<char> attributes)
        {
            var closeAt = NoIndex;
            if (closing)
            {
                closeAt = Matching(tag);
            }
            else if (tag == Tag.Li)
            {
                closeAt = Matching(Tag.Li, Tag.Ul, Tag.Menu, Tag.Dir);
            }
            else if (tag is Tag.Dt or Tag.Dd)
            {
                var term = Matching(Tag.Dt, Tag.Dl);
                var definition = Matching(Tag.Dd, Tag.Dl);
                closeAt = term == NoIndex ? definition :
                    definition == NoIndex ? term : Math.Max(term, definition);
            }
            else if (IsHeading(tag))
            {
                for (var heading = Tag.H1; heading <= Tag.H6; heading++)
                {
                    var candidate = Matching(heading);
                    if (candidate != NoIndex && (closeAt == NoIndex || candidate > closeAt))
                    {
                        closeAt = candidate;
                    }
                }
            }

            if (closing && closeAt == NoIndex)
            {
                return;
            }

            FlushText();
            if (closeAt != NoIndex)
            {
                CloseTo(closeAt);
            }

            if (closing)
            {
                return;
            }

            var parent = _openStack.Count == 0 ? NoIndex : _openStack[^1];
            var parsedAttributes = tag == Tag.Anchor ? ParseAnchorAttributes(attributes) : [];
            var firstAttribute = parsedAttributes.Count == 0 ? NoIndex : _attributes.Count;
            foreach (var attribute in parsedAttributes)
            {
                _attributes.Add(attribute);
            }

            var node = new HtmlNode(
                HtmlNodeKind.Element,
                parent,
                NoIndex,
                NoIndex,
                TagNames[(int)tag],
                string.Empty,
                firstAttribute,
                parsedAttributes.Count);
            var index = AppendNode(node, IsEmpty(tag) ? Tag.Unknown : tag);
            if (!IsEmpty(tag))
            {
                _openStack.Add(index);
            }
        }

        private static List<HtmlAttribute> ParseAnchorAttributes(ReadOnlySpan<char> source)
        {
            var result = new List<HtmlAttribute>();
            var seen = new bool[3];
            var cursor = 0;
            while (cursor < source.Length && TryParseAttribute(source, cursor, out var parsed))
            {
                var attributeSlot = EqualsAsciiIgnoreCase(parsed.Name, "href") ? 0 :
                    EqualsAsciiIgnoreCase(parsed.Name, "name") ? 1 :
                    EqualsAsciiIgnoreCase(parsed.Name, "type") ? 2 : -1;
                if (attributeSlot >= 0 && !seen[attributeSlot])
                {
                    seen[attributeSlot] = true;
                    var canonicalName = attributeSlot switch
                    {
                        0 => "href",
                        1 => "name",
                        _ => "type",
                    };
                    var value = parsed.Value is null ? null : DecodeEntities(parsed.Value);
                    result.Add(new HtmlAttribute(canonicalName, value));
                }

                if (parsed.Next <= cursor)
                {
                    break;
                }

                cursor = parsed.Next;
            }

            return result;
        }

        private static bool TryParseAttribute(
            ReadOnlySpan<char> source,
            int cursor,
            out ParsedAttribute parsed)
        {
            var initial = cursor;
            while (cursor < source.Length && IsTagSpace(source[cursor]))
            {
                cursor++;
            }

            if (cursor == initial || cursor >= source.Length || !IsAsciiAlpha(source[cursor]))
            {
                parsed = default;
                return false;
            }

            var nameStart = cursor++;
            while (cursor < source.Length && IsNameCharacter(source[cursor]))
            {
                cursor++;
            }

            var name = source[nameStart..cursor];
            var valueStart = cursor;
            while (cursor < source.Length && IsTagSpace(source[cursor]))
            {
                cursor++;
            }

            if (cursor >= source.Length || source[cursor] != '=')
            {
                parsed = new ParsedAttribute(name.ToString(), null, valueStart);
                return true;
            }

            cursor++;
            while (cursor < source.Length && IsTagSpace(source[cursor]))
            {
                cursor++;
            }

            if (cursor >= source.Length)
            {
                parsed = new ParsedAttribute(name.ToString(), null, valueStart);
                return true;
            }

            if (source[cursor] is '\'' or '"')
            {
                var quote = source[cursor++];
                var start = cursor;
                while (cursor < source.Length && source[cursor] != quote)
                {
                    cursor++;
                }

                if (cursor >= source.Length)
                {
                    parsed = default;
                    return false;
                }

                parsed = new ParsedAttribute(name.ToString(), source[start..cursor].ToString(), cursor + 1);
                return true;
            }

            var unquotedStart = cursor;
            while (cursor < source.Length && !IsTagSpace(source[cursor]) &&
                   source[cursor] is not ('"' or '\'' or '=' or '<' or '>'))
            {
                cursor++;
            }

            if (cursor == unquotedStart)
            {
                parsed = new ParsedAttribute(name.ToString(), null, valueStart);
                return true;
            }

            parsed = new ParsedAttribute(
                name.ToString(),
                source[unquotedStart..cursor].ToString(),
                cursor);
            return true;
        }

        private static string DecodeEntities(string source)
        {
            var builder = new StringBuilder(source.Length);
            AppendDecoded(builder, source.AsSpan());
            return builder.ToString();
        }

        private (int Start, int End)? FindRawEnd(int cursor, Tag tag)
        {
            var needle = "</" + TagNames[(int)tag] + ">";
            while (true)
            {
                var start = _source.IndexOf("</", cursor, StringComparison.Ordinal);
                if (start < 0)
                {
                    return null;
                }

                if (needle.Length <= _source.Length - start &&
                    EqualsAsciiIgnoreCase(_source.AsSpan(start, needle.Length), needle))
                {
                    return (start, start + needle.Length);
                }

                cursor = start + 2;
            }
        }

        private readonly record struct ParsedTag(Tag Tag, bool Closing, ReadOnlyMemory<char> Attributes, int End);

        private readonly record struct ParsedAttribute(string Name, string? Value, int Next);

        private readonly record struct NodeMeta(int PreviousOpen, int StackSlot, int LastChild, Tag Tag);
    }
}
