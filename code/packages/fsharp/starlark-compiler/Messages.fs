namespace CodingAdventures.StarlarkCompiler.FSharp

module Messages =
    /// Diagnostic message when a byte is out of range 0..255 or in an unassigned hole.
    [<Literal>]
    let MessageInvalidByte = "starlark-compiler: invalid opcode byte value"

    /// Diagnostic message when an opcode symbol is unknown or empty.
    [<Literal>]
    let MessageUnknownOpcode = "starlark-compiler: unknown opcode symbol"

    /// Diagnostic message when an operator string is unsupported.
    [<Literal>]
    let MessageUnknownOperator = "starlark-compiler: unknown operator string"

    /// Diagnostic message when a corpus file exceeds maximum byte size.
    [<Literal>]
    let MessageCorpusByteLimitExceeded = "starlark-compiler: corpus byte limit exceeded"

    /// Diagnostic message when corpus transport or encoding is invalid (e.g. BOM, bad UTF-8).
    [<Literal>]
    let MessageCorpusMalformedTransport = "starlark-compiler: malformed corpus transport encoding"

    /// Diagnostic message when corpus schema or contents are corrupted.
    [<Literal>]
    let MessageCorpusSchemaViolation = "starlark-compiler: corpus schema violation or data corruption"
