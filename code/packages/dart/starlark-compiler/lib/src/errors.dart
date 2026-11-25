/// Strongly-typed, closed exception hierarchy for the Starlark opcode provider.
///
/// Complies with Standing Rule #20:
/// - All exceptions inherit from a sealed base class [StarlarkCompilerException].
/// - Constructors are closed and accept no caller-supplied message strings.
/// - Error messages are drawn exclusively from [StarlarkCompilerMessages].
/// - Caller values and payloads are never reflected in exception fields or messages.
library;

import 'messages.dart';

/// Error codes identifying distinct diagnostic conditions in Starlark compilation.
enum StarlarkCompilerErrorCode {
  /// Provided integer is outside the 0..255 byte domain.
  invalidByte('invalid-byte'),

  /// Opcode byte is unassigned or unrecognized.
  unknownOpcode('unknown-opcode'),

  /// Operator string does not map to any opcode.
  unknownOperator('unknown-operator'),

  /// Corpus data violates size, transport, or schema invariants.
  invalidCorpus('invalid-corpus');

  const StarlarkCompilerErrorCode(this.id);

  /// Centralized kebab-case error identifier.
  final String id;
}

/// Sealed base class for all exceptions raised by this package.
sealed class StarlarkCompilerException implements Exception {
  const StarlarkCompilerException._({
    required this.code,
    required this.message,
  });

  /// Canonical error code enumeration.
  final StarlarkCompilerErrorCode code;

  /// Centralized, localized error message string.
  final String message;

  @override
  String toString() => 'StarlarkCompilerException(${code.id}): $message';
}

/// Exception thrown when an invalid byte value is supplied to an operation.
final class InvalidByteException extends StarlarkCompilerException {
  /// Creates an [InvalidByteException] with closed, centralized diagnostic message.
  const InvalidByteException()
      : super._(
          code: StarlarkCompilerErrorCode.invalidByte,
          message: StarlarkCompilerMessages.invalidByte,
        );
}

/// Exception thrown when a byte does not correspond to an assigned Starlark opcode.
final class UnknownOpcodeException extends StarlarkCompilerException {
  /// Creates an [UnknownOpcodeException] with closed, centralized diagnostic message.
  const UnknownOpcodeException()
      : super._(
          code: StarlarkCompilerErrorCode.unknownOpcode,
          message: StarlarkCompilerMessages.unknownOpcode,
        );
}

/// Exception thrown when an operator string does not map to any Starlark opcode.
final class UnknownOperatorException extends StarlarkCompilerException {
  /// Creates an [UnknownOperatorException] with closed, centralized diagnostic message.
  const UnknownOperatorException()
      : super._(
          code: StarlarkCompilerErrorCode.unknownOperator,
          message: StarlarkCompilerMessages.unknownOperator,
        );
}

/// Exception thrown when a test corpus violates bounded transport or schema invariants.
final class InvalidCorpusException extends StarlarkCompilerException {
  /// Thrown when corpus exceeds the 128 KB byte ceiling.
  const InvalidCorpusException.byteLimitExceeded()
      : super._(
          code: StarlarkCompilerErrorCode.invalidCorpus,
          message: StarlarkCompilerMessages.corpusByteLimitExceeded,
        );

  /// Thrown when corpus JSON syntax, Unicode escapes, or delimiters are malformed.
  const InvalidCorpusException.malformedTransport()
      : super._(
          code: StarlarkCompilerErrorCode.invalidCorpus,
          message: StarlarkCompilerMessages.corpusMalformedTransport,
        );

  /// Thrown when corpus keys, row counts, or structural values violate schema rules.
  const InvalidCorpusException.schemaViolation()
      : super._(
          code: StarlarkCompilerErrorCode.invalidCorpus,
          message: StarlarkCompilerMessages.corpusSchemaViolation,
        );
}
