namespace CodingAdventures.WasmModuleParser.FSharp.Tests

open System.Linq
open CodingAdventures.WasmModuleParser.FSharp
open CodingAdventures.WasmTypes.FSharp
open Xunit

module WasmModuleParserTests =
    let emptyModule = [| 0x00uy; 0x61uy; 0x73uy; 0x6Duy; 0x01uy; 0x00uy; 0x00uy; 0x00uy |]

    [<Fact>]
    let ``has version`` () =
        Assert.Equal("0.1.0", Version.VERSION)

    [<Fact>]
    let ``parses empty module`` () =
        let moduleValue = WasmModuleParser().Parse(emptyModule)
        Assert.Empty(moduleValue.Types)
        Assert.Empty(moduleValue.Imports)
        Assert.Equal(None, moduleValue.Start)

    [<Fact>]
    let ``parses type function code and export sections`` () =
        let data =
            [|
                0x00uy; 0x61uy; 0x73uy; 0x6Duy; 0x01uy; 0x00uy; 0x00uy; 0x00uy
                0x01uy; 0x06uy; 0x01uy; 0x60uy; 0x01uy; 0x7Fuy; 0x01uy; 0x7Fuy
                0x03uy; 0x02uy; 0x01uy; 0x00uy
                0x07uy; 0x08uy; 0x01uy; 0x04uy; 0x6Duy; 0x61uy; 0x69uy; 0x6Euy; 0x00uy; 0x00uy
                0x0Auy; 0x06uy; 0x01uy; 0x04uy; 0x00uy; 0x20uy; 0x00uy; 0x0Buy
            |]

        let moduleValue = WasmModuleParser().Parse(data)
        Assert.Equal(1, moduleValue.Types.Count)
        Assert.Equal<ValueType list>([ ValueType.I32 ], moduleValue.Types[0].Params)
        Assert.Equal<ValueType list>([ ValueType.I32 ], moduleValue.Types[0].Results)
        Assert.Equal(0, moduleValue.Functions[0])
        Assert.Equal("main", moduleValue.Exports[0].Name)
        Assert.Equal<byte[]>([| 0x20uy; 0x00uy; 0x0Buy |], moduleValue.Code[0].Code)

    [<Fact>]
    let ``parses custom section`` () =
        let data =
            [|
                0x00uy; 0x61uy; 0x73uy; 0x6Duy; 0x01uy; 0x00uy; 0x00uy; 0x00uy
                0x00uy; 0x07uy; 0x04uy; 0x6Euy; 0x61uy; 0x6Duy; 0x65uy; 0x01uy; 0x02uy
            |]

        let moduleValue = WasmModuleParser().Parse(data)
        Assert.Equal(1, moduleValue.Customs.Count)
        Assert.Equal("name", moduleValue.Customs[0].Name)
        Assert.Equal<byte[]>([| 0x01uy; 0x02uy |], moduleValue.Customs[0].Data)

    [<Fact>]
    let ``rejects invalid magic`` () =
        let data = [| 0x01uy; 0x61uy; 0x73uy; 0x6Duy; 0x01uy; 0x00uy; 0x00uy; 0x00uy |]
        let error = Assert.Throws<WasmParseError>(fun () -> WasmModuleParser().Parse(data) |> ignore)
        Assert.Equal(0, error.Offset)

    [<Fact>]
    let ``rejects out of order sections`` () =
        let data =
            [|
                0x00uy; 0x61uy; 0x73uy; 0x6Duy; 0x01uy; 0x00uy; 0x00uy; 0x00uy
                0x03uy; 0x02uy; 0x01uy; 0x00uy
                0x01uy; 0x06uy; 0x01uy; 0x60uy; 0x01uy; 0x7Fuy; 0x01uy; 0x7Fuy
            |]

        let error = Assert.Throws<WasmParseError>(fun () -> WasmModuleParser().Parse(data) |> ignore)
        Assert.Contains("out of order", error.Message)
