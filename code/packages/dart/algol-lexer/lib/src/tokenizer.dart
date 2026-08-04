import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

const String defaultAlgolVersion = 'algol60';
const Set<String> supportedAlgolVersions = {defaultAlgolVersion};

const Map<String, String> _symbolicKeywords = {
  'NOT_SYM': 'not',
  'AND_SYM': 'and',
  'OR_SYM': 'or',
  'IMPL_SYM': 'impl',
  'EQV_SYM': 'eqv',
};

const Map<String, Map<String, String>> _symbolicOperatorValues = {
  'LEQ': {'\u2264': '<='},
  'GEQ': {'\u2265': '>='},
  'NEQ': {'\u2260': '!='},
  'CARET': {'\u2191': '^'},
  'STAR': {'\u00d7': '*'},
  'SLASH': {'\u00f7': '/'},
};

/// Resolves an ALGOL grammar version and rejects unsupported versions.
String resolveAlgolVersion([String? version]) {
  final resolved = version ?? defaultAlgolVersion;
  if (!supportedAlgolVersions.contains(resolved)) {
    throw ArgumentError.value(
      version,
      'version',
      'Unknown ALGOL version. Valid versions: algol60',
    );
  }
  return resolved;
}

/// Returns the embedded token grammar for a supported ALGOL version.
TokenGrammar loadAlgolTokenGrammar({String version = defaultAlgolVersion}) {
  resolveAlgolVersion(version);
  return tokenGrammar;
}

/// A configured grammar-driven ALGOL 60 lexer.
class AlgolLexer {
  AlgolLexer(this.source, {String version = defaultAlgolVersion})
      : version = resolveAlgolVersion(version);

  final String source;
  final String version;

  List<Token> tokenize() {
    final grammar = loadAlgolTokenGrammar(version: version);
    final preparedSource = _normalizeCommentKeywords(source);
    return _normalizeAlgolTokens(grammarTokenize(preparedSource, grammar));
  }
}

/// Creates an ALGOL 60 lexer for callers that want direct tokenizer access.
AlgolLexer createAlgolLexer(
  String source, {
  String version = defaultAlgolVersion,
}) =>
    AlgolLexer(source, version: version);

/// Tokenizes ALGOL 60 source using the embedded canonical grammar.
List<Token> tokenizeAlgol(
  String source, {
  String version = defaultAlgolVersion,
}) =>
    createAlgolLexer(source, version: version).tokenize();

List<Token> _normalizeAlgolTokens(List<Token> tokens) {
  final keywords =
      tokenGrammar.keywords.map((value) => value.toLowerCase()).toSet();
  return List<Token>.unmodifiable(
    tokens.map((token) {
      final symbolicKeyword = _symbolicKeywords[token.type];
      if (symbolicKeyword != null) {
        return _replaceToken(token, type: 'KEYWORD', value: symbolicKeyword);
      }

      final symbolicOperator =
          _symbolicOperatorValues[token.type]?[token.value];
      if (symbolicOperator != null) {
        return _replaceToken(token, value: symbolicOperator);
      }

      final lowerValue = token.value.toLowerCase();
      if (token.type == 'NAME' && keywords.contains(lowerValue)) {
        return _replaceToken(token, type: 'KEYWORD', value: lowerValue);
      }
      if (token.type == 'KEYWORD' && keywords.contains(lowerValue)) {
        return _replaceToken(token, value: lowerValue);
      }
      return token;
    }),
  );
}

Token _replaceToken(Token token, {String? type, String? value}) => Token(
      type: type ?? token.type,
      value: value ?? token.value,
      line: token.line,
      column: token.column,
      flags: token.flags,
    );

String _normalizeCommentKeywords(String source) {
  final characters = source.split('');
  var inSingleQuotedString = false;
  var inDoubleQuotedString = false;

  for (var index = 0; index < source.length; index += 1) {
    final character = source[index];
    if (character == "'" && !inDoubleQuotedString) {
      inSingleQuotedString = !inSingleQuotedString;
      continue;
    }
    if (character == '"' && !inSingleQuotedString) {
      inDoubleQuotedString = !inDoubleQuotedString;
      continue;
    }
    if (inSingleQuotedString || inDoubleQuotedString) {
      continue;
    }

    const keyword = 'comment';
    if (index + keyword.length > source.length ||
        source.substring(index, index + keyword.length).toLowerCase() !=
            keyword) {
      continue;
    }

    final startsAtBoundary =
        index == 0 || !_isIdentifierCodeUnit(source.codeUnitAt(index - 1));
    final end = index + keyword.length;
    final endsAtBoundary =
        end == source.length || !_isIdentifierCodeUnit(source.codeUnitAt(end));
    if (!startsAtBoundary || !endsAtBoundary) {
      continue;
    }

    for (var offset = 0; offset < keyword.length; offset += 1) {
      characters[index + offset] = keyword[offset];
    }

    final terminator = source.indexOf(';', end);
    if (terminator != -1) {
      index = terminator;
    }
  }

  return characters.join();
}

bool _isIdentifierCodeUnit(int codeUnit) =>
    (codeUnit >= 0x30 && codeUnit <= 0x39) ||
    (codeUnit >= 0x41 && codeUnit <= 0x5a) ||
    (codeUnit >= 0x61 && codeUnit <= 0x7a) ||
    codeUnit == 0x5f;
