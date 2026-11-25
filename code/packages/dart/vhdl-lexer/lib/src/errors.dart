import 'messages.dart';

/// Closed enumeration of VHDL lexer error codes.
enum VhdlLexerErrorCode {
  invalidVersion(errorIdInvalidVersion),
  invalidSource(errorIdInvalidSource),
  lexicalError(errorIdLexicalError);

  const VhdlLexerErrorCode(this.errorId);

  /// Stable machine-readable identifier for the error.
  final String errorId;
}

/// Sealed base exception for all VHDL lexer operations.
///
/// Ensures Standing Rule #20 compliance: no caller-injected strings and no
/// payload reflection.
sealed class VhdlLexerException implements Exception {
  const VhdlLexerException(
    this.errorCode,
    this.message, {
    this.line,
    this.column,
  });

  /// The error code category.
  final VhdlLexerErrorCode errorCode;

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

/// Thrown when an unknown or unsupported VHDL edition version string is provided.
final class InvalidVhdlVersionException extends VhdlLexerException {
  const InvalidVhdlVersionException()
      : super(VhdlLexerErrorCode.invalidVersion, messageInvalidVersion);
}

/// Thrown when source code violates bounds or encoding invariants.
final class InvalidVhdlSourceException extends VhdlLexerException {
  const InvalidVhdlSourceException._(String message)
      : super(VhdlLexerErrorCode.invalidSource, message);

  /// Thrown when VHDL source length exceeds the 1 MB ceiling.
  const InvalidVhdlSourceException.sourceTooLarge()
      : this._(messageSourceTooLarge);

  /// Thrown when VHDL source contains lone or reversed UTF-16 surrogates.
  const InvalidVhdlSourceException.invalidSurrogates()
      : this._(messageInvalidSurrogates);
}

/// Thrown when tokenization encounters an unrecognized character or syntax violation.
final class VhdlLexicalException extends VhdlLexerException {
  const VhdlLexicalException({int? line, int? column})
      : super(
          VhdlLexerErrorCode.lexicalError,
          messageLexicalError,
          line: line,
          column: column,
        );
}
