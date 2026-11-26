using System.Numerics;
using CodingAdventures.TestBootstrap.Fuzz;
using CodingAdventures.TestBootstrap.Prng;
using static CodingAdventures.TestBootstrap.Tests.FuzzFx;

namespace CodingAdventures.TestBootstrap.Tests;

/// <summary>
/// Unit tests of the mutator algebra, including the targeted tests for the known weaknesses of the
/// shared corpus (backlog PT-F1/F7): the single splice case has equal splits and the single
/// mutate_sequence case does not pin every constant.
/// </summary>
public sealed class FuzzMutateTests
{
    // ---- PT-F1: splice splits ------------------------------------------------------

    [Fact]
    public void SpliceUsesTheTargetSplitForThePrefixAndTheDonorSplitForTheSuffix()
    {
        byte[] t = Hx("01020304");
        byte[] d = Hx("05060708");
        Assert.Equal("0108", Hex(Mutate.Splice(t, d, 1, 3)));
        Assert.Equal("010203060708", Hex(Mutate.Splice(t, d, 3, 1))); // swapped splits differ
        Assert.Equal("0506070304", Hex(Mutate.Splice(d, t, 3, 2))); // swapped target and donor differ
        byte[] longer = Hx("0506070809");
        Assert.Equal("010203040809", Hex(Mutate.Splice(t, longer, 4, 3)));
        Assert.Equal("01020309", Hex(Mutate.Splice(t, longer, 3, 4)));
        Assert.Equal("010506070809", Hex(Mutate.Splice(t, longer, 1, 0)));
        Assert.Equal("0506070809" + "01020304", Hex(Mutate.Splice(longer, t, 5, 0)));
        Assert.Equal("050607" + "04", Hex(Mutate.Splice(longer, t, 3, 3)));
    }

    [Fact]
    public void SpliceClampsBothSplitsIndependently()
    {
        byte[] t = Hx("0102");
        byte[] d = Hx("0304");
        Assert.Equal("0102", Hex(Mutate.Splice(t, d, 99, 99)));
        Assert.Equal("010204", Hex(Mutate.Splice(t, d, 99, 1)));
        Assert.Equal("0304", Hex(Mutate.Splice(t, d, 0, 0)));
        Assert.Equal("04", Hex(Mutate.Splice(Array.Empty<byte>(), d, 7, 1)));
        Assert.Equal("0102", Hex(Mutate.SpliceBig(t, d, Bg("100000000000000000000000000"), Bg("100000000000000000000000000"))));
        Assert.Equal("0304", Hex(Mutate.SpliceBig(t, d, Bg("-100000000000000000000000000"), Bg("-100000000000000000000000000"))));
        Assert.Equal("0304", Hex(Mutate.SpliceBig(t, d, Bg("-5"), Bg("0"))));
        Assert.Equal(5, Mutate.SpliceLength(4, 4, 3, 2));
    }

    // ---- PT-F1: mutate_sequence constants -------------------------------------------

    [Fact]
    public void InterestListsAreExactlyTheSpecifiedConstants()
    {
        Assert.Equal(new ulong[] { 0, 1, 127, 128, 255 }, Mutate.InterestValues(1));
        Assert.Equal(new ulong[] { 0, 1, 255, 256, 32767, 32768, 65535 }, Mutate.InterestValues(2));
        Assert.Equal(new ulong[] { 0, 1, 65535, 65536, 2147483647, 2147483648, 4294967295 }, Mutate.InterestValues(4));
        string[] expected = { "0", "1", "4294967295", "4294967296", "9223372036854775807", "9223372036854775808", "18446744073709551615" };
        Assert.Equal(expected, Mutate.InterestValues(8)!.Select(v => v.ToString()).ToArray());
        Assert.Null(Mutate.InterestValues(3));
        Assert.Null(Mutate.InterestValues(0));
        Mutate.InterestValues(1)![0] = 99; // a copy
        Assert.Equal(0UL, Mutate.InterestValues(1)![0]);
        Assert.Equal(new long[] { -35, -8, -1, 1, 8, 35 }, Mutate.SequenceDeltas());
        Mutate.SequenceDeltas()[0] = 0;
        Assert.Equal(-35, Mutate.SequenceDeltas()[0]);
    }

    [Fact]
    public void EveryInterestValueLandsAtEveryWidthAndEndian()
    {
        foreach (int w in new[] { 1, 2, 4, 8 })
        {
            foreach (ulong v in Mutate.InterestValues(w)!)
            {
                foreach (Endian e in new[] { Endian.Little, Endian.Big })
                {
                    byte[] got = Mutate.BoundaryInsert(new byte[w + 1], 1, w, v, e);
                    byte[] want = new byte[w + 1];
                    for (int i = 0; i < w; i++)
                    {
                        int shift = 8 * (e == Endian.Big ? w - 1 - i : i);
                        want[1 + i] = (byte)(v >> shift);
                    }

                    Assert.True(want.AsSpan().SequenceEqual(got), "w=" + w + " v=" + v + " " + e);
                }
            }
        }
    }

    [Fact]
    public void ASingleStepSequenceFromOneZeroByteProducesExactlyTheSpecifiedValues()
    {
        SortedSet<int> seen = new();
        for (ulong seed = 0; seed < 6000; seed++)
        {
            byte[] output = Mutate.MutateSequence(new byte[] { 0 }, seed, 1, 1); // maxLen 1: no growth
            Assert.Single(output);
            seen.Add(output[0]);
        }

        SortedSet<int> want = new();
        for (int k = 0; k < 8; k++)
        {
            want.Add(1 << k); // bit flips
        }

        want.Add(0xFF); // byte flip, and delta -1
        foreach (int d in new[] { -35, -8, -1, 1, 8, 35 })
        {
            want.Add(d & 0xFF); // arithmetic deltas on 0
        }

        foreach (int v in new[] { 0, 1, 127, 128, 255 })
        {
            want.Add(v); // boundary values
        }

        Assert.Equal(want, seen);
    }

    [Fact]
    public void ArithmeticDeltasAreAddedExactly()
    {
        SortedSet<string> seen = new();
        for (ulong seed = 0; seed < 6000; seed++)
        {
            seen.Add(Hex(Mutate.MutateSequence(Hx("80"), seed, 1, 1)));
        }

        foreach (string s in new[] { "5d", "78", "7f", "81", "88", "a3" })
        {
            Assert.Contains(s, seen);
        }

        foreach (string s in new[] { "5c", "5e", "77", "79", "a2", "a4" })
        {
            Assert.DoesNotContain(s, seen);
        }
    }

    [Fact]
    public void SequenceReferenceValuesFromTheOracle()
    {
        Assert.Equal("00ddb08a3e0c0753ff7ffd0100c6fd", Hex(Mutate.MutateSequence(Hx("b9892d5158ae8d3dfdfa"), 42, 40, 1048576)));
        Assert.Equal("72bfca0b54", Hex(Mutate.MutateSequence(Hx("72cb54"), 42, 4, 1048576)));
        Assert.Equal("012bff013b", Hex(Mutate.MutateSequence(Hx("5d296d413b"), ulong.MaxValue, 8, 5)));
        Assert.Equal("7f82", Hex(Mutate.MutateSequence(Hx("4a8b6329b5"), 0, 16, 1)));
        Assert.Equal("03ff00000000bf080000", Hex(Mutate.MutateSequence(Hx("0000000000000000"), 42, 4, 1 << 20)));
        Assert.Equal("00fff6", Hex(Mutate.MutateSequence(Hx("0000"), 7, 6, 1 << 20)));
    }

    // ---- flips ----------------------------------------------------------------------

    [Fact]
    public void FlipsApplyByParityForEnormousCounts()
    {
        byte[] data = { 0, 0 };
        Assert.Equal("feff", Hex(Mutate.BitFlip(data, 0, 17)));
        Assert.Equal(2, Mutate.BitFlip(data, 3, 1_000_000_000_000_000_000UL).Length);
        Assert.Equal("0002", Hex(Mutate.BitFlip(data, 9, 1)));
        Assert.Equal("0100", Hex(Mutate.BitFlip(data, 0, 1)));
        Assert.Equal("3c00", Hex(Mutate.BitFlip(data, 2, 4)));
        Assert.Equal("ffff", Hex(Mutate.BitFlip(data, 0, 16)));
        Assert.Equal("0000", Hex(Mutate.BitFlip(data, 0, 32)));
        Assert.Equal("0000", Hex(Mutate.BitFlip(data, 5, 0)));
        Assert.Equal(string.Empty, Hex(Mutate.BitFlip(Array.Empty<byte>(), 5, 3)));
        Assert.Equal("0001", Hex(Mutate.BitFlipBig(data, Bg("-8"), BigInteger.One))); // floor modulo: -8 is bit 8
        Assert.Equal("0080", Hex(Mutate.BitFlipBig(data, Bg("-1"), BigInteger.One)));
        Assert.Equal("0000", Hex(Mutate.BitFlipBig(data, Bg("5"), Bg("-3"))));
        Assert.Equal("edcba9", Hex(Mutate.ByteFlip(Hx("123456"), 0, 3)));
        Assert.Equal("12cba9", Hex(Mutate.ByteFlip(Hx("123456"), 1, 2)));
        Assert.Equal("12cba9", Hex(Mutate.ByteFlipBig(Hx("123456"), Bg("-2"), Bg("2"))));
        Assert.Equal("1234a9", Hex(Mutate.ByteFlip(Hx("123456"), 0, 5)));
        Assert.Equal("123456", Hex(Mutate.ByteFlip(Hx("123456"), 7, 0)));
        BigInteger huge = Bg("1000000000000000000000000000001");
        Assert.Equal(Hex(Mutate.BitFlip(data, 1, 1)), Hex(Mutate.BitFlipBig(data, Bg("1"), BigInteger.One)));
        Assert.Equal(2, Mutate.BitFlipBig(data, huge, huge).Length);
        Assert.Equal("0000", Hex(Mutate.BitFlipBig(new byte[2], BigInteger.Zero, BigInteger.Zero)));
        Assert.Equal(string.Empty, Hex(Mutate.ByteFlipBig(Array.Empty<byte>(), huge, huge)));
    }

    [Fact]
    public void FlipsMatchANaiveLoopForEveryOffsetAndSmallCount()
    {
        byte[] data = Hx("a53c");
        for (int off = -20; off < 20; off++)
        {
            for (int count = 0; count < 40; count++)
            {
                byte[] naive = (byte[])data.Clone();
                for (int i = 0; i < count; i++)
                {
                    int pos = (((off + i) % 16) + 16) % 16;
                    naive[pos / 8] ^= (byte)(1 << (pos % 8));
                }

                byte[] viaBig = Mutate.BitFlipBig(data, off, count);
                Assert.True(naive.AsSpan().SequenceEqual(viaBig), "bit off=" + off + " count=" + count);
                if (off >= 0)
                {
                    Assert.True(naive.AsSpan().SequenceEqual(Mutate.BitFlip(data, (ulong)off, (ulong)count)));
                }

                byte[] naiveBytes = (byte[])data.Clone();
                for (int i = 0; i < count; i++)
                {
                    naiveBytes[(((off + i) % 2) + 2) % 2] ^= 0xFF;
                }

                Assert.True(naiveBytes.AsSpan().SequenceEqual(Mutate.ByteFlipBig(data, off, count)), "byte off=" + off + " count=" + count);
            }
        }
    }

    // ---- arithmetic and boundary -----------------------------------------------------

    [Fact]
    public void ArithmeticWrapsAtEveryWidthAndEndian()
    {
        Assert.Equal("00", Hex(Mutate.ByteArithmetic(Hx("ff"), 0, 1, 1, Endian.Little)));
        Assert.Equal("0001", Hex(Mutate.ByteArithmetic(Hx("ff00"), 0, 1, 2, Endian.Little)));
        Assert.Equal("0100", Hex(Mutate.ByteArithmetic(Hx("00ff"), 0, 1, 2, Endian.Big)));
        Assert.Equal("00000000", Hex(Mutate.ByteArithmetic(Hx("ffffffff"), 0, 1, 4, Endian.Big)));
        Assert.Equal("0000000000000000", Hex(Mutate.ByteArithmetic(Hx("ffffffffffffffff"), 0, 1, 8, Endian.Little)));
        Assert.Equal("ffffffffffffffff", Hex(Mutate.ByteArithmetic(Hx("0000000000000000"), 0, -1, 8, Endian.Big)));
        Assert.Equal("fe", Hex(Mutate.ByteArithmetic(Hx("00"), 0, -2, 1, Endian.Little)));
        Assert.Equal("0000ff", Hex(Mutate.ByteArithmeticBig(Hx("0000fe"), Bg("-1"), Bg("1"), 1, Endian.Little))); // offset wraps over 3 positions
        Assert.Equal("ff0000", Hex(Mutate.ByteArithmetic(Hx("fe0000"), 3, 1, 1, Endian.Little)));
        Assert.Equal("ffff", Hex(Mutate.ByteArithmetic(Hx("ffff"), 0, 0x10000, 2, Endian.Little))); // delta mod 2^16
        Assert.Equal("ab", Hex(Mutate.ByteArithmetic(Hx("ab"), 0, 5, 3, Endian.Little))); // bad width
        Assert.Equal("ab", Hex(Mutate.ByteArithmetic(Hx("ab"), 0, 5, 2, Endian.Little))); // too short
        Assert.Equal("ab", Hex(Mutate.ByteArithmetic(Hx("ab"), 0, 5, 0, Endian.Little)));
        Assert.Equal("ab", Hex(Mutate.ByteArithmeticBig(Hx("ab"), BigInteger.Zero, BigInteger.One, 9, Endian.Big)));
        Assert.Equal(string.Empty, Hex(Mutate.ByteArithmetic(Array.Empty<byte>(), 0, 1, 1, Endian.Little)));
        Assert.Equal(
            Hex(Mutate.ByteArithmetic(Hx("0123456789abcdef"), 0, -3, 8, Endian.Big)),
            Hex(Mutate.ByteArithmeticBig(Hx("0123456789abcdef"), BigInteger.Zero, Bg("18446744073709551613"), 8, Endian.Big)));
    }

    [Fact]
    public void BoundaryInsertsExactBytes()
    {
        Assert.Equal("00000080", Hex(Mutate.BoundaryInsert(Hx("00000000"), 0, 4, 0x80000000UL, Endian.Little)));
        Assert.Equal("80000000", Hex(Mutate.BoundaryInsert(Hx("00000000"), 0, 4, 0x80000000UL, Endian.Big)));
        Assert.Equal("00ffff", Hex(Mutate.BoundaryInsert(Hx("000000"), 1, 2, 65535, Endian.Big)));
        Assert.Equal("ffff", Hex(Mutate.BoundaryInsert(Hx("0000"), 0, 2, ulong.MaxValue, Endian.Little)));
        Assert.Equal("7fffffffffffffff", Hex(Mutate.BoundaryInsert(new byte[8], 0, 8, long.MaxValue, Endian.Big)));
        Assert.Equal("0000000000000080", Hex(Mutate.BoundaryInsert(new byte[8], 0, 8, 1UL << 63, Endian.Little)));
        Assert.Equal("ff", Hex(Mutate.BoundaryInsertBig(Hx("00"), BigInteger.Zero, 1, Bg("-1"), Endian.Little)));
        Assert.Equal("00", Hex(Mutate.BoundaryInsertBig(Hx("ab"), BigInteger.Zero, 1, Bg("256"), Endian.Little)));
        Assert.Equal("ab", Hex(Mutate.BoundaryInsert(Hx("ab"), 0, 3, 1, Endian.Little)));
        Assert.Equal("ab", Hex(Mutate.BoundaryInsert(Hx("ab"), 0, 2, 1, Endian.Little)));
        Assert.Equal("ab", Hex(Mutate.BoundaryInsertBig(Hx("ab"), BigInteger.Zero, 0, BigInteger.One, Endian.Little)));
        Assert.Equal("00ab", Hex(Mutate.BoundaryInsertBig(Hx("0000"), Bg("-1"), 1, Bg("171"), Endian.Little))); // -1 wraps to position 1
    }

    // ---- blocks -----------------------------------------------------------------------

    [Fact]
    public void BlockOperationsClampAndWrap()
    {
        Assert.Equal("01020506", Hex(Mutate.BlockDelete(Hx("010203040506"), 2, 2)));
        Assert.Equal("0102", Hex(Mutate.BlockDelete(Hx("01020304"), 2, 100)));
        Assert.Equal("0304", Hex(Mutate.BlockDeleteBig(Hx("01020304"), Bg("-4"), Bg("2"))));
        Assert.Equal("01020304", Hex(Mutate.BlockDelete(Hx("01020304"), 1, 0)));
        Assert.Equal(string.Empty, Hex(Mutate.BlockDelete(Array.Empty<byte>(), 0, 1)));
        Assert.Equal(string.Empty, Hex(Mutate.BlockDeleteBig(Hx("0102"), BigInteger.Zero, Bg("100000000000000000000000000"))));
        Assert.Equal("01020304", Hex(Mutate.BlockDeleteBig(Hx("01020304"), BigInteger.Zero, Bg("-1"))));
        Assert.Equal("0102ff03", Hex(Mutate.BlockInsert(Hx("010203"), 2, Hx("ff"))));
        Assert.Equal("010203ff", Hex(Mutate.BlockInsert(Hx("010203"), 3, Hx("ff"))));
        Assert.Equal("ff010203", Hex(Mutate.BlockInsert(Hx("010203"), 4, Hx("ff"))));
        Assert.Equal("ff", Hex(Mutate.BlockInsert(Array.Empty<byte>(), 9, Hx("ff"))));
        Assert.Equal("ff", Hex(Mutate.BlockInsertBig(Array.Empty<byte>(), Bg("9"), Hx("ff"))));
        Assert.Equal("01ff0203", Hex(Mutate.BlockInsertBig(Hx("010203"), Bg("1000000000000000000000000001"), Hx("ff"))));
        Assert.Equal("01010203", Hex(Mutate.BlockDuplicate(Hx("010203"), 0, 1, 1)));
        Assert.Equal("0102030203", Hex(Mutate.BlockDuplicate(Hx("010203"), 1, 100, 3)));
        Assert.Equal("010203", Hex(Mutate.BlockDuplicate(Hx("010203"), 1, 0, 3)));
        Assert.Equal(string.Empty, Hex(Mutate.BlockDuplicate(Array.Empty<byte>(), 1, 1, 1)));
        Assert.Equal("01010203", Hex(Mutate.BlockDuplicateBig(Hx("010203"), Bg("0"), Bg("1"), Bg("1"))));
        Assert.Equal("010203", Hex(Mutate.BlockDuplicateBig(Hx("010203"), Bg("0"), Bg("-1"), Bg("1"))));
        Assert.Equal("010203010203", Hex(Mutate.BlockDuplicateBig(Hx("010203"), Bg("0"), Bg("100000000000000000000000000"), Bg("3"))));
    }

    [Fact]
    public void NoMutatorModifiesItsArgumentOrReturnsItsAlias()
    {
        byte[] d = Hx("0102030405060708");
        byte[] keep = (byte[])d.Clone();
        byte[] ins = Hx("aa");
        byte[][] results =
        {
            Mutate.BitFlip(d, 3, 5), Mutate.BitFlip(d, 3, 0), Mutate.ByteFlip(d, 1, 2), Mutate.ByteFlip(d, 1, 0),
            Mutate.ByteArithmetic(d, 1, 5, 2, Endian.Big), Mutate.ByteArithmetic(d, 1, 5, 3, Endian.Big),
            Mutate.BoundaryInsert(d, 1, 2, 9, Endian.Little), Mutate.BoundaryInsert(d, 1, 3, 9, Endian.Little),
            Mutate.BlockDelete(d, 1, 2), Mutate.BlockDelete(d, 1, 0), Mutate.BlockInsert(d, 1, ins),
            Mutate.BlockDuplicate(d, 1, 2, 3), Mutate.BlockDuplicate(d, 1, 0, 3), Mutate.Splice(d, ins, 1, 0),
            Mutate.MutateSequence(d, 7, 0, 100), Mutate.MutateSequence(d, 7, -3, 100),
        };
        Assert.Equal(keep, d);
        foreach (byte[] r in results)
        {
            Assert.NotSame(d, r);
        }

        Assert.Equal(keep, Mutate.MutateSequence(d, 7, 0, 100));
        Assert.NotSame(ins, Mutate.BlockInsert(Array.Empty<byte>(), 0, ins));
    }

    // ---- MutateSequence mechanics -------------------------------------------------------

    [Fact]
    public void MutateSequenceIsDeterministicAndRefillsAnEmptyBuffer()
    {
        byte[] a = Mutate.MutateSequence(Hx("0102030405"), 99, 30, 1 << 20);
        Assert.Equal(a, Mutate.MutateSequence(Hx("0102030405"), 99, 30, 1 << 20));
        Assert.True(Mutate.MutateSequence(Array.Empty<byte>(), 5, 1, 64).Length >= 1, "an empty buffer is refilled to 00 first");
        Assert.Empty(Mutate.MutateSequence(Array.Empty<byte>(), 5, 0, 64));
    }

    [Fact]
    public void GrowthPastMaxLenIsSkipped()
    {
        for (ulong seed = 0; seed < 200; seed++)
        {
            byte[] output = Mutate.MutateSequence(Hx("01020304"), seed, 60, 6);
            Assert.True(output.Length <= 6, "length " + output.Length + " for seed " + seed);
        }
    }

    [Fact]
    public void MutateSequenceIsCapped()
    {
        Assert.Throws<ArgumentException>(() => Mutate.MutateSequence(new byte[1], 1, Mutate.MaxIterations + 1, 10));
        Assert.Throws<ArgumentException>(() => Mutate.MutateSequence(new byte[Mutate.MaxInputBytes + 1], 1, 1, 10));
        Assert.ThrowsAny<ArgumentException>(() => Mutate.MutateSequence(new byte[100], new SplitMix64(1), 50, 1000, 250));
        Assert.Equal(100, Mutate.MutateSequence(new byte[100], new SplitMix64(1), 0, 1000, 100).Length);
        Assert.Throws<Mutate.WorkExhausted>(() => Mutate.MutateSequence(new byte[100], new SplitMix64(1), 0, 1000, 99));
        Assert.True(Mutate.MutateSequence(new byte[100], new SplitMix64(1), 2, 1000, 300).Length > 0);
        Assert.Throws<Mutate.WorkExhausted>(() => Mutate.MutateSequence(new byte[100], new SplitMix64(1), 2, 1000, 299));
        Mutate.WorkExhausted e = Assert.Throws<Mutate.WorkExhausted>(() => Mutate.MutateSequence(new byte[100], new SplitMix64(1), 50, 1000, 250));
        Assert.Equal(200, e.Work);
        Assert.Equal(Mutate.MaxInputBytes, Mutate.MutateSequence(new byte[Mutate.MaxInputBytes], 1, 0, 10).Length);
        Assert.True(Mutate.MutateSequence(new byte[Mutate.MaxInputBytes], 1, 4, 10).Length > 0);
        Assert.Contains("budget", e.Message);
    }

    [Fact]
    public void BigIntegerReductionsSaturateInsteadOfOverflowing()
    {
        Assert.Equal(ulong.MaxValue, Mutate.SatU64(Bg("99999999999999999999999")));
        Assert.Equal(0UL, Mutate.SatU64(Bg("-99999999999999999999999")));
        Assert.Equal(5UL, Mutate.SatU64(Bg("5")));
        Assert.Equal(ulong.MaxValue, Mutate.Low64(Bg("-1")));
        Assert.Equal(0UL, Mutate.Low64(Bg("18446744073709551616")));
    }
}
