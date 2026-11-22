using System.Text;
using System.Text.Json;
using Xunit;

namespace CodingAdventures.StarlarkCompiler.CSharp.Tests;

public class StarlarkCompilerTests
{
    private static readonly (int Byte, string Symbol, string Category)[] IndependentOpcodes =
    [
        (0x01, "LoadConst", "stack"),
        (0x02, "Pop", "stack"),
        (0x03, "Dup", "stack"),
        (0x04, "LoadNone", "stack"),
        (0x05, "LoadTrue", "stack"),
        (0x06, "LoadFalse", "stack"),

        (0x10, "StoreName", "variable"),
        (0x11, "LoadName", "variable"),
        (0x12, "StoreLocal", "variable"),
        (0x13, "LoadLocal", "variable"),
        (0x14, "StoreClosure", "variable"),
        (0x15, "LoadClosure", "variable"),

        (0x20, "Add", "arithmetic"),
        (0x21, "Sub", "arithmetic"),
        (0x22, "Mul", "arithmetic"),
        (0x23, "Div", "arithmetic"),
        (0x24, "FloorDiv", "arithmetic"),
        (0x25, "Mod", "arithmetic"),
        (0x26, "Power", "arithmetic"),
        (0x27, "Negate", "arithmetic"),
        (0x28, "BitAnd", "arithmetic"),
        (0x29, "BitOr", "arithmetic"),
        (0x2A, "BitXor", "arithmetic"),
        (0x2B, "BitNot", "arithmetic"),
        (0x2C, "LShift", "arithmetic"),
        (0x2D, "RShift", "arithmetic"),

        (0x30, "CmpEq", "comparison"),
        (0x31, "CmpNe", "comparison"),
        (0x32, "CmpLt", "comparison"),
        (0x33, "CmpGt", "comparison"),
        (0x34, "CmpLe", "comparison"),
        (0x35, "CmpGe", "comparison"),
        (0x36, "CmpIn", "comparison"),
        (0x37, "CmpNotIn", "comparison"),
        (0x38, "Not", "comparison"),

        (0x40, "Jump", "control_flow"),
        (0x41, "JumpIfFalse", "control_flow"),
        (0x42, "JumpIfTrue", "control_flow"),
        (0x43, "JumpIfFalseOrPop", "control_flow"),
        (0x44, "JumpIfTrueOrPop", "control_flow"),

        (0x50, "MakeFunction", "function"),
        (0x51, "CallFunction", "function"),
        (0x52, "CallFunctionKw", "function"),
        (0x53, "Return", "function"),

        (0x60, "BuildList", "collection"),
        (0x61, "BuildDict", "collection"),
        (0x62, "BuildTuple", "collection"),
        (0x63, "ListAppend", "collection"),
        (0x64, "DictSet", "collection"),

        (0x70, "LoadSubscript", "subscript_attribute"),
        (0x71, "StoreSubscript", "subscript_attribute"),
        (0x72, "LoadAttr", "subscript_attribute"),
        (0x73, "StoreAttr", "subscript_attribute"),
        (0x74, "LoadSlice", "subscript_attribute"),

        (0x80, "GetIter", "iteration"),
        (0x81, "ForIter", "iteration"),
        (0x82, "UnpackSequence", "iteration"),

        (0x90, "LoadModule", "module"),
        (0x91, "ImportFrom", "module"),

        (0xA0, "Print", "io"),

        (0xFF, "Halt", "vm_control"),
    ];

    private static readonly (string Group, string Op, string ExpectedSymbol)[] IndependentOperators =
    [
        // Binary (12)
        ("binary", "+", "Add"),
        ("binary", "-", "Sub"),
        ("binary", "*", "Mul"),
        ("binary", "/", "Div"),
        ("binary", "//", "FloorDiv"),
        ("binary", "%", "Mod"),
        ("binary", "**", "Power"),
        ("binary", "&", "BitAnd"),
        ("binary", "|", "BitOr"),
        ("binary", "^", "BitXor"),
        ("binary", "<<", "LShift"),
        ("binary", ">>", "RShift"),

        // Comparison (8)
        ("compare", "==", "CmpEq"),
        ("compare", "!=", "CmpNe"),
        ("compare", "<", "CmpLt"),
        ("compare", ">", "CmpGt"),
        ("compare", "<=", "CmpLe"),
        ("compare", ">=", "CmpGe"),
        ("compare", "in", "CmpIn"),
        ("compare", "not in", "CmpNotIn"),

        // Augmented Assignment (12)
        ("augmented_assign", "+=", "Add"),
        ("augmented_assign", "-=", "Sub"),
        ("augmented_assign", "*=", "Mul"),
        ("augmented_assign", "/=", "Div"),
        ("augmented_assign", "//=", "FloorDiv"),
        ("augmented_assign", "%=", "Mod"),
        ("augmented_assign", "**=", "Power"),
        ("augmented_assign", "&=", "BitAnd"),
        ("augmented_assign", "|=", "BitOr"),
        ("augmented_assign", "^=", "BitXor"),
        ("augmented_assign", "<<=", "LShift"),
        ("augmented_assign", ">>=", "RShift"),

        // Unary (2)
        ("unary", "-", "Negate"),
        ("unary", "~", "BitNot"),
    ];

    private static string LocateFixture(string relativePath)
    {
        var current = AppContext.BaseDirectory;
        for (var i = 0; i < 10; i++)
        {
            var candidate = Path.Combine(current, relativePath);
            if (File.Exists(candidate))
            {
                return candidate;
            }
            var parent = Directory.GetParent(current);
            if (parent == null) break;
            current = parent.FullName;
        }
        throw new FileNotFoundException($"Cannot locate fixture: {relativePath}");
    }

    [Fact]
    public void STP01_CasesJson_Conformance_Verification()
    {
        var fixturePath = LocateFixture("code/specs/fixtures/starlark-opcode-provider-v1/cases.json");
        var rawBytes = File.ReadAllBytes(fixturePath);
        Assert.True(rawBytes.Length <= StarlarkCompilerConstants.MaxCorpusBytes);

        // Validate via provider
        StarlarkOpcodes.ValidateCorpusBytes(rawBytes);

        using var doc = JsonDocument.Parse(rawBytes);
        var root = doc.RootElement;

        // Opcodes array verification
        var opcodes = root.GetProperty("opcodes");
        Assert.Equal(61, opcodes.GetArrayLength());
        foreach (var item in opcodes.EnumerateArray())
        {
            var symbol = item.GetProperty("symbol").GetString()!;
            var b = item.GetProperty("byte").GetInt32();
            var category = item.GetProperty("category").GetString()!;

            var op = StarlarkOpcodes.OpFromSymbol(symbol);
            Assert.NotNull(op);
            Assert.Equal(b, StarlarkOpcodes.OpByte(op.Value));
            Assert.Equal(symbol, StarlarkOpcodes.OpSymbol(op.Value));
            Assert.Equal(category, StarlarkOpcodes.OpCategoryOf(op.Value).Id());

            var opFromByte = StarlarkOpcodes.OpFromByte(b);
            Assert.Equal(op, opFromByte);
        }

        // Decode table verification (256 entries)
        var decode = root.GetProperty("decode");
        Assert.Equal(256, decode.GetArrayLength());
        var decodeIdx = 0;
        var nonNullCount = 0;
        foreach (var item in decode.EnumerateArray())
        {
            var opFromByte = StarlarkOpcodes.OpFromByte(decodeIdx);
            var symbolProp = item.GetProperty("symbol");
            if (symbolProp.ValueKind == JsonValueKind.Null)
            {
                Assert.Null(opFromByte);
            }
            else
            {
                nonNullCount++;
                var symbol = symbolProp.GetString();
                Assert.NotNull(opFromByte);
                Assert.Equal(symbol, StarlarkOpcodes.OpSymbol(opFromByte.Value));
            }
            decodeIdx++;
        }
        Assert.Equal(61, nonNullCount);
        Assert.Equal(195, 256 - nonNullCount);

        // Operators map verification
        var operators = root.GetProperty("operators");
        var binMap = StarlarkOpcodes.BinaryOpMap();
        var cmpMap = StarlarkOpcodes.CompareOpMap();
        var augMap = StarlarkOpcodes.AugmentedAssignMap();
        var unMap = StarlarkOpcodes.UnaryOpMap();

        foreach (var item in operators.GetProperty("binary").EnumerateArray())
        {
            var opStr = item.GetProperty("operator").GetString()!;
            var sym = item.GetProperty("symbol").GetString()!;
            Assert.True(binMap.ContainsKey(opStr));
            Assert.Equal(sym, StarlarkOpcodes.OpSymbol(binMap[opStr]));
            Assert.Equal(sym, StarlarkOpcodes.OpSymbol(StarlarkOpcodes.BinaryOpcode(opStr)!.Value));
        }

        foreach (var item in operators.GetProperty("compare").EnumerateArray())
        {
            var opStr = item.GetProperty("operator").GetString()!;
            var sym = item.GetProperty("symbol").GetString()!;
            Assert.True(cmpMap.ContainsKey(opStr));
            Assert.Equal(sym, StarlarkOpcodes.OpSymbol(cmpMap[opStr]));
            Assert.Equal(sym, StarlarkOpcodes.OpSymbol(StarlarkOpcodes.CompareOpcode(opStr)!.Value));
        }

        foreach (var item in operators.GetProperty("augmented_assign").EnumerateArray())
        {
            var opStr = item.GetProperty("operator").GetString()!;
            var sym = item.GetProperty("symbol").GetString()!;
            Assert.True(augMap.ContainsKey(opStr));
            Assert.Equal(sym, StarlarkOpcodes.OpSymbol(augMap[opStr]));
            Assert.Equal(sym, StarlarkOpcodes.OpSymbol(StarlarkOpcodes.AugmentedAssignOpcode(opStr)!.Value));
        }

        foreach (var item in operators.GetProperty("unary").EnumerateArray())
        {
            var opStr = item.GetProperty("operator").GetString()!;
            var sym = item.GetProperty("symbol").GetString()!;
            Assert.True(unMap.ContainsKey(opStr));
            Assert.Equal(sym, StarlarkOpcodes.OpSymbol(unMap[opStr]));
            Assert.Equal(sym, StarlarkOpcodes.OpSymbol(StarlarkOpcodes.UnaryOpcode(opStr)!.Value));
        }

        // Unknown lookups verification
        var unknownLookups = root.GetProperty("unknown_lookups");
        foreach (var item in unknownLookups.EnumerateArray())
        {
            var group = item.GetProperty("group").GetString()!;
            var opStr = item.GetProperty("operator").GetString()!;
            Op? result = group switch
            {
                "binary" => StarlarkOpcodes.BinaryOpcode(opStr),
                "compare" => StarlarkOpcodes.CompareOpcode(opStr),
                "augmented_assign" => StarlarkOpcodes.AugmentedAssignOpcode(opStr),
                "unary" => StarlarkOpcodes.UnaryOpcode(opStr),
                _ => throw new Exception("unknown group"),
            };
            Assert.Null(result);
        }
    }

    [Fact]
    public void IndependentLiteralVerification_OpcodesAndOperators()
    {
        Assert.Equal(61, IndependentOpcodes.Length);
        foreach (var (b, sym, cat) in IndependentOpcodes)
        {
            var op = StarlarkOpcodes.OpFromByte(b);
            Assert.NotNull(op);
            Assert.Equal(sym, StarlarkOpcodes.OpSymbol(op.Value));
            Assert.Equal(cat, StarlarkOpcodes.OpCategoryOf(op.Value).Id());

            var opFromSym = StarlarkOpcodes.OpFromSymbol(sym);
            Assert.Equal(op, opFromSym);
        }

        Assert.Equal(34, IndependentOperators.Length);
        foreach (var (group, opStr, expectedSym) in IndependentOperators)
        {
            Op? op = group switch
            {
                "binary" => StarlarkOpcodes.BinaryOpcode(opStr),
                "compare" => StarlarkOpcodes.CompareOpcode(opStr),
                "augmented_assign" => StarlarkOpcodes.AugmentedAssignOpcode(opStr),
                "unary" => StarlarkOpcodes.UnaryOpcode(opStr),
                _ => null,
            };
            Assert.NotNull(op);
            Assert.Equal(expectedSym, StarlarkOpcodes.OpSymbol(op.Value));
        }
    }

    [Fact]
    public void OpCategory_MetadataAndExtensions()
    {
        var allCats = Enum.GetValues<OpCategory>();
        Assert.Equal(12, allCats.Length);
        foreach (var cat in allCats)
        {
            Assert.False(string.IsNullOrEmpty(cat.Id()));
            Assert.False(string.IsNullOrEmpty(cat.CamelName()));
            Assert.True(cat.Nibble() <= 0xF);
        }

        Assert.Equal("stack", OpCategory.Stack.Id());
        Assert.Equal("control_flow", OpCategory.ControlFlow.Id());
        Assert.Equal("controlFlow", OpCategory.ControlFlow.CamelName());
        Assert.Equal("subscriptAttribute", OpCategory.SubscriptAttribute.CamelName());
        Assert.Equal("vmControl", OpCategory.VmControl.CamelName());
        Assert.Equal(0x0, OpCategory.Stack.Nibble());
        Assert.Equal(0xF, OpCategory.VmControl.Nibble());

        var invalidCat = (OpCategory)999;
        Assert.Throws<StarlarkCompilerException>(() => invalidCat.Id());
        Assert.Throws<StarlarkCompilerException>(() => invalidCat.CamelName());
    }

    [Fact]
    public void BoundaryAndLookupFailures()
    {
        Assert.Null(StarlarkOpcodes.OpFromByte(-1));
        Assert.Null(StarlarkOpcodes.OpFromByte(256));
        Assert.Null(StarlarkOpcodes.OpFromByte(1000));
        Assert.Null(StarlarkOpcodes.OpFromByte(0x00)); // Unassigned hole 0x00
        Assert.Null(StarlarkOpcodes.OpFromByte(0x07)); // Unassigned hole 0x07
        Assert.Null(StarlarkOpcodes.OpFromByte(0xFE)); // Unassigned hole 0xFE

        Assert.Null(StarlarkOpcodes.OpCategoryFromByte(-1));
        Assert.Null(StarlarkOpcodes.OpCategoryFromByte(256));
        Assert.Null(StarlarkOpcodes.OpCategoryFromByte(0x00));
        Assert.Equal(OpCategory.Stack, StarlarkOpcodes.OpCategoryFromByte(0x01));

        Assert.Null(StarlarkOpcodes.OpFromSymbol(null));
        Assert.Null(StarlarkOpcodes.OpFromSymbol(""));
        Assert.Null(StarlarkOpcodes.OpFromSymbol("UnknownOpcode"));
        Assert.Null(StarlarkOpcodes.OpFromSymbol("loadconst")); // Case-sensitive

        Assert.Null(StarlarkOpcodes.BinaryOpcode(null));
        Assert.Null(StarlarkOpcodes.BinaryOpcode("invalid"));
        Assert.Null(StarlarkOpcodes.CompareOpcode(null));
        Assert.Null(StarlarkOpcodes.CompareOpcode("invalid"));
        Assert.Null(StarlarkOpcodes.AugmentedAssignOpcode(null));
        Assert.Null(StarlarkOpcodes.AugmentedAssignOpcode("invalid"));
        Assert.Null(StarlarkOpcodes.UnaryOpcode(null));
        Assert.Null(StarlarkOpcodes.UnaryOpcode("invalid"));
    }

    [Fact]
    public void FreshMapOwnershipGuarantee()
    {
        var map1 = StarlarkOpcodes.BinaryOpMap();
        var map2 = StarlarkOpcodes.BinaryOpMap();
        Assert.NotSame(map1, map2);
        map1["+"] = Op.Halt;
        Assert.Equal(Op.Add, map2["+"]);
        Assert.Equal(Op.Add, StarlarkOpcodes.BinaryOpcode("+"));

        var cmp1 = StarlarkOpcodes.CompareOpMap();
        var cmp2 = StarlarkOpcodes.CompareOpMap();
        Assert.NotSame(cmp1, cmp2);
        cmp1["=="] = Op.Halt;
        Assert.Equal(Op.CmpEq, cmp2["=="]);

        var aug1 = StarlarkOpcodes.AugmentedAssignMap();
        var aug2 = StarlarkOpcodes.AugmentedAssignMap();
        Assert.NotSame(aug1, aug2);
        aug1["+="] = Op.Halt;
        Assert.Equal(Op.Add, aug2["+="]);

        var un1 = StarlarkOpcodes.UnaryOpMap();
        var un2 = StarlarkOpcodes.UnaryOpMap();
        Assert.NotSame(un1, un2);
        un1["-"] = Op.Halt;
        Assert.Equal(Op.Negate, un2["-"]);

        var ops1 = StarlarkOpcodes.AllOps();
        var ops2 = StarlarkOpcodes.AllOps();
        Assert.NotSame(ops1, ops2);
        Assert.Equal(61, ops1.Count);
    }

    [Fact]
    public void StandingRule20_DiagnosticCatalogAndExceptions()
    {
        var exInvalidByte = StarlarkCompilerException.InvalidByte();
        Assert.Equal(StarlarkCompilerConstants.ErrorIdInvalidByte, exInvalidByte.ErrorId);
        Assert.Equal(StarlarkCompilerErrorCode.InvalidByte, exInvalidByte.Code);
        Assert.DoesNotContain("0x", exInvalidByte.Message);

        var exUnknownOpcode = StarlarkCompilerException.UnknownOpcode();
        Assert.Equal(StarlarkCompilerConstants.ErrorIdUnknownOpcode, exUnknownOpcode.ErrorId);
        Assert.Equal(StarlarkCompilerErrorCode.UnknownOpcode, exUnknownOpcode.Code);

        var exUnknownOperator = StarlarkCompilerException.UnknownOperator();
        Assert.Equal(StarlarkCompilerConstants.ErrorIdUnknownOperator, exUnknownOperator.ErrorId);
        Assert.Equal(StarlarkCompilerErrorCode.UnknownOperator, exUnknownOperator.Code);

        var exByteLimit = StarlarkCompilerException.CorpusByteLimitExceeded();
        Assert.Equal(StarlarkCompilerConstants.ErrorIdInvalidCorpus, exByteLimit.ErrorId);
        Assert.Equal(StarlarkCompilerErrorCode.CorpusByteLimitExceeded, exByteLimit.Code);

        var exTransport = StarlarkCompilerException.CorpusMalformedTransport();
        Assert.Equal(StarlarkCompilerConstants.ErrorIdInvalidCorpus, exTransport.ErrorId);
        Assert.Equal(StarlarkCompilerErrorCode.CorpusMalformedTransport, exTransport.Code);

        var exSchema = StarlarkCompilerException.CorpusSchemaViolation();
        Assert.Equal(StarlarkCompilerConstants.ErrorIdInvalidCorpus, exSchema.ErrorId);
        Assert.Equal(StarlarkCompilerErrorCode.CorpusSchemaViolation, exSchema.Code);
    }

    [Fact]
    public void TransportAndSchemaCorruptionRejections()
    {
        // Null payload rejection
        var exNull = Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(null!));
        Assert.Equal(StarlarkCompilerErrorCode.CorpusMalformedTransport, exNull.Code);

        // Byte limit exceeded
        var oversized = new byte[StarlarkCompilerConstants.MaxCorpusBytes + 1];
        var ex1 = Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(oversized));
        Assert.Equal(StarlarkCompilerErrorCode.CorpusByteLimitExceeded, ex1.Code);

        // BOM rejection
        var bomBytes = new byte[] { 0xEF, 0xBB, 0xBF, (byte)'{', (byte)'}' };
        var ex2 = Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(bomBytes));
        Assert.Equal(StarlarkCompilerErrorCode.CorpusMalformedTransport, ex2.Code);

        // Invalid UTF-8
        var invalidUtf8 = new byte[] { (byte)'{', 0xFF, (byte)'}' };
        var ex3 = Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(invalidUtf8));
        Assert.Equal(StarlarkCompilerErrorCode.CorpusMalformedTransport, ex3.Code);

        // Lone high surrogate byte sequence (CESU-8 / invalid UTF-8)
        var highSurrogateBytes = new byte[] { (byte)'{', 0xED, 0xA0, 0x80, (byte)'}' };
        var exSurrogate = Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(highSurrogateBytes));
        Assert.Equal(StarlarkCompilerErrorCode.CorpusMalformedTransport, exSurrogate.Code);

        // Malformed JSON syntax
        var malformedJson = Encoding.UTF8.GetBytes("{broken");
        var exMalformed = Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(malformedJson));
        Assert.Equal(StarlarkCompilerErrorCode.CorpusMalformedTransport, exMalformed.Code);

        // Schema violations
        var notAnObject = Encoding.UTF8.GetBytes("[1, 2, 3]");
        Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(notAnObject));

        var wrongVersion = Encoding.UTF8.GetBytes("{\"schema_version\": 99, \"profile\": \"standalone-provider-v1\", \"opcodes\": [], \"decode\": []}");
        Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(wrongVersion));

        var wrongVersionType = Encoding.UTF8.GetBytes("{\"schema_version\": \"1\", \"profile\": \"standalone-provider-v1\", \"opcodes\": [], \"decode\": []}");
        var exWrongVersionType = Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(wrongVersionType));
        Assert.Equal(StarlarkCompilerErrorCode.CorpusSchemaViolation, exWrongVersionType.Code);

        var wrongProfile = Encoding.UTF8.GetBytes("{\"schema_version\": 1, \"profile\": \"wrong\", \"opcodes\": [], \"decode\": []}");
        Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(wrongProfile));

        var wrongProfileType = Encoding.UTF8.GetBytes("{\"schema_version\": 1, \"profile\": 123, \"opcodes\": [], \"decode\": []}");
        var exWrongProfileType = Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(wrongProfileType));
        Assert.Equal(StarlarkCompilerErrorCode.CorpusSchemaViolation, exWrongProfileType.Code);

        var wrongOpCount = Encoding.UTF8.GetBytes("{\"schema_version\": 1, \"profile\": \"standalone-provider-v1\", \"opcodes\": [{}], \"decode\": []}");
        Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(wrongOpCount));

        var dummyOps = new string[61];
        Array.Fill(dummyOps, "{}");
        var opsJson = string.Join(",", dummyOps);
        var wrongDecodeCount = Encoding.UTF8.GetBytes($"{{\"schema_version\": 1, \"profile\": \"standalone-provider-v1\", \"opcodes\": [{opsJson}], \"decode\": []}}");
        Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(wrongDecodeCount));
    }

    [Fact]
    public void SecurityShowcase_5ClassResilience()
    {
        // Class 1: Memory Exhaustion / Payload Size Defense
        var bomb = new byte[StarlarkCompilerConstants.MaxCorpusBytes + 10];
        Assert.Throws<StarlarkCompilerException>(() => StarlarkOpcodes.ValidateCorpusBytes(bomb));

        // Class 2: Stack Exhaustion Defense - Rapid 100k lookups without recursion
        for (var i = 0; i < 100_000; i++)
        {
            var op = StarlarkOpcodes.OpFromByte(i & 0xFF);
            if (op.HasValue)
            {
                Assert.NotNull(StarlarkOpcodes.OpSymbol(op.Value));
            }
        }

        // Class 3: Opcode Integrity & Byte Validation
        for (var b = -50; b <= 300; b++)
        {
            var op = StarlarkOpcodes.OpFromByte(b);
            if (b >= 0 && b <= 255)
            {
                if (op.HasValue)
                {
                    Assert.Equal((byte)b, StarlarkOpcodes.OpByte(op.Value));
                }
            }
            else
            {
                Assert.Null(op);
            }
        }

        // Class 4: Injection & Parsing Boundary Defense (Rule #20)
        var ex = StarlarkCompilerException.InvalidByte();
        Assert.Equal("starlark-compiler: invalid opcode byte value", ex.Message);
        Assert.Equal("invalid-byte", ex.ErrorId);

        // Class 5: State Mutation Defense
        var bin = StarlarkOpcodes.BinaryOpMap();
        bin.Clear();
        Assert.Equal(12, StarlarkOpcodes.BinaryOpMap().Count);
    }

    [Fact]
    public void Deterministic_200Case_PRNG_PropertyTest()
    {
        var rng = new Random(1337);
        for (var i = 0; i < 200; i++)
        {
            var val = rng.Next(-500, 500);
            var op = StarlarkOpcodes.OpFromByte(val);
            if (val is >= 0 and <= 255)
            {
                if (op.HasValue)
                {
                    Assert.Equal(val, StarlarkOpcodes.OpByte(op.Value));
                    Assert.Equal(op, StarlarkOpcodes.OpFromSymbol(StarlarkOpcodes.OpSymbol(op.Value)));
                }
            }
            else
            {
                Assert.Null(op);
            }
        }
    }

    [Fact]
    public void Adversarial_10000Iteration_MutationFuzzLoop()
    {
        var rng = new Random(42);
        var baseCorpus = Encoding.UTF8.GetBytes("{\"schema_version\": 1, \"profile\": \"standalone-provider-v1\"}");

        for (var iter = 0; iter < 10_000; iter++)
        {
            var mutated = (byte[])baseCorpus.Clone();
            var mutationType = rng.Next(0, 4);
            switch (mutationType)
            {
                case 0:
                    // Random byte flip
                    mutated[rng.Next(0, mutated.Length)] = (byte)rng.Next(0, 256);
                    break;
                case 1:
                    // Insert BOM or prefix
                    mutated = [0xEF, 0xBB, 0xBF, .. mutated];
                    break;
                case 2:
                    // Truncation
                    var newLen = rng.Next(0, mutated.Length);
                    var truncated = new byte[newLen];
                    Array.Copy(mutated, truncated, newLen);
                    mutated = truncated;
                    break;
                case 3:
                    // Append random bytes
                    var extra = new byte[rng.Next(1, 100)];
                    rng.NextBytes(extra);
                    mutated = [.. mutated, .. extra];
                    break;
            }

            // Must either pass or throw typed StarlarkCompilerException (never unhandled crash or external JsonException)
            try
            {
                StarlarkOpcodes.ValidateCorpusBytes(mutated);
            }
            catch (StarlarkCompilerException)
            {
                // Expected resilient rejection
            }
        }
    }
}
