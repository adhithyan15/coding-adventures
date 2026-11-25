/// Tokenizer implementation and version resolver for Starlark source text.
library;

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart' as starlark_grammar;
import 'constants.dart';
import 'errors.dart';

/// Validates that [source] does not exceed [maxSourceLength] and has valid UTF-16 surrogate pairs.
void validateStarlarkSource(String source) {
  if (source.length > maxSourceLength) {
    throw const InvalidStarlarkSourceException.sourceTooLarge();
  }
  for (var i = 0; i < source.length; i++) {
    final code = source.codeUnitAt(i);
    if (code >= minSurrogateCodeUnit && code <= maxSurrogateCodeUnit) {
      if (code <= maxHighSurrogate) {
        if (i + 1 < source.length) {
          final next = source.codeUnitAt(i + 1);
          if (next >= minLowSurrogate && next <= maxLowSurrogate) {
            i++;
            continue;
          }
        }
        throw const InvalidStarlarkSourceException.invalidSurrogates();
      } else {
        throw const InvalidStarlarkSourceException.invalidSurrogates();
      }
    }
  }
}

/// Resolves a caller-supplied version string to the canonical Starlark version identifier.
///
/// Throws [InvalidStarlarkVersionException] if the version string is unrecognized.
String resolveStarlarkVersion([String? version]) {
  if (version == null) {
    return defaultStarlarkVersion;
  }
  final normalized = version.trim().toLowerCase();
  if (normalized == 'starlark' || normalized == '1') {
    return defaultStarlarkVersion;
  }
  throw const InvalidStarlarkVersionException();
}

/// Loads the precompiled [TokenGrammar] for Starlark.
TokenGrammar loadStarlarkTokenGrammar([String? version]) {
  resolveStarlarkVersion(version);
  return starlark_grammar.tokenGrammar;
}

/// A grammar-driven lexical analyzer for Starlark source text.
final class StarlarkLexer {
  /// Constructs a [StarlarkLexer] for [source] text.
  StarlarkLexer(this.source, {String? version})
      : version = _initLexer(source, version);

  static String _initLexer(String source, String? version) {
    validateStarlarkSource(source);
    return resolveStarlarkVersion(version);
  }

  /// The input Starlark source code.
  final String source;

  /// The canonical resolved Starlark version.
  final String version;

  /// Tokenizes the source text and returns the token stream.
  List<Token> tokenize() => tokenizeStarlark(source, version: version);
}

/// Creates a [StarlarkLexer] instance for [source].
StarlarkLexer createStarlarkLexer(String source, {String? version}) =>
    StarlarkLexer(source, version: version);

/// Tokenizes Starlark source code into a list of [Token] objects.
List<Token> tokenizeStarlark(String source, {String? version}) {
  validateStarlarkSource(source);
  resolveStarlarkVersion(version);
  final grammar = loadStarlarkTokenGrammar(version);
  try {
    return List<Token>.unmodifiable(grammarTokenize(source, grammar));
  } on LexerError catch (e) {
    throw StarlarkLexicalException(line: e.line, column: e.column);
  }
}

/// Alias for [tokenizeStarlark].
List<Token> tokenizeStarlarkLexer(String source, {String? version}) =>
    tokenizeStarlark(source, version: version);
