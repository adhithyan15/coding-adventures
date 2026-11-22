import 'messages.dart';

/// Closed enumeration of C# lexer error codes.
enum CSharpLexerErrorCode {
  invalidVersion(csharpLexerErrorInvalidVersion),
  invalidSource(csharpLexerErrorInvalidSource),
  lexicalError(csharpLexerErrorLexicalError);

  final String errorId;
  const CSharpLexerErrorCode(this.errorId);
}

/// Sealed base error type for C# lexer operations.
sealed class CSharpLexerException implements Exception {
  final CSharpLexerErrorCode errorCode;
  final String message;
  final int? line;
  final int? column;

  const CSharpLexerException(
    this.errorCode,
    this.message, {
    this.line,
    this.column,
  });

  String get errorId => errorCode.errorId;

  @override
  String toString() => formatLocation(message, line, column);
}

/// Thrown when an unknown or unsupported C# edition version string is provided.
final class InvalidCSharpVersionException extends CSharpLexerException {
  const InvalidCSharpVersionException()
      : super(CSharpLexerErrorCode.invalidVersion, messageInvalidVersion);
}

/// Thrown when source code violates bounds or encoding invariants (oversized or lone surrogates).
final class InvalidCSharpSourceException extends CSharpLexerException {
  const InvalidCSharpSourceException._(String message)
      : super(CSharpLexerErrorCode.invalidSource, message);

  /// Thrown when source length exceeds the 1 MB ceiling.
  const InvalidCSharpSourceException.sourceTooLarge()
      : this._(messageSourceTooLarge);

  /// Thrown when source contains unpaired high or low UTF-16 surrogates.
  const InvalidCSharpSourceException.invalidSurrogates()
      : this._(messageInvalidSurrogates);
}

/// Thrown when tokenization encounters unrecognized syntax or lexical violation.
final class CSharpLexicalException extends CSharpLexerException {
  const CSharpLexicalException({int? line, int? column})
      : super(
          CSharpLexerErrorCode.lexicalError,
          messageLexicalError,
          line: line,
          column: column,
        );
}
