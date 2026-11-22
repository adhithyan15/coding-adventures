/// Strongly-typed, closed exception hierarchy for the Verilog lexical analyzer.
///
/// Complies with Standing Rule #20:
/// - All exceptions inherit from a sealed base class [VerilogLexerException].
/// - Constructors are closed and accept no caller-supplied message strings.
/// - Error messages are drawn exclusively from [VerilogLexerMessages].
/// - Caller values and payloads are never reflected in exception fields or messages.
library;

import 'messages.dart';

/// Error codes identifying distinct diagnostic conditions in Verilog lexing.
enum VerilogLexerErrorCode {
  /// Provided version string is unmapped or unsupported.
  invalidVersion('invalid-version'),

  /// Source code exceeds size or depth limits or contains ill-formed surrogates.
  invalidSource('invalid-source'),

  /// Lexer encountered an unexpected character sequence.
  lexicalError('lexical-error');

  const VerilogLexerErrorCode(this.id);

  /// Centralized kebab-case error identifier.
  final String id;
}

/// Sealed base class for all exceptions raised by the Verilog lexer.
sealed class VerilogLexerException implements Exception {
  const VerilogLexerException._({
    required this.code,
    required this.message,
    this.line,
    this.column,
  });

  /// Canonical error code enumeration.
  final VerilogLexerErrorCode code;

  /// Centralized, localized error message string.
  final String message;

  /// 1-based source line where the error occurred, if known.
  final int? line;

  /// 1-based source column where the error occurred, if known.
  final int? column;

  @override
  String toString() {
    final loc = line != null ? ' at line $line, column $column' : '';
    return 'VerilogLexerException(${code.id})$loc: $message';
  }
}

/// Exception thrown when an unrecognized or unsupported Verilog edition is requested.
final class InvalidVerilogVersionException extends VerilogLexerException {
  /// Creates an [InvalidVerilogVersionException] with closed, centralized diagnostic message.
  const InvalidVerilogVersionException()
      : super._(
          code: VerilogLexerErrorCode.invalidVersion,
          message: VerilogLexerMessages.invalidVersion,
        );
}

/// Exception thrown when Verilog source violates bounded size or character invariants.
final class InvalidVerilogSourceException extends VerilogLexerException {
  /// Thrown when source text exceeds the 1 MB size limit.
  const InvalidVerilogSourceException.sourceTooLarge()
      : super._(
          code: VerilogLexerErrorCode.invalidSource,
          message: VerilogLexerMessages.sourceTooLarge,
        );

  /// Thrown when source text contains malformed UTF-16 surrogates.
  const InvalidVerilogSourceException.invalidSurrogates()
      : super._(
          code: VerilogLexerErrorCode.invalidSource,
          message: VerilogLexerMessages.invalidSurrogates,
        );

  /// Thrown when preprocessor conditional nesting depth exceeds the maximum ceiling.
  const InvalidVerilogSourceException.conditionalDepthExceeded()
      : super._(
          code: VerilogLexerErrorCode.invalidSource,
          message: VerilogLexerMessages.conditionalDepthExceeded,
        );
}

/// Exception thrown when lexical tokenization fails.
final class VerilogLexicalException extends VerilogLexerException {
  /// Creates a [VerilogLexicalException] with optional coordinate positions.
  const VerilogLexicalException({super.line, super.column})
      : super._(
          code: VerilogLexerErrorCode.lexicalError,
          message: VerilogLexerMessages.lexicalError,
        );
}
