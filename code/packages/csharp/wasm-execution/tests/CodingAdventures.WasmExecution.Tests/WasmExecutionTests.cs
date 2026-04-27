using System;
using CodingAdventures.WasmExecution;
using CodingAdventures.WasmLeb128;
using CodingAdventures.WasmTypes;
using Xunit;
using WasmValueType = CodingAdventures.WasmTypes.ValueType;

namespace CodingAdventures.WasmExecution.Tests;

public class WasmExecutionTests
{
    [Fact]
    public void HasVersion() => Assert.Equal("0.1.0", WasmExecutionVersion.VERSION);

    [Fact]
    public void LinearMemoryRoundTripsI32()
    {
        var memory = new LinearMemory(1);
        memory.StoreI32(0, 0x01020304);
        Assert.Equal(0x01020304, memory.LoadI32(0));
        Assert.Equal(0x04, memory.LoadI32_8u(0));
    }

    [Fact]
    public void EvaluateConstExprReturnsConstant()
    {
        var encoded = CodingAdventures.WasmLeb128.WasmLeb128.EncodeSigned(42);
        var expr = new byte[encoded.Length + 2];
        expr[0] = 0x41;
        Array.Copy(encoded, 0, expr, 1, encoded.Length);
        expr[^1] = 0x0B;

        var results = WasmExecution.EvaluateConstExpr(expr, Array.Empty<WasmValue>());
        Assert.Single(results);
        Assert.Equal(42, results[0].AsI32());
    }

    [Fact]
    public void ExecutionEngineAddsTwoArguments()
    {
        var funcType = WasmTypeFactory.MakeFuncType([WasmValueType.I32, WasmValueType.I32], [WasmValueType.I32]);
        var body = new FunctionBody(Array.Empty<WasmValueType>(), [0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B]);
        var engine = new WasmExecutionEngine(new WasmExecutionEngineOptions
        {
            Memory = null,
            Tables = Array.Empty<Table>(),
            Globals = Array.Empty<WasmValue>(),
            GlobalTypes = Array.Empty<GlobalType>(),
            FuncTypes = new[] { funcType },
            FuncBodies = new FunctionBody?[] { body },
            HostFunctions = new IHostFunction?[] { null },
        });

        var results = engine.CallFunction(0, [WasmValue.I32(3), WasmValue.I32(4)]);
        Assert.Single(results);
        Assert.Equal(7, results[0].AsI32());
    }

    [Fact]
    public void ExecutionEngineStoresAndLoadsMemory()
    {
        var funcType = WasmTypeFactory.MakeFuncType([WasmValueType.I32], [WasmValueType.I32]);
        var body = new FunctionBody(Array.Empty<WasmValueType>(),
        [
            0x41, 0x00,
            0x20, 0x00,
            0x36, 0x00, 0x00,
            0x41, 0x00,
            0x28, 0x00, 0x00,
            0x0B,
        ]);

        var engine = new WasmExecutionEngine(new WasmExecutionEngineOptions
        {
            Memory = new LinearMemory(1),
            Tables = Array.Empty<Table>(),
            Globals = Array.Empty<WasmValue>(),
            GlobalTypes = Array.Empty<GlobalType>(),
            FuncTypes = new[] { funcType },
            FuncBodies = new FunctionBody?[] { body },
            HostFunctions = new IHostFunction?[] { null },
        });

        var results = engine.CallFunction(0, [WasmValue.I32(99)]);
        Assert.Single(results);
        Assert.Equal(99, results[0].AsI32());
    }
}
