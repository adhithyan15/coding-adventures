// ============================================================================
// JavaLexerMessages.swift — Diagnostic messages for Java lexical analysis.
// Strictly payload-blind per Standing Rule #20.
// ============================================================================

import Foundation

public enum JavaLexerMessages {
  public static let errorIdInvalidVersion = "invalid-version"
  public static let errorIdInvalidSource = "invalid-source"
  public static let errorIdLexicalError = "lexical-error"

  public static let messageInvalidVersion = "Unknown or unsupported Java version"
  public static let messageSourceTooLarge = "Java source exceeds maximum allowed size"
  public static let messageLexicalError = "Lexical analysis failed due to unrecognized token"

  public static let redactedPlaceholder = "<redacted>"
  public static let invalidPlaceholder = "<invalid>"

  public static func errorId(for code: JavaLexerErrorCode) -> String {
    switch code {
    case .invalidVersion:
      return errorIdInvalidVersion
    case .invalidSource:
      return errorIdInvalidSource
    case .lexicalError:
      return errorIdLexicalError
    }
  }

  /// Static message for each closed error code.
  ///
  /// `invalidSource` maps to the size-ceiling message in both lanes. Swift `String` cannot
  /// hold lone UTF-16 surrogates, so the surrogate variant only exists in the Dart lane,
  /// where it is raised by its own named constructor.
  public static func message(for code: JavaLexerErrorCode) -> String {
    switch code {
    case .invalidVersion:
      return messageInvalidVersion
    case .invalidSource:
      return messageSourceTooLarge
    case .lexicalError:
      return messageLexicalError
    }
  }

  public static func formatLocation(message: String, line: Int?, column: Int?) -> String {
    guard let line = line, let column = column else {
      return message
    }
    return "\(message) (\(line):\(column))"
  }
}
