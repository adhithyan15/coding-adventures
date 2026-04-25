namespace CodingAdventures.LogicGates.Tests;

public sealed class LogicGatesTests
{
    private static readonly (int A, int B, int NotA, int And, int Or, int Xor, int Nand, int Nor, int Xnor)[] TruthTable =
    [
        (0, 0, 1, 0, 0, 0, 1, 1, 1),
        (0, 1, 1, 0, 1, 1, 1, 0, 0),
        (1, 0, 0, 0, 1, 1, 1, 0, 0),
        (1, 1, 0, 1, 1, 0, 0, 0, 1),
    ];

    [Fact]
    public void FundamentalGatesMatchTruthTables()
    {
        foreach (var row in TruthTable)
        {
            Assert.Equal(row.NotA, LogicGates.Not(row.A));
            Assert.Equal(row.And, LogicGates.And(row.A, row.B));
            Assert.Equal(row.Or, LogicGates.Or(row.A, row.B));
            Assert.Equal(row.Xor, LogicGates.Xor(row.A, row.B));
            Assert.Equal(row.Nand, LogicGates.Nand(row.A, row.B));
            Assert.Equal(row.Nor, LogicGates.Nor(row.A, row.B));
            Assert.Equal(row.Xnor, LogicGates.Xnor(row.A, row.B));
        }
    }

    [Fact]
    public void NandDerivedGatesMatchDirectGates()
    {
        foreach (var row in TruthTable)
        {
            Assert.Equal(LogicGates.Not(row.A), LogicGates.NandNot(row.A));
            Assert.Equal(LogicGates.And(row.A, row.B), LogicGates.NandAnd(row.A, row.B));
            Assert.Equal(LogicGates.Or(row.A, row.B), LogicGates.NandOr(row.A, row.B));
            Assert.Equal(LogicGates.Xor(row.A, row.B), LogicGates.NandXor(row.A, row.B));
        }
    }

    [Fact]
    public void MultiInputGatesWork()
    {
        Assert.Equal(1, LogicGates.AndN(1, 1, 1, 1));
        Assert.Equal(0, LogicGates.AndN(1, 1, 0, 1));
        Assert.Equal(0, LogicGates.OrN(0, 0, 0));
        Assert.Equal(1, LogicGates.OrN(0, 0, 1, 0));
        Assert.Equal(0, LogicGates.XorN());
        Assert.Equal(1, LogicGates.XorN(1));
        Assert.Equal(0, LogicGates.XorN(1, 1, 1, 1));
        Assert.Equal(1, LogicGates.XorN(1, 1, 1));
    }

    [Fact]
    public void InvalidInputsAreRejected()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => LogicGates.Not(-1));
        Assert.Throws<ArgumentOutOfRangeException>(() => LogicGates.And(2, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => LogicGates.Or(0, -1));
        Assert.Throws<ArgumentOutOfRangeException>(() => LogicGates.Xor(0, 2));
        Assert.Throws<ArgumentException>(() => LogicGates.AndN(1));
        Assert.Throws<ArgumentException>(() => LogicGates.OrN());
        Assert.Throws<ArgumentOutOfRangeException>(() => LogicGates.XorN(0, 2));
        Assert.Throws<ArgumentNullException>(() => LogicGates.AndN(null!));
    }

    [Fact]
    public void DeMorganRelationshipsHold()
    {
        foreach (var row in TruthTable)
        {
            Assert.Equal(LogicGates.Nand(row.A, row.B), LogicGates.Or(LogicGates.Not(row.A), LogicGates.Not(row.B)));
            Assert.Equal(LogicGates.Nor(row.A, row.B), LogicGates.And(LogicGates.Not(row.A), LogicGates.Not(row.B)));
            Assert.Equal(LogicGates.Xnor(row.A, row.B), LogicGates.Not(LogicGates.Xor(row.A, row.B)));
        }
    }
}
