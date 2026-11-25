import 'messages.dart';

/// Closed enumeration of VHDL parser error codes.
enum VhdlParserErrorCode {
  invalidVersion(errorIdInvalidVersion),
  tokenCountExceeded(errorIdTokenCountExceeded),
  nestingTooDeep(errorIdNestingTooDeep),
  syntaxError(errorIdSyntaxError);

  const VhdlParserErrorCode(this.errorId);

  /// Stable machine-readable identifier for the error.
  final String errorId;
}

/// Sealed base exception for all VHDL parser operations.
///
/// Ensures Standing Rule #20 compliance: no caller-injected strings and no
/// payload reflection.
sealed class VhdlParserException implements Exception {
  const VhdlParserException(
    this.errorCode,
    this.message, {
    this.line,
    this.column,
  });

  /// The error code category.
  final VhdlParserErrorCode errorCode;

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

/// Thrown when an unknown or unsupported VHDL version string is provided.
final class InvalidVhdlParserVersionException extends VhdlParserException {
  const InvalidVhdlParserVersionException()
      : super(VhdlParserErrorCode.invalidVersion, messageInvalidVersion);
}

/// Thrown when parsing fails due to bounds, nesting depth, or syntax violations.
final class VhdlParseException extends VhdlParserException {
  const VhdlParseException._(
    VhdlParserErrorCode code,
    String message, {
    int? line,
    int? column,
  }) : super(code, message, line: line, column: column);

  /// Thrown when input token count exceeds allowed ceiling.
  const VhdlParseException.tokenCountExceeded()
      : this._(
            VhdlParserErrorCode.tokenCountExceeded, messageTokenCountExceeded);

  /// Thrown when delimiter nesting depth exceeds allowed ceiling.
  const VhdlParseException.nestingTooDeep()
      : this._(VhdlParserErrorCode.nestingTooDeep, messageNestingTooDeep);

  /// Thrown when grammar parsing encounters an unexpected token or syntax violation.
  const VhdlParseException.syntaxError({int? line, int? column})
      : this._(
          VhdlParserErrorCode.syntaxError,
          messageSyntaxError,
          line: line,
          column: column,
        );
}
