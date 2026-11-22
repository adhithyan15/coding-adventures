/// Strongly-typed, closed exception hierarchy for the Starlark lexical analyzer.
///
/// Complies with Standing Rule #20:
/// - All exceptions inherit from a sealed base class [StarlarkLexerException].
/// - Constructors are closed and accept no caller-supplied message strings.
/// - Error messages are drawn exclusively from [StarlarkLexerMessages].
/// - Caller values and payloads are never reflected in exception fields or messages.
library;

import 'messages.dart';

/// Error codes identifying distinct diagnostic conditions in Starlark lexing.
enum StarlarkLexerErrorCode {
  /// Provided version string is unmapped or unsupported.
  invalidVersion('invalid-version'),

  /// Source code exceeds size limits or contains ill-formed surrogates.
  invalidSource('invalid-source'),

  /// Lexer encountered an unexpected character sequence or indentation error.
  lexicalError('lexical-error');

  const StarlarkLexerErrorCode(this.id);

  /// Centralized kebab-case error identifier.
  final String id;
}

/// Sealed base class for all exceptions raised by the Starlark lexer.
sealed class StarlarkLexerException implements Exception {
  const StarlarkLexerException._({
    required this.code,
    required this.message,
    this.line,
    this.column,
  });

  /// Canonical error code enumeration.
  final StarlarkLexerErrorCode code;

  /// Centralized, localized error message string.
  final String message;

  /// 1-based source line where the error occurred, if known.
  final int? line;

  /// 1-based source column where the error occurred, if known.
  final int? column;

  @override
  String toString() {
    final loc = line != null ? ' at line $line, column $column' : '';
    return 'StarlarkLexerException(${code.id})$loc: $message';
  }
}

/// Exception thrown when an unrecognized or unsupported Starlark edition is requested.
final class InvalidStarlarkVersionException extends StarlarkLexerException {
  /// Creates an [InvalidStarlarkVersionException] with closed, centralized diagnostic message.
  const InvalidStarlarkVersionException()
      : super._(
          code: StarlarkLexerErrorCode.invalidVersion,
          message: StarlarkLexerMessages.invalidVersion,
        );
}

/// Exception thrown when Starlark source violates bounded size or character invariants.
final class InvalidStarlarkSourceException extends StarlarkLexerException {
  /// Thrown when source text exceeds the 1 MB size limit.
  const InvalidStarlarkSourceException.sourceTooLarge()
      : super._(
          code: StarlarkLexerErrorCode.invalidSource,
          message: StarlarkLexerMessages.sourceTooLarge,
        );

  /// Thrown when source text contains malformed UTF-16 surrogates.
  const InvalidStarlarkSourceException.invalidSurrogates()
      : super._(
          code: StarlarkLexerErrorCode.invalidSource,
          message: StarlarkLexerMessages.invalidSurrogates,
        );
}

/// Exception thrown when lexical tokenization fails.
final class StarlarkLexicalException extends StarlarkLexerException {
  /// Creates a [StarlarkLexicalException] with optional coordinate positions.
  const StarlarkLexicalException({super.line, super.column})
      : super._(
          code: StarlarkLexerErrorCode.lexicalError,
          message: StarlarkLexerMessages.lexicalError,
        );
}
