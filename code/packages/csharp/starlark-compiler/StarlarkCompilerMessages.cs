namespace CodingAdventures.StarlarkCompiler;

/// <summary>
/// Static, centralized localizable diagnostic message strings.
/// Complies with Standing Rule #20: 100% centralized error catalog without payload reflection.
/// </summary>
public static class StarlarkCompilerMessages
{
    /// <summary>Diagnostic message when a byte is out of range 0..255 or in an unassigned hole.</summary>
    public const string MessageInvalidByte = "starlark-compiler: invalid opcode byte value";

    /// <summary>Diagnostic message when an opcode symbol is unknown or empty.</summary>
    public const string MessageUnknownOpcode = "starlark-compiler: unknown opcode symbol";

    /// <summary>Diagnostic message when an operator string is unsupported.</summary>
    public const string MessageUnknownOperator = "starlark-compiler: unknown operator string";

    /// <summary>Diagnostic message when a corpus file exceeds maximum byte size.</summary>
    public const string MessageCorpusByteLimitExceeded = "starlark-compiler: corpus byte limit exceeded";

    /// <summary>Diagnostic message when corpus transport or encoding is invalid (e.g. BOM, bad UTF-8).</summary>
    public const string MessageCorpusMalformedTransport = "starlark-compiler: malformed corpus transport encoding";

    /// <summary>Diagnostic message when corpus schema or contents are corrupted.</summary>
    public const string MessageCorpusSchemaViolation = "starlark-compiler: corpus schema violation or data corruption";
}
