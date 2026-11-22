namespace CodingAdventures.StarlarkCompiler.FSharp

open System

module Errors =
    /// Machine-readable error code enumeration.
    type StarlarkCompilerErrorCode =
        | InvalidByte = 0
        | UnknownOpcode = 1
        | UnknownOperator = 2
        | CorpusByteLimitExceeded = 3
        | CorpusMalformedTransport = 4
        | CorpusSchemaViolation = 5

    /// Sealed exception hierarchy for Starlark compiler operations.
    /// Complies with Standing Rule #20: closed sealed hierarchy without public string constructors.
    [<Sealed>]
    type StarlarkCompilerException internal (code: StarlarkCompilerErrorCode, errorId: string, message: string) =
        inherit Exception(message)
        member _.Code = code
        member _.ErrorId = errorId

    /// Creates an exception indicating an invalid or out-of-range opcode byte.
    let invalidByte () =
        StarlarkCompilerException(
            StarlarkCompilerErrorCode.InvalidByte,
            Constants.ErrorIdInvalidByte,
            Messages.MessageInvalidByte
        )

    /// Creates an exception indicating an unknown or unassigned opcode symbol.
    let unknownOpcode () =
        StarlarkCompilerException(
            StarlarkCompilerErrorCode.UnknownOpcode,
            Constants.ErrorIdUnknownOpcode,
            Messages.MessageUnknownOpcode
        )

    /// Creates an exception indicating an unknown operator string.
    let unknownOperator () =
        StarlarkCompilerException(
            StarlarkCompilerErrorCode.UnknownOperator,
            Constants.ErrorIdUnknownOperator,
            Messages.MessageUnknownOperator
        )

    /// Creates an exception indicating the corpus file size exceeds the upper bound.
    let corpusByteLimitExceeded () =
        StarlarkCompilerException(
            StarlarkCompilerErrorCode.CorpusByteLimitExceeded,
            Constants.ErrorIdInvalidCorpus,
            Messages.MessageCorpusByteLimitExceeded
        )

    /// Creates an exception indicating malformed transport encoding (e.g. BOM, non-UTF-8).
    let corpusMalformedTransport () =
        StarlarkCompilerException(
            StarlarkCompilerErrorCode.CorpusMalformedTransport,
            Constants.ErrorIdInvalidCorpus,
            Messages.MessageCorpusMalformedTransport
        )

    /// Creates an exception indicating corpus schema violation or data corruption.
    let corpusSchemaViolation () =
        StarlarkCompilerException(
            StarlarkCompilerErrorCode.CorpusSchemaViolation,
            Constants.ErrorIdInvalidCorpus,
            Messages.MessageCorpusSchemaViolation
        )
