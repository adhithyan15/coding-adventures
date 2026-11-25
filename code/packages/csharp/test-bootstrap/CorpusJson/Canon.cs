using System.Collections;
using System.Globalization;
using System.Numerics;
using System.Runtime.CompilerServices;
using System.Text;

namespace CodingAdventures.TestBootstrap.CorpusJson;

/// <summary>
/// Renders a witness as stable text, for failure messages and the CI digest.
/// </summary>
/// <remarks>
/// It is <em>total</em>: it never throws, never recurses without bound and never reads
/// the whole of a hostile value. Nesting deeper than 16 renders as <c>...</c>, a witness
/// with more than 10,000 nodes truncates with <c>...</c>, a value whose
/// <c>ToString()</c> throws renders as its type name, and a hostile enumerator,
/// dictionary or <c>ITuple</c> degrades the whole witness to the constant
/// <see cref="Unrenderable"/>. Only <see cref="OutOfMemoryException"/> escapes. This
/// matters because redacted mode computes the digest from the rendering: a custom
/// generator can build arbitrarily deep values, and a runner that crashes on one never
/// returns a verdict. (A .NET stack overflow cannot be caught, hence the explicit depth cap.)
/// <code>
///   Render(true)                  -> true
///   Render(BigInteger(-12))       -> -12
///   Render(new byte[] {0, 255})   -> 0x00ff
///   Render(new List&lt;int&gt; {1,2})   -> [1,2]
///   Render(new Pair(1, 2))        -> {1,2}          (ITuple renders as {fields})
///   Render(null)                  -> nil
/// </code>
/// </remarks>
public static class Canon
{
    /// <summary>Container nesting bound.</summary>
    public const int MaxDepth = 16;

    /// <summary>Total values rendered.</summary>
    public const int MaxNodes = 10_000;

    /// <summary>Largest integer (in bits) rendered as digits.</summary>
    public const int MaxBigIntBits = 100_000;

    /// <summary>What a witness renders as when rendering itself throws.</summary>
    public const string Unrenderable = "<unrenderable>";

    /// <summary>Total rendered characters; the output is cut with "..." beyond it (bounds the digest preimage too).</summary>
    public const int MaxOutput = 1 << 20;

    private const int MaxToString = 200;

    // A witness string is clipped: a 10 MiB string must not become a 10 MiB message.
    private const int MaxStringChars = 10_000;

    /// <summary>Stable text for <paramref name="v"/>. Never throws (except <see cref="OutOfMemoryException"/>).</summary>
    public static string Render(object? v)
    {
        StringBuilder b = new();
        int[] nodes = { 0 };
        try
        {
            RenderInto(b, v, 0, nodes);
            if (b.Length > MaxOutput)
            {
                b.Length = MaxOutput;
                b.Append("...");
            }
        }
        catch (Exception e) when (Safe.Recoverable(e))
        {
            // A hostile enumerator, dictionary or indexer: the contract is totality, so
            // the whole witness degrades to a constant.
            return Unrenderable;
        }

        return b.ToString();
    }

    private static void RenderInto(StringBuilder b, object? v, int depth, int[] nodes)
    {
        if (b.Length > MaxOutput)
        {
            return; // already over the output cap: append nothing more
        }

        nodes[0]++;
        if (nodes[0] > MaxNodes)
        {
            b.Append("...");
            return;
        }

        switch (v)
        {
            case null:
                b.Append("nil");
                break;
            case bool flag:
                b.Append(flag ? "true" : "false");
                break;
            case BigInteger big:
                RenderBig(b, big);
                break;
            case sbyte or byte or short or ushort or int or uint or long or ulong or Int128 or UInt128:
                b.Append(((IFormattable)v).ToString(null, CultureInfo.InvariantCulture));
                break;
            case double d:
                b.Append(d.ToString("R", CultureInfo.InvariantCulture));
                break;
            case float f:
                b.Append(f.ToString("R", CultureInfo.InvariantCulture));
                break;
            case decimal m:
                b.Append(m.ToString(CultureInfo.InvariantCulture));
                break;
            case char c:
                Quote(b, c.ToString());
                break;
            case string s:
                Quote(b, Clip.Apply(s, MaxStringChars));
                break;
            case byte[] raw:
                RenderBytes(b, raw);
                break;
            case Array array:
                Sequence(b, ArrayView(array), depth, nodes);
                break;
            case ITuple tuple:
                Tuple(b, tuple, depth, nodes);
                break;
            case IDictionary dict:
                Dictionary(b, dict, depth, nodes);
                break;
            case IEnumerable seq:
                Sequence(b, seq, depth, nodes);
                break;
            default:
                Other(b, v);
                break;
        }
    }

    private static void RenderBig(StringBuilder b, BigInteger big)
    {
        long bits = (long)big.GetBitLength();
        if (bits > MaxBigIntBits)
        {
            // Decimal conversion of a huge value is itself super-linear work.
            b.Append("<bigint ").Append(bits.ToString(CultureInfo.InvariantCulture)).Append(" bits>");
            return;
        }

        b.Append(big.ToString(CultureInfo.InvariantCulture));
    }

    private static void RenderBytes(StringBuilder b, byte[] raw)
    {
        b.Append("0x");
        foreach (byte x in raw)
        {
            b.Append("0123456789abcdef"[x >> 4]).Append("0123456789abcdef"[x & 0xf]);
            if (b.Length > 4 * MaxNodes)
            {
                b.Append("...");
                break;
            }
        }
    }

    private static List<object?> ArrayView(Array array)
    {
        long n = Math.Min(array.LongLength, MaxNodes + 1);
        List<object?> output = new((int)Math.Min(n, 1024));
        int taken = 0;
        foreach (object? item in array)
        {
            if (taken++ >= n)
            {
                break;
            }

            output.Add(item);
        }

        return output;
    }

    private static void Sequence(StringBuilder b, IEnumerable items, int depth, int[] nodes)
    {
        if (depth >= MaxDepth)
        {
            b.Append("...");
            return;
        }

        b.Append('[');
        bool first = true;
        foreach (object? item in items)
        {
            if (!first)
            {
                b.Append(',');
            }

            first = false;
            if (nodes[0] >= MaxNodes)
            {
                // Out of budget: stop, do not walk the rest of a possibly infinite sequence.
                b.Append("...");
                break;
            }

            RenderInto(b, item, depth + 1, nodes);
        }

        b.Append(']');
    }

    private static void Tuple(StringBuilder b, ITuple tuple, int depth, int[] nodes)
    {
        if (depth >= MaxDepth)
        {
            b.Append("...");
            return;
        }

        b.Append('{');
        int length = Math.Min(tuple.Length, MaxNodes);
        for (int i = 0; i < length; i++)
        {
            if (i > 0)
            {
                b.Append(',');
            }

            if (nodes[0] >= MaxNodes)
            {
                b.Append("...");
                break;
            }

            RenderInto(b, tuple[i], depth + 1, nodes);
        }

        b.Append('}');
    }

    private static void Dictionary(StringBuilder b, IDictionary dict, int depth, int[] nodes)
    {
        if (depth >= MaxDepth)
        {
            b.Append("...");
            return;
        }

        List<(string Key, string Value)> entries = new();
        long accumulated = 0;
        IDictionaryEnumerator e = dict.GetEnumerator();
        while (e.MoveNext())
        {
            StringBuilder kb = new();
            StringBuilder vb = new();
            RenderInto(kb, e.Key, depth + 1, nodes);
            RenderInto(vb, e.Value, depth + 1, nodes);
            entries.Add((kb.ToString(), vb.ToString()));
            accumulated += kb.Length + vb.Length;
            if (nodes[0] >= MaxNodes || accumulated > MaxOutput)
            {
                break;
            }
        }

        entries.Sort((x, y) => string.CompareOrdinal(x.Key, y.Key));
        b.Append('{');
        for (int i = 0; i < entries.Count; i++)
        {
            if (i > 0)
            {
                b.Append(',');
            }

            b.Append(entries[i].Key).Append(':').Append(entries[i].Value);
        }

        b.Append('}');
    }

    private static void Other(StringBuilder b, object v)
    {
        // This guard only protects against Canon's OWN recursion having used up the stack: with
        // little stack left we do not call ToString at all. It cannot protect against a user
        // ToString that recurses by itself (a linked structure printing its tail): .NET cannot
        // catch a StackOverflowException and that would end the process. Witness types
        // therefore MUST have a bounded, shallow ToString.
        if (!RuntimeHelpers.TryEnsureSufficientExecutionStack())
        {
            b.Append(Unrenderable);
            return;
        }

        string text;
        try
        {
            text = v.ToString() ?? string.Empty;
        }
        catch (Exception e) when (Safe.Recoverable(e))
        {
            text = "<" + v.GetType().FullName + ">";
        }

        Quote(b, Clip.Apply(text, MaxToString));
    }

    /// <summary>C0 and C1 controls (<see cref="char.IsControl(char)"/>) plus the line/paragraph separators.</summary>
    internal static bool IsNonPrinting(char c)
    {
        return char.IsControl(c) || c == '\u2028' || c == '\u2029';
    }

    private static void Quote(StringBuilder b, string s)
    {
        b.Append('"');
        foreach (char c in s)
        {
            switch (c)
            {
                case '"':
                    b.Append("\\\"");
                    break;
                case '\\':
                    b.Append("\\\\");
                    break;
                case '\n':
                    b.Append("\\n");
                    break;
                case '\r':
                    b.Append("\\r");
                    break;
                case '\t':
                    b.Append("\\t");
                    break;
                default:
                    if (IsNonPrinting(c))
                    {
                        b.Append("\\u").Append(((int)c).ToString("x4", CultureInfo.InvariantCulture));
                    }
                    else
                    {
                        b.Append(c);
                    }

                    break;
            }
        }

        b.Append('"');
    }
}
