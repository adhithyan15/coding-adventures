using System;

namespace CodingAdventures.StarlarkCompiler;

/// <summary>
/// Error codes for Starlark compiler exceptions.
/// </summary>
public enum StarlarkCompilerErrorCode
{
    /// <summary>Invalid opcode byte.</summary>
    InvalidByte,

    /// <summary>Unknown opcode symbol.</summary>
    UnknownOpcode,

    /// <summary>Unknown operator string.</summary>
    UnknownOperator,

    /// <summary>Corpus size exceeded bound.</summary>
    CorpusByteLimitExceeded,

    /// <summary>Corpus transport encoding malformed.</summary>
    CorpusMalformedTransport,

    /// <summary>Corpus schema violation.</summary>
    CorpusSchemaViolation,
}

/// <summary>
/// Sealed exception hierarchy for Starlark compiler operations.
/// Complies with Standing Rule #20: closed sealed hierarchy without public string constructors.
/// </summary>
public sealed class StarlarkCompilerException : Exception
{
    private StarlarkCompilerException(StarlarkCompilerErrorCode code, string errorId, string message)
        : base(message)
    {
        Code = code;
        ErrorId = errorId;
    }

    /// <summary>Gets the machine-readable error code enum.</summary>
    public StarlarkCompilerErrorCode Code { get; }

    /// <summary>Gets the centralized stable error identifier.</summary>
    public string ErrorId { get; }

    /// <summary>Creates an exception indicating an invalid or out-of-range opcode byte.</summary>
    public static StarlarkCompilerException InvalidByte() =>
        new(
            StarlarkCompilerErrorCode.InvalidByte,
            StarlarkCompilerConstants.ErrorIdInvalidByte,
            StarlarkCompilerMessages.MessageInvalidByte
        );

    /// <summary>Creates an exception indicating an unknown or unassigned opcode symbol.</summary>
    public static StarlarkCompilerException UnknownOpcode() =>
        new(
            StarlarkCompilerErrorCode.UnknownOpcode,
            StarlarkCompilerConstants.ErrorIdUnknownOpcode,
            StarlarkCompilerMessages.MessageUnknownOpcode
        );

    /// <summary>Creates an exception indicating an unknown operator string.</summary>
    public static StarlarkCompilerException UnknownOperator() =>
        new(
            StarlarkCompilerErrorCode.UnknownOperator,
            StarlarkCompilerConstants.ErrorIdUnknownOperator,
            StarlarkCompilerMessages.MessageUnknownOperator
        );

    /// <summary>Creates an exception indicating the corpus file size exceeds the upper bound.</summary>
    public static StarlarkCompilerException CorpusByteLimitExceeded() =>
        new(
            StarlarkCompilerErrorCode.CorpusByteLimitExceeded,
            StarlarkCompilerConstants.ErrorIdInvalidCorpus,
            StarlarkCompilerMessages.MessageCorpusByteLimitExceeded
        );

    /// <summary>Creates an exception indicating malformed transport encoding (e.g. BOM, non-UTF-8).</summary>
    public static StarlarkCompilerException CorpusMalformedTransport() =>
        new(
            StarlarkCompilerErrorCode.CorpusMalformedTransport,
            StarlarkCompilerConstants.ErrorIdInvalidCorpus,
            StarlarkCompilerMessages.MessageCorpusMalformedTransport
        );

    /// <summary>Creates an exception indicating corpus schema violation or data corruption.</summary>
    public static StarlarkCompilerException CorpusSchemaViolation() =>
        new(
            StarlarkCompilerErrorCode.CorpusSchemaViolation,
            StarlarkCompilerConstants.ErrorIdInvalidCorpus,
            StarlarkCompilerMessages.MessageCorpusSchemaViolation
        );
}
