using CodingAdventures.HuffmanTree;

namespace CodingAdventures.HuffmanTree.Tests;

public sealed class HuffmanTreeTests
{
    [Fact]
    public void Build_RejectsEmptyWeights()
    {
        var error = Assert.Throws<ArgumentException>(() => HuffmanTree.Build([]));
        Assert.Contains("weights must not be empty", error.Message);
    }

    [Fact]
    public void Build_RejectsNonPositiveFrequencies()
    {
        var error = Assert.Throws<ArgumentException>(() => HuffmanTree.Build([(42, 0)]));
        Assert.Contains("symbol=42, freq=0", error.Message);
    }

    [Fact]
    public void SingleSymbolTree_UsesZeroCodeByConvention()
    {
        var tree = HuffmanTree.Build([(65, 5)]);

        Assert.Equal(1, tree.SymbolCount());
        Assert.Equal(5, tree.Weight());
        Assert.Equal(0, tree.Depth());
        Assert.Equal("0", tree.CodeTable()[65]);
        Assert.Equal("0", tree.CanonicalCodeTable()[65]);
        Assert.Equal([65, 65, 65], tree.DecodeAll("000", 3));
        Assert.Equal([65], tree.DecodeAll(string.Empty, 1));
        Assert.True(tree.IsValid());
    }

    [Fact]
    public void ClassicThreeSymbolExample_HasDeterministicCodes()
    {
        var tree = HuffmanTree.Build([(65, 3), (66, 2), (67, 1)]);
        var codes = tree.CodeTable();

        Assert.Equal("0", codes[65]);
        Assert.Equal("10", codes[67]);
        Assert.Equal("11", codes[66]);
        Assert.Equal("10", tree.CodeFor(67));
        Assert.Null(tree.CodeFor(99));
        Assert.Equal([(65, "0"), (67, "10"), (66, "11")], tree.Leaves());
        Assert.True(tree.IsValid());
    }

    [Fact]
    public void CanonicalCodeTable_SortsByLengthThenSymbol()
    {
        var tree = HuffmanTree.Build([(65, 3), (66, 2), (67, 1)]);
        var canonical = tree.CanonicalCodeTable();

        Assert.Equal("0", canonical[65]);
        Assert.Equal("10", canonical[66]);
        Assert.Equal("11", canonical[67]);
    }

    [Fact]
    public void DecodeAll_ThrowsWhenBitsRunOutMidSymbol()
    {
        var tree = HuffmanTree.Build([(65, 3), (66, 2), (67, 1)]);

        var error = Assert.Throws<InvalidOperationException>(() => tree.DecodeAll("1", 1));
        Assert.Contains("exhausted", error.Message);
    }

    [Fact]
    public void DecodeAll_RejectsNonBinaryCharacters()
    {
        var tree = HuffmanTree.Build([(65, 3), (66, 2)]);

        var error = Assert.Throws<InvalidOperationException>(() => tree.DecodeAll("2", 1));
        Assert.Contains("only '0' and '1'", error.Message);
    }

    [Fact]
    public void TwoSymbolTree_WeightDepthAndDecodeMatchTreeShape()
    {
        var tree = HuffmanTree.Build([(65, 3), (66, 1)]);
        var bits = string.Concat(tree.CodeTable()[66], tree.CodeTable()[65], tree.CodeTable()[66]);

        Assert.Equal(2, tree.SymbolCount());
        Assert.Equal(4, tree.Weight());
        Assert.Equal(1, tree.Depth());
        Assert.Equal([66, 65, 66], tree.DecodeAll(bits, 3));
    }
}
