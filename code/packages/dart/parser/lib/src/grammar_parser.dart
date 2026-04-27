import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import 'ast.dart';

typedef GrammarPreParseHook = List<Token> Function(List<Token> tokens);
typedef GrammarPostParseHook = ASTNode Function(ASTNode ast);
typedef ASTNodeTransform = ASTNode? Function(ASTNode node, ASTNode? parent);

class GrammarParserOptions {
  const GrammarParserOptions({this.trace = false, this.traceWriter});

  final bool trace;
  final void Function(String message)? traceWriter;
}

class ASTVisitor {
  const ASTVisitor({this.enter, this.leave});

  final ASTNodeTransform? enter;
  final ASTNodeTransform? leave;
}

class GrammarParseError implements Exception {
  GrammarParseError(this.message, [this.token]);

  final String message;
  final Token? token;

  @override
  String toString() {
    if (token == null) {
      return 'Parse error: $message';
    }
    return 'Parse error at ${token!.line}:${token!.column}: $message';
  }
}

class _MemoEntry {
  const _MemoEntry({
    required this.children,
    required this.endPos,
    required this.ok,
  });

  final List<Object>? children;
  final int endPos;
  final bool ok;
}

class GrammarParser {
  GrammarParser(
    List<Token> tokens,
    this.grammar, {
    GrammarParserOptions? options,
  })  : _tokens = List<Token>.from(tokens),
        _trace = options?.trace ?? false,
        _traceWriter = options?.traceWriter {
    for (var index = 0; index < grammar.rules.length; index++) {
      final rule = grammar.rules[index];
      _rules[rule.name] = rule;
      _ruleIndex[rule.name] = index;
    }
    _newlinesSignificant = _grammarReferencesNewline();
  }

  List<Token> _tokens;
  final ParserGrammar grammar;
  final Map<String, GrammarRule> _rules = <String, GrammarRule>{};
  final Map<String, int> _ruleIndex = <String, int>{};
  final Map<String, _MemoEntry> _memo = <String, _MemoEntry>{};
  final bool _trace;
  final void Function(String message)? _traceWriter;
  final List<GrammarPreParseHook> _preParseHooks = <GrammarPreParseHook>[];
  final List<GrammarPostParseHook> _postParseHooks = <GrammarPostParseHook>[];

  late final bool _newlinesSignificant;
  int _position = 0;
  int _furthestPos = 0;
  final List<String> _furthestExpected = <String>[];

  bool get newlinesSignificant => _newlinesSignificant;

  void addPreParse(GrammarPreParseHook hook) {
    _preParseHooks.add(hook);
  }

  void addPostParse(GrammarPostParseHook hook) {
    _postParseHooks.add(hook);
  }

  ASTNode parse() {
    if (_preParseHooks.isNotEmpty) {
      var mutableTokens = List<Token>.from(_tokens);
      for (final hook in _preParseHooks) {
        mutableTokens = hook(mutableTokens);
      }
      _tokens = mutableTokens;
    }

    if (grammar.rules.isEmpty) {
      throw GrammarParseError('Grammar has no rules');
    }

    final entryRule = grammar.rules.first;
    final result = _parseRule(entryRule.name);
    if (result == null) {
      final token = _current();
      if (_furthestExpected.isNotEmpty) {
        final furthestToken = _furthestToken(token);
        throw GrammarParseError(
          'Expected ${_furthestExpected.join(' or ')}, got '
          '${_quoted(furthestToken.value)}',
          furthestToken,
        );
      }
      throw GrammarParseError('Failed to parse', token);
    }

    while (_position < _tokens.length && _current().type == 'NEWLINE') {
      _position += 1;
    }

    if (_position < _tokens.length && _current().type != 'EOF') {
      final token = _current();
      if (_furthestExpected.isNotEmpty && _furthestPos > _position) {
        final furthestToken = _furthestToken(token);
        throw GrammarParseError(
          'Expected ${_furthestExpected.join(' or ')}, got '
          '${_quoted(furthestToken.value)}',
          furthestToken,
        );
      }
      throw GrammarParseError(
        'Unexpected token: ${_quoted(token.value)}',
        token,
      );
    }

    var ast = result;
    for (final hook in _postParseHooks) {
      ast = hook(ast);
    }
    return ast;
  }

  Token _current() {
    if (_position < _tokens.length) {
      return _tokens[_position];
    }
    return _tokens.last;
  }

  void _recordFailure(String expected) {
    if (_position > _furthestPos) {
      _furthestPos = _position;
      _furthestExpected
        ..clear()
        ..add(expected);
      return;
    }
    if (_position == _furthestPos && !_furthestExpected.contains(expected)) {
      _furthestExpected.add(expected);
    }
  }

  Token _furthestToken(Token fallback) {
    if (_furthestPos < _tokens.length) {
      return _tokens[_furthestPos];
    }
    return fallback;
  }

  bool _grammarReferencesNewline() {
    for (final rule in grammar.rules) {
      if (_elementReferencesNewline(rule.body)) {
        return true;
      }
    }
    return false;
  }

  bool _elementReferencesNewline(GrammarElement element) {
    if (element is RuleReference) {
      return element.isToken && element.name == 'NEWLINE';
    }
    if (element is Sequence) {
      return element.elements.any(_elementReferencesNewline);
    }
    if (element is Alternation) {
      return element.choices.any(_elementReferencesNewline);
    }
    if (element is Repetition) {
      return _elementReferencesNewline(element.element);
    }
    if (element is Optional) {
      return _elementReferencesNewline(element.element);
    }
    if (element is Group) {
      return _elementReferencesNewline(element.element);
    }
    if (element is PositiveLookahead) {
      return _elementReferencesNewline(element.element);
    }
    if (element is NegativeLookahead) {
      return _elementReferencesNewline(element.element);
    }
    if (element is OneOrMoreRepetition) {
      return _elementReferencesNewline(element.element);
    }
    if (element is SeparatedRepetition) {
      return _elementReferencesNewline(element.element) ||
          _elementReferencesNewline(element.separator);
    }
    return false;
  }

  ASTNode? _parseRule(String ruleName) {
    final rule = _rules[ruleName];
    if (rule == null) {
      return null;
    }

    final idx = _ruleIndex[ruleName];
    if (idx != null) {
      final key = '$idx,$_position';
      final cached = _memo[key];
      if (cached != null) {
        _position = cached.endPos;
        if (!cached.ok) {
          return null;
        }
        return _createNode(ruleName, cached.children!);
      }
    }

    final startPos = _position;
    if (idx != null) {
      final key = '$idx,$startPos';
      _memo[key] = _MemoEntry(children: null, endPos: startPos, ok: false);
    }

    if (_trace) {
      final token = _current();
      _emitTrace(
        "[TRACE] rule '$ruleName' at token $startPos "
        "(${token.type} ${_quoted(token.value)}) -> ",
        newline: false,
      );
    }

    var children = _matchElement(rule.body);

    if (_trace) {
      _emitTrace(children != null ? 'match' : 'fail');
    }

    if (idx != null) {
      final key = '$idx,$startPos';
      if (children != null) {
        _memo[key] = _MemoEntry(
          children: List<Object>.unmodifiable(children),
          endPos: _position,
          ok: true,
        );
      } else {
        _memo[key] = _MemoEntry(children: null, endPos: _position, ok: false);
      }

      if (children != null) {
        while (true) {
          final previousEnd = _position;
          _position = startPos;
          _memo[key] = _MemoEntry(
            children: List<Object>.unmodifiable(children!),
            endPos: previousEnd,
            ok: true,
          );
          final newChildren = _matchElement(rule.body);
          if (newChildren == null || _position <= previousEnd) {
            _position = previousEnd;
            _memo[key] = _MemoEntry(
              children: List<Object>.unmodifiable(children),
              endPos: previousEnd,
              ok: true,
            );
            break;
          }
          children = newChildren;
        }
      }
    }

    if (children == null) {
      _position = startPos;
      _recordFailure(ruleName);
      return null;
    }

    return _createNode(ruleName, children);
  }

  ASTNode _createNode(String ruleName, List<Object> children) {
    final position = _computeNodePosition(children);
    return ASTNode(
      ruleName: ruleName,
      children: children,
      startLine: position?.startLine,
      startColumn: position?.startColumn,
      endLine: position?.endLine,
      endColumn: position?.endColumn,
    );
  }

  List<Object>? _matchElement(GrammarElement element) {
    final savedPosition = _position;

    if (element is Sequence) {
      final children = <Object>[];
      for (final subElement in element.elements) {
        final result = _matchElement(subElement);
        if (result == null) {
          _position = savedPosition;
          return null;
        }
        children.addAll(result);
      }
      return children;
    }

    if (element is Alternation) {
      for (final choice in element.choices) {
        _position = savedPosition;
        final result = _matchElement(choice);
        if (result != null) {
          return result;
        }
      }
      _position = savedPosition;
      return null;
    }

    if (element is Repetition) {
      final children = <Object>[];
      while (true) {
        final repetitionPosition = _position;
        final result = _matchElement(element.element);
        if (result == null) {
          _position = repetitionPosition;
          break;
        }
        children.addAll(result);
      }
      return children;
    }

    if (element is Optional) {
      final result = _matchElement(element.element);
      return result ?? <Object>[];
    }

    if (element is Group) {
      return _matchElement(element.element);
    }

    if (element is RuleReference) {
      if (element.isToken) {
        return _matchTokenReference(element.name);
      }
      final node = _parseRule(element.name);
      if (node == null) {
        _position = savedPosition;
        return null;
      }
      return <Object>[node];
    }

    if (element is Literal) {
      var token = _current();
      if (!_newlinesSignificant) {
        while (token.type == 'NEWLINE') {
          _position += 1;
          token = _current();
        }
      }
      if (token.value == element.value) {
        _position += 1;
        return <Object>[token];
      }
      _recordFailure(_quoted(element.value));
      return null;
    }

    if (element is PositiveLookahead) {
      final result = _matchElement(element.element);
      _position = savedPosition;
      return result != null ? <Object>[] : null;
    }

    if (element is NegativeLookahead) {
      final result = _matchElement(element.element);
      _position = savedPosition;
      return result == null ? <Object>[] : null;
    }

    if (element is OneOrMoreRepetition) {
      final first = _matchElement(element.element);
      if (first == null) {
        _position = savedPosition;
        return null;
      }
      final children = <Object>[...first];
      while (true) {
        final repetitionPosition = _position;
        final result = _matchElement(element.element);
        if (result == null) {
          _position = repetitionPosition;
          break;
        }
        children.addAll(result);
      }
      return children;
    }

    if (element is SeparatedRepetition) {
      final first = _matchElement(element.element);
      if (first == null) {
        _position = savedPosition;
        return element.atLeastOne ? null : <Object>[];
      }
      final children = <Object>[...first];
      while (true) {
        final separatorPosition = _position;
        final separator = _matchElement(element.separator);
        if (separator == null) {
          _position = separatorPosition;
          break;
        }
        final next = _matchElement(element.element);
        if (next == null) {
          _position = separatorPosition;
          break;
        }
        children
          ..addAll(separator)
          ..addAll(next);
      }
      return children;
    }

    return null;
  }

  List<Object>? _matchTokenReference(String expectedType) {
    var token = _current();
    if (!_newlinesSignificant && expectedType != 'NEWLINE') {
      while (token.type == 'NEWLINE') {
        _position += 1;
        token = _current();
      }
    }

    if (token.type == expectedType) {
      _position += 1;
      return <Object>[token];
    }

    _recordFailure(expectedType);
    return null;
  }

  void _emitTrace(String message, {bool newline = true}) {
    if (!_trace) {
      return;
    }
    final text = newline ? message : message;
    if (_traceWriter != null) {
      _traceWriter!(newline ? '$text\n' : text);
    }
  }
}

class _NodePosition {
  const _NodePosition({
    required this.startLine,
    required this.startColumn,
    required this.endLine,
    required this.endColumn,
  });

  final int startLine;
  final int startColumn;
  final int endLine;
  final int endColumn;
}

_NodePosition? _computeNodePosition(List<Object> children) {
  final first = _findFirstToken(children);
  final last = _findLastToken(children);
  if (first == null || last == null) {
    return null;
  }
  return _NodePosition(
    startLine: first.line,
    startColumn: first.column,
    endLine: last.line,
    endColumn: last.column,
  );
}

Token? _findFirstToken(List<Object> children) {
  for (final child in children) {
    if (child is ASTNode) {
      final token = _findFirstToken(child.children);
      if (token != null) {
        return token;
      }
      continue;
    }
    if (child is Token) {
      return child;
    }
  }
  return null;
}

Token? _findLastToken(List<Object> children) {
  for (var index = children.length - 1; index >= 0; index -= 1) {
    final child = children[index];
    if (child is ASTNode) {
      final token = _findLastToken(child.children);
      if (token != null) {
        return token;
      }
      continue;
    }
    if (child is Token) {
      return child;
    }
  }
  return null;
}

ASTNode walkAST(ASTNode node, ASTVisitor visitor) {
  return _walkNode(node, null, visitor);
}

ASTNode _walkNode(ASTNode node, ASTNode? parent, ASTVisitor visitor) {
  var current = visitor.enter?.call(node, parent) ?? node;

  var childrenChanged = false;
  final newChildren = <Object>[];
  for (final child in current.children) {
    if (child is ASTNode) {
      final walked = _walkNode(child, current, visitor);
      if (walked != child) {
        childrenChanged = true;
      }
      newChildren.add(walked);
      continue;
    }
    newChildren.add(child);
  }

  if (childrenChanged) {
    current = current.copyWith(children: newChildren);
  }

  return visitor.leave?.call(current, parent) ?? current;
}

List<ASTNode> findNodes(ASTNode node, String ruleName) {
  final results = <ASTNode>[];
  walkAST(
    node,
    ASTVisitor(
      enter: (current, _) {
        if (current.ruleName == ruleName) {
          results.add(current);
        }
        return null;
      },
    ),
  );
  return results;
}

List<Token> collectTokens(ASTNode node, {String? type}) {
  final results = <Token>[];

  void walk(ASTNode current) {
    for (final child in current.children) {
      if (child is ASTNode) {
        walk(child);
      } else if (child is Token) {
        if (type == null || child.type == type) {
          results.add(child);
        }
      }
    }
  }

  walk(node);
  return results;
}

String _quoted(String value) => '"$value"';
