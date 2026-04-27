using System;
using CodingAdventures.WasmSimulator;
using Xunit;

namespace CodingAdventures.WasmSimulator.Tests;

public class WasmSimulatorTests
{
    [Fact]
    public void HasVersion() => Assert.Equal("0.1.0", WasmSimulatorVersion.VERSION);

    [Fact]
    public void EncodingHelpersProduceExpectedBytes()
    {
        Assert.Equal(new byte[] { 0x6A }, WasmSimulator.EncodeI32Add());
        Assert.Equal(new byte[] { 0x21, 0x02 }, WasmSimulator.EncodeLocalSet(2));
    }

    [Fact]
    public void DecoderReadsI32Const()
    {
        var decoder = new WasmDecoder();
        var instruction = decoder.Decode(WasmSimulator.EncodeI32Const(42), 0);
        Assert.Equal("i32.const", instruction.Mnemonic);
        Assert.Equal(42, instruction.Operand);
        Assert.Equal(5, instruction.Size);
    }

    [Fact]
    public void SimulatorRunsSimpleProgram()
    {
        var simulator = new WasmSimulator(4);
        var program = WasmSimulator.AssembleWasm(
        [
            WasmSimulator.EncodeI32Const(1),
            WasmSimulator.EncodeI32Const(2),
            WasmSimulator.EncodeI32Add(),
            WasmSimulator.EncodeLocalSet(0),
            WasmSimulator.EncodeEnd(),
        ]);

        var traces = simulator.Run(program);
        Assert.Equal(5, traces.Count);
        Assert.Equal(3, simulator.Locals[0]);
        Assert.True(simulator.Halted);
    }
}
