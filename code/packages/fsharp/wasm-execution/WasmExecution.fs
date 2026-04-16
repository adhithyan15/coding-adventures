namespace CodingAdventures.WasmExecution.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic
open CodingAdventures.WasmLeb128.FSharp
open CodingAdventures.WasmOpcodes.FSharp
open CodingAdventures.WasmTypes.FSharp

module Version =
    [<Literal>]
    let VERSION = "0.1.0"

type TrapError(message: string) =
    inherit Exception(message)

type WasmValue =
    | I32 of int
    | I64 of int64
    | F32 of single
    | F64 of double

[<RequireQualifiedAccess>]
module WasmValue =
    let valueType value =
        match value with
        | I32 _ -> ValueType.I32
        | I64 _ -> ValueType.I64
        | F32 _ -> ValueType.F32
        | F64 _ -> ValueType.F64

    let defaultFor valueType =
        match valueType with
        | ValueType.I32 -> I32 0
        | ValueType.I64 -> I64 0L
        | ValueType.F32 -> F32 0.0f
        | ValueType.F64 -> F64 0.0
        | _ -> raise (TrapError(sprintf "Unsupported wasm value type %A" valueType))

    let asI32 value =
        match value with
        | I32 current -> current
        | _ -> raise (TrapError(sprintf "Type mismatch: expected i32, got %A" (valueType value)))

    let asI64 value =
        match value with
        | I64 current -> current
        | _ -> raise (TrapError(sprintf "Type mismatch: expected i64, got %A" (valueType value)))

    let asF32 value =
        match value with
        | F32 current -> current
        | _ -> raise (TrapError(sprintf "Type mismatch: expected f32, got %A" (valueType value)))

    let asF64 value =
        match value with
        | F64 current -> current
        | _ -> raise (TrapError(sprintf "Type mismatch: expected f64, got %A" (valueType value)))

type LinearMemory(initialPages: int, ?maxPages: int) =
    let mutable data = Array.zeroCreate<byte> (initialPages * 65536)
    let maxPagesValue = maxPages
    let mutable currentPages = initialPages

    let ensureBounds offset size =
        if offset < 0 || size < 0 || offset + size > data.Length then
            raise (TrapError(sprintf "Memory access out of bounds at offset %d for %d bytes" offset size))

    member _.Size() = currentPages
    member _.ByteLength() = data.Length

    member _.Grow(deltaPages: int) =
        if deltaPages < 0 then
            raise (TrapError("Memory cannot grow by a negative number of pages"))

        let oldPages = currentPages
        let newPages = currentPages + deltaPages

        match maxPagesValue with
        | Some limit when newPages > limit -> -1
        | _ ->
            if newPages <> currentPages then
                let newData = Array.zeroCreate<byte> (newPages * 65536)
                Buffer.BlockCopy(data, 0, newData, 0, data.Length)
                data <- newData
                currentPages <- newPages

            oldPages

    member _.ReadBytes(offset: int, length: int) =
        ensureBounds offset length
        data[offset .. offset + length - 1]

    member _.WriteBytes(offset: int, bytes: byte array) =
        ensureBounds offset bytes.Length
        Buffer.BlockCopy(bytes, 0, data, offset, bytes.Length)

    member _.LoadI32(offset: int) =
        ensureBounds offset 4
        BinaryPrimitives.ReadInt32LittleEndian(ReadOnlySpan(data, offset, 4))

    member _.StoreI32(offset: int, value: int) =
        ensureBounds offset 4
        BinaryPrimitives.WriteInt32LittleEndian(Span(data, offset, 4), value)

    member _.LoadI64(offset: int) =
        ensureBounds offset 8
        BinaryPrimitives.ReadInt64LittleEndian(ReadOnlySpan(data, offset, 8))

    member _.StoreI64(offset: int, value: int64) =
        ensureBounds offset 8
        BinaryPrimitives.WriteInt64LittleEndian(Span(data, offset, 8), value)

    member this.LoadF32(offset: int) = this.LoadI32(offset) |> BitConverter.Int32BitsToSingle
    member this.StoreF32(offset: int, value: single) = this.StoreI32(offset, BitConverter.SingleToInt32Bits(value))
    member this.LoadF64(offset: int) = this.LoadI64(offset) |> BitConverter.Int64BitsToDouble
    member this.StoreF64(offset: int, value: double) = this.StoreI64(offset, BitConverter.DoubleToInt64Bits(value))

    member _.LoadI32_8u(offset: int) =
        ensureBounds offset 1
        int data[offset]

    member _.StoreI32_8(offset: int, value: int) =
        ensureBounds offset 1
        data[offset] <- byte value

    member _.StoreI32_16(offset: int, value: int) =
        ensureBounds offset 2
        BinaryPrimitives.WriteInt16LittleEndian(Span(data, offset, 2), int16 value)

    member this.StoreI64_8(offset: int, value: int64) = this.StoreI32_8(offset, int value)
    member this.StoreI64_16(offset: int, value: int64) = this.StoreI32_16(offset, int value)
    member this.StoreI64_32(offset: int, value: int64) = this.StoreI32(offset, int value)

type Table(size: int) =
    let elements = Array.create<int option> size None
    member _.Item
        with get index = elements[index]
        and set index value = elements[index] <- value

type IHostFunction =
    abstract member FuncType: FuncType
    abstract member Call: WasmValue list -> WasmValue list

type IHostInterface =
    abstract member ResolveFunction: string * string -> IHostFunction option

type HostFunction(funcType: FuncType, callback: WasmValue list -> WasmValue list) =
    interface IHostFunction with
        member _.FuncType = funcType
        member _.Call(args) = callback args

type DecodedInstruction =
    {
        Info: OpcodeInfo
        Index: int option
        Align: int option
        MemoryOffset: int option
        Constant: WasmValue option
    }

type WasmExecutionEngineOptions =
    {
        Memory: LinearMemory option
        Tables: Table list
        Globals: WasmValue list
        GlobalTypes: GlobalType list
        FuncTypes: FuncType list
        FuncBodies: FunctionBody option list
        HostFunctions: IHostFunction option list
    }

[<RequireQualifiedAccess>]
module WasmExecution =
    let private decodeInstructions (bytes: byte array) =
        let mutable offset = 0
        let instructions = ResizeArray<DecodedInstruction>()

        while offset < bytes.Length do
            let opcode = bytes[offset]
            offset <- offset + 1
            let info = WasmOpcodes.getOpcode opcode |> Option.defaultWith (fun () -> raise (TrapError(sprintf "Unknown opcode 0x%02x" opcode)))
            let mutable index = None
            let mutable align = None
            let mutable memoryOffset = None
            let mutable constant = None

            for immediate in info.Immediates do
                match immediate with
                | "blocktype" ->
                    offset <- offset + 1
                | "labelidx"
                | "funcidx"
                | "typeidx"
                | "localidx"
                | "globalidx"
                | "tableidx"
                | "memidx" ->
                    let value, consumed = WasmLeb128.decodeUnsignedAt bytes offset
                    offset <- offset + consumed
                    index <- Some(int value)
                | "memarg" ->
                    let alignValue, consumed1 = WasmLeb128.decodeUnsignedAt bytes offset
                    offset <- offset + consumed1
                    let offsetValue, consumed2 = WasmLeb128.decodeUnsignedAt bytes offset
                    offset <- offset + consumed2
                    align <- Some(int alignValue)
                    memoryOffset <- Some(int offsetValue)
                | "i32" ->
                    let value, consumed = WasmLeb128.decodeSignedAt bytes offset
                    offset <- offset + consumed
                    constant <- Some(I32 value)
                | "i64" ->
                    let value, consumed = WasmLeb128.decodeSignedAt bytes offset
                    offset <- offset + consumed
                    constant <- Some(I64(int64 value))
                | "f32" ->
                    let value = BinaryPrimitives.ReadInt32LittleEndian(ReadOnlySpan(bytes, offset, 4)) |> BitConverter.Int32BitsToSingle
                    offset <- offset + 4
                    constant <- Some(F32 value)
                | "f64" ->
                    let value = BinaryPrimitives.ReadInt64LittleEndian(ReadOnlySpan(bytes, offset, 8)) |> BitConverter.Int64BitsToDouble
                    offset <- offset + 8
                    constant <- Some(F64 value)
                | "vec_labelidx" ->
                    let count, consumed = WasmLeb128.decodeUnsignedAt bytes offset
                    offset <- offset + consumed
                    for _ in 0 .. int count do
                        let _, size = WasmLeb128.decodeUnsignedAt bytes offset
                        offset <- offset + size
                | other ->
                    raise (TrapError(sprintf "Unsupported immediate '%s'" other))

            instructions.Add({ Info = info; Index = index; Align = align; MemoryOffset = memoryOffset; Constant = constant })

        instructions |> Seq.toList

    let decodeFunctionBody (body: FunctionBody) = decodeInstructions body.Code

    let evaluateConstExpr (expr: byte array) (importedGlobals: WasmValue list) =
        let stack = Stack<WasmValue>()

        for instruction in decodeInstructions expr do
            match instruction.Info.Name with
            | "i32.const"
            | "i64.const"
            | "f32.const"
            | "f64.const" -> stack.Push(instruction.Constant.Value)
            | "global.get" ->
                let index = defaultArg instruction.Index -1
                if index < 0 || index >= importedGlobals.Length then
                    raise (TrapError(sprintf "Constant expression references unavailable global %d" index))
                stack.Push(importedGlobals[index])
            | "end" -> ()
            | name -> raise (TrapError(sprintf "Opcode '%s' is not allowed in a constant expression" name))

        stack |> Seq.rev |> Seq.toList

type WasmExecutionEngine(options: WasmExecutionEngineOptions) =
    let memory = options.Memory
    let globals = ResizeArray<WasmValue>(options.Globals)

    let pop (stack: ResizeArray<WasmValue>) =
        if stack.Count = 0 then
            raise (TrapError("Operand stack underflow"))
        let value = stack[stack.Count - 1]
        stack.RemoveAt(stack.Count - 1)
        value

    let requireMemory() =
        match memory with
        | Some current -> current
        | None -> raise (TrapError("Instruction requires linear memory, but none is configured"))

    let ensureIndex kind length index =
        if index < 0 || index >= length then
            raise (TrapError(sprintf "Invalid %s index %d" kind index))
        index

    let rec callFunction functionIndex (args: WasmValue list) =
        ensureIndex "function" options.FuncTypes.Length functionIndex |> ignore
        let funcType = options.FuncTypes[functionIndex]
        if funcType.Params.Length <> args.Length then
            raise (TrapError(sprintf "Function %d expects %d argument(s), got %d" functionIndex funcType.Params.Length args.Length))

        match options.HostFunctions[functionIndex], options.FuncBodies[functionIndex] with
        | Some hostFunction, _ -> hostFunction.Call args
        | None, Some body ->
            let locals = Array.zeroCreate<WasmValue> (funcType.Params.Length + body.Locals.Length)
            args |> List.iteri (fun index arg -> locals[index] <- arg)
            body.Locals |> List.iteri (fun index localType -> locals[funcType.Params.Length + index] <- WasmValue.defaultFor localType)

            let stack = ResizeArray<WasmValue>()
            let instructions = WasmExecution.decodeFunctionBody body

            let resolveAddress instruction =
                let baseAddress = pop stack |> WasmValue.asI32
                baseAddress + defaultArg instruction.MemoryOffset 0

            let collectResults() =
                [ for index in funcType.Results.Length - 1 .. -1 .. 0 do yield pop stack ]
                |> List.rev

            let mutable returned = None

            for instruction in instructions do
                if Option.isNone returned then
                    match instruction.Info.Name with
                    | "nop" -> ()
                    | "drop" -> pop stack |> ignore
                    | "local.get" ->
                        stack.Add(locals[ensureIndex "local" locals.Length (defaultArg instruction.Index -1)])
                    | "local.set" ->
                        locals[ensureIndex "local" locals.Length (defaultArg instruction.Index -1)] <- pop stack
                    | "local.tee" ->
                        let value = pop stack
                        locals[ensureIndex "local" locals.Length (defaultArg instruction.Index -1)] <- value
                        stack.Add(value)
                    | "global.get" ->
                        stack.Add(globals[ensureIndex "global" globals.Count (defaultArg instruction.Index -1)])
                    | "global.set" ->
                        let index = ensureIndex "global" globals.Count (defaultArg instruction.Index -1)
                        if not options.GlobalTypes[index].Mutable then
                            raise (TrapError(sprintf "Global %d is immutable" index))
                        globals[index] <- pop stack
                    | "i32.const"
                    | "i64.const"
                    | "f32.const"
                    | "f64.const" -> stack.Add(instruction.Constant.Value)
                    | "i32.add" ->
                        let right = pop stack |> WasmValue.asI32
                        let left = pop stack |> WasmValue.asI32
                        stack.Add(I32(left + right))
                    | "i32.sub" ->
                        let right = pop stack |> WasmValue.asI32
                        let left = pop stack |> WasmValue.asI32
                        stack.Add(I32(left - right))
                    | "i32.mul" ->
                        let right = pop stack |> WasmValue.asI32
                        let left = pop stack |> WasmValue.asI32
                        stack.Add(I32(left * right))
                    | "call" ->
                        let target = ensureIndex "function" options.FuncTypes.Length (defaultArg instruction.Index -1)
                        let targetType = options.FuncTypes[target]
                        let callArgs =
                            [ for _ in 1 .. targetType.Params.Length -> pop stack ]
                            |> List.rev
                        for result in callFunction target callArgs do
                            stack.Add(result)
                    | "i32.load" ->
                        stack.Add(I32(requireMemory().LoadI32(resolveAddress instruction)))
                    | "i32.store" ->
                        let value = pop stack |> WasmValue.asI32
                        requireMemory().StoreI32(resolveAddress instruction, value)
                    | "end"
                    | "return" ->
                        returned <- Some(collectResults())
                    | name ->
                        raise (TrapError(sprintf "Instruction '%s' is not implemented in the F# wasm execution engine yet" name))

            match returned with
            | Some results -> results
            | None -> raise (TrapError(sprintf "Function %d terminated without end" functionIndex))
        | _ -> raise (TrapError(sprintf "Function %d has neither a body nor a host binding" functionIndex))

    member _.Memory = memory
    member _.Globals = globals |> Seq.toList
    member _.CallFunction(functionIndex: int, args: WasmValue list) = callFunction functionIndex args
