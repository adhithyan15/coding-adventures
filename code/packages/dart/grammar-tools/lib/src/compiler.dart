import 'dart:convert';

import 'parser_grammar.dart';
import 'token_grammar.dart';

String compileTokenGrammar(TokenGrammar grammar, {String sourceFile = ''}) {
  final sanitizedSource = sourceFile
      .replaceAll('\n', '_')
      .replaceAll('\r', '_');
  final sourceLine = sanitizedSource.isEmpty
      ? ''
      : '// Source: $sanitizedSource\n';

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
  final sanitizedSource = sourceFile
      .replaceAll('\n', '_')
      .replaceAll('\r', '_');
  final sourceLine = sanitizedSource.isEmpty
      ? ''
      : '// Source: $sanitizedSource\n';

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
  return 'GrammarRule(name: ${jsonEncode(rule.name)}, body: ${_compileElement(rule.body)}, lineNumber: ${rule.lineNumber})';
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
  return 'TokenDefinition(name: ${jsonEncode(definition.name)}, pattern: ${jsonEncode(definition.pattern)}, isRegex: ${definition.isRegex}, lineNumber: ${definition.lineNumber}, alias: ${_compileNullableString(definition.alias)})';
}

String _compileStringList(List<String> values) {
  if (values.isEmpty) {
    return 'const []';
  }
  return '[${values.map(jsonEncode).join(', ')}]';
}

String _compileNullableString(String? value) =>
    value == null ? 'null' : jsonEncode(value);

String _compileGroups(Map<String, PatternGroup> groups) {
  if (groups.isEmpty) {
    return 'const {}';
  }
  final entries = groups.entries
      .map(
        (entry) =>
            '    ${jsonEncode(entry.key)}: PatternGroup(name: ${jsonEncode(entry.value.name)}, definitions: ${_compileTokenDefinitionList(entry.value.definitions)}),',
      )
      .join('\n');
  return '{\n$entries\n  }';
}

String _compileElement(GrammarElement element) {
  if (element is RuleReference) {
    return 'RuleReference(${jsonEncode(element.name)}, isToken: ${element.isToken})';
  }
  if (element is Literal) {
    return 'Literal(${jsonEncode(element.value)})';
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
