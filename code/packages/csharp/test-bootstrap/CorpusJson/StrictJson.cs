using System.Text;

namespace CodingAdventures.TestBootstrap.CorpusJson;

/// <summary>A JSON number kept as its literal text (a 30-digit integer must not lose its low digits).</summary>
public sealed record JsonNumber(string Text);

/// <summary>Thrown for any malformed or over-limit JSON document.</summary>
public sealed class JsonException : Exception
{
    /// <summary>Creates the exception.</summary>
    public JsonException(string message)
        : base(message)
    {
    }
}

/// <summary>
/// A small, strict JSON reader (VER05g section 3, VER05e section 6).
/// </summary>
/// <remarks>
/// The BCL has <c>System.Text.Json</c>, but its defaults are lenient in exactly the
/// ways a conformance fixture must not be (it keeps the last duplicate key, for one),
/// so the corpus loader owns its parser. "Strict" is the point: a conformance fixture
/// is <em>data</em>, and a lenient parser quietly papers over the corruption a
/// conformance check exists to notice.
/// <list type="bullet">
/// <item>duplicate object keys are refused;</item>
/// <item>trailing data after the document is refused;</item>
/// <item>nesting deeper than <see cref="MaxDepth"/> is refused (a hostile million-deep
/// list would overflow the stack of a recursive reader, and .NET cannot catch that);</item>
/// <item>documents over <see cref="MaxBytes"/> bytes and over <see cref="MaxNodes"/>
/// values are refused;</item>
/// <item>invalid UTF-8 (and a byte-order mark), raw control characters in strings, leading
/// zeros, <c>+1</c>, <c>NaN</c>, comments and trailing commas are refused.</item>
/// </list>
/// Values become <c>Dictionary&lt;string, object?&gt;</c> (insertion ordered),
/// <c>List&lt;object?&gt;</c>, <c>string</c>, <c>bool</c>, <c>null</c> and
/// <see cref="JsonNumber"/>.
/// </remarks>
public static class StrictJson
{
    /// <summary>JSON nesting limit.</summary>
    public const int MaxDepth = 64;

    /// <summary>
    /// Total values (objects, arrays, strings, numbers, literals) one document may hold.
    /// 8 MiB of <c>0,</c> would otherwise become ~4 million boxed nodes.
    /// </summary>
    public const int MaxNodes = 200_000;

    /// <summary>Document size limit, in bytes.</summary>
    public const int MaxBytes = 8 << 20;

    /// <summary>Parses a UTF-8 document.</summary>
    public static object? Parse(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Length > MaxBytes)
        {
            throw new JsonException(data.Length + " bytes exceeds " + MaxBytes);
        }

        string text;
        try
        {
            // throwOnInvalidBytes: true. GetString does not strip a BOM, so a BOM reaches
            // the reader as U+FEFF and is refused as an unexpected character.
            text = new UTF8Encoding(false, true).GetString(data);
        }
        catch (DecoderFallbackException)
        {
            throw new JsonException("document is not valid UTF-8");
        }

        return new Reader(text).Document();
    }

    private sealed class Reader
    {
        private readonly string s;
        private int pos;
        private int nodes;

        internal Reader(string s)
        {
            this.s = s;
        }

        internal object? Document()
        {
            SkipWs();
            object? v = Value(0);
            SkipWs();
            if (pos != s.Length)
            {
                throw new JsonException("trailing data after the document");
            }

            return v;
        }

        private void SkipWs()
        {
            while (pos < s.Length)
            {
                char c = s[pos];
                if (c == ' ' || c == '\t' || c == '\n' || c == '\r')
                {
                    pos++;
                }
                else
                {
                    break;
                }
            }
        }

        private JsonException Err(string what)
        {
            return new JsonException(what + " at offset " + pos);
        }

        private object? Value(int depth)
        {
            if (++nodes > MaxNodes)
            {
                throw new JsonException("more than " + MaxNodes + " values in one document");
            }

            if (pos >= s.Length)
            {
                throw Err("unexpected end of input");
            }

            char c = s[pos];
            switch (c)
            {
                case '{':
                    return Obj(depth);
                case '[':
                    return Arr(depth);
                case '"':
                    return Str();
                case 't':
                    Literal("true");
                    return true;
                case 'f':
                    Literal("false");
                    return false;
                case 'n':
                    Literal("null");
                    return null;
                default:
                    if (c == '-' || (c >= '0' && c <= '9'))
                    {
                        return Num();
                    }

                    throw Err("unexpected character");
            }
        }

        private void Literal(string word)
        {
            if (string.CompareOrdinal(s, pos, word, 0, word.Length) != 0)
            {
                throw Err("bad literal");
            }

            pos += word.Length;
        }

        // At most MaxDepth nested containers (the outermost is level 1): the 65th is refused
        // before any recursion happens, so the parser's stack use is bounded.
        private void EnterContainer(int depth)
        {
            if (depth >= MaxDepth)
            {
                throw new JsonException("nesting deeper than " + MaxDepth);
            }
        }

        private Dictionary<string, object?> Obj(int depth)
        {
            EnterContainer(depth);
            pos++; // {
            Dictionary<string, object?> output = new();
            SkipWs();
            if (Peek() == '}')
            {
                pos++;
                return output;
            }

            while (true)
            {
                SkipWs();
                if (Peek() != '"')
                {
                    throw Err("object key is not a string");
                }

                string key = Str();
                if (output.ContainsKey(key))
                {
                    throw new JsonException("duplicate JSON key \"" + Clip.Apply(key, 64) + "\"");
                }

                SkipWs();
                if (Peek() != ':')
                {
                    throw Err("expected ':'");
                }

                pos++;
                SkipWs();
                output[key] = Value(depth + 1);
                SkipWs();
                char c = Peek();
                pos++;
                if (c == '}')
                {
                    return output;
                }

                if (c != ',')
                {
                    throw Err("expected ',' or '}'");
                }
            }
        }

        private List<object?> Arr(int depth)
        {
            EnterContainer(depth);
            pos++; // [
            List<object?> output = new();
            SkipWs();
            if (Peek() == ']')
            {
                pos++;
                return output;
            }

            while (true)
            {
                SkipWs();
                output.Add(Value(depth + 1));
                SkipWs();
                char c = Peek();
                pos++;
                if (c == ']')
                {
                    return output;
                }

                if (c != ',')
                {
                    throw Err("expected ',' or ']'");
                }
            }
        }

        private char Peek()
        {
            if (pos >= s.Length)
            {
                throw Err("unexpected end of input");
            }

            return s[pos];
        }

        private string Str()
        {
            pos++; // opening quote
            StringBuilder b = new();
            while (true)
            {
                if (pos >= s.Length)
                {
                    throw Err("unterminated string");
                }

                char c = s[pos++];
                if (c == '"')
                {
                    return b.ToString();
                }

                if (c < 0x20)
                {
                    throw Err("raw control character in string");
                }

                if (c != '\\')
                {
                    b.Append(c);
                    continue;
                }

                if (pos >= s.Length)
                {
                    throw Err("unterminated escape");
                }

                char e = s[pos++];
                switch (e)
                {
                    case '"':
                    case '\\':
                    case '/':
                        b.Append(e);
                        break;
                    case 'b':
                        b.Append('\b');
                        break;
                    case 'f':
                        b.Append('\f');
                        break;
                    case 'n':
                        b.Append('\n');
                        break;
                    case 'r':
                        b.Append('\r');
                        break;
                    case 't':
                        b.Append('\t');
                        break;
                    case 'u':
                        b.Append(UnicodeEscape());
                        break;
                    default:
                        throw Err("bad escape");
                }
            }
        }

        private char UnicodeEscape()
        {
            if (pos + 4 > s.Length)
            {
                throw Err("short \\u escape");
            }

            int code = 0;
            for (int i = 0; i < 4; i++)
            {
                char h = s[pos + i];
                int d = h >= '0' && h <= '9' ? h - '0'
                    : h >= 'a' && h <= 'f' ? h - 'a' + 10
                    : h >= 'A' && h <= 'F' ? h - 'A' + 10
                    : -1;
                if (d < 0)
                {
                    throw Err("bad \\u escape");
                }

                code = (code * 16) + d;
            }

            pos += 4;
            return (char)code;
        }

        private JsonNumber Num()
        {
            int start = pos;
            if (Peek() == '-')
            {
                pos++;
            }

            char c = Peek();
            if (c == '0')
            {
                pos++;
            }
            else if (c >= '1' && c <= '9')
            {
                while (pos < s.Length && s[pos] >= '0' && s[pos] <= '9')
                {
                    pos++;
                }
            }
            else
            {
                throw Err("bad number");
            }

            if (pos < s.Length && s[pos] == '.')
            {
                pos++;
                Digits();
            }

            if (pos < s.Length && (s[pos] == 'e' || s[pos] == 'E'))
            {
                pos++;
                if (pos < s.Length && (s[pos] == '+' || s[pos] == '-'))
                {
                    pos++;
                }

                Digits();
            }

            return new JsonNumber(s.Substring(start, pos - start));
        }

        private void Digits()
        {
            int begin = pos;
            while (pos < s.Length && s[pos] >= '0' && s[pos] <= '9')
            {
                pos++;
            }

            if (pos == begin)
            {
                throw Err("digits expected");
            }
        }
    }
}
