using System.Numerics;
using CodingAdventures.TestBootstrap.CorpusJson;
using CodingAdventures.TestBootstrap.Tfv;
using static CodingAdventures.TestBootstrap.CorpusJson.Values;

namespace CodingAdventures.TestBootstrap.Pbt;

/// <summary>How many cases were fully evaluated (match or not) and every problem found.</summary>
public sealed record CorpusReport(int Evaluated, IReadOnlyList<string> Problems);

/// <summary>
/// The production runner for the shared pbt-v1 corpus
/// (<c>code/specs/fixtures/pbt-v1/cases.json</c>, VER05 section 6).
/// </summary>
/// <remarks>
/// It takes the raw JSON, strictly decodes it, evaluates every case with this package's own
/// public engine functions and returns human-readable problems. An empty list means
/// "conforms". It does no file I/O (the package is capability-free); the test that reads the
/// committed fixture lives in the test tree.
/// <para>
/// Why a runner that can fail matters: a conformance check that cannot report a problem is
/// worse than none. So the runner is strict:
/// </para>
/// <list type="bullet">
/// <item>a case must have exactly the fields of its op, with canonical encodings (ints are
/// decimal strings, bytes are even-length lowercase hex);</item>
/// <item>an unknown op, unknown property name or malformed value is a problem, never a skip;</item>
/// <item>every op and all four failure ids must appear, at least one unregistered id must be
/// rejected, and the number of cases must equal <see cref="Pbtv1CaseCount"/>, so deleting cases
/// from the list is itself a problem;</item>
/// <item>every problem about a case starts with that case's id.</item>
/// </list>
/// </remarks>
public static class Corpus
{
    /// <summary>How many cases pbt-v1 holds; a different count is a problem.</summary>
    public const int Pbtv1CaseCount = 25;

    private const int MaxCasesCap = 10_000;

    /// <summary>
    /// Bounds the total bytes one gen case may generate and hold: count times the per-item
    /// width. The separate caps on count and width alone would still allow 10,000 x 10,000
    /// bytes (CWE-400).
    /// </summary>
    private const int MaxGenBytes = 1 << 20;

    /// <summary>
    /// Strictly decodes a pbt-v1 document into its case list. The document must be exactly
    /// <c>{"schema_version": 1, "cases": [...]}</c>. Duplicate object keys, trailing data,
    /// nesting deeper than 64 and documents over 8 MiB are refused with
    /// <see cref="JsonException"/>. Cases are returned unvalidated.
    /// </summary>
    public static List<object?> Parse(byte[] data)
    {
        object? root = StrictJson.Parse(data);
        if (root is not Dictionary<string, object?> doc || doc.Count != 2)
        {
            throw new JsonException("pbt corpus: want exactly {\"schema_version\": 1, \"cases\": [...]}");
        }

        if (!doc.TryGetValue("schema_version", out object? version) || version is not JsonNumber { Text: "1" })
        {
            throw new JsonException("pbt corpus: schema_version must be 1");
        }

        if (!doc.TryGetValue("cases", out object? cases) || cases is not List<object?> list)
        {
            throw new JsonException("pbt corpus: \"cases\" must be an array");
        }

        return new List<object?>(list);
    }

    // ---------------------------------------------------------------------------
    // Properties named by the corpus. true means "holds".
    // ---------------------------------------------------------------------------

    private static readonly BigInteger B100 = new(100);
    private static readonly BigInteger B50 = new(50);
    private static readonly BigInteger BM10 = new(-10);
    private static readonly BigInteger B10 = new(10);
    private static readonly BigInteger B20 = new(20);

    private static readonly Dictionary<string, Func<BigInteger, bool>> IntProps = new()
    {
        ["int_le_100"] = x => x <= B100,
        ["int_in_range_neg10_50"] = x => x >= BM10 && x <= B50,
        ["int_is_even"] = x => x.IsEven, // two's complement semantics: correct for negatives
        ["int_abs_le_10"] = x => BigInteger.Abs(x) <= B10,
        ["refinement_x_le_50"] = x => x <= B50,
    };

    private static readonly Dictionary<string, Func<List<BigInteger>, bool>> ListProps = new()
    {
        ["list_no_consecutive_duplicates"] = l =>
        {
            for (int i = 0; i + 1 < l.Count; i++)
            {
                if (l[i] == l[i + 1])
                {
                    return false;
                }
            }

            return true;
        },
        ["list_sum_le_20"] = l =>
        {
            BigInteger sum = BigInteger.Zero;
            foreach (BigInteger v in l)
            {
                sum += v;
            }

            return sum <= B20;
        },
    };

    private static readonly Dictionary<string, Func<byte[], bool>> BytesProps = new()
    {
        ["bytes_len_lt_5"] = b => b.Length < 5,
        ["bytes_no_zero_byte"] = b => Array.IndexOf(b, (byte)0) < 0,
    };

    /// <summary>The closed field set of each op (besides id and op).</summary>
    private static readonly Dictionary<string, string[]> Shapes = new()
    {
        ["gen_primitive"] = new[] { "type", "seed", "count", "outputs" },
        ["gen_bounded"] = new[] { "min", "max", "seed", "count", "outputs" },
        ["gen_bytes"] = new[] { "min_len", "max_len", "seed", "count", "outputs" },
        ["shrink_int"] = new[] { "initial", "target", "min", "max", "property", "expected_minimal", "expected_steps" },
        ["shrink_refinement"] = new[] { "initial", "target", "min", "max", "property", "expected_minimal", "expected_steps" },
        ["shrink_list"] = new[] { "initial", "elem_target", "property", "expected_minimal", "expected_steps" },
        ["shrink_bytes"] = new[] { "initial", "property", "expected_minimal", "expected_steps" },
        ["replay_consistency"] = new[] { "seed", "property", "initial_fail", "expected_minimal" },
        ["filter_exhausted"] = new[] { "max_retries", "verdict", "failure_id" },
        ["failure_id_valid"] = new[] { "input", "valid" },
        ["corpus_reject"] = new[] { "text" },
    };

    private static bool FieldSetMatches(Dictionary<string, object?> c, string op)
    {
        string[] fields = Shapes[op];
        if (c.Count != fields.Length + 2 || !c.ContainsKey("id") || !c.ContainsKey("op"))
        {
            return false;
        }

        foreach (string f in fields)
        {
            if (!c.ContainsKey(f))
            {
                return false;
            }
        }

        return true;
    }

    /// <summary><see cref="Evaluate(IReadOnlyList{object?}, int)"/> with the pbt-v1 case count.</summary>
    public static CorpusReport Evaluate(IReadOnlyList<object?> cases)
    {
        return Evaluate(cases, Pbtv1CaseCount);
    }

    /// <summary>
    /// Evaluates every case and returns the problems (none means the engine conforms). A
    /// negative <paramref name="expected"/> skips the case-count check (for ad-hoc case lists
    /// in tests).
    /// </summary>
    public static CorpusReport Evaluate(IReadOnlyList<object?> casesIn, int expected)
    {
        List<string> problems = new();
        int evaluated = 0;
        IReadOnlyList<object?> cases = casesIn;
        if (expected >= 0 && cases.Count != expected)
        {
            problems.Add("pbt-v1: expected " + expected + " cases, got " + cases.Count);
        }

        if (cases.Count > MaxCasesCap)
        {
            problems.Add("pbt-v1: " + cases.Count + " cases exceeds the cap of " + MaxCasesCap);
            cases = cases.Take(MaxCasesCap).ToList();
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

        HashSet<string> opsSeen = new();
        HashSet<string> registeredSeen = new();
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

            Eval ev = EvaluateCase(cid, op, c);
            problems.AddRange(ev.Problems);
            if (!ev.RanOk)
            {
                continue;
            }

            if (op == "failure_id_valid")
            {
                if (ev.Registered)
                {
                    registeredSeen.Add(ev.Candidate);
                }
                else
                {
                    rejectedIdSeen = true;
                }
            }

            opsSeen.Add(op);
            evaluated++;
        }

        List<string> missingOps = Shapes.Keys.Where(k => !opsSeen.Contains(k)).OrderBy(k => k, StringComparer.Ordinal).ToList();
        if (missingOps.Count > 0)
        {
            problems.Add("pbt-v1: operations never exercised: [" + string.Join(", ", missingOps) + "]");
        }

        List<string> missingIds = FailureIds.All.Where(id => !registeredSeen.Contains(id)).ToList();
        if (missingIds.Count > 0)
        {
            problems.Add("pbt-v1: registered failure ids missing: [" + string.Join(", ", missingIds) + "]");
        }

        if (!rejectedIdSeen)
        {
            problems.Add("pbt-v1: failure_id_valid needs an unregistered-id rejection");
        }

        return new CorpusReport(evaluated, problems.Select(p => Clip.Apply(p, MaxProblemLen)).ToList());
    }

    private sealed record Eval(IReadOnlyList<string> Problems, bool RanOk, string Candidate, bool Registered);

    // Decodes and checks one case. Any exception (a malformed case or an engine that
    // threw) is reported as a problem.
    private static Eval EvaluateCase(string cid, string op, Dictionary<string, object?> c)
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
            string candidate = string.Empty;
            bool registered = false;
            switch (op)
            {
                case "gen_primitive":
                case "gen_bounded":
                case "gen_bytes":
                    found = CheckGen(cid, op, v);
                    break;
                case "shrink_int":
                case "shrink_refinement":
                    found = CheckShrinkInt(cid, op, v);
                    break;
                case "shrink_list":
                    found = CheckShrinkList(cid, v);
                    break;
                case "shrink_bytes":
                    found = CheckShrinkBytes(cid, v);
                    break;
                case "replay_consistency":
                    found = CheckReplay(cid, v);
                    break;
                case "filter_exhausted":
                    found = CheckFilterExhausted(cid, v);
                    break;
                case "failure_id_valid":
                    {
                        string? input = v["input"] as string;
                        bool got = input != null && FailureIds.IsPbtFailureId(input);
                        if (!Same(got, v["valid"]))
                        {
                            found = new[] { cid + ": failure id membership disagrees for " + Show(v["input"]) };
                        }

                        candidate = input ?? string.Empty;
                        registered = got;
                        break;
                    }

                default: // corpus_reject
                    {
                        if (v["text"] is not string text)
                        {
                            throw new CaseException("text must be a string");
                        }

                        if (TfvValidator.IsValid(text))
                        {
                            found = new[] { cid + ": corpus_reject text was accepted by the strict parser" };
                        }

                        break;
                    }
            }

            return new Eval(found, true, candidate, registered);
        }
        catch (Exception e) when (Safe.Recoverable(e))
        {
            return new Eval(
                new[] { cid + ": " + op + " could not be evaluated: " + Clip.Apply(Safe.SafeString(e), MaxFragmentLen) },
                false,
                string.Empty,
                false);
        }
    }

    /// <summary>A corpus seed reduced modulo 2^64, as the reference masks it (negative seeds wrap).</summary>
    private static ulong SeedOf(object? v)
    {
        if (v is not BigInteger b)
        {
            throw new CaseException("seed must be an int");
        }

        // BigInteger's bitwise AND uses two's complement, so a negative seed keeps its low 64 bits.
        return (ulong)(b & ulong.MaxValue);
    }

    private static BigInteger IntOf(object? v, string what)
    {
        if (v is not BigInteger b)
        {
            throw new CaseException(what + " must be an int");
        }

        return b;
    }

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

    private static string PropName(object? v)
    {
        if (v is not string s)
        {
            throw new CaseException("unknown property " + Show(v));
        }

        return s;
    }

    private static IReadOnlyList<string> CheckGen(string cid, string op, Dictionary<string, object?> v)
    {
        ulong seed = SeedOf(v["seed"]);
        int count = SizeOf(v["count"], "count");

        // The expected outputs are in hand already: a case whose list length is not count
        // cannot conform, so refuse it before generating anything.
        if (v["outputs"] is not IList<object?> expected || expected.Count != count)
        {
            throw new CaseException("outputs must be a list of exactly count (" + count + ") values");
        }

        List<object?> got;
        switch (op)
        {
            case "gen_primitive":
                {
                    if (v["type"] is not string name)
                    {
                        throw new CaseException("type must be a string");
                    }

                    got = Gens.GenPrimitive(seed, name, count).Cast<object?>().ToList();
                    break;
                }

            case "gen_bounded":
                {
                    BigInteger lo = IntOf(v["min"], "min");
                    BigInteger hi = IntOf(v["max"], "max");

                    // Gens.GenBounded enforces the count x width generation budget itself.
                    got = Gens.GenBounded(seed, lo, hi, count).Cast<object?>().ToList();
                    break;
                }

            default:
                {
                    int minLen = SizeOf(v["min_len"], "min_len");
                    int maxLen = SizeOf(v["max_len"], "max_len");
                    if ((long)count * maxLen > MaxGenBytes)
                    {
                        throw new CaseException(
                            "count " + count + " x max_len " + maxLen + " exceeds the generation budget of " + MaxGenBytes + " bytes");
                    }

                    got = Gens.GenBytes(seed, minLen, maxLen, count).Cast<object?>().ToList();
                    break;
                }
        }

        return Same(got, v["outputs"]) ? Array.Empty<string>() : new[] { cid + ": " + op + " outputs mismatch" };
    }

    private static IReadOnlyList<string> CheckShrinkInt(string cid, string op, Dictionary<string, object?> v)
    {
        string name = PropName(v["property"]);
        if (!IntProps.TryGetValue(name, out Func<BigInteger, bool>? prop))
        {
            throw new CaseException("unknown property " + Show(name) + " for " + op);
        }

        BigInteger initial = IntOf(v["initial"], "initial");
        BigInteger target = IntOf(v["target"], "target");
        BigInteger lo = IntOf(v["min"], "min");
        BigInteger hi = IntOf(v["max"], "max");
        Shrunk<BigInteger> r = Shrink.ShrinkInt(initial, target, lo, hi, prop, 1000);
        List<string> problems = new();
        if (!Same(r.Value, v["expected_minimal"]))
        {
            problems.Add(cid + ": " + op + " minimal mismatch: got " + Show(r.Value) + ", corpus " + Show(v["expected_minimal"]));
        }

        if (!Same(new BigInteger(r.Steps), v["expected_steps"]))
        {
            problems.Add(cid + ": " + op + " step count mismatch: got " + r.Steps + ", corpus " + Show(v["expected_steps"]));
        }

        // Engine invariants, independent of the corpus's expected values.
        if (prop(r.Value))
        {
            problems.Add(cid + ": " + op + " minimal " + Show(r.Value) + " does not falsify the property");
        }

        if (r.Value < lo || r.Value > hi)
        {
            problems.Add(cid + ": " + op + " minimal " + Show(r.Value) + " left [" + Show(lo) + ", " + Show(hi) + "]");
        }

        return problems;
    }

    private static IReadOnlyList<string> CheckShrinkList(string cid, Dictionary<string, object?> v)
    {
        string name = PropName(v["property"]);
        if (!ListProps.TryGetValue(name, out Func<List<BigInteger>, bool>? prop))
        {
            throw new CaseException("unknown property " + Show(name) + " for shrink_list");
        }

        if (v["initial"] is not IList<object?> items)
        {
            throw new CaseException("initial must be a list of ints");
        }

        List<BigInteger> initial = new();
        foreach (object? item in items)
        {
            initial.Add(IntOf(item, "element"));
        }

        BigInteger elemTarget = IntOf(v["elem_target"], "elem_target");
        Shrunk<List<BigInteger>> r = Shrink.ShrinkList(initial, elemTarget, prop, 1000);
        List<string> problems = new();
        if (!Same(r.Value.Cast<object?>().ToList(), v["expected_minimal"]))
        {
            problems.Add(
                cid + ": shrink_list minimal mismatch: got " + Canon.Render(r.Value) + ", corpus "
                + Clip.Apply(Canon.Render(v["expected_minimal"]), MaxFragmentLen));
        }

        if (!Same(new BigInteger(r.Steps), v["expected_steps"]))
        {
            problems.Add(cid + ": shrink_list steps mismatch: got " + r.Steps + ", corpus " + Show(v["expected_steps"]));
        }

        if (prop(r.Value))
        {
            problems.Add(cid + ": shrink_list minimal " + Show(r.Value) + " does not falsify the property");
        }

        return problems;
    }

    private static IReadOnlyList<string> CheckShrinkBytes(string cid, Dictionary<string, object?> v)
    {
        string name = PropName(v["property"]);
        if (!BytesProps.TryGetValue(name, out Func<byte[], bool>? prop))
        {
            throw new CaseException("unknown property " + Show(name) + " for shrink_bytes");
        }

        if (v["initial"] is not byte[] initial)
        {
            throw new CaseException("initial must be bytes");
        }

        Shrunk<byte[]> r = Shrink.ShrinkBytes(initial, prop, 1000);
        List<string> problems = new();
        if (!Same(r.Value, v["expected_minimal"]))
        {
            problems.Add(cid + ": shrink_bytes minimal mismatch: got " + Show(r.Value) + ", corpus " + Show(v["expected_minimal"]));
        }

        if (!Same(new BigInteger(r.Steps), v["expected_steps"]))
        {
            problems.Add(cid + ": shrink_bytes steps mismatch: got " + r.Steps + ", corpus " + Show(v["expected_steps"]));
        }

        if (prop(r.Value))
        {
            problems.Add(cid + ": shrink_bytes minimal " + Show(r.Value) + " does not falsify the property");
        }

        return problems;
    }

    private static IReadOnlyList<string> CheckReplay(string cid, Dictionary<string, object?> v)
    {
        ulong seed = SeedOf(v["seed"]);
        string name = PropName(v["property"]);
        if (!IntProps.TryGetValue(name, out Func<BigInteger, bool>? prop))
        {
            throw new CaseException("unknown property " + Show(name) + " for replay_consistency");
        }

        Shrink.Replay r = Shrink.SimulateReplay(seed, prop);
        List<string> problems = new();
        if (!Same(r.InitialFail, v["initial_fail"]))
        {
            problems.Add(cid + ": replay initial_fail mismatch: got " + Show(r.InitialFail) + ", corpus " + Show(v["initial_fail"]));
        }

        if (!Same(r.Minimal, v["expected_minimal"]))
        {
            problems.Add(cid + ": replay minimal mismatch: got " + Show(r.Minimal) + ", corpus " + Show(v["expected_minimal"]));
        }

        return problems;
    }

    private static IReadOnlyList<string> CheckFilterExhausted(string cid, Dictionary<string, object?> v)
    {
        int maxRetries = SizeOf(v["max_retries"], "max_retries");
        if (maxRetries < 1)
        {
            throw new CaseException("max_retries must be at least 1");
        }

        int calls = 0;
        Gen<BigInteger> gen = Gens.Filter(
            Gens.Integer(BigInteger.Zero, BigInteger.Parse("10")),
            x =>
            {
                calls++;
                return false;
            },
            maxRetries);
        Verdict verdict = Runner.Check(0, gen, _ => true, new Options(1, 1000, CiMode.On));
        List<string> problems = new();
        if (!Same("fail", v["verdict"]) || verdict.Passed)
        {
            problems.Add(cid + ": an unsatisfiable filter must be a fail verdict");
        }

        if (!Same(verdict.FailureId, v["failure_id"]))
        {
            problems.Add(cid + ": failure id \"" + verdict.FailureId + "\", corpus " + Show(v["failure_id"]));
        }

        if (calls != maxRetries)
        {
            problems.Add(cid + ": filter sampled " + calls + " times, expected " + maxRetries);
        }

        return problems;
    }
}
