namespace CodingAdventures.NibWasmCompiler.FSharp.Tests

open System
open System.IO
open CodingAdventures.NibWasmCompiler.FSharp
open CodingAdventures.WasmRuntime.FSharp
open Xunit

module NibWasmCompilerTests =
    [<Fact>]
    let ``compiles and runs literal function`` () =
        let result = NibWasmCompiler.compileSource "fn answer() -> u4 { return 7; }"
        Assert.Equal("answer", result.Functions.Head.Name)
        Assert.Equal("7", result.Functions.Head.Expression)
        Assert.Equal<byte array>([| 0x00uy; 0x61uy; 0x73uy; 0x6Duy |], result.WasmBytes[..3])
        Assert.Equal(None, result.WasmPath)
        Assert.Equal<obj list>([ box 7 ], WasmRuntime().LoadAndRun(result.WasmBytes, "answer", [||]))

    [<Fact>]
    let ``compiles parameters and wrapping addition`` () =
        let result = NibWasmCompiler.compileSource "fn add(a: u4, b: u4) -> u4 { return a +% b; }"
        let runtime = WasmRuntime()
        let moduleValue = runtime.Load(result.WasmBytes)
        Assert.Equal<string list>([ "a"; "b" ], result.Functions.Head.Parameters)
        runtime.Validate(moduleValue) |> ignore
        Assert.Contains(0x6Auy, result.WasmBytes)
        Assert.Contains(0x71uy, result.WasmBytes)

    [<Fact>]
    let ``compiles nested calls and exports every function`` () =
        let source = "fn id(x: u4) -> u4 { return x; }\nfn twice(x: u4) -> u4 { return id(id(x)); }"
        let result = NibWasmCompiler.compileSource source
        let runtime = WasmRuntime()
        let moduleValue = runtime.Load(result.WasmBytes)
        Assert.Equal<string list>([ "id"; "twice" ], moduleValue.Exports |> Seq.map _.Name |> Seq.toList)
        Assert.Equal<obj list>([ box 15 ], runtime.LoadAndRun(result.WasmBytes, "twice", [| 15 |]))

    [<Fact>]
    let ``pack is alias and result defends bytes`` () =
        let result = NibWasmCompiler.packSource "fn answer() -> u4 { return 7; }"
        let bytes = result.WasmBytes
        bytes[0] <- 0xFFuy
        Assert.Equal(0x00uy, result.WasmBytes[0])
        Assert.Equal("0.1.0", Version.VERSION)

    [<Fact>]
    let ``writes wasm file and records path`` () =
        let path = Path.Combine(Path.GetTempPath(), $"nib-wasm-{Guid.NewGuid():N}.wasm")
        try
            let result = NibWasmCompiler.writeWasmFile "fn answer() -> u4 { return 7; }" path
            Assert.Equal(Some path, result.WasmPath)
            Assert.Equal<byte array>(result.WasmBytes, File.ReadAllBytes path)
        finally
            File.Delete path

    [<Fact>]
    let ``wraps write errors`` () =
        let path = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"), "x.wasm")
        let error = Assert.Throws<PackageError>(fun () -> NibWasmCompiler.writeWasmFile "fn answer() -> u4 { return 7; }" path |> ignore)
        Assert.Equal("write", error.Stage)

    [<Fact>]
    let ``reports invalid nib by stage`` () =
        let cases =
            [
                "", "parse"
                "garbage fn answer() -> u4 { return 7; }", "parse"
                "fn bad(x: u8) -> u4 { return 1; }", "parse"
                "fn bad(x: u4, x: u4) -> u4 { return x; }", "validate"
                "fn same() -> u4 { return 1; } fn same() -> u4 { return 2; }", "validate"
                "fn bad() -> u4 { return 16; }", "validate"
                "fn bad() -> u4 { return missing(); }", "validate"
                "fn one(x: u4) -> u4 { return x; } fn bad() -> u4 { return one(); }", "validate"
                "fn bad() -> u4 { return nope; }", "validate"
            ]

        for source, stage in cases do
            let error = Assert.Throws<PackageError>(fun () -> NibWasmCompiler.compileSource source |> ignore)
            Assert.Equal(stage, error.Stage)
            Assert.StartsWith($"[{stage}]", error.ToString())

    [<Fact>]
    let ``rejects excessive expression nesting`` () =
        let mutable expression = "0"
        for _ = 0 to 257 do
            expression <- $"id({expression})"
        let source = $"fn id(x: u4) -> u4 {{ return x; }} fn main() -> u4 {{ return {expression}; }}"
        let error = Assert.Throws<PackageError>(fun () -> NibWasmCompiler.compileSource source |> ignore)
        Assert.Equal("validate", error.Stage)
        Assert.Contains("nesting", error.Message)

    [<Fact>]
    let ``rejects excessive source length and nulls`` () =
        let error = Assert.Throws<PackageError>(fun () -> NibWasmCompiler.compileSource (String(' ', 1_000_001)) |> ignore)
        Assert.Equal("parse", error.Stage)
        Assert.Throws<ArgumentNullException>(fun () -> NibWasmCompiler.compileSource null |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> NibWasmCompiler.writeWasmFile "fn x() -> u4 { return 0; }" null |> ignore) |> ignore
