import 'messages.dart';

/// Closed enumeration of Verilog parser error codes.
enum VerilogParserErrorCode {
  invalidVersion(errorIdInvalidVersion),
  tokenCountExceeded(errorIdTokenCountExceeded),
  nestingTooDeep(errorIdNestingTooDeep),
  syntaxError(errorIdSyntaxError);

  const VerilogParserErrorCode(this.errorId);

  /// Stable machine-readable identifier for the error.
  final String errorId;
}

/// Sealed base exception for all Verilog parser operations.
///
/// Ensures Standing Rule #20 compliance: no caller-injected strings and no
/// payload reflection.
sealed class VerilogParserException implements Exception {
  const VerilogParserException(
    this.errorCode,
    this.message, {
    this.line,
    this.column,
  });

  /// The error code category.
  final VerilogParserErrorCode errorCode;

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

/// Thrown when an unknown or unsupported Verilog edition string is provided.
final class InvalidVerilogParserVersionException
    extends VerilogParserException {
  const InvalidVerilogParserVersionException()
      : super(
          VerilogParserErrorCode.invalidVersion,
          messageInvalidVersion,
        );
}

/// Thrown when parsing fails due to resource exhaustion or syntax violations.
final class VerilogParseException extends VerilogParserException {
  const VerilogParseException._(
    VerilogParserErrorCode code,
    String message, {
    int? line,
    int? column,
  }) : super(code, message, line: line, column: column);

  /// Thrown when source token stream exceeds [maxTokenCount].
  const VerilogParseException.tokenCountExceeded()
      : this._(
          VerilogParserErrorCode.tokenCountExceeded,
          messageTokenCountExceeded,
        );

  /// Thrown when delimiter, block, or prefix nesting depth exceeds [maxNestingDepth].
  const VerilogParseException.nestingTooDeep()
      : this._(
          VerilogParserErrorCode.nestingTooDeep,
          messageNestingTooDeep,
        );

  /// Thrown when grammar parsing encounters an unexpected token or syntax violation.
  const VerilogParseException.syntaxError({int? line, int? column})
      : this._(
          VerilogParserErrorCode.syntaxError,
          messageSyntaxError,
          line: line,
          column: column,
        );
}
