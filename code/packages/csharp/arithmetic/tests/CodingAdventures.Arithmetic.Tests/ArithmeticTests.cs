namespace CodingAdventures.Arithmetic.Tests;

public sealed class ArithmeticTests
{
    private static int[] IntToBits(int value, int width) =>
        Enumerable.Range(0, width).Select(i => (value >> i) & 1).ToArray();

    private static int BitsToInt(IReadOnlyList<int> bits) =>
        bits.Select((bit, i) => bit << i).Sum();

    [Theory]
    [InlineData(0, 0, 0, 0)]
    [InlineData(0, 1, 1, 0)]
    [InlineData(1, 0, 1, 0)]
    [InlineData(1, 1, 0, 1)]
    public void HalfAdderMatchesTruthTable(int a, int b, int expectedSum, int expectedCarry)
    {
        Assert.Equal((expectedSum, expectedCarry), Adders.HalfAdder(a, b));
    }

    [Theory]
    [InlineData(0, 0, 0, 0, 0)]
    [InlineData(0, 0, 1, 1, 0)]
    [InlineData(0, 1, 1, 0, 1)]
    [InlineData(1, 0, 1, 0, 1)]
    [InlineData(1, 1, 0, 0, 1)]
    [InlineData(1, 1, 1, 1, 1)]
    public void FullAdderMatchesTruthTable(int a, int b, int carryIn, int expectedSum, int expectedCarry)
    {
        Assert.Equal((expectedSum, expectedCarry), Adders.FullAdder(a, b, carryIn));
    }

    [Fact]
    public void RippleCarryAdderAddsAndCarries()
    {
        var result = Adders.RippleCarryAdder(IntToBits(15, 4), IntToBits(1, 4));

        Assert.Equal(0, BitsToInt(result.Sum));
        Assert.Equal(1, result.CarryOut);
    }

    [Fact]
    public void RippleCarryAdderSupportsCarryIn()
    {
        var result = Adders.RippleCarryAdder(IntToBits(1, 4), IntToBits(1, 4), carryIn: 1);

        Assert.Equal(3, BitsToInt(result.Sum));
        Assert.Equal(0, result.CarryOut);
    }

    [Fact]
    public void RippleCarryAdderValidatesInputs()
    {
        Assert.Throws<ArgumentException>(() => Adders.RippleCarryAdder([0, 1], [0, 1, 0]));
        Assert.Throws<ArgumentException>(() => Adders.RippleCarryAdder([], []));
        Assert.Throws<ArgumentOutOfRangeException>(() => Adders.RippleCarryAdder([2], [0]));
    }

    [Fact]
    public void AluAddsSubtractsAndSetsFlags()
    {
        var alu = new Alu(8);

        var add = alu.Execute(AluOp.Add, IntToBits(255, 8), IntToBits(1, 8));
        var sub = alu.Execute(AluOp.Sub, IntToBits(5, 8), IntToBits(3, 8));

        Assert.Equal(0, BitsToInt(add.Value));
        Assert.True(add.Carry);
        Assert.True(add.Zero);
        Assert.Equal(2, BitsToInt(sub.Value));
        Assert.False(sub.Zero);
    }

    [Theory]
    [InlineData(AluOp.And, 0xCC, 0xAA, 0x88)]
    [InlineData(AluOp.Or, 0xCC, 0xAA, 0xEE)]
    [InlineData(AluOp.Xor, 0xCC, 0xAA, 0x66)]
    public void AluBitwiseOperations(AluOp op, int a, int b, int expected)
    {
        var result = new Alu(8).Execute(op, IntToBits(a, 8), IntToBits(b, 8));

        Assert.Equal(expected, BitsToInt(result.Value));
        Assert.False(result.Carry);
        Assert.False(result.Overflow);
    }

    [Fact]
    public void AluNotIgnoresB()
    {
        var result = new Alu(8).Execute(AluOp.Not, IntToBits(0, 8), []);

        Assert.Equal(255, BitsToInt(result.Value));
        Assert.True(result.Negative);
    }

    [Fact]
    public void AluDetectsSignedOverflow()
    {
        var result = new Alu(8).Execute(AluOp.Add, IntToBits(127, 8), IntToBits(1, 8));

        Assert.True(result.Overflow);
        Assert.True(result.Negative);
        Assert.Same(result.Value, result.Result);
    }

    [Fact]
    public void AluValidatesWidth()
    {
        var alu = new Alu(8);

        Assert.Throws<ArgumentException>(() => alu.Execute(AluOp.Add, [0, 1], [0, 1]));
        Assert.Throws<ArgumentException>(() => alu.Execute(AluOp.And, IntToBits(1, 8), [1]));
        Assert.Throws<ArgumentOutOfRangeException>(() => new Alu(0));
    }
}
