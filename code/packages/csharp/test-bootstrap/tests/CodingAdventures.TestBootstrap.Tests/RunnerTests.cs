using System.Numerics;
using CodingAdventures.TestBootstrap.CorpusJson;
using CodingAdventures.TestBootstrap.Pbt;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>Verdict honesty: no failure ever becomes a pass, and every call returns a verdict.</summary>
public sealed class RunnerTests : HookedTest
{
    private static readonly Options Local = new(100, 1000, CiMode.Off);

    private static BigInteger Big(long v) => new(v);

    private static Gen<BigInteger> Ints => Gens.Integer(Big(0), Big(1000));

    [Fact]
    public void AHoldingPropertyPasses()
    {
        Verdict v = Runner.Check(1, Ints, x => x >= 0, Local);
        Assert.True(v.Passed);
        Assert.False(v.Failed);
        Assert.Equal(100, v.CasesRun);
        Assert.Null(v.FailureId);
        v.AssertPassed();
    }

    [Fact]
    public void AFalsifiablePropertyIsNeverAPass()
    {
        for (ulong seed = 0; seed < 25; seed++)
        {
            Verdict v = Runner.Check(seed, Ints, x => x < 500, new Options(1000, 1000, CiMode.Off));
            Assert.True(v.Failed);
            Assert.Equal(FailureIds.Counterexample, v.FailureId);
            Assert.Equal(Big(500), v.Minimal); // shrinks to the threshold
            Assert.NotNull(v.Counterexample);
            Assert.Equal(seed, v.Seed);
            Assert.Contains("seed=" + seed, v.Message);
            PropertyFailedException e = Assert.Throws<PropertyFailedException>(() => v.AssertPassed());
            Assert.Equal(FailureIds.Counterexample, e.FailureId);
            Assert.StartsWith("[property-counterexample]", e.Message, StringComparison.Ordinal);
        }
    }

    [Fact]
    public void ZeroCasesIsNotAPass()
    {
        foreach (CiMode mode in Enum.GetValues<CiMode>())
        {
            Verdict v = Runner.Check(1, Ints, _ => true, new Options(0, 100, mode));
            Assert.True(v.Failed);
            Assert.Equal(FailureIds.Exhausted, v.FailureId);
            Assert.Equal(0, v.CasesRun);
        }
    }

    [Fact]
    public void InvalidOptionsProduceNoVerdictAtAll()
    {
        Assert.Throws<ArgumentException>(() => Runner.Check(1, Ints, _ => true, new Options(-1, 10, CiMode.On)));
        Assert.Throws<ArgumentException>(() => Runner.Check(1, Ints, _ => true, new Options(Limits.MaxCases + 1, 10, CiMode.On)));
        Assert.Throws<ArgumentException>(() => Runner.Check(1, Ints, _ => true, new Options(10, -1, CiMode.On)));
        Assert.Throws<ArgumentException>(() => Runner.Check(1, Ints, _ => true, new Options(10, Limits.MaxShrinkSteps + 1, CiMode.On)));
        Assert.Throws<ArgumentNullException>(() => Runner.Check<BigInteger>(1, Ints, null!, Local));
        Assert.Throws<ArgumentNullException>(() => Runner.Check(1, null!, (BigInteger _) => true, Local));
    }

    [Fact]
    public void NullOptionsMeanRedactedDefaults()
    {
        Verdict v = Runner.Check(1, Ints, x => x < 10, null);
        Assert.Equal(FailureIds.Counterexample, v.FailureId);
        Assert.Null(v.Seed);
    }

    [Fact]
    public void APropertyThatThrowsIsAPanicVerdict()
    {
        Verdict local = Runner.Check(3, Ints, _ => throw new InvalidOperationException("boom"), Local);
        Assert.Equal(FailureIds.Panic, local.FailureId);
        Assert.Contains("boom", local.Message);
        Assert.Equal(3UL, local.Seed);

        Verdict redacted = Runner.Check(3, Ints, _ => throw new InvalidOperationException("secret-witness"), new Options(10, 10, CiMode.On));
        Assert.Equal(FailureIds.Panic, redacted.FailureId);
        Assert.DoesNotContain("secret-witness", redacted.Message);
        Assert.Contains("InvalidOperationException", redacted.Message);
        Assert.Null(redacted.Seed);
    }

    [Fact]
    public void GeneratorFailuresAreExhausted()
    {
        Gen<int> never = Gens.Filter(Gens.Constant(1), _ => false, 3);
        Verdict filter = Runner.Check(1, never, _ => true, Local);
        Assert.Equal(FailureIds.Exhausted, filter.FailureId);
        Assert.Contains("filter precondition", filter.Message);

        Gen<int> broken = Gen<int>.Of(_ => throw new InvalidOperationException("gen-bug"), null);
        Verdict open = Runner.Check(1, broken, _ => true, Local);
        Assert.Equal(FailureIds.Exhausted, open.FailureId);
        Assert.Contains("gen-bug", open.Message);
        Verdict redacted = Runner.Check(1, broken, _ => true, new Options(5, 5, CiMode.On));
        Assert.DoesNotContain("gen-bug", redacted.Message);
        Assert.Contains("InvalidOperationException", redacted.Message);
    }

    [Fact]
    public void ShrinkingFindsTheMinimalWitness()
    {
        Verdict v = Runner.Check(7, Gens.ListOf(Gens.U8(), 0, 10), l => l.Sum(b => b) <= 20, new Options(500, 1000, CiMode.Off));
        Assert.Equal(FailureIds.Counterexample, v.FailureId);
        List<byte> minimal = (List<byte>)v.Minimal!;
        Assert.True(minimal.Sum(b => b) > 20);
        Assert.True(v.ShrinkSteps > 0);
        Assert.Contains("minimal=[", v.Message);
    }

    [Fact]
    public void ALyingShrinkerCannotChangeTheVerdict()
    {
        // Returns a value that does NOT fail, and claims many steps.
        Shrinker<BigInteger> liar = (value, falsifies, max) => new Shrunk<BigInteger>(Big(0), 5);
        Verdict v = Runner.Check(1, Ints, x => x < 500, Local, liar);
        Assert.Equal(FailureIds.Counterexample, v.FailureId);
        Assert.Equal(v.Counterexample, v.Minimal);
        Assert.Equal(0, v.ShrinkSteps);
        Assert.True((BigInteger)v.Minimal! >= 500);

        // Nonsensical step counts.
        Shrinker<BigInteger> negative = (value, falsifies, max) => new Shrunk<BigInteger>(value, -1);
        Assert.Equal(0, Runner.Check(1, Ints, x => x < 500, Local, negative).ShrinkSteps);
        Shrinker<BigInteger> huge = (value, falsifies, max) => new Shrunk<BigInteger>(value, max + 1);
        Assert.Equal(0, Runner.Check(1, Ints, x => x < 500, Local, huge).ShrinkSteps);

        // A shrinker that throws.
        Shrinker<BigInteger> thrower = (value, falsifies, max) => throw new InvalidOperationException("bad shrinker");
        Verdict t = Runner.Check(1, Ints, x => x < 500, Local, thrower);
        Assert.Equal(FailureIds.Counterexample, t.FailureId);
        Assert.Equal(t.Counterexample, t.Minimal);
    }

    [Fact]
    public void APropertyThatThrowsOnTheShrunkValueReportsTheOriginal()
    {
        // Falsified by x >= 500 with a normal property, but throws for the shrink target.
        BigInteger? original = null;
        Verdict v = Runner.Check(
            1,
            Ints,
            x =>
            {
                if (original != null && x == 500)
                {
                    throw new InvalidOperationException("only on the minimum");
                }

                if (x >= 500)
                {
                    original ??= x;
                    return false;
                }

                return true;
            },
            Local);
        Assert.Equal(FailureIds.Counterexample, v.FailureId);
        Assert.True((BigInteger)v.Minimal! > 500, "a candidate the property throws on is never accepted as smaller");
    }

    [Fact]
    public void CiMessagesCarryNoWitnessAndNoSeed()
    {
        foreach (CiMode mode in new[] { CiMode.Unset, CiMode.On })
        {
            Verdict v = Runner.Check(123456789, Gens.Constant(Big(424242)), x => x < 0, new Options(10, 100, mode));
            Assert.Equal(FailureIds.Counterexample, v.FailureId);
            Assert.DoesNotContain("424242", v.Message);
            Assert.DoesNotContain("123456789", v.Message);
            Assert.Null(v.Seed);
            Assert.Null(v.Counterexample);
            Assert.Null(v.Minimal);
            Assert.NotNull(v.Digest);
            Assert.Matches("^[0-9a-f]{16}$", v.Digest!);
            Assert.Contains("repro-fnv: " + v.Digest, v.Message);
            Assert.Contains("iterations", v.Message);
        }

        Verdict open = Runner.Check(123456789, Gens.Constant(Big(424242)), x => x < 0, Local);
        Assert.Contains("424242", open.Message);
        Assert.Contains("123456789", open.Message);
        Assert.Null(open.Digest);
    }

    [Fact]
    public void TheDigestIsAPureFunctionOfIterationStepsAndWitness()
    {
        Verdict a = Runner.Check(5, Ints, x => x < 400, new Options(100, 1000, CiMode.On));
        Verdict b = Runner.Check(5, Ints, x => x < 400, new Options(100, 1000, CiMode.On));
        Assert.Equal(a.Digest, b.Digest);
        string preimage = "iter:" + a.Iteration + ";steps:" + a.ShrinkSteps + ";shrunk:400";
        Assert.Equal(SplitMix64.Fnv1a64(preimage).ToString("x16"), a.Digest);
    }

    [Fact]
    public void CiFromLookup()
    {
        Assert.False(Options.CiFromLookup(_ => null));
        foreach (string off in new[] { string.Empty, "  ", "0", "false", "FALSE", " False " })
        {
            Assert.False(Options.CiFromLookup(_ => off), off);
        }

        foreach (string on in new[] { "1", "true", "yes", "TRUE", "anything" })
        {
            Assert.True(Options.CiFromLookup(_ => on), on);
        }

        Assert.Equal(CiMode.Off, Options.Defaults().WithCi(CiMode.Off).Ci);
        Assert.Equal(CiMode.Unset, Options.Defaults().Ci);
    }

    [Fact]
    public void FailureVocabularyIsClosed()
    {
        foreach (string id in new[] { "property-counterexample", "property-exhausted", "property-timeout", "property-panic" })
        {
            Assert.True(FailureIds.IsPbtFailureId(id));
        }

        foreach (string? id in new[] { null, string.Empty, "unregistered-id", "Property-Panic", "property-panic ", "panic" })
        {
            Assert.False(FailureIds.IsPbtFailureId(id));
        }

        Assert.Equal(4, FailureIds.All.Count);
    }

    [Fact]
    public void TheVerdictIsDeterministicForASeed()
    {
        Verdict a = Runner.Check(99, Ints, x => x < 300, Local);
        Verdict b = Runner.Check(99, Ints, x => x < 300, Local);
        Assert.Equal(a, b);
    }

    // ---- hostile witnesses ---------------------------------------------------------------

    private sealed class Hostile
    {
        public override string ToString() => throw new InvalidOperationException("hostile ToString");
    }

    private sealed class HostileEnumerable : System.Collections.IEnumerable
    {
        public System.Collections.IEnumerator GetEnumerator() => throw new InvalidOperationException("hostile enumerator");
    }

    [Fact]
    public void CheckReturnsAVerdictForAHostileWitness()
    {
        foreach (CiMode mode in new[] { CiMode.On, CiMode.Off })
        {
            Verdict v = Runner.Check(1, Gens.Constant(new Hostile()), _ => false, new Options(3, 100, mode));
            Assert.Equal(FailureIds.Counterexample, v.FailureId);
            Verdict e = Runner.Check(1, Gens.Constant(new HostileEnumerable()), _ => false, new Options(3, 100, mode));
            Assert.Equal(FailureIds.Counterexample, e.FailureId);
            if (mode == CiMode.On)
            {
                Assert.Contains("repro-fnv", e.Message);
                Assert.Equal(
                    SplitMix64.Fnv1a64("iter:1;steps:0;shrunk:" + Canon.Unrenderable).ToString("x16"),
                    e.Digest);
            }
        }
    }

    [Fact]
    public void ADeeplyNestedWitnessStillGetsAVerdict()
    {
        object nested = 1;
        for (int i = 0; i < 100_000; i++)
        {
            nested = new List<object> { nested };
        }

        Verdict v = Runner.Check(1, Gens.Constant(nested), _ => false, new Options(1, 10, CiMode.On));
        Assert.Equal(FailureIds.Counterexample, v.FailureId);
        Assert.NotNull(v.Digest);
    }

    [Fact]
    public void OutOfMemoryIsNeverSwallowed()
    {
        Assert.Throws<OutOfMemoryException>(() => Runner.Check(1, Ints, _ => throw new OutOfMemoryException(), Local));
        Gen<int> oom = Gen<int>.Of(_ => throw new OutOfMemoryException(), null);
        Assert.Throws<OutOfMemoryException>(() => Runner.Check(1, oom, _ => true, Local));
        Shrinker<BigInteger> shrinkOom = (v, f, m) => throw new OutOfMemoryException();
        Assert.Throws<OutOfMemoryException>(() => Runner.Check(1, Ints, x => x < 1, Local, shrinkOom));
        Assert.Throws<OutOfMemoryException>(() => Canon.Render(new ThrowsOom()));
        Assert.False(Safe.Recoverable(new OutOfMemoryException()));
        Assert.True(Safe.Recoverable(new InvalidOperationException()));
    }

    private sealed class ThrowsOom
    {
        public override string ToString() => throw new OutOfMemoryException();
    }

    [Fact]
    public void RunawayGeneratorNestingBecomesExhaustedNotAProcessCrash()
    {
        // A recursive generator with no base case would overflow the stack, which .NET
        // cannot catch. Gen.Sample checks the stack and throws a catchable exception.
        Gen<int>? recursive = null;
        recursive = Gen<int>.Of(rng => recursive!.Sample(rng) + 1, null);
        Verdict v = Runner.Check(1, recursive, _ => true, Local);
        Assert.Equal(FailureIds.Exhausted, v.FailureId);
        Assert.Contains("InsufficientExecutionStackException", v.Message);
    }

    [Fact]
    public void ASlowShrinkerStillTerminatesWithinTheGlobalBudget()
    {
        int evaluations = 0;
        Verdict v = Runner.Check(
            1,
            Gens.ListOf(Gens.U8(), 50, 60),
            l => { evaluations++; return l.Count < 50; }, // the original fails, every candidate "holds" except size>=50
            new Options(5, 3, CiMode.Off));
        Assert.Equal(FailureIds.Counterexample, v.FailureId);
        Assert.True(evaluations < 5 + Limits.AttemptBudget(3) + 5, "evaluations " + evaluations);
    }
}
