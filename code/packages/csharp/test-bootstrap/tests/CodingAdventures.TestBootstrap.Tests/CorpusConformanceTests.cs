using System.Numerics;
using CodingAdventures.TestBootstrap.CorpusJson;
using CodingAdventures.TestBootstrap.Pbt;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>
/// Replays the whole committed pbt-v1 corpus and carries its negative controls
/// (VER05g section 6). The fixture is read from its committed path, never a copy.
/// </summary>
public sealed class CorpusConformanceTests : HookedTest
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

    private static void CorruptFirst(Dictionary<string, object?> c, string field)
    {
        List<object?> list = (List<object?>)c[field]!;
        list[0] = CorruptValue(list[0]);
    }

    /// <summary>Each variant is one corruption of the case; every one must be noticed.</summary>
    private static List<Action<Dictionary<string, object?>>> Corruptions(string op) => op switch
    {
        "gen_primitive" or "gen_bounded" or "gen_bytes" => new() { c => CorruptFirst(c, "outputs") },
        "shrink_int" or "shrink_refinement" or "shrink_bytes" => new()
        {
            c => c["expected_minimal"] = CorruptValue(c["expected_minimal"]),
            c => c["expected_steps"] = CorruptValue(c["expected_steps"]),
        },
        "shrink_list" => new()
        {
            c => CorruptFirst(c, "expected_minimal"),
            c => c["expected_steps"] = CorruptValue(c["expected_steps"]),
        },
        "replay_consistency" => new()
        {
            c => c["initial_fail"] = CorruptValue(c["initial_fail"]),
            c => c["expected_minimal"] = CorruptValue(c["expected_minimal"]),
        },
        "filter_exhausted" => new() { c => c["failure_id"] = "property-panic", c => c["verdict"] = "pass" },
        "failure_id_valid" => new() { c => c["valid"] = CorruptValue(c["valid"]) },
        "corpus_reject" => new() { c => c["text"] = "tfv 1\nr R1 Kop S706274\n" },
        _ => throw new InvalidOperationException("no corruption defined for op " + op),
    };

    // ---- the positive run --------------------------------------------------------

    [Fact]
    public void CorpusConformsToAllTwentyFiveCases()
    {
        CorpusReport report = Corpus.Evaluate(Fx.LoadCases());
        Assert.Empty(report.Problems);
        Assert.Equal(25, report.Evaluated);
        Assert.Equal(25, Corpus.Pbtv1CaseCount);
    }

    [Fact]
    public void EveryOpIsPresentInTheFixture()
    {
        SortedSet<string> ops = new(Fx.LoadCases().Select(c => (string)Fx.Obj(c)["op"]!));
        Assert.Equal(11, ops.Count);
    }

    // ---- negative controls ---------------------------------------------------------

    /// <summary>
    /// Per op, per case: corrupt the expected value of EVERY real case in every way its op
    /// allows and require the run to fail and name exactly that case. A negative control on a
    /// single case proves only that case's path.
    /// </summary>
    [Fact]
    public void CorruptingAnyCaseMakesTheRunFailAndNamesThatCase()
    {
        List<object?> pristine = Fx.LoadCases();
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
                CorpusReport report = Corpus.Evaluate(cases);
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
        List<object?> pristine = Fx.LoadCases();
        for (int i = 0; i < pristine.Count; i++)
        {
            List<object?> cases = new(pristine);
            cases.RemoveAt(i);
            CorpusReport report = Corpus.Evaluate(cases);
            Assert.NotEmpty(report.Problems);
            Assert.Contains("expected 25 cases, got 24", report.Problems[0]);
        }

        Assert.NotEmpty(Corpus.Evaluate(new List<object?>()).Problems);
        Assert.NotEmpty(Corpus.Evaluate(pristine.Take(8).ToList()).Problems);
    }

    [Fact]
    public void ATruncatedDocumentFailsToParse()
    {
        byte[] data = File.ReadAllBytes(Fx.FixturePath());
        byte[] half = data.AsSpan(0, data.Length / 2).ToArray();
        Assert.Throws<JsonException>(() => Corpus.Parse(half));
        Assert.Throws<JsonException>(() => Corpus.Parse(Array.Empty<byte>()));
    }

    [Fact]
    public void DuplicatedCaseIdsAreAProblem()
    {
        List<object?> cases = Fx.CopyCases(Fx.LoadCases());
        cases[1] = Fx.DeepCopy(cases[0]);
        Assert.Contains("duplicate or missing case id", Corpus.Evaluate(cases).Problems);
    }

    private static bool ProblemFor(List<object?> cases, string id, string fragment) =>
        Corpus.Evaluate(cases).Problems.Any(p => p.StartsWith(id + ":", StringComparison.Ordinal) && p.Contains(fragment));

    [Fact]
    public void AnUnknownOpAndABadFieldSetAreProblems()
    {
        List<object?> cases = Fx.CopyCases(Fx.LoadCases());
        Dictionary<string, object?> first = Fx.Obj(cases[0]);
        string id = (string)first["id"]!;
        first["op"] = "gen_everything";
        Assert.True(ProblemFor(cases, id, "unknown op"));
        first["op"] = "gen_primitive";
        first["surprise"] = "field";
        Assert.True(ProblemFor(cases, id, "fields"));
        first.Remove("surprise");
        first.Remove("count");
        Assert.True(ProblemFor(cases, id, "fields"));
    }

    [Fact]
    public void ANonObjectCaseAndANonStringIdAreProblemsNotCrashes()
    {
        List<object?> cases = new(Fx.LoadCases());
        cases[0] = "not an object";
        Assert.Contains(Corpus.Evaluate(cases).Problems, p => p.Contains("case is not an object"));
        cases[0] = new Dictionary<string, object?> { ["id"] = new JsonNumber("7"), ["op"] = "x" };
        Assert.Contains(Corpus.Evaluate(cases).Problems, p => p.Contains("missing a string id"));
    }

    [Fact]
    public void CoverageGatesNeedEveryOpAndEveryFailureId()
    {
        List<object?> only = Fx.LoadCases()
            .Where(c =>
            {
                Dictionary<string, object?> m = Fx.Obj(c);
                return (string)m["op"]! == "failure_id_valid"
                    && (string)m["input"]! != "unregistered-id"
                    && (string)m["input"]! != "property-panic";
            })
            .ToList();
        List<string> problems = Corpus.Evaluate(only, -1).Problems.ToList();
        Assert.Contains(problems, p => p.Contains("operations never exercised"));
        Assert.Contains(problems, p => p.Contains("registered failure ids missing"));
        Assert.Contains(problems, p => p.Contains("unregistered-id rejection"));
    }

    [Fact]
    public void AnUndecodableCaseIsAProblemNamingTheCase()
    {
        List<object?> cases = Fx.CopyCases(Fx.LoadCases());
        Dictionary<string, object?> first = Fx.Obj(cases[0]);
        string id = (string)first["id"]!;
        first["count"] = new Dictionary<string, object?> { ["kind"] = "int", ["value"] = "007" };
        Assert.True(ProblemFor(cases, id, "non-canonical int"));
        first["count"] = new Dictionary<string, object?> { ["kind"] = "int", ["value"] = "99999" };
        Assert.True(ProblemFor(cases, id, "outside"));
        first["count"] = new Dictionary<string, object?> { ["kind"] = "int", ["value"] = "3" };
        Assert.True(ProblemFor(cases, id, "exactly count"));
    }

    // ---- engine sabotage ------------------------------------------------------------

    [Fact]
    public void SabotagedBisectionIsCaught()
    {
        Hooks.BisectCandidate = (current, diff) => current - BigInteger.Max(diff >> 2, BigInteger.One);
        Assert.NotEmpty(Corpus.Evaluate(Fx.LoadCases()).Problems);
    }

    [Fact]
    public void SabotagedUnitStepIsCaught()
    {
        Hooks.UnitCandidate = (current, diff) => current - (2 * diff.Sign);
        Assert.NotEmpty(Corpus.Evaluate(Fx.LoadCases()).Problems);
    }

    [Fact]
    public void SkippingTheDeletionPhaseIsCaught()
    {
        Hooks.SkipDeletionPhase = true;
        List<string> problems = Corpus.Evaluate(Fx.LoadCases()).Problems.ToList();
        Assert.NotEmpty(problems);
        Assert.Contains(problems, p => p.StartsWith("pbt/shrink_", StringComparison.Ordinal));
    }

    [Fact]
    public void AShiftedRngIsCaught()
    {
        Hooks.NewRng = seed => new SplitMix64(seed + 1);
        List<string> problems = Corpus.Evaluate(Fx.LoadCases()).Problems.ToList();
        Assert.Contains(problems, p => p.StartsWith("pbt/gen_", StringComparison.Ordinal));
        Assert.Contains(problems, p => p.StartsWith("pbt/replay/", StringComparison.Ordinal));
    }

    [Fact]
    public void HooksResetToProduction()
    {
        Hooks.SkipDeletionPhase = true;
        Hooks.Reset();
        Assert.Empty(Corpus.Evaluate(Fx.LoadCases()).Problems);
    }

    // ---- strict reader ------------------------------------------------------------------

    [Fact]
    public void ParseCorpusIsStrict()
    {
        string deep = new string('[', 100) + new string(']', 100);
        Dictionary<string, string> bad = new()
        {
            ["not json"] = "nope",
            ["empty"] = "",
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
            ["unterminated obj"] = "{\"schema_version\":1,\"cases\":[{\"id\":\"a\"",
            ["mismatched close"] = "{\"schema_version\":1,\"cases\":[}",
            ["oversized"] = "{\"schema_version\":1,\"cases\":[],\"pad\":\"" + new string('a', StrictJson.MaxBytes) + "\"}",
        };
        foreach (KeyValuePair<string, string> e in bad)
        {
            Assert.True(
                Record.Exception(() => Corpus.Parse(Fx.Utf8(e.Value))) is JsonException,
                e.Key + ": accepted");
        }

        List<object?> cases = Corpus.Parse(Fx.Utf8("{\"schema_version\":1,\"cases\":[1,\"a\",null,true,{\"x\":[]}]}"));
        Assert.Equal(5, cases.Count);
        Assert.NotNull(cases[0]);
    }
}
