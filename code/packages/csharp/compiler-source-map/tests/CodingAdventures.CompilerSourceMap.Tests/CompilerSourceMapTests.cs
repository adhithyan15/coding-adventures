namespace CodingAdventures.CompilerSourceMap.Tests;

public sealed class CompilerSourceMapTests
{
    private static SourceMapChain BuildTestChain()
    {
        var chain = SourceMapChain.New();
        chain.SourceToAst.Add(new SourcePosition("hello.bf", 1, 1, 1), 0);
        chain.SourceToAst.Add(new SourcePosition("hello.bf", 1, 2, 1), 1);
        chain.AstToIr.Add(0, [2, 3, 4, 5]);
        chain.AstToIr.Add(1, [6, 7]);

        var identity = new IrToIr("identity");
        for (var i = 0L; i < 8; i++)
        {
            identity.AddMapping(i, [i]);
        }

        chain.AddOptimizerPass(identity);

        var machineCode = new IrToMachineCode();
        machineCode.Add(0, 0x00, 8);
        machineCode.Add(1, 0x08, 4);
        machineCode.Add(2, 0x0C, 8);
        machineCode.Add(3, 0x14, 4);
        machineCode.Add(4, 0x18, 4);
        machineCode.Add(5, 0x1C, 8);
        machineCode.Add(6, 0x24, 8);
        machineCode.Add(7, 0x2C, 8);
        chain.IrToMachineCode = machineCode;
        return chain;
    }

    [Fact]
    public void SourcePositionFormatsAndCompares()
    {
        var position = new SourcePosition("hello.bf", 1, 3, 1);

        Assert.Equal("hello.bf:1:3 (len=1)", position.ToString());
        Assert.Equal(position, new SourcePosition("hello.bf", 1, 3, 1));
        Assert.NotEqual(position, new SourcePosition("hello.bf", 1, 4, 1));
    }

    [Fact]
    public void SourceToAstAddsAndLooksUpNodeIds()
    {
        var segment = new SourceToAst();
        var position = new SourcePosition("a.bf", 1, 1, 1);

        segment.Add(position, 42);

        Assert.Equal(position, segment.LookupByNodeId(42));
        Assert.Null(segment.LookupByNodeId(999));
    }

    [Fact]
    public void AstToIrSupportsForwardAndReverseLookup()
    {
        var segment = new AstToIr();

        segment.Add(5, [20, 21]);
        segment.Add(6, [22]);

        Assert.Equal([20L, 21L], segment.LookupByAstNodeId(5));
        Assert.Equal(5, segment.LookupByIrId(21));
        Assert.Equal(6, segment.LookupByIrId(22));
        Assert.Equal(-1, segment.LookupByIrId(999));
    }

    [Fact]
    public void IrToIrHandlesMappingsDeletionAndReverseLookup()
    {
        var segment = new IrToIr("contraction");

        segment.AddMapping(7, [100]);
        segment.AddMapping(8, [100]);
        segment.AddDeletion(9);

        Assert.Equal([100L], segment.LookupByOriginalId(7));
        Assert.Equal(7, segment.LookupByNewId(100));
        Assert.Null(segment.LookupByOriginalId(9));
        Assert.Contains(9, segment.Deleted);
        Assert.Equal("contraction", segment.PassName);
    }

    [Fact]
    public void IrToMachineCodeFindsInstructionRanges()
    {
        var segment = new IrToMachineCode();

        segment.Add(5, 0x20, 4);
        segment.Add(6, 0x24, 8);

        Assert.Equal((0x20L, 4L), segment.LookupByIrId(5));
        Assert.Equal((-1L, 0L), segment.LookupByIrId(999));
        Assert.Equal(5, segment.LookupByMachineCodeOffset(0x23));
        Assert.Equal(6, segment.LookupByMachineCodeOffset(0x24));
        Assert.Equal(-1, segment.LookupByMachineCodeOffset(0xFF));
    }

    [Fact]
    public void NewChainStartsEmpty()
    {
        var chain = SourceMapChain.New();

        Assert.Empty(chain.SourceToAst.Entries);
        Assert.Empty(chain.AstToIr.Entries);
        Assert.Empty(chain.IrToIr);
        Assert.Null(chain.IrToMachineCode);
    }

    [Fact]
    public void ForwardLookupComposesAllSegments()
    {
        var chain = BuildTestChain();

        var results = chain.SourceToMc(new SourcePosition("hello.bf", 1, 1, 1));

        Assert.NotNull(results);
        Assert.Equal([2L, 3L, 4L, 5L], results.Select(entry => entry.IrId));
        Assert.Equal(0x0C, results[0].MachineCodeOffset);
    }

    [Fact]
    public void ReverseLookupComposesSegmentsBackToSource()
    {
        var chain = BuildTestChain();

        var plus = chain.McToSource(0x14);
        var dot = chain.McToSource(0x2C);

        Assert.Equal(new SourcePosition("hello.bf", 1, 1, 1), plus);
        Assert.Equal(new SourcePosition("hello.bf", 1, 2, 1), dot);
    }

    [Fact]
    public void IncompleteOrMissingMappingsReturnNull()
    {
        var chain = SourceMapChain.New();
        chain.SourceToAst.Add(new SourcePosition("a.bf", 1, 1, 1), 0);
        chain.AstToIr.Add(0, [1, 2]);

        Assert.Null(chain.SourceToMc(new SourcePosition("a.bf", 1, 1, 1)));
        Assert.Null(chain.McToSource(0));

        chain.IrToMachineCode = new IrToMachineCode();
        Assert.Null(chain.SourceToMc(new SourcePosition("a.bf", 9, 1, 1)));
    }

    [Fact]
    public void OptimizerContractionFeedsForwardLookup()
    {
        var chain = SourceMapChain.New();
        var position = new SourcePosition("t.bf", 1, 1, 1);
        chain.SourceToAst.Add(position, 0);
        chain.AstToIr.Add(0, [1, 2, 3]);

        var pass = new IrToIr("contraction");
        pass.AddMapping(1, [100]);
        pass.AddMapping(2, [100]);
        pass.AddMapping(3, [100]);
        chain.AddOptimizerPass(pass);

        var machineCode = new IrToMachineCode();
        machineCode.Add(100, 0, 4);
        chain.IrToMachineCode = machineCode;

        var results = chain.SourceToMc(position);

        Assert.NotNull(results);
        Assert.Equal(3, results.Count);
        Assert.All(results, entry => Assert.Equal(100, entry.IrId));
    }

    [Fact]
    public void OptimizerDeletionDropsForwardResults()
    {
        var chain = SourceMapChain.New();
        var position = new SourcePosition("t.bf", 1, 1, 1);
        chain.SourceToAst.Add(position, 0);
        chain.AstToIr.Add(0, [1, 2]);

        var pass = new IrToIr("dead-store");
        pass.AddMapping(1, [1]);
        pass.AddDeletion(2);
        chain.AddOptimizerPass(pass);

        var machineCode = new IrToMachineCode();
        machineCode.Add(1, 0, 4);
        machineCode.Add(2, 4, 4);
        chain.IrToMachineCode = machineCode;

        var results = chain.SourceToMc(position);

        Assert.NotNull(results);
        Assert.Single(results);
        Assert.Equal(1, results[0].IrId);
    }

    [Fact]
    public void ReverseLookupFailsWhenOptimizerTraceIsMissing()
    {
        var chain = SourceMapChain.New();
        chain.SourceToAst.Add(new SourcePosition("t.bf", 1, 1, 1), 0);
        chain.AstToIr.Add(0, [1]);

        var pass = new IrToIr("renumber");
        pass.AddMapping(1, [100]);
        chain.AddOptimizerPass(pass);

        var machineCode = new IrToMachineCode();
        machineCode.Add(200, 0, 4);
        chain.IrToMachineCode = machineCode;

        Assert.Null(chain.McToSource(0));
    }
}
