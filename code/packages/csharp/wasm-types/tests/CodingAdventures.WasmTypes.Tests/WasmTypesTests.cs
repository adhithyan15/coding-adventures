using System.Linq;
using CodingAdventures.WasmTypes;
using Xunit;

namespace CodingAdventures.WasmTypes.Tests;

public class WasmTypesTests
{
    [Fact]
    public void HasVersion()
    {
        Assert.Equal("0.1.0", WasmTypesVersion.VERSION);
    }

    [Fact]
    public void ExposesSpecConstants()
    {
        Assert.Equal((byte)0x7F, (byte)ValueType.I32);
        Assert.Equal((byte)0x7E, (byte)ValueType.I64);
        Assert.Equal((byte)0x7D, (byte)ValueType.F32);
        Assert.Equal((byte)0x7C, (byte)ValueType.F64);
        Assert.Equal((byte)0x40, BlockType.EMPTY);
        Assert.Equal((byte)0x70, ReferenceType.FUNCREF);
        Assert.Equal((byte)0x00, (byte)ExternalKind.FUNCTION);
        Assert.Equal((byte)0x03, (byte)ExternalKind.GLOBAL);
    }

    [Fact]
    public void MakeFuncTypeCopiesParamsAndResults()
    {
        var parameters = new[] { ValueType.I32, ValueType.I64 };
        var results = new[] { ValueType.F64 };

        var funcType = WasmTypeFactory.MakeFuncType(parameters, results);

        parameters[0] = ValueType.F32;
        results[0] = ValueType.I32;

        Assert.Equal(new[] { ValueType.I32, ValueType.I64 }, funcType.Params);
        Assert.Equal(new[] { ValueType.F64 }, funcType.Results);
    }

    [Fact]
    public void SupportsLimitsAndStorageTypes()
    {
        var limits = new Limits(1, 8);
        var memory = new MemoryType(limits);
        var table = new TableType(ReferenceType.FUNCREF, new Limits(0, null));
        var globalType = new GlobalType(ValueType.I32, false);

        Assert.Equal(1, memory.Limits.Min);
        Assert.Equal(8, memory.Limits.Max);
        Assert.Equal(ReferenceType.FUNCREF, table.ElementType);
        Assert.Null(table.Limits.Max);
        Assert.Equal(ValueType.I32, globalType.ValueType);
        Assert.False(globalType.Mutable);
    }

    [Fact]
    public void SupportsTypedImportsAndExports()
    {
        var functionImport = new Import("env", "add", ExternalKind.FUNCTION, new FunctionImportDescriptor(2));
        var memoryImport = new Import(
            "env",
            "memory",
            ExternalKind.MEMORY,
            new MemoryImportDescriptor(new MemoryType(new Limits(1, null))));
        var export = new Export("main", ExternalKind.FUNCTION, 0);

        Assert.Equal("env", functionImport.ModuleName);
        Assert.IsType<FunctionImportDescriptor>(functionImport.Descriptor);
        Assert.IsType<MemoryImportDescriptor>(memoryImport.Descriptor);
        Assert.Equal("main", export.Name);
        Assert.Equal(0, export.Index);
    }

    [Fact]
    public void CopiesByteBackedStructures()
    {
        var initExpr = new byte[] { 0x41, 0x2A, 0x0B };
        var offsetExpr = new byte[] { 0x41, 0x00, 0x0B };
        var data = new byte[] { 0x48, 0x69 };
        var code = new byte[] { 0x20, 0x00, 0x0B };
        var customData = new byte[] { 0x01, 0x02 };

        var global = new Global(new GlobalType(ValueType.I32, false), initExpr);
        var element = new Element(0, offsetExpr, new[] { 0, 1, 2 });
        var segment = new DataSegment(0, offsetExpr, data);
        var body = new FunctionBody(new[] { ValueType.I32, ValueType.I32 }, code);
        var custom = new CustomSection("name", customData);

        initExpr[0] = 0x00;
        offsetExpr[1] = 0x05;
        data[0] = 0x00;
        code[0] = 0x00;
        customData[0] = 0xFF;

        Assert.Equal(new byte[] { 0x41, 0x2A, 0x0B }, global.InitExpr);
        Assert.Equal(new byte[] { 0x41, 0x00, 0x0B }, element.OffsetExpr);
        Assert.Equal(new[] { 0, 1, 2 }, element.FunctionIndices);
        Assert.Equal(new byte[] { 0x48, 0x69 }, segment.Data);
        Assert.Equal(new[] { ValueType.I32, ValueType.I32 }, body.Locals);
        Assert.Equal(new byte[] { 0x20, 0x00, 0x0B }, body.Code);
        Assert.Equal(new byte[] { 0x01, 0x02 }, custom.Data);
    }

    [Fact]
    public void WasmModuleStartsEmptyAndCanBePopulated()
    {
        var module = new WasmModule();
        Assert.Empty(module.Types);
        Assert.Empty(module.Imports);
        Assert.Empty(module.Functions);
        Assert.Empty(module.Tables);
        Assert.Empty(module.Memories);
        Assert.Empty(module.Globals);
        Assert.Empty(module.Exports);
        Assert.Empty(module.Elements);
        Assert.Empty(module.Code);
        Assert.Empty(module.Data);
        Assert.Empty(module.Customs);
        Assert.Null(module.Start);

        var funcType = WasmTypeFactory.MakeFuncType([ValueType.I32], [ValueType.I32]);
        module.Types.Add(funcType);
        module.Functions.Add(0);
        module.Exports.Add(new Export("main", ExternalKind.FUNCTION, 0));
        module.Start = 0;

        Assert.Single(module.Types);
        Assert.Equal(ValueType.I32, module.Types.Single().Params.Single());
        Assert.Equal(0, module.Functions.Single());
        Assert.Equal("main", module.Exports.Single().Name);
        Assert.Equal(0, module.Start);
    }
}
