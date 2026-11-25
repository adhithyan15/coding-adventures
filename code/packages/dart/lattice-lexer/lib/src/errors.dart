import 'messages.dart';

/// Closed enumeration of Lattice lexer error codes.
enum LatticeLexerErrorCode {
  invalidVersion(errorIdInvalidVersion),
  invalidSource(errorIdInvalidSource),
  lexicalError(errorIdLexicalError);

  const LatticeLexerErrorCode(this.errorId);

  /// Stable machine-readable identifier for the error.
  final String errorId;
}

/// Sealed base exception for all Lattice lexer operations.
///
/// Ensures Standing Rule #20 compliance: no caller-injected strings and no
/// payload reflection.
sealed class LatticeLexerException implements Exception {
  const LatticeLexerException(
    this.errorCode,
    this.message, {
    this.line,
    this.column,
  });

  /// The error code category.
  final LatticeLexerErrorCode errorCode;

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

/// Thrown when an unknown or unsupported Lattice edition version string is provided.
final class InvalidLatticeVersionException extends LatticeLexerException {
  const InvalidLatticeVersionException()
      : super(LatticeLexerErrorCode.invalidVersion, messageInvalidVersion);
}

/// Thrown when source code violates bounds or encoding invariants.
final class InvalidLatticeSourceException extends LatticeLexerException {
  const InvalidLatticeSourceException._(String message)
      : super(LatticeLexerErrorCode.invalidSource, message);

  /// Thrown when Lattice source length exceeds the 1 MB ceiling.
  const InvalidLatticeSourceException.sourceTooLarge()
      : this._(messageSourceTooLarge);

  /// Thrown when Lattice source contains lone or reversed UTF-16 surrogates.
  const InvalidLatticeSourceException.invalidSurrogates()
      : this._(messageInvalidSurrogates);
}

/// Thrown when tokenization encounters an unrecognized character or syntax violation.
final class LatticeLexicalException extends LatticeLexerException {
  const LatticeLexicalException({int? line, int? column})
      : super(
          LatticeLexerErrorCode.lexicalError,
          messageLexicalError,
          line: line,
          column: column,
        );
}
