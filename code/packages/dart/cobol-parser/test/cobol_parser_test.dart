/// Port of the reference `rust/cobol-parser` unit suite (all 19
/// tests) plus nullable-twin and error-shape tests. Fixtures reuse
/// the reference's card helpers: each code line is carded with a
/// 6-column sequence number plus the indicator column.
library;

import 'package:coding_adventures_cobol_parser/cobol_parser.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

/// A one-line card (6 sequence cols + 1 space indicator, then code).
String card(String code) => '000000 $code';

/// Assemble a program from code lines, carding each one.
String program(List<String> lines) => lines.map(card).join('\n');

ASTNode root(String src) => parseCobol(src);

bool hasRule(ASTNode node, String target) {
  if (node.ruleName == target) return true;
  return node.children.any((c) => c is ASTNode && hasRule(c, target));
}

int countRule(ASTNode node, String target) {
  var here = node.ruleName == target ? 1 : 0;
  for (final c in node.children) {
    if (c is ASTNode) here += countRule(c, target);
  }
  return here;
}

/// Whether any token in the tree has the given type name.
bool hasToken(ASTNode node, String typeName) {
  bool walk(Object current) {
    if (current is Token) return current.type == typeName;
    if (current is ASTNode) {
      return current.children.any(walk);
    }
    return false;
  }

  return walk(node);
}

ASTNode? findRule(ASTNode node, String target) {
  if (node.ruleName == target) return node;
  for (final c in node.children) {
    if (c is ASTNode) {
      final found = findRule(c, target);
      if (found != null) return found;
    }
  }
  return null;
}

void main() {
  group('reference suite', () {
    test('minimal program', () {
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. HELLO.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    STOP RUN.',
      ]));
      expect(ast.ruleName, equals('program'));
      expect(hasRule(ast, 'identification_division'), isTrue);
      expect(hasRule(ast, 'procedure_division'), isTrue);
      expect(hasRule(ast, 'stop_stmt'), isTrue);
      // No optional divisions were invented.
      expect(hasRule(ast, 'environment_division'), isFalse);
      expect(hasRule(ast, 'data_division'), isFalse);
    });

    test('working storage entries', () {
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. PAY.',
        'DATA DIVISION.',
        'WORKING-STORAGE SECTION.',
        '01  EMPLOYEE-RECORD.',
        '    02  EMP-NAME  PICTURE X(20).',
        '    02  EMP-RATE  PIC 9(3)V99.',
        '77  GROSS-PAY  PIC 9(6)V99 VALUE ZERO.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    STOP RUN.',
      ]));
      expect(hasRule(ast, 'working_storage_section'), isTrue);
      // Four data entries: the 01 record, two 02 items, the 77 item.
      expect(countRule(ast, 'data_entry'), equals(4));
      // Two PICTURE clauses carry PIC_STRINGs; the 01 group item
      // has none.
      expect(countRule(ast, 'picture_clause'), equals(3));
      expect(hasRule(ast, 'value_clause'), isTrue);
    });

    test('filler and value is', () {
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'DATA DIVISION.',
        'WORKING-STORAGE SECTION.',
        '01  FILLER  PIC X VALUE IS SPACE.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    STOP RUN.',
      ]));
      expect(countRule(ast, 'data_entry'), equals(1));
      expect(hasRule(ast, 'value_clause'), isTrue);
    });

    test('procedure paragraphs and verbs', () {
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN-PARAGRAPH.',
        '    MOVE ZERO TO GROSS-PAY.',
        '    MULTIPLY EMP-RATE BY EMP-HOURS GIVING GROSS-PAY.',
        '    DISPLAY EMP-NAME GROSS-PAY.',
        '    PERFORM SUB-ROUTINE.',
        '    GO TO MAIN-PARAGRAPH.',
        'SUB-ROUTINE.',
        '    STOP RUN.',
      ]));
      expect(countRule(ast, 'paragraph'), equals(2));
      expect(hasRule(ast, 'move_stmt'), isTrue);
      expect(hasRule(ast, 'multiply_stmt'), isTrue);
      expect(hasRule(ast, 'display_stmt'), isTrue);
      expect(hasRule(ast, 'perform_stmt'), isTrue);
      expect(hasRule(ast, 'goto_stmt'), isTrue);
    });

    test('reference modification operand shape', () {
      // `DISPLAY WS(2:3).` — the operand carries a
      // reference-modification suffix: a COLON token separates two
      // nested `operand` subnodes (start 2, len 3), so the whole
      // DISPLAY holds three `operand`s (outer + start + len) versus
      // one for a bare `DISPLAY WS.`.
      final refmod = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    DISPLAY WS(2:3).',
        '    STOP RUN.',
      ]));
      final display = findRule(refmod, 'display_stmt');
      expect(display, isNotNull);
      // Outer operand + the two inner index operands.
      expect(countRule(display!, 'operand'), equals(3));
      // The reference-modification COLON is present.
      expect(hasToken(display, 'COLON'), isTrue);

      // A bare NAME still parses with no COLON.
      final plain = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    DISPLAY WS.',
        '    STOP RUN.',
      ]));
      expect(hasToken(plain, 'COLON'), isFalse);
    });

    test('reference modification omitted length shape', () {
      // `WS(3:)` omits the length — one inner operand, still with a
      // COLON.
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    DISPLAY WS(3:).',
        '    STOP RUN.',
      ]));
      expect(hasToken(ast, 'COLON'), isTrue);
      // Outer operand + a single inner start operand (no length).
      expect(countRule(ast, 'operand'), equals(2));
    });

    test('perform until and times', () {
      // PERFORM … UNTIL <condition> carries a condition node;
      // PERFORM … TIMES carries an operand. Both are the optional
      // trailing clause.
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    PERFORM STEP UNTIL I GREATER 9.',
        '    PERFORM STEP 3 TIMES.',
        '    STOP RUN.',
        'STEP.',
        '    DISPLAY I.',
      ]));
      expect(countRule(ast, 'perform_stmt'), equals(2));
      // The UNTIL clause introduces a condition inside a
      // perform_stmt.
      expect(hasRule(ast, 'condition'), isTrue);
    });

    test('arithmetic rounded and on size error', () {
      // ADD/SUBTRACT/MULTIPLY/DIVIDE take the trailing ROUNDED and
      // ON SIZE ERROR clauses, sharing the `size_error` rule with
      // COMPUTE.
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    ADD 1 TO R ROUNDED.',
        '    DIVIDE 3 INTO X GIVING Y ROUNDED',
        '        ON SIZE ERROR DISPLAY "OVR".',
        '    STOP RUN.',
      ]));
      expect(hasRule(ast, 'add_stmt'), isTrue);
      expect(hasRule(ast, 'divide_stmt'), isTrue);
      // The DIVIDE's ON SIZE ERROR introduces a size_error node.
      expect(hasRule(ast, 'size_error'), isTrue);
    });

    test('perform varying', () {
      // PERFORM … VARYING id FROM start BY step UNTIL cond parses
      // to a perform_varying node carrying the induction variable
      // and condition.
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    PERFORM SHOW VARYING I FROM 1 BY 1 UNTIL I GREATER 3.',
        '    STOP RUN.',
        'SHOW.',
        '    DISPLAY I.',
      ]));
      expect(hasRule(ast, 'perform_varying'), isTrue);
      expect(hasRule(ast, 'condition'), isTrue);
    });

    test('if else statement', () {
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    IF GROSS-PAY IS GREATER THAN ZERO',
        '        DISPLAY GROSS-PAY',
        '    ELSE',
        '        DISPLAY ZERO.',
        '    STOP RUN.',
      ]));
      expect(hasRule(ast, 'if_stmt'), isTrue);
      expect(hasRule(ast, 'condition'), isTrue);
      // Two DISPLAYs: one in the then-branch, one in the
      // else-branch.
      expect(countRule(ast, 'display_stmt'), equals(2));
    });

    test('compute with operator precedence', () {
      // `A + B * C ** D` must nest so that ** binds tightest, then
      // *, then +.
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    COMPUTE RESULT = A + B * C ** D.',
        '    STOP RUN.',
      ]));
      expect(hasRule(ast, 'compute_stmt'), isTrue);
      // One additive expr at the top; the multiplicative and
      // exponent layers each appear once, proving the precedence
      // cascade was built.
      expect(countRule(ast, 'arith_expr'), equals(1));
      expect(countRule(ast, 'arith_term'), equals(2));
      expect(countRule(ast, 'arith_factor'), equals(3));
    });

    test('compute parentheses regroup precedence', () {
      // Parentheses force the addition to evaluate first: a nested
      // arith_expr.
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    COMPUTE X = (A + B) * C.',
        '    STOP RUN.',
      ]));
      expect(hasRule(ast, 'compute_stmt'), isTrue);
      // The parenthesised `A + B` is a second, nested arith_expr.
      expect(countRule(ast, 'arith_expr'), equals(2));
    });

    test('compute rounded and on size error', () {
      // ROUNDED and the ON SIZE ERROR clause both parse; the clause
      // carries a statement to run on overflow.
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        // Split across cards so no line exceeds the 72-column code
        // area (a COBOL statement flows freely from one card to the
        // next).
        '    COMPUTE TOTAL ROUNDED = PRICE * QTY',
        '        ON SIZE ERROR DISPLAY "OVR".',
        '    STOP RUN.',
      ]));
      expect(hasRule(ast, 'compute_stmt'), isTrue);
      expect(hasRule(ast, 'size_error'), isTrue);
      // The overflow handler is an ordinary statement (a DISPLAY
      // here).
      expect(hasRule(ast, 'display_stmt'), isTrue);
    });

    test('compute negative literal vs subtraction', () {
      // `A - 3` (spaced) is subtraction; `-3` (unspaced) would be a
      // negative literal. Here we exercise the binary minus.
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    COMPUTE D = A - 3.',
        '    STOP RUN.',
      ]));
      expect(hasRule(ast, 'compute_stmt'), isTrue);
      expect(countRule(ast, 'arith_term'), equals(2));
    });

    test('environment configuration section', () {
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'ENVIRONMENT DIVISION.',
        'CONFIGURATION SECTION.',
        'SOURCE-COMPUTER. UNIVAC-II.',
        'OBJECT-COMPUTER. UNIVAC-II.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    STOP RUN.',
      ]));
      expect(hasRule(ast, 'environment_division'), isTrue);
      expect(hasRule(ast, 'configuration_section'), isTrue);
      expect(countRule(ast, 'config_paragraph'), equals(2));
    });

    test('full four division program', () {
      const src = '000100 IDENTIFICATION DIVISION.\n'
          '000200 PROGRAM-ID. PAYROLL.\n'
          '000300 AUTHOR. GRACE HOPPER.\n'
          '000400 ENVIRONMENT DIVISION.\n'
          '000500 CONFIGURATION SECTION.\n'
          '000600 SOURCE-COMPUTER. UNIVAC-II.\n'
          '000700 OBJECT-COMPUTER. UNIVAC-II.\n'
          '000800 DATA DIVISION.\n'
          '000900 WORKING-STORAGE SECTION.\n'
          '001000 01  EMP-NAME     PICTURE X(20).\n'
          '001100 77  GROSS-PAY    PIC 9(6)V99 VALUE ZERO.\n'
          '001200 PROCEDURE DIVISION.\n'
          '001300 MAIN-PARAGRAPH.\n'
          '001400     MOVE ZERO TO GROSS-PAY.\n'
          '001500     DISPLAY EMP-NAME GROSS-PAY.\n'
          '001600     STOP RUN.';
      final ast = root(src);
      expect(ast.ruleName, equals('program'));
      // All four divisions present.
      for (final rule in [
        'identification_division',
        'environment_division',
        'data_division',
        'procedure_division',
      ]) {
        expect(hasRule(ast, rule), isTrue, reason: 'missing $rule');
      }
      // The commentary AUTHOR paragraph parsed.
      expect(hasRule(ast, 'id_paragraph'), isTrue);
    });

    test('missing procedure division is error', () {
      // A program with no PROCEDURE DIVISION is incomplete.
      final src = program(['IDENTIFICATION DIVISION.', 'PROGRAM-ID. P.']);
      expect(() => parseCobol(src), throwsA(isA<GrammarParseError>()));
      expect(tryParseCobol(src), isNull);
    });

    test('deeply nested if is a clean error not a stack overflow', () {
      // 4096 nested IFs — far past the cap — must come back as a
      // recoverable error whose message explains the depth refusal.
      final lines = [
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        for (var i = 0; i < 4096; i++) '    IF GROSS-PAY IS GREATER THAN ZERO',
        '        DISPLAY ZERO.',
        '    STOP RUN.',
      ];
      try {
        parseCobol(program(lines));
        fail('must not parse');
      } on GrammarParseError catch (e) {
        final lower = e.message.toLowerCase();
        expect(
          lower.contains('nest') || lower.contains('depth'),
          isTrue,
          reason: 'depth refusal should be self-explanatory, '
              'got: ${e.message}',
        );
      }
      expect(tryParseCobol(program(lines)), isNull);
    });

    test('lexical error is reported', () {
      // A stray `@` in the code area surfaces as a failure.
      final src = program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    MOVE @ TO X.',
      ]);
      expect(() => parseCobol(src), throwsA(isA<LexerError>()));
      expect(tryParseCobol(src), isNull);
    });
  });

  group('twins and errors', () {
    test('empty and blank inputs fail closed', () {
      for (final src in ['', '\n']) {
        expect(() => parseCobol(src), throwsA(isA<GrammarParseError>()));
        expect(tryParseCobol(src), isNull);
      }
    });

    test('factory returns a wired parser', () {
      final parser = createCobolParser(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    STOP RUN.',
      ]));
      expect(parser.parse().ruleName, equals('program'));
    });

    test('shallow if nests parse', () {
      final ast = root(program([
        'IDENTIFICATION DIVISION.',
        'PROGRAM-ID. P.',
        'PROCEDURE DIVISION.',
        'MAIN.',
        '    IF A IF B DISPLAY C.',
        '    STOP RUN.',
      ]));
      expect(hasRule(ast, 'if_stmt'), isTrue);
    });
  });
}
