module CodingAdventures.WasmSimulator.FSharp

open System
open System.Collections.Generic

module Version =
    [<Literal>]
    let VERSION = "0.1.0"

[<Literal>]
let OP_END = 0x0Buy

[<Literal>]
let OP_LOCAL_GET = 0x20uy

[<Literal>]
let OP_LOCAL_SET = 0x21uy

[<Literal>]
let OP_I32_CONST = 0x41uy

[<Literal>]
let OP_I32_ADD = 0x6Auy

[<Literal>]
let OP_I32_SUB = 0x6Buy

type WasmInstruction =
    {
        Opcode: byte
        Mnemonic: string
        Operand: int option
        Size: int
    }

type WasmStepTrace =
    {
        Pc: int
        Instruction: WasmInstruction
        StackBefore: int list
        StackAfter: int list
        LocalsSnapshot: int list
        Description: string
        Halted: bool
    }

type WasmDecoder() =
    member _.Decode(bytecode: byte array, pc: int) =
        match bytecode[pc] with
        | value when value = OP_I32_CONST ->
            {
                Opcode = value
                Mnemonic = "i32.const"
                Operand = Some(BitConverter.ToInt32(bytecode, pc + 1))
                Size = 5
            }
        | value when value = OP_I32_ADD ->
            { Opcode = value; Mnemonic = "i32.add"; Operand = None; Size = 1 }
        | value when value = OP_I32_SUB ->
            { Opcode = value; Mnemonic = "i32.sub"; Operand = None; Size = 1 }
        | value when value = OP_LOCAL_GET ->
            { Opcode = value; Mnemonic = "local.get"; Operand = Some(int bytecode[pc + 1]); Size = 2 }
        | value when value = OP_LOCAL_SET ->
            { Opcode = value; Mnemonic = "local.set"; Operand = Some(int bytecode[pc + 1]); Size = 2 }
        | value when value = OP_END ->
            { Opcode = value; Mnemonic = "end"; Operand = None; Size = 1 }
        | value ->
            raise (InvalidOperationException(sprintf "Unknown WASM opcode 0x%02X at PC=%d" value pc))

type WasmExecutor() =
    member _.Execute(instruction: WasmInstruction, stack: ResizeArray<int>, locals: int array, pc: int) =
        let pop () =
            if stack.Count = 0 then
                raise (InvalidOperationException("Stack underflow"))
            let value = stack[stack.Count - 1]
            stack.RemoveAt(stack.Count - 1)
            value

        let before = stack |> Seq.toList

        match instruction.Mnemonic with
        | "i32.const" -> stack.Add(instruction.Operand.Value)
        | "i32.add" ->
            let right = pop ()
            let left = pop ()
            stack.Add(left + right)
        | "i32.sub" ->
            let right = pop ()
            let left = pop ()
            stack.Add(left - right)
        | "local.get" -> stack.Add(locals[instruction.Operand.Value])
        | "local.set" -> locals[instruction.Operand.Value] <- pop ()
        | "end" -> ()
        | other -> raise (InvalidOperationException(sprintf "Cannot execute %s" other))

        {
            Pc = pc
            Instruction = instruction
            StackBefore = before
            StackAfter = stack |> Seq.toList
            LocalsSnapshot = locals |> Array.toList
            Description = instruction.Mnemonic
            Halted = instruction.Mnemonic = "end"
        }

type WasmSimulator(numLocals: int) =
    let decoder = WasmDecoder()
    let executor = WasmExecutor()

    member val Stack = ResizeArray<int>() with get
    member val Locals = Array.zeroCreate<int> numLocals with get
    member val Pc = 0 with get, set
    member val Halted = false with get, set
    member val Bytecode = [||] with get, set

    member this.Load(bytecode: byte array) =
        this.Bytecode <- Array.copy bytecode
        this.Pc <- 0
        this.Halted <- false
        this.Stack.Clear()
        Array.Clear(this.Locals, 0, this.Locals.Length)

    member this.Step() =
        if this.Halted then
            raise (InvalidOperationException("WASM simulator has halted"))

        let instruction = decoder.Decode(this.Bytecode, this.Pc)
        let trace = executor.Execute(instruction, this.Stack, this.Locals, this.Pc)
        this.Pc <- this.Pc + instruction.Size
        this.Halted <- trace.Halted
        trace

    member this.Run(program: byte array, ?maxSteps: int) =
        let limit = defaultArg maxSteps 1000
        this.Load(program)
        let traces = ResizeArray<WasmStepTrace>()

        for _ in 1 .. limit do
            if not this.Halted then
                traces.Add(this.Step())

        traces |> Seq.toList

let encodeI32Const (value: int) = Array.append [| OP_I32_CONST |] (BitConverter.GetBytes(value))
let encodeI32Add () = [| OP_I32_ADD |]
let encodeI32Sub () = [| OP_I32_SUB |]
let encodeLocalGet index = [| OP_LOCAL_GET; byte index |]
let encodeLocalSet index = [| OP_LOCAL_SET; byte index |]
let encodeEnd () = [| OP_END |]
let assembleWasm (instructions: byte array list) = instructions |> List.collect Array.toList |> List.toArray
