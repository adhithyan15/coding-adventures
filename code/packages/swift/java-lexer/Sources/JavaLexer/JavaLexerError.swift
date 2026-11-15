// ============================================================================
// JavaLexerError.swift — Error types for Java lexical analysis.
// ============================================================================

import Foundation

/// Closed enumeration of Java lexer error codes.
public enum JavaLexerErrorCode: Sendable, Equatable {
  case invalidVersion
  case invalidSource
  case lexicalError

  public var errorId: String {
    JavaLexerMessages.errorId(for: self)
  }
}

/// Structured error thrown during Java lexical analysis.
public struct JavaLexerError: Error, Sendable, Equatable, CustomStringConvertible {
  public let code: JavaLexerErrorCode
  public let message: String
  public let line: Int?
  public let column: Int?

  public var errorId: String {
    code.errorId
  }

  private init(code: JavaLexerErrorCode, message: String, line: Int? = nil, column: Int? = nil) {
    self.code = code
    self.message = message
    self.line = line
    self.column = column
  }

  /// Public constructor accepting only closed error codes and optional source location.
  /// Diagnostic message text is strictly resolved from static messages.
  public init(code: JavaLexerErrorCode, line: Int? = nil, column: Int? = nil) {
    self.init(
      code: code,
      message: JavaLexerMessages.message(for: code),
      line: line,
      column: column
    )
  }

  public static func invalidVersion() -> JavaLexerError {
    JavaLexerError(code: .invalidVersion)
  }

  public static func sourceTooLarge() -> JavaLexerError {
    JavaLexerError(code: .invalidSource)
  }

  public static func lexicalError(line: Int? = nil, column: Int? = nil) -> JavaLexerError {
    JavaLexerError(code: .lexicalError, line: line, column: column)
  }

  public var description: String {
    JavaLexerMessages.formatLocation(message: message, line: line, column: column)
  }
}
