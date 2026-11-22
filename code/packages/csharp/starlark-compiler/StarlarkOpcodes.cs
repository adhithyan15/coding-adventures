using System;
using System.Collections.Generic;
using System.Text;
using System.Text.Json;

namespace CodingAdventures.StarlarkCompiler;

/// <summary>
/// Opcode categories representing functional bytecode domains.
/// </summary>
public enum OpCategory
{
    /// <summary>Stack manipulation (0x0).</summary>
    Stack = 0x0,

    /// <summary>Variable and scope management (0x1).</summary>
    Variable = 0x1,

    /// <summary>Arithmetic and bitwise operations (0x2).</summary>
    Arithmetic = 0x2,

    /// <summary>Comparison and logical tests (0x3).</summary>
    Comparison = 0x3,

    /// <summary>Control flow jumps (0x4).</summary>
    ControlFlow = 0x4,

    /// <summary>Function definition and calls (0x5).</summary>
    Function = 0x5,

    /// <summary>Collection construction and mutation (0x6).</summary>
    Collection = 0x6,

    /// <summary>Subscript, attribute, and slice access (0x7).</summary>
    SubscriptAttribute = 0x7,

    /// <summary>Iteration and loop protocols (0x8).</summary>
    Iteration = 0x8,

    /// <summary>Module imports and namespaces (0x9).</summary>
    Module = 0x9,

    /// <summary>Input and output operations (0xA).</summary>
    Io = 0xA,

    /// <summary>Virtual machine execution control (0xF).</summary>
    VmControl = 0xF,
}

/// <summary>
/// Extension methods for OpCategory.
/// </summary>
public static class OpCategoryExtensions
{
    /// <summary>Gets the canonical snake_case string identifier.</summary>
    public static string Id(this OpCategory category) => category switch
    {
        OpCategory.Stack => "stack",
        OpCategory.Variable => "variable",
        OpCategory.Arithmetic => "arithmetic",
        OpCategory.Comparison => "comparison",
        OpCategory.ControlFlow => "control_flow",
        OpCategory.Function => "function",
        OpCategory.Collection => "collection",
        OpCategory.SubscriptAttribute => "subscript_attribute",
        OpCategory.Iteration => "iteration",
        OpCategory.Module => "module",
        OpCategory.Io => "io",
        OpCategory.VmControl => "vm_control",
        _ => throw StarlarkCompilerException.CorpusSchemaViolation(),
    };

    /// <summary>Gets the camelCase string identifier.</summary>
    public static string CamelName(this OpCategory category) => category switch
    {
        OpCategory.Stack => "stack",
        OpCategory.Variable => "variable",
        OpCategory.Arithmetic => "arithmetic",
        OpCategory.Comparison => "comparison",
        OpCategory.ControlFlow => "controlFlow",
        OpCategory.Function => "function",
        OpCategory.Collection => "collection",
        OpCategory.SubscriptAttribute => "subscriptAttribute",
        OpCategory.Iteration => "iteration",
        OpCategory.Module => "module",
        OpCategory.Io => "io",
        OpCategory.VmControl => "vmControl",
        _ => throw StarlarkCompilerException.CorpusSchemaViolation(),
    };

    /// <summary>Gets the category high-nibble byte value.</summary>
    public static byte Nibble(this OpCategory category) => (byte)category;
}

/// <summary>
/// The 61 distinct Starlark bytecode opcode enumerations.
/// </summary>
public enum Op : byte
{
    // Stack (0x01..0x06)
    /// <summary>Pushes a constant value from the pool onto the stack.</summary>
    LoadConst = 0x01,
    /// <summary>Pops and discards the top value on the stack.</summary>
    Pop = 0x02,
    /// <summary>Duplicates the top value on the stack.</summary>
    Dup = 0x03,
    /// <summary>Pushes None onto the stack.</summary>
    LoadNone = 0x04,
    /// <summary>Pushes True onto the stack.</summary>
    LoadTrue = 0x05,
    /// <summary>Pushes False onto the stack.</summary>
    LoadFalse = 0x06,

    // Variable (0x10..0x15)
    /// <summary>Stores top stack value in a global/module variable.</summary>
    StoreName = 0x10,
    /// <summary>Loads variable value by name onto the stack.</summary>
    LoadName = 0x11,
    /// <summary>Stores top stack value into local scope slot.</summary>
    StoreLocal = 0x12,
    /// <summary>Loads value from local scope slot onto the stack.</summary>
    LoadLocal = 0x13,
    /// <summary>Stores value into closure cell.</summary>
    StoreClosure = 0x14,
    /// <summary>Loads value from closure cell onto the stack.</summary>
    LoadClosure = 0x15,

    // Arithmetic (0x20..0x2D)
    /// <summary>Binary addition.</summary>
    Add = 0x20,
    /// <summary>Binary subtraction.</summary>
    Sub = 0x21,
    /// <summary>Binary multiplication.</summary>
    Mul = 0x22,
    /// <summary>Binary floating point division.</summary>
    Div = 0x23,
    /// <summary>Binary floor/integer division.</summary>
    FloorDiv = 0x24,
    /// <summary>Modulo remainder.</summary>
    Mod = 0x25,
    /// <summary>Exponentiation power.</summary>
    Power = 0x26,
    /// <summary>Unary numerical negation.</summary>
    Negate = 0x27,
    /// <summary>Bitwise AND.</summary>
    BitAnd = 0x28,
    /// <summary>Bitwise OR.</summary>
    BitOr = 0x29,
    /// <summary>Bitwise XOR.</summary>
    BitXor = 0x2A,
    /// <summary>Bitwise NOT inversion.</summary>
    BitNot = 0x2B,
    /// <summary>Bitwise left shift.</summary>
    LShift = 0x2C,
    /// <summary>Bitwise right shift.</summary>
    RShift = 0x2D,

    // Comparison (0x30..0x38)
    /// <summary>Equality comparison.</summary>
    CmpEq = 0x30,
    /// <summary>Inequality comparison.</summary>
    CmpNe = 0x31,
    /// <summary>Less than comparison.</summary>
    CmpLt = 0x32,
    /// <summary>Greater than comparison.</summary>
    CmpGt = 0x33,
    /// <summary>Less than or equal comparison.</summary>
    CmpLe = 0x34,
    /// <summary>Greater than or equal comparison.</summary>
    CmpGe = 0x35,
    /// <summary>Containment inclusion check (in).</summary>
    CmpIn = 0x36,
    /// <summary>Containment exclusion check (not in).</summary>
    CmpNotIn = 0x37,
    /// <summary>Logical Boolean NOT.</summary>
    Not = 0x38,

    // Control Flow (0x40..0x44)
    /// <summary>Unconditional relative or absolute jump.</summary>
    Jump = 0x40,
    /// <summary>Jump if top stack value is falsey, popping value.</summary>
    JumpIfFalse = 0x41,
    /// <summary>Jump if top stack value is truthy, popping value.</summary>
    JumpIfTrue = 0x42,
    /// <summary>Jump if top stack value is falsey, else pop value.</summary>
    JumpIfFalseOrPop = 0x43,
    /// <summary>Jump if top stack value is truthy, else pop value.</summary>
    JumpIfTrueOrPop = 0x44,

    // Function (0x50..0x53)
    /// <summary>Constructs function object from code descriptor.</summary>
    MakeFunction = 0x50,
    /// <summary>Calls positional arguments function.</summary>
    CallFunction = 0x51,
    /// <summary>Calls keyword arguments function.</summary>
    CallFunctionKw = 0x52,
    /// <summary>Returns from function with top stack value.</summary>
    Return = 0x53,

    // Collection (0x60..0x64)
    /// <summary>Builds mutable list from n stack elements.</summary>
    BuildList = 0x60,
    /// <summary>Builds dictionary from 2n key-value pairs.</summary>
    BuildDict = 0x61,
    /// <summary>Builds immutable tuple from n stack elements.</summary>
    BuildTuple = 0x62,
    /// <summary>Appends element to list in place.</summary>
    ListAppend = 0x63,
    /// <summary>Sets dictionary key-value entry in place.</summary>
    DictSet = 0x64,

    // Subscript / Attribute (0x70..0x74)
    /// <summary>Loads element via index/key subscripting.</summary>
    LoadSubscript = 0x70,
    /// <summary>Stores element into target container subscript.</summary>
    StoreSubscript = 0x71,
    /// <summary>Loads attribute or property by name.</summary>
    LoadAttr = 0x72,
    /// <summary>Stores attribute or property by name.</summary>
    StoreAttr = 0x73,
    /// <summary>Loads slicing range from container.</summary>
    LoadSlice = 0x74,

    // Iteration (0x80..0x82)
    /// <summary>Obtains iterator object from iterable container.</summary>
    GetIter = 0x80,
    /// <summary>Advances iteration or jumps if exhausted.</summary>
    ForIter = 0x81,
    /// <summary>Unpacks sequence elements onto stack.</summary>
    UnpackSequence = 0x82,

    // Module (0x90..0x91)
    /// <summary>Loads external module or rule definition.</summary>
    LoadModule = 0x90,
    /// <summary>Imports specified binding symbol from module.</summary>
    ImportFrom = 0x91,

    // IO (0xA0)
    /// <summary>Prints values to execution standard output.</summary>
    Print = 0xA0,

    // VM Control (0xFF)
    /// <summary>Terminates virtual machine execution.</summary>
    Halt = 0xFF,
}

/// <summary>
/// Static provider providing lookup tables, opcode metadata, and operator maps.
/// </summary>
public static class StarlarkOpcodes
{
    private static readonly Op[] CanonicalOps =
    [
        Op.LoadConst, Op.Pop, Op.Dup, Op.LoadNone, Op.LoadTrue, Op.LoadFalse,
        Op.StoreName, Op.LoadName, Op.StoreLocal, Op.LoadLocal, Op.StoreClosure, Op.LoadClosure,
        Op.Add, Op.Sub, Op.Mul, Op.Div, Op.FloorDiv, Op.Mod, Op.Power, Op.Negate,
        Op.BitAnd, Op.BitOr, Op.BitXor, Op.BitNot, Op.LShift, Op.RShift,
        Op.CmpEq, Op.CmpNe, Op.CmpLt, Op.CmpGt, Op.CmpLe, Op.CmpGe, Op.CmpIn, Op.CmpNotIn, Op.Not,
        Op.Jump, Op.JumpIfFalse, Op.JumpIfTrue, Op.JumpIfFalseOrPop, Op.JumpIfTrueOrPop,
        Op.MakeFunction, Op.CallFunction, Op.CallFunctionKw, Op.Return,
        Op.BuildList, Op.BuildDict, Op.BuildTuple, Op.ListAppend, Op.DictSet,
        Op.LoadSubscript, Op.StoreSubscript, Op.LoadAttr, Op.StoreAttr, Op.LoadSlice,
        Op.GetIter, Op.ForIter, Op.UnpackSequence,
        Op.LoadModule, Op.ImportFrom,
        Op.Print,
        Op.Halt,
    ];

    private static readonly Op?[] DecodeTable = new Op?[StarlarkCompilerConstants.TotalDecodeEntries];
    private static readonly Dictionary<string, Op> SymbolToOp = new(StringComparer.Ordinal);
    private static readonly Dictionary<Op, string> OpToSymbol = new();
    private static readonly Dictionary<Op, OpCategory> OpToCategory = new();

    private static readonly Dictionary<string, Op> RawBinaryOps = new(StringComparer.Ordinal)
    {
        { "+", Op.Add },
        { "-", Op.Sub },
        { "*", Op.Mul },
        { "/", Op.Div },
        { "//", Op.FloorDiv },
        { "%", Op.Mod },
        { "**", Op.Power },
        { "&", Op.BitAnd },
        { "|", Op.BitOr },
        { "^", Op.BitXor },
        { "<<", Op.LShift },
        { ">>", Op.RShift },
    };

    private static readonly Dictionary<string, Op> RawCompareOps = new(StringComparer.Ordinal)
    {
        { "==", Op.CmpEq },
        { "!=", Op.CmpNe },
        { "<", Op.CmpLt },
        { ">", Op.CmpGt },
        { "<=", Op.CmpLe },
        { ">=", Op.CmpGe },
        { "in", Op.CmpIn },
        { "not in", Op.CmpNotIn },
    };

    private static readonly Dictionary<string, Op> RawAugmentedAssignOps = new(StringComparer.Ordinal)
    {
        { "+=", Op.Add },
        { "-=", Op.Sub },
        { "*=", Op.Mul },
        { "/=", Op.Div },
        { "//=", Op.FloorDiv },
        { "%=", Op.Mod },
        { "**=", Op.Power },
        { "&=", Op.BitAnd },
        { "|=", Op.BitOr },
        { "^=", Op.BitXor },
        { "<<=", Op.LShift },
        { ">>=", Op.RShift },
    };

    private static readonly Dictionary<string, Op> RawUnaryOps = new(StringComparer.Ordinal)
    {
        { "-", Op.Negate },
        { "~", Op.BitNot },
    };

    static StarlarkOpcodes()
    {
        foreach (var op in CanonicalOps)
        {
            var b = (byte)op;
            var sym = op.ToString();
            DecodeTable[b] = op;
            SymbolToOp[sym] = op;
            OpToSymbol[op] = sym;

            var cat = (b >> StarlarkCompilerConstants.CategoryNibbleShift) switch
            {
                0x0 => OpCategory.Stack,
                0x1 => OpCategory.Variable,
                0x2 => OpCategory.Arithmetic,
                0x3 => OpCategory.Comparison,
                0x4 => OpCategory.ControlFlow,
                0x5 => OpCategory.Function,
                0x6 => OpCategory.Collection,
                0x7 => OpCategory.SubscriptAttribute,
                0x8 => OpCategory.Iteration,
                0x9 => OpCategory.Module,
                0xA => OpCategory.Io,
                0xF => OpCategory.VmControl,
                _ => throw StarlarkCompilerException.CorpusSchemaViolation(),
            };
            OpToCategory[op] = cat;
        }
    }

    /// <summary>Returns a fresh copy of the canonical list of all 61 opcodes.</summary>
    public static IReadOnlyList<Op> AllOps() => (Op[])CanonicalOps.Clone();

    /// <summary>Decodes an opcode from a byte value (0..255), returning null for holes or invalid numbers.</summary>
    public static Op? OpFromByte(int value)
    {
        if (value < 0 || value >= StarlarkCompilerConstants.TotalDecodeEntries)
        {
            return null;
        }
        return DecodeTable[value];
    }

    /// <summary>Looks up an opcode by its exact PascalCase symbol name, returning null if not found.</summary>
    public static Op? OpFromSymbol(string? symbol)
    {
        if (string.IsNullOrEmpty(symbol)) return null;
        return SymbolToOp.TryGetValue(symbol, out var op) ? op : null;
    }

    /// <summary>Gets the raw byte value of an opcode.</summary>
    public static byte OpByte(Op op) => (byte)op;

    /// <summary>Gets the symbol string of an opcode.</summary>
    public static string OpSymbol(Op op) => OpToSymbol[op];

    /// <summary>Gets the category of an opcode.</summary>
    public static OpCategory OpCategoryOf(Op op) => OpToCategory[op];

    /// <summary>Gets the category corresponding to an opcode byte, or null if unassigned.</summary>
    public static OpCategory? OpCategoryFromByte(int value)
    {
        var op = OpFromByte(value);
        return op.HasValue ? OpToCategory[op.Value] : null;
    }

    /// <summary>Returns a fresh mutable dictionary copy of the 12 binary operators.</summary>
    public static Dictionary<string, Op> BinaryOpMap() =>
        new(RawBinaryOps, StringComparer.Ordinal);

    /// <summary>Returns a fresh mutable dictionary copy of the 8 comparison operators.</summary>
    public static Dictionary<string, Op> CompareOpMap() =>
        new(RawCompareOps, StringComparer.Ordinal);

    /// <summary>Returns a fresh mutable dictionary copy of the 12 augmented assignment operators.</summary>
    public static Dictionary<string, Op> AugmentedAssignMap() =>
        new(RawAugmentedAssignOps, StringComparer.Ordinal);

    /// <summary>Returns a fresh mutable dictionary copy of the 2 unary operators.</summary>
    public static Dictionary<string, Op> UnaryOpMap() =>
        new(RawUnaryOps, StringComparer.Ordinal);

    /// <summary>Finds the opcode corresponding to a binary operator string, or null.</summary>
    public static Op? BinaryOpcode(string? op)
    {
        if (op == null) return null;
        return RawBinaryOps.TryGetValue(op, out var value) ? value : null;
    }

    /// <summary>Finds the opcode corresponding to a comparison operator string, or null.</summary>
    public static Op? CompareOpcode(string? op)
    {
        if (op == null) return null;
        return RawCompareOps.TryGetValue(op, out var value) ? value : null;
    }

    /// <summary>Finds the opcode corresponding to an augmented assignment operator string, or null.</summary>
    public static Op? AugmentedAssignOpcode(string? op)
    {
        if (op == null) return null;
        return RawAugmentedAssignOps.TryGetValue(op, out var value) ? value : null;
    }

    /// <summary>Finds the opcode corresponding to a unary operator string, or null.</summary>
    public static Op? UnaryOpcode(string? op)
    {
        if (op == null) return null;
        return RawUnaryOps.TryGetValue(op, out var value) ? value : null;
    }

    /// <summary>
    /// Parses and validates a raw STP01 corpus payload against strict bounds and schema.
    /// Complies with Standing Rule #20 and 5-class security defenses.
    /// </summary>
    public static void ValidateCorpusBytes(byte[] rawBytes)
    {
        if (rawBytes is null)
        {
            throw StarlarkCompilerException.CorpusMalformedTransport();
        }

        if (rawBytes.Length > StarlarkCompilerConstants.MaxCorpusBytes)
        {
            throw StarlarkCompilerException.CorpusByteLimitExceeded();
        }

        // BOM rejection (UTF-8 BOM: 0xEF, 0xBB, 0xBF)
        if (rawBytes.Length >= StarlarkCompilerConstants.Utf8BomLength &&
            rawBytes[0] == StarlarkCompilerConstants.Utf8BomByte0 &&
            rawBytes[1] == StarlarkCompilerConstants.Utf8BomByte1 &&
            rawBytes[2] == StarlarkCompilerConstants.Utf8BomByte2)
        {
            throw StarlarkCompilerException.CorpusMalformedTransport();
        }

        string text;
        try
        {
            var utf8Strict = new UTF8Encoding(false, true);
            text = utf8Strict.GetString(rawBytes);
        }
        catch (DecoderFallbackException)
        {
            throw StarlarkCompilerException.CorpusMalformedTransport();
        }

        JsonDocument doc;
        try
        {
            doc = JsonDocument.Parse(text);
        }
        catch (JsonException)
        {
            throw StarlarkCompilerException.CorpusMalformedTransport();
        }

        using (doc)
        {
            var root = doc.RootElement;
            if (root.ValueKind != JsonValueKind.Object)
            {
                throw StarlarkCompilerException.CorpusSchemaViolation();
            }

            if (!root.TryGetProperty(StarlarkCompilerConstants.PropSchemaVersion, out var schemaProp) ||
                schemaProp.ValueKind != JsonValueKind.Number ||
                !schemaProp.TryGetInt32(out var schemaVersion) ||
                schemaVersion != StarlarkCompilerConstants.ExpectedSchemaVersion)
            {
                throw StarlarkCompilerException.CorpusSchemaViolation();
            }

            if (!root.TryGetProperty(StarlarkCompilerConstants.PropProfile, out var profileProp) ||
                profileProp.ValueKind != JsonValueKind.String ||
                profileProp.GetString() != StarlarkCompilerConstants.ProfileStandaloneProviderV1)
            {
                throw StarlarkCompilerException.CorpusSchemaViolation();
            }

            if (!root.TryGetProperty(StarlarkCompilerConstants.PropOpcodes, out var opcodesProp) ||
                opcodesProp.ValueKind != JsonValueKind.Array ||
                opcodesProp.GetArrayLength() != StarlarkCompilerConstants.TotalOpcodeCount)
            {
                throw StarlarkCompilerException.CorpusSchemaViolation();
            }

            if (!root.TryGetProperty(StarlarkCompilerConstants.PropDecode, out var decodeProp) ||
                decodeProp.ValueKind != JsonValueKind.Array ||
                decodeProp.GetArrayLength() != StarlarkCompilerConstants.TotalDecodeEntries)
            {
                throw StarlarkCompilerException.CorpusSchemaViolation();
            }
        }
    }
}
