namespace CodingAdventures.WasmOpcodes.FSharp.Tests

open System.Linq
open CodingAdventures.WasmOpcodes.FSharp
open Xunit

module WasmOpcodesTests =
    [<Fact>]
    let ``has version`` () =
        Assert.Equal("0.1.0", Version.VERSION)

    [<Fact>]
    let ``includes complete opcode table`` () =
        Assert.True(WasmOpcodes.OPCODES.Count >= 172)
        Assert.Equal(WasmOpcodes.OPCODES.Count, WasmOpcodes.OPCODES_BY_NAME.Count)

    [<Fact>]
    let ``supports lookup by opcode`` () =
        let info = WasmOpcodes.getOpcode 0x6Auy
        Assert.True(info.IsSome)
        Assert.Equal("i32.add", info.Value.Name)
        Assert.Equal(0x6Auy, info.Value.Opcode)
        Assert.Equal("numeric_i32", info.Value.Category)
        Assert.Equal(None, WasmOpcodes.getOpcode 0xFFuy)
        Assert.Equal(None, WasmOpcodes.getOpcode 0x06uy)

    [<Fact>]
    let ``supports lookup by name`` () =
        let info = WasmOpcodes.getOpcodeByName "call_indirect"
        Assert.True(info.IsSome)
        Assert.Equal(0x11uy, info.Value.Opcode)
        Assert.Equal<string list>([ "typeidx"; "tableidx" ], info.Value.Immediates)
        Assert.Equal(None, WasmOpcodes.getOpcodeByName "i32.foo")

    [<Fact>]
    let ``preserves stack effects`` () =
        let add = WasmOpcodes.getOpcodeByName "i32.add" |> Option.get
        let constOp = WasmOpcodes.getOpcodeByName "i32.const" |> Option.get
        let select = WasmOpcodes.getOpcodeByName "select" |> Option.get
        let dropOp = WasmOpcodes.getOpcodeByName "drop" |> Option.get

        Assert.Equal((2, 1), (add.StackPop, add.StackPush))
        Assert.Equal((0, 1), (constOp.StackPop, constOp.StackPush))
        Assert.Equal((3, 1), (select.StackPop, select.StackPush))
        Assert.Equal((1, 0), (dropOp.StackPop, dropOp.StackPush))

    [<Fact>]
    let ``preserves immediates and categories`` () =
        Assert.Equal<string list>([ "memarg" ], (WasmOpcodes.getOpcodeByName "i32.load" |> Option.get).Immediates)
        Assert.Equal<string list>([ "funcidx" ], (WasmOpcodes.getOpcodeByName "call" |> Option.get).Immediates)
        Assert.Equal<string list>([ "blocktype" ], (WasmOpcodes.getOpcodeByName "block" |> Option.get).Immediates)
        Assert.Equal("numeric_i64", (WasmOpcodes.getOpcodeByName "i64.add" |> Option.get).Category)
        Assert.Equal("numeric_f32", (WasmOpcodes.getOpcodeByName "f32.sqrt" |> Option.get).Category)
        Assert.Equal("conversion", (WasmOpcodes.getOpcode 0xBFuy |> Option.get).Category)

    [<Fact>]
    let ``keeps names and bytes unique`` () =
        Assert.Equal(WasmOpcodes.OPCODES.Count, WasmOpcodes.OPCODES.Keys.Distinct().Count())
        Assert.Equal(WasmOpcodes.OPCODES_BY_NAME.Count, WasmOpcodes.OPCODES_BY_NAME.Keys.Distinct().Count())
