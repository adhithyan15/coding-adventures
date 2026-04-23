namespace CodingAdventures.WasmExecution.Tests

open CodingAdventures.WasmExecution.FSharp
open CodingAdventures.WasmLeb128.FSharp
open CodingAdventures.WasmTypes.FSharp
open Xunit

type WasmExecutionTests() =
    [<Fact>]
    member _.``Has version``() =
        Assert.Equal("0.1.0", Version.VERSION)

    [<Fact>]
    member _.``Linear memory round trips i32``() =
        let memory = LinearMemory(1)
        memory.StoreI32(0, 0x01020304)
        Assert.Equal(0x01020304, memory.LoadI32(0))
        Assert.Equal(0x04, memory.LoadI32_8u(0))

    [<Fact>]
    member _.``Evaluate constant expression``() =
        let encoded = WasmLeb128.encodeSigned 42
        let expr = Array.append [| 0x41uy |] (Array.append encoded [| 0x0Buy |])
        let results = WasmExecution.evaluateConstExpr expr []
        Assert.Single(results) |> ignore
        Assert.Equal(42, results.Head |> WasmValue.asI32)

    [<Fact>]
    member _.``Execution engine adds two arguments``() =
        let funcType = WasmTypes.makeFuncType [ ValueType.I32; ValueType.I32 ] [ ValueType.I32 ]
        let body = FunctionBody([], [| 0x20uy; 0x00uy; 0x20uy; 0x01uy; 0x6Auy; 0x0Buy |])
        let engine =
            WasmExecutionEngine(
                {
                    Memory = None
                    Tables = []
                    Globals = []
                    GlobalTypes = []
                    FuncTypes = [ funcType ]
                    FuncBodies = [ Some body ]
                    HostFunctions = [ None ]
                }
            )

        let results = engine.CallFunction(0, [ I32 3; I32 4 ])
        Assert.Equal(7, results.Head |> WasmValue.asI32)

    [<Fact>]
    member _.``Execution engine stores and loads memory``() =
        let funcType = WasmTypes.makeFuncType [ ValueType.I32 ] [ ValueType.I32 ]
        let body =
            FunctionBody(
                [],
                [|
                    0x41uy; 0x00uy
                    0x20uy; 0x00uy
                    0x36uy; 0x00uy; 0x00uy
                    0x41uy; 0x00uy
                    0x28uy; 0x00uy; 0x00uy
                    0x0Buy
                |]
            )

        let engine =
            WasmExecutionEngine(
                {
                    Memory = Some(LinearMemory(1))
                    Tables = []
                    Globals = []
                    GlobalTypes = []
                    FuncTypes = [ funcType ]
                    FuncBodies = [ Some body ]
                    HostFunctions = [ None ]
                }
            )

        let results = engine.CallFunction(0, [ I32 99 ])
        Assert.Equal(99, results.Head |> WasmValue.asI32)
