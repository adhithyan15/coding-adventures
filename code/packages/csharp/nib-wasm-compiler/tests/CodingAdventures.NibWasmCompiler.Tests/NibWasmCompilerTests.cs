using CodingAdventures.NibWasmCompiler;
using CodingAdventures.WasmRuntime;

public sealed class NibWasmCompilerTests
{
    [Fact]
    public void CompilesAndRunsLiteralFunction()
    {
        var result = NibWasmCompiler.CompileSource("fn answer() -> u4 { return 7; }");

        Assert.Equal("answer", result.Functions.Single().Name);
        Assert.Equal("7", result.Functions.Single().Expression);
        Assert.Equal([0x00, 0x61, 0x73, 0x6D], result.WasmBytes[..4]);
        Assert.Null(result.WasmPath);
        Assert.Equal([7], new WasmRuntime().LoadAndRun(result.WasmBytes, "answer"));
    }

    [Fact]
    public void CompilesParametersAndWrappingAddition()
    {
        const string source = "fn add(a: u4, b: u4) -> u4 { return a +% b; }";
        var result = NibWasmCompiler.CompileSource(source);
        var runtime = new WasmRuntime();
        var module = runtime.Load(result.WasmBytes);

        Assert.Equal(["a", "b"], result.Functions.Single().Parameters);
        runtime.Validate(module);
        Assert.Contains((byte)0x6A, result.WasmBytes);
        Assert.Contains((byte)0x71, result.WasmBytes);
    }

    [Fact]
    public void CompilesNestedCallsAndExportsEveryFunction()
    {
        const string source = "fn id(x: u4) -> u4 { return x; }\nfn twice(x: u4) -> u4 { return id(id(x)); }";
        var result = NibWasmCompiler.CompileSource(source);
        var runtime = new WasmRuntime();
        var module = runtime.Load(result.WasmBytes);

        Assert.Equal(["id", "twice"], module.Exports.Select(item => item.Name));
        Assert.Equal([15], runtime.LoadAndRun(result.WasmBytes, "twice", 15));
    }

    [Fact]
    public void PackIsAliasAndResultDefendsBytes()
    {
        var result = NibWasmCompiler.PackSource("fn answer() -> u4 { return 7; }");
        var bytes = result.WasmBytes;
        bytes[0] = 0xFF;

        Assert.Equal((byte)0, result.WasmBytes[0]);
        Assert.Equal("0.1.0", NibWasmCompilerVersion.VERSION);
    }

    [Fact]
    public void WritesWasmFileAndRecordsPath()
    {
        var path = Path.Combine(Path.GetTempPath(), $"nib-wasm-{Guid.NewGuid():N}.wasm");
        try
        {
            var result = NibWasmCompiler.WriteWasmFile("fn answer() -> u4 { return 7; }", path);
            Assert.Equal(path, result.WasmPath);
            Assert.Equal(result.WasmBytes, File.ReadAllBytes(path));
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public void WrapsWriteErrors()
    {
        var path = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"), "x.wasm");
        var error = Assert.Throws<PackageError>(() => NibWasmCompiler.WriteWasmFile("fn answer() -> u4 { return 7; }", path));
        Assert.Equal("write", error.Stage);
    }

    [Theory]
    [InlineData("", "parse")]
    [InlineData("garbage fn answer() -> u4 { return 7; }", "parse")]
    [InlineData("fn bad(x: u8) -> u4 { return 1; }", "parse")]
    [InlineData("fn bad(x: u4, x: u4) -> u4 { return x; }", "validate")]
    [InlineData("fn same() -> u4 { return 1; } fn same() -> u4 { return 2; }", "validate")]
    [InlineData("fn bad() -> u4 { return 16; }", "validate")]
    [InlineData("fn bad() -> u4 { return missing(); }", "validate")]
    [InlineData("fn one(x: u4) -> u4 { return x; } fn bad() -> u4 { return one(); }", "validate")]
    [InlineData("fn bad() -> u4 { return nope; }", "validate")]
    public void ReportsInvalidNibByStage(string source, string stage)
    {
        var error = Assert.Throws<PackageError>(() => NibWasmCompiler.CompileSource(source));
        Assert.Equal(stage, error.Stage);
        Assert.StartsWith($"[{stage}]", error.ToString());
    }

    [Fact]
    public void RejectsExcessiveExpressionNesting()
    {
        var expression = "0";
        for (var index = 0; index < 258; index++)
        {
            expression = $"id({expression})";
        }

        var source = $"fn id(x: u4) -> u4 {{ return x; }} fn main() -> u4 {{ return {expression}; }}";
        var error = Assert.Throws<PackageError>(() => NibWasmCompiler.CompileSource(source));
        Assert.Equal("validate", error.Stage);
        Assert.Contains("nesting", error.Message);
    }

    [Fact]
    public void RejectsExcessiveSourceLengthAndNulls()
    {
        var error = Assert.Throws<PackageError>(() => NibWasmCompiler.CompileSource(new string(' ', 1_000_001)));
        Assert.Equal("parse", error.Stage);
        Assert.Throws<ArgumentNullException>(() => NibWasmCompiler.CompileSource(null!));
        Assert.Throws<ArgumentNullException>(() => NibWasmCompiler.WriteWasmFile("fn x() -> u4 { return 0; }", null!));
    }
}
