using System;
using CodingAdventures.WasmTypes;
using CodingAdventures.WasmValidator;
using Xunit;
using WasmValueType = CodingAdventures.WasmTypes.ValueType;

namespace CodingAdventures.WasmValidator.Tests;

public class WasmValidatorTests
{
    [Fact]
    public void HasVersion() => Assert.Equal("0.1.0", WasmValidatorVersion.VERSION);

    [Fact]
    public void EmptyModuleIsValid()
    {
        var validated = WasmValidator.Validate(new WasmModule());
        Assert.Empty(validated.FuncTypes);
    }

    [Fact]
    public void RejectsDuplicateExports()
    {
        var module = new WasmModule();
        module.Exports.Add(new Export("x", ExternalKind.FUNCTION, 0));
        module.Exports.Add(new Export("x", ExternalKind.FUNCTION, 0));
        Assert.Throws<ValidationError>(() => WasmValidator.ValidateStructure(module));
    }

    [Fact]
    public void ValidatesConstExpr()
    {
        var spaces = new IndexSpaces
        {
            FuncTypes = Array.Empty<FuncType>(),
            NumImportedFuncs = 0,
            TableTypes = Array.Empty<TableType>(),
            NumImportedTables = 0,
            MemoryTypes = Array.Empty<MemoryType>(),
            NumImportedMemories = 0,
            GlobalTypes = new[] { new GlobalType(WasmValueType.I32, false) },
            NumImportedGlobals = 1,
            NumTypes = 0,
        };

        WasmValidator.ValidateConstExpr(new byte[] { 0x23, 0x00, 0x0B }, WasmValueType.I32, spaces);
    }

    [Fact]
    public void RejectsMissingFunctionEnd()
    {
        var module = new WasmModule();
        module.Types.Add(WasmTypeFactory.MakeFuncType([WasmValueType.I32], [WasmValueType.I32]));
        module.Functions.Add(0);
        module.Code.Add(new FunctionBody(Array.Empty<WasmValueType>(), new byte[] { 0x20, 0x00 }));
        Assert.Throws<ValidationError>(() => WasmValidator.Validate(module));
    }
}
