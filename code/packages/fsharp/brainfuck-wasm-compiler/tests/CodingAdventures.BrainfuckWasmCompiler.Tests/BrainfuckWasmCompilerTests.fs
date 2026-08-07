namespace CodingAdventures.BrainfuckWasmCompiler.FSharp.Tests

open System
open System.IO
open CodingAdventures.BrainfuckWasmCompiler.FSharp
open CodingAdventures.WasmRuntime.FSharp
open CodingAdventures.WasmTypes.FSharp
open Xunit

module BrainfuckWasmCompilerTests =
    [<Fact>]
    let ``compile filters comments and builds a valid module`` () =
        let result = BrainfuckWasmCompiler.compileSource "note: ++[>+<-] done"
        Assert.Equal<char list>([ '+'; '+'; '['; '>'; '+'; '<'; '-'; ']' ], result.Operations)
        Assert.Equal<byte array>([| 0x00uy; 0x61uy; 0x73uy; 0x6Duy |], result.WasmBytes[..3])
        Assert.Equal(None, result.WasmPath)

        let runtime = WasmRuntime()
        let moduleValue = runtime.Load(result.WasmBytes)
        Assert.Empty(moduleValue.Imports)
        Assert.Single(moduleValue.Memories) |> ignore
        Assert.Equal<string list>([ "_start"; "memory" ], moduleValue.Exports |> Seq.map _.Name |> Seq.toList)
        runtime.Validate(moduleValue) |> ignore

    [<Fact>]
    let ``compile adds only required WASI imports`` () =
        let cases =
            [
                ".", [ "fd_write" ]
                ",", [ "fd_read" ]
                ".,", [ "fd_write"; "fd_read" ]
            ]

        for source, expectedNames in cases do
            let runtime = WasmRuntime()
            let moduleValue = runtime.Load((BrainfuckWasmCompiler.compileSource source).WasmBytes)
            Assert.Equal<string list>(expectedNames, moduleValue.Imports |> Seq.map _.Name |> Seq.toList)
            Assert.All(
                moduleValue.Imports,
                fun item ->
                    Assert.Equal("wasi_snapshot_preview1", item.ModuleName)
                    Assert.Equal(ExternalKind.FUNCTION, item.Kind)
            )
            Assert.Equal(expectedNames.Length + 1, moduleValue.Types.Count)
            Assert.Equal(expectedNames.Length, moduleValue.Functions[0])

    [<Fact>]
    let ``pack is a compile alias and results defend their bytes`` () =
        let result = BrainfuckWasmCompiler.packSource "+"
        let bytes = result.WasmBytes
        bytes[0] <- 0xFFuy

        Assert.Equal(0x00uy, result.WasmBytes[0])
        Assert.Equal("0.1.0", Version.VERSION)

    [<Fact>]
    let ``pointer operations execute in the local runtime`` () =
        let result = BrainfuckWasmCompiler.compileSource "><"
        let runtime = WasmRuntime()
        let instance = runtime.Instantiate(result.WasmBytes)

        runtime.Call(instance, "_start", [||]) |> ignore
        let memory = instance.Engine.Memory |> Option.get
        Assert.Equal<byte array>([| 0uy; 0uy |], memory.ReadBytes(0, 2))

    [<Fact>]
    let ``writeWasmFile writes bytes and records path`` () =
        let directory = Path.Combine(Path.GetTempPath(), sprintf "brainfuck-wasm-%s" (Guid.NewGuid().ToString("N")))
        Directory.CreateDirectory(directory) |> ignore
        let path = Path.Combine(directory, "program.wasm")
        try
            let result = BrainfuckWasmCompiler.writeWasmFile "+" path
            Assert.Equal(Some path, result.WasmPath)
            Assert.Equal<byte array>(result.WasmBytes, File.ReadAllBytes(path))
        finally
            Directory.Delete(directory, true)

    [<Fact>]
    let ``writeWasmFile wraps filesystem errors`` () =
        let path = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"), "x.wasm")
        let error = Assert.Throws<PackageError>(fun () -> BrainfuckWasmCompiler.writeWasmFile "+" path |> ignore)
        Assert.Equal("write", error.Stage)

    [<Fact>]
    let ``compile rejects unmatched loops`` () =
        let cases = [ "[", "unmatched ["; "abc]", "unmatched ] at byte 3" ]
        for source, message in cases do
            let error = Assert.Throws<PackageError>(fun () -> BrainfuckWasmCompiler.compileSource source |> ignore)
            Assert.Equal("parse", error.Stage)
            Assert.Equal(message, error.Message)

    [<Fact>]
    let ``compile rejects excessive nesting`` () =
        let error =
            Assert.Throws<PackageError>(fun () -> BrainfuckWasmCompiler.compileSource (String('[', 513)) |> ignore)
        Assert.Equal("parse", error.Stage)
        Assert.Equal("loop nesting exceeds 512", error.Message)

    [<Fact>]
    let ``compile rejects excessive source length`` () =
        let error =
            Assert.Throws<PackageError>(fun () -> BrainfuckWasmCompiler.compileSource (String('x', 1_000_001)) |> ignore)
        Assert.Equal("parse", error.Stage)
        Assert.Equal("source exceeds 1000000 characters", error.Message)
