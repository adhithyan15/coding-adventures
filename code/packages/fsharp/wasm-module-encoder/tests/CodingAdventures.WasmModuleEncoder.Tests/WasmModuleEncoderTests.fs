namespace CodingAdventures.WasmModuleEncoder.FSharp.Tests

open System
open CodingAdventures.WasmModuleEncoder.FSharp
open CodingAdventures.WasmModuleParser.FSharp
open CodingAdventures.WasmTypes.FSharp
open Xunit

module WasmModuleEncoderTests =
    [<Fact>]
    let ``empty module contains only header`` () =
        let expected = Array.append WasmModuleEncoder.WASM_MAGIC WasmModuleEncoder.WASM_VERSION
        Assert.Equal<byte array>(expected, WasmModuleEncoder.encodeModule (WasmModule()))
        Assert.Equal("0.1.0", CodingAdventures.WasmModuleEncoder.FSharp.Version.VERSION)

    [<Fact>]
    let ``minimal module round trips through parser`` () =
        let moduleValue = WasmModule()
        moduleValue.Types.Add(WasmTypes.makeFuncType [ ValueType.I32 ] [ ValueType.I32 ])
        moduleValue.Functions.Add(0)
        moduleValue.Exports.Add({ Name = "identity"; Kind = ExternalKind.FUNCTION; Index = 0 })
        moduleValue.Code.Add(FunctionBody([], [| 0x20uy; 0x00uy; 0x0Buy |]))

        let parsed = WasmModuleParser().Parse(WasmModuleEncoder.encodeModule moduleValue)
        Assert.Equal<ValueType list>([ ValueType.I32 ], parsed.Types[0].Params)
        Assert.Equal<int list>([ 0 ], parsed.Functions |> Seq.toList)
        Assert.Equal("identity", parsed.Exports[0].Name)
        Assert.Equal<byte array>([| 0x20uy; 0x00uy; 0x0Buy |], parsed.Code[0].Code)

    [<Fact>]
    let ``every section and import descriptor round trips`` () =
        let moduleValue = WasmModule()
        moduleValue.Customs.Add(CustomSection("name", [| 0x01uy; 0x02uy |]))
        moduleValue.Types.Add(WasmTypes.makeFuncType [] [])
        moduleValue.Imports.Add(
            { ModuleName = "env"; Name = "f"; Kind = ExternalKind.FUNCTION; Descriptor = FunctionImportDescriptor 0 }
        )
        moduleValue.Imports.Add(
            {
                ModuleName = "env"
                Name = "table"
                Kind = ExternalKind.TABLE
                Descriptor = TableImportDescriptor { ElementType = ReferenceType.FUNCREF; Limits = { Min = 1; Max = Some 4 } }
            }
        )
        moduleValue.Imports.Add(
            {
                ModuleName = "env"
                Name = "memory"
                Kind = ExternalKind.MEMORY
                Descriptor = MemoryImportDescriptor { Limits = { Min = 1; Max = None } }
            }
        )
        moduleValue.Imports.Add(
            {
                ModuleName = "env"
                Name = "global"
                Kind = ExternalKind.GLOBAL
                Descriptor = GlobalImportDescriptor { ValueType = ValueType.I32; Mutable = true }
            }
        )
        moduleValue.Functions.Add(0)
        moduleValue.Tables.Add({ ElementType = ReferenceType.FUNCREF; Limits = { Min = 1; Max = Some 2 } })
        moduleValue.Memories.Add({ Limits = { Min = 1; Max = Some 3 } })
        moduleValue.Globals.Add(Global({ ValueType = ValueType.I32; Mutable = false }, [| 0x41uy; 0x2Auy; 0x0Buy |]))
        moduleValue.Exports.Add({ Name = "main"; Kind = ExternalKind.FUNCTION; Index = 1 })
        moduleValue.Start <- Some 1
        moduleValue.Elements.Add(Element(0, [| 0x41uy; 0x00uy; 0x0Buy |], [ 1; 2 ]))
        moduleValue.Code.Add(
            FunctionBody([ ValueType.I32; ValueType.I32; ValueType.F64 ], [| 0x41uy; 0x07uy; 0x0Buy |])
        )
        moduleValue.Data.Add(DataSegment(0, [| 0x41uy; 0x00uy; 0x0Buy |], [| 0x4Euy; 0x69uy; 0x62uy |]))

        let parsed = WasmModuleParser().Parse(WasmModuleEncoder.encodeModule moduleValue)
        Assert.Equal(4, parsed.Imports.Count)
        Assert.Single(parsed.Tables) |> ignore
        Assert.Single(parsed.Memories) |> ignore
        Assert.Single(parsed.Globals) |> ignore
        Assert.Equal(Some 1, parsed.Start)
        Assert.Equal<int list>([ 1; 2 ], parsed.Elements[0].FunctionIndices)
        Assert.Equal<ValueType list>([ ValueType.I32; ValueType.I32; ValueType.F64 ], parsed.Code[0].Locals)
        Assert.Equal<byte array>([| 0x4Euy; 0x69uy; 0x62uy |], parsed.Data[0].Data)
        Assert.Equal("name", parsed.Customs[0].Name)

    [<Fact>]
    let ``invalid import descriptors are rejected`` () =
        let cases =
            [
                ExternalKind.FUNCTION, MemoryImportDescriptor { Limits = { Min = 1; Max = None } }, "function imports"
                ExternalKind.TABLE, FunctionImportDescriptor 0, "table imports"
                ExternalKind.MEMORY, FunctionImportDescriptor 0, "memory imports"
                ExternalKind.GLOBAL, FunctionImportDescriptor 0, "global imports"
                enum<ExternalKind> 255, FunctionImportDescriptor 0, "unsupported import kind"
            ]

        for kind, descriptor, message in cases do
            let moduleValue = WasmModule()
            moduleValue.Imports.Add({ ModuleName = "env"; Name = "bad"; Kind = kind; Descriptor = descriptor })
            let error = Assert.Throws<WasmEncodeError>(fun () -> WasmModuleEncoder.encodeModule moduleValue |> ignore)
            Assert.Contains(message, error.Message)

    [<Fact>]
    let ``null module is rejected`` () =
        Assert.Throws<ArgumentNullException>(fun () -> WasmModuleEncoder.encodeModule null |> ignore)
        |> ignore
