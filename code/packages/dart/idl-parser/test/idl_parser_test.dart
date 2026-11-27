/// Port of the reference `rust/idl-parser` unit suite (all 59
/// tests, with all six recursion-shape boundaries re-measured on
/// this engine: parens 13/14, nested `IF` 26/27, call args 12/13,
/// subscripts 11/12, unary chain 134/135, array literals 12/13 —
/// identical to the reference) plus nullable-twin and error-shape
/// tests.
library;

import 'package:coding_adventures_idl_parser/idl_parser.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

bool containsRule(ASTNode node, String name) {
  if (node.ruleName == name) return true;
  return node.children.any((c) => c is ASTNode && containsRule(c, name));
}

/// Find the first descendant node (or self) with the given rule name.
ASTNode? findRule(ASTNode node, String name) {
  if (node.ruleName == name) return node;
  for (final c in node.children) {
    if (c is ASTNode) {
      final found = findRule(c, name);
      if (found != null) return found;
    }
  }
  return null;
}

/// Collect every descendant node (or self) with the given rule name.
List<ASTNode> findRules(ASTNode node, String name) {
  final out = <ASTNode>[];
  if (node.ruleName == name) out.add(node);
  for (final c in node.children) {
    if (c is ASTNode) out.addAll(findRules(c, name));
  }
  return out;
}

/// Collect every token value under a node (in order), for shape
/// assertions.
List<String> tokenValues(ASTNode node) {
  final out = <String>[];
  void walk(ASTNode n) {
    for (final c in n.children) {
      if (c is Token) {
        out.add(c.value);
      } else if (c is ASTNode) {
        walk(c);
      }
    }
  }

  walk(node);
  return out;
}

bool parses(String src) => tryParseIdl(src) != null;

String nestedParenSource(int n) => 'y = ${'(' * n}5${')' * n}\n';

String nestedIfSource(int n) {
  final buf = StringBuffer();
  for (var i = 0; i < n; i++) {
    buf.write('IF 1 THEN BEGIN ');
  }
  buf.write('y = 5');
  for (var i = 0; i < n; i++) {
    buf.write(' ENDIF');
  }
  buf.write('\n');
  return buf.toString();
}

String nestedCallSource(int n) => 'y = ${'f(' * n}5${')' * n}\n';
String nestedSubscriptSource(int n) => 'y = ${'a[' * n}5${']' * n}\n';
String unaryPrefixChainSource(int n) => 'y = ${'-' * n}5\n';
String nestedArrayLiteralSource(int n) => 'y = ${'[' * n}5${']' * n}\n';

void main() {
  group('reference suite', () {
    test('program is the root', () {
      expect(parseIdl('x = 5\n').ruleName, equals('program'));
    });

    test('slash boolean keyword shorthand inside a procedure call', () {
      final ast = parseIdl('PLOT, x, /YLOG\n');
      final call = findRule(ast, 'procedure_call_stmt');
      expect(call, isNotNull);
      expect(containsRule(call!, 'bool_keyword_arg'), isTrue);
      final bka = findRule(call, 'bool_keyword_arg')!;
      expect(tokenValues(bka), equals(['/', 'YLOG']));
      // The SLASH was consumed by `bool_keyword_arg`, not by a
      // division — no `multiplicative` node anywhere contains a "/"
      // token value.
      expect(
          findRules(call, 'multiplicative')
              .any((n) => tokenValues(n).contains('/')),
          isFalse);
    });

    test('slash is ordinary division in an assignment rhs', () {
      final ast = parseIdl('x = a/YLOG\n');
      final assign = findRule(ast, 'assignment_stmt');
      expect(assign, isNotNull);
      final mult = findRule(assign!, 'multiplicative');
      expect(mult, isNotNull, reason: 'multiplicative — division tier');
      expect(tokenValues(mult!), equals(['a', '/', 'YLOG']));
      // No bool_keyword_arg anywhere — this statement has no
      // argument list at all.
      expect(containsRule(ast, 'bool_keyword_arg'), isFalse);
    });

    test('slash is ordinary division as a positional call argument', () {
      // PLOT, a/YLOG — a positional argument that is itself a
      // division, not a boolean keyword (the SLASH is not
      // argument-leading here).
      final ast = parseIdl('PLOT, a/YLOG\n');
      final call = findRule(ast, 'procedure_call_stmt');
      expect(call, isNotNull);
      expect(containsRule(call!, 'bool_keyword_arg'), isFalse);
      final mult = findRule(call, 'multiplicative');
      expect(mult, isNotNull);
      expect(tokenValues(mult!), equals(['a', '/', 'YLOG']));
    });

    test('boolean keyword shorthand in a function call too', () {
      // MA12 §3 item 2: keyword args (and by the same production,
      // the /BOOLEAN shorthand) work identically in procedure AND
      // function calls.
      final ast = parseIdl('y = HISTOGRAM(a, /NAN)\n');
      expect(containsRule(ast, 'bool_keyword_arg'), isTrue);
    });

    test('equals is keyword bind inside a call argument list', () {
      final ast = parseIdl("PLOT, x, TITLE='flux'\n");
      final call = findRule(ast, 'procedure_call_stmt');
      expect(call, isNotNull);
      expect(containsRule(call!, 'keyword_arg'), isTrue);
      expect(containsRule(call, 'assignment_stmt'), isFalse);
    });

    test('equals is ordinary assignment at statement level', () {
      final ast = parseIdl('x = 5\n');
      expect(containsRule(ast, 'assignment_stmt'), isTrue);
      expect(containsRule(ast, 'keyword_arg'), isFalse);
    });

    test('keyword arg and positional and boolean shorthand mix freely', () {
      final ast = parseIdl("PLOT, x, y, TITLE='flux', COLOR=255, /YLOG\n");
      final call = findRule(ast, 'procedure_call_stmt');
      expect(call, isNotNull);
      expect(containsRule(call!, 'keyword_arg'), isTrue);
      expect(containsRule(call, 'bool_keyword_arg'), isTrue);
    });

    test('bare name with no comma is an expr statement not a call', () {
      // Disclosed scope note (see idl.grammar's header comment): a
      // zero-arg call is syntactically identical to a bare variable
      // read.
      final ast = parseIdl('STOP\n');
      expect(containsRule(ast, 'expr_stmt'), isTrue);
      expect(containsRule(ast, 'procedure_call_stmt'), isFalse);
    });

    test('function call as an expression statement', () {
      final ast = parseIdl('SIN(x)\n');
      expect(containsRule(ast, 'expr_stmt'), isTrue);
      expect(containsRule(ast, 'call_suffix'), isTrue);
    });

    test('procedure call with only positional arguments', () {
      final ast = parseIdl('PRINT, x, y\n');
      expect(containsRule(ast, 'procedure_call_stmt'), isTrue);
    });

    test('plain assignment', () {
      expect(parses('x = 5\n'), isTrue);
    });

    test('subscripted assignment 1d', () {
      final ast = parseIdl('a[0] = 5\n');
      final assign = findRule(ast, 'assignment_stmt');
      expect(assign, isNotNull);
      expect(containsRule(assign!, 'index_suffix'), isTrue);
    });

    test('subscripted assignment 2d', () {
      expect(parses('a[0, 1] = 5\n'), isTrue);
    });

    test('subscript plain index', () {
      expect(parses('y = a[0]\n'), isTrue);
    });

    test('subscript two d', () {
      expect(parses('y = a[0, 1]\n'), isTrue);
    });

    test('subscript negative from end', () {
      final ast = parseIdl('y = a[-1]\n');
      expect(containsRule(ast, 'index_suffix'), isTrue);
      expect(containsRule(ast, 'unary'), isTrue);
    });

    test('subscript range', () {
      final ast = parseIdl('y = a[0:5]\n');
      expect(containsRule(ast, 'range_subscript'), isTrue);
    });

    test('subscript strided range', () {
      final ast = parseIdl('y = a[0:10:2]\n');
      expect(containsRule(ast, 'range_subscript'), isTrue);
    });

    test('subscript whole wildcard', () {
      expect(parses('y = a[*]\n'), isTrue);
    });

    test('subscript from start to wildcard', () {
      final ast = parseIdl('y = a[2:*]\n');
      expect(containsRule(ast, 'range_subscript'), isTrue);
      expect(containsRule(ast, 'range_end'), isTrue);
    });

    test('subscript from start to wildcard strided', () {
      expect(parses('y = a[2:*:3]\n'), isTrue);
    });

    test('array literal basic', () {
      final ast = parseIdl('a = [1, 2, 3]\n');
      expect(containsRule(ast, 'array_literal'), isTrue);
    });

    test('array literal immediately subscripted', () {
      expect(parses('y = [1, 2, 3][0]\n'), isTrue);
    });

    test('if then single statement no terminator needed', () {
      expect(parses('IF x GT 0 THEN y = 1\n'), isTrue);
    });

    test('if then else single statement forms', () {
      final ast = parseIdl('IF x GT 0 THEN y = 1 ELSE y = 2\n');
      expect(containsRule(ast, 'if_stmt'), isTrue);
    });

    test('if then block form with endif', () {
      expect(parses('IF x GT 0 THEN BEGIN\n y = 1\nENDIF\n'), isTrue);
    });

    test('if then block form with generic end', () {
      expect(parses('IF x GT 0 THEN BEGIN\n y = 1\nEND\n'), isTrue);
    });

    test('if then else block form with endelse', () {
      expect(
          parses(
              'IF x GT 0 THEN BEGIN\n y = 1\nENDIF ELSE BEGIN\n y = 2\nENDELSE\n'),
          isTrue);
    });

    test('for loop two arg range', () {
      expect(parses('FOR i = 0, 10 DO y = i\n'), isTrue);
    });

    test('for loop three arg range with step', () {
      expect(parses('FOR i = 0, 10, 2 DO y = i\n'), isTrue);
    });

    test('for loop block form', () {
      expect(parses('FOR i = 0, 10 DO BEGIN\n y = i\nENDFOR\n'), isTrue);
    });

    test('while loop single statement', () {
      expect(parses('WHILE x GT 0 DO x = x - 1\n'), isTrue);
    });

    test('while loop block form', () {
      expect(parses('WHILE x GT 0 DO BEGIN\n x = x - 1\nENDWHILE\n'), isTrue);
    });

    test('repeat until single statement', () {
      expect(parses('REPEAT x = x - 1 UNTIL x LE 0\n'), isTrue);
    });

    test('repeat until block form endrep precedes until', () {
      final ast = parseIdl('REPEAT BEGIN\n x = x - 1\nENDREP UNTIL x LE 0\n');
      expect(containsRule(ast, 'repeat_stmt'), isTrue);
    });

    test('repeat until reversed order is a syntax error', () {
      // UNTIL cannot precede ENDREP — confirms the order is
      // load-bearing, not accidentally accepted either way.
      expect(tryParseIdl('REPEAT BEGIN\n x = x - 1\nUNTIL x LE 0 ENDREP\n'),
          isNull);
    });

    test('break and continue inside loops', () {
      expect(parses('WHILE x GT 0 DO BEGIN\n BREAK\nENDWHILE\n'), isTrue);
      expect(parses('FOR i = 0, 10 DO BEGIN\n CONTINUE\nENDFOR\n'), isTrue);
    });

    test('generic begin end block as a statement', () {
      expect(parses('BEGIN\n x = 1\n y = 2\nEND\n'), isTrue);
    });

    test('pro def with no params', () {
      final ast = parseIdl('PRO simple\n PRINT, 1\nEND\n');
      expect(containsRule(ast, 'pro_def'), isTrue);
    });

    test('pro def with positional and keyword params', () {
      final ast = parseIdl('PRO plot_it, x, y, COLOR=color\n PRINT, x\nEND\n');
      expect(containsRule(ast, 'pro_def'), isTrue);
      expect(containsRule(ast, 'params'), isTrue);
    });

    test('pro def return with no expression', () {
      final ast = parseIdl(
          'PRO early_out, x\n IF x GT 0 THEN RETURN\n PRINT, x\nEND\n');
      expect(containsRule(ast, 'return_stmt'), isTrue);
    });

    test('func def with return value', () {
      final ast = parseIdl('FUNCTION square, x\n RETURN, x * x\nEND\n');
      expect(containsRule(ast, 'func_def'), isTrue);
      final ret = findRule(ast, 'return_stmt');
      expect(ret, isNotNull);
      expect(tokenValues(ret!), contains('*'));
    });

    test('func def with keyword param', () {
      expect(
          parses(
              'FUNCTION scaled, x, FACTOR=factor\n RETURN, x * factor\nEND\n'),
          isTrue);
    });

    test('logical is loosest', () {
      final ast = parseIdl('y = a EQ b AND c EQ d\n');
      expect(containsRule(ast, 'logical'), isTrue);
      expect(containsRule(ast, 'comparison'), isTrue);
    });

    test('comparison binds looser than additive', () {
      final ast = parseIdl('y = a + 1 EQ b - 1\n');
      expect(containsRule(ast, 'comparison'), isTrue);
      expect(containsRule(ast, 'additive'), isTrue);
    });

    test('all six comparison operators parse at the same tier', () {
      for (final op in ['EQ', 'NE', 'LE', 'LT', 'GE', 'GT']) {
        final src = 'y = a $op b\n';
        expect(parses(src), isTrue, reason: '`$src` should parse');
      }
    });

    test('all three logical operators parse', () {
      for (final op in ['AND', 'OR', 'XOR']) {
        final src = 'y = a $op b\n';
        expect(parses(src), isTrue, reason: '`$src` should parse');
      }
    });

    test('unary minus binds looser than multiplicative and power', () {
      // -a*b == -(a*b): the unary MINUS must wrap the whole
      // multiplicative term, per IDL's own documented precedence
      // (tier 5, same as binary +/-, NOT tighter than * the way
      // Scilab/MATLAB's own unary sits).
      final ast = parseIdl('y = -a*b\n');
      final unary = findRule(ast, 'unary');
      expect(unary, isNotNull);
      expect(tokenValues(unary!), equals(['-', 'a', '*', 'b']));
    });

    test('unary not parses at the same tier as unary minus', () {
      expect(parses('y = NOT a\n'), isTrue);
      final ast = parseIdl('y = NOT a EQ b\n');
      // NOT binds tighter than EQ (tier 5 vs tier 6): `(NOT a) EQ b`.
      expect(containsRule(ast, 'comparison'), isTrue);
      expect(containsRule(ast, 'unary'), isTrue);
    });

    test('power is left associative', () {
      // 2^3^2 == (2^3)^2 in real IDL (left-to-right, per the
      // official precedence table), NOT right-associative like
      // Scilab/MATLAB's `^`.
      final ast = parseIdl('y = 2^3^2\n');
      final power = findRule(ast, 'power');
      expect(power, isNotNull);
      expect(tokenValues(power!), equals(['2', '^', '3', '^', '2']));
    });

    test('multiplicative includes both matrix product operators', () {
      expect(parses('y = a # b\n'), isTrue);
      expect(parses('y = a ## b\n'), isTrue);
    });

    test('grouping parens parse', () {
      expect(parses('y = (1 + 2) * 3\n'), isTrue);
      expect(
        containsRule(parseIdl('y = (1 + 2) * 3\n'), 'group'),
        isTrue,
      );
    });

    test('syntax error is reported', () {
      expect(tryParseIdl('y = 1 +\n'), isNull);
      expect(tryParseIdl('y = (1 + 2\n'), isNull);
      expect(() => parseIdl('y = 1 +\n'), throwsA(isA<GrammarParseError>()));
    });

    test('strings single and double quoted', () {
      expect(parses("s = 'hello'\n"), isTrue);
      expect(parses('s = "hello"\n'), isTrue);
    });
  });

  group('depth guard (mirrors the reference suite)', () {
    test('deeply nested input errors for every shape', () {
      // 5000 levels (2000 for the verbose `IF` shape) — far past
      // maxRuleDepth — must produce a recoverable error, not
      // overflow the stack, for all six shapes.
      for (final src in [
        nestedParenSource(5000),
        nestedIfSource(2000),
        nestedCallSource(5000),
        nestedSubscriptSource(5000),
        unaryPrefixChainSource(5000),
        nestedArrayLiteralSource(5000),
      ]) {
        expect(() => parseIdl(src), throwsA(isA<GrammarParseError>()));
        expect(tryParseIdl(src), isNull);
      }
    });

    test('cap trips before overflow for every shape', () {
      // The Dart equivalent of the reference's default-stack-thread
      // test: far-too-deep input surfaces a catchable engine error
      // with no stack exhaustion on the ordinary event loop, and the
      // isolate stays healthy afterwards.
      for (final src in [
        nestedParenSource(5000),
        nestedIfSource(2000),
        nestedCallSource(5000),
        nestedSubscriptSource(5000),
        unaryPrefixChainSource(5000),
        nestedArrayLiteralSource(5000),
      ]) {
        try {
          parseIdl(src);
          fail('must not parse');
        } on GrammarParseError catch (e) {
          expect(e.message, isNotEmpty);
        }
      }
      expect(parseIdl('x = 5\n').ruleName, equals('program'));
    });

    test('reasonable nesting stays under the cap for every shape', () {
      expect(tryParseIdl(nestedParenSource(3)) != null, isTrue);
      expect(tryParseIdl(nestedIfSource(3)) != null, isTrue);
      expect(tryParseIdl(nestedCallSource(3)) != null, isTrue);
      expect(tryParseIdl(nestedSubscriptSource(3)) != null, isTrue);
      expect(tryParseIdl(unaryPrefixChainSource(5)) != null, isTrue);
      expect(tryParseIdl(nestedArrayLiteralSource(3)) != null, isTrue);
    });

    test('headroom boundary for every shape', () {
      // Re-measured on this engine via scratch binary-search
      // probes, all identical to the reference: parens 13/14,
      // nested `IF` 26/27, call args 12/13, subscripts 11/12, unary
      // chain 134/135, array literals 12/13.
      expect(tryParseIdl(nestedParenSource(13)) != null, isTrue);
      expect(tryParseIdl(nestedParenSource(14)), isNull);

      expect(tryParseIdl(nestedIfSource(26)) != null, isTrue);
      expect(tryParseIdl(nestedIfSource(27)), isNull);

      expect(tryParseIdl(nestedCallSource(12)) != null, isTrue);
      expect(tryParseIdl(nestedCallSource(13)), isNull);

      expect(tryParseIdl(nestedSubscriptSource(11)) != null, isTrue);
      expect(tryParseIdl(nestedSubscriptSource(12)), isNull);

      expect(tryParseIdl(unaryPrefixChainSource(134)) != null, isTrue);
      expect(tryParseIdl(unaryPrefixChainSource(135)), isNull);

      expect(tryParseIdl(nestedArrayLiteralSource(12)) != null, isTrue);
      expect(tryParseIdl(nestedArrayLiteralSource(13)), isNull);
    });
  });

  group('twins and errors', () {
    test('factory returns a wired parser', () {
      expect(
        createIdlParser('x = 5\n').parse().ruleName,
        equals('program'),
      );
    });
  });
}
