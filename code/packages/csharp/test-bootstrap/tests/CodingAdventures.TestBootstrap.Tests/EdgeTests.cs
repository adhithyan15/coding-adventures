using System.Numerics;
using System.Text;
using CodingAdventures.TestBootstrap.CorpusJson;
using CodingAdventures.TestBootstrap.Pbt;
using CodingAdventures.TestBootstrap.Prng;
using CodingAdventures.TestBootstrap.Tfv;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>Branches the corpus cannot reach: budgets, strict-JSON syntax, hand-built bad cases.</summary>
public sealed class EdgeTests : HookedTest
{
    private static BigInteger Big(long v) => new(v);

    private static object? Parse(string json) => StrictJson.Parse(Encoding.UTF8.GetBytes(json));

    // ---- budgets ----------------------------------------------------------------------------

    [Fact]
    public void ABudgetSpentInsideTheElementPhaseStopsFurtherWork()
    {
        // 74 elements, maxSteps 1 => budget 80. Probe 1 + bisection 2 + deletions 74 = 77 attempts,
        // so the element phase starts with 3 left: element 0 uses two, element 1 hits the wall
        // mid-walk (bisection candidate charged, unit candidate refused).
        const int n = 74;
        List<BigInteger> start = Enumerable.Repeat(Big(5), n).ToList();
        int calls = 0;
        Shrunk<List<BigInteger>> r = Shrink.ShrinkList(start, BigInteger.Zero, l => { calls++; return !(l.Count == n && l.All(v => v == Big(5))); }, 1);
        Assert.Equal(0, r.Steps);
        Assert.Equal(n, r.Value.Count);
        Assert.Equal(80, Limits.AttemptBudget(1));
        Assert.True(calls <= 80 + 1); // + the initial check in ShrinkList
    }

    [Fact]
    public void ASpinningCustomShrinkerIsCutOffByTheRunnerBudget()
    {
        int evaluations = 0;
        Shrinker<BigInteger> spinner = (value, falsifies, max) =>
        {
            for (int i = 0; i < 1_000_000; i++)
            {
                falsifies(value);
            }

            return new Shrunk<BigInteger>(value, 0);
        };
        Verdict v = Runner.Check(
            1,
            Gens.Integer(Big(0), Big(100)),
            x => { evaluations++; return x < 0; },
            new Options(5, 10, CiMode.Off),
            spinner);
        Assert.Equal(FailureIds.Counterexample, v.FailureId);
        Assert.True(evaluations <= 1 + Limits.AttemptBudget(10) + 1, "evaluations " + evaluations);
    }

    [Fact]
    public void FilterAcceptsAndU64Samples()
    {
        Gen<BigInteger> g = Gens.Filter(Gens.Integer(Big(0), Big(9)), x => x >= 5);
        SplitMix64 rng = new(1);
        for (int i = 0; i < 20; i++)
        {
            Assert.True(g.Sample(rng) >= 5);
        }

        SplitMix64 a = new(7);
        SplitMix64 b = new(7);
        Assert.Equal(b.NextU64(), Gens.U64().Sample(a));
    }

    [Fact]
    public void ListOfShrinksElementsThroughTheElementGenerator()
    {
        Gen<List<BigInteger>> g = Gens.ListOf(Gens.Integer(Big(10), Big(20)), 0, 5);
        Shrunk<List<BigInteger>> r = g.Shrink(new List<BigInteger> { Big(18), Big(15) }, l => l.Count >= 1, 1000);
        Assert.Single(r.Value);
        Assert.Equal(Big(10), r.Value[0]); // integer shrinks to its in-range minimum, never to 0
    }

    private sealed class HugeTuple : System.Runtime.CompilerServices.ITuple
    {
        public int Length => int.MaxValue;

        public object? this[int index] => index;
    }

    [Fact]
    public void CanonStopsHugeArraysAndTuples()
    {
        string arr = Canon.Render(new int[50_000]);
        Assert.EndsWith("...]", arr);
        string tup = Canon.Render(new HugeTuple());
        Assert.EndsWith("...}", tup);
        Assert.True(tup.Length < 100_000);
    }

    [Fact]
    public void TfvFieldNamesAreCheckedPastTheFirstCharacter()
    {
        Assert.False(TfvValidator.IsValid("tfv 1\nr R1 Kab-c N\n"));
        Assert.False(TfvValidator.IsValid("tfv 1\nr R1 KaB N\n"));
        Assert.True(TfvValidator.IsValid("tfv 1\nr R1 Ka_b9 N\n"));
    }

    // ---- strict JSON syntax --------------------------------------------------------------------

    [Fact]
    public void ParsesEveryValueKind()
    {
        Dictionary<string, object?> o = (Dictionary<string, object?>)Parse(
            " {\"a\" : [1, -2.5e3, true, false, null, \"s\"], \"b\": {}, \"c\": []} ")!;
        List<object?> a = (List<object?>)o["a"]!;
        Assert.Equal(new JsonNumber("1"), a[0]);
        Assert.Equal(new JsonNumber("-2.5e3"), a[1]);
        Assert.Equal(true, a[2]);
        Assert.Equal(false, a[3]);
        Assert.Null(a[4]);
        Assert.Equal("s", a[5]);
        Assert.Empty((Dictionary<string, object?>)o["b"]!);
        Assert.Empty((List<object?>)o["c"]!);
        Assert.True((bool)Parse("true")!);
        Assert.Null(Parse("null"));
    }

    [Fact]
    public void StringEscapesDecodeExactly()
    {
        Assert.Equal("\"\\/\b\f\n\r\t", Parse("\"\\\"\\\\\\/\\b\\f\\n\\r\\t\""));
        Assert.Equal("A\u00e9\u20ac", Parse("\"\\u0041\\u00e9\\u20AC\""));
        Assert.Equal("\ud83d\ude00", Parse("\"\\ud83d\\ude00\""));
        Assert.Equal("caf\u00e9", Parse("\"caf\u00e9\""));
    }

    [Fact]
    public void RefusesBadSyntax()
    {
        string[] bad =
        {
            "\"abc", "\"a\\", "\"a\\x\"", "\"\\u12\"", "\"\\u12g4\"", "\"\\uzzzz\"", "\"tab\there\"", "\"nl\nhere\"",
            "tru", "nul", "falsy", "[", "[1", "[1,", "{", "{\"a\"", "{\"a\":", "{\"a\":1", "{'a':1}", "{\"a\":1 \"b\":2}",
            "[1;2]", "@", "\u00e9", string.Empty, "  ", "-", "1e5e5", "1.2.3", "0.", "[01]", "{\"a\":01}",
        };
        foreach (string text in bad)
        {
            Assert.True(Record.Exception(() => Parse(text)) is JsonException, "accepted: " + text.Replace("\n", "\\n"));
        }

        Assert.Throws<ArgumentNullException>(() => StrictJson.Parse(null!));
    }

    // ---- hand-built bad cases through the corpus runner ----------------------------------------------

    private static (List<object?> Cases, Dictionary<string, object?> Case, string Id) Victim(string op)
    {
        List<object?> cases = Fx.CopyCases(Fx.LoadCases());
        Dictionary<string, object?> c = cases.Select(Fx.Obj).First(x => (string)x["op"]! == op);
        return (cases, c, (string)c["id"]!);
    }

    private static Dictionary<string, object?> IntNode(long v) => new() { ["kind"] = "int", ["value"] = v.ToString() };

    private static bool Named(List<object?> cases, string id, string fragment) =>
        Corpus.Evaluate(cases).Problems.Any(p => p.StartsWith(id + ":", StringComparison.Ordinal) && p.Contains(fragment));

    [Fact]
    public void UnknownPropertiesTypesAndWrongKindsAreProblems()
    {
        foreach (string op in new[] { "shrink_int", "shrink_refinement", "shrink_list", "shrink_bytes", "replay_consistency" })
        {
            (List<object?> cases, Dictionary<string, object?> c, string id) = Victim(op);
            c["property"] = "no_such_property";
            Assert.True(Named(cases, id, "unknown property"), op);
            c["property"] = true;
            Assert.True(Named(cases, id, "unknown property"), op);
        }

        {
            (List<object?> cases, Dictionary<string, object?> c, string id) = Victim("gen_primitive");
            c["type"] = "float128";
            Assert.True(Named(cases, id, "could not be evaluated"));
            c["type"] = true;
            Assert.True(Named(cases, id, "type must be a string"));
            c["type"] = "bool";
            c["seed"] = true;
            Assert.True(Named(cases, id, "seed must be an int"));
        }

        {
            (List<object?> cases, Dictionary<string, object?> c, string id) = Victim("gen_bounded");
            c["min"] = IntNode(5);
            c["max"] = IntNode(1);
            Assert.True(Named(cases, id, "could not be evaluated"));
            c["min"] = "x";
            Assert.True(Named(cases, id, "min must be an int"));
        }

        {
            (List<object?> cases, Dictionary<string, object?> c, string id) = Victim("gen_bytes");
            c["min_len"] = IntNode(9);
            c["max_len"] = IntNode(2);
            Assert.True(Named(cases, id, "could not be evaluated"));
        }

        {
            (List<object?> cases, Dictionary<string, object?> c, string id) = Victim("shrink_list");
            c["initial"] = IntNode(3);
            Assert.True(Named(cases, id, "initial must be a list"));
            c["initial"] = new List<object?> { true };
            Assert.True(Named(cases, id, "element must be an int"));
        }

        {
            (List<object?> cases, Dictionary<string, object?> c, string id) = Victim("shrink_bytes");
            c["initial"] = IntNode(3);
            Assert.True(Named(cases, id, "initial must be bytes"));
        }

        {
            (List<object?> cases, Dictionary<string, object?> c, string id) = Victim("filter_exhausted");
            c["max_retries"] = IntNode(0);
            Assert.True(Named(cases, id, "max_retries must be at least 1"));
            c["max_retries"] = IntNode(-1);
            Assert.True(Named(cases, id, "outside"));
        }

        {
            (List<object?> cases, Dictionary<string, object?> c, string id) = Victim("corpus_reject");
            c["text"] = true;
            Assert.True(Named(cases, id, "text must be a string"));
        }

        {
            (List<object?> cases, Dictionary<string, object?> c, string id) = Victim("failure_id_valid");
            c["input"] = true; // not a string: never a registered id, so the case's `valid` must be false
            Assert.True(Named(cases, id, "membership disagrees"));
        }
    }

    [Fact]
    public void ShrinkOfAValueThatAlreadyHoldsIsAProblemNotACrash()
    {
        (List<object?> cases, Dictionary<string, object?> c, string id) = Victim("shrink_int");
        c["initial"] = IntNode(5); // 5 satisfies "x <= 100": the engine throws NotFalsifyingException
        Assert.True(Named(cases, id, "could not be evaluated"));
        (List<object?> cases2, Dictionary<string, object?> c2, string id2) = Victim("replay_consistency");
        c2["property"] = "int_in_range_neg10_50";
        c2["seed"] = IntNode(1);
        Assert.Contains(Corpus.Evaluate(cases2).Problems, p => p.StartsWith(id2 + ":", StringComparison.Ordinal));
    }

    [Fact]
    public void EveryFieldReplacedByEveryWrongKindNeverCrashesAndNamesTheCase()
    {
        List<object?> pristine = Fx.LoadCases();
        object?[] wrong = { true, "x", IntNode(-1), IntNode(10_001), new List<object?>(), null, new Dictionary<string, object?>() };
        for (int i = 0; i < pristine.Count; i++)
        {
            Dictionary<string, object?> original = Fx.Obj(pristine[i]);
            string id = (string)original["id"]!;
            foreach (string field in original.Keys.Where(k => k != "id" && k != "op"))
            {
                foreach (object? replacement in wrong)
                {
                    List<object?> cases = Fx.CopyCases(pristine);
                    Fx.Obj(cases[i])[field] = Fx.DeepCopy(replacement);
                    CorpusReport report = Corpus.Evaluate(cases);
                    foreach (string p in report.Problems)
                    {
                        // A broken case may also trip the corpus-wide gates (missing op, case count).
                        Assert.True(
                            p.StartsWith(id + ":", StringComparison.Ordinal) || p.StartsWith("pbt-v1:", StringComparison.Ordinal),
                            id + "/" + field + ": " + p);
                    }
                }
            }
        }
    }

    [Fact]
    public void TooManyCasesAreCapped()
    {
        List<object?> many = new();
        for (int i = 0; i < 10_001; i++)
        {
            many.Add(new Dictionary<string, object?> { ["id"] = "c" + i, ["op"] = "nope" });
        }

        CorpusReport r = Corpus.Evaluate(many, -1);
        Assert.Contains(r.Problems, p => p.Contains("exceeds the cap"));
    }

    [Fact]
    public void NegativeSeedsWrapModuloTwoToTheSixtyFour()
    {
        // The reference masks seeds with 2^64 - 1, so seed -1 behaves as 2^64 - 1.
        List<object?> cases = Fx.CopyCases(Fx.LoadCases());
        Dictionary<string, object?> c = cases.Select(Fx.Obj).First(x => (string)x["op"]! == "gen_primitive" && (string)x["type"]! == "u64");
        string id = (string)c["id"]!;
        c["seed"] = IntNode(-1);
        List<object> want = Gens.GenPrimitive(ulong.MaxValue, "u64", 4);
        c["outputs"] = want.Select(w => (object?)new Dictionary<string, object?> { ["kind"] = "int", ["value"] = w.ToString() }).ToList();
        Assert.DoesNotContain(Corpus.Evaluate(cases).Problems, p => p.StartsWith(id + ":", StringComparison.Ordinal));
        // 2^64 + 5 wraps to 5.
        c["seed"] = new Dictionary<string, object?> { ["kind"] = "int", ["value"] = ((BigInteger.One << 64) + 5).ToString() };
        c["outputs"] = Gens.GenPrimitive(5, "u64", 4).Select(w => (object?)new Dictionary<string, object?> { ["kind"] = "int", ["value"] = w.ToString() }).ToList();
        Assert.DoesNotContain(Corpus.Evaluate(cases).Problems, p => p.StartsWith(id + ":", StringComparison.Ordinal));
    }
}
