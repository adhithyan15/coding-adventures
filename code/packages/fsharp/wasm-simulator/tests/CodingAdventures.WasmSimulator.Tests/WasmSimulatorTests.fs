module CodingAdventures.WasmSimulator.Tests

open CodingAdventures.WasmSimulator.FSharp
open Xunit

type WasmSimulatorTests() =
    [<Fact>]
    member _.``Has version``() =
        Assert.Equal("0.1.0", Version.VERSION)

    [<Fact>]
    member _.``Encoding helpers produce expected bytes``() =
        Assert.Equal<byte>([| 0x6Auy |], encodeI32Add ())
        Assert.Equal<byte>([| 0x21uy; 0x02uy |], encodeLocalSet 2)

    [<Fact>]
    member _.``Decoder reads i32 const``() =
        let decoder = WasmDecoder()
        let instruction = decoder.Decode(encodeI32Const 42, 0)
        Assert.Equal("i32.const", instruction.Mnemonic)
        Assert.Equal(42, instruction.Operand.Value)
        Assert.Equal(5, instruction.Size)

    [<Fact>]
    member _.``Simulator runs simple program``() =
        let simulator = WasmSimulator(4)
        let program =
            assembleWasm
                [
                    encodeI32Const 1
                    encodeI32Const 2
                    encodeI32Add ()
                    encodeLocalSet 0
                    encodeEnd ()
                ]

        let traces = simulator.Run(program)
        Assert.Equal(5, traces.Length)
        Assert.Equal(3, simulator.Locals[0])
        Assert.True(simulator.Halted)
