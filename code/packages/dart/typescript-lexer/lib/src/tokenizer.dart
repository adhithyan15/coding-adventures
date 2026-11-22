import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart' as g_generic;
import '_grammar_ts1_0.dart' as g_ts1_0;
import '_grammar_ts2_0.dart' as g_ts2_0;
import '_grammar_ts3_0.dart' as g_ts3_0;
import '_grammar_ts4_0.dart' as g_ts4_0;
import '_grammar_ts5_0.dart' as g_ts5_0;
import '_grammar_ts5_8.dart' as g_ts5_8;
import 'constants.dart';
import 'errors.dart';

/// Resolves and validates a TypeScript edition version string.
///
/// Returns canonical version string. Throws [InvalidTypescriptVersionException]
/// if unsupported.
String resolveTypescriptVersion([String? version]) {
  final normalized = (version ?? '').trim().toLowerCase();
  final resolved = typescriptVersionAliases[normalized];
  if (resolved == null) {
    throw const InvalidTypescriptVersionException();
  }
  return resolved;
}

/// Returns the embedded pre-compiled [TokenGrammar] for [version].
TokenGrammar loadTypescriptTokenGrammar({String? version}) {
  final resolved = resolveTypescriptVersion(version);
  return switch (resolved) {
    'typescript' => g_generic.tokenGrammar,
    'ts1.0' => g_ts1_0.tokenGrammar,
    'ts2.0' => g_ts2_0.tokenGrammar,
    'ts3.0' => g_ts3_0.tokenGrammar,
    'ts4.0' => g_ts4_0.tokenGrammar,
    'ts5.0' => g_ts5_0.tokenGrammar,
    'ts5.8' => g_ts5_8.tokenGrammar,
    _ => throw const InvalidTypescriptVersionException(),
  };
}

void _validateSource(String source) {
  if (source.length > maxSourceLength) {
    throw const InvalidTypescriptSourceException.sourceTooLarge();
  }

  for (final scalar in source.runes) {
    if (scalar < minUnicodeScalar ||
        scalar > maxUnicodeScalar ||
        (scalar >= highSurrogateMin && scalar <= lowSurrogateMax)) {
      throw const InvalidTypescriptSourceException.invalidSurrogates();
    }
  }
}

/// A grammar-driven lexical analyzer for TypeScript.
final class TypescriptLexer {
  /// Constructs a [TypescriptLexer] for [source] text.
  ///
  /// Source bounds and UTF-16 encoding are validated BEFORE version resolution.
  TypescriptLexer(this.source, {String? version})
      : version = _initLexer(source, version);

  static String _initLexer(String source, String? version) {
    _validateSource(source);
    return resolveTypescriptVersion(version);
  }

  /// The input TypeScript source code.
  final String source;

  /// The canonical resolved TypeScript edition version string.
  final String version;

  /// Tokenizes the source text and returns the complete token stream.
  List<Token> tokenize() => tokenizeTypescript(source, version: version);
}

/// Creates a [TypescriptLexer] instance for [source].
TypescriptLexer createTypescriptLexer(String source, {String? version}) =>
    TypescriptLexer(source, version: version);

/// Alias for [createTypescriptLexer] with PascalCase casing.
TypescriptLexer createTypeScriptLexer(String source, {String? version}) =>
    createTypescriptLexer(source, version: version);

/// Tokenizes [source] using the TypeScript grammar for [version].
List<Token> tokenizeTypescript(String source, {String? version}) {
  _validateSource(source);
  final resolved = resolveTypescriptVersion(version);
  final grammar = loadTypescriptTokenGrammar(version: resolved);

  try {
    return List<Token>.unmodifiable(grammarTokenize(source, grammar));
  } on LexerError catch (e) {
    throw TypescriptLexicalException(line: e.line, column: e.column);
  }
}

/// Alias for [tokenizeTypescript] with PascalCase casing.
List<Token> tokenizeTypeScript(String source, {String? version}) =>
    tokenizeTypescript(source, version: version);

/// Alias for [tokenizeTypescript] matching TypeScript/Python naming.
List<Token> tokenizeTypescriptLexer(String source, {String? version}) =>
    tokenizeTypescript(source, version: version);
