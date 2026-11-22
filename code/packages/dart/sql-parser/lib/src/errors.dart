import 'messages.dart';

/// Closed enumeration of SQL parser error codes.
enum SqlParserErrorCode {
  invalidVersion(errorIdInvalidVersion),
  tokenCountExceeded(errorIdTokenCountExceeded),
  nestingTooDeep(errorIdNestingTooDeep),
  syntaxError(errorIdSyntaxError);

  const SqlParserErrorCode(this.errorId);

  /// Stable machine-readable identifier for the error.
  final String errorId;
}

/// Sealed base exception for all SQL parser operations.
///
/// Ensures Standing Rule #20 compliance: no caller-injected strings and no
/// payload reflection.
sealed class SqlParserException implements Exception {
  const SqlParserException(
    this.errorCode,
    this.message, {
    this.line,
    this.column,
  });

  /// The error code category.
  final SqlParserErrorCode errorCode;

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

/// Thrown when an unknown or unsupported SQL version string is provided.
final class InvalidSqlParserVersionException extends SqlParserException {
  const InvalidSqlParserVersionException()
      : super(SqlParserErrorCode.invalidVersion, messageInvalidVersion);
}

/// Thrown when parsing fails due to bounds, nesting depth, or syntax violations.
final class SqlParseException extends SqlParserException {
  const SqlParseException._(
    SqlParserErrorCode code,
    String message, {
    int? line,
    int? column,
  }) : super(code, message, line: line, column: column);

  /// Thrown when input token count exceeds allowed ceiling.
  const SqlParseException.tokenCountExceeded()
      : this._(
            SqlParserErrorCode.tokenCountExceeded, messageTokenCountExceeded);

  /// Thrown when parenthetical nesting depth exceeds allowed ceiling.
  const SqlParseException.nestingTooDeep()
      : this._(SqlParserErrorCode.nestingTooDeep, messageNestingTooDeep);

  /// Thrown when grammar parsing encounters an unexpected token or syntax violation.
  const SqlParseException.syntaxError({int? line, int? column})
      : this._(
          SqlParserErrorCode.syntaxError,
          messageSyntaxError,
          line: line,
          column: column,
        );
}
