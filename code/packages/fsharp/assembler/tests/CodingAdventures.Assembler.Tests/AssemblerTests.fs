namespace CodingAdventures.Assembler.Tests

open System
open Xunit
open CodingAdventures.Assembler.FSharp

module AssemblerTests =
    [<Fact>]
    let ``parses registers and immediates`` () =
        Assert.Equal(Some 0u, ArmParsing.tryParseRegister "R0")
        Assert.Equal(Some 13u, ArmParsing.tryParseRegister "sp")
        Assert.Equal(Some 255u, ArmParsing.tryParseImmediate "#0xFF")
        Assert.Equal(Some 42u, ArmParsing.tryParseImmediate "42")
        Assert.Equal(None, ArmParsing.tryParseRegister "R16")
        Assert.Equal(None, ArmParsing.tryParseRegister "X0")

    [<Fact>]
    let ``parses data processing instructions`` () =
        let asm = Assembler()
        let instructions = asm.Parse("MOV R0, #42\nADD R2, R0, R1\nCMP R0, R1")

        match instructions[0] with
        | DataProcessing(opcode, Some rd, None, Immediate value, false) ->
            Assert.Equal(ArmOpcode.Mov, opcode)
            Assert.Equal(0u, rd)
            Assert.Equal(42u, value)
        | other -> Assert.Fail(sprintf "Expected MOV, got %A" other)

        match instructions[1] with
        | DataProcessing(opcode, Some rd, Some rn, Register rm, false) ->
            Assert.Equal(ArmOpcode.Add, opcode)
            Assert.Equal(2u, rd)
            Assert.Equal(0u, rn)
            Assert.Equal(1u, rm)
        | other -> Assert.Fail(sprintf "Expected ADD, got %A" other)

        match instructions[2] with
        | DataProcessing(opcode, None, Some rn, Register rm, true) ->
            Assert.Equal(ArmOpcode.Cmp, opcode)
            Assert.Equal(0u, rn)
            Assert.Equal(1u, rm)
        | other -> Assert.Fail(sprintf "Expected CMP, got %A" other)

    [<Fact>]
    let ``parses memory instructions nop labels and comments`` () =
        let asm = Assembler()
        let instructions = asm.Parse("; full line comment\nstart:\nLDR R0, [R1] ; load\nSTR R2, [SP] // store\nNOP")

        Assert.Equal(Label "start", instructions[0])
        Assert.Equal(Load(0u, 1u), instructions[1])
        Assert.Equal(Store(2u, 13u), instructions[2])
        Assert.Equal(Nop, instructions[3])
        Assert.Equal(0, asm.Labels["start"])

    [<Fact>]
    let ``encodes data processing instructions`` () =
        let asm = Assembler()
        let words = asm.Assemble("MOV R0, 42\nADD R2, R0, R1\nCMP R0, #1")

        Assert.Equal(3, words.Length)
        Assert.Equal(0xEu, words[0] >>> 28)
        Assert.Equal(1u, (words[0] >>> 25) &&& 1u)
        Assert.Equal(0xDu, (words[0] >>> 21) &&& 0xFu)
        Assert.Equal(42u, words[0] &&& 0xFFFu)
        Assert.Equal(0x4u, (words[1] >>> 21) &&& 0xFu)
        Assert.Equal(2u, (words[1] >>> 12) &&& 0xFu)
        Assert.Equal(1u, words[1] &&& 0xFu)
        Assert.Equal(1u, (words[2] >>> 20) &&& 1u)

    [<Fact>]
    let ``encodes nop load store and skips labels`` () =
        let asm = Assembler()
        let words = asm.Assemble("start:\nNOP\nLDR R0, [R1]\nSTR R0, [R1]")

        Assert.Equal<uint32 array>([| 0xE1A00000u; 0xE5900000u ||| (1u <<< 16); 0xE5800000u ||| (1u <<< 16) |], words)
        Assert.Equal(1u, (words[1] >>> 20) &&& 1u)
        Assert.Equal(0u, (words[2] >>> 20) &&& 1u)

    [<Fact>]
    let ``supports flag suffixes and logical ops`` () =
        let asm = Assembler()
        let instructions = asm.Parse("ADDS R1, R2, #3\nANDS R0, R0, R1\nORR R3, R1, R2\nEOR R4, R4, R5\nRSB R6, R7, R8")

        match instructions[0] with
        | DataProcessing(_, _, _, _, true) -> ()
        | other -> Assert.Fail(sprintf "Expected flag-setting instruction, got %A" other)

        match instructions[1] with
        | DataProcessing(_, _, _, _, true) -> ()
        | other -> Assert.Fail(sprintf "Expected flag-setting instruction, got %A" other)

        match instructions[2], instructions[3], instructions[4] with
        | DataProcessing(ArmOpcode.Orr, _, _, _, _), DataProcessing(ArmOpcode.Eor, _, _, _, _), DataProcessing(ArmOpcode.Rsb, _, _, _, _) -> ()
        | other -> Assert.Fail(sprintf "Unexpected opcodes: %A" other)

    [<Fact>]
    let ``rejects invalid source`` () =
        let asm = Assembler()

        Assert.Throws<AssemblerException>(fun () -> asm.Parse("BLAH R0, R1") |> ignore) |> ignore
        Assert.Throws<AssemblerException>(fun () -> asm.Parse("MOV X0, #1") |> ignore) |> ignore
        Assert.Throws<AssemblerException>(fun () -> asm.Parse("ADD R0, R1") |> ignore) |> ignore
        Assert.Throws<AssemblerException>(fun () -> asm.Parse("MOV R0, #nope") |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> asm.Parse(null) |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> asm.Encode(null) |> ignore) |> ignore
