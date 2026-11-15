// ============================================================================
// JavaLexer.swift — Grammar-driven Java lexical analyzer.
// ============================================================================

import Foundation
import GrammarTools
import Lexer

/// Configured grammar-driven Java lexer instance.
///
/// Literate why — JLS Lexical Grammar vs Parser Responsibilities:
/// Under Java Language Specifications (JLS §3.9), contextual keywords such as `var`
/// (Java 10+), `yield` in statements, and `sealed` / `permits` (Java 17+) are classified
/// as restricted identifiers or restricted keywords. At the lexical grammar layer,
/// `var`, `sealed`, and `permits` are lexed as standard `NAME` tokens; their semantic
/// roles as modifiers or type names are disambiguated downstream in syntactic analysis.
public struct JavaLexer: Sendable {
  public let source: String
  public let version: String
  public let grammar: TokenGrammar

  /// Resolves the caller-provided version string or alias into its canonical edition string.
  ///
  /// Literate why — Alias Table Resolution:
  /// Using a pre-defined alias dictionary eliminates ad-hoc string slicing and magic
  /// numbers, guaranteeing cross-lane parity in version handling across Dart and Swift.
  public static func resolveVersion(_ version: String?) throws -> String {
    let lookup: String
    if let version = version, !version.isEmpty {
      lookup = version
    } else {
      lookup = JavaLexerConstants.defaultVersion
    }
    guard let canonical = JavaLexerConstants.versionAliases[lookup] else {
      throw JavaLexerError.invalidVersion()
    }
    return canonical
  }

  /// Loads the embedded TokenGrammar for the requested Java edition.
  ///
  /// Literate why — Authority-free embedded grammars:
  /// Precompiled grammars in `_Grammar.swift` eliminate all runtime filesystem access
  /// and dynamic file parsing, providing guaranteed hermeticity and deterministic performance.
  public static func loadGrammar(version: String? = nil) throws -> TokenGrammar {
    let canonical = try resolveVersion(version)
    guard let label = JavaLexerConstants.grammarKeys[canonical],
      let grammar = EmbeddedGrammar.tokenGrammars[label]
    else {
      throw JavaLexerError.invalidVersion()
    }
    return grammar
  }

  /// Initializes a JavaLexer after validating source bounds and resolving the grammar edition.
  ///
  /// Literate why — Validation Precedence Order:
  /// Source bounds checking precedes version resolution across all lanes (CWE-400 mitigation),
  /// ensuring oversized inputs fail closed before entering version parsing or grammar tables.
  ///
  /// Literate why — UTF-16 code units as the ceiling unit:
  /// The ceiling counts UTF-16 code units (`source.utf16.count`) so that Swift accepts and
  /// rejects exactly the same inputs as the Dart lane, whose `String.length` is UTF-16 based.
  /// Counting UTF-8 bytes here would reject non-ASCII sources that Dart accepts.
  public init(source: String, version: String? = nil) throws {
    if source.utf16.count > JavaLexerConstants.maxSourceLength {
      throw JavaLexerError.sourceTooLarge()
    }
    let canonical = try Self.resolveVersion(version)
    let grammar = try Self.loadGrammar(version: canonical)
    self.source = source
    self.version = canonical
    self.grammar = grammar
  }

  /// Tokenizes the configured Java source into a list of Tokens.
  public func tokenize() throws -> [Token] {
    let lexer = GrammarLexer(source: source, grammar: grammar)
    do {
      return try lexer.tokenize()
    } catch let err as LexerError {
      throw JavaLexerError.lexicalError(line: err.line, column: err.column)
    } catch {
      throw JavaLexerError.lexicalError()
    }
  }

  /// Tokenizes Java source code directly using the requested version.
  public static func tokenize(_ source: String, version: String? = nil) throws -> [Token] {
    let lexer = try JavaLexer(source: source, version: version)
    return try lexer.tokenize()
  }
}

/// Convenience top-level function matching the universal grammar lexer API.
public func tokenizeJava(_ source: String, version: String? = nil) throws -> [Token] {
  try JavaLexer.tokenize(source, version: version)
}

/// Convenience top-level function to instantiate a JavaLexer.
public func createJavaLexer(_ source: String, version: String? = nil) throws -> JavaLexer {
  try JavaLexer(source: source, version: version)
}

/// Convenience top-level function to load an embedded TokenGrammar for Java.
public func loadJavaTokenGrammar(version: String? = nil) throws -> TokenGrammar {
  try JavaLexer.loadGrammar(version: version)
}
