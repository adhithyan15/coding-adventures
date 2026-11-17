import 'messages.dart';

/// Closed enumeration of Java lexer error codes.
enum JavaLexerErrorCode {
  invalidVersion(javaLexerErrorInvalidVersion),
  invalidSource(javaLexerErrorInvalidSource),
  lexicalError(javaLexerErrorLexicalError);

  final String errorId;
  const JavaLexerErrorCode(this.errorId);
}

/// Sealed base error type for Java lexer operations.
sealed class JavaLexerException implements Exception {
  final JavaLexerErrorCode errorCode;
  final String message;
  final int? line;
  final int? column;

  const JavaLexerException(
    this.errorCode,
    this.message, {
    this.line,
    this.column,
  });

  String get errorId => errorCode.errorId;

  @override
  String toString() => formatLocation(message, line, column);
}

/// Thrown when an unknown or unsupported Java edition version string is provided.
final class InvalidJavaVersionException extends JavaLexerException {
  const InvalidJavaVersionException()
      : super(JavaLexerErrorCode.invalidVersion, messageInvalidVersion);
}

/// Thrown when source code violates bounds or encoding invariants (oversized or lone surrogates).
final class InvalidJavaSourceException extends JavaLexerException {
  const InvalidJavaSourceException._(String message)
      : super(JavaLexerErrorCode.invalidSource, message);

  /// Thrown when source length exceeds the 1 MB ceiling.
  const InvalidJavaSourceException.sourceTooLarge()
      : this._(messageSourceTooLarge);

  /// Thrown when source contains unpaired high or low UTF-16 surrogates.
  const InvalidJavaSourceException.invalidSurrogates()
      : this._(messageInvalidSurrogates);
}

/// Thrown when tokenization encounters unrecognized syntax or lexical violation.
final class JavaLexicalException extends JavaLexerException {
  const JavaLexicalException({int? line, int? column})
      : super(
          JavaLexerErrorCode.lexicalError,
          messageLexicalError,
          line: line,
          column: column,
        );
}
