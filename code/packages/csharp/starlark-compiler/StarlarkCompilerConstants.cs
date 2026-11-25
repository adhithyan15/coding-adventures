namespace CodingAdventures.StarlarkCompiler;

/// <summary>
/// Centralized constants and bounds for Starlark opcode provider.
/// </summary>
public static class StarlarkCompilerConstants
{
    /// <summary>Maximum allowed byte size for corpus files to mitigate CWE-400.</summary>
    public const int MaxCorpusBytes = 131072;

    /// <summary>Canonical STP01 schema version.</summary>
    public const int ExpectedSchemaVersion = 1;

    /// <summary>Canonical standalone provider profile string.</summary>
    public const string ProfileStandaloneProviderV1 = "standalone-provider-v1";

    /// <summary>Total number of valid assigned opcodes in Starlark bytecode.</summary>
    public const int TotalOpcodeCount = 61;

    /// <summary>Total number of entries in the single-byte decode table (0x00..0xFF).</summary>
    public const int TotalDecodeEntries = 256;

    /// <summary>Total number of unassigned byte holes in the decode table.</summary>
    public const int TotalUnassignedHoles = 195;

    /// <summary>Total count of supported operator mappings.</summary>
    public const int TotalOperatorsCount = 34;

    /// <summary>Total count of opcode categories.</summary>
    public const int TotalCategoriesCount = 12;

    /// <summary>Count of binary operators.</summary>
    public const int TotalBinaryOpsCount = 12;

    /// <summary>Count of comparison operators.</summary>
    public const int TotalCompareOpsCount = 8;

    /// <summary>Count of augmented assignment operators.</summary>
    public const int TotalAugmentedAssignOpsCount = 12;

    /// <summary>Count of unary operators.</summary>
    public const int TotalUnaryOpsCount = 2;

    /// <summary>Bit shift to extract opcode category high nibble.</summary>
    public const int CategoryNibbleShift = 4;

    /// <summary>Length of UTF-8 Byte Order Mark in bytes.</summary>
    public const int Utf8BomLength = 3;

    /// <summary>First byte of UTF-8 BOM (0xEF).</summary>
    public const byte Utf8BomByte0 = 0xEF;

    /// <summary>Second byte of UTF-8 BOM (0xBB).</summary>
    public const byte Utf8BomByte1 = 0xBB;

    /// <summary>Third byte of UTF-8 BOM (0xBF).</summary>
    public const byte Utf8BomByte2 = 0xBF;

    /// <summary>JSON property name for schema_version.</summary>
    public const string PropSchemaVersion = "schema_version";

    /// <summary>JSON property name for profile.</summary>
    public const string PropProfile = "profile";

    /// <summary>JSON property name for opcodes.</summary>
    public const string PropOpcodes = "opcodes";

    /// <summary>JSON property name for decode.</summary>
    public const string PropDecode = "decode";

    /// <summary>Standard error ID for out-of-range or invalid byte value.</summary>
    public const string ErrorIdInvalidByte = "invalid-byte";

    /// <summary>Standard error ID for unassigned or unrecognized opcode symbol.</summary>
    public const string ErrorIdUnknownOpcode = "unknown-opcode";

    /// <summary>Standard error ID for unsupported operator symbol.</summary>
    public const string ErrorIdUnknownOperator = "unknown-operator";

    /// <summary>Standard error ID for corrupted corpus file or schema mismatch.</summary>
    public const string ErrorIdInvalidCorpus = "invalid-corpus";

    /// <summary>Redacted token placeholder for payload-blind reporting (Standing Rule #20).</summary>
    public const string Redacted = "<redacted>";

    /// <summary>Invalid token placeholder for payload-blind reporting (Standing Rule #20).</summary>
    public const string Invalid = "<invalid>";
}
