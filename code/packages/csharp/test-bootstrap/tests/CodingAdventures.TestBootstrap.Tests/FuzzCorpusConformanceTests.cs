using System.Numerics;
using CodingAdventures.TestBootstrap.CorpusJson;
using CodingAdventures.TestBootstrap.Fuzz;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>
/// Replays the whole committed fuzz-v1 corpus and carries its negative controls (VER06h section 10).
/// The fixture is read from its committed path, never a copy.
/// </summary>
public sealed class FuzzCorpusConformanceTests : FuzzHookedTest
{
    // ---- corruption ------------------------------------------------------------

    /// <summary>Same shape, different value.</summary>
    private static object? CorruptValue(object? node)
    {
        if (node is bool b)
        {
            return !b;
        }

        Dictionary<string, object?> m = Fx.Obj(node);
        if ((string)m["kind"]! == "int")
        {
            m["value"] = (Fx.Big((string)m["value"]!) + 1).ToString();
        }
        else
        {
            string hex = (string)m["hex"]!;
            m["hex"] = hex.Length == 0 ? "ff" : hex.Substring(2);
        }

        return m;
    }

    private static List<Action<Dictionary<string, object?>>> Corruptions(string op) => op switch
    {
        "mutate_bit_flip" or "mutate_byte_flip" or "mutate_arithmetic" or "mutate_boundary" or "mutate_block_delete"
            or "mutate_block_insert" or "mutate_block_duplicate" or "mutate_splice" or "mutate_sequence"
            => new() { c => c["output"] = CorruptValue(c["output"]) },
        "corpus_admit" => new() { c => c["admitted"] = CorruptValue(c["admitted"]), c => c["new_edges"] = CorruptValue(c["new_edges"]) },
        "crash_dedup" => new() { c => c["is_duplicate"] = CorruptValue(c["is_duplicate"]) },
        "failure_id_valid" => new() { c => c["valid"] = CorruptValue(c["valid"]) },
        "private_reporting" => new()
        {
            c => c["tap_line"] = "not ok 1 - ct_compare [fuzz-crash:3a8f19b3]",
            c => c["contains_raw_payload"] = CorruptValue(c["contains_raw_payload"]),
        },
        "corpus_reject" => new() { c => c["text"] = "tfv 1\n" },
        _ => throw new InvalidOperationException("no corruption defined for op " + op),
    };

    // ---- the positive run ------------------------------------------------------

    [Fact]
    public void CorpusConformsToAllTwentyFiveCases()
    {
        FuzzCorpusReport report = FuzzCorpus.Evaluate(FuzzFx.LoadCases());
        Assert.Empty(report.Problems);
        Assert.Equal(25, report.Evaluated);
        Assert.Equal(25, FuzzCorpus.Fuzzv1CaseCount);
    }

    [Fact]
    public void EveryOpIsPresentInTheFixtureAndHasACorruption()
    {
        SortedSet<string> ops = new(FuzzFx.LoadCases().Select(c => (string)Fx.Obj(c)["op"]!));
        Assert.Equal(14, ops.Count);
        Assert.Equal(new SortedSet<string>(FuzzCorpus.Shapes.Keys), ops);
        foreach (string op in ops)
        {
            Assert.NotEmpty(Corruptions(op));
        }
    }

    // ---- negative controls -----------------------------------------------------

    [Fact]
    public void CorruptingAnyCaseMakesTheRunFailAndNamesThatCase()
    {
        List<object?> pristine = FuzzFx.LoadCases();
        int corrupted = 0;
        for (int i = 0; i < pristine.Count; i++)
        {
            Dictionary<string, object?> original = Fx.Obj(pristine[i]);
            string id = (string)original["id"]!;
            string op = (string)original["op"]!;
            foreach (Action<Dictionary<string, object?>> corruption in Corruptions(op))
            {
                List<object?> cases = Fx.CopyCases(pristine);
                Dictionary<string, object?> victim = Fx.Obj(cases[i]);
                corruption(victim);
                Assert.NotEqual(Canon.Render(Fx.DeepCopy(original)), Canon.Render(victim));
                FuzzCorpusReport report = FuzzCorpus.Evaluate(cases);
                Assert.True(report.Problems.Count > 0, id + " (" + op + "): corruption went unnoticed");
                foreach (string p in report.Problems)
                {
                    Assert.True(p.StartsWith(id + ":", StringComparison.Ordinal), id + ": problem names another case or none: " + p);
                }

                corrupted++;
            }
        }

        Assert.True(corrupted >= 25, "every case must have been corrupted at least once, got " + corrupted);
    }

    [Fact]
    public void DroppingAnyCaseIsAProblem()
    {
        List<object?> pristine = FuzzFx.LoadCases();
        for (int i = 0; i < pristine.Count; i++)
        {
            List<object?> cases = new(pristine);
            cases.RemoveAt(i);
            FuzzCorpusReport report = FuzzCorpus.Evaluate(cases);
            Assert.NotEmpty(report.Problems);
            Assert.Contains("expected 25 cases, got 24", report.Problems[0]);
        }

        Assert.NotEmpty(FuzzCorpus.Evaluate(new List<object?>()).Problems);
        Assert.NotEmpty(FuzzCorpus.Evaluate(pristine.Take(8).ToList()).Problems);
    }

    [Fact]
    public void DroppingEveryCaseOfAnOpIsAProblemEvenWithTheCountDisabled()
    {
        List<object?> pristine = FuzzFx.LoadCases();
        foreach (string op in FuzzCorpus.Shapes.Keys)
        {
            List<object?> cases = pristine.Where(c => (string)Fx.Obj(c)["op"]! != op).ToList();
            Assert.True(FuzzFx.AnyContains(FuzzCorpus.Evaluate(cases, -1).Problems, "operations never exercised"), op);
        }
    }

    [Fact]
    public void ATruncatedDocumentFailsToParse()
    {
        byte[] data = File.ReadAllBytes(FuzzFx.FixturePath());
        Assert.Throws<JsonException>(() => FuzzCorpus.Parse(data.AsSpan(0, data.Length / 2).ToArray()));
        Assert.Throws<JsonException>(() => FuzzCorpus.Parse(Array.Empty<byte>()));
    }

    [Fact]
    public void DuplicatedCaseIdsAreAProblem()
    {
        List<object?> cases = Fx.CopyCases(FuzzFx.LoadCases());
        cases[1] = Fx.DeepCopy(cases[0]);
        Assert.Contains("duplicate or missing case id", FuzzCorpus.Evaluate(cases).Problems);
    }

    private static bool ProblemFor(List<object?> cases, string id, string fragment) =>
        FuzzCorpus.Evaluate(cases).Problems.Any(p => p.StartsWith(id + ":", StringComparison.Ordinal) && p.Contains(fragment));

    [Fact]
    public void AnUnknownOpAMissingAndAnExtraFieldAreProblems()
    {
        List<object?> cases = Fx.CopyCases(FuzzFx.LoadCases());
        Dictionary<string, object?> first = Fx.Obj(cases[0]);
        string id = (string)first["id"]!;
        first["op"] = "gen_everything";
        Assert.True(ProblemFor(cases, id, "unknown op"));
        first["op"] = "mutate_bit_flip";
        first["surprise"] = "field";
        Assert.True(ProblemFor(cases, id, "fields"));
        first.Remove("surprise");
        first.Remove("count");
        Assert.True(ProblemFor(cases, id, "fields"));
    }

    [Fact]
    public void ANonObjectCaseAndANonStringIdAreProblemsNotCrashes()
    {
        List<object?> cases = new(FuzzFx.LoadCases());
        cases[0] = "not an object";
        Assert.True(FuzzFx.AnyContains(FuzzCorpus.Evaluate(cases).Problems, "case is not an object"));
        cases[0] = new Dictionary<string, object?> { ["id"] = new JsonNumber("7"), ["op"] = "x" };
        Assert.True(FuzzFx.AnyContains(FuzzCorpus.Evaluate(cases).Problems, "missing a string id"));
    }

    [Theory]
    [InlineData("mutate_bit_flip", "bit_offset", "int:007")]
    [InlineData("mutate_bit_flip", "bit_offset", "int:+1")]
    [InlineData("mutate_bit_flip", "input", "bytes:0G")]
    [InlineData("mutate_bit_flip", "input", "bytes:ABCD")]
    [InlineData("mutate_bit_flip", "input", "bytes:abc")]
    [InlineData("mutate_bit_flip", "count", "bytes:00")]
    [InlineData("mutate_bit_flip", "input", "int:1")]
    [InlineData("mutate_arithmetic", "endian", "str:middle")]
    [InlineData("mutate_arithmetic", "endian", "bool:true")]
    [InlineData("mutate_arithmetic", "width", "str:4")]
    [InlineData("mutate_sequence", "steps", "int:10001")]
    [InlineData("mutate_sequence", "steps", "int:-1")]
    [InlineData("mutate_sequence", "seed", "str:42")]
    [InlineData("mutate_splice", "donor", "int:1")]
    [InlineData("corpus_admit", "trace_edges", "int:1")]
    [InlineData("corpus_admit", "trace_edges", "list:str")]
    [InlineData("corpus_admit", "trace_edges", "list:neg")]
    [InlineData("corpus_admit", "trace_edges", "list:big")]
    [InlineData("corpus_admit", "admitted", "int:1")]
    [InlineData("crash_dedup", "crash_a_frames", "str:x")]
    [InlineData("crash_dedup", "crash_a_frames", "list:int")]
    [InlineData("crash_dedup", "crash_a_kind", "int:1")]
    [InlineData("crash_dedup", "is_duplicate", "str:true")]
    [InlineData("private_reporting", "raw_payload", "str:x")]
    [InlineData("private_reporting", "contains_raw_payload", "str:x")]
    [InlineData("private_reporting", "sig_hash", "str:NOTHEX00")]
    [InlineData("private_reporting", "failure_id", "str:fuzz-bogus")]
    [InlineData("private_reporting", "tap_line", "int:1")]
    [InlineData("corpus_reject", "text", "int:1")]
    public void NonCanonicalAndWronglyTypedFieldsAreProblemsNamingTheCase(string op, string field, string replacement)
    {
        List<object?> cases = Fx.CopyCases(FuzzFx.LoadCases());
        Dictionary<string, object?> victim = FuzzFx.FirstOf(cases, op);
        string id = (string)victim["id"]!;
        string[] kv = replacement.Split(':', 2);
        victim[field] = kv[0] switch
        {
            "int" => FuzzFx.IntNode(kv[1]),
            "bytes" => FuzzFx.BytesNode(kv[1]),
            "bool" => bool.Parse(kv[1]),
            "str" => kv[1],
            _ => kv[1] switch
            {
                "str" => new List<object?> { "x" },
                "int" => new List<object?> { FuzzFx.IntNode("1") },
                "neg" => new List<object?> { FuzzFx.IntNode("-1") },
                _ => new List<object?> { FuzzFx.IntNode("18446744073709551616") },
            },
        };
        List<string> problems = FuzzFx.Problems(FuzzFx.Swapped(cases, victim));
        Assert.True(problems.Any(p => p.StartsWith(id + ":", StringComparison.Ordinal)), field + ": " + string.Join(" | ", problems));
    }

    [Fact]
    public void AnUnsupportedWidthLeavesTheInputUnchangedNotAProblem()
    {
        List<object?> cases = Fx.CopyCases(FuzzFx.LoadCases());
        Dictionary<string, object?> c = FuzzFx.FirstOf(cases, "mutate_arithmetic");
        c["width"] = FuzzFx.IntNode("3");
        c["output"] = c["input"];
        Assert.Empty(FuzzFx.Problems(FuzzFx.Swapped(cases, c)));
        c["width"] = FuzzFx.IntNode("99999");
        Assert.Empty(FuzzFx.Problems(FuzzFx.Swapped(cases, c)));
    }

    [Fact]
    public void CoverageGatesNeedAnAcceptedAndARejectedFailureId()
    {
        List<object?> only = FuzzFx.LoadCases().Where(c => Fx.Obj(c).TryGetValue("input", out object? i) && i is "fuzz-crash").ToList();
        Assert.True(FuzzFx.AnyContains(FuzzCorpus.Evaluate(only, -1).Problems, "unregistered-id rejection"));
        only = FuzzFx.LoadCases().Where(c => Fx.Obj(c).TryGetValue("input", out object? i) && i is "fuzz-arbitrary-error").ToList();
        Assert.True(FuzzFx.AnyContains(FuzzCorpus.Evaluate(only, -1).Problems, "registered-id acceptance"));
    }

    // ---- engine sabotage ---------------------------------------------------------

    private static readonly BigInteger One = BigInteger.One;

    private static void AssertCaught(string name)
    {
        List<string> problems = FuzzFx.Problems(FuzzFx.LoadCases());
        Assert.True(problems.Count > 0, "sabotage was not caught: " + name);
        FuzzHooks.Reset();
        Assert.Empty(FuzzFx.Problems(FuzzFx.LoadCases()));
    }

    [Fact]
    public void EverySabotagedEntryPointIsCaught()
    {
        FuzzHooks.BitFlip = (d, o, c) => Mutate.BitFlipBig(d, o + One, c);
        AssertCaught("bit_flip offset+1");
        FuzzHooks.ByteFlip = (d, o, c) => Mutate.ByteFlipBig(d, o, c + One);
        AssertCaught("byte_flip count+1");
        FuzzHooks.ByteArithmetic = (d, o, dl, w, e) => Mutate.ByteArithmeticBig(d, o, dl, w, e == Endian.Big ? Endian.Little : Endian.Big);
        AssertCaught("arithmetic endian swapped");
        FuzzHooks.BoundaryInsert = (d, o, w, v, e) => Mutate.BoundaryInsertBig(d, o, w, v ^ One, e);
        AssertCaught("boundary value xor 1");
        FuzzHooks.BlockDelete = (d, o, l) => Mutate.BlockDeleteBig(d, o, l + One);
        AssertCaught("block_delete length+1");
        FuzzHooks.BlockInsert = (d, o, ins) => Mutate.BlockInsertBig(d, o + One, ins);
        AssertCaught("block_insert offset+1");
        FuzzHooks.BlockDuplicate = (d, s, l, dst) => Mutate.BlockDuplicateBig(d, s, l, dst + One);
        AssertCaught("block_duplicate dst+1");
        FuzzHooks.SpliceHook = (a, b, ts, ds) => Mutate.SpliceBig(a, b, ts + One, ds);
        AssertCaught("splice target split+1");
        FuzzHooks.MutateSequence = (d, seed, n, m, cap) => Mutate.MutateSequenceCounted(d, seed + 1, n, m, cap);
        AssertCaught("mutate_sequence seed+1");
        FuzzHooks.CorpusAdmit = (e, t) => new Bitmap.Novelty(true, 1);
        AssertCaught("corpus_admit always admits");
        FuzzHooks.IsDuplicate = (a, b, c, d) => true;
        AssertCaught("dedup always duplicate");
        FuzzHooks.IsDuplicate = (a, b, c, d) => false;
        AssertCaught("dedup never duplicate");
        FuzzHooks.IsFailureId = s => true;
        AssertCaught("every failure id valid");
        FuzzHooks.FormatTap = (n, t, f, s) => FuzzSignature.FormatTapFailure(n, t, f, s) + " " + t;
        AssertCaught("tap line leaks the target");
        FuzzHooks.TfvValid = s => true;
        AssertCaught("tfv accepts everything");
    }

    [Fact]
    public void AShiftedStreamIsCaughtOnTheSequenceCase()
    {
        FuzzHooks.MutateSequence = (d, seed, n, m, cap) => new Mutate.CountedResult(Mutate.MutateSequence(d, new SplitMix64(seed + 1), n, m, long.MaxValue), 0);
        Assert.True(FuzzFx.AnyContains(FuzzCorpus.Evaluate(FuzzFx.LoadCases()).Problems, "fuzz/mutate_sequence/seed42_steps4"));
    }

    [Fact]
    public void EngineInvariantsAreCheckedIndependentlyOfExpectedValues()
    {
        List<object?> all = FuzzFx.LoadCases();
        Dictionary<string, object?> c = FuzzFx.FirstOf(all, "mutate_bit_flip");
        FuzzHooks.BitFlip = (d, o, n) => d.Concat(new byte[] { 0 }).ToArray();
        c["output"] = FuzzFx.BytesNode((string)Fx.Obj(c["input"])["hex"]! + "00");
        Assert.True(FuzzFx.AnyContains(FuzzFx.Problems(FuzzFx.Swapped(all, c)), "changed the length"));
        FuzzHooks.Reset();

        c = FuzzFx.FirstOf(all, "mutate_splice");
        FuzzHooks.SpliceHook = (a, b, ts, ds) => new byte[] { 1 };
        c["output"] = FuzzFx.BytesNode("01");
        Assert.True(FuzzFx.AnyContains(FuzzFx.Problems(FuzzFx.Swapped(all, c)), "prefix + suffix"));
        FuzzHooks.Reset();

        c = FuzzFx.FirstOf(all, "corpus_admit"); // the first corpus_admit case is the novel-edge one
        Assert.Equal(true, c["admitted"]);
        FuzzHooks.CorpusAdmit = (e, t) => new Bitmap.Novelty(false, 0);
        c["admitted"] = false;
        c["new_edges"] = FuzzFx.IntNode("0");
        Assert.True(FuzzFx.AnyContains(FuzzFx.Problems(FuzzFx.Swapped(all, c)), "set difference"));
        FuzzHooks.Reset();

        c = FuzzFx.FirstOf(FuzzFx.LoadCases(), "private_reporting");
        string want = (string)c["tap_line"]!;
        FuzzHooks.FormatTap = (n, t, f, s) => want + "\n";
        Assert.True(FuzzFx.AnyContains(FuzzFx.Problems(FuzzFx.Swapped(FuzzFx.LoadCases(), c)), "more than one line"));
        FuzzHooks.Reset();

        FuzzHooks.IsFailureId = s => s == "fuzz-crash";
        Assert.True(FuzzFx.AnyContains(FuzzFx.Problems(FuzzFx.LoadCases()), "rejects registered ids"));
    }

    [Fact]
    public void APayloadLeakIsRecomputedFromTheBytes()
    {
        List<object?> all = FuzzFx.LoadCases();
        Dictionary<string, object?> c = FuzzFx.FirstOf(all, "private_reporting");
        string want = (string)c["tap_line"]!;
        FuzzHooks.FormatTap = (n, t, f, s) => want;
        c["raw_payload"] = FuzzFx.BytesNode("3a8f19b2"); // the sig hash itself appears in the line
        Assert.True(FuzzFx.AnyContains(FuzzFx.Problems(FuzzFx.Swapped(all, c)), "privacy verdict wrong"));
    }

    [Fact]
    public void AThrowingEngineIsAClippedProblemNotACrash()
    {
        FuzzHooks.BitFlip = (d, o, n) => throw new InvalidOperationException("engine bug " + new string('x', 5000));
        List<string> ps = FuzzFx.Problems(FuzzFx.LoadCases());
        Assert.True(FuzzFx.AnyContains(ps, "engine bug"));
        foreach (string p in ps)
        {
            Assert.True(p.Length <= 403, "an unclipped problem of " + p.Length);
        }

        FuzzHooks.Reset();
        FuzzHooks.FormatTap = (n, t, f, s) => throw new ArgumentException("nope");
        Assert.True(FuzzFx.AnyContains(FuzzFx.Problems(FuzzFx.LoadCases()), "refused"));
    }

    [Fact]
    public void AMutatorCaseWithANonBytesOutputIsAProblem()
    {
        List<object?> all = FuzzFx.LoadCases();
        Dictionary<string, object?> c = FuzzFx.FirstOf(all, "mutate_bit_flip");
        c["output"] = FuzzFx.IntNode("1");
        Assert.True(FuzzFx.AnyContains(FuzzFx.Problems(FuzzFx.Swapped(all, c)), "output must be bytes"));
    }

    [Fact]
    public void HooksResetToProduction()
    {
        FuzzHooks.BitFlip = (d, o, n) => Array.Empty<byte>();
        FuzzHooks.Reset();
        Assert.Empty(FuzzFx.Problems(FuzzFx.LoadCases()));
    }

    // ---- caps ----------------------------------------------------------------------

    [Fact]
    public void OversizedValuesAreProblemsNotHangs()
    {
        List<object?> all = FuzzFx.LoadCases();
        Dictionary<string, object?> c = FuzzFx.FirstOf(all, "mutate_sequence");
        c["steps"] = FuzzFx.IntNode("1000000000");
        Assert.True(FuzzFx.AnyContains(FuzzFx.Problems(FuzzFx.Swapped(all, c)), "outside [0, 10000]"));
        List<object?> many = new();
        for (int i = 0; i < 400; i++)
        {
            Dictionary<string, object?> d = Fx.Obj(Fx.DeepCopy(FuzzFx.FirstOf(all, "mutate_sequence")));
            d["id"] = "seq/" + i;
            d["steps"] = FuzzFx.IntNode("10000");
            d["input"] = FuzzFx.BytesNode(string.Concat(Enumerable.Repeat("00", 10_000)));
            many.Add(d);
        }

        Assert.True(FuzzFx.AnyContains(FuzzCorpus.Evaluate(many, -1).Problems, "work budget"));
    }

    [Fact]
    public void TheAggregateSequenceBudgetChargesActualWorkNotStepsTimesInitialLength()
    {
        List<object?> all = FuzzFx.LoadCases();
        List<object?> many = new();
        long real = 0;
        long estimate = 0;
        const int steps = 10_000;
        for (int i = 0; real <= FuzzCorpus.MaxAggregateSequenceWork; i++)
        {
            real += Mutate.MutateSequenceCounted(new byte[2], (ulong)i, steps, Mutate.MaxInputBytes, long.MaxValue).Work;
            estimate += steps * 2L;
            Dictionary<string, object?> d = Fx.Obj(Fx.DeepCopy(FuzzFx.FirstOf(all, "mutate_sequence")));
            d["id"] = "seq/" + i;
            d["steps"] = FuzzFx.IntNode(steps.ToString());
            d["seed"] = FuzzFx.IntNode(i.ToString());
            d["input"] = FuzzFx.BytesNode("0000");
            d["output"] = FuzzFx.BytesNode("00");
            many.Add(d);
        }

        Assert.True(estimate < FuzzCorpus.MaxAggregateSequenceWork, "the old steps x length estimate would have allowed this corpus: " + estimate);
        string last = "seq/" + (many.Count - 1) + ":";
        Assert.Contains(FuzzCorpus.Evaluate(many, -1).Problems, p => p.StartsWith(last, StringComparison.Ordinal) && p.Contains("work budget"));
    }

    [Fact]
    public void CasesThatHitTheCapAreChargedAndLaterOnesFailWithoutRunning()
    {
        List<object?> all = FuzzFx.LoadCases();
        List<object?> many = new();
        for (int i = 0; i < 300; i++)
        {
            Dictionary<string, object?> d = Fx.Obj(Fx.DeepCopy(FuzzFx.FirstOf(all, "mutate_sequence")));
            d["id"] = "seq/" + i;
            d["steps"] = FuzzFx.IntNode("10000");
            d["seed"] = FuzzFx.IntNode(i.ToString());
            d["input"] = FuzzFx.BytesNode(string.Concat(Enumerable.Repeat("00", 10_000)));
            many.Add(d);
        }

        long total = 0;
        int calls = 0;
        FuzzHooks.MutateSequence = (d, seed, n, m, cap) =>
        {
            calls++;
            try
            {
                Mutate.CountedResult c = Mutate.MutateSequenceCounted(d, seed, n, m, cap);
                total += c.Work;
                return c;
            }
            catch (Mutate.WorkExhausted e)
            {
                total += e.Work;
                throw;
            }
        };
        List<string> problems = FuzzFx.Problems(many);
        Assert.True(total <= FuzzCorpus.MaxAggregateSequenceWork, "actual work " + total + " exceeded the aggregate cap");
        Assert.True(calls < 50, "once the cap is breached no later case runs, but " + calls + " of 300 did");
        Assert.Equal(300 - (calls - 1), problems.Count(p => p.Contains("work budget")));
    }

    [Fact]
    public void TooManyCasesAreCappedAndReported()
    {
        List<object?> all = FuzzFx.LoadCases();
        List<object?> many = new();
        for (int i = 0; i < 10_001; i++)
        {
            Dictionary<string, object?> d = Fx.Obj(Fx.DeepCopy(FuzzFx.FirstOf(all, "failure_id_valid")));
            d["id"] = "f/" + i;
            many.Add(d);
        }

        Assert.True(FuzzFx.AnyContains(FuzzCorpus.Evaluate(many, -1).Problems, "exceeds the cap"));
    }

    [Fact]
    public void ProblemTextIsClipped()
    {
        List<object?> all = FuzzFx.LoadCases();
        Dictionary<string, object?> c = FuzzFx.FirstOf(all, "mutate_bit_flip");
        c["id"] = new string('i', 5000);
        c["output"] = FuzzFx.BytesNode(string.Concat(Enumerable.Repeat("ab", 5000)));
        foreach (string p in FuzzFx.Problems(FuzzFx.Swapped(all, c)))
        {
            Assert.True(p.Length <= 403, p.Length + " chars");
        }

        List<object?> bogus = new(all) { new List<object?> { new string('z', 5000) } };
        foreach (string p in FuzzFx.Problems(bogus))
        {
            Assert.True(p.Length <= 403, p.Length + " chars");
        }
    }

    [Fact]
    public void ParseCorpusIsStrict()
    {
        string deep = new string('[', 100) + new string(']', 100);
        Dictionary<string, string> bad = new()
        {
            ["not json"] = "nope",
            ["empty"] = string.Empty,
            ["trailing data"] = "{\"schema_version\":1,\"cases\":[]} {}",
            ["wrong version"] = "{\"schema_version\":2,\"cases\":[]}",
            ["string version"] = "{\"schema_version\":\"1\",\"cases\":[]}",
            ["missing cases"] = "{\"schema_version\":1}",
            ["extra key"] = "{\"schema_version\":1,\"cases\":[],\"x\":1}",
            ["cases not array"] = "{\"schema_version\":1,\"cases\":{}}",
            ["not an object"] = "[1]",
            ["duplicate key"] = "{\"schema_version\":1,\"cases\":[],\"cases\":[]}",
            ["nested duplicate"] = "{\"schema_version\":1,\"cases\":[{\"id\":\"a\",\"id\":\"b\"}]}",
            ["truncated"] = "{\"schema_version\":1,\"cases\":[",
            ["deep nesting"] = "{\"schema_version\":1,\"cases\":" + deep + "}",
            ["oversized"] = "{\"schema_version\":1,\"cases\":[],\"pad\":\"" + new string('a', StrictJson.MaxBytes) + "\"}",
        };
        foreach (KeyValuePair<string, string> e in bad)
        {
            Assert.True(Record.Exception(() => FuzzCorpus.Parse(Fx.Utf8(e.Value))) is JsonException, e.Key + ": accepted");
        }

        Assert.Equal(5, FuzzCorpus.Parse(Fx.Utf8("{\"schema_version\":1,\"cases\":[1,\"a\",null,true,{\"x\":[]}]}")).Count);
    }

    [Fact]
    public void AHandBuiltTooDeepValueIsAProblemNotAStackOverflow()
    {
        List<object?> all = FuzzFx.LoadCases();
        Dictionary<string, object?> c = FuzzFx.FirstOf(all, "mutate_bit_flip");
        object nested = new List<object?>();
        for (int i = 0; i < 100_000; i++)
        {
            nested = new List<object?> { nested };
        }

        c["count"] = nested;
        Assert.True(FuzzFx.AnyContains(FuzzFx.Problems(FuzzFx.Swapped(all, c)), "could not be evaluated"));
    }
}
