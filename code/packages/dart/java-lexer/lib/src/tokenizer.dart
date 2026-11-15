import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar_1_0.dart' as g1_0;
import '_grammar_1_1.dart' as g1_1;
import '_grammar_1_4.dart' as g1_4;
import '_grammar_5.dart' as g5;
import '_grammar_7.dart' as g7;
import '_grammar_8.dart' as g8;
import '_grammar_10.dart' as g10;
import '_grammar_14.dart' as g14;
import '_grammar_17.dart' as g17;
import '_grammar_21.dart' as g21;
import 'constants.dart';
import 'errors.dart';

/// Precompiled Java token grammars keyed by canonical edition string.
///
/// Literate why — Authority-free embedded grammars:
/// Embedding pre-parsed TokenGrammar objects directly into compiled code guarantees
/// 100% Hermetic execution with zero runtime filesystem reads or dynamic file resolution,
/// completely isolating lexical analysis from disk I/O, path traversal, or race conditions.
final Map<String, TokenGrammar> _versionedGrammars = {
  '1.0': g1_0.tokenGrammar,
  '1.1': g1_1.tokenGrammar,
  '1.4': g1_4.tokenGrammar,
  '5': g5.tokenGrammar,
  '7': g7.tokenGrammar,
  '8': g8.tokenGrammar,
  '10': g10.tokenGrammar,
  '14': g14.tokenGrammar,
  '17': g17.tokenGrammar,
  '21': g21.tokenGrammar,
};

/// Resolves the caller-provided Java edition version string into a canonical version.
/// Defaults to '21' when [version] is null or empty.
///
/// Aliases (e.g. 'java1.0', 'java1_0', '1.5', 'java21') are resolved via [javaVersionAliases].
String resolveJavaVersion([String? version]) {
  if (version == null || version.isEmpty) {
    return defaultJavaVersion;
  }
  final canonical = javaVersionAliases[version];
  if (canonical == null) {
    throw const InvalidJavaVersionException();
  }
  return canonical;
}

/// Returns the embedded TokenGrammar for the requested Java version.
TokenGrammar loadJavaTokenGrammar({String? version}) {
  final resolved = resolveJavaVersion(version);
  return _versionedGrammars[resolved]!;
}

/// Validates source code bounds and UTF-16 surrogates to prevent DoS and ReDoS.
///
/// Literate why — Fail-Closed Pre-Tokenization Security Checks:
/// 1. CWE-400: Limiting input to [maxSourceLength] (1 MB) prevents memory exhaustion
///    and regex execution amplification on uncontrolled source strings.
/// 2. CWE-1333 / Smuggling: Validating UTF-16 surrogate pairs prevents lone surrogates
///    from corrupting character offsets or exploiting regex engine surrogate handling.
void validateSource(String source) {
  if (source.length > maxSourceLength) {
    throw const InvalidJavaSourceException.sourceTooLarge();
  }
  final units = source.codeUnits;
  for (var i = 0; i < units.length; i++) {
    final unit = units[i];
    if (unit >= highSurrogateMin && unit <= highSurrogateMax) {
      if (++i >= units.length ||
          units[i] < lowSurrogateMin ||
          units[i] > lowSurrogateMax) {
        throw const InvalidJavaSourceException.invalidSurrogates();
      }
    } else if (unit >= lowSurrogateMin && unit <= lowSurrogateMax) {
      throw const InvalidJavaSourceException.invalidSurrogates();
    }
  }
}

/// A configured grammar-driven Java lexer.
///
/// Literate why — JLS Lexical Grammar vs Parser Responsibilities:
/// Under Java Language Specifications (JLS §3.9), contextual keywords such as `var`
/// (Java 10+), `yield` in statements, and `sealed` / `permits` (Java 17+) are classified
/// as restricted identifiers or restricted keywords. At the lexical grammar layer,
/// `var`, `sealed`, and `permits` are lexed as standard `NAME` tokens; their semantic
/// roles as modifiers or type names are disambiguated downstream in syntactic analysis.
class JavaLexer {
  final String source;
  final String version;
  final TokenGrammar grammar;

  JavaLexer._({
    required this.source,
    required this.version,
    required this.grammar,
  });

  /// Constructs a JavaLexer after validating source bounds and resolving the version.
  /// Precedence order: source bounds validation precedes version resolution across lanes.
  factory JavaLexer(String source, {String? version}) {
    validateSource(source);
    final canonicalVersion = resolveJavaVersion(version);
    final grammar = _versionedGrammars[canonicalVersion]!;
    return JavaLexer._(
      source: source,
      version: canonicalVersion,
      grammar: grammar,
    );
  }

  /// Tokenizes the configured Java source.
  List<Token> tokenize() {
    try {
      final tokens = grammarTokenize(source, grammar);
      return List.unmodifiable(tokens);
    } on LexerError catch (e) {
      throw JavaLexicalException(line: e.line, column: e.column);
    } catch (_) {
      throw const JavaLexicalException();
    }
  }
}

/// Creates a JavaLexer instance for the given source and optional version.
JavaLexer createJavaLexer(String source, {String? version}) =>
    JavaLexer(source, version: version);

/// Tokenizes Java source code using the embedded canonical grammar.
List<Token> tokenizeJava(String source, {String? version}) =>
    createJavaLexer(source, version: version).tokenize();
