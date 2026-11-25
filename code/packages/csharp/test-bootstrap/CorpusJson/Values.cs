using System.Numerics;

namespace CodingAdventures.TestBootstrap.CorpusJson;

/// <summary>A problem with one case's encoding; its text is already clipped.</summary>
public sealed class CaseException : Exception
{
    /// <summary>Creates the exception with a clipped message.</summary>
    public CaseException(string message)
        : base(Clip.Apply(message, Values.MaxProblemLen))
    {
    }
}

/// <summary>
/// Strict value decoding and comparison for corpus cases (VER05g section 3, VER05e section 6).
/// </summary>
/// <remarks>
/// Case encoding (JSON):
/// <code>
///   {"kind": "int",   "value": "-12"}      an integer (any width, decimal text)
///   {"kind": "bytes", "hex": "00ff"}       a byte string
///   true / false                           a bool
///   [ ... ]                                a list of the above
///   "text"                                 a string (types, property names)
/// </code>
/// Decoded values are <c>bool</c>, <c>string</c>, <see cref="BigInteger"/>, <c>byte[]</c>
/// and <c>List&lt;object?&gt;</c>.
/// </remarks>
public static class Values
{
    /// <summary>Bounds every fixture-controlled size: counts, lengths, list and bytes elements.</summary>
    public const int MaxSize = 10_000;

    /// <summary>Bounds the decimal digits of any integer in a case.</summary>
    public const int MaxIntDigits = 1_000;

    /// <summary>Bounds value nesting so decoding recursion is finite.</summary>
    public const int MaxDepth = 64;

    /// <summary>Bounds one reported problem.</summary>
    public const int MaxProblemLen = 400;

    /// <summary>Bounds one fixture-derived fragment inside a problem.</summary>
    public const int MaxFragmentLen = 200;

    /// <summary>Bounded rendering of a fixture-derived value for a message.</summary>
    public static string Show(object? v)
    {
        return Clip.Apply(Canon.Render(v), MaxFragmentLen);
    }

    /// <summary>
    /// Decimal digits only, a single leading "-" at most, no leading zeros, no "-0",
    /// nothing a lenient parser would also swallow (underscores, spaces, "+").
    /// </summary>
    public static bool IsCanonicalInt(string text)
    {
        string digits = text.StartsWith('-') ? text.Substring(1) : text;
        if (digits.Length == 0)
        {
            return false;
        }

        foreach (char c in digits)
        {
            if (c < '0' || c > '9')
            {
                return false;
            }
        }

        if (digits[0] == '0')
        {
            return text == "0";
        }

        return true;
    }

    /// <summary>Even length, digits and a-f only.</summary>
    public static bool IsLowercaseHex(string text)
    {
        if (text.Length % 2 != 0)
        {
            return false;
        }

        foreach (char c in text)
        {
            if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f')))
            {
                return false;
            }
        }

        return true;
    }

    /// <summary>Turns a parsed JSON node into a strictly decoded value.</summary>
    public static object? Decode(object? node)
    {
        return Decode(node, 0);
    }

    // Runners may be handed a hand-built tree that never went through the parser, so the
    // depth cap lives here too: a deeper input is a problem, not a stack overflow.
    private static object? Decode(object? node, int depth)
    {
        if (depth > MaxDepth)
        {
            throw new CaseException("value nesting deeper than " + MaxDepth);
        }

        switch (node)
        {
            case bool or string:
                return node;
            case IList<object?> list:
                {
                    if (list.Count > MaxSize)
                    {
                        throw new CaseException("list exceeds " + MaxSize + " elements");
                    }

                    List<object?> output = new(list.Count);
                    foreach (object? item in list)
                    {
                        output.Add(Decode(item, depth + 1));
                    }

                    return output;
                }

            case IDictionary<string, object?> map:
                return DecodeMap(map);
            default:
                throw new CaseException("unsupported JSON value " + Show(node));
        }
    }

    private static object DecodeMap(IDictionary<string, object?> map)
    {
        map.TryGetValue("kind", out object? kind);
        if (kind is "int" && map.Count == 2)
        {
            map.TryGetValue("value", out object? text);
            if (text is not string t || !IsCanonicalInt(t))
            {
                throw new CaseException("non-canonical int " + Show(text));
            }

            string digits = t.StartsWith('-') ? t.Substring(1) : t;
            if (digits.Length > MaxIntDigits)
            {
                throw new CaseException("int exceeds " + MaxIntDigits + " digits");
            }

            return BigInteger.Parse(t, System.Globalization.CultureInfo.InvariantCulture);
        }

        if (kind is "bytes" && map.Count == 2)
        {
            map.TryGetValue("hex", out object? text);
            if (text is not string t || !IsLowercaseHex(t))
            {
                throw new CaseException("bytes hex must be even-length lowercase: " + Show(text));
            }

            if (t.Length > 2 * MaxSize)
            {
                throw new CaseException("bytes exceed " + MaxSize + " bytes");
            }

            return Convert.FromHexString(t);
        }

        throw new CaseException("unrecognised value node " + Show(map));
    }

    /// <summary>Equality that does not conflate true with 1, lists with bytes, or an int with its text.</summary>
    public static bool Same(object? a, object? b)
    {
        return Same(a, b, 0);
    }

    private static bool Same(object? a, object? b, int depth)
    {
        if (depth > MaxDepth)
        {
            return false; // Decode caps real inputs; deeper is never "the same"
        }

        switch (a)
        {
            case bool xb:
                return b is bool yb && xb == yb;
            case string xs:
                return b is string ys && xs == ys;
            case BigInteger xi:
                return b is BigInteger yi && xi == yi;
            case byte[] xr:
                return b is byte[] yr && xr.AsSpan().SequenceEqual(yr);
            case IList<object?> xl:
                {
                    if (b is not IList<object?> yl || xl.Count != yl.Count)
                    {
                        return false;
                    }

                    for (int i = 0; i < xl.Count; i++)
                    {
                        if (!Same(xl[i], yl[i], depth + 1))
                        {
                            return false;
                        }
                    }

                    return true;
                }

            default:
                return false;
        }
    }
}
