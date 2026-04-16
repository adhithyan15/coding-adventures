namespace CodingAdventures.WasmValidator.Tests

open CodingAdventures.WasmTypes.FSharp
open CodingAdventures.WasmValidator.FSharp
open Xunit

type WasmValidatorTests() =
    [<Fact>]
    member _.``Has version``() =
        Assert.Equal("0.1.0", Version.VERSION)

    [<Fact>]
    member _.``Empty module is valid``() =
        let validated = WasmValidator.validate (WasmModule())
        Assert.Empty(validated.FuncTypes)

    [<Fact>]
    member _.``Rejects duplicate exports``() =
        let moduleValue = WasmModule()
        moduleValue.Exports.Add({ Name = "x"; Kind = ExternalKind.FUNCTION; Index = 0 })
        moduleValue.Exports.Add({ Name = "x"; Kind = ExternalKind.FUNCTION; Index = 0 })
        Assert.Throws<ValidationError>(fun () -> WasmValidator.validateStructure moduleValue |> ignore) |> ignore

    [<Fact>]
    member _.``Validates constant expressions``() =
        let indexSpaces =
            {
                FuncTypes = []
                NumImportedFuncs = 0
                TableTypes = []
                NumImportedTables = 0
                MemoryTypes = []
                NumImportedMemories = 0
                GlobalTypes = [ { ValueType = ValueType.I32; Mutable = false } ]
                NumImportedGlobals = 1
                NumTypes = 0
            }

        WasmValidator.validateConstExpr [| 0x23uy; 0x00uy; 0x0Buy |] ValueType.I32 indexSpaces

    [<Fact>]
    member _.``Rejects missing function end``() =
        let moduleValue = WasmModule()
        moduleValue.Types.Add(WasmTypes.makeFuncType [ ValueType.I32 ] [ ValueType.I32 ])
        moduleValue.Functions.Add(0)
        moduleValue.Code.Add(FunctionBody([], [| 0x20uy; 0x00uy |]))
        Assert.Throws<ValidationError>(fun () -> WasmValidator.validate moduleValue |> ignore) |> ignore
