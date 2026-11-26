import 'dart:convert';

import 'parser_grammar.dart';
import 'token_grammar.dart';

String compileTokenGrammar(TokenGrammar grammar, {String sourceFile = ''}) {
  final sanitizedSource =
      sourceFile.replaceAll('\n', '_').replaceAll('\r', '_');
  final sourceLine =
      sanitizedSource.isEmpty ? '' : '// Source: $sanitizedSource\n';

  return [
    '// AUTO-GENERATED FILE - DO NOT EDIT',
    if (sourceLine.isNotEmpty) sourceLine.trimRight(),
    '// Regenerate with: grammar-tools compile-tokens <source.tokens>',
    "import 'package:coding_adventures_grammar_tools/grammar_tools.dart';",
    '',
    'final tokenGrammar = ${_compileTokenGrammarValue(grammar)};',
    '',
  ].join('\n');
}

String compileParserGrammar(ParserGrammar grammar, {String sourceFile = ''}) {
  final sanitizedSource =
      sourceFile.replaceAll('\n', '_').replaceAll('\r', '_');
  final sourceLine =
      sanitizedSource.isEmpty ? '' : '// Source: $sanitizedSource\n';

  return [
    '// AUTO-GENERATED FILE - DO NOT EDIT',
    if (sourceLine.isNotEmpty) sourceLine.trimRight(),
    '// Regenerate with: grammar-tools compile-grammar <source.grammar>',
    "import 'package:coding_adventures_grammar_tools/grammar_tools.dart';",
    '',
    'final parserGrammar = ${_compileParserGrammarValue(grammar)};',
    '',
  ].join('\n');
}

String _compileTokenGrammarValue(TokenGrammar grammar) {
  return '''
TokenGrammar(
  version: ${grammar.version},
  caseInsensitive: ${grammar.caseInsensitive},
  definitions: ${_compileTokenDefinitionList(grammar.definitions)},
  keywords: ${_compileStringList(grammar.keywords)},
  mode: ${_compileNullableString(grammar.mode)},
  skipDefinitions: ${_compileTokenDefinitionList(grammar.skipDefinitions)},
  reservedKeywords: ${_compileStringList(grammar.reservedKeywords)},
  escapeMode: ${_compileNullableString(grammar.escapeMode)},
  errorDefinitions: ${_compileTokenDefinitionList(grammar.errorDefinitions)},
  groups: ${_compileGroups(grammar.groups)},
  caseSensitive: ${grammar.caseSensitive},
  layoutKeywords: ${_compileStringList(grammar.layoutKeywords)},
  contextKeywords: ${_compileStringList(grammar.contextKeywords)},
  softKeywords: ${_compileStringList(grammar.softKeywords)},
  startMode: ${_compileNullableString(grammar.startMode)},
  transitions: ${_compileTransitions(grammar.transitions)},
)''';
}

String _compileParserGrammarValue(ParserGrammar grammar) {
  return '''
ParserGrammar(
  version: ${grammar.version},
  rules: [
${grammar.rules.map((rule) => '    ${_compileRule(rule)},').join('\n')}
  ],
)''';
}

String _compileRule(GrammarRule rule) {
  return 'GrammarRule(name: ${_compileString(rule.name)}, body: ${_compileElement(rule.body)}, lineNumber: ${rule.lineNumber})';
}

String _compileTokenDefinitionList(List<TokenDefinition> definitions) {
  if (definitions.isEmpty) {
    return 'const []';
  }
  final items = definitions
      .map((definition) => '    ${_compileTokenDefinition(definition)},')
      .join('\n');
  return '[\n$items\n  ]';
}

String _compileTokenDefinition(TokenDefinition definition) {
  return 'TokenDefinition(name: ${_compileString(definition.name)}, pattern: ${_compileString(definition.pattern)}, isRegex: ${definition.isRegex}, lineNumber: ${definition.lineNumber}, alias: ${_compileNullableString(definition.alias)})';
}

String _compileStringList(List<String> values) {
  if (values.isEmpty) {
    return 'const []';
  }
  return '[${values.map(jsonEncode).join(', ')}]';
}

String _compileNullableString(String? value) =>
    value == null ? 'null' : _compileString(value);

/// Emits [value] as a Dart string literal. JSON escaping covers every
/// Dart escape except `$` (interpolation), which JSON leaves bare.
String _compileString(String value) =>
    jsonEncode(value).replaceAll(r'$', r'\$');

String _compileGroups(Map<String, PatternGroup> groups) {
  if (groups.isEmpty) {
    return 'const {}';
  }
  final entries = groups.entries
      .map(
        (entry) =>
            '    ${_compileString(entry.key)}: PatternGroup(name: ${_compileString(entry.value.name)}, definitions: ${_compileTokenDefinitionList(entry.value.definitions)}),',
      )
      .join('\n');
  return '{\n$entries\n  }';
}

String _compileTransitions(List<ModeTransition> transitions) {
  if (transitions.isEmpty) {
    return 'const []';
  }
  final items =
      transitions.map((rule) => '    ${_compileTransition(rule)},').join('\n');
  return '[\n$items\n  ]';
}

String _compileTransition(ModeTransition rule) {
  return 'ModeTransition(onTokens: ${_compileStringList(rule.onTokens)}, onValue: ${_compileNullableString(rule.onValue)}, inMode: ${_compileNullableString(rule.inMode)}, conditions: [${rule.conditions.map(_compileCondition).join(', ')}], actions: [${rule.actions.map(_compileAction).join(', ')}], lineNumber: ${rule.lineNumber})';
}

String _compileCondition(TransitionCondition condition) {
  return switch (condition) {
    TransitionCondition.spaceBefore => 'TransitionCondition.spaceBefore',
    TransitionCondition.noSpaceBefore => 'TransitionCondition.noSpaceBefore',
    TransitionCondition.spaceAfter => 'TransitionCondition.spaceAfter',
    TransitionCondition.noSpaceAfter => 'TransitionCondition.noSpaceAfter',
    TransitionCondition.lineStart => 'TransitionCondition.lineStart',
    TransitionCondition.notLineStart => 'TransitionCondition.notLineStart',
  };
}

String _compileAction(TransitionAction action) {
  return switch (action) {
    SetMode(:final mode) => 'SetMode(${_compileString(mode)})',
    PushRegion(:final group) => 'PushRegion(${_compileString(group)})',
    PopRegion() => 'const PopRegion()',
    EnableSkip() => 'const EnableSkip()',
    DisableSkip() => 'const DisableSkip()',
    RefuseToken() => 'const RefuseToken()',
  };
}

String _compileElement(GrammarElement element) {
  if (element is RuleReference) {
    return 'RuleReference(${_compileString(element.name)}, isToken: ${element.isToken})';
  }
  if (element is Literal) {
    return 'Literal(${_compileString(element.value)})';
  }
  if (element is Sequence) {
    return 'Sequence(elements: [${element.elements.map(_compileElement).join(', ')}])';
  }
  if (element is Alternation) {
    return 'Alternation(choices: [${element.choices.map(_compileElement).join(', ')}])';
  }
  if (element is Repetition) {
    return 'Repetition(element: ${_compileElement(element.element)})';
  }
  if (element is Optional) {
    return 'Optional(element: ${_compileElement(element.element)})';
  }
  if (element is Group) {
    return 'Group(element: ${_compileElement(element.element)})';
  }
  if (element is PositiveLookahead) {
    return 'PositiveLookahead(element: ${_compileElement(element.element)})';
  }
  if (element is NegativeLookahead) {
    return 'NegativeLookahead(element: ${_compileElement(element.element)})';
  }
  if (element is OneOrMoreRepetition) {
    return 'OneOrMoreRepetition(element: ${_compileElement(element.element)})';
  }
  if (element is SeparatedRepetition) {
    return 'SeparatedRepetition(element: ${_compileElement(element.element)}, separator: ${_compileElement(element.separator)}, atLeastOne: ${element.atLeastOne})';
  }
  throw ArgumentError('Unsupported grammar element: $element');
}
