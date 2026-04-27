using BitsetType = CodingAdventures.BitsetNative.Bitset;

namespace CodingAdventures.BitsetNative.Tests;

public sealed class BitsetNativeTests
{
    [Fact]
    public void NewTracksLengthCapacityAndEmptyQueries()
    {
        using var empty = new BitsetType(0);
        Assert.Equal(0, empty.Length);
        Assert.Equal(0, empty.Capacity);
        Assert.Equal(0, empty.PopCount());
        Assert.True(empty.None());
        Assert.True(empty.All());
        Assert.True(empty.IsEmpty);
        Assert.Equal(string.Empty, empty.ToBinaryString());
        Assert.Equal<ulong?>(0, empty.ToInteger());

        using var sized = new BitsetType(65);
        Assert.Equal(65, sized.Length);
        Assert.Equal(128, sized.Capacity);
        Assert.False(sized.Any());
        Assert.False(sized.Test(64));
    }

    [Fact]
    public void FromIntegerSupportsSmallAndCrossWordValues()
    {
        using var five = BitsetType.FromInteger((UInt128)5);
        Assert.Equal(3, five.Length);
        Assert.True(five.Test(0));
        Assert.False(five.Test(1));
        Assert.True(five.Test(2));
        Assert.Equal("101", five.ToBinaryString());
        Assert.Equal<ulong?>(5, five.ToInteger());

        using var crossWord = BitsetType.FromInteger((UInt128.One << 64) | UInt128.One);
        Assert.Equal(65, crossWord.Length);
        Assert.True(crossWord.Test(0));
        Assert.True(crossWord.Test(64));
        Assert.Null(crossWord.ToInteger());
    }

    [Fact]
    public void FromBinaryStringUnderstandsConventionalBitOrder()
    {
        using var bitset = BitsetType.FromBinaryString("00101");
        Assert.Equal(5, bitset.Length);
        Assert.True(bitset.Test(0));
        Assert.False(bitset.Test(1));
        Assert.True(bitset.Test(2));
        Assert.False(bitset.Test(3));
        Assert.False(bitset.Test(4));
        Assert.Equal("00101", bitset.ToBinaryString());
    }

    [Fact]
    public void FromBinaryStringRejectsInvalidCharacters()
    {
        Assert.Throws<BitsetError>(() => BitsetType.FromBinaryString("10x1"));
    }

    [Fact]
    public void SetClearTestAndToggleHandleSingleBits()
    {
        using var bitset = new BitsetType(10);

        bitset.Set(5);
        Assert.True(bitset.Test(5));
        Assert.Equal(1, bitset.PopCount());

        bitset.Set(5);
        Assert.Equal(1, bitset.PopCount());

        bitset.Clear(5);
        Assert.False(bitset.Test(5));
        Assert.Equal(0, bitset.PopCount());

        bitset.Toggle(2);
        Assert.True(bitset.Test(2));
        bitset.Toggle(2);
        Assert.False(bitset.Test(2));
    }

    [Fact]
    public void ClearAndTestBeyondLengthAreSafeNoOps()
    {
        using var bitset = new BitsetType(8);
        bitset.Set(1);

        bitset.Clear(100);

        Assert.True(bitset.Test(1));
        Assert.False(bitset.Test(100));
        Assert.Equal(8, bitset.Length);
    }

    [Fact]
    public void SetAndToggleAutoGrowWithDoublingCapacity()
    {
        using var bitset = new BitsetType(100);

        bitset.Set(200);

        Assert.Equal(201, bitset.Length);
        Assert.Equal(256, bitset.Capacity);
        Assert.True(bitset.Test(200));
        Assert.False(bitset.Test(199));

        bitset.Toggle(500);
        Assert.Equal(501, bitset.Length);
        Assert.Equal(512, bitset.Capacity);
        Assert.True(bitset.Test(500));
    }

    [Fact]
    public void BulkOperationsMatchReferenceTruthTables()
    {
        using var left = BitsetType.FromInteger((UInt128)0b1100);
        using var right = BitsetType.FromInteger((UInt128)0b1010);
        using var andResult = left.And(right);
        using var orResult = left.Or(right);
        using var xorResult = left.Xor(right);
        using var andNotResult = left.AndNot(right);
        using var opAnd = left & right;
        using var opOr = left | right;
        using var opXor = left ^ right;

        Assert.Equal("1000", andResult.ToBinaryString());
        Assert.Equal("1110", orResult.ToBinaryString());
        Assert.Equal("0110", xorResult.ToBinaryString());
        Assert.Equal("0100", andNotResult.ToBinaryString());

        Assert.Equal("1000", opAnd.ToBinaryString());
        Assert.Equal("1110", opOr.ToBinaryString());
        Assert.Equal("0110", opXor.ToBinaryString());
    }

    [Fact]
    public void BulkOperationsZeroExtendShorterInputs()
    {
        using var shortBitset = BitsetType.FromBinaryString("101");
        using var longBitset = BitsetType.FromBinaryString("100001");
        using var orResult = shortBitset.Or(longBitset);
        using var andResult = shortBitset.And(longBitset);
        using var xorResult = shortBitset.Xor(longBitset);

        Assert.Equal("100101", orResult.ToBinaryString());
        Assert.Equal("000001", andResult.ToBinaryString());
        Assert.Equal("100100", xorResult.ToBinaryString());
        Assert.Equal(6, orResult.Length);
    }

    [Fact]
    public void NotOnlyFlipsBitsInsideLogicalLength()
    {
        using var bitset = new BitsetType(5);
        bitset.Set(0);
        bitset.Set(2);

        using var inverted = bitset.Not();
        using var roundTrip = ~inverted;

        Assert.Equal("11010", inverted.ToBinaryString());
        Assert.Equal(3, inverted.PopCount());
        Assert.False(inverted.Test(5));
        Assert.Equal(bitset, roundTrip);
    }

    [Fact]
    public void AnyAllNoneAndIsEmptyReflectCurrentState()
    {
        using var empty = new BitsetType(0);
        Assert.False(empty.Any());
        Assert.True(empty.All());
        Assert.True(empty.None());
        Assert.True(empty.IsEmpty);

        using var partial = new BitsetType(5);
        partial.Set(0);
        partial.Set(4);
        Assert.True(partial.Any());
        Assert.False(partial.All());
        Assert.False(partial.None());

        using var full = BitsetType.FromBinaryString("11111");
        Assert.True(full.All());
        Assert.False(full.None());
    }

    [Fact]
    public void IterSetBitsYieldsAscendingIndicesAcrossWords()
    {
        using var bitset = new BitsetType(130);
        bitset.Set(0);
        bitset.Set(2);
        bitset.Set(64);
        bitset.Set(129);

        Assert.Equal(new[] { 0, 2, 64, 129 }, bitset.IterSetBits().ToArray());
        Assert.Equal(new[] { 0, 2, 64, 129 }, bitset.ToArray());
        Assert.True(bitset.Contains(64));
    }

    [Fact]
    public void BinaryStringRoundTripPreservesLeadingZeros()
    {
        using var bitset = new BitsetType(8);
        bitset.Set(0);
        bitset.Set(7);

        Assert.Equal("10000001", bitset.ToBinaryString());

        using var roundTrip = BitsetType.FromBinaryString(bitset.ToBinaryString());
        Assert.Equal(bitset, roundTrip);
    }

    [Fact]
    public void EqualityUsesLogicalBits()
    {
        using var left = BitsetType.FromBinaryString("101001");
        using var right = new BitsetType(2);
        right.Set(5);
        right.Set(3);
        right.Set(0);

        Assert.Equal(left, right);
        Assert.True(left == right);
        Assert.False(left != right);
        Assert.Equal(left.GetHashCode(), right.GetHashCode());
    }

    [Fact]
    public void NegativeIndicesThrow()
    {
        using var bitset = new BitsetType(4);
        Assert.Throws<ArgumentOutOfRangeException>(() => bitset.Set(-1));
        Assert.Throws<ArgumentOutOfRangeException>(() => bitset.Clear(-1));
        Assert.Throws<ArgumentOutOfRangeException>(() => bitset.Test(-1));
        Assert.Throws<ArgumentOutOfRangeException>(() => bitset.Toggle(-1));
    }

    [Fact]
    public void DisposePreventsFurtherUse()
    {
        var bitset = new BitsetType(4);
        bitset.Dispose();
        bitset.Dispose();

        Assert.Throws<ObjectDisposedException>(() => bitset.Set(0));
    }

    [Fact]
    public void ToStringUsesBinaryForm()
    {
        using var bitset = BitsetType.FromBinaryString("101");
        Assert.Equal("Bitset(101)", bitset.ToString());
    }
}
