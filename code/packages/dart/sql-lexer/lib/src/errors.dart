import 'messages.dart';

/// Closed enumeration of SQL lexer error codes.
enum SqlLexerErrorCode {
  invalidVersion(errorIdInvalidVersion),
  invalidSource(errorIdInvalidSource),
  lexicalError(errorIdLexicalError);

  const SqlLexerErrorCode(this.errorId);

  /// Stable machine-readable identifier for the error.
  final String errorId;
}

/// Sealed base exception for all SQL lexer operations.
///
/// Ensures Standing Rule #20 compliance: no caller-injected strings and no
/// payload reflection.
sealed class SqlLexerException implements Exception {
  const SqlLexerException(
    this.errorCode,
    this.message, {
    this.line,
    this.column,
  });

  /// The error code category.
  final SqlLexerErrorCode errorCode;

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

/// Thrown when an unknown or unsupported SQL edition version string is provided.
final class InvalidSqlVersionException extends SqlLexerException {
  const InvalidSqlVersionException()
      : super(SqlLexerErrorCode.invalidVersion, messageInvalidVersion);
}

/// Thrown when source code violates bounds or encoding invariants.
final class InvalidSqlSourceException extends SqlLexerException {
  const InvalidSqlSourceException._(String message)
      : super(SqlLexerErrorCode.invalidSource, message);

  /// Thrown when SQL source length exceeds the 1 MB ceiling.
  const InvalidSqlSourceException.sourceTooLarge()
      : this._(messageSourceTooLarge);

  /// Thrown when SQL source contains lone or reversed UTF-16 surrogates.
  const InvalidSqlSourceException.invalidSurrogates()
      : this._(messageInvalidSurrogates);
}

/// Thrown when tokenization encounters an unrecognized character or syntax violation.
final class SqlLexicalException extends SqlLexerException {
  const SqlLexicalException({int? line, int? column})
      : super(
          SqlLexerErrorCode.lexicalError,
          messageLexicalError,
          line: line,
          column: column,
        );
}
