using System.Runtime.CompilerServices;

namespace CodingAdventures.CtCompare.Tests;

public sealed class CtCompareTests
{
    [Fact]
    public void CtEqMatchesByteEquality()
    {
        Assert.True(CtCompare.CtEq("abcdef"u8, "abcdef"u8));
        Assert.True(CtCompare.CtEq(ReadOnlySpan<byte>.Empty, ReadOnlySpan<byte>.Empty));
        Assert.False(CtCompare.CtEq("abcdef"u8, "abcdeg"u8));
        Assert.False(CtCompare.CtEq("abcdef"u8, "bbcdef"u8));
        Assert.False(CtCompare.CtEq("abc"u8, "abcd"u8));
    }

    [Fact]
    public void CtEqDetectsEverySingleBitPosition()
    {
        var baseline = Enumerable.Repeat((byte)0x42, 32).ToArray();
        for (var index = 0; index < baseline.Length; index++)
        {
            for (var bit = 0; bit < 8; bit++)
            {
                var flipped = baseline.ToArray();
                flipped[index] ^= (byte)(1 << bit);
                Assert.False(CtCompare.CtEq(baseline, flipped));
            }
        }
    }

    [Fact]
    public void CtEqFixedIsDynamicAlias()
    {
        Assert.True(CtCompare.CtEqFixed(new byte[16], new byte[16]));
        var different = new byte[16];
        different[15] = 1;
        Assert.False(CtCompare.CtEqFixed(new byte[16], different));
    }

    [Fact]
    public void CtSelectBytesChoosesWithoutMutatingInputs()
    {
        var left = Enumerable.Range(0, 256).Select(i => (byte)i).ToArray();
        var right = left.Reverse().ToArray();

        Assert.Equal(left, CtCompare.CtSelectBytes(left, right, true));
        Assert.Equal(right, CtCompare.CtSelectBytes(left, right, false));
        Assert.Empty(CtCompare.CtSelectBytes([], [], true));
        Assert.Throws<ArgumentException>(() => CtCompare.CtSelectBytes("abc"u8, "abcd"u8, true));
    }

    [Fact]
    public void NullArraysAreRejectedNotTreatedAsEmpty()
    {
        // Regression: a null byte[] used to convert to an empty span, so
        // CtEq(null, Array.Empty<byte>()) returned true.
        byte[]? missing = null;
        Assert.Throws<ArgumentNullException>(() => CtCompare.CtEq(missing!, Array.Empty<byte>()));
        Assert.Throws<ArgumentNullException>(() => CtCompare.CtEq(Array.Empty<byte>(), missing!));
        Assert.Throws<ArgumentNullException>(() => CtCompare.CtEqFixed(missing!, Array.Empty<byte>()));
        Assert.Throws<ArgumentNullException>(() => CtCompare.CtSelectBytes(missing!, Array.Empty<byte>(), true));
        Assert.Throws<ArgumentNullException>(() => CtCompare.CtSelectBytes(Array.Empty<byte>(), missing!, false));
    }

    [Fact]
    public void ArrayOverloadsAgreeWithSpanOverloads()
    {
        var left = new byte[] { 1, 2, 3 };
        var right = new byte[] { 1, 2, 3 };
        Assert.True(CtCompare.CtEq(left, right));
        Assert.True(CtCompare.CtEqFixed(left, right));
        Assert.True(CtCompare.CtEqFixed("abc"u8, "abc"u8));
        Assert.False(CtCompare.CtEqFixed("abc"u8, "abd"u8));
        Assert.Equal(left, CtCompare.CtSelectBytes(left, new byte[] { 9, 9, 9 }, true));
    }

    [Fact]
    public void ChoiceStoredAsAnyNonZeroByteSelectsLeft()
    {
        // Unsafe code can store a non-0/1 byte in a bool; it still means true.
        byte stored = 2;
        var oddTrue = Unsafe.As<byte, bool>(ref stored);
        Assert.Equal(new byte[] { 7 }, CtCompare.CtSelectBytes(new byte[] { 7 }, new byte[] { 8 }, oddTrue));
    }

    [Fact]
    public void CtSelectBytesReturnsAFreshCopy()
    {
        var left = new byte[] { 1, 2, 3 };
        var picked = CtCompare.CtSelectBytes(left, new byte[] { 4, 5, 6 }, true);
        Assert.NotSame(left, picked);
        left[0] = 99;
        Assert.Equal(new byte[] { 1, 2, 3 }, picked);
    }

    [Fact]
    public void CtEqUInt64HandlesEdges()
    {
        Assert.True(CtCompare.CtEqUInt64(0, 0));
        Assert.True(CtCompare.CtEqUInt64(ulong.MaxValue, ulong.MaxValue));
        Assert.False(CtCompare.CtEqUInt64(0, 1UL << 63));

        const ulong baseline = 0x1234_5678_9ABC_DEF0;
        for (var bit = 0; bit < 64; bit++)
        {
            Assert.False(CtCompare.CtEqUInt64(baseline, baseline ^ (1UL << bit)));
        }
    }
}
