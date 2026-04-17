import 'package:coding_adventures_lexer/lexer.dart';

abstract class AstNode {
  const AstNode();
}

class ASTNode extends AstNode {
  ASTNode({
    required this.ruleName,
    required List<Object> children,
    this.startLine,
    this.startColumn,
    this.endLine,
    this.endColumn,
  }) : children = List<Object>.unmodifiable(children);

  final String ruleName;
  final List<Object> children;
  final int? startLine;
  final int? startColumn;
  final int? endLine;
  final int? endColumn;

  bool get isLeaf => children.length == 1 && children.first is Token;

  Token? get token => isLeaf ? children.first as Token : null;

  ASTNode copyWith({
    String? ruleName,
    List<Object>? children,
    int? startLine,
    int? startColumn,
    int? endLine,
    int? endColumn,
  }) {
    return ASTNode(
      ruleName: ruleName ?? this.ruleName,
      children: children ?? this.children,
      startLine: startLine ?? this.startLine,
      startColumn: startColumn ?? this.startColumn,
      endLine: endLine ?? this.endLine,
      endColumn: endColumn ?? this.endColumn,
    );
  }

  @override
  bool operator ==(Object other) {
    return other is ASTNode &&
        other.ruleName == ruleName &&
        _listEquals(other.children, children) &&
        other.startLine == startLine &&
        other.startColumn == startColumn &&
        other.endLine == endLine &&
        other.endColumn == endColumn;
  }

  @override
  int get hashCode {
    return Object.hash(
      ruleName,
      Object.hashAll(children),
      startLine,
      startColumn,
      endLine,
      endColumn,
    );
  }
}

abstract class Expression extends AstNode {
  const Expression();
}

abstract class Statement extends AstNode {
  const Statement();
}

class NumberLiteral extends Expression implements Statement {
  const NumberLiteral(this.value);

  final int value;

  @override
  bool operator ==(Object other) =>
      other is NumberLiteral && other.value == value;

  @override
  int get hashCode => value.hashCode;
}

class StringLiteral extends Expression implements Statement {
  const StringLiteral(this.value);

  final String value;

  @override
  bool operator ==(Object other) =>
      other is StringLiteral && other.value == value;

  @override
  int get hashCode => value.hashCode;
}

class Name extends Expression implements Statement {
  const Name(this.name);

  final String name;

  @override
  bool operator ==(Object other) => other is Name && other.name == name;

  @override
  int get hashCode => name.hashCode;
}

class BinaryOp extends Expression implements Statement {
  const BinaryOp({required this.left, required this.op, required this.right});

  final Expression left;
  final String op;
  final Expression right;

  @override
  bool operator ==(Object other) {
    return other is BinaryOp &&
        other.left == left &&
        other.op == op &&
        other.right == right;
  }

  @override
  int get hashCode => Object.hash(left, op, right);
}

class Assignment extends Statement {
  const Assignment({required this.target, required this.value});

  final Name target;
  final Expression value;

  @override
  bool operator ==(Object other) {
    return other is Assignment &&
        other.target == target &&
        other.value == value;
  }

  @override
  int get hashCode => Object.hash(target, value);
}

class Program extends AstNode {
  const Program(this.statements);

  final List<Statement> statements;

  @override
  bool operator ==(Object other) {
    return other is Program && _listEquals(other.statements, statements);
  }

  @override
  int get hashCode =>
      statements.fold<int>(0, (value, item) => value ^ item.hashCode);
}

class ParseError implements Exception {
  ParseError(this.message, this.token);

  final String message;
  final Token token;

  @override
  String toString() => '$message at line ${token.line}, column ${token.column}';
}

bool isASTNode(Object child) => child is ASTNode;

bool isLeafNode(ASTNode node) => node.isLeaf;

Token? getLeafToken(ASTNode node) => node.token;

bool _listEquals(List<Object?> left, List<Object?> right) {
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
