using CodingAdventures.WasmRuntime;
using CodingAdventures.WasmTypes;

namespace CodingAdventures.BrainfuckWasmCompiler.Tests;

public sealed class BrainfuckWasmCompilerTests
{
    [Fact]
    public void CompileSourceFiltersCommentsAndBuildsLoadableModule()
    {
        var result = BrainfuckWasmCompiler.CompileSource("note: ++[>+<-] done");

        Assert.Equal("++[>+<-]", new string(result.Operations.ToArray()));
        Assert.Equal([0x00, 0x61, 0x73, 0x6D], result.WasmBytes[..4]);
        Assert.Null(result.WasmPath);

        var module = new WasmRuntime.WasmRuntime().Load(result.WasmBytes);
        Assert.Empty(module.Imports);
        Assert.Single(module.Memories);
        Assert.Equal(["_start", "memory"], module.Exports.Select(item => item.Name));
        new WasmRuntime.WasmRuntime().Validate(module);
    }

    [Theory]
    [InlineData(".", "fd_write")]
    [InlineData(",", "fd_read")]
    [InlineData(".,", "fd_write", "fd_read")]
    public void CompileSourceAddsOnlyRequiredWasiImports(string source, params string[] names)
    {
        var module = new WasmRuntime.WasmRuntime().Load(BrainfuckWasmCompiler.CompileSource(source).WasmBytes);

        Assert.Equal(names, module.Imports.Select(item => item.Name));
        Assert.All(module.Imports, item => Assert.Equal("wasi_snapshot_preview1", item.ModuleName));
        Assert.All(module.Imports, item => Assert.Equal(ExternalKind.FUNCTION, item.Kind));
        Assert.Equal(names.Length + 1, module.Types.Count);
        Assert.Equal(names.Length, module.Functions.Single());
    }

    [Fact]
    public void PackSourceIsCompileAliasAndResultsDefendTheirBytes()
    {
        var result = BrainfuckWasmCompiler.PackSource("+");
        var bytes = result.WasmBytes;
        bytes[0] = 0xFF;

        Assert.Equal((byte)0x00, result.WasmBytes[0]);
        Assert.Equal(BrainfuckWasmCompilerVersion.VERSION, "0.1.0");
    }

    [Fact]
    public void PointerAndWrappingCellOperationsExecute()
    {
        var result = BrainfuckWasmCompiler.CompileSource("-+>+<");
        var runtime = new WasmRuntime.WasmRuntime();
        var instance = runtime.Instantiate(result.WasmBytes);

        runtime.Call(instance, "_start");
        Assert.Equal([0, 1], instance.Memory!.ReadBytes(0, 2));
    }

    [Fact]
    public void WriteWasmFileWritesBytesAndRecordsPath()
    {
        var directory = Path.Combine(Path.GetTempPath(), $"brainfuck-wasm-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        var path = Path.Combine(directory, "program.wasm");
        try
        {
            var result = BrainfuckWasmCompiler.WriteWasmFile("+", path);

            Assert.Equal(path, result.WasmPath);
            Assert.Equal(result.WasmBytes, File.ReadAllBytes(path));
        }
        finally
        {
            Directory.Delete(directory, true);
        }
    }

    [Fact]
    public void WriteWasmFileWrapsFilesystemErrors()
    {
        var error = Assert.Throws<PackageError>(() =>
            BrainfuckWasmCompiler.WriteWasmFile("+", Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"), "x.wasm")));

        Assert.Equal("write", error.Stage);
    }

    [Theory]
    [InlineData("[", "unmatched [")]
    [InlineData("abc]", "unmatched ] at byte 3")]
    public void CompileSourceRejectsUnmatchedLoops(string source, string message)
    {
        var error = Assert.Throws<PackageError>(() => BrainfuckWasmCompiler.CompileSource(source));

        Assert.Equal("parse", error.Stage);
        Assert.Equal(message, error.Message);
    }

    [Fact]
    public void CompileSourceRejectsExcessiveNesting()
    {
        var error = Assert.Throws<PackageError>(() => BrainfuckWasmCompiler.CompileSource(new string('[', 513)));

        Assert.Equal("parse", error.Stage);
        Assert.Equal("loop nesting exceeds 512", error.Message);
    }

    [Fact]
    public void CompileSourceRejectsExcessiveSourceLength()
    {
        var error = Assert.Throws<PackageError>(() => BrainfuckWasmCompiler.CompileSource(new string('x', 1_000_001)));

        Assert.Equal("parse", error.Stage);
        Assert.Equal("source exceeds 1000000 characters", error.Message);
    }
}
