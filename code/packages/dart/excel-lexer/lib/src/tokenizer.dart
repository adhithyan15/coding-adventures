import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';
import 'constants.dart';
import 'errors.dart';

/// Resolves and validates a Excel edition version string.
///
/// Throws [InvalidExcelVersionException] if the version string does not match
/// the canonical supported version ('excel').
String resolveExcelVersion(String version) {
  if (version.trim().toLowerCase() != defaultExcelVersion) {
    throw const InvalidExcelVersionException();
  }
  return defaultExcelVersion;
}

/// Returns the embedded pre-compiled [TokenGrammar] for Excel.
TokenGrammar loadExcelTokenGrammar({String version = defaultExcelVersion}) {
  resolveExcelVersion(version);
  return tokenGrammar;
}

void _validateSource(String source) {
  if (source.length > maxSourceLength) {
    throw const InvalidExcelSourceException.sourceTooLarge();
  }

  for (final scalar in source.runes) {
    if (scalar < minUnicodeScalar ||
        scalar > maxUnicodeScalar ||
        (scalar >= highSurrogateMin && scalar <= lowSurrogateMax)) {
      throw const InvalidExcelSourceException.invalidSurrogates();
    }
  }
}

/// A grammar-driven lexical analyzer for Excel formulas.
final class ExcelLexer {
  /// Constructs a [ExcelLexer] for [source] text.
  ExcelLexer(this.source, {this.version = defaultExcelVersion}) {
    _validateSource(source);
    resolveExcelVersion(version);
  }

  /// The input formula source code.
  final String source;

  /// The canonical resolved Excel edition version string.
  final String version;

  /// Tokenizes the source text and returns the complete token stream.
  List<Token> tokenize() => tokenizeExcel(source, version: version);
}

/// Creates a [ExcelLexer] instance for [source].
ExcelLexer createExcelLexer(
  String source, {
  String version = defaultExcelVersion,
}) =>
    ExcelLexer(source, version: version);

/// Tokenizes [source] using the Excel grammar.
List<Token> tokenizeExcel(
  String source, {
  String version = defaultExcelVersion,
}) {
  _validateSource(source);
  resolveExcelVersion(version);
  final grammar = loadExcelTokenGrammar(version: version);

  try {
    return List<Token>.unmodifiable(
      _reclassifyNames(grammarTokenize(source, grammar)),
    );
  } on LexerError catch (e) {
    throw ExcelLexicalException(line: e.line, column: e.column);
  }
}

/// Tokenizes an Excel formula; alias of [tokenizeExcel] for TypeScript parity.
List<Token> tokenizeExcelFormula(
  String source, {
  String version = defaultExcelVersion,
}) =>
    tokenizeExcel(source, version: version);

/// Promotes `NAME` tokens to `FUNCTION_NAME` / `TABLE_NAME`.
///
/// Literate why: the Excel grammar cannot tell `SUM(` (a function call) from
/// `Sum` (a defined name) or `Sales[` (a structured table reference) from a
/// plain name with a regular expression alone. The TypeScript lane uses an
/// on-token hook that peeks at the next non-space character; the Dart lexer
/// has no hook, so this pass reproduces the exact same rule on the token
/// stream: only ASCII-space `SPACE` tokens that are physically contiguous with
/// the name may be skipped (tabs and newlines are skipped by the grammar and
/// therefore break contiguity, exactly like the character peek would).
List<Token> _reclassifyNames(List<Token> tokens) {
  final result = List<Token>.of(tokens);
  for (var i = 0; i < tokens.length; i++) {
    final token = tokens[i];
    if (token.type != tokenTypeName) {
      continue;
    }
    final promoted = _promotedType(tokens, i);
    if (promoted != null) {
      result[i] = Token(
        type: promoted,
        value: token.value,
        line: token.line,
        column: token.column,
        flags: token.flags,
      );
    }
  }
  return result;
}

/// Returns the promoted token type for the name at [index], if any.
String? _promotedType(List<Token> tokens, int index) {
  final name = tokens[index];
  var expectedColumn = name.column + name.value.runes.length;
  for (var j = index + 1; j < tokens.length; j++) {
    final next = tokens[j];
    if (next.line != name.line || next.column != expectedColumn) {
      return null;
    }
    if (next.type == tokenTypeSpace) {
      expectedColumn += next.value.runes.length;
      continue;
    }
    if (next.value.startsWith(symbolLParen)) {
      return tokenTypeFunctionName;
    }
    if (next.value.startsWith(symbolLBracket)) {
      return tokenTypeTableName;
    }
    return null;
  }
  return null;
}

/// Alias for [tokenizeExcel] matching TypeScript/Python naming.
List<Token> tokenizeExcelLexer(
  String source, {
  String version = defaultExcelVersion,
}) =>
    tokenizeExcel(source, version: version);
