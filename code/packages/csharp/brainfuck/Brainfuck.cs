using System.Text;

namespace CodingAdventures.Brainfuck;

/// <summary>Brainfuck opcodes plus an internal halt marker.</summary>
public enum BrainfuckOpcode
{
    /// <summary>Move the data pointer one cell to the right.</summary>
    Right,
    /// <summary>Move the data pointer one cell to the left.</summary>
    Left,
    /// <summary>Increment the byte at the data pointer.</summary>
    Increment,
    /// <summary>Decrement the byte at the data pointer.</summary>
    Decrement,
    /// <summary>Output the byte at the data pointer as a character.</summary>
    Output,
    /// <summary>Read one input byte into the cell at the data pointer.</summary>
    Input,
    /// <summary>Jump forward when the current cell is zero.</summary>
    LoopStart,
    /// <summary>Jump backward when the current cell is nonzero.</summary>
    LoopEnd,
    /// <summary>Stop execution.</summary>
    Halt,
}

/// <summary>A translated Brainfuck instruction.</summary>
public readonly record struct BrainfuckInstruction(BrainfuckOpcode Opcode, int? Operand = null);

/// <summary>The result of executing a Brainfuck program.</summary>
public sealed class BrainfuckResult
{
    internal BrainfuckResult(string output, byte[] tape, int pointer, int steps)
    {
        Output = output;
        Tape = tape;
        Pointer = pointer;
        Steps = steps;
    }

    /// <summary>Text emitted by output commands.</summary>
    public string Output { get; }

    /// <summary>Final 30,000-cell tape snapshot.</summary>
    public byte[] Tape { get; }

    /// <summary>Final data pointer position.</summary>
    public int Pointer { get; }

    /// <summary>Number of translated instructions executed, including halt.</summary>
    public int Steps { get; }
}

/// <summary>Base exception for Brainfuck translation and execution errors.</summary>
public class BrainfuckException : Exception
{
    /// <summary>Create a Brainfuck exception with a message.</summary>
    public BrainfuckException(string message)
        : base(message)
    {
    }
}

/// <summary>Raised when Brainfuck source has mismatched brackets.</summary>
public sealed class BrainfuckTranslationException : BrainfuckException
{
    /// <summary>Create a Brainfuck translation exception with a message.</summary>
    public BrainfuckTranslationException(string message)
        : base(message)
    {
    }
}

/// <summary>Raised when execution leaves the fixed tape bounds or sees invalid bytecode.</summary>
public sealed class BrainfuckExecutionException : BrainfuckException
{
    /// <summary>Create a Brainfuck execution exception with a message.</summary>
    public BrainfuckExecutionException(string message)
        : base(message)
    {
    }
}

/// <summary>Standalone Brainfuck translator and interpreter.</summary>
public static class Brainfuck
{
    /// <summary>The classic Brainfuck tape size.</summary>
    public const int TapeSize = 30_000;

    /// <summary>Translate Brainfuck source to instructions with matched loop jump targets.</summary>
    public static BrainfuckInstruction[] Translate(string source)
    {
        ArgumentNullException.ThrowIfNull(source);

        var instructions = new List<BrainfuckInstruction>();
        var loopStarts = new Stack<int>();

        foreach (var ch in source)
        {
            switch (ch)
            {
                case '>':
                    instructions.Add(new BrainfuckInstruction(BrainfuckOpcode.Right));
                    break;
                case '<':
                    instructions.Add(new BrainfuckInstruction(BrainfuckOpcode.Left));
                    break;
                case '+':
                    instructions.Add(new BrainfuckInstruction(BrainfuckOpcode.Increment));
                    break;
                case '-':
                    instructions.Add(new BrainfuckInstruction(BrainfuckOpcode.Decrement));
                    break;
                case '.':
                    instructions.Add(new BrainfuckInstruction(BrainfuckOpcode.Output));
                    break;
                case ',':
                    instructions.Add(new BrainfuckInstruction(BrainfuckOpcode.Input));
                    break;
                case '[':
                    loopStarts.Push(instructions.Count);
                    instructions.Add(new BrainfuckInstruction(BrainfuckOpcode.LoopStart, 0));
                    break;
                case ']':
                    if (loopStarts.Count == 0)
                    {
                        throw new BrainfuckTranslationException("Unmatched ']' found without a matching '['.");
                    }

                    var start = loopStarts.Pop();
                    var end = instructions.Count;
                    instructions[start] = new BrainfuckInstruction(BrainfuckOpcode.LoopStart, end + 1);
                    instructions.Add(new BrainfuckInstruction(BrainfuckOpcode.LoopEnd, start));
                    break;
            }
        }

        if (loopStarts.Count > 0)
        {
            throw new BrainfuckTranslationException($"Unmatched '[' found: {loopStarts.Count} unclosed bracket(s).");
        }

        instructions.Add(new BrainfuckInstruction(BrainfuckOpcode.Halt));
        return instructions.ToArray();
    }

    /// <summary>Execute Brainfuck source with optional UTF-8 input text.</summary>
    public static BrainfuckResult Execute(string source, string input = "")
    {
        ArgumentNullException.ThrowIfNull(source);
        ArgumentNullException.ThrowIfNull(input);

        var program = Translate(source);
        return Execute(program, Encoding.UTF8.GetBytes(input));
    }

    /// <summary>Execute translated Brainfuck instructions with optional raw input bytes.</summary>
    public static BrainfuckResult Execute(IReadOnlyList<BrainfuckInstruction> program, byte[]? input = null)
    {
        ArgumentNullException.ThrowIfNull(program);

        var tape = new byte[TapeSize];
        var inputBytes = input ?? [];
        var inputPosition = 0;
        var pointer = 0;
        var pc = 0;
        var steps = 0;
        var output = new StringBuilder();

        while (pc >= 0 && pc < program.Count)
        {
            var instruction = program[pc];
            steps++;

            switch (instruction.Opcode)
            {
                case BrainfuckOpcode.Right:
                    pointer++;
                    if (pointer >= TapeSize)
                    {
                        throw new BrainfuckExecutionException($"Data pointer moved past end of tape at position {pointer}.");
                    }

                    pc++;
                    break;

                case BrainfuckOpcode.Left:
                    pointer--;
                    if (pointer < 0)
                    {
                        throw new BrainfuckExecutionException("Data pointer moved before start of tape at position -1.");
                    }

                    pc++;
                    break;

                case BrainfuckOpcode.Increment:
                    tape[pointer] = unchecked((byte)(tape[pointer] + 1));
                    pc++;
                    break;

                case BrainfuckOpcode.Decrement:
                    tape[pointer] = unchecked((byte)(tape[pointer] - 1));
                    pc++;
                    break;

                case BrainfuckOpcode.Output:
                    output.Append((char)tape[pointer]);
                    pc++;
                    break;

                case BrainfuckOpcode.Input:
                    tape[pointer] = inputPosition < inputBytes.Length ? inputBytes[inputPosition++] : (byte)0;
                    pc++;
                    break;

                case BrainfuckOpcode.LoopStart:
                    pc = tape[pointer] == 0 ? RequireOperand(instruction) : pc + 1;
                    break;

                case BrainfuckOpcode.LoopEnd:
                    pc = tape[pointer] != 0 ? RequireOperand(instruction) : pc + 1;
                    break;

                case BrainfuckOpcode.Halt:
                    return new BrainfuckResult(output.ToString(), tape.ToArray(), pointer, steps);

                default:
                    throw new BrainfuckExecutionException($"Unknown Brainfuck opcode: {instruction.Opcode}.");
            }
        }

        return new BrainfuckResult(output.ToString(), tape.ToArray(), pointer, steps);
    }

    private static int RequireOperand(BrainfuckInstruction instruction) =>
        instruction.Operand ?? throw new BrainfuckExecutionException($"{instruction.Opcode} requires a jump target.");
}
