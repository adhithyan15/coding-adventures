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
    // An empty literal matches everywhere with zero width, which would
    // spin the match loop forever: never match, like empty regexes.
    if (literal.isEmpty) return null;
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

    // Built-in whitespace rule: indentation mode always skips ' ', '\t'
    // and '\r' between tokens (the reference's `tokenize_indentation`
    // does this unconditionally, before skip patterns) — grammars
    // without a WHITESPACE skip (e.g. every python*.tokens) rely on
    // it. Only tabs in indentation position are an error (handled in
    // the line-start block above).
    {
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

// ── Pattern groups + callbacks (F04 core) ────────────────────────────────
//
// The [GrammarLexer] class below ports the group/callback engine: named
// pattern groups with a stack, an on-token callback driving a
// [LexerContext] (push/pop/emit/suppress/skip-toggle/peek/lookbehind),
// and gap flags. It mirrors the reference's collect-then-apply
// semantics exactly.
//
// Deliberately deferred (no in-lane consumer yet; the reference
// features stay documented here so the next slice knows where they go):
// definition guards, bracket and template tracking,
// `precededByNewline`, source ranges, and layout mode. `tokenize()`
// rejects indentation/layout grammars — [grammarTokenize] serves
// those.
//
// Two deliberate divergences, both documented where they occur:
// custom token types keep their grammar names verbatim (the reference
// maps known ones through a `TokenType` enum whose display differs in
// case), and diagnostics stay payload-blind per Standing Rule #20 while
// keeping every reference assertion prefix.

/// An action a [LexerContext] callback records for the engine to apply
/// after the callback returns.
sealed class ContextAction {
  const ContextAction();
}

/// Pushes a validated group name onto the group stack.
final class PushGroup extends ContextAction {
  const PushGroup(this.group);
  final String group;

  @override
  bool operator ==(Object other) => other is PushGroup && other.group == group;

  @override
  int get hashCode => group.hashCode;

  @override
  String toString() => 'PushGroup($group)';
}

/// Pops the group stack unless only `default` remains.
final class PopGroup extends ContextAction {
  const PopGroup();

  @override
  bool operator ==(Object other) => other is PopGroup;

  @override
  int get hashCode => 1;

  @override
  String toString() => 'PopGroup()';
}

/// Injects a synthetic token after the current one (no callback).
final class EmitToken extends ContextAction {
  const EmitToken(this.token);
  final Token token;

  @override
  bool operator ==(Object other) => other is EmitToken && other.token == token;

  @override
  int get hashCode => token.hashCode;

  @override
  String toString() => 'EmitToken($token)';
}

/// Toggles skip-pattern processing.
final class SetSkipEnabled extends ContextAction {
  const SetSkipEnabled(this.enabled);
  final bool enabled;

  @override
  bool operator ==(Object other) =>
      other is SetSkipEnabled && other.enabled == enabled;

  @override
  int get hashCode => enabled.hashCode;

  @override
  String toString() => 'SetSkipEnabled($enabled)';
}

/// The on-token callback signature: the matched token plus a mutable
/// context for recording actions.
typedef OnTokenCallback = void Function(Token token, LexerContext ctx);

/// A source transform run before tokenization.
typedef PreTokenizeHook = String Function(String source);

/// A token-list transform run after tokenization (EOF included).
typedef PostTokenizeHook = List<Token> Function(List<Token> tokens);

/// The callback's view of the lexer.
///
/// State-changing methods record [ContextAction]s applied in order after
/// the callback returns; reads observe the pre-callback state. The engine
/// constructs one per matched token; the constructor is public so tests
/// can exercise the context in isolation.
final class LexerContext {
  LexerContext({
    required Set<String> groupNames,
    required List<String> groupStack,
    required this.source,
    required this.posAfterToken,
    this.previousToken,
  })  : _groupNames = groupNames,
        _groupStack = groupStack,
        actions = [];

  final Set<String> _groupNames;
  final List<String> _groupStack;

  /// The source being tokenized (for peeking).
  final String source;

  /// UTF-16 offset immediately after the current token.
  final int posAfterToken;

  /// The most recently emitted token (`null` at the start of input).
  /// Suppressed tokens do not count.
  final Token? previousToken;

  /// Recorded actions, applied in order after the callback returns.
  final List<ContextAction> actions;

  /// Whether the current token is suppressed from the output.
  bool suppressed = false;

  /// Pushes [group] for the next match. Throws [ArgumentError] for
  /// unknown group names (grammar identifiers, safe to echo).
  void pushGroup(String group) {
    if (!_groupNames.contains(group)) {
      final available = _groupNames.toList()..sort();
      final quoted = available.map((n) => '"$n"').join(', ');
      throw ArgumentError.value(group, 'group',
          'Unknown pattern group: "$group". Available groups: [$quoted]');
    }
    actions.add(PushGroup(group));
  }

  /// Pops the group stack; a no-op on `default` alone.
  void popGroup() {
    actions.add(const PopGroup());
  }

  /// The active (top-of-stack) group name.
  String get activeGroup => _groupStack.last;

  /// The group stack depth (always >= 1).
  int get groupStackDepth => _groupStack.length;

  /// Injects [token] after the current one, without re-triggering the
  /// callback. Multiple emits keep call order.
  void emit(Token token) {
    actions.add(EmitToken(token));
  }

  /// Omits the current token from the output.
  void suppress() {
    suppressed = true;
  }

  /// The rune [offset] past the current token (1-based), or `''` past
  /// EOF (or for non-positive offsets, which the reference leaves to
  /// unsigned arithmetic).
  String peek(int offset) {
    if (offset < 1) return '';
    final it = source.runes.skip(_runePosAfterToken() + offset - 1).iterator;
    if (!it.moveNext()) return '';
    return String.fromCharCode(it.current);
  }

  /// Up to [length] runes past the current token (fewer at EOF).
  String peekStr(int length) {
    if (length <= 0) return '';
    final runes = source.runes.skip(_runePosAfterToken()).take(length).toList();
    return String.fromCharCodes(runes);
  }

  /// The rune index matching [posAfterToken].
  int _runePosAfterToken() {
    var units = 0;
    var runes = 0;
    for (final rune in source.runes) {
      if (units >= posAfterToken) break;
      units += rune > maxBmpCodePoint
          ? utf16UnitsPerAstralCodePoint
          : utf16UnitsPerBmpCodePoint;
      runes++;
    }
    return runes;
  }

  /// Toggles skip-pattern processing for subsequent matches.
  void setSkipEnabled(bool enabled) {
    actions.add(SetSkipEnabled(enabled));
  }
}

// ── GrammarLexer ─────────────────────────────────────────────────────────

/// Line terminators for gap tracking: LF, CR, LS, PS.
final RegExp _lineTerminator = RegExp(r'[\n\r\u2028\u2029]');

/// Facts about the token just emitted, for transition conditions.
class _TokenFacts {
  const _TokenFacts({
    required this.spaceBefore,
    required this.lineStart,
    required this.nextChar,
  });

  /// Trivia came right before the token.
  final bool spaceBefore;

  /// The token's first character was at a line start.
  final bool lineStart;

  /// The code point right after the token, or `null` at end of input.
  final int? nextChar;

  /// Whether [condition] holds.
  ///
  /// `spaceAfter` treats end of input as a separator: nothing is
  /// adjacent. Whitespace is the ASCII set (space, tab, `\n`, `\r`,
  /// vertical tab, form feed).
  bool holds(TransitionCondition condition) {
    final after = nextChar;
    final spaceAfter = after == null ||
        after == 0x20 ||
        after == 0x09 ||
        after == 0x0A ||
        after == 0x0D ||
        after == 0x0B ||
        after == 0x0C;
    return switch (condition) {
      TransitionCondition.spaceBefore => spaceBefore,
      TransitionCondition.noSpaceBefore => !spaceBefore,
      TransitionCondition.spaceAfter => spaceAfter,
      TransitionCondition.noSpaceAfter => !spaceAfter,
      TransitionCondition.lineStart => lineStart,
      TransitionCondition.notLineStart => !lineStart,
    };
  }
}

/// A lexer that tokenizes source code according to a [TokenGrammar].
///
/// Compiles the grammar's definitions (plus each named group's) into
/// anchored matchers once, then walks the source trying the active
/// group's patterns first-match-wins. An [OnTokenCallback] registered
/// with [setOnToken] drives group switches, synthetic tokens,
/// suppression, and skip toggling through a [LexerContext]. A
/// declarative `transitions:` table, when present, refines the mode
/// after each emitted token (callback runs first, the table second).
///
/// Standard mode only: indentation/layout grammars throw
/// [UnsupportedError] ([grammarTokenize] serves those). Token types keep
/// their grammar names verbatim (`alias ?? name`, `KEYWORD` for promoted
/// keywords) rather than mapping through an enum display layer.
///
/// Trust boundary: grammars are developer-authored. Sources may be
/// hostile, but a grammar carrying nested-quantifier regexes would
/// backtrack under Dart's engine (the reference uses linear-time
/// regex), and a malformed pattern throws [FormatException] at
/// construction — both acceptable only behind this boundary.
final class GrammarLexer {
  GrammarLexer(String source, TokenGrammar grammar)
      : _source = source,
        _grammar = grammar,
        _keywordSet = {
          for (final keyword in grammar.keywords)
            if (grammar.caseInsensitive) keyword.toUpperCase() else keyword
        },
        _reservedSet = {
          for (final keyword in grammar.reservedKeywords)
            if (grammar.caseInsensitive) keyword.toUpperCase() else keyword
        },
        _skipPatterns = grammar.skipDefinitions
            .map((definition) => _CompiledPattern(
                definition: definition, caseSensitive: grammar.caseSensitive))
            .toList(growable: false),
        _groupPatterns = {
          'default': grammar.definitions
              .map((definition) => _CompiledPattern(
                  definition: definition, caseSensitive: grammar.caseSensitive))
              .toList(growable: false),
          for (final entry in grammar.groups.entries)
            entry.key: entry.value.definitions
                .map((definition) => _CompiledPattern(
                    definition: definition,
                    caseSensitive: grammar.caseSensitive))
                .toList(growable: false),
        },
        _groupNames = {'default', ...grammar.groups.keys},
        _startMode = _resolveStartMode(grammar),
        _transitions = grammar.transitions,
        _inheritingModes = _classifyInheritingModes(grammar.transitions),
        _groupStack = [_resolveStartMode(grammar)],
        _contextKeywords = Set<String>.from(grammar.contextKeywords);

  String _source;
  final TokenGrammar _grammar;
  final Set<String> _keywordSet;
  final Set<String> _reservedSet;
  final List<_CompiledPattern> _skipPatterns;
  final Map<String, List<_CompiledPattern>> _groupPatterns;
  final Set<String> _groupNames;
  final String _startMode;
  final List<ModeTransition> _transitions;
  final Set<String> _inheritingModes;
  final List<String> _groupStack;
  final Set<String> _contextKeywords;

  /// The validated start mode: `default` unless the grammar names a
  /// declared group. Falls back to `default` on a malformed compiled
  /// artifact so tokenization never fails to construct.
  static String _resolveStartMode(TokenGrammar grammar) {
    final mode = grammar.startMode;
    if (mode != null &&
        (mode == 'default' || grammar.groups.containsKey(mode))) {
      return mode;
    }
    return 'default';
  }

  /// Groups that are *flat modes* (reached via `set-mode`) rather than
  /// *nested regions* (reached via `push`). A flat mode's own patterns
  /// are tried first, then the default group's patterns are inherited.
  /// `push` targets win the classification when a group is both (it is
  /// then treated as exclusive).
  static Set<String> _classifyInheritingModes(
      List<ModeTransition> transitions) {
    final setModeTargets = <String>{};
    final pushTargets = <String>{};
    for (final rule in transitions) {
      for (final action in rule.actions) {
        switch (action) {
          case SetMode(:final mode):
            setModeTargets.add(mode);
          case PushRegion(:final group):
            pushTargets.add(group);
          default:
            break;
        }
      }
    }
    return setModeTargets
        .where((m) => m != 'default' && !pushTargets.contains(m))
        .toSet();
  }

  OnTokenCallback? _onToken;
  final List<PreTokenizeHook> _preHooks = [];
  final List<PostTokenizeHook> _postHooks = [];
  bool _skipEnabled = true;
  Token? _previousToken;
  bool _newlineBeforeNext = false;
  bool _spaceBeforeNext = false;

  int _position = 0;
  int _line = 1;
  int _column = 1;

  /// The currently active group (top of the group stack).
  String get activeGroup => _groupStack.last;

  /// The current group stack depth (always at least 1).
  int get groupStackDepth => _groupStack.length;

  /// Registers the on-token callback (pass `null` to clear).
  ///
  /// The callback fires after each token match, before emission — but
  /// never for skip matches, callback-emitted tokens, NEWLINE, or EOF.
  void setOnToken(OnTokenCallback? callback) {
    _onToken = callback;
  }

  /// Registers a source transform to run before tokenization.
  ///
  /// The hook receives the source string and returns a (possibly
  /// modified) source string. Multiple hooks compose left-to-right.
  /// The transformed source replaces the original (like the
  /// reference), so positions in the output refer to it.
  void addPreTokenize(PreTokenizeHook hook) {
    _preHooks.add(hook);
  }

  /// Registers a token transform to run after tokenization.
  ///
  /// The hook receives the full token list — including the trailing
  /// EOF — and returns a (possibly modified) token list. Multiple
  /// hooks compose left-to-right.
  void addPostTokenize(PostTokenizeHook hook) {
    _postHooks.add(hook);
  }

  /// Tokenizes the source according to the grammar.
  ///
  /// Throws [LexerError] on unexpected sequences and reserved keywords,
  /// [UnsupportedError] for indentation/layout grammars.
  List<Token> tokenize() {
    if (_grammar.mode == 'indentation' || _grammar.mode == 'layout') {
      throw UnsupportedError(
          'GrammarLexer serves standard mode; use grammarTokenize() for ${_grammar.mode} grammars.');
    }
    // Stage 1: pre-tokenize hooks transform the source text.
    if (_preHooks.isNotEmpty) {
      var source = _source;
      for (final hook in _preHooks) {
        source = hook(source);
      }
      _source = source;
    }
    var tokens = <Token>[];
    final hasSkipPatterns = _skipPatterns.isNotEmpty;

    while (_position < _source.length) {
      // Skip patterns take over whitespace handling when the grammar has
      // them and skipping is enabled.
      if (hasSkipPatterns) {
        if (_skipEnabled && _trySkip()) {
          continue;
        }
      } else {
        final ch = _source.codeUnitAt(_position);
        if (ch == spaceCodePoint ||
            ch == tabCodePoint ||
            ch == carriageReturnCodePoint) {
          _noteBuiltinWhitespace(ch);
          _advanceText(_source.substring(_position, _position + 1));
          continue;
        }
      }

      // A bare `\n` becomes the built-in NEWLINE token, checked before
      // any grammar pattern. It never fires the callback, never updates
      // lookbehind, but arms the gap flags for the next token.
      if (_source.codeUnitAt(_position) == lineFeedCodePoint) {
        final newline =
            Token(type: 'NEWLINE', value: r'\n', line: _line, column: _column);
        // The built-in NEWLINE drives the transition table exactly like
        // a declared token. Facts are read before the cursor moves: the
        // gap is still pending, and the next character is the one after
        // this newline.
        _applyTransitions(
            newline,
            _TokenFacts(
              spaceBefore: _spaceBeforeNext,
              lineStart: _atLineStart(),
              nextChar: _charAfter(_position + 1),
            ));
        tokens.add(newline);
        _advanceText('\n');
        _noteBuiltinNewline();
        continue;
      }

      final active = _groupStack.last;
      // Whether this token starts a line, read before the cursor moves
      // past it.
      final tokenLineStart = _atLineStart();
      final match = _matchInGroup(active);
      if (match == null) {
        throw LexerError(messageUnexpectedSequence, _line, _column);
      }
      final (definition, matched) = match;
      final startLine = _line;
      final startColumn = _column;
      _advanceText(matched);

      final effectiveName = definition.alias ?? definition.name;
      final type = _resolveType(
          definition.name, effectiveName, matched, startLine, startColumn);
      final value = _resolveValue(effectiveName, matched, type);

      var flagBits = 0;
      if ((definition.name == 'NAME' || effectiveName == 'NAME') &&
          _contextKeywords.contains(value)) {
        flagBits |= TOKEN_CONTEXT_KEYWORD;
      }
      flagBits |= _takeGapFlags();

      final token = Token(
        type: type,
        value: value,
        line: startLine,
        column: startColumn,
        flags: flagBits == 0 ? null : flagBits,
      );

      final callback = _onToken;
      if (callback == null) {
        // Consult the declarative table before the token lands, so the
        // new mode governs the NEXT match.
        _applyTransitions(
            token,
            _TokenFacts(
              spaceBefore: flagBits & TOKEN_PRECEDED_BY_SPACE != 0,
              lineStart: tokenLineStart,
              nextChar: _charAfter(_position),
            ));
        _previousToken = token;
        tokens.add(token);
        continue;
      }

      final ctx = LexerContext(
        groupNames: _groupNames,
        groupStack: _groupStack,
        source: _source,
        posAfterToken: _position,
        previousToken: _previousToken,
      );
      callback(token, ctx);

      Token? mainToken;
      if (!ctx.suppressed) {
        mainToken = token;
        _previousToken = token;
        tokens.add(token);
      }

      // A suppressed token's gap lands on the first emitted
      // replacement, if any — else it stays pending (below).
      int? unplacedGap = mainToken == null ? flagBits : null;
      for (final action in ctx.actions) {
        switch (action) {
          case PushGroup(:final group):
            _groupStack.add(group);
          case PopGroup():
            if (_groupStack.length > 1) {
              _groupStack.removeLast();
            }
          case EmitToken(:final token):
            final bits = unplacedGap;
            unplacedGap = null;
            if (bits != null && bits != 0) {
              tokens.add(Token(
                type: token.type,
                value: token.value,
                line: token.line,
                column: token.column,
                flags: (token.flags ?? 0) | bits,
              ));
              _previousToken = tokens.last;
            } else {
              _previousToken = token;
              tokens.add(token);
            }
          case SetSkipEnabled(:final enabled):
            _skipEnabled = enabled;
        }
      }
      // The declarative table is the default; a registered callback
      // runs first (above) and the table refines. A suppressed token
      // that nothing replaced drives no transitions.
      final main = mainToken;
      if (main != null) {
        _applyTransitions(
            main,
            _TokenFacts(
              spaceBefore: flagBits & TOKEN_PRECEDED_BY_SPACE != 0,
              lineStart: tokenLineStart,
              nextChar: _charAfter(_position),
            ));
      }
      final pending = unplacedGap;
      if (pending != null) {
        _restoreGapFlags(pending, matched);
      }
    }

    tokens.add(Token(type: 'EOF', value: '', line: _line, column: _column));

    // Stage 3: post-tokenize hooks transform the token list.
    for (final hook in _postHooks) {
      tokens = hook(tokens);
    }

    // Reset for reuse (the cursor stays spent, like the reference).
    _groupStack
      ..clear()
      ..add(_startMode);
    _skipEnabled = true;
    _takeGapFlags();
    return List<Token>.unmodifiable(tokens);
  }

  /// Advances the cursor over matched [text], tracking line/column.
  ///
  /// `position` indexes UTF-16 code units (regex offsets) while `column`
  /// counts code points.
  void _advanceText(String text) {
    for (final codePoint in text.runes) {
      _position += codePoint > maxBmpCodePoint
          ? utf16UnitsPerAstralCodePoint
          : utf16UnitsPerBmpCodePoint;
      if (codePoint == lineFeedCodePoint) {
        _line += 1;
        _column = 1;
      } else {
        _column += 1;
      }
    }
  }

  /// Tries each skip pattern in order; consumes the first match.
  bool _trySkip() {
    for (final pattern in _skipPatterns) {
      final text = pattern.match(_source, _position);
      if (text == null) continue;
      if (_lineTerminator.hasMatch(text)) {
        _newlineBeforeNext = true;
      }
      _spaceBeforeNext = true;
      _advanceText(text);
      return true;
    }
    return false;
  }

  /// Records a builtin-whitespace skip (`\r` also arms the newline bit).
  void _noteBuiltinWhitespace(int codeUnit) {
    _spaceBeforeNext = true;
    if (codeUnit == carriageReturnCodePoint) {
      _newlineBeforeNext = true;
    }
  }

  /// Records a builtin NEWLINE gap for the next token.
  void _noteBuiltinNewline() {
    _spaceBeforeNext = true;
    _newlineBeforeNext = true;
  }

  /// Takes the pending gap bits as token flags, clearing them.
  int _takeGapFlags() {
    var bits = 0;
    if (_newlineBeforeNext) bits |= TOKEN_PRECEDED_BY_NEWLINE;
    if (_spaceBeforeNext) bits |= TOKEN_PRECEDED_BY_SPACE;
    _newlineBeforeNext = false;
    _spaceBeforeNext = false;
    return bits;
  }

  /// Returns a suppressed token's gap (plus its own text) to the pending
  /// gap for the next matched token.
  void _restoreGapFlags(int gapBits, String suppressedText) {
    _spaceBeforeNext = true;
    if (gapBits & TOKEN_PRECEDED_BY_NEWLINE != 0 ||
        _lineTerminator.hasMatch(suppressedText)) {
      _newlineBeforeNext = true;
    }
  }

  /// First-match-wins over [patterns] at the cursor.
  (TokenDefinition, String)? _firstMatch(List<_CompiledPattern> patterns) {
    for (final pattern in patterns) {
      final text = pattern.match(_source, _position);
      if (text == null) continue;
      return (pattern.definition, text);
    }
    return null;
  }

  /// First-match-wins in [groupName], with flat-mode inheritance.
  ///
  /// The group's own patterns are tried first; a flat mode (a
  /// `set-mode` target) then falls through to the default group's
  /// patterns, while a nested region (a `push` target) stays
  /// exclusive. An unknown group falls back to the default patterns
  /// so tokenization never throws on a malformed compiled artifact.
  (TokenDefinition, String)? _matchInGroup(String groupName) {
    final patterns = _groupPatterns[groupName];
    if (patterns != null) {
      final match = _firstMatch(patterns);
      if (match != null) return match;
      if (groupName == 'default' || !_inheritingModes.contains(groupName)) {
        return null;
      }
    }
    return _firstMatch(_groupPatterns['default']!);
  }

  /// Applies the declarative mode transition table after [token].
  ///
  /// The first rule whose trigger token, optional `in MODE` guard,
  /// optional value guard, and every `if` condition match wins; its
  /// actions run in order. An empty table is a no-op (behavior is
  /// then identical to the pre-table engine).
  void _applyTransitions(Token token, _TokenFacts facts) {
    if (_transitions.isEmpty) {
      return;
    }
    final active = _groupStack.last;
    List<TransitionAction>? matched;
    for (final rule in _transitions) {
      if (!rule.onTokens.contains(token.type)) {
        continue;
      }
      final inMode = rule.inMode;
      if (inMode != null && inMode != active) {
        continue;
      }
      final onValue = rule.onValue;
      if (onValue != null && onValue != token.value) {
        continue;
      }
      if (!rule.conditions.every(facts.holds)) {
        continue;
      }
      matched = rule.actions;
      break;
    }
    if (matched == null) {
      return;
    }
    for (final action in matched) {
      switch (action) {
        case SetMode(:final mode):
          _groupStack[_groupStack.length - 1] = mode;
        case PushRegion(:final group):
          _groupStack.add(group);
        case PopRegion():
          if (_groupStack.length > 1) {
            _groupStack.removeLast();
          }
        case EnableSkip():
          _skipEnabled = true;
        case DisableSkip():
          _skipEnabled = false;
        case RefuseToken():
          throw LexerError(messageTokenRefusedInMode, token.line, token.column);
      }
    }
  }

  /// Whether the cursor sits at a line start (input start or right
  /// after `\n`).
  bool _atLineStart() =>
      _position == 0 || _source.codeUnitAt(_position - 1) == lineFeedCodePoint;

  /// The code point at UTF-16 offset [offset], or `null` past the end
  /// of input.
  int? _charAfter(int offset) {
    if (offset >= _source.length) return null;
    final unit = _source.codeUnitAt(offset);
    if (unit & 0xFC00 != 0xD800 || offset + 1 >= _source.length) {
      return unit;
    }
    final low = _source.codeUnitAt(offset + 1);
    if (low & 0xFC00 != 0xDC00) return unit;
    return 0x10000 + ((unit & 0x3FF) << 10) + (low & 0x3FF);
  }

  /// Resolves the emitted type: reserved rejection, keyword promotion,
  /// else the effective (alias-resolved) grammar name verbatim.
  String _resolveType(
      String name, String effectiveName, String matched, int line, int column) {
    final lookup = _grammar.caseInsensitive ? matched.toUpperCase() : matched;
    if (name == 'NAME' && _reservedSet.contains(lookup)) {
      throw LexerError(messageReservedKeyword, line, column);
    }
    if (name == 'NAME' && _keywordSet.contains(lookup)) {
      return 'KEYWORD';
    }
    return effectiveName;
  }

  /// Resolves the emitted value: STRING processing, case-insensitive
  /// keyword normalization, else the raw match.
  String _resolveValue(String effectiveName, String matched, String type) {
    if (effectiveName.endsWith('STRING') &&
        matched.length >= 2 &&
        _isQuoted(matched)) {
      final quoteLength = _quoteLength(matched);
      final inner =
          matched.substring(quoteLength, matched.length - quoteLength);
      if (_grammar.escapeMode == 'none') {
        return inner;
      }
      return _processEscapes(inner);
    }
    if (_grammar.caseInsensitive && type == 'KEYWORD') {
      return matched.toUpperCase();
    }
    return matched;
  }

  /// Whether [text] starts and ends with matching quote characters
  /// (single or triple, either flavor).
  bool _isQuoted(String text) {
    if (text.length >= 6 &&
        ((text.startsWith('"""') && text.endsWith('"""')) ||
            (text.startsWith("'''") && text.endsWith("'''")))) {
      return true;
    }
    if (text.length >= 2) {
      final first = text.codeUnitAt(0);
      final last = text.codeUnitAt(text.length - 1);
      return (first == 0x22 && last == 0x22) || (first == 0x27 && last == 0x27);
    }
    return false;
  }

  /// The quote width on each side: 3 for triple quotes, else 1.
  int _quoteLength(String text) {
    if (text.length >= 6 &&
        (text.startsWith('"""') || text.startsWith("'''"))) {
      return 3;
    }
    return 1;
  }

  /// Decodes `\n`, `\t`, `\r`, `\\`, `\"`, `\'`; any other escaped
  /// character stands for itself.
  String _processEscapes(String text) {
    final buffer = StringBuffer();
    final runes = text.runes.toList();
    var i = 0;
    while (i < runes.length) {
      if (runes[i] == 0x5C && i + 1 < runes.length) {
        final next = runes[i + 1];
        switch (next) {
          case 0x6E:
            buffer.write('\n');
          case 0x74:
            buffer.write('\t');
          case 0x72:
            buffer.write('\r');
          case 0x5C:
            buffer.write(r'\');
          case 0x22:
            buffer.write('"');
          case 0x27:
            buffer.write("'");
          default:
            buffer.writeCharCode(next);
        }
        i += 2;
      } else {
        buffer.writeCharCode(runes[i]);
        i += 1;
      }
    }
    return buffer.toString();
  }
}
