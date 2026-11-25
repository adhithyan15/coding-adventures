using System.Globalization;
using System.Text;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Fuzz;

/// <summary>
/// Crash signatures, frame normalisation and the one public failure line (VER06 sections 5.2 and 6,
/// VER06h sections 5 and 7).
/// </summary>
public static class FuzzSignature
{
    /// <summary>Caps the target id in a TAP line, in UTF-8 bytes.</summary>
    internal const int MaxTargetLen = 128;

    /// <summary>The innermost frames kept in a signature.</summary>
    internal const int MaxFrames = 64;

    /// <summary>Caps a crash kind.</summary>
    internal const int MaxKindLen = 64;

    /// <summary>Caps one normalised frame.</summary>
    internal const int MaxFrameLen = 128;

    /// <summary>
    /// The low 32 bits of <c>fnv1a64(kind ":" frames joined by ":")</c>, as 8 lowercase hex digits.
    /// </summary>
    /// <remarks>
    /// Frames are NOT escaped (VER06 section 9.4 item 6): one frame <c>"x:y"</c> and two frames
    /// <c>"x", "y"</c> hash alike. This lane never produces a frame with a colon
    /// (<see cref="NormaliseFrame"/>), so the ambiguity cannot arise from .NET targets.
    /// </remarks>
    public static string CrashSignature(string kind, IEnumerable<string> frames)
    {
        string raw = kind + ":" + string.Join(":", frames);
        uint low = (uint)(SplitMix64.Fnv1a64(raw) & 0xFFFFFFFFUL);
        return low.ToString("x8", CultureInfo.InvariantCulture);
    }

    /// <summary>Whether two crash reports share one signature.</summary>
    public static bool IsDuplicateCrash(string kindA, IEnumerable<string> framesA, string kindB, IEnumerable<string> framesB)
    {
        return CrashSignature(kindA, framesA) == CrashSignature(kindB, framesB);
    }

    // ---------------------------------------------------------------------------
    // TAP: the only public failure line
    // ---------------------------------------------------------------------------

    private static bool IsIdByte(int c)
    {
        return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '.' || c == '_' || c == '/' || c == '-';
    }

    private static bool IsHexChar(char c) => (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f');

    /// <summary>
    /// Reduces text to the characters its grammar allows; every other UTF-8 BYTE becomes <c>_</c>.
    /// Without this a target id holding a newline would let a caller forge a second TAP line inside
    /// "an opaque token". Multi-byte characters are replaced byte for byte, so the result is pure
    /// ASCII. At most <paramref name="limit"/> bytes are produced, and only the first
    /// <paramref name="limit"/> characters of the input are encoded (every character is at least one
    /// byte), so a hostile megabyte-long id costs O(limit).
    /// </summary>
    private static string Token(string s, int limit, bool extra)
    {
        string head = s.Length <= limit ? s : s.Substring(0, limit);
        byte[] bytes = Encoding.UTF8.GetBytes(head);
        int n = Math.Min(bytes.Length, limit);
        StringBuilder b = new(n);
        for (int i = 0; i < n; i++)
        {
            int c = bytes[i];
            bool ok = IsIdByte(c) || (extra && (c == '<' || c == '>' || c == '$'));
            b.Append(ok ? (char)c : '_');
        }

        return b.ToString();
    }

    /// <summary>
    /// The only public failure line:
    /// <code>
    ///   not ok N - target [failure-id:hex8]
    /// </code>
    /// It carries no input bytes, seed, message or stack trace (VER00 7.3). <paramref name="failureId"/>
    /// must be one of the five registered ids and <paramref name="sigHash"/> exactly 8 lowercase hex
    /// digits, else <see cref="ArgumentException"/>. The target is reduced to
    /// <c>[A-Za-z0-9._/-]</c> (every other byte becoming <c>_</c>) and truncated to 128 bytes.
    /// Well-formed arguments pass through unchanged.
    /// </summary>
    public static string FormatTapFailure(int testNum, string target, string failureId, string sigHash)
    {
        if (!FuzzFailureIds.IsFuzzFailureId(failureId))
        {
            throw new ArgumentException("failure id is not one of the registered fuzz failure ids", nameof(failureId));
        }

        if (sigHash == null || sigHash.Length != 8 || !sigHash.All(IsHexChar))
        {
            throw new ArgumentException("sig hash must be exactly 8 lowercase hex digits", nameof(sigHash));
        }

        string safeTarget = Token(target ?? string.Empty, MaxTargetLen, false);
        return "not ok " + testNum.ToString(CultureInfo.InvariantCulture) + " - " + safeTarget + " [" + failureId + ":" + sigHash + "]";
    }

    /// <summary>
    /// Renders a report as public, opaque failure lines (none for a clean one): one per distinct
    /// crash signature, then one for timeouts and one for OOMs. A timeout or OOM has no stack to
    /// hash, so its sig hash is the signature of the bare kind.
    /// </summary>
    public static IReadOnlyList<string> TapFailures(FuzzReport r, string target, int firstTest)
    {
        List<string> lines = new();

        // A hand-built record cannot exist (CrashRecord's constructor is internal), but the
        // fail-closed fallback stays: a malformed signature still yields a line.
        string invalidSig = CrashSignature("invalid", Array.Empty<string>());
        foreach (CrashRecord rec in r.Crashes)
        {
            lines.Add(TapLine(firstTest + lines.Count, target, FuzzFailureIds.Crash, rec.Signature, invalidSig));
        }

        if (r.Timeouts > 0)
        {
            lines.Add(TapLine(firstTest + lines.Count, target, FuzzFailureIds.Timeout, CrashSignature("timeout", Array.Empty<string>()), invalidSig));
        }

        if (r.Ooms > 0)
        {
            lines.Add(TapLine(firstTest + lines.Count, target, FuzzFailureIds.Oom, CrashSignature("oom", Array.Empty<string>()), invalidSig));
        }

        return lines;
    }

    private static string TapLine(int n, string target, string id, string sig, string fallbackSig)
    {
        try
        {
            return FormatTapFailure(n, target, id, sig);
        }
        catch (ArgumentException)
        {
            return FormatTapFailure(n, target, id, fallbackSig);
        }
    }

    // ---------------------------------------------------------------------------
    // Frame normalisation (VER06h section 5)
    // ---------------------------------------------------------------------------

    private static string SafeName(string s) => Token(s, MaxFrameLen, true);

    private static bool IsGenerated(string name) => name.StartsWith('<');

    /// <summary>The user-facing name inside a compiler-generated member name, or null.</summary>
    private static string? GeneratedOwner(string generated)
    {
        // "<Run>b__3_0" (lambda), "<Run>d__5" (iterator / async), "<Outer>g__Inner|0_0" (local function).
        int close = generated.IndexOf('>');
        if (!generated.StartsWith('<') || close <= 1)
        {
            return null; // "<>c", "<>c__DisplayClass1_0": no owner name inside
        }

        string owner = generated.Substring(1, close - 1);
        int g = generated.IndexOf(">g__", StringComparison.Ordinal);
        if (g >= 0)
        {
            int bar = generated.IndexOf('|', g);
            string local = bar > g ? generated.Substring(g + 4, bar - g - 4) : generated.Substring(g + 4);
            return owner + "." + local;
        }

        return owner;
    }

    /// <summary>
    /// Turns one stack frame into <c>Type.Method</c>. <paramref name="typeSegments"/> is the declaring
    /// type's nesting chain, outermost first, WITHOUT the namespace:
    /// <code>
    ///   [Parser, Inner] + Read                      -> Parser$Inner.Read
    ///   [App, &lt;&gt;c__DisplayClass1_0] + &lt;Run&gt;b__0  -> App.Run
    ///   [App, &lt;Run&gt;d__5] + MoveNext                -> App.Run
    ///   [Box`1] + Get                               -> Box_1.Get
    /// </code>
    /// Compiler-generated segments (closures, state machines) are dropped and the member they came
    /// from is reported. The result contains no <c>:</c> (every character outside
    /// <c>[A-Za-z0-9._&lt;&gt;$/-]</c> becomes <c>_</c>).
    /// </summary>
    internal static string NormaliseFrame(IReadOnlyList<string> typeSegments, string method)
    {
        List<string> segs = new(typeSegments);
        string name = method;
        string? ownerFromType = null;
        while (segs.Count > 0 && IsGenerated(segs[^1]))
        {
            ownerFromType ??= GeneratedOwner(segs[^1]);
            segs.RemoveAt(segs.Count - 1);
        }

        if (IsGenerated(name))
        {
            name = GeneratedOwner(name) ?? ownerFromType ?? name;
        }
        else if (name == "MoveNext" && ownerFromType != null)
        {
            name = ownerFromType;
        }

        string type = segs.Count == 0 ? "_" : string.Join("$", segs);
        return SafeName(type + "." + name);
    }

    /// <summary>Turns a type's name into a signature kind: sanitised, at most 64 characters.</summary>
    internal static string NormaliseKind(string typeName)
    {
        return Token(typeName, MaxKindLen, true);
    }

    /// <summary>
    /// Removes consecutive duplicate frames, so a recursive crash hashes alike at any depth, and keeps
    /// the innermost <see cref="MaxFrames"/>.
    /// </summary>
    internal static List<string> Collapse(IEnumerable<string> frames)
    {
        List<string> output = new();
        foreach (string f in frames)
        {
            if (output.Count == 0 || output[^1] != f)
            {
                output.Add(f);
                if (output.Count >= MaxFrames)
                {
                    break;
                }
            }
        }

        return output;
    }
}
