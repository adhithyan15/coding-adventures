using System;
using System.Linq;
using CodingAdventures.WasmModuleParser;
using CodingAdventures.WasmTypes;
using Xunit;
using Encoder = CodingAdventures.WasmModuleEncoder.WasmModuleEncoder;
using WasmValueType = CodingAdventures.WasmTypes.ValueType;

namespace CodingAdventures.WasmModuleEncoder.Tests;

public sealed class WasmModuleEncoderTests
{
    [Fact]
    public void EmptyModuleContainsOnlyHeader()
    {
        Assert.Equal(
            Encoder.WASM_MAGIC.Concat(Encoder.WASM_VERSION).ToArray(),
            Encoder.EncodeModule(new WasmModule()));
        Assert.Equal("0.1.0", WasmModuleEncoderVersion.VERSION);
    }

    [Fact]
    public void MinimalModuleRoundTripsThroughParser()
    {
        var module = new WasmModule();
        module.Types.Add(new FuncType([WasmValueType.I32], [WasmValueType.I32]));
        module.Functions.Add(0);
        module.Exports.Add(new Export("identity", ExternalKind.FUNCTION, 0));
        module.Code.Add(new FunctionBody([], [0x20, 0x00, 0x0B]));

        var parsed = new WasmModuleParser.WasmModuleParser().Parse(Encoder.EncodeModule(module));

        Assert.Single(parsed.Types);
        Assert.Equal([WasmValueType.I32], parsed.Types[0].Params);
        Assert.Equal([WasmValueType.I32], parsed.Types[0].Results);
        Assert.Equal([0], parsed.Functions);
        Assert.Equal(new Export("identity", ExternalKind.FUNCTION, 0), Assert.Single(parsed.Exports));
        Assert.Equal([0x20, 0x00, 0x0B], Assert.Single(parsed.Code).Code);
    }

    [Fact]
    public void EverySectionAndImportDescriptorRoundTrips()
    {
        var module = new WasmModule();
        module.Customs.Add(new CustomSection("name", [0x01, 0x02]));
        module.Types.Add(new FuncType([], []));
        module.Imports.Add(new Import("env", "f", ExternalKind.FUNCTION, new FunctionImportDescriptor(0)));
        module.Imports.Add(new Import("env", "table", ExternalKind.TABLE,
            new TableImportDescriptor(new TableType(ReferenceType.FUNCREF, new Limits(1, 4)))));
        module.Imports.Add(new Import("env", "memory", ExternalKind.MEMORY,
            new MemoryImportDescriptor(new MemoryType(new Limits(1, null)))));
        module.Imports.Add(new Import("env", "global", ExternalKind.GLOBAL,
            new GlobalImportDescriptor(new GlobalType(WasmValueType.I32, true))));
        module.Functions.Add(0);
        module.Tables.Add(new TableType(ReferenceType.FUNCREF, new Limits(1, 2)));
        module.Memories.Add(new MemoryType(new Limits(1, 3)));
        module.Globals.Add(new Global(new GlobalType(WasmValueType.I32, false), [0x41, 0x2A, 0x0B]));
        module.Exports.Add(new Export("main", ExternalKind.FUNCTION, 1));
        module.Start = 1;
        module.Elements.Add(new Element(0, [0x41, 0x00, 0x0B], [1, 2]));
        module.Code.Add(new FunctionBody(
            [WasmValueType.I32, WasmValueType.I32, WasmValueType.F64],
            [0x41, 0x07, 0x0B]));
        module.Data.Add(new DataSegment(0, [0x41, 0x00, 0x0B], [0x4E, 0x69, 0x62]));

        var parsed = new WasmModuleParser.WasmModuleParser().Parse(Encoder.EncodeModule(module));

        Assert.Equal(4, parsed.Imports.Count);
        Assert.Single(parsed.Tables);
        Assert.Single(parsed.Memories);
        Assert.Single(parsed.Globals);
        Assert.Equal(1, parsed.Start);
        Assert.Equal([1, 2], Assert.Single(parsed.Elements).FunctionIndices);
        Assert.Equal([WasmValueType.I32, WasmValueType.I32, WasmValueType.F64], Assert.Single(parsed.Code).Locals);
        Assert.Equal([0x4E, 0x69, 0x62], Assert.Single(parsed.Data).Data);
        Assert.Equal("name", Assert.Single(parsed.Customs).Name);
    }

    public static TheoryData<ExternalKind, ImportDescriptor, string> InvalidDescriptors => new()
    {
        { ExternalKind.FUNCTION, new MemoryImportDescriptor(new MemoryType(new Limits(1, null))), "function imports" },
        { ExternalKind.TABLE, new FunctionImportDescriptor(0), "table imports" },
        { ExternalKind.MEMORY, new FunctionImportDescriptor(0), "memory imports" },
        { ExternalKind.GLOBAL, new FunctionImportDescriptor(0), "global imports" },
        { (ExternalKind)0xFF, new FunctionImportDescriptor(0), "unsupported import kind" },
    };

    [Theory]
    [MemberData(nameof(InvalidDescriptors))]
    public void InvalidImportDescriptorsAreRejected(
        ExternalKind kind,
        ImportDescriptor descriptor,
        string message)
    {
        var module = new WasmModule();
        module.Imports.Add(new Import("env", "bad", kind, descriptor));

        var error = Assert.Throws<WasmEncodeError>(() => Encoder.EncodeModule(module));
        Assert.Contains(message, error.Message);
    }

    [Fact]
    public void NullModuleIsRejected()
    {
        Assert.Throws<ArgumentNullException>(() => Encoder.EncodeModule(null!));
    }
}
