using BrainfuckEngine = CodingAdventures.Brainfuck.Brainfuck;

namespace CodingAdventures.Brainfuck.Tests;

public sealed class BrainfuckTests
{
    [Fact]
    public void TranslateMapsCommandsAndIgnoresComments()
    {
        var program = BrainfuckEngine.Translate("hello + world -><.,");

        Assert.Equal(
            [
                BrainfuckOpcode.Increment,
                BrainfuckOpcode.Decrement,
                BrainfuckOpcode.Right,
                BrainfuckOpcode.Left,
                BrainfuckOpcode.Output,
                BrainfuckOpcode.Input,
                BrainfuckOpcode.Halt,
            ],
            program.Select(instruction => instruction.Opcode));
    }

    [Fact]
    public void TranslatePatchesLoopTargets()
    {
        var program = BrainfuckEngine.Translate("[>+<-]");

        Assert.Equal(BrainfuckOpcode.LoopStart, program[0].Opcode);
        Assert.Equal(6, program[0].Operand);
        Assert.Equal(BrainfuckOpcode.LoopEnd, program[5].Opcode);
        Assert.Equal(0, program[5].Operand);
    }

    [Fact]
    public void TranslatePatchesNestedLoops()
    {
        var program = BrainfuckEngine.Translate("[[]]");

        Assert.Equal(4, program[0].Operand);
        Assert.Equal(3, program[1].Operand);
        Assert.Equal(1, program[2].Operand);
        Assert.Equal(0, program[3].Operand);
    }

    [Fact]
    public void TranslateRejectsMismatchedBrackets()
    {
        Assert.Throws<BrainfuckTranslationException>(() => BrainfuckEngine.Translate("["));
        Assert.Throws<BrainfuckTranslationException>(() => BrainfuckEngine.Translate("]"));
        Assert.Throws<BrainfuckTranslationException>(() => BrainfuckEngine.Translate("[[]"));
    }

    [Fact]
    public void ExecuteHandlesEmptyProgramAndComments()
    {
        var empty = BrainfuckEngine.Execute("");
        var comments = BrainfuckEngine.Execute("this is all comments");

        Assert.Equal("", empty.Output);
        Assert.Equal(0, empty.Pointer);
        Assert.Equal(1, empty.Steps);
        Assert.Equal("", comments.Output);
        Assert.Equal(1, comments.Steps);
    }

    [Fact]
    public void ExecuteSupportsCellArithmeticAndWrapping()
    {
        Assert.Equal(1, BrainfuckEngine.Execute("+").Tape[0]);
        Assert.Equal(255, BrainfuckEngine.Execute("-").Tape[0]);
        Assert.Equal(0, BrainfuckEngine.Execute(new string('+', 256)).Tape[0]);
    }

    [Fact]
    public void ExecuteSupportsPointerMovementAndBoundsErrors()
    {
        Assert.Equal(3, BrainfuckEngine.Execute(">>>").Pointer);
        Assert.Throws<BrainfuckExecutionException>(() => BrainfuckEngine.Execute("<"));
        Assert.Throws<BrainfuckExecutionException>(() => BrainfuckEngine.Execute(new string('>', BrainfuckEngine.TapeSize)));
    }

    [Fact]
    public void ExecuteSupportsLoops()
    {
        var addition = BrainfuckEngine.Execute("++>+++++[<+>-]");
        var move = BrainfuckEngine.Execute("+++++[>+<-]");
        var skipped = BrainfuckEngine.Execute("[]+++");

        Assert.Equal(7, addition.Tape[0]);
        Assert.Equal(0, addition.Tape[1]);
        Assert.Equal(0, move.Tape[0]);
        Assert.Equal(5, move.Tape[1]);
        Assert.Equal(3, skipped.Tape[0]);
    }

    [Fact]
    public void ExecuteSupportsNestedLoops()
    {
        var result = BrainfuckEngine.Execute("++>+++<[>[>+>+<<-]>>[<<+>>-]<<<-]");

        Assert.Equal(6, result.Tape[2]);
        Assert.Equal(0, result.Tape[0]);
    }

    [Fact]
    public void ExecuteSupportsOutputAndInput()
    {
        var h = BrainfuckEngine.Execute("+++++++++[>++++++++<-]>.");
        var echo = BrainfuckEngine.Execute(",.,.,.", "ABC");
        var eof = BrainfuckEngine.Execute(",,", "A");

        Assert.Equal("H", h.Output);
        Assert.Equal("ABC", echo.Output);
        Assert.Equal(0, eof.Tape[0]);
    }

    [Fact]
    public void ExecuteRunsHelloWorld()
    {
        const string helloWorld =
            "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]" +
            ">>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.";

        Assert.Equal("Hello World!\n", BrainfuckEngine.Execute(helloWorld).Output);
    }

    [Fact]
    public void ExecuteAcceptsTranslatedProgram()
    {
        var program = BrainfuckEngine.Translate("+++.");
        var result = BrainfuckEngine.Execute(program);

        Assert.Equal(3, result.Output[0]);
        Assert.Equal(5, result.Steps);
    }

    [Fact]
    public void NullInputsAreRejected()
    {
        Assert.Throws<ArgumentNullException>(() => BrainfuckEngine.Translate(null!));
        Assert.Throws<ArgumentNullException>(() => BrainfuckEngine.Execute((string)null!));
        Assert.Throws<ArgumentNullException>(() => BrainfuckEngine.Execute("+", null!));
        Assert.Throws<ArgumentNullException>(() => BrainfuckEngine.Execute((IReadOnlyList<BrainfuckInstruction>)null!));
    }
}
