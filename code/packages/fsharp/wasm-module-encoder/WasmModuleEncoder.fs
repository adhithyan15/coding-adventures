namespace CodingAdventures.WasmModuleEncoder.FSharp

open System
open System.Collections.Generic
open System.Text
open CodingAdventures.WasmLeb128.FSharp
open CodingAdventures.WasmTypes.FSharp

module Version =
    [<Literal>]
    let VERSION = "0.1.0"

type WasmEncodeError(message: string) =
    inherit Exception(message)

[<RequireQualifiedAccess>]
module WasmModuleEncoder =
    let WASM_MAGIC = [| 0x00uy; 0x61uy; 0x73uy; 0x6Duy |]
    let WASM_VERSION = [| 0x01uy; 0x00uy; 0x00uy; 0x00uy |]

    let private append (target: ResizeArray<byte>) (bytes: seq<byte>) =
        target.AddRange(bytes)

    let private u32 value = WasmLeb128.encodeUnsignedInt value

    let private section sectionId (payload: byte array) =
        let result = ResizeArray<byte>(payload.Length + 6)
        result.Add(sectionId)
        append result (u32 payload.Length)
        append result payload
        result.ToArray()

    let private name (text: string) =
        let data = Encoding.UTF8.GetBytes(text)
        let result = ResizeArray<byte>(data.Length + 5)
        append result (u32 data.Length)
        append result data
        result.ToArray()

    let private vector (values: seq<'T>) (encoder: 'T -> byte array) =
        let materialized = values |> Seq.toArray
        let result = ResizeArray<byte>()
        append result (u32 materialized.Length)
        for value in materialized do
            append result (encoder value)
        result.ToArray()

    let private valueTypes (values: seq<ValueType>) =
        let materialized = values |> Seq.toArray
        let result = ResizeArray<byte>()
        append result (u32 materialized.Length)
        for value in materialized do
            result.Add(byte value)
        result.ToArray()

    let private encodeFuncType (funcType: FuncType) =
        let result = ResizeArray<byte>()
        result.Add(0x60uy)
        append result (valueTypes funcType.Params)
        append result (valueTypes funcType.Results)
        result.ToArray()

    let private encodeLimits (limits: Limits) =
        let result = ResizeArray<byte>()
        match limits.Max with
        | Some maximum ->
            result.Add(0x01uy)
            append result (u32 limits.Min)
            append result (u32 maximum)
        | None ->
            result.Add(0x00uy)
            append result (u32 limits.Min)
        result.ToArray()

    let private encodeMemoryType (memoryType: MemoryType) =
        encodeLimits memoryType.Limits

    let private encodeTableType (tableType: TableType) =
        let result = ResizeArray<byte>()
        result.Add(tableType.ElementType)
        append result (encodeLimits tableType.Limits)
        result.ToArray()

    let private encodeGlobalType (globalType: GlobalType) =
        [| byte globalType.ValueType; if globalType.Mutable then 0x01uy else 0x00uy |]

    let private encodeImport (importValue: Import) =
        let result = ResizeArray<byte>()
        append result (name importValue.ModuleName)
        append result (name importValue.Name)
        result.Add(byte importValue.Kind)

        match importValue.Kind, importValue.Descriptor with
        | ExternalKind.FUNCTION, FunctionImportDescriptor typeIndex -> append result (u32 typeIndex)
        | ExternalKind.FUNCTION, _ -> raise (WasmEncodeError("function imports require a FunctionImportDescriptor"))
        | ExternalKind.TABLE, TableImportDescriptor tableType -> append result (encodeTableType tableType)
        | ExternalKind.TABLE, _ -> raise (WasmEncodeError("table imports require a TableImportDescriptor"))
        | ExternalKind.MEMORY, MemoryImportDescriptor memoryType -> append result (encodeMemoryType memoryType)
        | ExternalKind.MEMORY, _ -> raise (WasmEncodeError("memory imports require a MemoryImportDescriptor"))
        | ExternalKind.GLOBAL, GlobalImportDescriptor globalType -> append result (encodeGlobalType globalType)
        | ExternalKind.GLOBAL, _ -> raise (WasmEncodeError("global imports require a GlobalImportDescriptor"))
        | kind, _ -> raise (WasmEncodeError(sprintf "unsupported import kind: %d" (byte kind)))

        result.ToArray()

    let private encodeExport (exportValue: Export) =
        let result = ResizeArray<byte>()
        append result (name exportValue.Name)
        result.Add(byte exportValue.Kind)
        append result (u32 exportValue.Index)
        result.ToArray()

    let private encodeGlobal (globalValue: Global) =
        let result = ResizeArray<byte>()
        append result (encodeGlobalType globalValue.GlobalType)
        append result globalValue.InitExpr
        result.ToArray()

    let private encodeElement (element: Element) =
        let result = ResizeArray<byte>()
        append result (u32 element.TableIndex)
        append result element.OffsetExpr
        append result (u32 element.FunctionIndices.Length)
        for functionIndex in element.FunctionIndices do
            append result (u32 functionIndex)
        result.ToArray()

    let private encodeDataSegment (segment: DataSegment) =
        let result = ResizeArray<byte>()
        append result (u32 segment.MemoryIndex)
        append result segment.OffsetExpr
        append result (u32 segment.Data.Length)
        append result segment.Data
        result.ToArray()

    let private groupLocals (locals: ValueType list) =
        match locals with
        | [] -> []
        | first :: rest ->
            let groups = ResizeArray<int * ValueType>()
            let mutable currentType = first
            let mutable count = 1

            for valueType in rest do
                if valueType = currentType then
                    count <- count + 1
                else
                    groups.Add(count, currentType)
                    currentType <- valueType
                    count <- 1

            groups.Add(count, currentType)
            groups |> Seq.toList

    let private encodeFunctionBody (body: FunctionBody) =
        let localGroups = groupLocals body.Locals
        let payload = ResizeArray<byte>()
        append payload (u32 localGroups.Length)
        for count, valueType in localGroups do
            append payload (u32 count)
            payload.Add(byte valueType)
        append payload body.Code

        let result = ResizeArray<byte>()
        append result (u32 payload.Count)
        append result payload
        result.ToArray()

    let private encodeCustom (custom: CustomSection) =
        let result = ResizeArray<byte>()
        append result (name custom.Name)
        append result custom.Data
        result.ToArray()

    let private addVectorSection
        (destination: ResizeArray<byte>)
        sectionId
        (values: seq<'T>)
        (encoder: 'T -> byte array)
        =
        let materialized = values |> Seq.toArray
        if materialized.Length > 0 then
            append destination (section sectionId (vector materialized encoder))

    let encodeModule (moduleValue: WasmModule) =
        if isNull (box moduleValue) then
            nullArg "moduleValue"

        let result = ResizeArray<byte>(WASM_MAGIC.Length + WASM_VERSION.Length)
        append result WASM_MAGIC
        append result WASM_VERSION

        for custom in moduleValue.Customs do
            append result (section 0uy (encodeCustom custom))

        addVectorSection result 1uy moduleValue.Types encodeFuncType
        addVectorSection result 2uy moduleValue.Imports encodeImport
        addVectorSection result 3uy moduleValue.Functions u32
        addVectorSection result 4uy moduleValue.Tables encodeTableType
        addVectorSection result 5uy moduleValue.Memories encodeMemoryType
        addVectorSection result 6uy moduleValue.Globals encodeGlobal
        addVectorSection result 7uy moduleValue.Exports encodeExport

        match moduleValue.Start with
        | Some start -> append result (section 8uy (u32 start))
        | None -> ()

        addVectorSection result 9uy moduleValue.Elements encodeElement
        addVectorSection result 10uy moduleValue.Code encodeFunctionBody
        addVectorSection result 11uy moduleValue.Data encodeDataSegment
        result.ToArray()
