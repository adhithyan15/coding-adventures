namespace CodingAdventures.Assembler.FSharp

open System
open System.Collections.Generic

/// ARM data-processing opcode values.
type ArmOpcode =
    | And = 0x0
    | Eor = 0x1
    | Sub = 0x2
    | Rsb = 0x3
    | Add = 0x4
    | Cmp = 0xA
    | Orr = 0xC
    | Mov = 0xD

/// Second operand for ARM data-processing instructions.
type Operand2 =
    | Register of uint32
    | Immediate of uint32

/// Parsed ARM assembly instruction.
type ArmInstruction =
    | DataProcessing of opcode: ArmOpcode * rd: uint32 option * rn: uint32 option * operand2: Operand2 * setFlags: bool
    | Load of rd: uint32 * rn: uint32
    | Store of rd: uint32 * rn: uint32
    | Nop
    | Label of string

/// Raised when assembly source cannot be parsed or encoded.
exception AssemblerException of string

[<RequireQualifiedAccess>]
module ArmParsing =
    /// Try to parse an ARM register name such as R0, R15, SP, LR, or PC.
    let tryParseRegister (text: string) =
        if isNull text then
            None
        else
            match text.Trim().ToUpperInvariant() with
            | "SP" -> Some 13u
            | "LR" -> Some 14u
            | "PC" -> Some 15u
            | value when value.Length >= 2 && value[0] = 'R' ->
                match UInt32.TryParse(value[1..]) with
                | true, parsed when parsed <= 15u -> Some parsed
                | _ -> None
            | _ -> None

    /// Try to parse a decimal or hexadecimal immediate, with or without #.
    let tryParseImmediate (text: string) =
        if isNull text then
            None
        else
            let mutable trimmed = text.Trim()
            if trimmed.StartsWith("#", StringComparison.Ordinal) then
                trimmed <- trimmed[1..].Trim()

            if trimmed.StartsWith("0x", StringComparison.OrdinalIgnoreCase) then
                match UInt32.TryParse(trimmed[2..], Globalization.NumberStyles.HexNumber, null) with
                | true, value -> Some value
                | _ -> None
            else
                match UInt32.TryParse(trimmed) with
                | true, value -> Some value
                | _ -> None

type Assembler() =
    let labels = Dictionary<string, int>(StringComparer.Ordinal)

    /// Label-to-instruction-address table built during parsing.
    member _.Labels = labels

    /// Parse assembly source into structured instructions.
    member this.Parse(source: string) =
        if isNull source then
            nullArg "source"

        labels.Clear()
        let instructions = ResizeArray<ArmInstruction>()
        let mutable address = 0

        let normalizedSource = source.Replace("\r\n", "\n", StringComparison.Ordinal)
        for (rawLine: string) in normalizedSource.Split([| '\n' |], StringSplitOptions.None) do
            let line = this.StripComment(rawLine).Trim()
            if line.Length > 0 then
                if line.EndsWith(":", StringComparison.Ordinal) then
                    let label = line.Substring(0, line.Length - 1).Trim()
                    labels[label] <- address
                    instructions.Add(Label label)
                else
                    let instruction = this.ParseInstruction(line)
                    match instruction with
                    | Label _ -> ()
                    | _ -> address <- address + 1
                    instructions.Add instruction

        instructions.ToArray()

    /// Encode structured instructions into ARM 32-bit words.
    member this.Encode(instructions: ArmInstruction seq) =
        if isNull (box instructions) then
            nullArg "instructions"

        let words = ResizeArray<uint32>()
        for instruction in instructions do
            match instruction with
            | Label _ -> ()
            | Nop -> words.Add 0xE1A0_0000u
            | DataProcessing(opcode, rd, rn, operand2, setFlags) ->
                words.Add(this.EncodeDataProcessing(opcode, rd, rn, operand2, setFlags))
            | Load(rd, rn) ->
                words.Add(0xE590_0000u ||| (rn <<< 16) ||| (rd <<< 12))
            | Store(rd, rn) ->
                words.Add(0xE580_0000u ||| (rn <<< 16) ||| (rd <<< 12))

        words.ToArray()

    /// Parse and encode assembly source in one call.
    member this.Assemble(source: string) =
        this.Parse(source) |> this.Encode

    member private _.StripComment(line: string) : string =
        let semicolon = line.IndexOf(';')
        let slashSlash = line.IndexOf("//", StringComparison.Ordinal)
        let mutable cut = line.Length
        if semicolon >= 0 then
            cut <- min cut semicolon
        if slashSlash >= 0 then
            cut <- min cut slashSlash
        if cut = 0 then "" else line[.. cut - 1]

    member private this.ParseInstruction(line: string) =
        let firstSpace = line.IndexOfAny([| ' '; '\t' |])
        let mnemonic =
            if firstSpace < 0 then
                line.ToUpperInvariant()
            else
                line[.. firstSpace - 1].Trim().ToUpperInvariant()
        let operandsText =
            if firstSpace < 0 then
                ""
            else
                line[firstSpace + 1 ..].Trim()

        match mnemonic with
        | "NOP" -> Nop
        | "MOV" | "MOVS" -> this.ParseMov(mnemonic, operandsText)
        | "CMP" -> this.ParseCmp operandsText
        | "LDR" -> this.ParseLoadStore(mnemonic, operandsText, true)
        | "STR" -> this.ParseLoadStore(mnemonic, operandsText, false)
        | "ADD" | "ADDS" | "SUB" | "SUBS" | "AND" | "ANDS" | "ORR" | "ORRS" | "EOR" | "EORS" | "RSB" | "RSBS" ->
            this.ParseThreeOperandDataProcessing(mnemonic, operandsText)
        | _ -> raise (AssemblerException (sprintf "Unknown mnemonic: %s." mnemonic))

    member private _.SplitOperands(text: string) =
        if String.IsNullOrWhiteSpace text then
            [||]
        else
            text.Split([| ',' |], StringSplitOptions.TrimEntries ||| StringSplitOptions.RemoveEmptyEntries)

    member private this.ParseMov(mnemonic: string, operandsText: string) =
        let operands = this.SplitOperands operandsText
        this.RequireOperandCount(mnemonic, operands, 2)
        DataProcessing(ArmOpcode.Mov, Some(this.ParseRegisterOrThrow operands[0]), None, this.ParseOperand2 operands[1], mnemonic = "MOVS")

    member private this.ParseCmp(operandsText: string) =
        let operands = this.SplitOperands operandsText
        this.RequireOperandCount("CMP", operands, 2)
        DataProcessing(ArmOpcode.Cmp, None, Some(this.ParseRegisterOrThrow operands[0]), this.ParseOperand2 operands[1], true)

    member private this.ParseThreeOperandDataProcessing(mnemonic: string, operandsText: string) =
        let baseMnemonic = mnemonic.TrimEnd([| 'S' |])
        let operands = this.SplitOperands operandsText
        this.RequireOperandCount(mnemonic, operands, 3)
        DataProcessing(
            this.MnemonicToOpcode baseMnemonic,
            Some(this.ParseRegisterOrThrow operands[0]),
            Some(this.ParseRegisterOrThrow operands[1]),
            this.ParseOperand2 operands[2],
            mnemonic.Length > baseMnemonic.Length)

    member private this.ParseLoadStore(mnemonic: string, operandsText: string, isLoad: bool) =
        let operands = this.SplitOperands operandsText
        this.RequireOperandCount(mnemonic, operands, 2)
        let rd = this.ParseRegisterOrThrow operands[0]
        let baseRegister = operands[1].Trim().TrimStart('[').TrimEnd(']').Trim()
        let rn = this.ParseRegisterOrThrow baseRegister
        if isLoad then Load(rd, rn) else Store(rd, rn)

    member private _.RequireOperandCount(mnemonic: string, operands: string array, expected: int) =
        if operands.Length <> expected then
            raise (AssemblerException (sprintf "%s: expected %d operands, got %d." mnemonic expected operands.Length))

    member private _.ParseRegisterOrThrow(text: string) =
        match ArmParsing.tryParseRegister text with
        | Some register -> register
        | None -> raise (AssemblerException (sprintf "Invalid register: %s." text))

    member private this.ParseOperand2(text: string) =
        if text.Trim().StartsWith("#", StringComparison.Ordinal) then
            match ArmParsing.tryParseImmediate text with
            | Some value -> Immediate value
            | None -> raise (AssemblerException (sprintf "Invalid immediate: %s." text))
        else
            match ArmParsing.tryParseRegister text with
            | Some register -> Register register
            | None ->
                match ArmParsing.tryParseImmediate text with
                | Some value -> Immediate value
                | None -> raise (AssemblerException (sprintf "Cannot parse operand: %s." text))

    member private _.MnemonicToOpcode(mnemonic: string) =
        match mnemonic with
        | "AND" -> ArmOpcode.And
        | "EOR" -> ArmOpcode.Eor
        | "SUB" -> ArmOpcode.Sub
        | "RSB" -> ArmOpcode.Rsb
        | "ADD" -> ArmOpcode.Add
        | "CMP" -> ArmOpcode.Cmp
        | "ORR" -> ArmOpcode.Orr
        | "MOV" -> ArmOpcode.Mov
        | _ -> raise (AssemblerException (sprintf "Unknown mnemonic: %s." mnemonic))

    member private _.EncodeDataProcessing(opcode: ArmOpcode, rd: uint32 option, rn: uint32 option, operand2: Operand2, setFlags: bool) =
        let cond = 0xEu
        let rdValue = defaultArg rd 0u
        let rnValue = defaultArg rn 0u
        let setFlagsBit = if setFlags then 1u else 0u
        let opcodeValue = uint32 opcode

        let immediateBit, operand2Value =
            match operand2 with
            | Immediate value -> 1u, value &&& 0xFFFu
            | Register register -> 0u, register &&& 0xFu

        (cond <<< 28)
        ||| (immediateBit <<< 25)
        ||| (opcodeValue <<< 21)
        ||| (setFlagsBit <<< 20)
        ||| (rnValue <<< 16)
        ||| (rdValue <<< 12)
        ||| operand2Value
