using System.Numerics;
using CodingAdventures.TestBootstrap.Pbt;
using CodingAdventures.TestBootstrap.Prng;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>The generator algebra: ranges, draw order, shrinkers and argument checking.</summary>
public sealed class GensTests : HookedTest
{
    private static BigInteger Big(long v) => new(v);

    // falsifies(v) is true while v still makes the property fail.
    private static Func<T, bool> Never<T>() => _ => false;

    private static Func<T, bool> Always<T>() => _ => true;

    [Fact]
    public void IntegerStaysInRangeAcrossSeeds()
    {
        Gen<BigInteger> g = Gens.Integer(Big(-5), Big(5));
        for (ulong seed = 0; seed < 100; seed++)
        {
            SplitMix64 rng = new(seed);
            for (int i = 0; i < 20; i++)
            {
                BigInteger v = g.Sample(rng);
                Assert.True(v >= -5 && v <= 5);
            }
        }
    }

    [Fact]
    public void IntegerRejectsBadBounds()
    {
        Assert.Throws<ArgumentException>(() => Gens.Integer(Big(2), Big(1)));
        Assert.Throws<ArgumentException>(() => Gens.Int64(2, 1));
    }

    [Fact]
    public void Int64MatchesIntegerDrawsIncludingTheFullRange()
    {
        Gen<long> a = Gens.Int64(-1000, 1000);
        Gen<BigInteger> b = Gens.Integer(Big(-1000), Big(1000));
        SplitMix64 r1 = new(5);
        SplitMix64 r2 = new(5);
        for (int i = 0; i < 50; i++)
        {
            Assert.Equal(b.Sample(r2), new BigInteger(a.Sample(r1)));
        }

        // Full range: span wraps to 0, meaning 2^64, which is one raw word.
        Gen<long> full = Gens.Int64(long.MinValue, long.MaxValue);
        SplitMix64 f1 = new(8);
        SplitMix64 f2 = new(8);
        for (int i = 0; i < 10; i++)
        {
            Assert.Equal(unchecked((long)(f2.NextU64() + (ulong)long.MinValue)), full.Sample(f1));
        }

        Assert.Equal(long.MinValue, Gens.Int64(long.MinValue, long.MinValue).Sample(new SplitMix64(1)));
    }

    [Fact]
    public void I128CoversBothSignsAndStaysInRange()
    {
        Gen<BigInteger> g = Gens.I128();
        BigInteger min = -(BigInteger.One << 127);
        BigInteger max = (BigInteger.One << 127) - 1;
        bool neg = false;
        bool pos = false;
        SplitMix64 rng = new(42);
        for (int i = 0; i < 200; i++)
        {
            BigInteger v = g.Sample(rng);
            Assert.True(v >= min && v <= max);
            neg |= v.Sign < 0;
            pos |= v.Sign > 0;
        }

        Assert.True(neg && pos);
    }

    [Fact]
    public void PrimitivesAndTheirShrinkers()
    {
        Assert.Equal(new Shrunk<bool>(false, 1), Gens.Bool().Shrink(true, Always<bool>(), 10));
        Assert.Equal(new Shrunk<bool>(true, 0), Gens.Bool().Shrink(true, Never<bool>(), 10));
        Assert.Equal(new Shrunk<bool>(false, 0), Gens.Bool().Shrink(false, Always<bool>(), 10));
        Assert.Equal(new Shrunk<bool>(true, 0), Gens.Bool().Shrink(true, Always<bool>(), 0));

        Assert.Equal(new Shrunk<byte>(0, 1), Gens.U8().Shrink(200, Always<byte>(), 10));
        Assert.Equal(new Shrunk<ulong>(0, 1), Gens.U64().Shrink(ulong.MaxValue, Always<ulong>(), 10));
        Assert.Equal(new Shrunk<BigInteger>(BigInteger.Zero, 1), Gens.I128().Shrink(Big(-99), Always<BigInteger>(), 10));
        Assert.Equal(new Shrunk<long>(0, 1), Gens.Int64(-50, 50).Shrink(-42, Always<long>(), 10));
        Assert.Equal(new Shrunk<BigInteger>(Big(5), 1), Gens.Integer(Big(5), Big(10)).Shrink(Big(9), Always<BigInteger>(), 10));
        Assert.Equal(new Shrunk<BigInteger>(Big(-3), 1), Gens.Integer(Big(-10), Big(-3)).Shrink(Big(-9), Always<BigInteger>(), 10));
        Assert.Equal(new Shrunk<int>(7, 0), Gens.Constant(7).Shrink(7, Always<int>(), 10));

        // Nothing falsifies: the value stays put.
        Assert.Equal(new Shrunk<byte>(200, 0), Gens.U8().Shrink(200, Never<byte>(), 10));

        // The U8 walk ends exactly on the smallest failing byte.
        Assert.Equal(100, Gens.U8().Shrink(250, b => b >= 100, 1000).Value);
    }

    [Fact]
    public void ClampZeroTargets()
    {
        Assert.Equal(Big(5), Gens.ClampZero(Big(5), Big(10)));
        Assert.Equal(Big(-3), Gens.ClampZero(Big(-10), Big(-3)));
        Assert.Equal(Big(0), Gens.ClampZero(Big(-10), Big(10)));
    }

    [Fact]
    public void BytesAndListLengthsAndStreamPosition()
    {
        Gen<byte[]> g = Gens.Bytes(4, 8);
        SplitMix64 rng = new(13);
        SplitMix64 mirror = new(13);
        for (int i = 0; i < 20; i++)
        {
            byte[] v = g.Sample(rng);
            Assert.InRange(v.Length, 4, 8);
            int n = 4 + (int)mirror.Bounded(5);
            Assert.Equal(n, v.Length);
            foreach (byte b in v)
            {
                Assert.Equal((byte)mirror.Bounded(256), b);
            }
        }

        Gen<List<byte>> l = Gens.ListOf(Gens.U8(), 1, 3);
        SplitMix64 lr = new(2);
        for (int i = 0; i < 20; i++)
        {
            Assert.InRange(l.Sample(lr).Count, 1, 3);
        }
    }

    [Fact]
    public void ListRespectsMinLenWhileShrinking()
    {
        Gen<List<byte>> g = Gens.ListOf(Gens.U8(), 3, 10);
        Shrunk<List<byte>> r = g.Shrink(new List<byte> { 5, 6, 7, 8, 9, 10 }, l => l.Count >= 3, 1000);
        Assert.Equal(3, r.Value.Count);
        Gen<byte[]> bytes = Gens.Bytes(2, 10);
        Assert.Equal(2, bytes.Shrink(new byte[] { 1, 2, 3, 4, 5 }, b => b.Length >= 2, 1000).Value.Length);
    }

    [Fact]
    public void FilterKeepsShrinkingValid()
    {
        Gen<BigInteger> even = Gens.Filter(Gens.Integer(Big(0), Big(1000)), x => x.IsEven);
        List<BigInteger> offered = new();
        Shrunk<BigInteger> r = even.Shrink(Big(900), x => { offered.Add(x); return x >= 7; }, 1000);
        Assert.True(r.Value.IsEven, "a shrink candidate that fails the filter is not a valid input");
        Assert.True(r.Value >= 7);
        Assert.DoesNotContain(offered, x => !x.IsEven); // invalid candidates never reach the property
        Assert.NotEmpty(offered);
        Assert.Throws<ArgumentException>(() => Gens.Filter(Gens.Bool(), _ => true, 0));
    }

    [Fact]
    public void FilterThatNeverAcceptsIsExhaustedAfterExactlyMaxRetries()
    {
        int calls = 0;
        Gen<bool> g = Gens.Filter(Gens.Bool(), _ => { calls++; return false; }, 7);
        Assert.Throws<ExhaustedException>(() => g.Sample(new SplitMix64(1)));
        Assert.Equal(7, calls);
        calls = 0;
        Assert.Throws<ExhaustedException>(() => Gens.Filter(Gens.Bool(), _ => { calls++; return false; }).Sample(new SplitMix64(1)));
        Assert.Equal(Limits.DefaultMaxRetries, calls);
    }

    [Fact]
    public void MapFlatMapConstantAndGenWithoutSampler()
    {
        Gen<BigInteger> doubled = Gens.Map(Gens.Integer(Big(1), Big(5)), x => x * 2);
        Assert.True(doubled.Sample(new SplitMix64(3)).IsEven);
        Assert.Equal(0, doubled.Shrink(Big(6), Never<BigInteger>(), 10).Steps); // map drops shrinking
        Gen<long> dep = Gens.FlatMap(Gens.Integer(Big(1), Big(3)), n => Gens.Int64(0, (long)n));
        for (ulong s = 0; s < 20; s++)
        {
            Assert.InRange(dep.Sample(new SplitMix64(s)), 0, 3);
        }

        SplitMix64 untouched = new(9);
        Assert.Equal(5, Gens.Constant(5).Sample(untouched));
        Assert.Equal(9UL, untouched.State);
        Assert.Throws<ArgumentNullException>(() => Gen<int>.Of(null!, null));
    }

    [Fact]
    public void OneOfAndFrequency()
    {
        Gen<int> one = Gens.OneOf(Gens.Constant(1), Gens.Constant(2), Gens.Constant(3));
        HashSet<int> seen = new();
        SplitMix64 rng = new(4);
        for (int i = 0; i < 100; i++)
        {
            seen.Add(one.Sample(rng));
        }

        Assert.Equal(3, seen.Count);
        Assert.Throws<ArgumentException>(() => Gens.OneOf<int>());

        Gen<string> f = Gens.Frequency(new Arm<string>(1, Gens.Constant("a")), new Arm<string>(0, Gens.Constant("never")), new Arm<string>(3, Gens.Constant("b")));
        int b = 0;
        int a = 0;
        SplitMix64 fr = new(11);
        for (int i = 0; i < 4000; i++)
        {
            string v = f.Sample(fr);
            Assert.NotEqual("never", v);
            if (v == "a")
            {
                a++;
            }
            else
            {
                b++;
            }
        }

        Assert.InRange(b, 2800, 3200);
        Assert.InRange(a, 800, 1200);

        Assert.Throws<ArgumentException>(() => Gens.Frequency<int>());
        Assert.Throws<ArgumentException>(() => Gens.Frequency(new Arm<int>(0, Gens.Constant(1))));
        Assert.Throws<ArgumentException>(() => Gens.Frequency(new Arm<int>(-1, Gens.Constant(1))));
        Assert.Throws<ArgumentException>(() => Gens.Frequency(new Arm<int>(Limits.MaxFrequencyTotal + 1, Gens.Constant(1))));
        Assert.Throws<ArgumentException>(() => Gens.Frequency(new Arm<int>(Limits.MaxFrequencyTotal, Gens.Constant(1)), new Arm<int>(1, Gens.Constant(2))));
        Assert.NotNull(Gens.Frequency(new Arm<int>(Limits.MaxFrequencyTotal, Gens.Constant(1))));
    }

    [Fact]
    public void Tuples()
    {
        Gen<Pair<byte, BigInteger>> g = Gens.Tuple2(Gens.U8(), Gens.Integer(Big(0), Big(100)));
        SplitMix64 rng = new(21);
        SplitMix64 mirror = new(21);
        Pair<byte, BigInteger> p = g.Sample(rng);
        Assert.Equal((byte)mirror.Bounded(256), p.First);
        Assert.Equal(Big(0) + mirror.BoundedBig(Big(101)), p.Second);

        // Both coordinates shrink to their minimal failing value, across repeated passes.
        Shrunk<Pair<byte, BigInteger>> r = g.Shrink(new Pair<byte, BigInteger>(200, Big(90)), q => q.First >= 50 && q.Second >= 30, 1000);
        Assert.Equal(50, r.Value.First);
        Assert.Equal(Big(30), r.Value.Second);

        Gen<Triple<byte, bool, long>> t = Gens.Tuple3(Gens.U8(), Gens.Bool(), Gens.Int64(-10, 10));
        Triple<byte, bool, long> s = t.Sample(new SplitMix64(1));
        Assert.InRange(s.Third, -10, 10);
        Shrunk<Triple<byte, bool, long>> tr = t.Shrink(new Triple<byte, bool, long>(77, true, 9), x => true, 1000);
        Assert.Equal(new Triple<byte, bool, long>(0, false, 0), tr.Value);

        // ITuple views for rendering.
        ITupleCheck(p, 2);
        ITupleCheck(s, 3);
        Assert.Throws<IndexOutOfRangeException>(() => ((System.Runtime.CompilerServices.ITuple)p)[2]);
        Assert.Throws<IndexOutOfRangeException>(() => ((System.Runtime.CompilerServices.ITuple)s)[3]);
    }

    private static void ITupleCheck(System.Runtime.CompilerServices.ITuple t, int length)
    {
        Assert.Equal(length, t.Length);
        for (int i = 0; i < length; i++)
        {
            Assert.NotNull(t[i]);
        }
    }

    [Fact]
    public void ConstructorArgumentChecks()
    {
        Assert.Throws<ArgumentException>(() => Gens.Bytes(5, 4));
        Assert.Throws<ArgumentException>(() => Gens.Bytes(-1, 4));
        Assert.Throws<ArgumentException>(() => Gens.Bytes(0, Limits.MaxInputBytes + 1));
        Assert.Throws<ArgumentException>(() => Gens.ListOf(Gens.Bool(), 5, 4));
        Assert.Throws<ArgumentException>(() => Gens.ListOf(Gens.Bool(), 0, Limits.MaxListLen + 1));
        Assert.NotNull(Gens.Bytes(0, Limits.MaxInputBytes));
        Assert.NotNull(Gens.ListOf(Gens.Bool(), 0, Limits.MaxListLen));
    }

    [Fact]
    public void CorpusEntryPointErrors()
    {
        Assert.Throws<ArgumentException>(() => Gens.GenPrimitive(0, "float", 1));
        Assert.Throws<ArgumentException>(() => Gens.GenBounded(0, Big(2), Big(1), 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => Gens.GenPrimitive(0, "bool", -1));
        Assert.Throws<ArgumentException>(() => Gens.GenBytes(0, 5, 4, 1));
        Assert.Throws<ArgumentException>(() => Gens.GenBytes(0, 0, 1000, 5000));
        Assert.Empty(Gens.GenPrimitive(0, "bool", 0));
    }

    [Fact]
    public void GenPrimitiveMatchesTheTypedGenerators()
    {
        List<object> prims = Gens.GenPrimitive(5, "u8", 10);
        SplitMix64 rng = new(5);
        Gen<byte> g = Gens.U8();
        foreach (object p in prims)
        {
            Assert.Equal(new BigInteger(g.Sample(rng)), (BigInteger)p);
        }
    }
}
