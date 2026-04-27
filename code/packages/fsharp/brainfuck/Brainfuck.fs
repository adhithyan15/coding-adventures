namespace CodingAdventures.Brainfuck.FSharp

open System
open System.Collections.Generic
open System.Text

/// Brainfuck opcodes plus an internal halt marker.
type BrainfuckOpcode =
    | Right
    | Left
    | Increment
    | Decrement
    | Output
    | Input
    | LoopStart
    | LoopEnd
    | Halt

/// A translated Brainfuck instruction.
type BrainfuckInstruction =
    {
        Opcode: BrainfuckOpcode
        Operand: int option
    }

/// The result of executing a Brainfuck program.
type BrainfuckResult =
    {
        /// Text emitted by output commands.
        Output: string
        /// Final 30,000-cell tape snapshot.
        Tape: byte array
        /// Final data pointer position.
        Pointer: int
        /// Number of translated instructions executed, including halt.
        Steps: int
    }

/// Raised when Brainfuck source has mismatched brackets.
exception BrainfuckTranslationException of string

/// Raised when execution leaves the fixed tape bounds or sees invalid bytecode.
exception BrainfuckExecutionException of string

[<RequireQualifiedAccess>]
module Brainfuck =
    /// The classic Brainfuck tape size.
    [<Literal>]
    let TapeSize = 30_000

    let private instruction opcode operand =
        { Opcode = opcode; Operand = operand }

    let private commandToInstruction = function
        | '>' -> Some (instruction Right None)
        | '<' -> Some (instruction Left None)
        | '+' -> Some (instruction Increment None)
        | '-' -> Some (instruction Decrement None)
        | '.' -> Some (instruction Output None)
        | ',' -> Some (instruction Input None)
        | _ -> None

    /// Translate Brainfuck source to instructions with matched loop jump targets.
    let translate (source: string) =
        if isNull source then
            nullArg "source"

        let instructions = ResizeArray<BrainfuckInstruction>()
        let loopStarts = Stack<int>()

        for ch in source do
            match ch with
            | '[' ->
                loopStarts.Push(instructions.Count)
                instructions.Add(instruction LoopStart (Some 0))
            | ']' ->
                if loopStarts.Count = 0 then
                    raise (BrainfuckTranslationException "Unmatched ']' found without a matching '['.")

                let start = loopStarts.Pop()
                let ending = instructions.Count
                instructions[start] <- instruction LoopStart (Some (ending + 1))
                instructions.Add(instruction LoopEnd (Some start))
            | _ ->
                match commandToInstruction ch with
                | Some inst -> instructions.Add inst
                | None -> ()

        if loopStarts.Count > 0 then
            raise (BrainfuckTranslationException (sprintf "Unmatched '[' found: %d unclosed bracket(s)." loopStarts.Count))

        instructions.Add(instruction Halt None)
        instructions.ToArray()

    let private requireOperand instruction =
        match instruction.Operand with
        | Some target -> target
        | None -> raise (BrainfuckExecutionException (sprintf "%A requires a jump target." instruction.Opcode))

    /// Execute translated Brainfuck instructions with optional raw input bytes.
    let executeProgram (program: BrainfuckInstruction array) (input: byte array) =
        if isNull program then
            nullArg "program"

        let inputBytes =
            if isNull input then
                [||]
            else
                Array.copy input

        let tape = Array.zeroCreate<byte> TapeSize
        let output = StringBuilder()
        let mutable inputPosition = 0
        let mutable pointer = 0
        let mutable pc = 0
        let mutable steps = 0
        let mutable halted = false

        while not halted && pc >= 0 && pc < program.Length do
            let current = program[pc]
            steps <- steps + 1

            match current.Opcode with
            | Right ->
                pointer <- pointer + 1
                if pointer >= TapeSize then
                    raise (BrainfuckExecutionException (sprintf "Data pointer moved past end of tape at position %d." pointer))
                pc <- pc + 1

            | Left ->
                pointer <- pointer - 1
                if pointer < 0 then
                    raise (BrainfuckExecutionException "Data pointer moved before start of tape at position -1.")
                pc <- pc + 1

            | Increment ->
                tape[pointer] <- byte ((int tape[pointer] + 1) &&& 0xff)
                pc <- pc + 1

            | Decrement ->
                tape[pointer] <- byte ((int tape[pointer] - 1) &&& 0xff)
                pc <- pc + 1

            | Output ->
                output.Append(char tape[pointer]) |> ignore
                pc <- pc + 1

            | Input ->
                tape[pointer] <-
                    if inputPosition < inputBytes.Length then
                        let value = inputBytes[inputPosition]
                        inputPosition <- inputPosition + 1
                        value
                    else
                        0uy
                pc <- pc + 1

            | LoopStart ->
                pc <-
                    if tape[pointer] = 0uy then
                        requireOperand current
                    else
                        pc + 1

            | LoopEnd ->
                pc <-
                    if tape[pointer] <> 0uy then
                        requireOperand current
                    else
                        pc + 1

            | Halt ->
                halted <- true

        {
            Output = output.ToString()
            Tape = Array.copy tape
            Pointer = pointer
            Steps = steps
        }

    /// Execute Brainfuck source with explicit UTF-8 input text.
    let executeWithInput (source: string) (input: string) =
        if isNull source then
            nullArg "source"

        if isNull input then
            nullArg "input"

        executeProgram (translate source) (Encoding.UTF8.GetBytes input)

    /// Execute Brainfuck source with empty input.
    let execute (source: string) =
        executeWithInput source ""
