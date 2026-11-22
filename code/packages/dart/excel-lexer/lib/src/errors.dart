import 'messages.dart';

/// Closed enumeration of Excel lexer error codes.
enum ExcelLexerErrorCode {
  invalidVersion(errorIdInvalidVersion),
  invalidSource(errorIdInvalidSource),
  lexicalError(errorIdLexicalError);

  const ExcelLexerErrorCode(this.errorId);

  /// Stable machine-readable identifier for the error.
  final String errorId;
}

/// Sealed base exception for all Excel lexer operations.
///
/// Ensures Standing Rule #20 compliance: no caller-injected strings and no
/// payload reflection.
sealed class ExcelLexerException implements Exception {
  const ExcelLexerException(
    this.errorCode,
    this.message, {
    this.line,
    this.column,
  });

  /// The error code category.
  final ExcelLexerErrorCode errorCode;

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

/// Thrown when an unknown or unsupported Excel edition version string is provided.
final class InvalidExcelVersionException extends ExcelLexerException {
  const InvalidExcelVersionException()
      : super(ExcelLexerErrorCode.invalidVersion, messageInvalidVersion);
}

/// Thrown when source code violates bounds or encoding invariants.
final class InvalidExcelSourceException extends ExcelLexerException {
  const InvalidExcelSourceException._(String message)
      : super(ExcelLexerErrorCode.invalidSource, message);

  /// Thrown when Excel source length exceeds the 1 MB ceiling.
  const InvalidExcelSourceException.sourceTooLarge()
      : this._(messageSourceTooLarge);

  /// Thrown when Excel source contains lone or reversed UTF-16 surrogates.
  const InvalidExcelSourceException.invalidSurrogates()
      : this._(messageInvalidSurrogates);
}

/// Thrown when tokenization encounters an unrecognized character or syntax violation.
final class ExcelLexicalException extends ExcelLexerException {
  const ExcelLexicalException({int? line, int? column})
      : super(
          ExcelLexerErrorCode.lexicalError,
          messageLexicalError,
          line: line,
          column: column,
        );
}
