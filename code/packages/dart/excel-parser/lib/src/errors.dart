import 'messages.dart';

/// Closed enumeration of Excel parser error codes.
enum ExcelParserErrorCode {
  invalidVersion(errorIdInvalidVersion),
  tokenCountExceeded(errorIdTokenCountExceeded),
  nestingTooDeep(errorIdNestingTooDeep),
  syntaxError(errorIdSyntaxError);

  const ExcelParserErrorCode(this.errorId);

  /// Stable machine-readable identifier for the error.
  final String errorId;
}

/// Sealed base exception for all Excel parser operations.
///
/// Ensures Standing Rule #20 compliance: no caller-injected strings and no
/// payload reflection.
sealed class ExcelParserException implements Exception {
  const ExcelParserException(
    this.errorCode,
    this.message, {
    this.line,
    this.column,
  });

  /// The error code category.
  final ExcelParserErrorCode errorCode;

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

/// Thrown when an unknown or unsupported Excel version string is provided.
final class InvalidExcelParserVersionException extends ExcelParserException {
  const InvalidExcelParserVersionException()
      : super(ExcelParserErrorCode.invalidVersion, messageInvalidVersion);
}

/// Thrown when parsing fails due to bounds, nesting depth, or syntax violations.
final class ExcelParseException extends ExcelParserException {
  const ExcelParseException._(
    ExcelParserErrorCode code,
    String message, {
    int? line,
    int? column,
  }) : super(code, message, line: line, column: column);

  /// Thrown when input token count exceeds allowed ceiling.
  const ExcelParseException.tokenCountExceeded()
      : this._(
          ExcelParserErrorCode.tokenCountExceeded,
          messageTokenCountExceeded,
        );

  /// Thrown when delimiter nesting depth exceeds allowed ceiling.
  const ExcelParseException.nestingTooDeep()
      : this._(ExcelParserErrorCode.nestingTooDeep, messageNestingTooDeep);

  /// Thrown when grammar parsing encounters an unexpected token or syntax violation.
  const ExcelParseException.syntaxError({int? line, int? column})
      : this._(
          ExcelParserErrorCode.syntaxError,
          messageSyntaxError,
          line: line,
          column: column,
        );
}
