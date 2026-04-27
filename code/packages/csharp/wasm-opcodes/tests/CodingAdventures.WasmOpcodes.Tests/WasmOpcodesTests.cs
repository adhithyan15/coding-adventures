using System.Linq;
using CodingAdventures.WasmOpcodes;
using Xunit;

namespace CodingAdventures.WasmOpcodes.Tests;

public class WasmOpcodesTests
{
    [Fact]
    public void HasVersion()
    {
        Assert.Equal("0.1.0", WasmOpcodesVersion.VERSION);
    }

    [Fact]
    public void IncludesCompleteOpcodeTable()
    {
        Assert.True(WasmOpcodes.OPCODES.Count >= 172);
        Assert.Equal(WasmOpcodes.OPCODES.Count, WasmOpcodes.OPCODES_BY_NAME.Count);
    }

    [Fact]
    public void SupportsLookupByOpcode()
    {
        var info = WasmOpcodes.GetOpcode(0x6A);
        Assert.NotNull(info);
        Assert.Equal("i32.add", info!.Name);
        Assert.Equal((byte)0x6A, info.Opcode);
        Assert.Equal("numeric_i32", info.Category);
        Assert.Null(WasmOpcodes.GetOpcode(0xFF));
        Assert.Null(WasmOpcodes.GetOpcode(0x06));
    }

    [Fact]
    public void SupportsLookupByName()
    {
        var info = WasmOpcodes.GetOpcodeByName("call_indirect");
        Assert.NotNull(info);
        Assert.Equal((byte)0x11, info!.Opcode);
        Assert.Equal(new[] { "typeidx", "tableidx" }, info.Immediates);
        Assert.Null(WasmOpcodes.GetOpcodeByName("i32.foo"));
    }

    [Fact]
    public void PreservesStackEffects()
    {
        Assert.Equal((2, 1), (WasmOpcodes.GetOpcodeByName("i32.add")!.StackPop, WasmOpcodes.GetOpcodeByName("i32.add")!.StackPush));
        Assert.Equal((0, 1), (WasmOpcodes.GetOpcodeByName("i32.const")!.StackPop, WasmOpcodes.GetOpcodeByName("i32.const")!.StackPush));
        Assert.Equal((3, 1), (WasmOpcodes.GetOpcodeByName("select")!.StackPop, WasmOpcodes.GetOpcodeByName("select")!.StackPush));
        Assert.Equal((1, 0), (WasmOpcodes.GetOpcodeByName("drop")!.StackPop, WasmOpcodes.GetOpcodeByName("drop")!.StackPush));
    }

    [Fact]
    public void PreservesImmediatesAndCategories()
    {
        Assert.Equal(new[] { "memarg" }, WasmOpcodes.GetOpcodeByName("i32.load")!.Immediates);
        Assert.Equal(new[] { "funcidx" }, WasmOpcodes.GetOpcodeByName("call")!.Immediates);
        Assert.Equal(new[] { "blocktype" }, WasmOpcodes.GetOpcodeByName("block")!.Immediates);
        Assert.Equal("numeric_i64", WasmOpcodes.GetOpcodeByName("i64.add")!.Category);
        Assert.Equal("numeric_f32", WasmOpcodes.GetOpcodeByName("f32.sqrt")!.Category);
        Assert.Equal("conversion", WasmOpcodes.GetOpcode(0xBF)!.Category);
    }

    [Fact]
    public void KeepsNamesAndBytesUnique()
    {
        Assert.Equal(WasmOpcodes.OPCODES.Count, WasmOpcodes.OPCODES.Keys.Distinct().Count());
        Assert.Equal(WasmOpcodes.OPCODES_BY_NAME.Count, WasmOpcodes.OPCODES_BY_NAME.Keys.Distinct().Count());
    }
}
