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
  );
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
    final remainder = patternPart.substring(slashIndex + 1).trim();
    final alias = _parseAliasRemainder(remainder, name, lineNumber);
    return TokenDefinition(
      name: name,
      pattern: body,
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

int _findClosingQuote(String patternPart) {
  var index = 1;
  while (index < patternPart.length) {
    final char = patternPart[index];
    if (char == r'\') {
      index += 2;
      continue;
    }
    if (char == '"') {
      return index;
    }
    index += 1;
  }
  return -1;
}

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
