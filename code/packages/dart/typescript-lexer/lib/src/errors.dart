import 'messages.dart';

/// Closed enumeration of TypeScript lexer error codes.
enum TypescriptLexerErrorCode {
  invalidVersion(errorIdInvalidVersion),
  invalidSource(errorIdInvalidSource),
  lexicalError(errorIdLexicalError);

  const TypescriptLexerErrorCode(this.errorId);

  /// Stable machine-readable identifier for the error.
  final String errorId;
}

/// Sealed base exception for all TypeScript lexer operations.
///
/// Ensures Standing Rule #20 compliance: no caller-injected strings and no
/// payload reflection.
sealed class TypescriptLexerException implements Exception {
  const TypescriptLexerException(
    this.errorCode,
    this.message, {
    this.line,
    this.column,
  });

  /// The error code category.
  final TypescriptLexerErrorCode errorCode;

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
final class InvalidTypescriptVersionException extends TypescriptLexerException {
  const InvalidTypescriptVersionException()
      : super(
          TypescriptLexerErrorCode.invalidVersion,
          messageInvalidVersion,
        );
}

/// Thrown when source code violates bounds or encoding invariants.
final class InvalidTypescriptSourceException extends TypescriptLexerException {
  const InvalidTypescriptSourceException._(String message)
      : super(TypescriptLexerErrorCode.invalidSource, message);

  /// Thrown when TypeScript source length exceeds the 1 MB ceiling.
  const InvalidTypescriptSourceException.sourceTooLarge()
      : this._(messageSourceTooLarge);

  /// Thrown when TypeScript source contains lone or reversed UTF-16 surrogates.
  const InvalidTypescriptSourceException.invalidSurrogates()
      : this._(messageInvalidSurrogates);
}

/// Thrown when tokenization encounters an unrecognized character or syntax violation.
final class TypescriptLexicalException extends TypescriptLexerException {
  const TypescriptLexicalException({int? line, int? column})
      : super(
          TypescriptLexerErrorCode.lexicalError,
          messageLexicalError,
          line: line,
          column: column,
        );
}
