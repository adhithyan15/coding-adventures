import 'messages.dart';

/// Closed enumeration of Lattice parser error codes.
enum LatticeParserErrorCode {
  invalidVersion(errorIdInvalidVersion),
  tokenCountExceeded(errorIdTokenCountExceeded),
  nestingTooDeep(errorIdNestingTooDeep),
  syntaxError(errorIdSyntaxError);

  const LatticeParserErrorCode(this.errorId);

  /// Stable machine-readable identifier for the error.
  final String errorId;
}

/// Sealed base exception for all Lattice parser operations.
///
/// Ensures Standing Rule #20 compliance: no caller-injected strings and no
/// payload reflection.
sealed class LatticeParserException implements Exception {
  const LatticeParserException(
    this.errorCode,
    this.message, {
    this.line,
    this.column,
  });

  /// The error code category.
  final LatticeParserErrorCode errorCode;

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

/// Thrown when an unknown or unsupported Lattice version string is provided.
final class InvalidLatticeParserVersionException
    extends LatticeParserException {
  const InvalidLatticeParserVersionException()
      : super(LatticeParserErrorCode.invalidVersion, messageInvalidVersion);
}

/// Thrown when parsing fails due to bounds, nesting depth, or syntax violations.
final class LatticeParseException extends LatticeParserException {
  const LatticeParseException._(
    LatticeParserErrorCode code,
    String message, {
    int? line,
    int? column,
  }) : super(code, message, line: line, column: column);

  /// Thrown when input token count exceeds allowed ceiling.
  const LatticeParseException.tokenCountExceeded()
      : this._(
          LatticeParserErrorCode.tokenCountExceeded,
          messageTokenCountExceeded,
        );

  /// Thrown when delimiter nesting depth exceeds allowed ceiling.
  const LatticeParseException.nestingTooDeep()
      : this._(LatticeParserErrorCode.nestingTooDeep, messageNestingTooDeep);

  /// Thrown when grammar parsing encounters an unexpected token or syntax violation.
  const LatticeParseException.syntaxError({int? line, int? column})
      : this._(
          LatticeParserErrorCode.syntaxError,
          messageSyntaxError,
          line: line,
          column: column,
        );
}
