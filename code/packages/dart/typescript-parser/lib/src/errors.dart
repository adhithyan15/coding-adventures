import 'messages.dart';

/// Closed enumeration of TypeScript parser error codes.
enum TypescriptParserErrorCode {
  invalidVersion(errorIdInvalidVersion),
  tokenCountExceeded(errorIdTokenCountExceeded),
  nestingTooDeep(errorIdNestingTooDeep),
  syntaxError(errorIdSyntaxError);

  const TypescriptParserErrorCode(this.errorId);

  /// Stable machine-readable identifier for the error.
  final String errorId;
}

/// Sealed base exception for all TypeScript parser operations.
///
/// Ensures Standing Rule #20 compliance: no caller-injected strings and no
/// payload reflection.
sealed class TypescriptParserException implements Exception {
  const TypescriptParserException(
    this.errorCode,
    this.message, {
    this.line,
    this.column,
  });

  /// The error code category.
  final TypescriptParserErrorCode errorCode;

  /// The static, payload-blind explanation of the error.
  final String message;

  /// Optional 1-based line offset where the error occurred.
  final int? line;

  /// Optional 1-based column offset where the error occurred.
  final int? column;

  /// Stable machine-readable identifier matching [errorCode.errorId].
  String get errorId => errorCode.errorId;

  @override
  String toString() => formatLocation(message, line, column);
}

/// Thrown when an unknown or unsupported TypeScript version string is provided.
final class InvalidTypescriptParserVersionException
    extends TypescriptParserException {
  const InvalidTypescriptParserVersionException()
      : super(
          TypescriptParserErrorCode.invalidVersion,
          messageInvalidVersion,
        );
}

/// Thrown when parsing fails due to resource exhaustion or syntax violations.
final class TypescriptParseException extends TypescriptParserException {
  const TypescriptParseException._(
    TypescriptParserErrorCode code,
    String message, {
    int? line,
    int? column,
  }) : super(code, message, line: line, column: column);

  /// Thrown when source token stream exceeds [maxTokenCount].
  const TypescriptParseException.tokenCountExceeded()
      : this._(
          TypescriptParserErrorCode.tokenCountExceeded,
          messageTokenCountExceeded,
        );

  /// Thrown when delimiter nesting depth exceeds [maxNestingDepth].
  const TypescriptParseException.nestingTooDeep()
      : this._(
          TypescriptParserErrorCode.nestingTooDeep,
          messageNestingTooDeep,
        );

  /// Thrown when grammar parsing encounters an unexpected token or syntax violation.
  const TypescriptParseException.syntaxError({int? line, int? column})
      : this._(
          TypescriptParserErrorCode.syntaxError,
          messageSyntaxError,
          line: line,
          column: column,
        );
}
