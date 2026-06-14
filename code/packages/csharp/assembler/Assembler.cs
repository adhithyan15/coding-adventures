namespace CodingAdventures.Assembler;

/// <summary>ARM data-processing opcode values.</summary>
public enum ArmOpcode : uint
{
    /// <summary>Bitwise AND.</summary>
    And = 0x0,
    /// <summary>Bitwise exclusive OR.</summary>
    Eor = 0x1,
    /// <summary>Subtract.</summary>
    Sub = 0x2,
    /// <summary>Reverse subtract.</summary>
    Rsb = 0x3,
    /// <summary>Add.</summary>
    Add = 0x4,
    /// <summary>Compare and set flags.</summary>
    Cmp = 0xA,
    /// <summary>Bitwise OR.</summary>
    Orr = 0xC,
    /// <summary>Move.</summary>
    Mov = 0xD,
}

/// <summary>Second operand for ARM data-processing instructions.</summary>
public abstract record Operand2
{
    /// <summary>A register operand.</summary>
    public sealed record Register(uint Number) : Operand2;

    /// <summary>An immediate operand.</summary>
    public sealed record Immediate(uint Value) : Operand2;
}

/// <summary>Parsed ARM assembly instruction.</summary>
public abstract record ArmInstruction
{
    /// <summary>A data-processing instruction such as MOV, ADD, SUB, or CMP.</summary>
    public sealed record DataProcessing(ArmOpcode Opcode, uint? Rd, uint? Rn, Operand2 Operand2, bool SetFlags) : ArmInstruction;

    /// <summary>Load a word from [Rn] into Rd.</summary>
    public sealed record Load(uint Rd, uint Rn) : ArmInstruction;

    /// <summary>Store Rd into [Rn].</summary>
    public sealed record Store(uint Rd, uint Rn) : ArmInstruction;

    /// <summary>No operation.</summary>
    public sealed record Nop : ArmInstruction;

    /// <summary>A symbolic label. Labels do not emit binary words.</summary>
    public sealed record Label(string Name) : ArmInstruction;
}

/// <summary>Raised when assembly source cannot be parsed or encoded.</summary>
public sealed class AssemblerException : Exception
{
    /// <summary>Create an assembler exception with a message.</summary>
    public AssemblerException(string message)
        : base(message)
    {
    }
}

/// <summary>ARM assembly parser and 32-bit instruction encoder.</summary>
public sealed class Assembler
{
    /// <summary>Label-to-instruction-address table built during parsing.</summary>
    public Dictionary<string, int> Labels { get; } = new(StringComparer.Ordinal);

    /// <summary>Parse assembly source into structured instructions.</summary>
    public ArmInstruction[] Parse(string source)
    {
        ArgumentNullException.ThrowIfNull(source);

        Labels.Clear();
        var instructions = new List<ArmInstruction>();
        var address = 0;

        foreach (var rawLine in source.Replace("\r\n", "\n", StringComparison.Ordinal).Split('\n'))
        {
            var line = StripComment(rawLine).Trim();
            if (line.Length == 0)
            {
                continue;
            }

            if (line.EndsWith(':'))
            {
                var label = line[..^1].Trim();
                Labels[label] = address;
                instructions.Add(new ArmInstruction.Label(label));
                continue;
            }

            var instruction = ParseInstruction(line);
            if (instruction is not ArmInstruction.Label)
            {
                address++;
            }

            instructions.Add(instruction);
        }

        return instructions.ToArray();
    }

    /// <summary>Encode structured instructions into ARM 32-bit words.</summary>
    public uint[] Encode(IEnumerable<ArmInstruction> instructions)
    {
        ArgumentNullException.ThrowIfNull(instructions);

        var words = new List<uint>();
        foreach (var instruction in instructions)
        {
            switch (instruction)
            {
                case ArmInstruction.Label:
                    break;

                case ArmInstruction.Nop:
                    words.Add(0xE1A0_0000u);
                    break;

                case ArmInstruction.DataProcessing data:
                    words.Add(EncodeDataProcessing(data));
                    break;

                case ArmInstruction.Load load:
                    words.Add(0xE590_0000u | (load.Rn << 16) | (load.Rd << 12));
                    break;

                case ArmInstruction.Store store:
                    words.Add(0xE580_0000u | (store.Rn << 16) | (store.Rd << 12));
                    break;

                default:
                    throw new AssemblerException($"Unsupported instruction: {instruction.GetType().Name}.");
            }
        }

        return words.ToArray();
    }

    /// <summary>Parse and encode assembly source in one call.</summary>
    public uint[] Assemble(string source) => Encode(Parse(source));

    /// <summary>Try to parse an ARM register name such as R0, R15, SP, LR, or PC.</summary>
    public static bool TryParseRegister(string text, out uint register)
    {
        register = 0;
        if (text is null)
        {
            return false;
        }

        var value = text.Trim().ToUpperInvariant();
        switch (value)
        {
            case "SP":
                register = 13;
                return true;
            case "LR":
                register = 14;
                return true;
            case "PC":
                register = 15;
                return true;
        }

        if (value.Length >= 2 && value[0] == 'R' && uint.TryParse(value[1..], out var parsed) && parsed <= 15)
        {
            register = parsed;
            return true;
        }

        return false;
    }

    /// <summary>Try to parse a decimal or hexadecimal immediate, with or without #.</summary>
    public static bool TryParseImmediate(string text, out uint value)
    {
        value = 0;
        if (text is null)
        {
            return false;
        }

        var trimmed = text.Trim();
        if (trimmed.StartsWith('#'))
        {
            trimmed = trimmed[1..].Trim();
        }

        if (trimmed.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
        {
            return uint.TryParse(trimmed[2..], System.Globalization.NumberStyles.HexNumber, null, out value);
        }

        return uint.TryParse(trimmed, out value);
    }

    private ArmInstruction ParseInstruction(string line)
    {
        var firstSpace = line.IndexOfAny([' ', '\t']);
        var mnemonic = firstSpace < 0 ? line.ToUpperInvariant() : line[..firstSpace].Trim().ToUpperInvariant();
        var operandsText = firstSpace < 0 ? string.Empty : line[(firstSpace + 1)..].Trim();

        return mnemonic switch
        {
            "NOP" => new ArmInstruction.Nop(),
            "MOV" or "MOVS" => ParseMov(mnemonic, operandsText),
            "CMP" => ParseCmp(operandsText),
            "LDR" => ParseLoadStore(mnemonic, operandsText, isLoad: true),
            "STR" => ParseLoadStore(mnemonic, operandsText, isLoad: false),
            "ADD" or "ADDS" or "SUB" or "SUBS" or "AND" or "ANDS" or "ORR" or "ORRS" or "EOR" or "EORS" or "RSB" or "RSBS" =>
                ParseThreeOperandDataProcessing(mnemonic, operandsText),
            _ => throw new AssemblerException($"Unknown mnemonic: {mnemonic}."),
        };
    }

    private static ArmInstruction ParseMov(string mnemonic, string operandsText)
    {
        var operands = SplitOperands(operandsText);
        RequireOperandCount(mnemonic, operands, 2);
        return new ArmInstruction.DataProcessing(
            ArmOpcode.Mov,
            ParseRegisterOrThrow(operands[0]),
            null,
            ParseOperand2(operands[1]),
            mnemonic == "MOVS");
    }

    private static ArmInstruction ParseCmp(string operandsText)
    {
        var operands = SplitOperands(operandsText);
        RequireOperandCount("CMP", operands, 2);
        return new ArmInstruction.DataProcessing(
            ArmOpcode.Cmp,
            null,
            ParseRegisterOrThrow(operands[0]),
            ParseOperand2(operands[1]),
            SetFlags: true);
    }

    private static ArmInstruction ParseThreeOperandDataProcessing(string mnemonic, string operandsText)
    {
        var baseMnemonic = mnemonic.TrimEnd('S');
        var operands = SplitOperands(operandsText);
        RequireOperandCount(mnemonic, operands, 3);

        return new ArmInstruction.DataProcessing(
            MnemonicToOpcode(baseMnemonic),
            ParseRegisterOrThrow(operands[0]),
            ParseRegisterOrThrow(operands[1]),
            ParseOperand2(operands[2]),
            mnemonic.Length > baseMnemonic.Length);
    }

    private static ArmInstruction ParseLoadStore(string mnemonic, string operandsText, bool isLoad)
    {
        var operands = SplitOperands(operandsText);
        RequireOperandCount(mnemonic, operands, 2);

        var rd = ParseRegisterOrThrow(operands[0]);
        var baseRegister = operands[1].Trim().TrimStart('[').TrimEnd(']').Trim();
        var rn = ParseRegisterOrThrow(baseRegister);

        return isLoad ? new ArmInstruction.Load(rd, rn) : new ArmInstruction.Store(rd, rn);
    }

    private static Operand2 ParseOperand2(string text)
    {
        if (text.Trim().StartsWith('#'))
        {
            return TryParseImmediate(text, out var immediate)
                ? new Operand2.Immediate(immediate)
                : throw new AssemblerException($"Invalid immediate: {text}.");
        }

        if (TryParseRegister(text, out var register))
        {
            return new Operand2.Register(register);
        }

        return TryParseImmediate(text, out var bareImmediate)
            ? new Operand2.Immediate(bareImmediate)
            : throw new AssemblerException($"Cannot parse operand: {text}.");
    }

    private static uint ParseRegisterOrThrow(string text) =>
        TryParseRegister(text, out var register) ? register : throw new AssemblerException($"Invalid register: {text}.");

    private static ArmOpcode MnemonicToOpcode(string mnemonic) =>
        mnemonic switch
        {
            "AND" => ArmOpcode.And,
            "EOR" => ArmOpcode.Eor,
            "SUB" => ArmOpcode.Sub,
            "RSB" => ArmOpcode.Rsb,
            "ADD" => ArmOpcode.Add,
            "CMP" => ArmOpcode.Cmp,
            "ORR" => ArmOpcode.Orr,
            "MOV" => ArmOpcode.Mov,
            _ => throw new AssemblerException($"Unknown mnemonic: {mnemonic}."),
        };

    private static uint EncodeDataProcessing(ArmInstruction.DataProcessing instruction)
    {
        const uint cond = 0xEu;
        var rd = instruction.Rd ?? 0;
        var rn = instruction.Rn ?? 0;
        var setFlags = instruction.SetFlags ? 1u : 0u;
        var opcode = (uint)instruction.Opcode;

        var (immediateBit, operand2) = instruction.Operand2 switch
        {
            Operand2.Immediate imm => (1u, imm.Value & 0xFFFu),
            Operand2.Register reg => (0u, reg.Number & 0xFu),
            _ => throw new AssemblerException("Unsupported operand2."),
        };

        return (cond << 28) | (immediateBit << 25) | (opcode << 21) | (setFlags << 20) | (rn << 16) | (rd << 12) | operand2;
    }

    private static string[] SplitOperands(string text) =>
        string.IsNullOrWhiteSpace(text)
            ? []
            : text.Split(',', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries);

    private static void RequireOperandCount(string mnemonic, string[] operands, int expected)
    {
        if (operands.Length != expected)
        {
            throw new AssemblerException($"{mnemonic}: expected {expected} operands, got {operands.Length}.");
        }
    }

    private static string StripComment(string line)
    {
        var semicolon = line.IndexOf(';');
        var slashSlash = line.IndexOf("//", StringComparison.Ordinal);
        var cut = line.Length;
        if (semicolon >= 0)
        {
            cut = Math.Min(cut, semicolon);
        }

        if (slashSlash >= 0)
        {
            cut = Math.Min(cut, slashSlash);
        }

        return line[..cut];
    }
}
