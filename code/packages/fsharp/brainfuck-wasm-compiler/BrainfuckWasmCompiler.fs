namespace CodingAdventures.BrainfuckWasmCompiler.FSharp

open System
open System.Collections.Generic
open System.IO
open CodingAdventures.WasmLeb128.FSharp
open CodingAdventures.WasmModuleEncoder.FSharp
open CodingAdventures.WasmTypes.FSharp

module Version =
    [<Literal>]
    let VERSION = "0.1.0"

type PackageError(stage: string, message: string) =
    inherit Exception(message)
    member _.Stage = stage

type PackageResult(source: string, operations: char list, wasmBytes: byte array, wasmPath: string option) =
    let copiedBytes = Array.copy wasmBytes

    member _.Source = source
    member _.Operations = operations
    member _.WasmBytes = Array.copy copiedBytes
    member _.WasmPath = wasmPath

type private ParsedProgram =
    {
        Operations: char array
        LoopEnds: IReadOnlyDictionary<int, int>
    }

[<RequireQualifiedAccess>]
module BrainfuckWasmCompiler =
    let private maxSourceLength = 1_000_000
    let private maxLoopNesting = 512
    let private wasiModule = "wasi_snapshot_preview1"

    let private parse (source: string) =
        if source.Length > maxSourceLength then
            raise (PackageError("parse", sprintf "source exceeds %d characters" maxSourceLength))

        let operations = ResizeArray<char>()
        let stack = Stack<int>()
        let loopEnds = Dictionary<int, int>()

        for sourceIndex = 0 to source.Length - 1 do
            let operation = source[sourceIndex]
            if "><+-.,[]".Contains(operation) then
                let operationIndex = operations.Count
                match operation with
                | '[' ->
                    stack.Push(operationIndex)
                    if stack.Count > maxLoopNesting then
                        raise (PackageError("parse", sprintf "loop nesting exceeds %d" maxLoopNesting))
                | ']' ->
                    if stack.Count = 0 then
                        raise (PackageError("parse", sprintf "unmatched ] at byte %d" sourceIndex))
                    loopEnds[stack.Pop()] <- operationIndex
                | _ -> ()
                operations.Add(operation)

        if stack.Count > 0 then
            raise (PackageError("parse", "unmatched ["))

        {
            Operations = operations.ToArray()
            LoopEnds = loopEnds
        }

    let private append (target: ResizeArray<byte>) (bytes: seq<byte>) =
        target.AddRange(bytes)

    let private u32 (target: ResizeArray<byte>) value =
        append target (WasmLeb128.encodeUnsignedInt value)

    let private i32 (target: ResizeArray<byte>) value =
        target.Add(0x41uy)
        append target (WasmLeb128.encodeSigned value)

    let private loadCell (target: ResizeArray<byte>) =
        target.Add(0x20uy)
        u32 target 0
        target.Add(0x2Duy)
        u32 target 0
        u32 target 0

    let private mutateCell (target: ResizeArray<byte>) delta =
        loadCell target
        i32 target delta
        target.Add(0x6Auy)
        target.Add(0x21uy)
        u32 target 1
        target.Add(0x20uy)
        u32 target 0
        target.Add(0x20uy)
        u32 target 1
        target.Add(0x3Auy)
        u32 target 0
        u32 target 0

    let private addToLocal (target: ResizeArray<byte>) local delta =
        target.Add(0x20uy)
        u32 target local
        i32 target delta
        target.Add(0x6Auy)
        target.Add(0x21uy)
        u32 target local

    let private storeByteConstAddress (target: ResizeArray<byte>) address local =
        i32 target address
        target.Add(0x20uy)
        u32 target local
        target.Add(0x3Auy)
        u32 target 0
        u32 target 0

    let private storeI32Const (target: ResizeArray<byte>) address value =
        i32 target address
        i32 target value
        target.Add(0x36uy)
        u32 target 2
        u32 target 0

    let private emitWrite (target: ResizeArray<byte>) writeIndex =
        loadCell target
        target.Add(0x21uy)
        u32 target 1
        storeByteConstAddress target 30012 1
        storeI32Const target 30000 30012
        storeI32Const target 30004 1
        i32 target 1
        i32 target 30000
        i32 target 1
        i32 target 30008
        target.Add(0x10uy)
        u32 target writeIndex
        target.Add(0x21uy)
        u32 target 2

    let private emitRead (target: ResizeArray<byte>) readIndex =
        storeByteConstAddress target 30012 0
        storeI32Const target 30000 30012
        storeI32Const target 30004 1
        i32 target 0
        i32 target 30000
        i32 target 1
        i32 target 30008
        target.Add(0x10uy)
        u32 target readIndex
        target.Add(0x21uy)
        u32 target 2
        i32 target 30012
        target.Add(0x2Duy)
        u32 target 0
        u32 target 0
        target.Add(0x21uy)
        u32 target 1
        target.Add(0x20uy)
        u32 target 0
        target.Add(0x20uy)
        u32 target 1
        target.Add(0x3Auy)
        u32 target 0
        u32 target 0

    let rec private emitOperations
        (target: ResizeArray<byte>)
        (program: ParsedProgram)
        startIndex
        endIndex
        writeIndex
        readIndex
        =
        let mutable index = startIndex
        while index < endIndex do
            match program.Operations[index] with
            | '>' -> addToLocal target 0 1
            | '<' -> addToLocal target 0 -1
            | '+' -> mutateCell target 1
            | '-' -> mutateCell target -1
            | '.' -> emitWrite target writeIndex
            | ',' -> emitRead target readIndex
            | '[' ->
                let close = program.LoopEnds[index]
                append target [ 0x02uy; 0x40uy; 0x03uy; 0x40uy ]
                loadCell target
                target.Add(0x45uy)
                target.Add(0x0Duy)
                u32 target 1
                emitOperations target program (index + 1) close writeIndex readIndex
                target.Add(0x0Cuy)
                u32 target 0
                append target [ 0x0Buy; 0x0Buy ]
                index <- close
            | _ -> ()
            index <- index + 1

    let private functionBody program writeIndex readIndex =
        let result = ResizeArray<byte>()
        emitOperations result program 0 program.Operations.Length writeIndex readIndex
        result.Add(0x0Buy)
        result.ToArray()

    let private emitModule program =
        let needsWrite = program.Operations |> Array.contains '.'
        let needsRead = program.Operations |> Array.contains ','
        let importCount = (if needsWrite then 1 else 0) + (if needsRead then 1 else 0)
        let writeIndex = if needsWrite then 0 else -1
        let readIndex = if needsRead then (if needsWrite then 1 else 0) else -1
        let moduleValue = WasmModule()
        let wasiType =
            WasmTypes.makeFuncType
                [ ValueType.I32; ValueType.I32; ValueType.I32; ValueType.I32 ]
                [ ValueType.I32 ]

        if needsWrite then
            moduleValue.Types.Add(wasiType)
            moduleValue.Imports.Add(
                {
                    ModuleName = wasiModule
                    Name = "fd_write"
                    Kind = ExternalKind.FUNCTION
                    Descriptor = FunctionImportDescriptor writeIndex
                }
            )

        if needsRead then
            moduleValue.Types.Add(wasiType)
            moduleValue.Imports.Add(
                {
                    ModuleName = wasiModule
                    Name = "fd_read"
                    Kind = ExternalKind.FUNCTION
                    Descriptor = FunctionImportDescriptor readIndex
                }
            )

        moduleValue.Types.Add(WasmTypes.makeFuncType [] [])
        moduleValue.Functions.Add(importCount)
        moduleValue.Memories.Add({ Limits = { Min = 1; Max = None } })
        moduleValue.Exports.Add({ Name = "_start"; Kind = ExternalKind.FUNCTION; Index = importCount })
        moduleValue.Exports.Add({ Name = "memory"; Kind = ExternalKind.MEMORY; Index = 0 })
        moduleValue.Code.Add(
            FunctionBody(
                [ ValueType.I32; ValueType.I32; ValueType.I32 ],
                functionBody program writeIndex readIndex
            )
        )
        WasmModuleEncoder.encodeModule moduleValue

    let compileSource (source: string) =
        if isNull source then
            nullArg "source"
        let program = parse source
        PackageResult(source, program.Operations |> Array.toList, emitModule program, None)

    let packSource source = compileSource source

    let writeWasmFile source path =
        if isNull path then
            nullArg "path"
        let result = compileSource source
        try
            File.WriteAllBytes(path, result.WasmBytes)
        with
        | :? IOException as error -> raise (PackageError("write", error.Message))
        | :? UnauthorizedAccessException as error -> raise (PackageError("write", error.Message))
        PackageResult(result.Source, result.Operations, result.WasmBytes, Some path)
