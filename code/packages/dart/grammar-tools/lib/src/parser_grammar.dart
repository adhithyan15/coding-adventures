final RegExp _parserMagicCommentPattern = RegExp(r'^#\s*@(\w+)\s*(.*)$');

class ParserGrammarError implements Exception {
  ParserGrammarError(this.message, this.lineNumber);

  final String message;
  final int lineNumber;

  @override
  String toString() => 'Line $lineNumber: $message';
}

abstract class GrammarElement {
  const GrammarElement();
}

class RuleReference extends GrammarElement {
  const RuleReference(this.name, {required this.isToken});

  final String name;
  final bool isToken;

  @override
  bool operator ==(Object other) {
    return other is RuleReference &&
        other.name == name &&
        other.isToken == isToken;
  }

  @override
  int get hashCode => Object.hash(name, isToken);
}

class Literal extends GrammarElement {
  const Literal(this.value);

  final String value;

  @override
  bool operator ==(Object other) => other is Literal && other.value == value;

  @override
  int get hashCode => value.hashCode;
}

class Sequence extends GrammarElement {
  const Sequence({required this.elements});

  final List<GrammarElement> elements;

  @override
  bool operator ==(Object other) {
    return other is Sequence && _grammarListEquals(other.elements, elements);
  }

  @override
  int get hashCode => Object.hashAll(elements);
}

class Alternation extends GrammarElement {
  const Alternation({required this.choices});

  final List<GrammarElement> choices;

  @override
  bool operator ==(Object other) {
    return other is Alternation && _grammarListEquals(other.choices, choices);
  }

  @override
  int get hashCode => Object.hashAll(choices);
}

class Repetition extends GrammarElement {
  const Repetition({required this.element});

  final GrammarElement element;

  @override
  bool operator ==(Object other) =>
      other is Repetition && other.element == element;

  @override
  int get hashCode => element.hashCode;
}

class Optional extends GrammarElement {
  const Optional({required this.element});

  final GrammarElement element;

  @override
  bool operator ==(Object other) =>
      other is Optional && other.element == element;

  @override
  int get hashCode => element.hashCode;
}

class Group extends GrammarElement {
  const Group({required this.element});

  final GrammarElement element;

  @override
  bool operator ==(Object other) => other is Group && other.element == element;

  @override
  int get hashCode => element.hashCode;
}

class PositiveLookahead extends GrammarElement {
  const PositiveLookahead({required this.element});

  final GrammarElement element;

  @override
  bool operator ==(Object other) {
    return other is PositiveLookahead && other.element == element;
  }

  @override
  int get hashCode => element.hashCode;
}

class NegativeLookahead extends GrammarElement {
  const NegativeLookahead({required this.element});

  final GrammarElement element;

  @override
  bool operator ==(Object other) {
    return other is NegativeLookahead && other.element == element;
  }

  @override
  int get hashCode => element.hashCode;
}

class OneOrMoreRepetition extends GrammarElement {
  const OneOrMoreRepetition({required this.element});

  final GrammarElement element;

  @override
  bool operator ==(Object other) {
    return other is OneOrMoreRepetition && other.element == element;
  }

  @override
  int get hashCode => element.hashCode;
}

class SeparatedRepetition extends GrammarElement {
  const SeparatedRepetition({
    required this.element,
    required this.separator,
    required this.atLeastOne,
  });

  final GrammarElement element;
  final GrammarElement separator;
  final bool atLeastOne;

  @override
  bool operator ==(Object other) {
    return other is SeparatedRepetition &&
        other.element == element &&
        other.separator == separator &&
        other.atLeastOne == atLeastOne;
  }

  @override
  int get hashCode => Object.hash(element, separator, atLeastOne);
}

class GrammarRule {
  const GrammarRule({
    required this.name,
    required this.body,
    required this.lineNumber,
  });

  final String name;
  final GrammarElement body;
  final int lineNumber;

  @override
  bool operator ==(Object other) {
    return other is GrammarRule &&
        other.name == name &&
        other.body == body &&
        other.lineNumber == lineNumber;
  }

  @override
  int get hashCode => Object.hash(name, body, lineNumber);
}

class ParserGrammar {
  const ParserGrammar({this.version = 0, this.rules = const []});

  final int version;
  final List<GrammarRule> rules;

  Set<String> ruleNames() => rules.map((rule) => rule.name).toSet();

  Set<String> tokenReferences() {
    final refs = <String>{};
    for (final rule in rules) {
      _collectTokenRefs(rule.body, refs);
    }
    return refs;
  }

  Set<String> ruleReferences() {
    final refs = <String>{};
    for (final rule in rules) {
      _collectRuleRefs(rule.body, refs);
    }
    return refs;
  }
}

class _GrammarToken {
  const _GrammarToken(this.kind, this.value, this.line);

  final String kind;
  final String value;
  final int line;
}

ParserGrammar parseParserGrammar(String source) {
  var version = 0;
  for (final rawLine in source.split('\n')) {
    final stripped = rawLine.trim();
    if (!stripped.startsWith('#')) {
      continue;
    }
    final match = _parserMagicCommentPattern.firstMatch(stripped);
    if (match != null && match.group(1) == 'version') {
      final parsed = int.tryParse((match.group(2) ?? '').trim());
      if (parsed != null) {
        version = parsed;
      }
    }
  }

  final parser = _Parser(_tokenizeGrammar(source));
  return ParserGrammar(version: version, rules: parser.parse());
}

List<String> validateParserGrammar(
  ParserGrammar grammar, {
  Set<String>? tokenNames,
}) {
  final issues = <String>[];
  final defined = grammar.ruleNames();
  final referencedRules = grammar.ruleReferences();
  final referencedTokens = grammar.tokenReferences();

  final seen = <String, int>{};
  for (final rule in grammar.rules) {
    final firstLine = seen[rule.name];
    if (firstLine != null) {
      issues.add(
        "Line ${rule.lineNumber}: Duplicate rule name '${rule.name}' (first defined on line $firstLine)",
      );
    } else {
      seen[rule.name] = rule.lineNumber;
    }
  }

  for (final rule in grammar.rules) {
    if (rule.name != rule.name.toLowerCase()) {
      issues.add(
        "Line ${rule.lineNumber}: Rule name '${rule.name}' should be lowercase",
      );
    }
  }

  for (final reference in referencedRules.toList()..sort()) {
    if (!defined.contains(reference)) {
      issues.add("Undefined rule reference: '$reference'");
    }
  }

  if (tokenNames != null) {
    const syntheticTokens = {'NEWLINE', 'INDENT', 'DEDENT', 'EOF'};
    for (final reference in referencedTokens.toList()..sort()) {
      if (!tokenNames.contains(reference) &&
          !syntheticTokens.contains(reference)) {
        issues.add("Undefined token reference: '$reference'");
      }
    }
  }

  if (grammar.rules.isNotEmpty) {
    final startRule = grammar.rules.first.name;
    for (final rule in grammar.rules) {
      if (rule.name != startRule && !referencedRules.contains(rule.name)) {
        issues.add(
          "Line ${rule.lineNumber}: Rule '${rule.name}' is defined but never referenced (unreachable)",
        );
      }
    }
  }

  return issues;
}

List<_GrammarToken> _tokenizeGrammar(String source) {
  final tokens = <_GrammarToken>[];
  final lines = source.split('\n');

  for (var index = 0; index < lines.length; index++) {
    final lineNumber = index + 1;
    final line = lines[index].replaceFirst(RegExp(r'\r$'), '');
    final stripped = line.trim();

    if (stripped.isEmpty || stripped.startsWith('#')) {
      continue;
    }

    var cursor = 0;
    while (cursor < line.length) {
      final char = line[cursor];
      if (char == ' ' || char == '\t') {
        cursor += 1;
        continue;
      }
      if (char == '#') {
        break;
      }
      switch (char) {
        case '=':
          tokens.add(_GrammarToken('EQUALS', '=', lineNumber));
          cursor += 1;
          continue;
        case ';':
          tokens.add(_GrammarToken('SEMI', ';', lineNumber));
          cursor += 1;
          continue;
        case '|':
          tokens.add(_GrammarToken('PIPE', '|', lineNumber));
          cursor += 1;
          continue;
        case '{':
          tokens.add(_GrammarToken('LBRACE', '{', lineNumber));
          cursor += 1;
          continue;
        case '}':
          tokens.add(_GrammarToken('RBRACE', '}', lineNumber));
          cursor += 1;
          continue;
        case '[':
          tokens.add(_GrammarToken('LBRACKET', '[', lineNumber));
          cursor += 1;
          continue;
        case ']':
          tokens.add(_GrammarToken('RBRACKET', ']', lineNumber));
          cursor += 1;
          continue;
        case '(':
          tokens.add(_GrammarToken('LPAREN', '(', lineNumber));
          cursor += 1;
          continue;
        case ')':
          tokens.add(_GrammarToken('RPAREN', ')', lineNumber));
          cursor += 1;
          continue;
        case '&':
          tokens.add(_GrammarToken('AMPERSAND', '&', lineNumber));
          cursor += 1;
          continue;
        case '!':
          tokens.add(_GrammarToken('BANG', '!', lineNumber));
          cursor += 1;
          continue;
        case '+':
          tokens.add(_GrammarToken('PLUS', '+', lineNumber));
          cursor += 1;
          continue;
        case '/':
          if (cursor + 1 < line.length && line[cursor + 1] == '/') {
            tokens.add(_GrammarToken('DOUBLE_SLASH', '//', lineNumber));
            cursor += 2;
            continue;
          }
          throw ParserGrammarError("Unexpected character: '$char'", lineNumber);
        case '"':
          final end = _findGrammarStringEnd(line, cursor);
          if (end == -1) {
            throw ParserGrammarError('Unterminated string literal', lineNumber);
          }
          tokens.add(
            _GrammarToken(
              'STRING',
              line.substring(cursor + 1, end),
              lineNumber,
            ),
          );
          cursor = end + 1;
          continue;
        default:
          if (_isIdentifierStart(char)) {
            var end = cursor + 1;
            while (end < line.length && _isIdentifierPart(line[end])) {
              end += 1;
            }
            tokens.add(
              _GrammarToken('IDENT', line.substring(cursor, end), lineNumber),
            );
            cursor = end;
            continue;
          }
          throw ParserGrammarError("Unexpected character: '$char'", lineNumber);
      }
    }
  }

  tokens.add(_GrammarToken('EOF', '', lines.length));
  return tokens;
}

int _findGrammarStringEnd(String line, int start) {
  var index = start + 1;
  while (index < line.length) {
    final char = line[index];
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

bool _isIdentifierStart(String char) => RegExp(r'[A-Za-z_]').hasMatch(char);
bool _isIdentifierPart(String char) => RegExp(r'[A-Za-z0-9_]').hasMatch(char);

class _Parser {
  _Parser(this._tokens);

  final List<_GrammarToken> _tokens;
  int _position = 0;

  List<GrammarRule> parse() {
    final rules = <GrammarRule>[];
    while (_peek().kind != 'EOF') {
      rules.add(_parseRule());
    }
    return rules;
  }

  GrammarRule _parseRule() {
    final name = _expect('IDENT');
    _expect('EQUALS');
    final body = _parseBody();
    _expect('SEMI');
    return GrammarRule(name: name.value, body: body, lineNumber: name.line);
  }

  GrammarElement _parseBody() {
    final first = _parseSequence();
    final choices = <GrammarElement>[first];
    while (_peek().kind == 'PIPE') {
      _advance();
      choices.add(_parseSequence());
    }
    if (choices.length == 1) {
      return choices.first;
    }
    return Alternation(choices: choices);
  }

  GrammarElement _parseSequence() {
    final elements = <GrammarElement>[];
    while (!const {
      'PIPE',
      'SEMI',
      'RBRACE',
      'RBRACKET',
      'RPAREN',
      'DOUBLE_SLASH',
      'EOF',
    }.contains(_peek().kind)) {
      elements.add(_parseElement());
    }
    if (elements.isEmpty) {
      throw ParserGrammarError(
        'Expected at least one element in sequence',
        _peek().line,
      );
    }
    if (elements.length == 1) {
      return elements.first;
    }
    return Sequence(elements: elements);
  }

  GrammarElement _parseElement() {
    final token = _peek();
    switch (token.kind) {
      case 'AMPERSAND':
        _advance();
        return PositiveLookahead(element: _parseElement());
      case 'BANG':
        _advance();
        return NegativeLookahead(element: _parseElement());
      case 'IDENT':
        _advance();
        final value = token.value;
        final isToken =
            value == value.toUpperCase() && _isIdentifierStart(value[0]);
        return RuleReference(value, isToken: isToken);
      case 'STRING':
        _advance();
        return Literal(token.value);
      case 'LBRACE':
        _advance();
        final body = _parseBody();
        if (_peek().kind == 'DOUBLE_SLASH') {
          _advance();
          final separator = _parseBody();
          _expect('RBRACE');
          final atLeastOne = _peek().kind == 'PLUS';
          if (atLeastOne) {
            _advance();
          }
          return SeparatedRepetition(
            element: body,
            separator: separator,
            atLeastOne: atLeastOne,
          );
        }
        _expect('RBRACE');
        if (_peek().kind == 'PLUS') {
          _advance();
          return OneOrMoreRepetition(element: body);
        }
        return Repetition(element: body);
      case 'LBRACKET':
        _advance();
        final body = _parseBody();
        _expect('RBRACKET');
        return Optional(element: body);
      case 'LPAREN':
        _advance();
        final body = _parseBody();
        _expect('RPAREN');
        return Group(element: body);
      default:
        throw ParserGrammarError(
          "Unexpected token: ${token.kind} ('${token.value}')",
          token.line,
        );
    }
  }

  _GrammarToken _peek() => _tokens[_position];

  _GrammarToken _advance() => _tokens[_position++];

  _GrammarToken _expect(String kind) {
    final token = _advance();
    if (token.kind != kind) {
      throw ParserGrammarError(
        'Expected $kind, got ${token.kind} (\'${token.value}\')',
        token.line,
      );
    }
    return token;
  }
}

void _collectTokenRefs(GrammarElement node, Set<String> refs) {
  if (node is RuleReference) {
    if (node.isToken) {
      refs.add(node.name);
    }
    return;
  }
  if (node is Literal) {
    return;
  }
  if (node is Sequence) {
    for (final element in node.elements) {
      _collectTokenRefs(element, refs);
    }
    return;
  }
  if (node is Alternation) {
    for (final choice in node.choices) {
      _collectTokenRefs(choice, refs);
    }
    return;
  }
  if (node is Repetition) {
    _collectTokenRefs(node.element, refs);
    return;
  }
  if (node is Optional) {
    _collectTokenRefs(node.element, refs);
    return;
  }
  if (node is Group) {
    _collectTokenRefs(node.element, refs);
    return;
  }
  if (node is PositiveLookahead) {
    _collectTokenRefs(node.element, refs);
    return;
  }
  if (node is NegativeLookahead) {
    _collectTokenRefs(node.element, refs);
    return;
  }
  if (node is OneOrMoreRepetition) {
    _collectTokenRefs(node.element, refs);
    return;
  }
  if (node is SeparatedRepetition) {
    _collectTokenRefs(node.element, refs);
    _collectTokenRefs(node.separator, refs);
  }
}

void _collectRuleRefs(GrammarElement node, Set<String> refs) {
  if (node is RuleReference) {
    if (!node.isToken) {
      refs.add(node.name);
    }
    return;
  }
  if (node is Literal) {
    return;
  }
  if (node is Sequence) {
    for (final element in node.elements) {
      _collectRuleRefs(element, refs);
    }
    return;
  }
  if (node is Alternation) {
    for (final choice in node.choices) {
      _collectRuleRefs(choice, refs);
    }
    return;
  }
  if (node is Repetition) {
    _collectRuleRefs(node.element, refs);
    return;
  }
  if (node is Optional) {
    _collectRuleRefs(node.element, refs);
    return;
  }
  if (node is Group) {
    _collectRuleRefs(node.element, refs);
    return;
  }
  if (node is PositiveLookahead) {
    _collectRuleRefs(node.element, refs);
    return;
  }
  if (node is NegativeLookahead) {
    _collectRuleRefs(node.element, refs);
    return;
  }
  if (node is OneOrMoreRepetition) {
    _collectRuleRefs(node.element, refs);
    return;
  }
  if (node is SeparatedRepetition) {
    _collectRuleRefs(node.element, refs);
    _collectRuleRefs(node.separator, refs);
  }
}

bool _grammarListEquals(List<GrammarElement> left, List<GrammarElement> right) {
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
