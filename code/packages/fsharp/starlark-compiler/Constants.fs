namespace CodingAdventures.StarlarkCompiler.FSharp

module Constants =
    /// Maximum allowed byte size for corpus files to mitigate CWE-400.
    [<Literal>]
    let MaxCorpusBytes = 131072

    /// Canonical STP01 schema version.
    [<Literal>]
    let ExpectedSchemaVersion = 1

    /// Canonical standalone provider profile string.
    [<Literal>]
    let ProfileStandaloneProviderV1 = "standalone-provider-v1"

    /// Total number of valid assigned opcodes in Starlark bytecode.
    [<Literal>]
    let TotalOpcodeCount = 61

    /// Total number of entries in the single-byte decode table (0x00..0xFF).
    [<Literal>]
    let TotalDecodeEntries = 256

    /// Total number of unassigned byte holes in the decode table.
    [<Literal>]
    let TotalUnassignedHoles = 195

    /// Total count of supported operator mappings.
    [<Literal>]
    let TotalOperatorsCount = 34

    /// Total count of opcode categories.
    [<Literal>]
    let TotalCategoriesCount = 12

    /// Count of binary operators.
    [<Literal>]
    let TotalBinaryOpsCount = 12

    /// Count of comparison operators.
    [<Literal>]
    let TotalCompareOpsCount = 8

    /// Count of augmented assignment operators.
    [<Literal>]
    let TotalAugmentedAssignOpsCount = 12

    /// Count of unary operators.
    [<Literal>]
    let TotalUnaryOpsCount = 2

    /// Bit shift to extract opcode category high nibble.
    [<Literal>]
    let CategoryNibbleShift = 4

    /// Length of UTF-8 Byte Order Mark in bytes.
    [<Literal>]
    let Utf8BomLength = 3

    /// First byte of UTF-8 BOM (0xEF).
    [<Literal>]
    let Utf8BomByte0 = 0xEFuy

    /// Second byte of UTF-8 BOM (0xBB).
    [<Literal>]
    let Utf8BomByte1 = 0xBBuy

    /// Third byte of UTF-8 BOM (0xBF).
    [<Literal>]
    let Utf8BomByte2 = 0xBFuy

    /// JSON property name for schema_version.
    [<Literal>]
    let PropSchemaVersion = "schema_version"

    /// JSON property name for profile.
    [<Literal>]
    let PropProfile = "profile"

    /// JSON property name for opcodes.
    [<Literal>]
    let PropOpcodes = "opcodes"

    /// JSON property name for decode.
    [<Literal>]
    let PropDecode = "decode"

    /// Standard error ID for out-of-range or invalid byte value.
    [<Literal>]
    let ErrorIdInvalidByte = "invalid-byte"

    /// Standard error ID for unassigned or unrecognized opcode symbol.
    [<Literal>]
    let ErrorIdUnknownOpcode = "unknown-opcode"

    /// Standard error ID for unsupported operator symbol.
    [<Literal>]
    let ErrorIdUnknownOperator = "unknown-operator"

    /// Standard error ID for corrupted corpus file or schema mismatch.
    [<Literal>]
    let ErrorIdInvalidCorpus = "invalid-corpus"

    /// Redacted token placeholder for payload-blind reporting (Standing Rule #20).
    [<Literal>]
    let Redacted = "<redacted>"

    /// Invalid token placeholder for payload-blind reporting (Standing Rule #20).
    [<Literal>]
    let Invalid = "<invalid>"
