/// CST-to-[SExpr] lowering: the extractor half of the pipeline.
///
/// Public-in-src but NOT re-exported from the barrel: the reference
/// keeps these private, and so does this lane's public surface. Tests
/// import this file directly to pin the fail-closed `Internal error`
/// arms with hand-built CSTs (unreachable through the fixed grammar
/// unless grammar and extractor drift apart).
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import 'sexpr.dart';

/// An error that occurs during parsing.
///
/// Message-only: positions never survived the reference's
/// engine-error mapping either.
final class LispParseError implements Exception {
  const LispParseError(this.message);

  /// A human-readable description of what went wrong.
  final String message;

  @override
  bool operator ==(Object other) =>
      other is LispParseError && other.message == message;

  @override
  int get hashCode => message.hashCode;

  @override
  String toString() => 'ParseError: $message';
}

/// An extractor internal error: the CST has a shape the fixed
/// grammar cannot produce. Unreachable unless grammar and extractor
/// drift apart.
LispParseError internalError(String message) =>
    LispParseError('Internal error: $message');

/// Lower one `sexpr` CST node to an [SExpr].
SExpr extractSexpr(ASTNode node) {
  if (node.children.length != 1) {
    throw internalError(
        'sexpr node has ${node.children.length} children, want 1');
  }
  final child = node.children.single;
  if (child is! ASTNode) {
    throw internalError('sexpr child is a token, want a rule node');
  }
  switch (child.ruleName) {
    case 'atom':
      return extractAtom(child);
    case 'list':
      return extractList(child);
    case 'quoted':
      return extractQuoted(child);
    default:
      throw internalError('sexpr child is rule "${child.ruleName}"');
  }
}

/// Lower an `atom` CST node (single token child) to an [SExpr].
SExpr extractAtom(ASTNode node) {
  final token = node.token;
  if (token == null) {
    throw internalError('atom node is not a leaf');
  }
  final AtomKind kind;
  switch (token.type) {
    case 'NUMBER':
      kind = AtomKind.number;
    case 'SYMBOL':
      kind = AtomKind.symbol;
    case 'STRING':
      kind = AtomKind.string;
    default:
      throw internalError('atom token is "${token.type}"');
  }
  return SAtom(kind, token.value);
}

/// Lower a `list` CST node to [SList] or [SDottedPair].
///
/// Shape: `LPAREN list_body RPAREN`, where `list_body` holds element
/// sexprs with at most one `DOT sexpr` tail. A leading dot cannot
/// occur: the grammar requires an element before `DOT`, so `(. b)`
/// fails parsing (the hand reader accepted it as an empty-headed
/// pair).
SExpr extractList(ASTNode node) {
  ASTNode? body;
  for (final child in node.children) {
    if (child is ASTNode && child.ruleName == 'list_body') {
      body = child;
      break;
    }
  }
  if (body == null) {
    throw internalError('list node has no list_body child');
  }
  final elements = <SExpr>[];
  SExpr? dotValue;
  final kids = body.children;
  var i = 0;
  while (i < kids.length) {
    final child = kids[i];
    if (child is ASTNode) {
      elements.add(extractSexpr(child));
      i++;
    } else if (child is Token && child.type == 'DOT') {
      i++;
      if (i >= kids.length || kids[i] is! ASTNode) {
        throw internalError('DOT tail is not a rule node');
      }
      if (dotValue != null) {
        throw internalError('list_body has two DOT tails');
      }
      dotValue = extractSexpr(kids[i] as ASTNode);
      i++;
    } else if (child is Token) {
      throw internalError('list_body child is a "${child.type}" token');
    } else {
      // Dart's CST children are open `Object` (the reference's enum
      // is closed): a foreign child fails closed like any other
      // unexpected shape.
      throw internalError('list_body child is a foreign object');
    }
  }
  final tail = dotValue;
  if (tail != null) {
    return SDottedPair(elements, tail);
  }
  return SList(elements);
}

/// Lower a `quoted` CST node (`QUOTE sexpr`) to [SQuoted].
SExpr extractQuoted(ASTNode node) {
  ASTNode? inner;
  for (final child in node.children) {
    if (child is ASTNode) {
      if (inner != null) {
        throw internalError('quoted node has two sexprs');
      }
      inner = child;
    }
  }
  if (inner == null) {
    throw internalError('quoted node has no sexpr child');
  }
  return SQuoted(extractSexpr(inner));
}

/// Lower a `program` CST node to the top-level expression list.
List<SExpr> extractProgram(ASTNode node) {
  final out = <SExpr>[];
  for (final child in node.children) {
    if (child is ASTNode) {
      out.add(extractSexpr(child));
    } else if (child is Token) {
      throw internalError('program child is a "${child.type}" token');
    } else {
      // Open-`Object` children fail closed (see extractList).
      throw internalError('program child is a foreign object');
    }
  }
  return out;
}
