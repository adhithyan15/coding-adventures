// ============================================================================
// JavaLexerConstants.swift — Constants for Java lexical analysis.
// ============================================================================

import Foundation

public enum JavaLexerConstants {
  public static let packageVersion = "0.1.0"
  public static let defaultVersion = "21"

  /// Canonical Java language editions supported by the lexer.
  public static let canonicalVersions: [String] = [
    "1.0", "1.1", "1.4", "5", "7", "8", "10", "14", "17", "21",
  ]

  /// Lookup table mapping caller version aliases to canonical edition strings.
  /// Eliminates string slicing and inline magic literals in operational logic.
  public static let versionAliases: [String: String] = [
    "1.0": "1.0",
    "java1.0": "1.0",
    "java1_0": "1.0",
    "1_0": "1.0",
    "1.1": "1.1",
    "java1.1": "1.1",
    "java1_1": "1.1",
    "1_1": "1.1",
    "1.4": "1.4",
    "java1.4": "1.4",
    "java1_4": "1.4",
    "1_4": "1.4",
    "5": "5",
    "java5": "5",
    "1.5": "5",
    "java1.5": "5",
    "java1_5": "5",
    "1_5": "5",
    "7": "7",
    "java7": "7",
    "1.7": "7",
    "java1.7": "7",
    "java1_7": "7",
    "1_7": "7",
    "8": "8",
    "java8": "8",
    "1.8": "8",
    "java1.8": "8",
    "java1_8": "8",
    "1_8": "8",
    "10": "10",
    "java10": "10",
    "14": "14",
    "java14": "14",
    "17": "17",
    "java17": "17",
    "21": "21",
    "java21": "21",
  ]

  /// Mapping from canonical edition string to EmbeddedGrammar registry label.
  public static let grammarKeys: [String: String] = [
    "1.0": "java1_0",
    "1.1": "java1_1",
    "1.4": "java1_4",
    "5": "java5",
    "7": "java7",
    "8": "java8",
    "10": "java10",
    "14": "java14",
    "17": "java17",
    "21": "java21",
  ]

  /// Source ceiling: 1,048,576 UTF-16 code units (CWE-400 mitigation).
  /// Measured in UTF-16 code units so the Dart and Swift lanes accept exactly the same inputs.
  public static let maxSourceLength = 1_048_576
  public static let maxFixtureBytes = 2_097_152  // 2 MB fixture boundary
}
