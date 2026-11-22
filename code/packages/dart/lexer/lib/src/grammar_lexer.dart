import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

import 'constants.dart';
import 'messages.dart';
import 'token.dart';
import 'tokenizer.dart';

/// A token definition compiled once for matching at arbitrary positions.
///
/// Literate why — linear-time matching (CWE-400): the previous implementation
/// compiled `^(?:pattern)` and matched against `source.substring(position)`,
/// copying the whole remaining input for every attempt at every position,
/// which made lexing quadratic in source size. [RegExp.matchAsPrefix] anchors
/// the match at `position` inside the original string with no copy, so each
/// attempt costs only the length of what it matches. Patterns therefore see
/// the real preceding text (lookbehind and `\b` now behave as written) and
/// must not rely on `^` to mean "start of this token".
class _CompiledPattern {
  _CompiledPattern({required this.definition, required this.caseSensitive})
      : regex = definition.isRegex
            ? RegExp(
                '(?:${definition.pattern})',
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
      final match = regex!.matchAsPrefix(source, position);
      if (match == null) {
        return null;
      }
      final value = match.group(0)!;
      return value.isEmpty ? null : value;
    }

    final literal = definition.pattern;
    if (caseSensitive) {
      return source.startsWith(literal, position) ? literal : null;
    }
    if (position + literal.length > source.length) {
      return null;
    }
    final candidate = source.substring(position, position + literal.length);
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

  if (grammar.mode == 'indentation') {
    return _tokenizeIndentation(
      source,
      grammar,
      compiledDefinitions,
      compiledSkips,
      keywordSet,
      reservedSet,
      effectiveCaseSensitive,
    );
  }

  final tokens = <Token>[];
  var position = 0;
  var line = 1;
  var column = 1;

  void advanceText(String text) {
    for (final codePoint in text.runes) {
      // `position` indexes UTF-16 code units (it is used with substring and
      // regex offsets) while `column` counts code points, so an astral code
      // point advances the former by two and the latter by one.
      position += codePoint > maxBmpCodePoint
          ? utf16UnitsPerAstralCodePoint
          : utf16UnitsPerBmpCodePoint;
      if (codePoint == lineFeedCodePoint) {
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
      throw LexerError(messageUnexpectedCharacter, line, column);
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

List<Token> _tokenizeIndentation(
  String source,
  TokenGrammar grammar,
  List<_CompiledPattern> compiledDefinitions,
  List<_CompiledPattern> compiledSkips,
  Set<String> keywordSet,
  Set<String> reservedSet,
  bool effectiveCaseSensitive,
) {
  final tokens = <Token>[];
  final indentStack = <int>[0];
  var bracketDepth = 0;
  var atLineStart = true;

  var position = 0;
  var line = 1;
  var column = 1;

  void advanceText(String text) {
    for (final codePoint in text.runes) {
      position += codePoint > maxBmpCodePoint
          ? utf16UnitsPerAstralCodePoint
          : utf16UnitsPerBmpCodePoint;
      if (codePoint == lineFeedCodePoint) {
        line += 1;
        column = 1;
      } else {
        column += 1;
      }
    }
  }

  while (position < source.length) {
    if (atLineStart && bracketDepth == 0) {
      var indent = 0;
      var hasTab = false;
      var tabLine = line;
      var tabCol = column;

      while (position < source.length) {
        final code = source.codeUnitAt(position);
        if (code == spaceCodePoint) {
          indent++;
          advanceText(' ');
        } else if (code == tabCodePoint) {
          hasTab = true;
          tabLine = line;
          tabCol = column;
          break;
        } else {
          break;
        }
      }

      if (hasTab) {
        throw LexerError(messageTabInIndentation, tabLine, tabCol);
      }

      if (position >= source.length) {
        break;
      }

      if (source[position] == '\n') {
        advanceText('\n');
        atLineStart = true;
        continue;
      }

      var commentOnly = false;
      for (final pattern in compiledSkips) {
        final text = pattern.match(source, position);
        if (text != null) {
          final nextPos = position + text.length;
          if (nextPos >= source.length || source[nextPos] == '\n') {
            advanceText(text);
            if (position < source.length && source[position] == '\n') {
              advanceText('\n');
            }
            atLineStart = true;
            commentOnly = true;
            break;
          }
        }
      }

      if (commentOnly) {
        continue;
      }

      if (indent > indentStack.last) {
        indentStack.add(indent);
        tokens.add(Token(type: 'INDENT', value: '', line: line, column: 1));
      } else if (indent < indentStack.last) {
        while (indentStack.length > 1 && indentStack.last > indent) {
          indentStack.removeLast();
          tokens.add(Token(type: 'DEDENT', value: '', line: line, column: 1));
        }
        if (indentStack.last != indent) {
          throw LexerError(messageInconsistentDedent, line, column);
        }
      }
      atLineStart = false;
    }

    if (position >= source.length) {
      break;
    }

    final char = source[position];
    if (char == '\n') {
      if (bracketDepth == 0) {
        tokens.add(
          Token(type: 'NEWLINE', value: r'\n', line: line, column: column),
        );
        advanceText('\n');
        atLineStart = true;
      } else {
        advanceText('\n');
      }
      continue;
    }

    if (bracketDepth > 0) {
      final code = source.codeUnitAt(position);
      if (code == spaceCodePoint ||
          code == tabCodePoint ||
          code == carriageReturnCodePoint) {
        advanceText(source[position]);
        continue;
      }
    }

    var skipped = false;
    for (final pattern in compiledSkips) {
      final text = pattern.match(source, position);
      if (text == null) continue;
      advanceText(text);
      skipped = true;
      break;
    }
    if (skipped) continue;

    TokenDefinition? matchedDefinition;
    String? matchedText;
    for (final pattern in compiledDefinitions) {
      final text = pattern.match(source, position);
      if (text == null) continue;
      matchedDefinition = pattern.definition;
      matchedText = text;
      break;
    }

    if (matchedDefinition == null || matchedText == null) {
      throw LexerError(messageUnexpectedCharacter, line, column);
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

    if (matchedText == '(' || matchedText == '[' || matchedText == '{') {
      bracketDepth++;
    } else if (matchedText == ')' || matchedText == ']' || matchedText == '}') {
      if (bracketDepth > 0) {
        bracketDepth--;
      }
    }
  }

  if (tokens.isNotEmpty && tokens.last.type != 'NEWLINE') {
    tokens.add(
      Token(type: 'NEWLINE', value: r'\n', line: line, column: column),
    );
  }

  while (indentStack.length > 1) {
    indentStack.removeLast();
    tokens.add(Token(type: 'DEDENT', value: '', line: line, column: column));
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
    throw LexerError(messageReservedKeyword, line, column);
  }

  if (definition.name == 'NAME' && keywordSet.contains(normalizedText)) {
    return 'KEYWORD';
  }

  return definition.alias ?? definition.name;
}

String _resolveTokenValue(String matchedText, String type, String? escapeMode) {
  if (type != 'STRING' && !type.contains('STRING')) {
    return matchedText;
  }

  String? inner;
  if (matchedText.length >= 6 &&
      ((matchedText.startsWith('"""') && matchedText.endsWith('"""')) ||
          (matchedText.startsWith("'''") && matchedText.endsWith("'''")))) {
    inner = matchedText.substring(3, matchedText.length - 3);
  } else if (matchedText.length >= 2 &&
      ((matchedText.startsWith('"') && matchedText.endsWith('"')) ||
          (matchedText.startsWith("'") && matchedText.endsWith("'")))) {
    inner = matchedText.substring(1, matchedText.length - 1);
  }

  if (inner == null || escapeMode == 'none') {
    return inner ?? matchedText;
  }
  return _decodeEscapes(inner);
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
