using System.Linq;
using CodingAdventures.WasmModuleParser;
using CodingAdventures.WasmTypes;
using Xunit;

namespace CodingAdventures.WasmModuleParser.Tests;

public class WasmModuleParserTests
{
    private static readonly byte[] EmptyModule = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

    [Fact]
    public void HasVersion()
    {
        Assert.Equal("0.1.0", WasmModuleParserVersion.VERSION);
    }

    [Fact]
    public void ParsesEmptyModule()
    {
        var module = new WasmModuleParser().Parse(EmptyModule);

        Assert.Empty(module.Types);
        Assert.Empty(module.Imports);
        Assert.Null(module.Start);
    }

    [Fact]
    public void ParsesTypeFunctionCodeAndExportSections()
    {
        byte[] data =
        [
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
            0x01, 0x06, 0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F,
            0x03, 0x02, 0x01, 0x00,
            0x07, 0x08, 0x01, 0x04, 0x6D, 0x61, 0x69, 0x6E, 0x00, 0x00,
            0x0A, 0x06, 0x01, 0x04, 0x00, 0x20, 0x00, 0x0B,
        ];

        var module = new WasmModuleParser().Parse(data);

        Assert.Single(module.Types);
        Assert.Equal(new[] { ValueType.I32 }, module.Types.Single().Params);
        Assert.Equal(new[] { ValueType.I32 }, module.Types.Single().Results);
        Assert.Equal(0, module.Functions.Single());
        Assert.Equal("main", module.Exports.Single().Name);
        Assert.Equal(new byte[] { 0x20, 0x00, 0x0B }, module.Code.Single().Code);
    }

    [Fact]
    public void ParsesCustomSection()
    {
        byte[] data =
        [
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
            0x00, 0x07, 0x04, 0x6E, 0x61, 0x6D, 0x65, 0x01, 0x02,
        ];

        var module = new WasmModuleParser().Parse(data);

        Assert.Single(module.Customs);
        Assert.Equal("name", module.Customs.Single().Name);
        Assert.Equal(new byte[] { 0x01, 0x02 }, module.Customs.Single().Data);
    }

    [Fact]
    public void RejectsInvalidMagic()
    {
        byte[] data = [0x01, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        var error = Assert.Throws<WasmParseError>(() => new WasmModuleParser().Parse(data));
        Assert.Equal(0, error.Offset);
    }

    [Fact]
    public void RejectsOutOfOrderSections()
    {
        byte[] data =
        [
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
            0x03, 0x02, 0x01, 0x00,
            0x01, 0x06, 0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F,
        ];

        var error = Assert.Throws<WasmParseError>(() => new WasmModuleParser().Parse(data));
        Assert.Contains("out of order", error.Message);
    }
}
