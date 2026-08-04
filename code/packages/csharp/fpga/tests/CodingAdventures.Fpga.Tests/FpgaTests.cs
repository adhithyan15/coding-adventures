using System.Text.Json;
using CodingAdventures.Fpga;

namespace CodingAdventures.Fpga.Tests;

public sealed class FpgaTests
{
    private static int[] Zeros(int k = 4) => new int[1 << k];

    private static int[] AndTable(int k = 4)
    {
        var table = Zeros(k);
        for (var index = 0; index < table.Length; index++)
        {
            table[index] = (index & 3) == 3 ? 1 : 0;
        }

        return table;
    }

    private static int[] XorTable(int k = 4)
    {
        var table = Zeros(k);
        for (var index = 0; index < table.Length; index++)
        {
            table[index] = ((index & 1) ^ ((index >> 1) & 1));
        }

        return table;
    }

    private static Bitstream MakeBitstream()
    {
        var slice0 = new SliceConfig(AndTable(), Zeros());
        var slice1 = new SliceConfig(Zeros(), Zeros());
        return new Bitstream(
            new Dictionary<string, CLBConfig> { ["clb0"] = new(slice0, slice1) },
            new Dictionary<string, IReadOnlyList<RouteConfig>>
            {
                ["switch0"] = [new("clb_out", "east"), new("north", "south")],
            },
            new Dictionary<string, IOConfig>
            {
                ["in"] = new("input"),
                ["out"] = new("output"),
                ["tri"] = new("tristate"),
            });
    }

    [Fact]
    public void LutImplementsAndTruthTable()
    {
        var lut = new LUT(4, AndTable());
        Assert.Equal(0, lut.Evaluate([0, 0, 0, 0]));
        Assert.Equal(0, lut.Evaluate([1, 0, 0, 0]));
        Assert.Equal(0, lut.Evaluate([0, 1, 0, 0]));
        Assert.Equal(1, lut.Evaluate([1, 1, 0, 0]));
    }

    [Fact]
    public void LutImplementsXorTruthTable()
    {
        var lut = new LUT(4, XorTable());
        Assert.Equal(1, lut.Evaluate([1, 0, 0, 0]));
        Assert.Equal(1, lut.Evaluate([0, 1, 0, 0]));
        Assert.Equal(0, lut.Evaluate([1, 1, 0, 0]));
    }

    [Fact]
    public void LutDefaultsToZeros()
    {
        var lut = new LUT(2);
        Assert.Equal(2, lut.K);
        Assert.All(new[] { new[] { 0, 0 }, [1, 0], [0, 1], [1, 1] }, inputs => Assert.Equal(0, lut.Evaluate(inputs)));
    }

    [Fact]
    public void LutCanBeReconfigured()
    {
        var lut = new LUT(4, AndTable());
        lut.Configure(XorTable());
        Assert.Equal(0, lut.Evaluate([1, 1, 0, 0]));
        Assert.Equal(1, lut.Evaluate([1, 0, 0, 0]));
    }

    [Fact]
    public void LutTruthTableIsDefensivelyCopied()
    {
        var source = AndTable();
        var lut = new LUT(4, source);
        source[3] = 0;
        var snapshot = lut.TruthTable.ToArray();
        snapshot[3] = 0;
        Assert.Equal(1, lut.Evaluate([1, 1, 0, 0]));
    }

    [Theory]
    [InlineData(1)]
    [InlineData(7)]
    public void LutRejectsInvalidWidths(int k) => Assert.Throws<ArgumentOutOfRangeException>(() => new LUT(k));

    [Fact]
    public void LutRejectsWrongTruthTableLength() =>
        Assert.Throws<ArgumentException>(() => new LUT(4).Configure([0, 1]));

    [Fact]
    public void LutRejectsNonBitTruthTableValues()
    {
        var table = Zeros();
        table[5] = 2;
        Assert.Throws<ArgumentOutOfRangeException>(() => new LUT(4, table));
    }

    [Fact]
    public void LutRejectsInvalidInputs()
    {
        var lut = new LUT(4);
        Assert.Throws<ArgumentException>(() => lut.Evaluate([0, 1]));
        Assert.Throws<ArgumentOutOfRangeException>(() => lut.Evaluate([0, 0, -1, 0]));
        Assert.Throws<ArgumentNullException>(() => lut.Evaluate(null!));
    }

    [Fact]
    public void SliceEvaluatesIndependentCombinationalLuts()
    {
        var slice = new Slice();
        slice.Configure(AndTable(), XorTable());
        var result = slice.Evaluate([1, 1, 0, 0], [1, 0, 0, 0], 0);
        Assert.Equal(new SliceOutput(1, 1, 0), result);
    }

    [Fact]
    public void SliceRegistersOutputsAcrossHighThenLowClock()
    {
        var slice = new Slice();
        slice.Configure(AndTable(), AndTable(), true, true);
        Assert.Equal(new SliceOutput(0, 0, 0), slice.Evaluate([1, 1, 0, 0], [1, 1, 0, 0], 1));
        Assert.Equal(new SliceOutput(1, 1, 0), slice.Evaluate([1, 1, 0, 0], [1, 1, 0, 0], 0));
    }

    [Fact]
    public void SliceReconfigurationResetsRegisters()
    {
        var slice = new Slice();
        slice.Configure(AndTable(), AndTable(), true, true);
        slice.Evaluate([1, 1, 0, 0], [1, 1, 0, 0], 1);
        slice.Evaluate([1, 1, 0, 0], [1, 1, 0, 0], 0);
        slice.Configure(AndTable(), AndTable(), true, true);
        Assert.Equal(0, slice.Evaluate([1, 1, 0, 0], [1, 1, 0, 0], 1).OutputA);
    }

    [Fact]
    public void SliceCarryChainGeneratesPropagatesAndBlocks()
    {
        var slice = new Slice();
        slice.Configure(AndTable(), AndTable(), carryEnabled: true);
        Assert.Equal(1, slice.Evaluate([1, 1, 0, 0], [1, 1, 0, 0], 0).CarryOut);
        Assert.Equal(1, slice.Evaluate([1, 1, 0, 0], [0, 0, 0, 0], 0, 1).CarryOut);
        Assert.Equal(0, slice.Evaluate([0, 0, 0, 0], [0, 0, 0, 0], 0, 1).CarryOut);
    }

    [Fact]
    public void SliceValidatesClockAndCarry()
    {
        var slice = new Slice();
        Assert.Throws<ArgumentOutOfRangeException>(() => slice.Evaluate([0, 0, 0, 0], [0, 0, 0, 0], 2));
        Assert.Throws<ArgumentOutOfRangeException>(() => slice.Evaluate([0, 0, 0, 0], [0, 0, 0, 0], 0, -1));
    }

    [Fact]
    public void ClbEvaluatesSlicesIndependently()
    {
        var clb = new CLB();
        clb.Slice0.Configure(AndTable(), AndTable());
        clb.Slice1.Configure(AndTable(), AndTable());
        var result = clb.Evaluate([1, 1, 0, 0], [1, 1, 0, 0], [0, 1, 0, 0], [1, 0, 0, 0], 0);
        Assert.Equal(1, result.Slice0.OutputA);
        Assert.Equal(0, result.Slice1.OutputA);
        Assert.Equal(4, clb.K);
    }

    [Fact]
    public void ClbChainsCarryBetweenSlices()
    {
        var clb = new CLB();
        clb.Slice0.Configure(AndTable(), AndTable(), carryEnabled: true);
        clb.Slice1.Configure(AndTable(), AndTable(), carryEnabled: true);
        var ones = new[] { 1, 1, 0, 0 };
        var result = clb.Evaluate(ones, ones, ones, ones, 0);
        Assert.Equal(1, result.Slice0.CarryOut);
        Assert.Equal(1, result.Slice1.CarryOut);
    }

    [Fact]
    public void SwitchMatrixRoutesConnectedSignals()
    {
        var matrix = new SwitchMatrix(["north", "south", "east", "out"]);
        matrix.Connect("out", "east");
        matrix.Connect("north", "south");
        var routed = matrix.Route(new Dictionary<string, int> { ["out"] = 1, ["north"] = 0 });
        Assert.Equal(1, routed["east"]);
        Assert.Equal(0, routed["south"]);
    }

    [Fact]
    public void SwitchMatrixSupportsFanOut()
    {
        var matrix = new SwitchMatrix(["source", "a", "b"]);
        matrix.Connect("source", "a");
        matrix.Connect("source", "b");
        var routed = matrix.Route(new Dictionary<string, int> { ["source"] = 1 });
        Assert.Equal(1, routed["a"]);
        Assert.Equal(1, routed["b"]);
    }

    [Fact]
    public void SwitchMatrixOmitsDestinationsWithoutSourceValues()
    {
        var matrix = new SwitchMatrix(["a", "b"]);
        matrix.Connect("a", "b");
        Assert.Empty(matrix.Route(new Dictionary<string, int> { ["b"] = 1 }));
    }

    [Fact]
    public void SwitchMatrixDisconnectsAndClears()
    {
        var matrix = new SwitchMatrix(["a", "b", "c"]);
        matrix.Connect("a", "b");
        matrix.Disconnect("b");
        Assert.Equal(0, matrix.ConnectionCount);
        matrix.Connect("a", "b");
        matrix.Connect("a", "c");
        matrix.Clear();
        Assert.Empty(matrix.Connections);
    }

    [Fact]
    public void SwitchMatrixExposesPortsAndConnectionSnapshots()
    {
        var matrix = new SwitchMatrix(["a", "b"]);
        matrix.Connect("a", "b");
        Assert.Equal(2, matrix.Ports.Count);
        Assert.Equal("a", matrix.Connections["b"]);
    }

    [Fact]
    public void SwitchMatrixRejectsInvalidPortSets()
    {
        Assert.Throws<ArgumentNullException>(() => new SwitchMatrix(null!));
        Assert.Throws<ArgumentException>(() => new SwitchMatrix([]));
        Assert.Throws<ArgumentException>(() => new SwitchMatrix([""]));
    }

    [Fact]
    public void SwitchMatrixRejectsInvalidConnections()
    {
        var matrix = new SwitchMatrix(["a", "b", "c"]);
        Assert.Throws<ArgumentException>(() => matrix.Connect("x", "a"));
        Assert.Throws<ArgumentException>(() => matrix.Connect("a", "x"));
        Assert.Throws<ArgumentException>(() => matrix.Connect("a", "a"));
        matrix.Connect("a", "b");
        Assert.Throws<InvalidOperationException>(() => matrix.Connect("c", "b"));
    }

    [Fact]
    public void SwitchMatrixRejectsInvalidDisconnectsAndBits()
    {
        var matrix = new SwitchMatrix(["a", "b", "c"]);
        Assert.Throws<ArgumentException>(() => matrix.Disconnect("x"));
        Assert.Throws<InvalidOperationException>(() => matrix.Disconnect("c"));
        matrix.Connect("a", "b");
        Assert.Throws<ArgumentOutOfRangeException>(() => matrix.Route(new Dictionary<string, int> { ["a"] = 2 }));
    }

    [Fact]
    public void IoBlockInputModeReadsPad()
    {
        var io = new IOBlock("sensor");
        io.DrivePad(1);
        Assert.Equal(1, io.ReadInternal());
        Assert.Equal(1, io.ReadPad());
    }

    [Fact]
    public void IoBlockOutputModeDrivesPad()
    {
        var io = new IOBlock("led", IOMode.Output);
        io.DriveInternal(1);
        Assert.Equal(1, io.ReadInternal());
        Assert.Equal(1, io.ReadPad());
    }

    [Fact]
    public void IoBlockCanBeReconfiguredToTristate()
    {
        var io = new IOBlock("bus", IOMode.Output);
        io.DriveInternal(1);
        io.Configure(IOMode.Tristate);
        Assert.Null(io.ReadPad());
        Assert.Equal(IOMode.Tristate, io.Mode);
    }

    [Fact]
    public void IoBlockValidatesNameAndBits()
    {
        Assert.Throws<ArgumentException>(() => new IOBlock(" "));
        var io = new IOBlock("pin");
        Assert.Throws<ArgumentOutOfRangeException>(() => io.DrivePad(2));
        Assert.Throws<ArgumentOutOfRangeException>(() => io.DriveInternal(-1));
        Assert.Equal("pin", io.Name);
    }

    [Fact]
    public void EmptyBitstreamUsesRequestedLutWidth()
    {
        var bitstream = Bitstream.Empty(3);
        Assert.Equal(3, bitstream.LutK);
        Assert.Empty(bitstream.Clbs);
        Assert.Empty(bitstream.Routing);
        Assert.Empty(bitstream.IO);
    }

    [Fact]
    public void BitstreamDefensivelyCopiesCollections()
    {
        var clbs = new Dictionary<string, CLBConfig>();
        var bitstream = new Bitstream(clbs: clbs);
        clbs["later"] = new(new SliceConfig(), new SliceConfig());
        Assert.Empty(bitstream.Clbs);
    }

    [Fact]
    public void BitstreamRejectsInvalidLutWidths() =>
        Assert.Throws<ArgumentOutOfRangeException>(() => Bitstream.Empty(8));

    [Fact]
    public void JsonBitstreamParsesAllConfigurationSections()
    {
        var bitstream = Bitstream.ParseJson("""
            {"lut_k":2,"clbs":{"c":{"slice0":{"lut_a":[0,0,0,1],"ff_a":true}}},
             "routing":{"s":[{"src":"a","dst":"b"}]},
             "io":{"in":{"mode":"input"},"out":{"mode":"output"}}}
            """);
        Assert.Equal(2, bitstream.LutK);
        Assert.True(bitstream.Clbs["c"].Slice0.FlipFlopAEnabled);
        Assert.Equal(1, bitstream.Clbs["c"].Slice0.LutA[3]);
        Assert.Equal("b", Assert.Single(bitstream.Routing["s"]).Destination);
        Assert.Equal("output", bitstream.IO["out"].Mode);
    }

    [Fact]
    public void JsonBitstreamSuppliesMissingDefaults()
    {
        var bitstream = Bitstream.ParseJson("{\"clbs\":{\"c\":{\"slice0\":{\"ff_b\":true}}}}");
        Assert.Equal(4, bitstream.LutK);
        Assert.Equal(16, bitstream.Clbs["c"].Slice0.LutA.Count);
        Assert.Equal(16, bitstream.Clbs["c"].Slice1.LutB.Count);
        Assert.True(bitstream.Clbs["c"].Slice0.FlipFlopBEnabled);
    }

    [Fact]
    public void JsonBitstreamRejectsMalformedDocuments()
    {
        Assert.Throws<ArgumentNullException>(() => Bitstream.ParseJson(null!));
        Assert.Throws<JsonException>(() => Bitstream.ParseJson("[]"));
        Assert.Throws<JsonException>(() => Bitstream.ParseJson("{\"lut_k\":9}"));
        Assert.ThrowsAny<JsonException>(() => Bitstream.ParseJson("{invalid"));
    }

    [Fact]
    public void FpgaEvaluatesConfiguredClbs()
    {
        var fpga = new FPGA(MakeBitstream());
        var result = fpga.EvaluateCLB("clb0", [1, 1, 0, 0], Zeros()[..4], Zeros()[..4], Zeros()[..4], 0);
        Assert.Equal(1, result.Slice0.OutputA);
        Assert.Single(fpga.Clbs);
    }

    [Fact]
    public void FpgaRoutesConfiguredSignals()
    {
        var fpga = new FPGA(MakeBitstream());
        var routed = fpga.Route("switch0", new Dictionary<string, int> { ["clb_out"] = 1, ["north"] = 0 });
        Assert.Equal(1, routed["east"]);
        Assert.Equal(0, routed["south"]);
        Assert.Single(fpga.Switches);
    }

    [Fact]
    public void FpgaDrivesInputOutputAndTristatePins()
    {
        var fpga = new FPGA(MakeBitstream());
        fpga.SetInput("in", 1);
        Assert.Equal(1, fpga.ReadOutput("in"));
        fpga.DriveOutput("out", 1);
        Assert.Equal(1, fpga.ReadOutput("out"));
        Assert.Null(fpga.ReadOutput("tri"));
        Assert.Equal(3, fpga.IOBlocks.Count);
    }

    [Fact]
    public void FpgaRejectsUnknownResources()
    {
        var fpga = new FPGA(MakeBitstream());
        Assert.Throws<KeyNotFoundException>(() => fpga.EvaluateCLB("missing", Zeros()[..4], Zeros()[..4], Zeros()[..4], Zeros()[..4], 0));
        Assert.Throws<KeyNotFoundException>(() => fpga.Route("missing", new Dictionary<string, int>()));
        Assert.Throws<KeyNotFoundException>(() => fpga.SetInput("missing", 0));
        Assert.Throws<KeyNotFoundException>(() => fpga.DriveOutput("missing", 0));
        Assert.Throws<KeyNotFoundException>(() => fpga.ReadOutput("missing"));
    }

    [Fact]
    public void FpgaAcceptsAnEmptyBitstream()
    {
        var bitstream = Bitstream.Empty();
        var fpga = new FPGA(bitstream);
        Assert.Same(bitstream, fpga.Bitstream);
        Assert.Empty(fpga.Clbs);
        Assert.Empty(fpga.Switches);
        Assert.Empty(fpga.IOBlocks);
        Assert.Throws<ArgumentNullException>(() => new FPGA(null!));
    }
}
