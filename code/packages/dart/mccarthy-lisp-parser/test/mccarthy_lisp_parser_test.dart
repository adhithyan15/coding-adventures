/// Unit tests for `coding_adventures_mccarthy_lisp_parser`.
/// Mirrors of the reference suite — the 22 `lib.rs` unit tests, the 6
/// depth-guard tests, and the 26 integration tests (exact duplicates
/// folded together, the stronger variant kept) — plus API and
/// hand-built-CST extractor-shape tests pinning the Dart divergences
/// (throwing API + nullable twin, wrapped errors, GC reclamation).
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_mccarthy_lisp_lexer/mccarthy_lisp_lexer.dart';
import 'package:coding_adventures_mccarthy_lisp_parser/mccarthy_lisp_parser.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

/// Parse a source that must contain exactly one top-level form.
LispExpr one(String src) {
  final forms = parseMcCarthy(src);
  expect(forms, hasLength(1));
  return forms.single;
}

/// Parse a source that must fail, returning the structured error.
McCarthyParseError parseErr(String src) {
  try {
    parseMcCarthy(src);
  } on McCarthyParseError catch (e) {
    return e;
  }
  fail('expected McCarthyParseError for ${src.length} chars');
}

String nestedListSource(int n) => '${'(' * n}A${')' * n}';

String nestedQuoteSource(int n) => "${'\'' * n}A";

/// Parse via [createMcCarthyParserFromTokens], bypassing the
/// nesting pre-scan so the engine-level guard is exercised directly.
ASTNode parseBypassingPrescan(String src) =>
    createMcCarthyParserFromTokens(tokenizeMcCarthy(src)).parse();

void main() {
  group('atoms', () {
    test('symbol atom', () {
      expect(one('CAR'), equals(LispExpr.sym('CAR')));
      expect(one('FF'), equals(LispExpr.sym('FF')));
      expect(one('LIST-OF-3'), equals(LispExpr.sym('LIST-OF-3')));
    });

    test('integer atom', () {
      expect(one('0'), equals(const LispInt(0)));
      expect(one('42'), equals(const LispInt(42)));
      expect(one('-7'), equals(const LispInt(-7)));
      expect(one('-1'), equals(const LispInt(-1)));
    });

    test('i64 boundaries parse; just past them errors', () {
      expect(one('9223372036854775807'),
          equals(const LispInt(9223372036854775807)));
      expect(one('-9223372036854775808'),
          equals(const LispInt(-9223372036854775808)));
      expect(parseErr('9223372036854775808').message,
          contains('does not fit in i64'));
      expect(parseErr('-9223372036854775809').message,
          contains('does not fit in i64'));
    });
  });

  group('empty list / NIL', () {
    test('empty list is nil', () {
      expect(one('()'), equals(const LispNil()));
    });

    test('nil symbol is a symbol', () {
      // At the parser layer, `NIL` the symbol is distinct from `()`.
      expect(one('NIL'), equals(LispExpr.sym('NIL')));
    });
  });

  group('lists', () {
    test('proper list nests to cons cells', () {
      // (A B C) → (A . (B . (C . NIL)))
      final expected = LispCons(
        LispExpr.sym('A'),
        LispCons(
          LispExpr.sym('B'),
          LispCons(LispExpr.sym('C'), const LispNil()),
        ),
      );
      expect(one('(A B C)'), equals(expected));
      expect(
        one('(A B C)'),
        equals(LispExpr.list(
            [LispExpr.sym('A'), LispExpr.sym('B'), LispExpr.sym('C')])),
      );
    });

    test('singleton list', () {
      expect(one('(A)'), equals(LispExpr.list([LispExpr.sym('A')])));
    });

    test('nested lists', () {
      expect(
        one('((A) (B))'),
        equals(LispExpr.list([
          LispExpr.list([LispExpr.sym('A')]),
          LispExpr.list([LispExpr.sym('B')]),
        ])),
      );
    });

    test('simple dotted pair', () {
      expect(
        one('(A . B)'),
        equals(LispCons(LispExpr.sym('A'), LispExpr.sym('B'))),
      );
    });

    test('dotted tail after elements', () {
      // (A B . C) → (A . (B . C))
      expect(
        one('(A B . C)'),
        equals(LispCons(
          LispExpr.sym('A'),
          LispCons(LispExpr.sym('B'), LispExpr.sym('C')),
        )),
      );
    });

    test('dotted pair with integer cdr', () {
      expect(
        one('(A . -42)'),
        equals(LispCons(LispExpr.sym('A'), const LispInt(-42))),
      );
    });
  });

  group('quote sugar', () {
    test('quote a symbol', () {
      expect(one("'X"), equals(LispExpr.quote(LispExpr.sym('X'))));
    });

    test('quote a list', () {
      expect(
        one("'(A B)"),
        equals(LispExpr.quote(
            LispExpr.list([LispExpr.sym('A'), LispExpr.sym('B')]))),
      );
      expect(
        one("'(A B C)"),
        equals(LispExpr.quote(LispExpr.list([
          LispExpr.sym('A'),
          LispExpr.sym('B'),
          LispExpr.sym('C'),
        ]))),
      );
    });

    test('nested quotes', () {
      // ''X → (QUOTE (QUOTE X))
      expect(one("''X"),
          equals(LispExpr.quote(LispExpr.quote(LispExpr.sym('X')))));
    });
  });

  group('canonical paper examples', () {
    test('car of a literal list', () {
      expect(
        one("(CAR '(A B C))"),
        equals(LispExpr.list([
          LispExpr.sym('CAR'),
          LispExpr.quote(LispExpr.list([
            LispExpr.sym('A'),
            LispExpr.sym('B'),
            LispExpr.sym('C'),
          ])),
        ])),
      );
    });

    test('identity lambda', () {
      expect(
        one('(LAMBDA (X) X)'),
        equals(LispExpr.list([
          LispExpr.sym('LAMBDA'),
          LispExpr.list([LispExpr.sym('X')]),
          LispExpr.sym('X'),
        ])),
      );
    });

    test('label recursive definition', () {
      const src =
          '(LABEL FF (LAMBDA (X) (COND ((ATOM X) X) ((QUOTE T) (FF (CAR X))))))';
      final form = one(src);
      // Spot-check the spine: (LABEL FF (LAMBDA ...))
      expect(form, isA<LispCons>());
      expect((form as LispCons).car, equals(LispExpr.sym('LABEL')));
    });

    test('cond form', () {
      const src = "(COND ((ATOM X) (QUOTE A)) ('T (QUOTE B)))";
      final form = one(src);
      expect(form, isA<LispCons>());
      expect((form as LispCons).car, equals(LispExpr.sym('COND')));
    });
  });

  group('programs', () {
    test('empty program', () {
      expect(parseMcCarthy(''), isEmpty);
      expect(parseMcCarthy('  ; just a comment\n'), isEmpty);
    });

    test('multiple top-level forms', () {
      final forms = parseMcCarthy('(CAR X) (CDR X)');
      expect(forms, hasLength(2));
    });

    test('sequence of forms', () {
      final forms = parseMcCarthy('(CAR X)\n(CDR X)\nNIL');
      expect(forms, hasLength(3));
      expect(forms[2], equals(LispExpr.sym('NIL')));
    });
  });

  group('display', () {
    test('display prints dotted cons form', () {
      // Display always prints the fully-dotted cons form.
      expect(one('(A B)').toString(), equals('(A . (B . NIL))'));
      expect(one('(A . B)').toString(), equals('(A . B)'));
      expect(one('NIL').toString(), equals('NIL'));
      expect(one('()').toString(), equals('NIL'));
      expect(one('-5').toString(), equals('-5'));
    });

    test('every case renders', () {
      expect(const LispNil().toString(), equals('NIL'));
      expect(LispExpr.sym('FOO').toString(), equals('FOO'));
      expect(const LispInt(7).toString(), equals('7'));
      expect(
        const LispCons(LispInt(1), LispNil()).toString(),
        equals('(1 . NIL)'),
      );
    });
  });

  group('errors', () {
    test('unbalanced parens error', () {
      expect(() => parseMcCarthy('(CAR'), throwsA(isA<McCarthyParseError>()));
      expect(() => parseMcCarthy('CAR)'), throwsA(isA<McCarthyParseError>()));
      expect(() => parseMcCarthy('((('), throwsA(isA<McCarthyParseError>()));
    });

    test('malformed dotted forms error', () {
      expect(() => parseMcCarthy('(. X)'),
          throwsA(isA<McCarthyParseError>())); // no head before dot
      expect(() => parseMcCarthy('(A . B C)'),
          throwsA(isA<McCarthyParseError>())); // extra after tail
      expect(() => parseMcCarthy('(A . . B)'),
          throwsA(isA<McCarthyParseError>())); // two dots
      expect(() => parseMcCarthy('(A .)'),
          throwsA(isA<McCarthyParseError>())); // dot with no cdr
    });

    test('dialect violations error', () {
      expect(() => parseMcCarthy('car'), throwsA(isA<McCarthyParseError>()));
      expect(
          () => parseMcCarthy('(+ A B)'), throwsA(isA<McCarthyParseError>()));
      expect(() => parseMcCarthy('"hi"'), throwsA(isA<McCarthyParseError>()));
    });

    test('lexer errors propagate wrapped with positions', () {
      expect(() => parseMcCarthy('(- A B)'),
          throwsA(isA<McCarthyParseError>())); // operator symbol
      final err = parseErr('car');
      expect(err.message, startsWith('lexer error: '));
      expect(err.line, equals(1));
      expect(err.column, equals(1));
      expect(err.toString(), startsWith('parse error at 1:1: '));
    });

    test('integer overflow errors', () {
      final err = parseErr('99999999999999999999');
      expect(err.message, contains('does not fit in i64'));
      expect(() => parseMcCarthy('123456789012345678901234567890'),
          throwsA(isA<McCarthyParseError>()));
    });

    test('deeply nested parens are rejected', () {
      final deep = nestedListSource(maxParenDepth + 5);
      expect(parseErr(deep).message, contains('MAX_PAREN_DEPTH'));
    });

    test('deeply nested quotes are rejected', () {
      // Regression for the quote-chain stack-overflow DoS: a long run
      // of `'` has paren-depth 0 but unbounded parser recursion. It
      // must be rejected *before* the GrammarParser runs, not abort
      // the process. MAX+5000 is far past the cap and well into crash
      // territory if the guard regresses.
      final deep = nestedQuoteSource(maxParenDepth + 5000);
      expect(parseErr(deep).message, contains('MAX_PAREN_DEPTH'));
    });

    test('quote then deep parens rejected', () {
      // A quote wrapping a deep list — the quote adds one level on
      // top of the parens, so the combined depth must be what trips.
      final deep = "'${'(' * maxParenDepth}A${')' * maxParenDepth}";
      expect(() => parseMcCarthy(deep), throwsA(isA<McCarthyParseError>()));
    });

    test('quote directly before a close paren discharges in the prescan', () {
      // `'(')` is `(QUOTE LPAREN QUOTE RPAREN)`: the prescan pops the
      // pending quote at the close, then the grammar rejects the
      // operand-less quote.
      expect(() => parseMcCarthy("'(')"), throwsA(isA<McCarthyParseError>()));
    });

    test('pathological nesting does not crash', () {
      // DoS hardening: neither a deep paren nest nor a long quote
      // chain (which has paren-depth 0 but unbounded parser
      // recursion) may abort the process — both must fail cleanly.
      expect(
          () => parseMcCarthy('(' * 10000), throwsA(isA<McCarthyParseError>()));
      expect(() => parseMcCarthy("${'\'' * 10000}X"),
          throwsA(isA<McCarthyParseError>()));
    });
  });

  group('moderate nesting accepted', () {
    test('legal moderate paren nesting parses', () {
      // Comfortably under maxParenDepth (64) — must parse cleanly.
      expect(parseMcCarthy(nestedListSource(40)), hasLength(1));
    });

    test('legal moderate quote chain parses', () {
      // A quote chain just under the cap parses fine (and does not
      // overflow): 40 quotes → 40 nested QUOTE forms.
      expect(parseMcCarthy(nestedQuoteSource(40)), hasLength(1));
    });
  });

  group('depth guard bypassing the prescan', () {
    test('deeply nested lists fail clean, not overflow', () {
      // 5000 levels — far past the engine cap — must produce a
      // recoverable GrammarParseError, never a StackOverflowError.
      // (Dart has no thread-stack tuning, so this runs in-process:
      // the guard firing first is exactly what is under test.)
      expect(() => parseBypassingPrescan(nestedListSource(5000)),
          throwsA(isA<GrammarParseError>()));
    });

    test('list nesting up to the cap parses; one past trips', () {
      expect(parseBypassingPrescan(nestedListSource(59)).ruleName,
          equals('program'));
      expect(() => parseBypassingPrescan(nestedListSource(60)),
          throwsA(isA<GrammarParseError>()));
    });

    test('deeply nested quotes fail clean, not overflow', () {
      expect(() => parseBypassingPrescan(nestedQuoteSource(5000)),
          throwsA(isA<GrammarParseError>()));
    });

    test('quote chain up to the cap parses; one past trips', () {
      expect(parseBypassingPrescan(nestedQuoteSource(88)).ruleName,
          equals('program'));
      expect(() => parseBypassingPrescan(nestedQuoteSource(89)),
          throwsA(isA<GrammarParseError>()));
    });
  });

  group('huge flat list', () {
    test('builds without overflow', () {
      // A flat list `(A A … A)` is only paren-depth 1, so it bypasses
      // the nesting cap, yet it builds an N-deep Cons chain. Building
      // it (parser + extractor) must avoid recursing to a stack
      // overflow, and Dart's tracing collector reclaims it without
      // the reference's iterative `Drop` — there is no recursive
      // destructor to overflow. Only shallow assertions: no deep
      // equals, hash, or print of the 100k spine.
      const n = 100000;
      final buf = StringBuffer('(');
      for (var i = 0; i < n; i++) {
        buf.write('A ');
      }
      buf.write(')');
      final forms = parseMcCarthy(buf.toString());
      expect(forms, hasLength(1));
      final head = forms[0];
      expect(head, isA<LispCons>());
      expect((head as LispCons).car, equals(LispExpr.sym('A')));
    });
  });

  group('api', () {
    test('grammar exposes the six compiled rules', () {
      final grammar = mccarthyGrammar();
      expect(
        grammar.rules.map((r) => r.name),
        equals(['program', 'sexpr', 'atom', 'list', 'list_body', 'quoted']),
      );
    });

    test('token-stream factory parses', () {
      final tokens = tokenizeMcCarthy('(A)');
      final cst = createMcCarthyParserFromTokens(tokens).parse();
      expect(cst.ruleName, equals('program'));
    });

    test('cst entry point roots at program and wraps errors', () {
      expect(parseMcCarthyToCst('(A B)').ruleName, equals('program'));
      expect(
          () => parseMcCarthyToCst('(A'), throwsA(isA<McCarthyParseError>()));
      expect(
          () => parseMcCarthyToCst('nope'), throwsA(isA<McCarthyParseError>()));
    });

    test('nullable twin agrees with the throwing api', () {
      expect(tryParseMcCarthy('(A)'), isNotNull);
      expect(tryParseMcCarthy('(A'), isNull);
      expect(tryParseMcCarthy('car'), isNull);
    });

    test('error value semantics', () {
      const a = McCarthyParseError(message: 'm', line: 1, column: 2);
      const b = McCarthyParseError(message: 'm', line: 1, column: 2);
      expect(a, equals(b));
      expect(a.hashCode, equals(b.hashCode));
      expect(a.toString(), equals('parse error at 1:2: m'));
      expect(
        a,
        isNot(
            equals(const McCarthyParseError(message: 'n', line: 1, column: 2))),
      );
      expect(
        a,
        isNot(
            equals(const McCarthyParseError(message: 'm', line: 9, column: 2))),
      );
      expect(a, isNot(equals('parse error at 1:2: m')));
    });

    test('ast constructor laws', () {
      expect(LispExpr.list([]), equals(const LispNil()));
      expect(
        LispExpr.quote(LispExpr.sym('X')),
        equals(LispExpr.list([LispExpr.sym('QUOTE'), LispExpr.sym('X')])),
      );
      expect(
        LispExpr.list([const LispInt(1), const LispInt(2)]),
        equals(const LispCons(LispInt(1), LispCons(LispInt(2), LispNil()))),
      );
    });

    test('ast inequality and hashes', () {
      expect(const LispNil(), isNot(equals(LispExpr.sym('NIL'))));
      expect(LispExpr.sym('A'), isNot(equals(LispExpr.sym('B'))));
      expect(const LispInt(1), isNot(equals(const LispInt(2))));
      expect(
        const LispCons(LispInt(1), LispNil()),
        isNot(equals(const LispCons(LispInt(2), LispNil()))),
      );
      expect(
        const LispCons(LispInt(1), LispNil()),
        isNot(equals(const LispCons(LispInt(1), LispInt(2)))),
      );
      expect(LispExpr.sym('A'), isNot(equals(const LispInt(1))));
      expect(
        LispExpr.sym('A').hashCode,
        equals(LispExpr.sym('A').hashCode),
      );
      expect(
        const LispCons(LispInt(1), LispNil()).hashCode,
        equals(const LispCons(LispInt(1), LispNil()).hashCode),
      );
    });
  });

  group('extractor shapes', () {
    test('non-program root errors with fallback position', () {
      final err = extractProgramErr(ASTNode(ruleName: 'sexpr', children: []));
      expect(err.message, equals('expected \'program\' root, got "sexpr"'));
      expect(err.line, equals(1));
      expect(err.column, equals(1));
    });

    test('non-program root carries explicit positions', () {
      final err = extractProgramErr(ASTNode(
          ruleName: 'atom', children: [], startLine: 3, startColumn: 7));
      expect(err.line, equals(3));
      expect(err.column, equals(7));
    });

    test('non-sexpr program child errors', () {
      final err = extractProgramErr(ASTNode(
        ruleName: 'program',
        children: [ASTNode(ruleName: 'atom', children: [])],
      ));
      expect(err.message, equals('expected \'sexpr\', got "atom"'));
    });

    test('empty sexpr node errors', () {
      final err = extractProgramErr(ASTNode(
        ruleName: 'program',
        children: [ASTNode(ruleName: 'sexpr', children: [])],
      ));
      expect(err.message, equals('empty sexpr node'));
    });

    test('unexpected sexpr child errors', () {
      final err = extractProgramErr(ASTNode(
        ruleName: 'program',
        children: [
          ASTNode(ruleName: 'sexpr', children: [
            ASTNode(ruleName: 'bogus', children: []),
          ]),
        ],
      ));
      expect(err.message, equals('unexpected sexpr child: "bogus"'));
    });

    test('empty atom node errors', () {
      final err = extractProgramErr(ASTNode(
        ruleName: 'program',
        children: [
          ASTNode(ruleName: 'sexpr', children: [
            ASTNode(ruleName: 'atom', children: []),
          ]),
        ],
      ));
      expect(err.message, equals('empty atom node'));
    });

    test('unexpected atom token errors with token position', () {
      final err = extractProgramErr(ASTNode(
        ruleName: 'program',
        children: [
          ASTNode(ruleName: 'sexpr', children: [
            ASTNode(ruleName: 'atom', children: [
              const Token(type: 'DOT', value: '.', line: 5, column: 6),
            ]),
          ]),
        ],
      ));
      expect(
          err.message, equals('unexpected atom token: type="DOT" value="."'));
      expect(err.line, equals(5));
      expect(err.column, equals(6));
    });

    test('list with no body child is nil', () {
      final forms = extractProgram(ASTNode(
        ruleName: 'program',
        children: [
          ASTNode(ruleName: 'sexpr', children: [
            ASTNode(ruleName: 'list', children: [
              const Token(type: 'LPAREN', value: '(', line: 1, column: 1),
              const Token(type: 'RPAREN', value: ')', line: 1, column: 2),
            ]),
          ]),
        ],
      ));
      expect(forms, equals([const LispNil()]));
    });

    test('stray body tokens are ignored', () {
      final forms = extractProgram(ASTNode(
        ruleName: 'program',
        children: [
          ASTNode(ruleName: 'sexpr', children: [
            ASTNode(ruleName: 'list', children: [
              ASTNode(ruleName: 'list_body', children: [
                const Token(type: 'LPAREN', value: '(', line: 1, column: 1),
                ASTNode(ruleName: 'sexpr', children: [
                  ASTNode(ruleName: 'atom', children: [
                    const Token(type: 'SYMBOL', value: 'A', line: 1, column: 2),
                  ]),
                ]),
              ]),
            ]),
          ]),
        ],
      ));
      expect(
          forms,
          equals([
            LispExpr.list([LispExpr.sym('A')])
          ]));
    });

    test('quoted with no child errors', () {
      final err = extractProgramErr(ASTNode(
        ruleName: 'program',
        children: [
          ASTNode(ruleName: 'sexpr', children: [
            ASTNode(ruleName: 'quoted', children: []),
          ]),
        ],
      ));
      expect(err.message, equals("expected an expression after '"));
    });

    test('hand-built program extracts', () {
      final forms = extractProgram(ASTNode(
        ruleName: 'program',
        children: [
          ASTNode(ruleName: 'sexpr', children: [
            ASTNode(ruleName: 'atom', children: [
              const Token(type: 'SYMBOL', value: 'A', line: 2, column: 3),
            ]),
          ]),
        ],
      ));
      expect(forms, equals([LispExpr.sym('A')]));
    });

    test('ast depth breach errors', () {
      // 150 list wraps à two depth steps each = depth 300, past the
      // 256 cap the grammar-level prescan can never admit.
      ASTNode level(ASTNode inner) => ASTNode(
            ruleName: 'sexpr',
            children: [
              ASTNode(ruleName: 'list', children: [
                ASTNode(ruleName: 'list_body', children: [inner]),
              ]),
            ],
          );
      ASTNode deep = ASTNode(
        ruleName: 'sexpr',
        children: [
          ASTNode(ruleName: 'atom', children: [
            const Token(type: 'SYMBOL', value: 'A', line: 1, column: 1),
          ]),
        ],
      );
      for (var i = 0; i < 150; i++) {
        deep = level(deep);
      }
      final err =
          extractProgramErr(ASTNode(ruleName: 'program', children: [deep]));
      expect(err.message, contains('MAX_AST_DEPTH'));
    });
  });
}

/// Run [extractProgram] on a hand-built CST that must fail.
McCarthyParseError extractProgramErr(ASTNode root) {
  try {
    extractProgram(root);
  } on McCarthyParseError catch (e) {
    return e;
  }
  fail('expected McCarthyParseError');
}
