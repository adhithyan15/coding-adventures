import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar_1_0.dart' as g1_0;
import '_grammar_2_0.dart' as g2_0;
import '_grammar_3_0.dart' as g3_0;
import '_grammar_4_0.dart' as g4_0;
import '_grammar_5_0.dart' as g5_0;
import '_grammar_6_0.dart' as g6_0;
import '_grammar_7_0.dart' as g7_0;
import '_grammar_8_0.dart' as g8_0;
import '_grammar_9_0.dart' as g9_0;
import '_grammar_10_0.dart' as g10_0;
import '_grammar_11_0.dart' as g11_0;
import '_grammar_12_0.dart' as g12_0;
import 'constants.dart';
import 'errors.dart';

/// Precompiled C# token grammars keyed by canonical edition string.
///
/// Literate why — Authority-free embedded grammars:
/// Embedding pre-parsed TokenGrammar objects directly into compiled code guarantees
/// 100% Hermetic execution with zero runtime filesystem reads or dynamic file resolution,
/// completely isolating lexical analysis from disk I/O, path traversal, or race conditions.
final Map<String, TokenGrammar> _versionedGrammars = {
  '1.0': g1_0.tokenGrammar,
  '2.0': g2_0.tokenGrammar,
  '3.0': g3_0.tokenGrammar,
  '4.0': g4_0.tokenGrammar,
  '5.0': g5_0.tokenGrammar,
  '6.0': g6_0.tokenGrammar,
  '7.0': g7_0.tokenGrammar,
  '8.0': g8_0.tokenGrammar,
  '9.0': g9_0.tokenGrammar,
  '10.0': g10_0.tokenGrammar,
  '11.0': g11_0.tokenGrammar,
  '12.0': g12_0.tokenGrammar,
};

/// Resolves the caller-provided C# edition version string into a canonical version.
/// Defaults to '12.0' when [version] is null or empty.
///
/// Aliases (e.g. 'csharp1.0', 'csharp1_0', '1_0', 'csharp12.0') are resolved via [csharpVersionAliases].
String resolveCSharpVersion([String? version]) {
  if (version == null || version.isEmpty) {
    return defaultCSharpVersion;
  }
  final canonical = csharpVersionAliases[version];
  if (canonical == null) {
    throw const InvalidCSharpVersionException();
  }
  return canonical;
}

/// Returns the embedded TokenGrammar for the requested C# version.
TokenGrammar loadCSharpTokenGrammar({String? version}) {
  final resolved = resolveCSharpVersion(version);
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
    throw const InvalidCSharpSourceException.sourceTooLarge();
  }
  final units = source.codeUnits;
  for (var i = 0; i < units.length; i++) {
    final unit = units[i];
    if (unit >= highSurrogateMin && unit <= highSurrogateMax) {
      if (++i >= units.length ||
          units[i] < lowSurrogateMin ||
          units[i] > lowSurrogateMax) {
        throw const InvalidCSharpSourceException.invalidSurrogates();
      }
    } else if (unit >= lowSurrogateMin && unit <= lowSurrogateMax) {
      throw const InvalidCSharpSourceException.invalidSurrogates();
    }
  }
}

/// A configured grammar-driven C# lexer.
///
/// Literate why — C# Lexical Grammar vs Parser Responsibilities:
/// Under the C# Language Specification (ECMA-334), contextual keywords such as `var`
/// (C# 3.0+), `yield` (C# 2.0+), `async` / `await` (C# 5.0+), `record` / `init` / `with`
/// (C# 9.0+), `file` / `scoped` (C# 11.0+), and collection expressions (C# 12.0+) are
/// classified into lexical tokens according to the edition-specific token grammar.
/// Ambiguities between modifiers, type names, and expressions are resolved downstream
/// during syntactic analysis.
class CSharpLexer {
  final String source;
  final String version;
  final TokenGrammar grammar;

  CSharpLexer._({
    required this.source,
    required this.version,
    required this.grammar,
  });

  /// Constructs a CSharpLexer after validating source bounds and resolving the version.
  /// Precedence order: source bounds validation precedes version resolution across lanes.
  factory CSharpLexer(String source, {String? version}) {
    validateSource(source);
    final canonicalVersion = resolveCSharpVersion(version);
    final grammar = _versionedGrammars[canonicalVersion]!;
    return CSharpLexer._(
      source: source,
      version: canonicalVersion,
      grammar: grammar,
    );
  }

  /// Tokenizes the configured C# source.
  List<Token> tokenize() {
    try {
      final tokens = grammarTokenize(source, grammar);
      return List.unmodifiable(tokens);
    } on LexerError catch (e) {
      throw CSharpLexicalException(line: e.line, column: e.column);
    } catch (_) {
      throw const CSharpLexicalException();
    }
  }
}

/// Creates a CSharpLexer instance for the given source and optional version.
CSharpLexer createCSharpLexer(String source, {String? version}) =>
    CSharpLexer(source, version: version);

/// Tokenizes C# source code using the embedded canonical grammar.
List<Token> tokenizeCSharp(String source, {String? version}) =>
    createCSharpLexer(source, version: version).tokenize();
