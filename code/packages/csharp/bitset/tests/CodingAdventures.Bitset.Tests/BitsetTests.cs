using BitsetType = CodingAdventures.Bitset.Bitset;

namespace CodingAdventures.Bitset.Tests;

public sealed class BitsetTests
{
    [Fact]
    public void NewTracksLengthCapacityAndEmptyQueries()
    {
        var empty = new BitsetType(0);
        Assert.Equal(0, empty.Length);
        Assert.Equal(0, empty.Capacity);
        Assert.Equal(0, empty.PopCount());
        Assert.True(empty.None());
        Assert.True(empty.All());
        Assert.True(empty.IsEmpty);
        Assert.Equal(string.Empty, empty.ToBinaryString());
        Assert.Equal<ulong?>(0, empty.ToInteger());

        var sized = new BitsetType(65);
        Assert.Equal(65, sized.Length);
        Assert.Equal(128, sized.Capacity);
        Assert.False(sized.Any());
        Assert.False(sized.Test(64));
    }

    [Fact]
    public void FromIntegerSupportsSmallAndCrossWordValues()
    {
        var five = BitsetType.FromInteger((UInt128)5);
        Assert.Equal(3, five.Length);
        Assert.True(five.Test(0));
        Assert.False(five.Test(1));
        Assert.True(five.Test(2));
        Assert.Equal("101", five.ToBinaryString());
        Assert.Equal<ulong?>(5, five.ToInteger());

        var crossWord = BitsetType.FromInteger((UInt128.One << 64) | UInt128.One);
        Assert.Equal(65, crossWord.Length);
        Assert.True(crossWord.Test(0));
        Assert.True(crossWord.Test(64));
        Assert.Null(crossWord.ToInteger());
    }

    [Fact]
    public void FromBinaryStringUnderstandsConventionalBitOrder()
    {
        var bitset = BitsetType.FromBinaryString("00101");
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
        var bitset = new BitsetType(10);

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
        var bitset = new BitsetType(8);
        bitset.Set(1);

        bitset.Clear(100);

        Assert.True(bitset.Test(1));
        Assert.False(bitset.Test(100));
        Assert.Equal(8, bitset.Length);
    }

    [Fact]
    public void SetAndToggleAutoGrowWithDoublingCapacity()
    {
        var bitset = new BitsetType(100);

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
        var left = BitsetType.FromInteger((UInt128)0b1100);
        var right = BitsetType.FromInteger((UInt128)0b1010);

        Assert.Equal("1000", left.And(right).ToBinaryString());
        Assert.Equal("1110", left.Or(right).ToBinaryString());
        Assert.Equal("0110", left.Xor(right).ToBinaryString());
        Assert.Equal("0100", left.AndNot(right).ToBinaryString());

        Assert.Equal("1000", (left & right).ToBinaryString());
        Assert.Equal("1110", (left | right).ToBinaryString());
        Assert.Equal("0110", (left ^ right).ToBinaryString());
    }

    [Fact]
    public void BulkOperationsZeroExtendShorterInputs()
    {
        var shortBitset = BitsetType.FromBinaryString("101");
        var longBitset = BitsetType.FromBinaryString("100001");

        Assert.Equal("100101", shortBitset.Or(longBitset).ToBinaryString());
        Assert.Equal("000001", shortBitset.And(longBitset).ToBinaryString());
        Assert.Equal("100100", shortBitset.Xor(longBitset).ToBinaryString());
        Assert.Equal(6, shortBitset.Or(longBitset).Length);
    }

    [Fact]
    public void NotOnlyFlipsBitsInsideLogicalLength()
    {
        var bitset = new BitsetType(5);
        bitset.Set(0);
        bitset.Set(2);

        var inverted = bitset.Not();

        Assert.Equal("11010", inverted.ToBinaryString());
        Assert.Equal(3, inverted.PopCount());
        Assert.False(inverted.Test(5));
        Assert.Equal(bitset, ~inverted);
    }

    [Fact]
    public void AnyAllNoneAndIsEmptyReflectCurrentState()
    {
        var empty = new BitsetType(0);
        Assert.False(empty.Any());
        Assert.True(empty.All());
        Assert.True(empty.None());
        Assert.True(empty.IsEmpty);

        var partial = new BitsetType(5);
        partial.Set(0);
        partial.Set(4);
        Assert.True(partial.Any());
        Assert.False(partial.All());
        Assert.False(partial.None());

        var full = BitsetType.FromBinaryString("11111");
        Assert.True(full.All());
        Assert.False(full.None());
    }

    [Fact]
    public void IterSetBitsYieldsAscendingIndicesAcrossWords()
    {
        var bitset = new BitsetType(130);
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
        var bitset = new BitsetType(8);
        bitset.Set(0);
        bitset.Set(7);

        Assert.Equal("10000001", bitset.ToBinaryString());
        Assert.Equal(bitset, BitsetType.FromBinaryString(bitset.ToBinaryString()));
    }

    [Fact]
    public void EqualityIgnoresInternalStorageChoicesAndUsesLogicalBits()
    {
        var left = BitsetType.FromBinaryString("101001");
        var right = new BitsetType(2);
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
        var bitset = new BitsetType(4);
        Assert.Throws<ArgumentOutOfRangeException>(() => bitset.Set(-1));
        Assert.Throws<ArgumentOutOfRangeException>(() => bitset.Clear(-1));
        Assert.Throws<ArgumentOutOfRangeException>(() => bitset.Test(-1));
        Assert.Throws<ArgumentOutOfRangeException>(() => bitset.Toggle(-1));
    }

    [Fact]
    public void ToStringUsesBinaryForm()
    {
        var bitset = BitsetType.FromBinaryString("101");
        Assert.Equal("Bitset(101)", bitset.ToString());
    }
}
