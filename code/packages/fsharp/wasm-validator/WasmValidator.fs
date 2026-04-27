namespace CodingAdventures.WasmValidator.FSharp

open System
open CodingAdventures.WasmLeb128.FSharp
open CodingAdventures.WasmOpcodes.FSharp
open CodingAdventures.WasmTypes.FSharp

module Version =
    [<Literal>]
    let VERSION = "0.1.0"

type ValidationErrorKind =
    | InvalidTypeIndex
    | InvalidFuncIndex
    | InvalidTableIndex
    | InvalidMemoryIndex
    | InvalidGlobalIndex
    | InvalidLocalIndex
    | MultipleMemories
    | MultipleTables
    | DuplicateExportName
    | ExportIndexOutOfRange
    | StartFunctionBadType
    | ImmutableGlobalWrite
    | InitExprInvalid
    | InvalidFunctionShape

type ValidationError(kind: ValidationErrorKind, message: string) =
    inherit Exception(message)

    member _.Kind = kind

type IndexSpaces =
    {
        FuncTypes: FuncType list
        NumImportedFuncs: int
        TableTypes: TableType list
        NumImportedTables: int
        MemoryTypes: MemoryType list
        NumImportedMemories: int
        GlobalTypes: GlobalType list
        NumImportedGlobals: int
        NumTypes: int
    }

type ValidatedModule =
    {
        Module: WasmModule
        FuncTypes: FuncType list
        FuncLocals: ValueType list list
    }

type private DecodedInstruction =
    {
        Info: OpcodeInfo
        FuncIndex: int option
        TypeIndex: int option
        LocalIndex: int option
        GlobalIndex: int option
    }

module private Helpers =
    let fail kind message = raise (ValidationError(kind, message))

    let ensureIndex kind length index message =
        if index < 0 || index >= length then
            fail kind message

    let decodeInstructions context (bytes: byte array) =
        let mutable offset = 0
        let instructions = ResizeArray<DecodedInstruction>()

        while offset < bytes.Length do
            let opcode = bytes[offset]
            offset <- offset + 1

            let info =
                WasmOpcodes.getOpcode opcode
                |> Option.defaultWith (fun () -> fail InitExprInvalid (sprintf "Unknown opcode 0x%02x in %s" opcode context))

            let mutable funcIndex = None
            let mutable typeIndex = None
            let mutable localIndex = None
            let mutable globalIndex = None

            for immediate in info.Immediates do
                match immediate with
                | "blocktype" ->
                    offset <- offset + 1
                | "labelidx"
                | "tableidx"
                | "memidx" ->
                    let _, consumed = WasmLeb128.decodeUnsignedAt bytes offset
                    offset <- offset + consumed
                | "funcidx" ->
                    let value, consumed = WasmLeb128.decodeUnsignedAt bytes offset
                    offset <- offset + consumed
                    funcIndex <- Some(int value)
                | "typeidx" ->
                    let value, consumed = WasmLeb128.decodeUnsignedAt bytes offset
                    offset <- offset + consumed
                    typeIndex <- Some(int value)
                | "localidx" ->
                    let value, consumed = WasmLeb128.decodeUnsignedAt bytes offset
                    offset <- offset + consumed
                    localIndex <- Some(int value)
                | "globalidx" ->
                    let value, consumed = WasmLeb128.decodeUnsignedAt bytes offset
                    offset <- offset + consumed
                    globalIndex <- Some(int value)
                | "memarg" ->
                    let _, consumed1 = WasmLeb128.decodeUnsignedAt bytes offset
                    offset <- offset + consumed1
                    let _, consumed2 = WasmLeb128.decodeUnsignedAt bytes offset
                    offset <- offset + consumed2
                | "i32"
                | "i64" ->
                    let _, consumed = WasmLeb128.decodeSignedAt bytes offset
                    offset <- offset + consumed
                | "f32" ->
                    offset <- offset + 4
                | "f64" ->
                    offset <- offset + 8
                | "vec_labelidx" ->
                    let count, consumed = WasmLeb128.decodeUnsignedAt bytes offset
                    offset <- offset + consumed
                    for _ in 0 .. int count do
                        let _, size = WasmLeb128.decodeUnsignedAt bytes offset
                        offset <- offset + size
                | other ->
                    fail InitExprInvalid (sprintf "Unsupported immediate '%s' in %s" other context)

            instructions.Add(
                {
                    Info = info
                    FuncIndex = funcIndex
                    TypeIndex = typeIndex
                    LocalIndex = localIndex
                    GlobalIndex = globalIndex
                }
            )

        instructions |> Seq.toList

module WasmValidator =
    let private buildIndexSpaces (moduleValue: WasmModule) =
        if moduleValue.Functions.Count <> moduleValue.Code.Count then
            Helpers.fail InvalidFuncIndex "Function and code section counts differ"

        let importedFuncTypes = ResizeArray<FuncType>()
        let tableTypes = ResizeArray<TableType>()
        let memoryTypes = ResizeArray<MemoryType>()
        let globalTypes = ResizeArray<GlobalType>()
        let mutable importedFuncs = 0
        let mutable importedTables = 0
        let mutable importedMemories = 0
        let mutable importedGlobals = 0

        for importEntry in moduleValue.Imports do
            match importEntry.Kind, importEntry.Descriptor with
            | ExternalKind.FUNCTION, FunctionImportDescriptor typeIndex ->
                Helpers.ensureIndex InvalidTypeIndex moduleValue.Types.Count typeIndex (sprintf "Invalid imported function type index %d" typeIndex)
                importedFuncTypes.Add(moduleValue.Types[typeIndex])
                importedFuncs <- importedFuncs + 1
            | ExternalKind.TABLE, TableImportDescriptor tableType ->
                tableTypes.Add(tableType)
                importedTables <- importedTables + 1
            | ExternalKind.MEMORY, MemoryImportDescriptor memoryType ->
                memoryTypes.Add(memoryType)
                importedMemories <- importedMemories + 1
            | ExternalKind.GLOBAL, GlobalImportDescriptor globalType ->
                globalTypes.Add(globalType)
                importedGlobals <- importedGlobals + 1
            | _ -> ()

        for typeIndex in moduleValue.Functions do
            Helpers.ensureIndex InvalidTypeIndex moduleValue.Types.Count typeIndex (sprintf "Invalid local function type index %d" typeIndex)
            importedFuncTypes.Add(moduleValue.Types[typeIndex])

        tableTypes.AddRange(moduleValue.Tables)
        memoryTypes.AddRange(moduleValue.Memories)
        globalTypes.AddRange(moduleValue.Globals |> Seq.map (fun globalValue -> globalValue.GlobalType))

        {
            FuncTypes = importedFuncTypes |> Seq.toList
            NumImportedFuncs = importedFuncs
            TableTypes = tableTypes |> Seq.toList
            NumImportedTables = importedTables
            MemoryTypes = memoryTypes |> Seq.toList
            NumImportedMemories = importedMemories
            GlobalTypes = globalTypes |> Seq.toList
            NumImportedGlobals = importedGlobals
            NumTypes = moduleValue.Types.Count
        }

    let validateConstExpr (expr: byte array) (expectedType: ValueType) (indexSpaces: IndexSpaces) =
        let stack = Collections.Generic.Stack<ValueType>()

        for instruction in Helpers.decodeInstructions "constant expression" expr do
            match instruction.Info.Name with
            | "i32.const" -> stack.Push(ValueType.I32)
            | "i64.const" -> stack.Push(ValueType.I64)
            | "f32.const" -> stack.Push(ValueType.F32)
            | "f64.const" -> stack.Push(ValueType.F64)
            | "global.get" ->
                let index = defaultArg instruction.GlobalIndex -1
                if index < 0 || index >= indexSpaces.NumImportedGlobals then
                    Helpers.fail InitExprInvalid (sprintf "Constant expression may only access imported globals, saw %d" index)
                stack.Push(indexSpaces.GlobalTypes[index].ValueType)
            | "end" ->
                if stack.Count <> 1 || stack.Peek() <> expectedType then
                    Helpers.fail InitExprInvalid (sprintf "Constant expression must leave exactly %A on the stack" expectedType)
            | name ->
                Helpers.fail InitExprInvalid (sprintf "Opcode '%s' is not allowed in a constant expression" name)

    let private validateFunction (funcIndex: int) (funcType: FuncType) (body: FunctionBody) (indexSpaces: IndexSpaces) =
        let locals = funcType.Params @ body.Locals

        for instruction in Helpers.decodeInstructions (sprintf "function %d" funcIndex) body.Code do
            match instruction.Info.Name with
            | "local.get"
            | "local.set"
            | "local.tee" ->
                let index = defaultArg instruction.LocalIndex -1
                Helpers.ensureIndex InvalidLocalIndex locals.Length index (sprintf "Local index %d is out of range" index)
            | "global.get" ->
                let index = defaultArg instruction.GlobalIndex -1
                Helpers.ensureIndex InvalidGlobalIndex indexSpaces.GlobalTypes.Length index (sprintf "Global index %d is out of range" index)
            | "global.set" ->
                let index = defaultArg instruction.GlobalIndex -1
                Helpers.ensureIndex InvalidGlobalIndex indexSpaces.GlobalTypes.Length index (sprintf "Global index %d is out of range" index)
                if not indexSpaces.GlobalTypes[index].Mutable then
                    Helpers.fail ImmutableGlobalWrite (sprintf "Global %d is immutable" index)
            | "call" ->
                let index = defaultArg instruction.FuncIndex -1
                Helpers.ensureIndex InvalidFuncIndex indexSpaces.FuncTypes.Length index (sprintf "Function index %d is out of range" index)
            | name when instruction.Info.Category = "memory" && List.isEmpty indexSpaces.MemoryTypes ->
                Helpers.fail InvalidMemoryIndex (sprintf "Instruction '%s' requires memory" name)
            | _ -> ()

        if body.Code.Length = 0 || body.Code[body.Code.Length - 1] <> 0x0Buy then
            Helpers.fail InvalidFunctionShape (sprintf "Function %d does not end with opcode 0x0B" funcIndex)

        locals

    let validateStructure (moduleValue: WasmModule) =
        let indexSpaces = buildIndexSpaces moduleValue

        if indexSpaces.TableTypes.Length > 1 then
            Helpers.fail MultipleTables "WASM 1.0 allows at most one table"

        if indexSpaces.MemoryTypes.Length > 1 then
            Helpers.fail MultipleMemories "WASM 1.0 allows at most one memory"

        let seenExports = Collections.Generic.HashSet<string>()

        for exportEntry in moduleValue.Exports do
            if not (seenExports.Add(exportEntry.Name)) then
                Helpers.fail DuplicateExportName (sprintf "Duplicate export '%s'" exportEntry.Name)

            let upperBound =
                match exportEntry.Kind with
                | ExternalKind.FUNCTION -> indexSpaces.FuncTypes.Length
                | ExternalKind.TABLE -> indexSpaces.TableTypes.Length
                | ExternalKind.MEMORY -> indexSpaces.MemoryTypes.Length
                | ExternalKind.GLOBAL -> indexSpaces.GlobalTypes.Length
                | _ -> 0

            Helpers.ensureIndex ExportIndexOutOfRange upperBound exportEntry.Index (sprintf "Export '%s' index out of range" exportEntry.Name)

        match moduleValue.Start with
        | Some startIndex ->
            Helpers.ensureIndex InvalidFuncIndex indexSpaces.FuncTypes.Length startIndex (sprintf "Start function %d is out of range" startIndex)
            let startType = indexSpaces.FuncTypes[startIndex]
            if not (List.isEmpty startType.Params && List.isEmpty startType.Results) then
                Helpers.fail StartFunctionBadType "Start function must have type () -> ()"
        | None -> ()

        for globalValue in moduleValue.Globals do
            validateConstExpr globalValue.InitExpr globalValue.GlobalType.ValueType indexSpaces

        for dataSegment in moduleValue.Data do
            if dataSegment.MemoryIndex <> 0 || dataSegment.MemoryIndex >= indexSpaces.MemoryTypes.Length then
                Helpers.fail InvalidMemoryIndex (sprintf "Data segment references missing memory %d" dataSegment.MemoryIndex)
            validateConstExpr dataSegment.OffsetExpr ValueType.I32 indexSpaces

        indexSpaces

    let validate (moduleValue: WasmModule) =
        let indexSpaces = validateStructure moduleValue
        let funcLocals =
            moduleValue.Code
            |> Seq.mapi (fun index body -> validateFunction (indexSpaces.NumImportedFuncs + index) indexSpaces.FuncTypes[indexSpaces.NumImportedFuncs + index] body indexSpaces)
            |> Seq.toList

        {
            Module = moduleValue
            FuncTypes = indexSpaces.FuncTypes
            FuncLocals = funcLocals
        }
