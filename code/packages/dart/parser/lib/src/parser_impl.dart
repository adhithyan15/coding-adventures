import 'package:coding_adventures_lexer/lexer.dart';

import 'ast.dart';

class Parser {
  Parser(this.tokens);

  final List<Token> tokens;
  int _position = 0;

  Token _peek() => _position < tokens.length ? tokens[_position] : tokens.last;

  Token _advance() {
    final token = _peek();
    _position += 1;
    return token;
  }

  Token _expect(String type) {
    final token = _peek();
    if (token.type != type) {
      throw ParseError(
        'Expected $type, got ${token.type} (${_quoted(token.value)})',
        token,
      );
    }
    return _advance();
  }

  Token? _match(List<String> types) {
    final token = _peek();
    if (types.contains(token.type)) {
      return _advance();
    }
    return null;
  }

  bool _atEnd() => _peek().type == 'EOF';

  void _skipNewlines() {
    while (_peek().type == 'NEWLINE') {
      _advance();
    }
  }

  Program parse() => _parseProgram();

  Program _parseProgram() {
    final statements = <Statement>[];
    _skipNewlines();
    while (!_atEnd()) {
      statements.add(_parseStatement());
      _skipNewlines();
    }
    return Program(List<Statement>.unmodifiable(statements));
  }

  Statement _parseStatement() {
    if (_peek().type == 'NAME' &&
        _position + 1 < tokens.length &&
        tokens[_position + 1].type == 'EQUALS') {
      return _parseAssignment();
    }
    return _parseExpressionStatement();
  }

  Assignment _parseAssignment() {
    final nameToken = _expect('NAME');
    _expect('EQUALS');
    final value = _parseExpression();
    if (!_atEnd()) {
      _expect('NEWLINE');
    }
    return Assignment(target: Name(nameToken.value), value: value);
  }

  Statement _parseExpressionStatement() {
    final expression = _parseExpression();
    if (!_atEnd()) {
      _expect('NEWLINE');
    }
    return expression as Statement;
  }

  Expression _parseExpression() {
    var left = _parseTerm();
    while (true) {
      final operator = _match(const <String>['PLUS', 'MINUS']);
      if (operator == null) {
        break;
      }
      final right = _parseTerm();
      left = BinaryOp(left: left, op: operator.value, right: right);
    }
    return left;
  }

  Expression _parseTerm() {
    var left = _parseFactor();
    while (true) {
      final operator = _match(const <String>['STAR', 'SLASH']);
      if (operator == null) {
        break;
      }
      final right = _parseFactor();
      left = BinaryOp(left: left, op: operator.value, right: right);
    }
    return left;
  }

  Expression _parseFactor() {
    final token = _peek();
    if (token.type == 'NUMBER') {
      _advance();
      return NumberLiteral(int.parse(token.value));
    }
    if (token.type == 'STRING') {
      _advance();
      return StringLiteral(token.value);
    }
    if (token.type == 'NAME') {
      _advance();
      return Name(token.value);
    }
    if (token.type == 'LPAREN') {
      _advance();
      final expression = _parseExpression();
      _expect('RPAREN');
      return expression;
    }
    throw ParseError(
      'Unexpected token ${token.type} (${_quoted(token.value)})',
      token,
    );
  }
}

String _quoted(String value) => '"$value"';
