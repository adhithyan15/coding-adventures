import 'dart:collection';

final RegExp _magicCommentPattern = RegExp(r'^#\s*@(\w+)\s*(.*)$');

class TokenGrammarError implements Exception {
  TokenGrammarError(this.message, this.lineNumber);

  final String message;
  final int lineNumber;

  @override
  String toString() => 'Line $lineNumber: $message';
}

class TokenDefinition {
  const TokenDefinition({
    required this.name,
    required this.pattern,
    required this.isRegex,
    required this.lineNumber,
    this.alias,
  });

  final String name;
  final String pattern;
  final bool isRegex;
  final int lineNumber;
  final String? alias;

  @override
  bool operator ==(Object other) {
    return other is TokenDefinition &&
        other.name == name &&
        other.pattern == pattern &&
        other.isRegex == isRegex &&
        other.lineNumber == lineNumber &&
        other.alias == alias;
  }

  @override
  int get hashCode => Object.hash(name, pattern, isRegex, lineNumber, alias);
}

class PatternGroup {
  const PatternGroup({required this.name, required this.definitions});

  final String name;
  final List<TokenDefinition> definitions;

  @override
  bool operator ==(Object other) {
    return other is PatternGroup &&
        other.name == name &&
        _listEquals(other.definitions, definitions);
  }

  @override
  int get hashCode => Object.hash(name, Object.hashAll(definitions));
}

/// One action in a transition rule's action list.
sealed class TransitionAction {
  const TransitionAction();
}

/// Replaces the active mode in place (flat toggle, no depth change).
final class SetMode extends TransitionAction {
  const SetMode(this.mode);
  final String mode;

  @override
  bool operator ==(Object other) => other is SetMode && other.mode == mode;

  @override
  int get hashCode => mode.hashCode;

  @override
  String toString() => 'SetMode($mode)';
}

/// Pushes a nested region (exclusive: no default-pattern inheritance).
final class PushRegion extends TransitionAction {
  const PushRegion(this.group);
  final String group;

  @override
  bool operator ==(Object other) => other is PushRegion && other.group == group;

  @override
  int get hashCode => group.hashCode;

  @override
  String toString() => 'PushRegion($group)';
}

/// Pops a nested region (never pops the last mode).
final class PopRegion extends TransitionAction {
  const PopRegion();

  @override
  bool operator ==(Object other) => other is PopRegion;

  @override
  int get hashCode => 0;

  @override
  String toString() => 'PopRegion()';
}

/// Re-enables skip-pattern processing.
final class EnableSkip extends TransitionAction {
  const EnableSkip();

  @override
  bool operator ==(Object other) => other is EnableSkip;

  @override
  int get hashCode => 1;

  @override
  String toString() => 'EnableSkip()';
}

/// Disables skip-pattern processing (significant whitespace).
final class DisableSkip extends TransitionAction {
  const DisableSkip();

  @override
  bool operator ==(Object other) => other is DisableSkip;

  @override
  int get hashCode => 2;

  @override
  String toString() => 'DisableSkip()';
}

/// Fails lexing: the token is not supported in the active mode.
final class RefuseToken extends TransitionAction {
  const RefuseToken();

  @override
  bool operator ==(Object other) => other is RefuseToken;

  @override
  int get hashCode => 3;

  @override
  String toString() => 'RefuseToken()';
}

/// One condition in a transition rule's `if` list: reads one fact
/// about the token just emitted; all must hold for the rule to fire.
enum TransitionCondition {
  spaceBefore,
  noSpaceBefore,
  spaceAfter,
  noSpaceAfter,
  lineStart,
  notLineStart,
}

/// One declarative lexer mode transition rule, parsed from a
/// `transitions:` section line of the form
/// `on TOKENS [in MODE] [if COND && ...] -> ACTION [, ACTION ...]`.
///
/// Pure data — the lexer interprets it after each token is emitted
/// (first matching rule wins).
class ModeTransition {
  const ModeTransition({
    required this.onTokens,
    this.onValue,
    this.inMode,
    this.conditions = const [],
    required this.actions,
    required this.lineNumber,
  });

  /// Trigger token names; one entry for a bare name, several for an
  /// `(A | B)` alternation.
  final List<String> onTokens;

  /// Value guard from the `KEYWORD="value"` form, if any.
  final String? onValue;

  /// Active-mode guard from `in MODE`, if any.
  final String? inMode;

  /// `if` conditions, all of which must hold.
  final List<TransitionCondition> conditions;

  /// Actions to apply in order when the rule fires.
  final List<TransitionAction> actions;

  final int lineNumber;

  @override
  bool operator ==(Object other) {
    return other is ModeTransition &&
        _listEquals(other.onTokens, onTokens) &&
        other.onValue == onValue &&
        other.inMode == inMode &&
        _listEquals(other.conditions, conditions) &&
        _listEquals(other.actions, actions) &&
        other.lineNumber == lineNumber;
  }

  @override
  int get hashCode => Object.hash(
        Object.hashAll(onTokens),
        onValue,
        inMode,
        Object.hashAll(conditions),
        Object.hashAll(actions),
        lineNumber,
      );
}

/// Upper bound on the number of transition rules in one grammar.
///
/// A defensive cap so an untrusted `.tokens` file cannot blow up
/// generated-code size or lexer memory with a pathological transition
/// table. No real grammar approaches this.
const int maxTransitions = 4096;

class TokenGrammar {
  TokenGrammar({
    this.version = 0,
    this.caseInsensitive = false,
    this.definitions = const [],
    this.keywords = const [],
    this.mode,
    this.skipDefinitions = const [],
    this.reservedKeywords = const [],
    this.escapeMode,
    this.errorDefinitions = const [],
    Map<String, PatternGroup> groups = const {},
    this.caseSensitive = true,
    this.layoutKeywords = const [],
    this.contextKeywords = const [],
    this.softKeywords = const [],
    this.startMode,
    this.transitions = const [],
  }) : groups = UnmodifiableMapView(Map<String, PatternGroup>.from(groups));

  final int version;
  final bool caseInsensitive;
  final List<TokenDefinition> definitions;
  final List<String> keywords;
  final String? mode;
  final List<TokenDefinition> skipDefinitions;
  final List<String> reservedKeywords;
  final String? escapeMode;
  final List<TokenDefinition> errorDefinitions;
  final Map<String, PatternGroup> groups;
  final bool caseSensitive;
  final List<String> layoutKeywords;
  final List<String> contextKeywords;
  final List<String> softKeywords;

  /// The lexer mode (active group) the tokenizer begins in. `null`
  /// means `"default"`. Set by the `start_mode:` directive.
  final String? startMode;

  /// Declarative lexer mode transition table. Empty means no
  /// transitions — the lexer never switches modes on its own. Set by
  /// the `transitions:` section.
  final List<ModeTransition> transitions;

  Set<String> tokenNames() {
    final names = <String>{};
    for (final definition in _allDefinitions()) {
      names.add(definition.name);
      final alias = definition.alias;
      if (alias != null) {
        names.add(alias);
      }
    }
    return names;
  }

  Set<String> effectiveTokenNames() {
    return _allDefinitions()
        .map((definition) => definition.alias ?? definition.name)
        .toSet();
  }

  Iterable<TokenDefinition> _allDefinitions() sync* {
    yield* definitions;
    for (final group in groups.values) {
      yield* group.definitions;
    }
  }
}

TokenGrammar parseTokenGrammar(String source) {
  var version = 0;
  var caseInsensitive = false;
  String? mode;
  String? escapeMode;
  var caseSensitive = true;

  final definitions = <TokenDefinition>[];
  final keywords = <String>[];
  final skipDefinitions = <TokenDefinition>[];
  final reservedKeywords = <String>[];
  final errorDefinitions = <TokenDefinition>[];
  final groups = <String, PatternGroup>{};
  final contextKeywords = <String>[];
  final layoutKeywords = <String>[];
  final softKeywords = <String>[];
  String? startMode;
  final transitions = <ModeTransition>[];

  String? currentSection;
  final lines = source.split('\n');

  for (var index = 0; index < lines.length; index++) {
    final lineNumber = index + 1;
    final line = lines[index].replaceFirst(RegExp(r'\r$'), '');
    final stripped = line.trim();

    if (stripped.isEmpty) {
      continue;
    }

    if (stripped.startsWith('#')) {
      final match = _magicCommentPattern.firstMatch(stripped);
      if (match != null) {
        final key = match.group(1)!;
        final value = (match.group(2) ?? '').trim();
        if (key == 'version') {
          final parsed = int.tryParse(value);
          if (parsed != null) {
            version = parsed;
          }
        } else if (key == 'case_insensitive') {
          caseInsensitive = value == 'true';
        }
      }
      continue;
    }

    if (stripped.startsWith('mode:')) {
      final value = stripped.substring(5).trim();
      if (value.isEmpty) {
        throw TokenGrammarError("Missing value after 'mode:'", lineNumber);
      }
      mode = value;
      currentSection = null;
      continue;
    }

    if (stripped.startsWith('escapes:')) {
      final value = stripped.substring(8).trim();
      if (value.isEmpty) {
        throw TokenGrammarError("Missing value after 'escapes:'", lineNumber);
      }
      escapeMode = value;
      currentSection = null;
      continue;
    }

    if (stripped.startsWith('start_mode:') ||
        stripped.startsWith('start_mode :')) {
      final colonIndex = stripped.indexOf(':');
      final value = stripped.substring(colonIndex + 1).trim();
      if (value.isEmpty) {
        throw TokenGrammarError(
          "Missing mode name after 'start_mode:'",
          lineNumber,
        );
      }
      startMode = value;
      currentSection = null;
      continue;
    }

    if (stripped.startsWith('case_sensitive:')) {
      final value = stripped.substring(15).trim().toLowerCase();
      if (value != 'true' && value != 'false') {
        throw TokenGrammarError(
          "Invalid value for 'case_sensitive:': '$value' (expected 'true' or 'false')",
          lineNumber,
        );
      }
      caseSensitive = value == 'true';
      currentSection = null;
      continue;
    }

    if (stripped.startsWith('group ') && stripped.endsWith(':')) {
      final groupName = stripped.substring(6, stripped.length - 1).trim();
      if (groupName.isEmpty) {
        throw TokenGrammarError("Missing group name after 'group'", lineNumber);
      }
      if (!RegExp(r'^[a-z_][a-z0-9_]*$').hasMatch(groupName)) {
        throw TokenGrammarError(
          "Invalid group name: '$groupName' (must be a lowercase identifier like 'tag' or 'cdata')",
          lineNumber,
        );
      }
      const reservedNames = {
        'default',
        'skip',
        'keywords',
        'reserved',
        'errors',
        'layout_keywords',
        'context_keywords',
        'soft_keywords',
        'transitions',
        'modes',
        'start_mode',
      };
      if (reservedNames.contains(groupName)) {
        throw TokenGrammarError(
          "Reserved group name: '$groupName'",
          lineNumber,
        );
      }
      if (groups.containsKey(groupName)) {
        throw TokenGrammarError(
          "Duplicate group name: '$groupName'",
          lineNumber,
        );
      }
      groups[groupName] = PatternGroup(name: groupName, definitions: const []);
      currentSection = 'group:$groupName';
      continue;
    }

    if (stripped == 'keywords:' || stripped == 'keywords :') {
      currentSection = 'keywords';
      continue;
    }
    if (stripped == 'reserved:' || stripped == 'reserved :') {
      currentSection = 'reserved';
      continue;
    }
    if (stripped == 'skip:' || stripped == 'skip :') {
      currentSection = 'skip';
      continue;
    }
    if (stripped == 'errors:' || stripped == 'errors :') {
      currentSection = 'errors';
      continue;
    }
    if (stripped == 'context_keywords:' || stripped == 'context_keywords :') {
      currentSection = 'context_keywords';
      continue;
    }
    if (stripped == 'layout_keywords:' || stripped == 'layout_keywords :') {
      currentSection = 'layout_keywords';
      continue;
    }
    if (stripped == 'soft_keywords:' || stripped == 'soft_keywords :') {
      currentSection = 'soft_keywords';
      continue;
    }
    if (stripped == 'transitions:' || stripped == 'transitions :') {
      currentSection = 'transitions';
      continue;
    }

    if (currentSection != null) {
      final firstChar = line.isEmpty ? '' : line[0];
      if (firstChar == ' ' || firstChar == '\t') {
        if (currentSection == 'keywords') {
          keywords.add(stripped);
        } else if (currentSection == 'reserved') {
          reservedKeywords.add(stripped);
        } else if (currentSection == 'context_keywords') {
          contextKeywords.add(stripped);
        } else if (currentSection == 'layout_keywords') {
          layoutKeywords.add(stripped);
        } else if (currentSection == 'soft_keywords') {
          softKeywords.add(stripped);
        } else if (currentSection == 'skip') {
          skipDefinitions.add(
            _parseSectionDefinition(stripped, lineNumber, 'skip pattern'),
          );
        } else if (currentSection == 'errors') {
          errorDefinitions.add(
            _parseSectionDefinition(stripped, lineNumber, 'error pattern'),
          );
        } else if (currentSection == 'transitions') {
          transitions.add(_parseTransition(stripped, lineNumber));
        } else if (currentSection.startsWith('group:')) {
          final groupName = currentSection.substring(6);
          final definition = _parseSectionDefinition(
            stripped,
            lineNumber,
            'group token',
          );
          final group = groups[groupName]!;
          groups[groupName] = PatternGroup(
            name: group.name,
            definitions: [...group.definitions, definition],
          );
        }
        continue;
      }
      currentSection = null;
    }

    final equalsIndex = line.indexOf('=');
    if (equalsIndex == -1) {
      throw TokenGrammarError(
        "Expected token definition (NAME = pattern), got: '$stripped'",
        lineNumber,
      );
    }

    final name = line.substring(0, equalsIndex).trim();
    final pattern = line.substring(equalsIndex + 1).trim();
    if (name.isEmpty) {
      throw TokenGrammarError("Missing token name before '='", lineNumber);
    }
    if (!RegExp(r'^[a-zA-Z_][a-zA-Z0-9_]*$').hasMatch(name)) {
      throw TokenGrammarError(
        "Invalid token name: '$name' (must be an identifier like NAME or PLUS_EQUALS)",
        lineNumber,
      );
    }
    if (pattern.isEmpty) {
      throw TokenGrammarError(
        "Missing pattern after '=' for token '$name'",
        lineNumber,
      );
    }
    definitions.add(_parseDefinition(pattern, name, lineNumber));
  }

  return TokenGrammar(
    version: version,
    caseInsensitive: caseInsensitive,
    definitions: definitions,
    keywords: keywords,
    mode: mode,
    skipDefinitions: skipDefinitions,
    reservedKeywords: reservedKeywords,
    escapeMode: escapeMode,
    errorDefinitions: errorDefinitions,
    groups: groups,
    caseSensitive: caseSensitive,
    layoutKeywords: layoutKeywords,
    contextKeywords: contextKeywords,
    softKeywords: softKeywords,
    startMode: startMode,
    transitions: transitions,
  );
}

/// Parses one `transitions:` line into a [ModeTransition].
///
/// Grammar of a rule line (keywords are lowercase):
///
/// ```text
/// on TOKENS [in MODE] [if COND [&& COND ...]] -> ACTION [, ACTION ...]
/// ```
///
/// where `TOKENS` is one token name (`NAME`), an alternation
/// `(A | B | C)`, or the special `KEYWORD="value"` form for a specific
/// promoted keyword; and each `ACTION` is `set-mode M` | `push G` |
/// `pop` | `enable-skip` | `disable-skip` | `refuse`.
ModeTransition _parseTransition(String line, int lineNumber) {
  // Split on the mandatory `->` arrow separating the matcher from the
  // actions.
  final arrowIndex = line.indexOf('->');
  if (arrowIndex == -1) {
    throw TokenGrammarError(
      "Transition rule missing '->': '$line'",
      lineNumber,
    );
  }
  final head = line.substring(0, arrowIndex).trim();
  final actionStr = line.substring(arrowIndex + 2).trim();
  if (actionStr.isEmpty) {
    throw TokenGrammarError(
      "Transition rule has no actions after '->': '$line'",
      lineNumber,
    );
  }

  // --- Head: `on TOKENS [in MODE] [if COND && ...]` ---
  if (!head.startsWith('on ')) {
    throw TokenGrammarError(
      "Transition rule must start with 'on ': '$line'",
      lineNumber,
    );
  }
  var rest = head.substring(3).trim();

  // Optional `if COND [&& COND ...]` list. It is the last part of the
  // head, so split it off before looking for the `in MODE` guard.
  //
  // `if` is a whitespace-delimited *word*, not the literal `" if "`:
  // with the literal, `on X if -> refuse` and tab-separated `if`
  // would not split, the trigger would become `X if`, and the rule
  // would silently never fire.
  var conditions = <TransitionCondition>[];
  final ifSplit = _splitTopLevelWord(rest, 'if');
  if (ifSplit != null) {
    rest = ifSplit.$1.trim();
    conditions = _parseConditions(ifSplit.$2, line, lineNumber);
  }

  // Optional `in MODE` guard. Split on the whitespace-delimited word
  // so a token-set like `(A | B)` is not mistaken for the guard.
  String? inMode;
  final inSplit = _splitInGuard(rest);
  if (inSplit != null) {
    rest = inSplit.$1.trim();
    inMode = inSplit.$2.trim();
  }

  // --- Token set, with optional `KEYWORD="value"` form ---
  var tokensPart = rest;
  if (tokensPart.startsWith('(') && tokensPart.endsWith(')')) {
    tokensPart = tokensPart.substring(1, tokensPart.length - 1);
  }
  String? onValue;
  final onTokens = <String>[];
  for (final raw in tokensPart.split('|')) {
    final item = raw.trim();
    if (item.isEmpty) {
      continue;
    }
    // `KEYWORD="value"` — a value-guarded keyword. Only one value
    // guard is supported per rule; a second overwrites the first,
    // exactly like the reference.
    final eqIndex = item.indexOf('=');
    if (eqIndex != -1) {
      final name = item.substring(0, eqIndex).trim();
      final valueRaw = item.substring(eqIndex + 1).trim();
      var value = valueRaw;
      if (value.length >= 2 && value.startsWith('"') && value.endsWith('"')) {
        value = value.substring(1, value.length - 1);
      }
      onTokens.add(name);
      onValue = value;
    } else {
      onTokens.add(item);
    }
  }
  if (onTokens.isEmpty) {
    throw TokenGrammarError(
      "Transition rule has no trigger tokens: '$line'",
      lineNumber,
    );
  }

  // --- Actions: comma-separated ---
  final actions = <TransitionAction>[];
  for (final raw in actionStr.split(',')) {
    final act = raw.trim();
    if (act.isEmpty) {
      continue;
    }
    final TransitionAction parsed;
    if (act.startsWith('set-mode ')) {
      parsed = SetMode(act.substring(9).trim());
    } else if (act.startsWith('push ')) {
      parsed = PushRegion(act.substring(5).trim());
    } else if (act == 'pop') {
      parsed = const PopRegion();
    } else if (act == 'enable-skip') {
      parsed = const EnableSkip();
    } else if (act == 'disable-skip') {
      parsed = const DisableSkip();
    } else if (act == 'refuse') {
      parsed = const RefuseToken();
    } else {
      throw TokenGrammarError(
        "Unknown transition action '$act' "
        '(expected set-mode/push/pop/enable-skip/disable-skip/refuse)',
        lineNumber,
      );
    }
    actions.add(parsed);
  }
  if (actions.isEmpty) {
    throw TokenGrammarError(
      "Transition rule has no valid actions: '$line'",
      lineNumber,
    );
  }
  // A refusal ends lexing, so any action next to it could never run;
  // writing one is a mistake about what the rule does.
  if (actions.length > 1 && actions.any((a) => a is RefuseToken)) {
    throw TokenGrammarError(
      "'refuse' must be a transition rule's only action: '$line'",
      lineNumber,
    );
  }

  return ModeTransition(
    onTokens: onTokens,
    onValue: onValue,
    inMode: inMode,
    conditions: conditions,
    actions: actions,
    lineNumber: lineNumber,
  );
}

/// Parses the condition list of an `if` clause: `COND [&& COND ...]`,
/// where `COND` is `space_before`, `space_after` or `line_start`,
/// optionally negated with `!`.
List<TransitionCondition> _parseConditions(
  String text,
  String line,
  int lineNumber,
) {
  final conditions = <TransitionCondition>[];
  for (final raw in text.split('&&')) {
    final cond = raw.trim();
    final bool negated;
    final String name;
    if (cond.startsWith('!')) {
      negated = true;
      name = cond.substring(1).trim();
    } else {
      negated = false;
      name = cond;
    }
    final TransitionCondition parsed;
    if (name == 'space_before') {
      parsed = negated
          ? TransitionCondition.noSpaceBefore
          : TransitionCondition.spaceBefore;
    } else if (name == 'space_after') {
      parsed = negated
          ? TransitionCondition.noSpaceAfter
          : TransitionCondition.spaceAfter;
    } else if (name == 'line_start') {
      parsed = negated
          ? TransitionCondition.notLineStart
          : TransitionCondition.lineStart;
    } else {
      throw TokenGrammarError(
        "Unknown transition condition '$cond' in '$line' "
        "(expected space_before, space_after or line_start, optionally negated with '!')",
        lineNumber,
      );
    }
    conditions.add(parsed);
  }
  return conditions;
}

/// ASCII whitespace for transition-head word splitting. The reference
/// uses Unicode `is_whitespace`; grammar sources are ASCII by
/// convention, and anything exotic fails loudly downstream (the
/// mis-split trigger trips the unknown-trigger validation), never
/// silently.
bool _isHeadWhitespace(int codeUnit) {
  return codeUnit == 0x20 ||
      codeUnit == 0x09 ||
      codeUnit == 0x0A ||
      codeUnit == 0x0D ||
      codeUnit == 0x0B ||
      codeUnit == 0x0C;
}

/// Splits [head] at the first occurrence of the *word* [word] outside
/// any parenthesised token set and outside any `"…"` keyword value,
/// returning the text before and after it.
///
/// A word occurrence has whitespace immediately before it and
/// whitespace or the end of [head] immediately after it.
(String, String)? _splitTopLevelWord(String head, String word) {
  var depth = 0;
  var inQuotes = false;
  final units = head.codeUnits;
  for (var i = 0; i < units.length; i++) {
    final c = units[i];
    if (c == 0x22) {
      inQuotes = !inQuotes;
    } else if (c == 0x28 && !inQuotes) {
      depth += 1;
    } else if (c == 0x29 && !inQuotes) {
      depth -= 1;
    } else if (depth == 0 && !inQuotes && _isHeadWhitespace(c)) {
      final rest = head.substring(i + 1);
      if (rest.startsWith(word)) {
        final after = rest.substring(word.length);
        if (after.isEmpty || _isHeadWhitespace(after.codeUnitAt(0))) {
          return (head.substring(0, i), after);
        }
      }
    }
  }
  return null;
}

/// Splits a transition head on a top-level ` in ` mode guard, ignoring
/// any ` in ` inside a parenthesised token set. Returns
/// `(tokens, mode)` when a guard is present, else `null`.
(String, String)? _splitInGuard(String head) {
  var depth = 0;
  for (var i = 0; i < head.length; i++) {
    final c = head.codeUnitAt(i);
    if (c == 0x28) {
      depth += 1;
    } else if (c == 0x29) {
      depth -= 1;
    } else if (c == 0x20 &&
        depth == 0 &&
        head.substring(i).startsWith(' in ')) {
      return (head.substring(0, i), head.substring(i + 4));
    }
  }
  return null;
}

List<String> validateTokenGrammar(TokenGrammar grammar) {
  final issues = <String>[];
  issues.addAll(_validateDefinitions(grammar.definitions, 'token'));
  issues.addAll(_validateDefinitions(grammar.skipDefinitions, 'skip pattern'));
  issues.addAll(
    _validateDefinitions(grammar.errorDefinitions, 'error pattern'),
  );

  if (grammar.mode != null &&
      grammar.mode != 'indentation' &&
      grammar.mode != 'layout') {
    issues.add(
      "Unknown lexer mode '${grammar.mode}' (only 'indentation' and 'layout' are supported)",
    );
  }
  if (grammar.mode == 'layout' && grammar.layoutKeywords.isEmpty) {
    issues.add('Layout mode requires a non-empty layout_keywords section');
  }
  if (grammar.escapeMode != null && grammar.escapeMode != 'none') {
    issues.add(
      "Unknown escape mode '${grammar.escapeMode}' (only 'none' is supported)",
    );
  }

  for (final entry in grammar.groups.entries) {
    final groupName = entry.key;
    final group = entry.value;
    if (!RegExp(r'^[a-z_][a-z0-9_]*$').hasMatch(groupName)) {
      issues.add(
        "Invalid group name '$groupName' (must be a lowercase identifier)",
      );
    }
    if (group.definitions.isEmpty) {
      issues.add("Empty pattern group '$groupName' (has no token definitions)");
    }
    issues.addAll(
      _validateDefinitions(group.definitions, "group '$groupName' token"),
    );
  }

  // A mode exists when it is `default` or a declared group.
  // `set-mode`/`push` targets, `in MODE` guards, and `start_mode` must
  // all resolve to a real mode, otherwise the lexer would silently
  // fall back to the wrong group — a silent-failure hazard rejected
  // up front.
  bool modeExists(String name) =>
      name == 'default' || grammar.groups.containsKey(name);

  final startMode = grammar.startMode;
  if (startMode != null && !modeExists(startMode)) {
    issues.add(
      "start_mode '$startMode' is not 'default' or a declared group",
    );
  }

  // Defensive cap so an untrusted grammar cannot blow up codegen/lexer
  // size.
  if (grammar.transitions.length > maxTransitions) {
    issues.add(
      'Too many transition rules (${grammar.transitions.length}, max $maxTransitions)',
    );
  }

  // Conditions and refusal only take effect where transitions run. The
  // indentation tokenizer never applies the transition table, so a
  // `refuse` rule or an `if` condition there would silently do
  // nothing — reject them instead of ignoring them.
  if (grammar.mode == 'indentation') {
    for (final rule in grammar.transitions) {
      if (rule.actions.any((a) => a is RefuseToken) ||
          rule.conditions.isNotEmpty) {
        issues.add(
          "Transition rule on line ${rule.lineNumber} uses 'refuse' or an "
          "'if' condition, but 'mode: indentation' grammars do not apply "
          'transitions',
        );
      }
    }
  }

  // Every trigger must name a token the lexer can emit: a definition
  // or alias (top level or in a group), an error definition, `KEYWORD`
  // when the grammar has keywords, or a built-in (`NEWLINE`, `INDENT`,
  // `DEDENT`, `EOF`). A misspelt trigger never matches, so the rule —
  // a `refuse` included — would silently never fire.
  final emittable = <String>{'NEWLINE', 'INDENT', 'DEDENT', 'EOF'};
  if (grammar.keywords.isNotEmpty || grammar.reservedKeywords.isNotEmpty) {
    emittable.add('KEYWORD');
  }
  final allDefinitions = [
    ...grammar.definitions,
    ...grammar.errorDefinitions,
    for (final group in grammar.groups.values) ...group.definitions,
  ];
  for (final definition in allDefinitions) {
    emittable.add(definition.name);
    final alias = definition.alias;
    if (alias != null) {
      emittable.add(alias);
    }
  }

  for (final rule in grammar.transitions) {
    for (final token in rule.onTokens) {
      if (!emittable.contains(token)) {
        issues.add(
          "Transition trigger '$token' (line ${rule.lineNumber}) is not a token this grammar can emit",
        );
      }
    }
    final inMode = rule.inMode;
    if (inMode != null && !modeExists(inMode)) {
      issues.add(
        "Transition guard 'in $inMode' (line ${rule.lineNumber}) names an undeclared mode",
      );
    }
    for (final action in rule.actions) {
      final String? target = switch (action) {
        SetMode(:final mode) => mode,
        PushRegion(:final group) => group,
        _ => null,
      };
      if (target != null && !modeExists(target)) {
        issues.add(
          "Transition action targets undeclared mode '$target' (line ${rule.lineNumber})",
        );
      }
    }
  }

  return issues;
}

TokenDefinition _parseSectionDefinition(
  String stripped,
  int lineNumber,
  String label,
) {
  final equalsIndex = stripped.indexOf('=');
  if (equalsIndex == -1) {
    throw TokenGrammarError(
      "Expected $label definition (NAME = pattern), got: '$stripped'",
      lineNumber,
    );
  }
  final name = stripped.substring(0, equalsIndex).trim();
  final pattern = stripped.substring(equalsIndex + 1).trim();
  if (name.isEmpty || pattern.isEmpty) {
    throw TokenGrammarError(
      "Incomplete $label definition: '$stripped'",
      lineNumber,
    );
  }
  return _parseDefinition(pattern, name, lineNumber);
}

/// Normalizes Rust-regex patterns for Dart's [RegExp] (unicode
/// mode): expands `(?i:...)` scoped case-insensitive groups into
/// explicit ASCII case pairs (identical semantics over ASCII text;
/// every current use is ASCII-only) and rewrites the Annex-B-only
/// `\-` escape to `\x2D` (identical in every position). Other inline
/// flags (`(?m:)`, `(?s:)`, …) fail fast with a clear error instead
/// of a cryptic `RegExp` failure downstream.
String _normalizeRegexForDart(String pattern, String name, int lineNumber) {
  if (!pattern.contains('(?') && !pattern.contains(r'\-')) {
    return pattern;
  }
  final out = StringBuffer();
  // Sensitivity stack over an iterative scan (a `(?i:` marker pushes
  // `true`), so deeply nested patterns cannot overflow the call
  // stack, and nested `(?i:` groups stay insensitive without double
  // expansion.
  final stack = [false];
  var i = 0;
  while (i < pattern.length) {
    final c = pattern[i];
    final insensitive = stack.last;
    if (c == '\\' && i + 1 < pattern.length) {
      final escaped = pattern[i + 1];
      if (escaped == '-') {
        // `\-` is an Annex-B-only escape: Rust accepts it, but Dart's
        // RegExp in unicode mode rejects it. `\x2D` is identical in
        // every position (in or out of a class) with zero risk of
        // forming a range.
        out.write(r'\x2D');
        i += 2;
        continue;
      }
      if (insensitive &&
          (escaped == 'p' ||
              escaped == 'P' ||
              escaped == 'x' ||
              escaped == 'u')) {
        // Their payloads may hold foldable letters; none occur in any
        // shipped grammar, so fail fast instead of miscompiling.
        throw TokenGrammarError(
          "Escape '\\$escaped' inside (?i:...) in token '$name' needs "
          'explicit case handling',
          lineNumber,
        );
      }
      // Other escapes pass through: flags never change their meaning
      // (word/space classes already span cases; anchors and
      // properties are explicit sets).
      out.write(pattern.substring(i, i + 2));
      i += 2;
      continue;
    }
    if (c == '[') {
      final end = _classEnd(pattern, i, name, lineNumber);
      out.write(_expandClass(pattern.substring(i, end + 1), insensitive));
      i = end + 1;
      continue;
    }
    if (c == '(' && pattern.startsWith('(?i:', i)) {
      out.write('(?:');
      stack.add(true);
      i += 4;
      continue;
    }
    if (c == '(' &&
        pattern.startsWith('(?', i) &&
        i + 2 < pattern.length &&
        _isAsciiLetter(pattern[i + 2])) {
      throw TokenGrammarError(
        "Unsupported inline flag group '(?${pattern[i + 2]}...)' in token "
        "'$name': Dart RegExp accepts (?i:...) (expanded here) but no "
        'other inline flags',
        lineNumber,
      );
    }
    if (c == '(') {
      stack.add(insensitive);
      out.write(c);
      i++;
      continue;
    }
    if (c == ')') {
      if (stack.length > 1) {
        stack.removeLast();
      }
      out.write(c);
      i++;
      continue;
    }
    if (insensitive && _isAsciiLetter(c)) {
      out.write('[${c.toLowerCase()}${c.toUpperCase()}]');
    } else {
      out.write(c);
    }
    i++;
  }
  return out.toString();
}

bool _isAsciiLetter(String c) {
  final code = c.codeUnitAt(0);
  return (code >= 0x41 && code <= 0x5A) || (code >= 0x61 && code <= 0x7A);
}

/// Index of the `]` closing the class opened at [open]. A `]` first
/// (or right after `^`) is a literal, matching regex semantics. An
/// unterminated class fails fast (the engine's [RegExp] would reject
/// it less clearly downstream).
int _classEnd(String s, int open, String name, int lineNumber) {
  var i = open + 1;
  if (i < s.length && s[i] == '^') {
    i++;
  }
  if (i < s.length && s[i] == ']') {
    i++;
  }
  while (i < s.length) {
    if (s[i] == '\\' && i + 1 < s.length) {
      i += 2;
      continue;
    }
    if (s[i] == ']') {
      return i;
    }
    i++;
  }
  throw TokenGrammarError(
    "Unterminated character class in token '$name'",
    lineNumber,
  );
}

/// Expands ASCII letters inside the class [text] (brackets included)
/// when [insensitive]: single letters become pairs and same-case
/// ranges gain their flipped range (`[a-z]` → `[a-zA-Z]`). Mixed or
/// non-letter ranges pass through (they already span cases or have
/// none to fold), as does everything when sensitive.
String _expandClass(String text, bool insensitive) {
  // `\-` normalization always runs (it is not case-related); letter
  // folding below is gated on [insensitive] per item.
  final out = StringBuffer('[');
  var i = 1;
  if (i < text.length && text[i] == '^') {
    out.write('^');
    i++;
  }
  // A leading ']' is a literal member, not the closer.
  if (i < text.length && text[i] == ']') {
    out.write(']');
    i++;
  }
  while (i < text.length - 1) {
    if (text[i] == '\\' && i + 1 < text.length - 1) {
      // Same `\-` normalization as outside classes (`\x2D` cannot
      // form a range, unlike a bare `-` rewrite).
      if (text[i + 1] == '-') {
        out.write(r'\x2D');
      } else {
        out.write(text.substring(i, i + 2));
      }
      i += 2;
      continue;
    }
    final lo = text[i];
    if (insensitive &&
        i + 2 < text.length &&
        text[i + 1] == '-' &&
        text[i + 2] != ']' &&
        _isAsciiLetter(lo) &&
        _isAsciiLetter(text[i + 2])) {
      final hi = text[i + 2];
      final loLower = lo.toLowerCase();
      final hiLower = hi.toLowerCase();
      final loUpper = lo.toUpperCase();
      final hiUpper = hi.toUpperCase();
      if (lo == loLower && hi == hiLower) {
        out.write('$lo-$hi$loUpper-$hiUpper');
      } else if (lo == loUpper && hi == hiUpper) {
        out.write('$lo-$hi$loLower-$hiLower');
      } else {
        out.write('$lo-$hi');
      }
      i += 3;
      continue;
    }
    if (insensitive && _isAsciiLetter(lo)) {
      // Bare pair, not a nested class (which regex syntax forbids).
      out.write('${lo.toLowerCase()}${lo.toUpperCase()}');
    } else {
      out.write(lo);
    }
    i++;
  }
  out.write(']');
  return out.toString();
}

TokenDefinition _parseDefinition(
  String patternPart,
  String name,
  int lineNumber,
) {
  if (patternPart.startsWith('/')) {
    final slashIndex = _findClosingSlash(patternPart);
    if (slashIndex == -1) {
      throw TokenGrammarError(
        "Unclosed regex pattern for token '$name'",
        lineNumber,
      );
    }
    final body = patternPart.substring(1, slashIndex);
    if (body.isEmpty) {
      throw TokenGrammarError(
        "Empty regex pattern for token '$name'",
        lineNumber,
      );
    }
    final normalized = _normalizeRegexForDart(body, name, lineNumber);
    final remainder = patternPart.substring(slashIndex + 1).trim();
    final alias = _parseAliasRemainder(remainder, name, lineNumber);
    return TokenDefinition(
      name: name,
      pattern: normalized,
      isRegex: true,
      lineNumber: lineNumber,
      alias: alias,
    );
  }

  if (patternPart.startsWith('"')) {
    final quoteIndex = _findClosingQuote(patternPart);
    if (quoteIndex == -1) {
      throw TokenGrammarError(
        "Unclosed literal pattern for token '$name'",
        lineNumber,
      );
    }
    final body = patternPart.substring(1, quoteIndex);
    if (body.isEmpty) {
      throw TokenGrammarError(
        "Empty literal pattern for token '$name'",
        lineNumber,
      );
    }
    final remainder = patternPart.substring(quoteIndex + 1).trim();
    final alias = _parseAliasRemainder(remainder, name, lineNumber);
    return TokenDefinition(
      name: name,
      pattern: body,
      isRegex: false,
      lineNumber: lineNumber,
      alias: alias,
    );
  }

  throw TokenGrammarError(
    "Pattern for token '$name' must be /regex/ or \"literal\", got: '$patternPart'",
    lineNumber,
  );
}

String? _parseAliasRemainder(String remainder, String name, int lineNumber) {
  if (remainder.isEmpty) {
    return null;
  }
  if (!remainder.startsWith('->')) {
    throw TokenGrammarError(
      "Unexpected text after pattern for token '$name': '$remainder'",
      lineNumber,
    );
  }
  final alias = remainder.substring(2).trim();
  if (alias.isEmpty) {
    throw TokenGrammarError(
      "Missing alias after '->' for token '$name'",
      lineNumber,
    );
  }
  return alias;
}

int _findClosingSlash(String patternPart) {
  var index = 1;
  var inBracket = false;
  while (index < patternPart.length) {
    final char = patternPart[index];
    if (char == r'\') {
      index += 2;
      continue;
    }
    if (char == '[' && !inBracket) {
      inBracket = true;
    } else if (char == ']' && inBracket) {
      inBracket = false;
    } else if (char == '/' && !inBracket) {
      return index;
    }
    index += 1;
  }
  final fallback = patternPart.lastIndexOf('/');
  return fallback > 0 ? fallback : -1;
}

/// Finds the closing quote of a literal pattern: the FIRST `"` after
/// the opener, with no escape processing. Literals are escape-blind by
/// construction (bodies are taken raw — no lane unescapes them), so
/// `"\"` is the one-backslash pattern, exactly like the Rust and Go
/// parsers (`rest.find('"')`, `strings.Index`). An escape-skipping
/// scan would reject that line as unclosed and diverge from both.
int _findClosingQuote(String patternPart) => patternPart.indexOf('"', 1);

List<String> _validateDefinitions(
  List<TokenDefinition> definitions,
  String label,
) {
  final issues = <String>[];
  final seen = <String, int>{};

  for (final definition in definitions) {
    final firstLine = seen[definition.name];
    if (firstLine != null) {
      issues.add(
        "Line ${definition.lineNumber}: Duplicate $label name '${definition.name}' (first defined on line $firstLine)",
      );
    } else {
      seen[definition.name] = definition.lineNumber;
    }

    if (definition.pattern.isEmpty) {
      issues.add(
        "Line ${definition.lineNumber}: Empty pattern for $label '${definition.name}'",
      );
    }

    if (definition.isRegex) {
      try {
        RegExp(definition.pattern);
      } catch (error) {
        issues.add(
          "Line ${definition.lineNumber}: Invalid regex for $label '${definition.name}': $error",
        );
      }
    }

    if (definition.name != definition.name.toUpperCase()) {
      issues.add(
        "Line ${definition.lineNumber}: Token name '${definition.name}' should be UPPER_CASE",
      );
    }

    final alias = definition.alias;
    if (alias != null && alias != alias.toUpperCase()) {
      issues.add(
        "Line ${definition.lineNumber}: Alias '$alias' for token '${definition.name}' should be UPPER_CASE",
      );
    }
  }

  return issues;
}

bool _listEquals<T>(List<T> left, List<T> right) {
  if (identical(left, right)) {
    return true;
  }
  if (left.length != right.length) {
    return false;
  }
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) {
      return false;
    }
  }
  return true;
}
