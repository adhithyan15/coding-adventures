namespace CodingAdventures.WasmTypes.FSharp.Tests

open System
open CodingAdventures.WasmTypes.FSharp
open Xunit

module WasmTypesTests =
    [<Fact>]
    let ``has version`` () =
        Assert.Equal("0.1.0", Version.VERSION)

    [<Fact>]
    let ``exposes spec constants`` () =
        Assert.Equal(0x7Fuy, byte ValueType.I32)
        Assert.Equal(0x7Euy, byte ValueType.I64)
        Assert.Equal(0x7Duy, byte ValueType.F32)
        Assert.Equal(0x7Cuy, byte ValueType.F64)
        Assert.Equal(0x40uy, BlockType.EMPTY)
        Assert.Equal(0x70uy, ReferenceType.FUNCREF)
        Assert.Equal(0x00uy, byte ExternalKind.FUNCTION)
        Assert.Equal(0x03uy, byte ExternalKind.GLOBAL)

    [<Fact>]
    let ``makeFuncType copies params and results`` () =
        let mutable parameters = [| ValueType.I32; ValueType.I64 |]
        let mutable results = [| ValueType.F64 |]
        let funcType = WasmTypes.makeFuncType parameters results

        parameters <- [| ValueType.F32; ValueType.I64 |]
        results <- [| ValueType.I32 |]

        Assert.Equal<ValueType list>([ ValueType.I32; ValueType.I64 ], funcType.Params)
        Assert.Equal<ValueType list>([ ValueType.F64 ], funcType.Results)

    [<Fact>]
    let ``supports limits and storage types`` () =
        let memory = { Limits = { Min = 1; Max = Some 8 } }
        let table = { ElementType = ReferenceType.FUNCREF; Limits = { Min = 0; Max = None } }
        let globalType = { ValueType = ValueType.I32; Mutable = false }

        Assert.Equal(1, memory.Limits.Min)
        Assert.Equal(Some 8, memory.Limits.Max)
        Assert.Equal(ReferenceType.FUNCREF, table.ElementType)
        Assert.Equal(None, table.Limits.Max)
        Assert.Equal(ValueType.I32, globalType.ValueType)
        Assert.False(globalType.Mutable)

    [<Fact>]
    let ``supports typed imports and exports`` () =
        let functionImport =
            {
                ModuleName = "env"
                Name = "add"
                Kind = ExternalKind.FUNCTION
                Descriptor = FunctionImportDescriptor 2
            }

        let memoryImport =
            {
                ModuleName = "env"
                Name = "memory"
                Kind = ExternalKind.MEMORY
                Descriptor = MemoryImportDescriptor { Limits = { Min = 1; Max = None } }
            }

        let exportValue =
            {
                Name = "main"
                Kind = ExternalKind.FUNCTION
                Index = 0
            }

        Assert.Equal("env", functionImport.ModuleName)
        Assert.Equal(FunctionImportDescriptor 2, functionImport.Descriptor)
        match memoryImport.Descriptor with
        | MemoryImportDescriptor memoryType -> Assert.Equal(1, memoryType.Limits.Min)
        | _ -> Assert.Fail("expected memory import descriptor")
        Assert.Equal("main", exportValue.Name)
        Assert.Equal(0, exportValue.Index)

    [<Fact>]
    let ``copies byte-backed structures`` () =
        let initExpr = [| 0x41uy; 0x2Auy; 0x0Buy |]
        let offsetExpr = [| 0x41uy; 0x00uy; 0x0Buy |]
        let data = [| 0x48uy; 0x69uy |]
        let code = [| 0x20uy; 0x00uy; 0x0Buy |]
        let customData = [| 0x01uy; 0x02uy |]

        let globalValue = Global({ ValueType = ValueType.I32; Mutable = false }, initExpr)
        let element = Element(0, offsetExpr, [ 0; 1; 2 ])
        let segment = DataSegment(0, offsetExpr, data)
        let body = FunctionBody([ ValueType.I32; ValueType.I32 ], code)
        let custom = CustomSection("name", customData)

        initExpr[0] <- 0x00uy
        offsetExpr[1] <- 0x05uy
        data[0] <- 0x00uy
        code[0] <- 0x00uy
        customData[0] <- 0xFFuy

        Assert.Equal<byte[]>([| 0x41uy; 0x2Auy; 0x0Buy |], globalValue.InitExpr)
        Assert.Equal<byte[]>([| 0x41uy; 0x00uy; 0x0Buy |], element.OffsetExpr)
        Assert.Equal<int list>([ 0; 1; 2 ], element.FunctionIndices)
        Assert.Equal<byte[]>([| 0x48uy; 0x69uy |], segment.Data)
        Assert.Equal<ValueType list>([ ValueType.I32; ValueType.I32 ], body.Locals)
        Assert.Equal<byte[]>([| 0x20uy; 0x00uy; 0x0Buy |], body.Code)
        Assert.Equal<byte[]>([| 0x01uy; 0x02uy |], custom.Data)

    [<Fact>]
    let ``wasm module starts empty and can be populated`` () =
        let moduleValue = WasmModule()

        Assert.Empty(moduleValue.Types)
        Assert.Empty(moduleValue.Imports)
        Assert.Empty(moduleValue.Functions)
        Assert.Empty(moduleValue.Tables)
        Assert.Empty(moduleValue.Memories)
        Assert.Empty(moduleValue.Globals)
        Assert.Empty(moduleValue.Exports)
        Assert.Empty(moduleValue.Elements)
        Assert.Empty(moduleValue.Code)
        Assert.Empty(moduleValue.Data)
        Assert.Empty(moduleValue.Customs)
        Assert.Equal(None, moduleValue.Start)

        let funcType = WasmTypes.makeFuncType [ ValueType.I32 ] [ ValueType.I32 ]
        moduleValue.Types.Add(funcType)
        moduleValue.Functions.Add(0)
        moduleValue.Exports.Add({ Name = "main"; Kind = ExternalKind.FUNCTION; Index = 0 })
        moduleValue.Start <- Some 0

        Assert.Equal(1, moduleValue.Types.Count)
        Assert.Equal(ValueType.I32, moduleValue.Types[0].Params[0])
        Assert.Equal(0, moduleValue.Functions[0])
        Assert.Equal("main", moduleValue.Exports[0].Name)
        Assert.Equal(Some 0, moduleValue.Start)
