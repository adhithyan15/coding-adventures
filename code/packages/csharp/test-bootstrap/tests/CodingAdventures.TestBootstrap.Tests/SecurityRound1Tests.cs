using System.Numerics;
using System.Runtime.CompilerServices;
using CodingAdventures.TestBootstrap.CorpusJson;
using CodingAdventures.TestBootstrap.Pbt;
using CodingAdventures.TestBootstrap.Tfv;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>Regressions for the first C# security review (2 MEDIUM, 5 LOW).</summary>
public sealed class SecurityRound1Tests : HookedTest
{
    private static BigInteger Big(long v) => new(v);

    // MEDIUM-1: attempts alone do not bound the work; elements copied are charged too.
    [Fact]
    public void AMegabyteWitnessWithAStallingPropertyStaysInsideTheWorkBudget()
    {
        const int n = 1 << 20;
        long touched = 0;
        int calls = 0;
        Gen<byte[]> g = Gens.Bytes(0, Limits.MaxInputBytes);

        // Every smaller candidate "holds" (is rejected): the worst case for copy work.
        Shrunk<byte[]> r = g.Shrink(new byte[n], b => { calls++; touched += b.Length; return b.Length == n; }, 1000);

        Assert.Equal(n, r.Value.Length);
        Assert.True(ShrinkBudget.LastWork <= Limits.MaxShrinkWork, "charged work " + ShrinkBudget.LastWork);
        Assert.True(touched <= Limits.MaxShrinkWork, "elements handed to the property " + touched);
        // Without a work budget the attempt budget (16,064) would have allowed ~16 billion elements.
        Assert.True(calls < 100, "attempts " + calls);
        Assert.Null(ShrinkBudget.Current);
    }

    [Fact]
    public void TheRunnerShrinkOfAMegabyteWitnessIsAlsoBounded()
    {
        const int n = 1 << 20;
        long touched = 0;
        Verdict v = Runner.Check(
            1,
            Gens.Constant(new byte[n]),
            b => { touched += b.Length; return b.Length != n; },
            new Options(1, 1000, CiMode.Off));
        Assert.Equal(FailureIds.Counterexample, v.FailureId);
        Assert.True(touched <= Limits.MaxShrinkWork + (3L * n), "elements touched " + touched);
    }

    // MEDIUM-1 (second half): accepted moves are applied in place, from the move, never from a candidate
    // the property may have scribbled on.
    [Fact]
    public void AMutatingPropertyCannotCorruptBisectionOrProbe()
    {
        List<BigInteger> start = new() { 0, 7, 0, 0, 0, 0, 0, 0 };
        Shrunk<List<BigInteger>> r = Shrink.ShrinkSequence<BigInteger>(
            start,
            c =>
            {
                bool fails = c.Contains(7);
                c.Clear();
                c.Add(99);
                return fails;
            },
            1000,
            0,
            null);
        Assert.Equal(new List<BigInteger> { 7 }, r.Value);
        Assert.Equal(3, r.Steps);

        // minLen > 0: the prefix probe is accepted without reusing the candidate.
        Shrunk<List<byte>> p = Gens.ListOf(Gens.U8(), 2, 8).Shrink(new List<byte> { 9, 9, 9, 9, 9 }, c => { bool ok = c.Count >= 2; c.Clear(); return ok; }, 1000);
        Assert.Equal(new List<byte> { 0, 0 }, p.Value);
    }

    // MEDIUM-2: one budget for the whole shrink, across nested shrinkers.
    [Fact]
    public void NestedListsShareOneGlobalBudget()
    {
        Gen<List<List<byte>>> g = Gens.ListOf(Gens.ListOf(Gens.U8(), 0, 20), 0, 50);
        List<List<byte>> start = Enumerable.Range(0, 50).Select(_ => Enumerable.Repeat((byte)7, 20).ToList()).ToList();
        int calls = 0;
        const int maxSteps = 10;

        // Falsifies only for the exact original: every candidate at every level is rejected.
        Shrunk<List<List<byte>>> r = g.Shrink(
            start,
            l => { calls++; return l.Count == 50 && l.All(x => x.Count == 20 && x.All(b => b == 7)); },
            maxSteps);

        Assert.Equal(0, r.Steps);
        Assert.True(calls <= Limits.AttemptBudget(maxSteps), "nested evaluations " + calls + " exceed one global budget");
        Assert.True(ShrinkBudget.LastWork <= Limits.MaxShrinkWork);

        calls = 0;
        Verdict v = Runner.Check(
            1,
            Gens.Constant(start),
            l => { calls++; return !(l.Count == 50 && l.All(x => x.Count == 20 && x.All(b => b == 7))); },
            new Options(1, maxSteps, CiMode.Off));
        Assert.Equal(FailureIds.Counterexample, v.FailureId);
        Assert.True(calls <= 1 + Limits.AttemptBudget(maxSteps) + 1, "runner nested evaluations " + calls);
    }

    [Fact]
    public void ANestedShrinkerStillShrinksWhenTheBudgetIsAmple()
    {
        Gen<List<List<byte>>> g = Gens.ListOf(Gens.ListOf(Gens.U8(), 0, 5), 0, 5);
        Shrunk<List<List<byte>>> r = g.Shrink(new List<List<byte>> { new() { 5, 6 }, new() { 7 } }, l => l.Count >= 1, 1000);
        Assert.Single(r.Value);
        Assert.Empty(r.Value[0]);
    }

    [Fact]
    public void ABudgetSpentByWorkEndsEveryLoop()
    {
        using ShrinkBudget b = ShrinkBudget.EnterChild(1000, 10);
        Assert.True(b.Charge(6));
        Assert.False(b.Charge(6)); // 12 > 10
        Assert.True(b.Charge(4));
        Assert.False(b.HasRoom);
        Assert.Equal(2, b.Attempts);
        Assert.Equal(10, b.Work);
        using (ShrinkBudget nested = ShrinkBudget.Enter(5))
        {
            Assert.False(nested.Charge(1)); // joins the same, spent, budget
            Assert.False(nested.HasRoom);
            Assert.Equal(10, nested.Work);
        }

        Assert.Same(b, ShrinkBudget.Current); // a joining scope leaves the ambient one alone
    }

    [Fact]
    public void ChargeCopyChargesWorkButNotAttempts()
    {
        using ShrinkBudget b = ShrinkBudget.EnterChild(1000, 10);
        Assert.True(b.ChargeCopy(8));
        Assert.Equal(0, b.Attempts);
        Assert.False(b.ChargeCopy(3));
    }

    // LOW-1: do not call a user ToString with little stack left.
    private sealed class Odd
    {
        public override string ToString() => "odd";
    }

    [Fact]
    public void CanonDoesNotCallUserToStringWhenTheStackIsLow()
    {
        string? result = null;
        void Burn()
        {
            if (!RuntimeHelpers.TryEnsureSufficientExecutionStack())
            {
                result = Canon.Render(new Odd());
                return;
            }

            Burn();
        }

        Thread t = new(Burn, 512 * 1024);
        t.Start();
        t.Join();
        Assert.Equal(Canon.Unrenderable, result);
        Assert.Equal("\"odd\"", Canon.Render(new Odd())); // plenty of stack: rendered normally
    }

    // LOW-2: total output cap.
    [Fact]
    public void CanonOutputIsCappedInTotal()
    {
        List<string> strings = Enumerable.Range(0, 10_000).Select(_ => new string('x', 10_000)).ToList();
        string r = Canon.Render(strings);
        Assert.True(r.Length <= Canon.MaxOutput + 3, "rendered " + r.Length);
        Assert.EndsWith("...", r);

        Dictionary<int, string> d = Enumerable.Range(0, 9_000).ToDictionary(i => i, _ => new string('y', 10_000));
        string dr = Canon.Render(d);
        Assert.True(dr.Length <= (2 * Canon.MaxOutput) + 100_000, "dictionary rendered " + dr.Length);
        Assert.Equal(Canon.Render(strings).Length, Canon.Render(strings).Length); // deterministic
    }

    // LOW-3: Filter retries are capped.
    [Fact]
    public void FilterRetriesAreCapped()
    {
        Assert.Throws<ArgumentException>(() => Gens.Filter(Gens.Bool(), _ => true, Limits.MaxRetries + 1));
        Assert.Throws<ArgumentException>(() => Gens.Filter(Gens.Bool(), _ => true, int.MaxValue));
        Assert.NotNull(Gens.Filter(Gens.Bool(), _ => true, Limits.MaxRetries));
        Assert.True(Limits.MaxRetries >= Values.MaxSize); // the corpus' filter_exhausted cap fits
    }

    // LOW-4: non-printing characters are escaped in witnesses and in TFV diagnostics.
    [Fact]
    public void NonPrintingCharactersAreEscaped()
    {
        // Built from code points so the source file itself holds no raw control characters.
        char[] odd = { (char)0x85, (char)0x9f, (char)0x2028, (char)0x2029, (char)0x7f };
        string bs = new('\\', 1);
        Assert.Equal("\"" + bs + "u0085" + bs + "u009f" + bs + "u2028" + bs + "u2029" + bs + "u007f\"", Canon.Render(new string(odd)));
        string printable = new(new[] { (char)0xe9, (char)0xa0, 'x' });
        Assert.Equal("\"" + printable + "\"", Canon.Render(printable)); // printable non-ASCII stays

        string token = new(new[] { 'Q', (char)0x85, (char)0x2028, (char)0x01 });
        InvalidTfvException e = Assert.Throws<InvalidTfvException>(() => TfvValidator.Validate("tfv 1\nr R1 Kv " + token + "\n"));
        Assert.DoesNotContain(((char)0x85).ToString(), e.Message, StringComparison.Ordinal); // ordinal: culture compare ignores control chars
        Assert.DoesNotContain(((char)0x2028).ToString(), e.Message, StringComparison.Ordinal); // ordinal: culture compare ignores control chars
        Assert.DoesNotContain(((char)0x01).ToString(), e.Message, StringComparison.Ordinal); // ordinal: culture compare ignores control chars
        Assert.Contains("Q" + bs + "u0085" + bs + "u2028" + bs + "u0001", e.Message);
    }
}
