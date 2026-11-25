using System.Text;

namespace CodingAdventures.TestBootstrap.Tfv;

/// <summary>Every TFV rejection.</summary>
public sealed class InvalidTfvException : Exception
{
    /// <summary>Creates the exception; the message is prefixed with "invalid TFV: ".</summary>
    public InvalidTfvException(string message)
        : base("invalid TFV: " + message)
    {
    }

    internal string Detail => Message.Substring("invalid TFV: ".Length);
}

/// <summary>
/// Validates TFV v1 text, the line format that carries test corpora between languages
/// (TF01 section 2).
/// </summary>
/// <remarks>
/// This is a <em>validator</em>, not a reader: its one job in this package is to let the
/// corpus runner check the pbt-v1 <c>corpus_reject</c> cases with a real parser instead
/// of skipping them.
/// <h2>The format in one screen</h2>
/// <code>
///   tfv 1
///   # comment
///   &lt;id&gt; R&lt;n&gt; K&lt;field&gt; &lt;value&gt; K&lt;field&gt; &lt;value&gt; ...
/// </code>
/// A value is one of
/// <code>
///   N  T  F                 null, true, false
///   I&lt;int&gt;                  canonical decimal integer, at most 1000 digits
///   D&lt;16 hex&gt;               IEEE-754 bit pattern, lowercase
///   X&lt;hex&gt;  S&lt;hex&gt;          bytes / UTF-8 text as even-length lowercase hex
///   L&lt;n&gt; v1 ... vn          list
///   M&lt;n&gt; k1 v1 ... kn vn    map; no two keys deep-equal
/// </code>
/// Tokens are separated by exactly one ASCII space. Anything else is rejected: uppercase
/// or odd-length hex, <c>I-0</c>, <c>I007</c>, a count that disagrees with the tokens,
/// trailing tokens, doubled spaces, a duplicate field or record id, invalid UTF-8 inside
/// S, a carriage return in a record line, nesting deeper than 64 containers, and integers
/// of more than 1000 digits.
/// <para>
/// Worked example. <c>"tfv 1\nr R1 Kop S706274 Kdata XFF\n"</c> fails: XFF is uppercase hex
/// (and odd length). <c>"bad header\n"</c> fails: the first line must be exactly <c>tfv 1</c>.
/// </para>
/// </remarks>
public static class TfvValidator
{
    /// <summary>The exact first line of every TFV file.</summary>
    public const string Header = "tfv 1";

    /// <summary>Deepest container nesting accepted (outermost is 1).</summary>
    public const int MaxDepth = 64;

    /// <summary>Digits of an integer token (sign excluded).</summary>
    public const int MaxIntDigits = 1000;

    /// <summary>Chars of a document.</summary>
    public const int MaxInput = 1 << 20;

    private static string Q(string s)
    {
        const int limit = 64;
        string shown = s.Length > limit ? s.Substring(0, limit) + "..." : s;
        StringBuilder b = new("\"");
        foreach (char c in shown)
        {
            if (c == '\\' || c == '"')
            {
                b.Append('\\').Append(c);
            }
            else if (CorpusJson.Canon.IsNonPrinting(c))
            {
                b.Append("\\u").Append(((int)c).ToString("x4", System.Globalization.CultureInfo.InvariantCulture));
            }
            else
            {
                b.Append(c);
            }
        }

        return b.Append('"').ToString();
    }

    /// <summary>True when <paramref name="text"/> is a well-formed TFV v1 document. Never throws.</summary>
    public static bool IsValid(string text)
    {
        try
        {
            Validate(text);
            return true;
        }
        catch (InvalidTfvException)
        {
            return false;
        }
    }

    /// <summary>
    /// Returns normally when <paramref name="text"/> is a well-formed TFV v1 document and
    /// throws <see cref="InvalidTfvException"/> otherwise. Reserves no memory from a count
    /// in the input.
    /// </summary>
    public static void Validate(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        if (text.Length > MaxInput)
        {
            throw new InvalidTfvException("input of " + text.Length + " chars exceeds " + MaxInput);
        }

        try
        {
            // A lone surrogate has no UTF-8 encoding.
            new UTF8Encoding(false, true).GetByteCount(text);
        }
        catch (EncoderFallbackException)
        {
            throw new InvalidTfvException("document is not valid UTF-8");
        }

        string[] lines = text.Split('\n');
        int count = lines.Length;
        if (lines[count - 1].Length == 0)
        {
            count--; // the final newline is optional
        }

        if (count == 0 || lines[0] != Header)
        {
            throw new InvalidTfvException("the first line must be exactly " + Q(Header));
        }

        HashSet<string> seenIds = new();
        for (int n = 1; n < count; n++)
        {
            string line = lines[n];
            if (line.Length == 0 || line[0] == '#')
            {
                continue; // blank and comment lines are skipped whole
            }

            string id;
            try
            {
                id = ValidateRecord(line);
            }
            catch (InvalidTfvException e)
            {
                throw new InvalidTfvException("line " + (n + 1) + ": " + e.Detail);
            }

            if (!seenIds.Add(id))
            {
                throw new InvalidTfvException("line " + (n + 1) + ": duplicate record id " + Q(id));
            }
        }
    }

    private static bool IsIdChar(char c)
    {
        return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')
            || c == '.' || c == '_' || c == '/' || c == '-';
    }

    // The caller has already split the line on single spaces and rejected empty tokens,
    // so an id is never empty here.
    private static bool ValidId(string s)
    {
        foreach (char c in s)
        {
            if (!IsIdChar(c))
            {
                return false;
            }
        }

        return true;
    }

    private static bool ValidFieldName(string s)
    {
        if (s.Length == 0 || s[0] < 'a' || s[0] > 'z')
        {
            return false;
        }

        for (int i = 1; i < s.Length; i++)
        {
            char c = s[i];
            if (!((c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '_'))
            {
                return false;
            }
        }

        return true;
    }

    private sealed class Reader
    {
        private readonly string[] tokens;

        internal Reader(string[] tokens, int start)
        {
            this.tokens = tokens;
            Pos = start;
        }

        internal int Pos { get; private set; }

        internal string Next()
        {
            if (Pos >= tokens.Length)
            {
                throw new InvalidTfvException("record ends early");
            }

            return tokens[Pos++];
        }

        // Returns a deep-equality key: equal for two values exactly when TF01 section 3.1
        // calls them deep-equal. depth is the number of enclosing containers.
        internal string Value(int depth)
        {
            string tok = Next();
            char tag = tok[0];
            string body = tok.Substring(1);
            switch (tag)
            {
                case 'N':
                case 'T':
                case 'F':
                    if (body.Length != 0)
                    {
                        throw new InvalidTfvException("unexpected body in " + Q(tok));
                    }

                    return tag.ToString();
                case 'I':
                    ValidInt(body);
                    return tok;
                case 'D':
                    {
                        byte[]? raw = LowerHexBytes(body);
                        if (body.Length != 16 || raw == null)
                        {
                            throw new InvalidTfvException("float needs exactly 16 lowercase hex digits: " + Q(tok));
                        }

                        // Every NaN equals every NaN; -0 and +0 stay distinct.
                        int exp = ((raw[0] & 0x7f) << 4) | ((raw[1] & 0xff) >> 4);
                        bool mantissaZero = (raw[1] & 0x0f) == 0;
                        for (int i = 2; i < raw.Length; i++)
                        {
                            mantissaZero = mantissaZero && raw[i] == 0;
                        }

                        return exp == 0x7ff && !mantissaZero ? "Dnan" : tok;
                    }

                case 'X':
                    if (LowerHexBytes(body) == null)
                    {
                        throw new InvalidTfvException("bytes need even-length lowercase hex: " + Q(tok));
                    }

                    return tok;
                case 'S':
                    {
                        byte[]? raw = LowerHexBytes(body);
                        if (raw == null)
                        {
                            throw new InvalidTfvException("text needs even-length lowercase hex: " + Q(tok));
                        }

                        try
                        {
                            new UTF8Encoding(false, true).GetString(raw);
                        }
                        catch (DecoderFallbackException)
                        {
                            throw new InvalidTfvException("text is not strict UTF-8: " + Q(tok));
                        }

                        return tok;
                    }

                case 'L':
                case 'M':
                    return Container(tag, body, depth);
                default:
                    throw new InvalidTfvException("unknown tag in " + Q(tok));
            }
        }

        private string Container(char tag, string body, int depth)
        {
            if (depth + 1 > MaxDepth)
            {
                throw new InvalidTfvException("nesting deeper than " + MaxDepth);
            }

            int count = CanonicalNat(body);
            if (tag == 'L')
            {
                StringBuilder b = new("L[");
                for (int i = 0; i < count; i++)
                {
                    b.Append(Value(depth + 1)).Append(',');
                }

                return b.Append(']').ToString();
            }

            HashSet<string> keys = new();
            List<string> entries = new();
            for (int i = 0; i < count; i++)
            {
                string k = Value(depth + 1);
                string v = Value(depth + 1);
                if (!keys.Add(k))
                {
                    throw new InvalidTfvException("duplicate map key");
                }

                entries.Add("(" + k + "=>" + v + ")");
            }

            entries.Sort(string.CompareOrdinal);
            return "M{" + string.Concat(entries) + "}";
        }
    }

    private static string ValidateRecord(string line)
    {
        if (line.Contains('\r'))
        {
            throw new InvalidTfvException("carriage return in a record line");
        }

        string[] tokens = line.Split(' ');
        foreach (string t in tokens)
        {
            if (t.Length == 0)
            {
                throw new InvalidTfvException("leading, trailing or doubled space");
            }
        }

        if (tokens.Length < 2)
        {
            throw new InvalidTfvException("a record needs an id and R<n>");
        }

        string id = tokens[0];
        if (!ValidId(id))
        {
            throw new InvalidTfvException("bad record id " + Q(id));
        }

        if (tokens[1][0] != 'R')
        {
            throw new InvalidTfvException("expected R<n>, got " + Q(tokens[1]));
        }

        int count = CanonicalNat(tokens[1].Substring(1));
        Reader r = new(tokens, 2);
        HashSet<string> seen = new();
        for (int i = 0; i < count; i++)
        {
            string keyTok = r.Next();
            if (keyTok[0] != 'K' || !ValidFieldName(keyTok.Substring(1)))
            {
                throw new InvalidTfvException("bad field key " + Q(keyTok));
            }

            if (!seen.Add(keyTok))
            {
                throw new InvalidTfvException("duplicate field " + Q(keyTok.Substring(1)));
            }

            r.Value(0);
        }

        if (r.Pos != tokens.Length)
        {
            throw new InvalidTfvException("trailing tokens after the last field");
        }

        return id;
    }

    // 0 or digits without a leading zero. A count too large for the tokens present is
    // rejected later as "record ends early", and no allocation depends on it.
    private static int CanonicalNat(string s)
    {
        if (s.Length == 0)
        {
            throw new InvalidTfvException("empty count");
        }

        foreach (char c in s)
        {
            if (c < '0' || c > '9')
            {
                throw new InvalidTfvException("non-digit in count " + Q(s));
            }
        }

        if (s.Length > 1 && s[0] == '0')
        {
            throw new InvalidTfvException("leading zero in count " + Q(s));
        }

        if (s.Length > 9)
        {
            throw new InvalidTfvException("count " + Q(s) + " too large");
        }

        return int.Parse(s, System.Globalization.CultureInfo.InvariantCulture);
    }

    private static byte[]? LowerHexBytes(string s)
    {
        if (s.Length % 2 != 0)
        {
            return null;
        }

        byte[] output = new byte[s.Length / 2];
        for (int i = 0; i < s.Length; i += 2)
        {
            int hi = HexVal(s[i]);
            int lo = HexVal(s[i + 1]);
            if (hi < 0 || lo < 0)
            {
                return null;
            }

            output[i / 2] = (byte)((hi << 4) | lo);
        }

        return output;
    }

    private static int HexVal(char c)
    {
        if (c >= '0' && c <= '9')
        {
            return c - '0';
        }

        if (c >= 'a' && c <= 'f')
        {
            return c - 'a' + 10;
        }

        return -1;
    }

    private static void ValidInt(string body)
    {
        bool neg = body.StartsWith('-');
        string digits = neg ? body.Substring(1) : body;
        if (digits.Length == 0)
        {
            throw new InvalidTfvException("empty integer");
        }

        foreach (char c in digits)
        {
            if (c < '0' || c > '9')
            {
                throw new InvalidTfvException("non-digit in integer " + Q(body));
            }
        }

        if (digits[0] == '0' && (digits.Length > 1 || neg))
        {
            throw new InvalidTfvException("non-canonical integer " + Q(body));
        }

        if (digits.Length > MaxIntDigits)
        {
            throw new InvalidTfvException("integer of " + digits.Length + " digits exceeds " + MaxIntDigits);
        }
    }
}
