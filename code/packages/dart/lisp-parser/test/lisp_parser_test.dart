/// Port of the reference `rust/lisp-parser` unit suite (all 32
/// tests, with the 48/49 nesting boundary re-measured on this
/// engine) plus white-box extractor tests pinning every fail-closed
/// `Internal error` arm, value-semantics tests, and nullable-twin
/// tests.
library;

import 'dart:io';

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_lisp_lexer/lisp_lexer.dart' as lisp;
import 'package:coding_adventures_lisp_parser/lisp_parser.dart';
import 'package:coding_adventures_lisp_parser/src/extractor.dart' as extractor;
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

/// Collect all atom values from a program.
List<String> findAllAtoms(List<SExpr> program) {
  final out = <String>[];
  for (final expr in program) {
    out.addAll(expr.findAtoms());
  }
  return out;
}

/// Count all list nodes in a program.
int countAllLists(List<SExpr> program) =>
    program.fold<int>(0, (n, e) => n + e.countLists());

/// Count all quoted nodes in a program.
int countAllQuoted(List<SExpr> program) =>
    program.fold<int>(0, (n, e) => n + e.countQuoted());

Token tok(String type, String value) =>
    Token(type: type, value: value, line: 1, column: 1);

ASTNode node(String rule, List<Object> children) =>
    ASTNode(ruleName: rule, children: children);

void main() {
  group('reference suite', () {
    test('empty program', () {
      expect(parseLisp(''), isEmpty);
    });

    test('multiple top level', () {
      expect(parseLisp('1 2 3').length, equals(3));
    });

    test('number', () {
      expect(findAllAtoms(parseLisp('42')), equals(['42']));
    });

    test('negative number', () {
      expect(findAllAtoms(parseLisp('-7')), equals(['-7']));
    });

    test('symbol', () {
      expect(findAllAtoms(parseLisp('define')), equals(['define']));
    });

    test('operator symbol', () {
      expect(findAllAtoms(parseLisp('+')), equals(['+']));
    });

    test('string', () {
      expect(findAllAtoms(parseLisp('"hello"')).length, equals(1));
    });

    test('empty list', () {
      expect(countAllLists(parseLisp('()')), equals(1));
    });

    test('simple list', () {
      expect(findAllAtoms(parseLisp('(1 2 3)')), equals(['1', '2', '3']));
    });

    test('nested list', () {
      expect(countAllLists(parseLisp('((1 2) (3 4))')), equals(3));
    });

    test('function call', () {
      expect(findAllAtoms(parseLisp('(+ 1 2)')), equals(['+', '1', '2']));
    });

    test('define', () {
      expect(findAllAtoms(parseLisp('(define x 42)')),
          equals(['define', 'x', '42']));
    });

    test('deeply nested', () {
      expect(findAllAtoms(parseLisp('(+ (* 2 3) (- 10 4))')),
          equals(['+', '*', '2', '3', '-', '10', '4']));
    });

    test('quoted symbol', () {
      final program = parseLisp("'foo");
      expect(countAllQuoted(program), equals(1));
      expect(findAllAtoms(program), equals(['foo']));
    });

    test('quoted list', () {
      final program = parseLisp("'(1 2 3)");
      expect(countAllQuoted(program), equals(1));
      expect(findAllAtoms(program), equals(['1', '2', '3']));
    });

    test('quoted in expression', () {
      expect(countAllQuoted(parseLisp("(eq 'foo 'bar)")), equals(2));
    });

    test('simple dotted pair', () {
      final program = parseLisp('(a . b)');
      expect(findAllAtoms(program), equals(['a', 'b']));
      expect(program.single, isA<SDottedPair>());
    });

    test('numeric dotted pair', () {
      expect(findAllAtoms(parseLisp('(1 . 2)')), equals(['1', '2']));
    });

    test('lambda', () {
      final atoms = findAllAtoms(parseLisp('(lambda (x) (* x x))'));
      expect(atoms, contains('lambda'));
      expect(atoms, contains('x'));
      expect(atoms, contains('*'));
    });

    test('cond', () {
      final atoms = findAllAtoms(parseLisp('(cond ((eq x 0) 1) (t x))'));
      expect(atoms, contains('cond'));
      expect(atoms, contains('eq'));
      expect(atoms, contains('t'));
    });

    test('factorial', () {
      const source = '(define factorial '
          '(lambda (n) '
          '(cond ((eq n 0) 1) '
          '(t (* n (factorial (- n 1)))))))';
      final program = parseLisp(source);
      expect(program.length, equals(1));
      final atoms = findAllAtoms(program);
      expect(atoms, contains('define'));
      expect(atoms, contains('factorial'));
      expect(atoms, contains('lambda'));
      expect(atoms, contains('cond'));
    });

    test('multiple definitions', () {
      const source = '(define x 10)\n(define y 20)\n(+ x y)\n';
      expect(parseLisp(source).length, equals(3));
    });

    test('cons car cdr', () {
      expect(findAllAtoms(parseLisp('(car (cons 1 2))')),
          equals(['car', 'cons', '1', '2']));
    });

    test('unmatched lparen', () {
      expect(() => parseLisp('(+ 1 2'), throwsA(isA<LispParseError>()));
      expect(tryParseLisp('(+ 1 2'), isNull);
    });

    test('unexpected rparen', () {
      expect(() => parseLisp(')'), throwsA(isA<LispParseError>()));
      expect(tryParseLisp(')'), isNull);
    });

    test('checked in grammar matches lisp grammar', () {
      // Drift guard: the compiled `_grammar.dart` must equal a fresh
      // parse of the canonical source. Compared semantically
      // (version + rules, all deep-==), not byte-wise: `dart format`
      // reflows the emitted file, so bytes legitimately differ. Runs
      // from the package directory (guaranteed: test discovery
      // itself requires it).
      final packageDir = Directory.current.path;
      final source = File('$packageDir/../../../grammars/lisp/lisp.grammar')
          .readAsStringSync();
      final fresh = parseParserGrammar(source);
      final compiled = lispParserGrammar();
      expect(fresh.version, equals(compiled.version));
      expect(fresh.rules, equals(compiled.rules));
    });

    test('nesting at the limit parses', () {
      final open = '(' * maxNestingDepth;
      final close = ')' * maxNestingDepth;
      expect(parseLisp('$open x $close'), isNotNull);
    });

    test('nesting past the limit fails closed', () {
      final open = '(' * (maxNestingDepth + 1);
      final close = ')' * (maxNestingDepth + 1);
      try {
        parseLisp('$open x $close');
        fail('must not parse');
      } on LispParseError catch (e) {
        expect(e.message, contains('MAX_NESTING_DEPTH'));
      }
      expect(tryParseLisp('$open x $close'), isNull);
    });

    test('quote runs count toward nesting', () {
      // `quoted = QUOTE sexpr` recurses without consuming a paren,
      // so pending quotes count like parens.
      final over = '${"'" * (maxNestingDepth + 1)}x';
      try {
        parseLisp(over);
        fail('must not parse');
      } on LispParseError catch (e) {
        expect(e.message, contains('MAX_NESTING_DEPTH'));
      }
      expect(tryParseLisp(over), isNull);
      final ok = '${"'" * maxNestingDepth}x';
      expect(parseLisp(ok), isNotNull);
      expect(tryParseLisp(ok) != null, isTrue);
    });

    test('leading dot is an error', () {
      // The hand reader accepted `(. b)` as an empty-headed pair;
      // the grammar requires an element before DOT (`list_body = [
      // sexpr { sexpr } [ DOT sexpr ] ]`), and the grammar governs.
      expect(() => parseLisp('(. b)'), throwsA(isA<LispParseError>()));
      expect(tryParseLisp('(. b)'), isNull);
    });

    test('empty token stream parses to empty program', () {
      // Security review: the hand reader panicked indexing past the
      // end; the grammar accepts empty input, so return it.
      expect(parseLispTokens(const []), isEmpty);
      expect(tryParseLispTokens(const []), equals([]));
    });

    test('spec accessor exposes grammar', () {
      final names = lispParserGrammar().ruleNames();
      expect(names, contains('program'));
      expect(names, contains('sexpr'));
      expect(names, contains('list_body'));
    });
  });

  group('extractor internals fail closed', () {
    ASTNode atomNode(String type, String value) =>
        node('atom', [tok(type, value)]);

    test('sexpr rejects wrong child counts and shapes', () {
      expect(
          () => extractor.extractSexpr(node('sexpr', [])),
          throwsA(isA<LispParseError>()
              .having((e) => e.message, 'message', contains('want 1'))));
      expect(
          () => extractor.extractSexpr(node(
              'sexpr', [atomNode('SYMBOL', 'a'), atomNode('SYMBOL', 'b')])),
          throwsA(isA<LispParseError>()
              .having((e) => e.message, 'message', contains('want 1'))));
      expect(
          () => extractor.extractSexpr(node('sexpr', [tok('SYMBOL', 'a')])),
          throwsA(isA<LispParseError>().having(
              (e) => e.message, 'message', contains('want a rule node'))));
      expect(
          () => extractor.extractSexpr(node('sexpr', [node('bogus', [])])),
          throwsA(isA<LispParseError>()
              .having((e) => e.message, 'message', contains('bogus'))));
    });

    test('atom rejects non-leaves and unknown token types', () {
      expect(
          () => extractor.extractAtom(
              node('atom', [tok('SYMBOL', 'a'), tok('SYMBOL', 'b')])),
          throwsA(isA<LispParseError>()
              .having((e) => e.message, 'message', contains('not a leaf'))));
      expect(
          () => extractor.extractAtom(atomNode('DOT', '.')),
          throwsA(isA<LispParseError>()
              .having((e) => e.message, 'message', contains('DOT'))));
      // All three atom kinds lower.
      expect(extractor.extractAtom(atomNode('NUMBER', '42')),
          equals(const SAtom(AtomKind.number, '42')));
      expect(extractor.extractAtom(atomNode('SYMBOL', 'x')),
          equals(const SAtom(AtomKind.symbol, 'x')));
      expect(extractor.extractAtom(atomNode('STRING', '"hi"')),
          equals(const SAtom(AtomKind.string, '"hi"')));
    });

    test('list rejects malformed bodies', () {
      expect(
          () => extractor.extractList(node('list', [tok('LPAREN', '(')])),
          throwsA(isA<LispParseError>().having(
              (e) => e.message, 'message', contains('no list_body child'))));
      // DOT with no following node.
      expect(
          () => extractor.extractList(node('list', [
                node('list_body', [
                  node('sexpr', [atomNode('SYMBOL', 'a')]),
                  tok('DOT', '.'),
                ])
              ])),
          throwsA(isA<LispParseError>().having((e) => e.message, 'message',
              contains('DOT tail is not a rule node'))));
      // Two DOT tails.
      expect(
          () => extractor.extractList(node('list', [
                node('list_body', [
                  node('sexpr', [atomNode('SYMBOL', 'a')]),
                  tok('DOT', '.'),
                  node('sexpr', [atomNode('SYMBOL', 'b')]),
                  tok('DOT', '.'),
                  node('sexpr', [atomNode('SYMBOL', 'c')]),
                ])
              ])),
          throwsA(isA<LispParseError>()
              .having((e) => e.message, 'message', contains('two DOT tails'))));
      // Non-DOT stray token.
      expect(
          () => extractor.extractList(node('list', [
                node('list_body', [tok('LPAREN', '(')]),
              ])),
          throwsA(isA<LispParseError>()
              .having((e) => e.message, 'message', contains('LPAREN'))));
      // Foreign object child.
      expect(
          () => extractor.extractList(node('list', [
                node('list_body', ['nope']),
              ])),
          throwsA(isA<LispParseError>().having(
              (e) => e.message, 'message', contains('foreign object'))));
    });

    test('quoted rejects zero or two sexprs', () {
      expect(
          () => extractor.extractQuoted(node('quoted', [tok('QUOTE', "'")])),
          throwsA(isA<LispParseError>().having(
              (e) => e.message, 'message', contains('no sexpr child'))));
      expect(
          () => extractor.extractQuoted(node('quoted', [
                node('sexpr', [atomNode('SYMBOL', 'a')]),
                node('sexpr', [atomNode('SYMBOL', 'b')]),
              ])),
          throwsA(isA<LispParseError>()
              .having((e) => e.message, 'message', contains('two sexprs'))));
    });

    test('program rejects token and foreign children', () {
      expect(
          () => extractor.extractProgram(node('program', [tok('EOF', '')])),
          throwsA(isA<LispParseError>()
              .having((e) => e.message, 'message', contains('EOF'))));
      expect(
          () => extractor.extractProgram(node('program', ['nope'])),
          throwsA(isA<LispParseError>().having(
              (e) => e.message, 'message', contains('foreign object'))));
    });
  });

  group('value semantics', () {
    test('SExpr equality, hash, and rendering', () {
      const a = SAtom(AtomKind.symbol, 'x');
      const b = SAtom(AtomKind.symbol, 'x');
      const c = SAtom(AtomKind.number, 'x');
      expect(a, equals(b));
      expect(a.hashCode, equals(b.hashCode));
      expect(a == c, isFalse);
      expect(a == 'x', isFalse);
      expect(a.toString(), contains('x'));

      const l1 = SList([a]);
      const l2 = SList([b]);
      const l3 = SList([]);
      expect(l1, equals(l2));
      expect(l1.hashCode, equals(l2.hashCode));
      expect(l1 == l3, isFalse);
      expect(l1 == a, isFalse);
      expect(l1.toString(), contains('SList'));

      const d1 = SDottedPair([a], b);
      const d2 = SDottedPair([b], a);
      const d3 = SDottedPair([], b);
      expect(d1, equals(d2));
      expect(d1.hashCode, equals(d2.hashCode));
      expect(d1 == d3, isFalse);
      expect(d1 == l1, isFalse);
      expect(d1.toString(), contains('SDottedPair'));

      const q1 = SQuoted(a);
      const q2 = SQuoted(b);
      const q3 = SQuoted(c);
      expect(q1, equals(q2));
      expect(q1.hashCode, equals(q2.hashCode));
      expect(q1 == q3, isFalse);
      expect(q1 == a, isFalse);
      expect(q1.toString(), contains('SQuoted'));

      // Counter coverage on dotted shapes.
      final dotted = parseLisp('(a . b)').single as SDottedPair;
      expect(dotted.countLists(), equals(1));
      // countLists passes through quotes.
      expect(parseLisp("'(a)").single.countLists(), equals(1));
      final quotedDotted = parseLisp("' (a . b)").single;
      expect(quotedDotted.countQuoted(), equals(1));
      expect(parseLisp("('a . 'b)").single.countQuoted(), equals(2));
      expect(findAllAtoms(parseLisp('(a . b)')), equals(['a', 'b']));
    });

    test('LispParseError equality and rendering', () {
      const e1 = LispParseError('nope');
      const e2 = LispParseError('nope');
      const e3 = LispParseError('other');
      expect(e1, equals(e2));
      expect(e1.hashCode, equals(e2.hashCode));
      expect(e1 == e3, isFalse);
      expect(e1 == 'nope', isFalse);
      expect(e1.toString(), equals('ParseError: nope'));
      expect(extractor.internalError('x').message, contains('Internal error'));
    });
  });

  group('twins and errors', () {
    test('lexer errors surface with positions', () {
      try {
        parseLisp('#t');
        fail('must not parse');
      } on LispParseError catch (e) {
        expect(e.message, contains('Lexer error'));
      }
      expect(tryParseLisp('#t'), isNull);
    });

    test('token twins agree with source twins', () {
      final tokens = lisp.tokenize('(+ 1 2)');
      expect(parseLispTokens(tokens), equals(parseLisp('(+ 1 2)')));
      expect(tryParseLispTokens(tokens), equals(tryParseLisp('(+ 1 2)')));
      expect(tryParseLispTokens(lisp.tokenize('(')), isNull);
    });

    test('factory entry points stay wired', () {
      expect(lispParserGrammar().ruleNames(), contains('program'));
    });
  });
}
