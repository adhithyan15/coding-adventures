namespace CodingAdventures.Assembler.Tests;

public sealed class AssemblerTests
{
    [Fact]
    public void ParsesRegistersAndImmediates()
    {
        Assert.True(Assembler.TryParseRegister("R0", out var r0));
        Assert.True(Assembler.TryParseRegister("sp", out var sp));
        Assert.True(Assembler.TryParseImmediate("#0xFF", out var hex));
        Assert.True(Assembler.TryParseImmediate("42", out var bare));

        Assert.Equal(0u, r0);
        Assert.Equal(13u, sp);
        Assert.Equal(255u, hex);
        Assert.Equal(42u, bare);
        Assert.False(Assembler.TryParseRegister("R16", out _));
        Assert.False(Assembler.TryParseRegister("X0", out _));
    }

    [Fact]
    public void ParsesDataProcessingInstructions()
    {
        var asm = new Assembler();
        var instructions = asm.Parse("MOV R0, #42\nADD R2, R0, R1\nCMP R0, R1");

        var mov = Assert.IsType<ArmInstruction.DataProcessing>(instructions[0]);
        var add = Assert.IsType<ArmInstruction.DataProcessing>(instructions[1]);
        var cmp = Assert.IsType<ArmInstruction.DataProcessing>(instructions[2]);

        Assert.Equal(ArmOpcode.Mov, mov.Opcode);
        Assert.Equal(0u, mov.Rd);
        Assert.IsType<Operand2.Immediate>(mov.Operand2);
        Assert.Equal(ArmOpcode.Add, add.Opcode);
        Assert.Equal(2u, add.Rd);
        Assert.Equal(0u, add.Rn);
        Assert.IsType<Operand2.Register>(add.Operand2);
        Assert.Equal(ArmOpcode.Cmp, cmp.Opcode);
        Assert.Null(cmp.Rd);
        Assert.True(cmp.SetFlags);
    }

    [Fact]
    public void ParsesMemoryInstructionsNopLabelsAndComments()
    {
        var asm = new Assembler();
        var instructions = asm.Parse("; full line comment\nstart:\nLDR R0, [R1] ; load\nSTR R2, [SP] // store\nNOP");

        Assert.IsType<ArmInstruction.Label>(instructions[0]);
        var load = Assert.IsType<ArmInstruction.Load>(instructions[1]);
        var store = Assert.IsType<ArmInstruction.Store>(instructions[2]);
        Assert.IsType<ArmInstruction.Nop>(instructions[3]);
        Assert.Equal(0, asm.Labels["start"]);
        Assert.Equal(0u, load.Rd);
        Assert.Equal(1u, load.Rn);
        Assert.Equal(2u, store.Rd);
        Assert.Equal(13u, store.Rn);
    }

    [Fact]
    public void EncodesDataProcessingInstructions()
    {
        var asm = new Assembler();
        var words = asm.Assemble("MOV R0, 42\nADD R2, R0, R1\nCMP R0, #1");

        Assert.Equal(3, words.Length);
        Assert.Equal(0xEu, words[0] >> 28);
        Assert.Equal(1u, (words[0] >> 25) & 1u);
        Assert.Equal(0xDu, (words[0] >> 21) & 0xFu);
        Assert.Equal(42u, words[0] & 0xFFFu);
        Assert.Equal(0x4u, (words[1] >> 21) & 0xFu);
        Assert.Equal(2u, (words[1] >> 12) & 0xFu);
        Assert.Equal(1u, words[1] & 0xFu);
        Assert.Equal(1u, (words[2] >> 20) & 1u);
    }

    [Fact]
    public void EncodesNopLoadStoreAndSkipsLabels()
    {
        var asm = new Assembler();
        var words = asm.Assemble("start:\nNOP\nLDR R0, [R1]\nSTR R0, [R1]");

        Assert.Equal([0xE1A00000u, 0xE5900000u | (1u << 16), 0xE5800000u | (1u << 16)], words);
        Assert.Equal(1u, (words[1] >> 20) & 1u);
        Assert.Equal(0u, (words[2] >> 20) & 1u);
    }

    [Fact]
    public void SupportsFlagSettingSuffixesAndLogicalOps()
    {
        var asm = new Assembler();
        var instructions = asm.Parse("ADDS R1, R2, #3\nANDS R0, R0, R1\nORR R3, R1, R2\nEOR R4, R4, R5\nRSB R6, R7, R8");

        var adds = Assert.IsType<ArmInstruction.DataProcessing>(instructions[0]);
        var ands = Assert.IsType<ArmInstruction.DataProcessing>(instructions[1]);
        var orr = Assert.IsType<ArmInstruction.DataProcessing>(instructions[2]);
        var eor = Assert.IsType<ArmInstruction.DataProcessing>(instructions[3]);
        var rsb = Assert.IsType<ArmInstruction.DataProcessing>(instructions[4]);

        Assert.True(adds.SetFlags);
        Assert.True(ands.SetFlags);
        Assert.Equal(ArmOpcode.Orr, orr.Opcode);
        Assert.Equal(ArmOpcode.Eor, eor.Opcode);
        Assert.Equal(ArmOpcode.Rsb, rsb.Opcode);
    }

    [Fact]
    public void RejectsInvalidSource()
    {
        var asm = new Assembler();

        Assert.Throws<AssemblerException>(() => asm.Parse("BLAH R0, R1"));
        Assert.Throws<AssemblerException>(() => asm.Parse("MOV X0, #1"));
        Assert.Throws<AssemblerException>(() => asm.Parse("ADD R0, R1"));
        Assert.Throws<AssemblerException>(() => asm.Parse("MOV R0, #nope"));
        Assert.Throws<ArgumentNullException>(() => asm.Parse(null!));
        Assert.Throws<ArgumentNullException>(() => asm.Encode(null!));
    }
}
