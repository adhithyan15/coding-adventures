import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

void main() {
  Token tok(String type, String value, [int line = 1, int column = 1]) {
    return Token(type: type, value: value, line: line, column: column);
  }

  Token eof([int line = 1, int column = 1]) {
    return Token(type: 'EOF', value: '', line: line, column: column);
  }

  Token nl([int line = 1, int column = 1]) {
    return Token(type: 'NEWLINE', value: '\n', line: line, column: column);
  }

  Program parseTokens(List<Token> tokens) => Parser(tokens).parse();

  group('parser', () {
    test('parses literals and names', () {
      expect(
        parseTokens([tok('NUMBER', '42'), eof()]),
        Program([const NumberLiteral(42)]),
      );
      expect(
        parseTokens([tok('STRING', 'hello'), eof()]),
        Program([const StringLiteral('hello')]),
      );
      expect(
        parseTokens([tok('NAME', 'x'), eof()]),
        Program([const Name('x')]),
      );
    });

    test('parses binary operators with precedence', () {
      final program = parseTokens([
        tok('NUMBER', '1'),
        tok('PLUS', '+'),
        tok('NUMBER', '2'),
        tok('STAR', '*'),
        tok('NUMBER', '3'),
        eof(),
      ]);

      expect(
        program,
        Program([
          const BinaryOp(
            left: NumberLiteral(1),
            op: '+',
            right: BinaryOp(
              left: NumberLiteral(2),
              op: '*',
              right: NumberLiteral(3),
            ),
          ),
        ]),
      );
    });

    test('parses parentheses and assignments', () {
      final program = parseTokens([
        tok('NAME', 'x'),
        tok('EQUALS', '='),
        tok('LPAREN', '('),
        tok('NUMBER', '1'),
        tok('PLUS', '+'),
        tok('NUMBER', '2'),
        tok('RPAREN', ')'),
        tok('STAR', '*'),
        tok('NUMBER', '3'),
        nl(),
        eof(),
      ]);

      expect(
        program,
        Program([
          const Assignment(
            target: Name('x'),
            value: BinaryOp(
              left: BinaryOp(
                left: NumberLiteral(1),
                op: '+',
                right: NumberLiteral(2),
              ),
              op: '*',
              right: NumberLiteral(3),
            ),
          ),
        ]),
      );
    });

    test('parses multiple statements and skips blank lines', () {
      final program = parseTokens([
        tok('NAME', 'x'),
        tok('EQUALS', '='),
        tok('NUMBER', '1'),
        nl(),
        nl(),
        tok('NAME', 'x'),
        tok('PLUS', '+'),
        tok('NUMBER', '2'),
        nl(),
        eof(),
      ]);

      expect(program.statements, hasLength(2));
      expect(
        program.statements.first,
        const Assignment(target: Name('x'), value: NumberLiteral(1)),
      );
      expect(
        program.statements.last,
        const BinaryOp(left: Name('x'), op: '+', right: NumberLiteral(2)),
      );
    });

    test('parses empty programs', () {
      expect(parseTokens([eof()]), const Program([]));
      expect(parseTokens([nl(), nl(), eof()]), const Program([]));
    });

    test('throws parse errors with location', () {
      expect(
        () => parseTokens([
          tok('LPAREN', '(', 3, 7),
          tok('NUMBER', '1', 3, 8),
          eof(3, 9),
        ]),
        throwsA(isA<ParseError>()),
      );
      try {
        parseTokens([tok('PLUS', '+', 5, 10), eof(5, 11)]);
        fail('expected parse error');
      } on ParseError catch (error) {
        expect(error.token.line, 5);
        expect(error.token.column, 10);
      }
    });
  });
}
