import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

import 'token.dart';
import 'tokenizer.dart';

class _CompiledPattern {
  _CompiledPattern({
    required this.definition,
    required this.caseSensitive,
  }) : regex = definition.isRegex
            ? RegExp(
                '^(?:${definition.pattern})',
                caseSensitive: caseSensitive,
                unicode: true,
                dotAll: true,
              )
            : null;

  final TokenDefinition definition;
  final bool caseSensitive;
  final RegExp? regex;

  String? match(String source, int position) {
    if (definition.isRegex) {
      final slice = source.substring(position);
      final match = regex!.firstMatch(slice);
      if (match == null) {
        return null;
      }
      final value = match.group(0)!;
      return value.isEmpty ? null : value;
    }

    final literal = definition.pattern;
    if (position + literal.length > source.length) {
      return null;
    }
    final candidate = source.substring(position, position + literal.length);
    if (caseSensitive) {
      return candidate == literal ? candidate : null;
    }
    return candidate.toLowerCase() == literal.toLowerCase() ? candidate : null;
  }
}

List<Token> grammarTokenize(String source, TokenGrammar grammar) {
  final effectiveCaseSensitive =
      grammar.caseSensitive && !grammar.caseInsensitive;
  final compiledDefinitions = grammar.definitions
      .map(
        (definition) => _CompiledPattern(
          definition: definition,
          caseSensitive: effectiveCaseSensitive,
        ),
      )
      .toList(growable: false);
  final compiledSkips = grammar.skipDefinitions
      .map(
        (definition) => _CompiledPattern(
          definition: definition,
          caseSensitive: effectiveCaseSensitive,
        ),
      )
      .toList(growable: false);

  final keywordSet =
      grammar.keywords.map(_normalizeForLookup(effectiveCaseSensitive)).toSet();
  final reservedSet = grammar.reservedKeywords
      .map(_normalizeForLookup(effectiveCaseSensitive))
      .toSet();

  final tokens = <Token>[];
  var position = 0;
  var line = 1;
  var column = 1;

  void advanceText(String text) {
    for (final codePoint in text.runes) {
      position += 1;
      if (codePoint == 0x0A) {
        line += 1;
        column = 1;
      } else {
        column += 1;
      }
    }
  }

  void skipIgnored() {
    while (true) {
      var matched = false;
      for (final pattern in compiledSkips) {
        final text = pattern.match(source, position);
        if (text == null) {
          continue;
        }
        advanceText(text);
        matched = true;
        break;
      }
      if (!matched) {
        return;
      }
    }
  }

  while (true) {
    skipIgnored();

    if (position >= source.length) {
      break;
    }

    final current = source[position];
    if (current == '\n') {
      tokens.add(
        Token(type: 'NEWLINE', value: r'\n', line: line, column: column),
      );
      advanceText('\n');
      continue;
    }

    TokenDefinition? matchedDefinition;
    String? matchedText;
    for (final pattern in compiledDefinitions) {
      final text = pattern.match(source, position);
      if (text == null) {
        continue;
      }
      matchedDefinition = pattern.definition;
      matchedText = text;
      break;
    }

    if (matchedDefinition == null || matchedText == null) {
      throw LexerError(
        'Unexpected character: "${source[position]}"',
        line,
        column,
      );
    }

    final startLine = line;
    final startColumn = column;
    advanceText(matchedText);

    final type = _resolveTokenType(
      matchedDefinition,
      matchedText,
      grammar,
      keywordSet,
      reservedSet,
      effectiveCaseSensitive,
      startLine,
      startColumn,
    );
    final value = _resolveTokenValue(matchedText, type, grammar.escapeMode);

    tokens.add(
      Token(type: type, value: value, line: startLine, column: startColumn),
    );
  }

  tokens.add(Token(type: 'EOF', value: '', line: line, column: column));
  return List<Token>.unmodifiable(tokens);
}

String _resolveTokenType(
  TokenDefinition definition,
  String matchedText,
  TokenGrammar grammar,
  Set<String> keywordSet,
  Set<String> reservedSet,
  bool caseSensitive,
  int line,
  int column,
) {
  final normalizedText = _normalizeValue(matchedText, caseSensitive);
  if (definition.name == 'NAME' && reservedSet.contains(normalizedText)) {
    throw LexerError(
      "Reserved keyword '$matchedText' cannot be used as an identifier",
      line,
      column,
    );
  }

  if (definition.name == 'NAME' && keywordSet.contains(normalizedText)) {
    return 'KEYWORD';
  }

  return definition.alias ?? definition.name;
}

String _resolveTokenValue(String matchedText, String type, String? escapeMode) {
  if (type == 'STRING' &&
      matchedText.length >= 2 &&
      matchedText.startsWith('"') &&
      matchedText.endsWith('"')) {
    final inner = matchedText.substring(1, matchedText.length - 1);
    if (escapeMode == 'none') {
      return inner;
    }
    return _decodeEscapes(inner);
  }
  return matchedText;
}

String _decodeEscapes(String value) {
  final buffer = StringBuffer();
  var index = 0;
  while (index < value.length) {
    final current = value[index];
    if (current != r'\' || index + 1 >= value.length) {
      buffer.write(current);
      index += 1;
      continue;
    }

    final escaped = value[index + 1];
    switch (escaped) {
      case 'n':
        buffer.write('\n');
      case 't':
        buffer.write('\t');
      case 'r':
        buffer.write('\r');
      case 'b':
        buffer.write('\b');
      case 'f':
        buffer.write('\f');
      case '"':
        buffer.write('"');
      case r'\':
        buffer.write(r'\');
      case '/':
        buffer.write('/');
      case 'u':
        if (index + 5 < value.length) {
          final hex = value.substring(index + 2, index + 6);
          final codePoint = int.tryParse(hex, radix: 16);
          if (codePoint != null) {
            buffer.write(String.fromCharCode(codePoint));
            index += 6;
            continue;
          }
        }
        buffer.write(r'\u');
      default:
        buffer.write(escaped);
    }
    index += 2;
  }
  return buffer.toString();
}

String Function(String) _normalizeForLookup(bool caseSensitive) =>
    (value) => _normalizeValue(value, caseSensitive);

String _normalizeValue(String value, bool caseSensitive) =>
    caseSensitive ? value : value.toLowerCase();
