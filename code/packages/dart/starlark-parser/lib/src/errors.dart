/// Strongly-typed, closed exception hierarchy for the Starlark parser.
///
/// Complies with Standing Rule #20:
/// - All exceptions inherit from a sealed base class [StarlarkParserException].
/// - Constructors are closed and accept no caller-supplied message strings.
/// - Error messages are drawn exclusively from [StarlarkParserMessages].
/// - Caller values and payloads are never reflected in exception fields or messages.
library;

import 'messages.dart';

/// Error codes identifying distinct diagnostic conditions in Starlark parsing.
enum StarlarkParserErrorCode {
  /// Provided version string is unmapped or unsupported.
  invalidVersion('invalid-version'),

  /// Input violated structural invariants (token ceiling, depth ceiling, syntax error).
  parseError('parse-error');

  const StarlarkParserErrorCode(this.id);

  /// Centralized kebab-case error identifier.
  final String id;
}

/// Sealed base class for all exceptions raised by the Starlark parser.
sealed class StarlarkParserException implements Exception {
  const StarlarkParserException._({
    required this.code,
    required this.message,
    this.line,
    this.column,
  });

  /// Canonical error code enumeration.
  final StarlarkParserErrorCode code;

  /// Centralized, localized error message string.
  final String message;

  /// 1-based source line where the error occurred, if known.
  final int? line;

  /// 1-based source column where the error occurred, if known.
  final int? column;

  @override
  String toString() {
    final loc = line != null ? ' at line $line, column $column' : '';
    return 'StarlarkParserException(${code.id})$loc: $message';
  }
}

/// Exception thrown when an unrecognized or unsupported Starlark parser edition is requested.
final class InvalidStarlarkParserVersionException
    extends StarlarkParserException {
  /// Creates an [InvalidStarlarkParserVersionException] with closed, centralized diagnostic message.
  const InvalidStarlarkParserVersionException()
      : super._(
          code: StarlarkParserErrorCode.invalidVersion,
          message: StarlarkParserMessages.invalidVersion,
        );
}

/// Exception thrown when Starlark token stream violates structural invariants.
final class StarlarkParseException extends StarlarkParserException {
  /// Thrown when token stream length exceeds the [maxTokenCount] boundary.
  const StarlarkParseException.tokenCountExceeded()
      : super._(
          code: StarlarkParserErrorCode.parseError,
          message: StarlarkParserMessages.tokenCountExceeded,
        );

  /// Thrown when nesting depth exceeds the [maxNestingDepth] ceiling.
  const StarlarkParseException.nestingTooDeep()
      : super._(
          code: StarlarkParserErrorCode.parseError,
          message: StarlarkParserMessages.nestingTooDeep,
        );

  /// Thrown when syntax analysis fails on an unexpected token sequence.
  const StarlarkParseException.syntaxError({super.line, super.column})
      : super._(
          code: StarlarkParserErrorCode.parseError,
          message: StarlarkParserMessages.syntaxError,
        );
}
