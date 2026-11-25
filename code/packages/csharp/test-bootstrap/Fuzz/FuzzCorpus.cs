using System.Numerics;
using CodingAdventures.TestBootstrap.CorpusJson;
using static CodingAdventures.TestBootstrap.CorpusJson.Values;

namespace CodingAdventures.TestBootstrap.Fuzz;

/// <summary>How many cases were fully evaluated (match or not) and every problem found.</summary>
public sealed record FuzzCorpusReport(int Evaluated, IReadOnlyList<string> Problems);

/// <summary>
/// The production runner for the shared fuzz-v1 corpus
/// (<c>code/specs/fixtures/fuzz-v1/cases.json</c>, VER06 section 8, VER06h section 8).
/// </summary>
/// <remarks>
/// It takes the raw JSON, strictly decodes it, evaluates every case with this package's own public
/// engine functions and returns human-readable problems. An empty list means "conforms". It does no
/// file I/O (the package is capability-free); the tests read the committed fixture.
/// <para>A runner that cannot report a problem is worse than none, so this one is strict:</para>
/// <list type="bullet">
/// <item>a case has exactly the fields of its op, in canonical encodings (ints are decimal strings,
/// bytes are even-length lowercase hex);</item>
/// <item>an unknown op or a malformed value is a PROBLEM, never a silent skip;</item>
/// <item>every op appears, a failure id is accepted and one rejected, and the case count equals
/// <see cref="Fuzzv1CaseCount"/>, so deleting cases is itself a problem;</item>
/// <item>fixture-controlled sizes are capped (CWE-400): a hostile <c>steps = 10^9</c> is a reported
/// problem, not a hang;</item>
/// <item>every problem about a case starts with that case's id.</item>
/// </list>
/// </remarks>
public static class FuzzCorpus
{
    /// <summary>How many cases fuzz-v1 holds; a different count is a problem.</summary>
    public const int Fuzzv1CaseCount = 25;

    private const int MaxCases = 10_000;

    /// <summary>
    /// Bounds the aggregate mutate_sequence work of one corpus: the bytes ACTUALLY copied, summed over
    /// cases. Each case is already capped (10,000 steps, 10,000 bytes) but 10,000 such cases would
    /// still be 10^12 byte-steps (CWE-400).
    /// </summary>
    internal const long MaxAggregateSequenceWork = 1L << 24;

    /// <summary>
    /// Strictly decodes a fuzz-v1 document into its case list. The document must be exactly
    /// <c>{"schema_version": 1, "cases": [...]}</c>. Duplicate object keys, trailing data, nesting
    /// deeper than 64 and documents over 8 MiB are refused with <see cref="JsonException"/>. Cases are
    /// returned unvalidated.
    /// </summary>
    public static List<object?> Parse(byte[] data)
    {
        object? root = StrictJson.Parse(data);
        if (root is not Dictionary<string, object?> doc || doc.Count != 2)
        {
            throw new JsonException("fuzz corpus: want exactly {\"schema_version\": 1, \"cases\": [...]}");
        }

        if (!doc.TryGetValue("schema_version", out object? version) || version is not JsonNumber { Text: "1" })
        {
            throw new JsonException("fuzz corpus: schema_version must be 1");
        }

        if (!doc.TryGetValue("cases", out object? cases) || cases is not List<object?> list)
        {
            throw new JsonException("fuzz corpus: \"cases\" must be an array");
        }

        return new List<object?>(list);
    }

    /// <summary>The closed field set of each op (besides id and op).</summary>
    internal static readonly Dictionary<string, string[]> Shapes = new()
    {
        ["mutate_bit_flip"] = new[] { "input", "bit_offset", "count", "output" },
        ["mutate_byte_flip"] = new[] { "input", "byte_offset", "count", "output" },
        ["mutate_arithmetic"] = new[] { "input", "byte_offset", "delta", "width", "endian", "output" },
        ["mutate_boundary"] = new[] { "input", "byte_offset", "width", "val", "endian", "output" },
        ["mutate_block_delete"] = new[] { "input", "offset", "length", "output" },
        ["mutate_block_insert"] = new[] { "input", "offset", "insert_bytes", "output" },
        ["mutate_block_duplicate"] = new[] { "input", "src_offset", "length", "dst_offset", "output" },
        ["mutate_splice"] = new[] { "target", "donor", "target_split", "donor_split", "output" },
        ["mutate_sequence"] = new[] { "input", "seed", "steps", "output" },
        ["corpus_admit"] = new[] { "existing_edges", "trace_edges", "admitted", "new_edges" },
        ["crash_dedup"] = new[] { "crash_a_kind", "crash_a_frames", "crash_b_kind", "crash_b_frames", "is_duplicate" },
        ["failure_id_valid"] = new[] { "input", "valid" },
        ["private_reporting"] = new[] { "target", "failure_id", "raw_payload", "sig_hash", "tap_line", "contains_raw_payload" },
        ["corpus_reject"] = new[] { "text" },
    };

    private static bool FieldSetMatches(Dictionary<string, object?> c, string op)
    {
        string[] fields = Shapes[op];
        if (c.Count != fields.Length + 2 || !c.ContainsKey("id") || !c.ContainsKey("op"))
        {
            return false;
        }

        return fields.All(c.ContainsKey);
    }

    /// <summary><see cref="Evaluate(IReadOnlyList{object?}, int)"/> with the fuzz-v1 case count.</summary>
    public static FuzzCorpusReport Evaluate(IReadOnlyList<object?> cases) => Evaluate(cases, Fuzzv1CaseCount);

    /// <summary>
    /// Evaluates every case and returns the problems (none means the engine conforms). A negative
    /// <paramref name="expected"/> skips the case-count check (for ad-hoc case lists in tests).
    /// </summary>
    public static FuzzCorpusReport Evaluate(IReadOnlyList<object?> casesIn, int expected)
    {
        List<string> problems = new();
        int evaluated = 0;
        IReadOnlyList<object?> cases = casesIn;
        if (expected >= 0 && cases.Count != expected)
        {
            problems.Add("fuzz-v1: expected " + expected + " cases, got " + cases.Count);
        }

        if (cases.Count > MaxCases)
        {
            problems.Add("fuzz-v1: " + cases.Count + " cases exceeds the cap of " + MaxCases);
            cases = cases.Take(MaxCases).ToList();
        }

        // Ids are compared as strings only: a non-string id is a reported problem.
        HashSet<string> seenIds = new();
        foreach (object? raw in cases)
        {
            string? id = raw is Dictionary<string, object?> m && m.TryGetValue("id", out object? v) && v is string s ? s : null;
            if (string.IsNullOrEmpty(id) || !seenIds.Add(id))
            {
                problems.Add("duplicate or missing case id");
                break;
            }
        }

        long[] workLeft = { MaxAggregateSequenceWork };
        HashSet<string> opsSeen = new();
        bool acceptedIdSeen = false;
        bool rejectedIdSeen = false;

        foreach (object? raw in cases)
        {
            if (raw is not Dictionary<string, object?> c)
            {
                problems.Add("case is not an object: " + Show(raw));
                continue;
            }

            if (!c.TryGetValue("id", out object? rawIdObj) || rawIdObj is not string rawId || rawId.Length == 0)
            {
                problems.Add("case missing a string id: " + Show(c));
                continue;
            }

            string cid = Clip.Apply(rawId, 100);
            string op = c.TryGetValue("op", out object? opObj) && opObj is string os ? os : string.Empty;
            if (!Shapes.ContainsKey(op))
            {
                c.TryGetValue("op", out object? shown);
                problems.Add(cid + ": unknown op " + Show(shown));
                continue;
            }

            if (!FieldSetMatches(c, op))
            {
                List<string> keys = c.Keys.ToList();
                keys.Sort(string.CompareOrdinal);
                List<string> want = new() { "id", "op" };
                want.AddRange(Shapes[op]);
                problems.Add(cid + ": fields [" + string.Join(", ", keys) + "] != [" + string.Join(", ", want) + "]");
                continue;
            }

            Eval ev = EvaluateCase(cid, op, c, workLeft);
            problems.AddRange(ev.Problems);
            if (!ev.RanOk)
            {
                continue;
            }

            if (op == "failure_id_valid")
            {
                acceptedIdSeen |= ev.Accepted;
                rejectedIdSeen |= !ev.Accepted;
            }

            opsSeen.Add(op);
            evaluated++;
        }

        List<string> missingOps = Shapes.Keys.Where(k => !opsSeen.Contains(k)).OrderBy(k => k, StringComparer.Ordinal).ToList();
        if (missingOps.Count > 0)
        {
            problems.Add("fuzz-v1: operations never exercised: [" + string.Join(", ", missingOps) + "]");
        }

        if (!acceptedIdSeen)
        {
            problems.Add("fuzz-v1: failure_id_valid needs a registered-id acceptance");
        }

        if (!rejectedIdSeen)
        {
            problems.Add("fuzz-v1: failure_id_valid needs an unregistered-id rejection");
        }

        List<string> unregistered = FuzzFailureIds.All.Where(id => !FuzzHooks.IsFailureId(id)).ToList();
        if (unregistered.Count > 0)
        {
            problems.Add("fuzz-v1: engine rejects registered ids [" + string.Join(", ", unregistered) + "]");
        }

        return new FuzzCorpusReport(evaluated, problems.Select(p => Clip.Apply(p, MaxProblemLen)).ToList());
    }

    private sealed record Eval(IReadOnlyList<string> Problems, bool RanOk, bool Accepted);

    // Decodes and checks one case. Any exception (a malformed case or an engine that threw) is
    // reported as a problem.
    private static Eval EvaluateCase(string cid, string op, Dictionary<string, object?> c, long[] work)
    {
        try
        {
            Dictionary<string, object?> v = new();
            foreach (KeyValuePair<string, object?> e in c)
            {
                if (e.Key != "id" && e.Key != "op")
                {
                    v[e.Key] = Decode(e.Value);
                }
            }

            IReadOnlyList<string> found = Array.Empty<string>();
            bool accepted = false;
            switch (op)
            {
                case "corpus_admit":
                    found = CheckCorpusAdmit(cid, v);
                    break;
                case "crash_dedup":
                    found = CheckCrashDedup(cid, v);
                    break;
                case "failure_id_valid":
                    {
                        string? input = v["input"] as string;
                        accepted = input != null && FuzzHooks.IsFailureId(input);
                        if (!Same(accepted, v["valid"]))
                        {
                            found = new[] { cid + ": failure id membership disagrees for " + Show(v["input"]) };
                        }

                        break;
                    }

                case "private_reporting":
                    found = CheckPrivateReporting(cid, v);
                    break;
                case "corpus_reject":
                    {
                        if (v["text"] is not string text)
                        {
                            throw new CaseException("text must be a string");
                        }

                        if (FuzzHooks.TfvValid(text))
                        {
                            found = new[] { cid + ": corpus_reject text was accepted by the strict parser" };
                        }

                        break;
                    }

                default:
                    found = CheckMutator(cid, op, v, work); // the nine byte-in, byte-out mutators
                    break;
            }

            return new Eval(found, true, accepted);
        }
        catch (Exception e) when (Safe.Recoverable(e))
        {
            return new Eval(
                new[] { cid + ": " + op + " could not be evaluated: " + Clip.Apply(Safe.SafeString(e), MaxFragmentLen) },
                false,
                false);
        }
    }

    // ---------------------------------------------------------------------------
    // Typed field access
    // ---------------------------------------------------------------------------

    private static byte[] BytesOf(object? v, string what) => v as byte[] ?? throw new CaseException(what + " must be bytes");

    private static string TextOf(object? v, string what) => v as string ?? throw new CaseException(what + " must be a string");

    private static bool BoolOf(object? v, string what) => v is bool b ? b : throw new CaseException(what + " must be a bool");

    private static BigInteger IntOf(object? v, string what) => v is BigInteger b ? b : throw new CaseException(what + " must be an int");

    /// <summary>An int that drives a loop or an allocation: 0 &lt;= n &lt;= MaxSize.</summary>
    private static int SizeOf(object? v, string what)
    {
        BigInteger b = IntOf(v, what);
        if (b.Sign < 0 || b > MaxSize)
        {
            throw new CaseException(what + " " + Show(b) + " outside [0, " + MaxSize + "]");
        }

        return (int)b;
    }

    /// <summary>
    /// Maps a corpus width to an int; anything outside [0, 64] becomes -1, which the mutators treat as
    /// "not one of 1, 2, 4, 8" (input unchanged).
    /// </summary>
    private static int WidthOf(object? v)
    {
        BigInteger b = IntOf(v, "width");
        return b.Sign < 0 || b > 64 ? -1 : (int)b;
    }

    private static Endian EndianOf(object? v)
    {
        string s = TextOf(v, "endian");
        return s switch
        {
            "little" => Endian.Little,
            "big" => Endian.Big,
            _ => throw new CaseException("endian must be 'little' or 'big', got " + Show(s)),
        };
    }

    private static List<string> TextsOf(object? v, string what)
    {
        if (v is not List<object?> list)
        {
            throw new CaseException(what + " must be a list of strings");
        }

        List<string> output = new(list.Count);
        foreach (object? item in list)
        {
            output.Add(item as string ?? throw new CaseException(what + " must be a list of strings"));
        }

        return output;
    }

    private static readonly BigInteger Two64 = BigInteger.One << 64;

    /// <summary>A list of edge ids; each must fit in an unsigned 64-bit word.</summary>
    private static ulong[] EdgesOf(object? v, string what)
    {
        if (v is not List<object?> list)
        {
            throw new CaseException(what + " must be a list of ints");
        }

        ulong[] output = new ulong[list.Count];
        for (int i = 0; i < output.Length; i++)
        {
            if (list[i] is not BigInteger b)
            {
                throw new CaseException(what + " must be a list of ints");
            }

            if (b.Sign < 0 || b >= Two64)
            {
                throw new CaseException(what + " element " + Show(b) + " outside [0, 2^64)");
            }

            output[i] = (ulong)b;
        }

        return output;
    }

    // ---------------------------------------------------------------------------
    // Per-op checks
    // ---------------------------------------------------------------------------

    private static IReadOnlyList<string> CheckMutator(string cid, string op, Dictionary<string, object?> v, long[] work)
    {
        byte[] got;
        switch (op)
        {
            case "mutate_bit_flip":
                got = FuzzHooks.BitFlip(BytesOf(v["input"], "input"), IntOf(v["bit_offset"], "bit_offset"), IntOf(v["count"], "count"));
                break;
            case "mutate_byte_flip":
                got = FuzzHooks.ByteFlip(BytesOf(v["input"], "input"), IntOf(v["byte_offset"], "byte_offset"), IntOf(v["count"], "count"));
                break;
            case "mutate_arithmetic":
                {
                    byte[] input = BytesOf(v["input"], "input");
                    BigInteger offset = IntOf(v["byte_offset"], "byte_offset");
                    BigInteger delta = IntOf(v["delta"], "delta");
                    int width = WidthOf(v["width"]);
                    Endian endian = EndianOf(v["endian"]);
                    got = FuzzHooks.ByteArithmetic(input, offset, delta, width, endian);
                    break;
                }

            case "mutate_boundary":
                {
                    byte[] input = BytesOf(v["input"], "input");
                    BigInteger offset = IntOf(v["byte_offset"], "byte_offset");
                    int width = WidthOf(v["width"]);
                    BigInteger val = IntOf(v["val"], "val");
                    Endian endian = EndianOf(v["endian"]);
                    got = FuzzHooks.BoundaryInsert(input, offset, width, val, endian);
                    break;
                }

            case "mutate_block_delete":
                got = FuzzHooks.BlockDelete(BytesOf(v["input"], "input"), IntOf(v["offset"], "offset"), IntOf(v["length"], "length"));
                break;
            case "mutate_block_insert":
                {
                    byte[] input = BytesOf(v["input"], "input");
                    BigInteger offset = IntOf(v["offset"], "offset");
                    byte[] ins = BytesOf(v["insert_bytes"], "insert_bytes");
                    got = FuzzHooks.BlockInsert(input, offset, ins);
                    break;
                }

            case "mutate_block_duplicate":
                {
                    byte[] input = BytesOf(v["input"], "input");
                    BigInteger src = IntOf(v["src_offset"], "src_offset");
                    BigInteger length = IntOf(v["length"], "length");
                    BigInteger dst = IntOf(v["dst_offset"], "dst_offset");
                    got = FuzzHooks.BlockDuplicate(input, src, length, dst);
                    break;
                }

            case "mutate_splice":
                {
                    byte[] target = BytesOf(v["target"], "target");
                    byte[] donor = BytesOf(v["donor"], "donor");
                    BigInteger ts = IntOf(v["target_split"], "target_split");
                    BigInteger ds = IntOf(v["donor_split"], "donor_split");
                    got = FuzzHooks.SpliceHook(target, donor, ts, ds);
                    List<string> spliceProblems = Compare(cid, op, got, v);
                    long want = Mutate.SpliceLength(target.Length, donor.Length, Mutate.SatU64(ts), Mutate.SatU64(ds));
                    if (got.Length != want)
                    {
                        spliceProblems.Add(cid + ": splice length is not prefix + suffix");
                    }

                    return spliceProblems;
                }

            default: // mutate_sequence
                {
                    byte[] input = BytesOf(v["input"], "input");
                    BigInteger seed = IntOf(v["seed"], "seed");
                    int steps = SizeOf(v["steps"], "steps");

                    // Charge the REAL work: the budget handed down is what is left of the aggregate
                    // cap, and what the call actually copied (block inserts and duplicates grow the
                    // buffer, so steps x initial length would undercount) is deducted afterwards. A
                    // call that hits its cap is charged too, and an exhausted aggregate fails every
                    // later case at once without running it: otherwise each failing case would be
                    // handed the full budget again.
                    if (work[0] <= 0)
                    {
                        throw new CaseException("mutate_sequence work budget of " + MaxAggregateSequenceWork + " byte-steps exhausted");
                    }

                    Mutate.CountedResult done;
                    try
                    {
                        done = FuzzHooks.MutateSequence(input, Mutate.Low64(seed), steps, Mutate.MaxInputBytes, work[0]);
                    }
                    catch (Mutate.WorkExhausted e)
                    {
                        work[0] -= Math.Max(e.Work, 0);
                        if (work[0] > 0)
                        {
                            work[0] = 0; // the aggregate is breached: nothing is left for any later case
                        }

                        throw new CaseException("mutate_sequence work budget of " + MaxAggregateSequenceWork + " byte-steps exhausted");
                    }

                    work[0] -= done.Work;
                    got = done.Data;
                    break;
                }
        }

        List<string> problems = Compare(cid, op, got, v);

        // Engine invariant, independent of the corpus's expected bytes.
        if (op is "mutate_bit_flip" or "mutate_byte_flip" or "mutate_arithmetic" or "mutate_boundary"
            && got.Length != ((byte[])v["input"]!).Length)
        {
            problems.Add(cid + ": " + op + " changed the length of its input");
        }

        return problems;
    }

    private static List<string> Compare(string cid, string op, byte[] got, Dictionary<string, object?> v)
    {
        List<string> problems = new();
        if (v["output"] is not byte[] want)
        {
            problems.Add(cid + ": " + op + " output must be bytes");
            return problems;
        }

        if (!got.AsSpan().SequenceEqual(want))
        {
            problems.Add(
                cid + ": " + op + " expected " + Clip.Apply(Convert.ToHexString(want).ToLowerInvariant(), MaxFragmentLen)
                + ", got " + Clip.Apply(Convert.ToHexString(got).ToLowerInvariant(), MaxFragmentLen));
        }

        return problems;
    }

    private static List<string> CheckCorpusAdmit(string cid, Dictionary<string, object?> v)
    {
        ulong[] existing = EdgesOf(v["existing_edges"], "existing_edges");
        ulong[] trace = EdgesOf(v["trace_edges"], "trace_edges");
        bool admitted = BoolOf(v["admitted"], "admitted");
        BigInteger newEdges = IntOf(v["new_edges"], "new_edges");
        Bitmap.Novelty got = FuzzHooks.CorpusAdmit(existing, trace);
        List<string> problems = new();
        if (got.Admitted != admitted)
        {
            problems.Add(cid + ": corpus_admit admitted " + got.Admitted.ToString().ToLowerInvariant() + ", corpus " + admitted.ToString().ToLowerInvariant());
        }

        if (!Same(new BigInteger(got.NewEdges), newEdges))
        {
            problems.Add(cid + ": corpus_admit new_edges " + got.NewEdges + ", corpus " + Show(newEdges));
        }

        // Invariant: admission is exactly "some trace edge is unseen".
        HashSet<ulong> have = new(existing);
        bool novel = trace.Any(e => !have.Contains(e));
        if (got.Admitted != novel)
        {
            problems.Add(cid + ": corpus_admit disagrees with set difference");
        }

        return problems;
    }

    private static List<string> CheckCrashDedup(string cid, Dictionary<string, object?> v)
    {
        string ka = TextOf(v["crash_a_kind"], "crash_a_kind");
        List<string> fa = TextsOf(v["crash_a_frames"], "crash_a_frames");
        string kb = TextOf(v["crash_b_kind"], "crash_b_kind");
        List<string> fb = TextsOf(v["crash_b_frames"], "crash_b_frames");
        bool want = BoolOf(v["is_duplicate"], "is_duplicate");
        bool got = FuzzHooks.IsDuplicate(ka, fa, kb, fb);
        List<string> problems = new();
        if (got != want)
        {
            problems.Add(cid + ": crash_dedup is_duplicate " + got.ToString().ToLowerInvariant() + ", corpus " + want.ToString().ToLowerInvariant());
        }

        return problems;
    }

    private static List<string> CheckPrivateReporting(string cid, Dictionary<string, object?> v)
    {
        byte[] raw = BytesOf(v["raw_payload"], "raw_payload");
        string target = TextOf(v["target"], "target");
        string fid = TextOf(v["failure_id"], "failure_id");
        string sig = TextOf(v["sig_hash"], "sig_hash");
        string tap = TextOf(v["tap_line"], "tap_line");
        bool wantLeak = BoolOf(v["contains_raw_payload"], "contains_raw_payload");
        string line;
        try
        {
            line = FuzzHooks.FormatTap(1, target, fid, sig);
        }
        catch (ArgumentException e)
        {
            throw new CaseException("format_tap_failure refused: " + e.Message);
        }

        List<string> problems = new();
        if (line != tap)
        {
            problems.Add(cid + ": tap_line expected " + Clip.Apply(tap, MaxFragmentLen) + ", got " + Clip.Apply(line, MaxFragmentLen));
        }

        bool leaked = raw.Length > 0 && line.Contains(Convert.ToHexString(raw).ToLowerInvariant(), StringComparison.Ordinal);
        if (leaked != wantLeak)
        {
            problems.Add(cid + ": privacy verdict wrong, payload leaked: " + leaked.ToString().ToLowerInvariant());
        }

        if (line.Contains('\r') || line.Contains('\n'))
        {
            problems.Add(cid + ": failure line spans more than one line");
        }

        return problems;
    }
}
