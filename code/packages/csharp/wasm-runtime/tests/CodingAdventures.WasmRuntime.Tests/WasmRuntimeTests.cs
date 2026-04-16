using System.Collections.Generic;
using System.Text;
using CodingAdventures.WasmLeb128;
using CodingAdventures.WasmRuntime;
using Xunit;

namespace CodingAdventures.WasmRuntime.Tests;

public class WasmRuntimeTests
{
    [Fact]
    public void HasVersion() => Assert.Equal("0.1.0", WasmRuntimeVersion.VERSION);

    [Theory]
    [InlineData(5, 25)]
    [InlineData(0, 0)]
    [InlineData(-3, 9)]
    public void LoadAndRunExecutesSquareModule(int input, int expected)
    {
        var runtime = new WasmRuntime();
        var result = runtime.LoadAndRun(BuildSquareWasm(), "square", input);
        Assert.Single(result);
        Assert.Equal(expected, Assert.IsType<int>(result[0]));
    }

    [Fact]
    public void InstantiateExposesFunctionExport()
    {
        var runtime = new WasmRuntime();
        var instance = runtime.Instantiate(BuildSquareWasm());
        Assert.True(instance.Exports.ContainsKey("square"));
    }

    private static byte[] BuildSquareWasm()
    {
        var parts = new List<byte>();
        parts.AddRange([0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]);

        var typePayload = new List<byte> { 0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F };
        parts.Add(0x01);
        parts.AddRange(CodingAdventures.WasmLeb128.WasmLeb128.EncodeUnsigned(typePayload.Count));
        parts.AddRange(typePayload);

        var functionPayload = new List<byte> { 0x01, 0x00 };
        parts.Add(0x03);
        parts.AddRange(CodingAdventures.WasmLeb128.WasmLeb128.EncodeUnsigned(functionPayload.Count));
        parts.AddRange(functionPayload);

        var nameBytes = Encoding.UTF8.GetBytes("square");
        var exportPayload = new List<byte> { 0x01 };
        exportPayload.AddRange(CodingAdventures.WasmLeb128.WasmLeb128.EncodeUnsigned(nameBytes.Length));
        exportPayload.AddRange(nameBytes);
        exportPayload.Add(0x00);
        exportPayload.Add(0x00);
        parts.Add(0x07);
        parts.AddRange(CodingAdventures.WasmLeb128.WasmLeb128.EncodeUnsigned(exportPayload.Count));
        parts.AddRange(exportPayload);

        var body = new List<byte> { 0x00, 0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B };
        var codePayload = new List<byte> { 0x01 };
        codePayload.AddRange(CodingAdventures.WasmLeb128.WasmLeb128.EncodeUnsigned(body.Count));
        codePayload.AddRange(body);
        parts.Add(0x0A);
        parts.AddRange(CodingAdventures.WasmLeb128.WasmLeb128.EncodeUnsigned(codePayload.Count));
        parts.AddRange(codePayload);

        return parts.ToArray();
    }
}
