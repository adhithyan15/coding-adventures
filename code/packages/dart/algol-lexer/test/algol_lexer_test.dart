import 'package:coding_adventures_algol_lexer/algol_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

void main() {
  List<Token> meaningful(String source) => tokenizeAlgol(
        source,
      ).where((token) => token.type != 'EOF').toList(growable: false);

  List<String> types(String source) =>
      meaningful(source).map((token) => token.type).toList(growable: false);

  List<String> values(String source) =>
      meaningful(source).map((token) => token.value).toList(growable: false);

  group('ALGOL grammar selection', () {
    test('exposes the algol60 grammar and configured lexer', () {
      expect(defaultAlgolVersion, 'algol60');
      expect(supportedAlgolVersions, {'algol60'});
      expect(resolveAlgolVersion(), 'algol60');
      expect(loadAlgolTokenGrammar().version, 1);
      expect(createAlgolLexer('begin end').version, 'algol60');
    });

    test('rejects unknown grammar versions', () {
      expect(
        () => tokenizeAlgol('begin end', version: 'algol68'),
        throwsArgumentError,
      );
    });
  });

  group('ALGOL keywords and comments', () {
    test('normalizes keyword spelling without changing identifiers', () {
      final tokens = meaningful(
        'BEGIN Begin begin beginning INTEGER integer Integers',
      );
      expect(tokens.map((token) => token.type), [
        'KEYWORD',
        'KEYWORD',
        'KEYWORD',
        'NAME',
        'KEYWORD',
        'KEYWORD',
        'NAME',
      ]);
      expect(tokens.map((token) => token.value), [
        'begin',
        'begin',
        'begin',
        'beginning',
        'integer',
        'integer',
        'Integers',
      ]);
    });

    test('skips lowercase and uppercase comments outside strings', () {
      expect(values('x := 1; comment lower; y := 2'), [
        'x',
        ':=',
        '1',
        ';',
        'y',
        ':=',
        '2',
      ]);
      expect(values('BEGIN COMMENT upper case; END'), ['begin', 'end']);
      expect(values(r'"COMMENT preserved;"'), ['COMMENT preserved;']);
      expect(types('commentary := 1'), ['NAME', 'ASSIGN', 'INTEGER_LIT']);
    });
  });

  group('ALGOL operators and literals', () {
    test('recognizes assignment, equality, power, and punctuation', () {
      expect(types('x := y = 2 ** 3 ^ 4; a[1:10], b'), [
        'NAME',
        'ASSIGN',
        'NAME',
        'EQ',
        'INTEGER_LIT',
        'POWER',
        'INTEGER_LIT',
        'CARET',
        'INTEGER_LIT',
        'SEMICOLON',
        'NAME',
        'LBRACKET',
        'INTEGER_LIT',
        'COLON',
        'INTEGER_LIT',
        'RBRACKET',
        'COMMA',
        'NAME',
      ]);
    });

    test('normalizes publication symbols to ASCII and keyword values', () {
      const source = 'a \u2264 b \u2265 c \u2260 d \u2191 2 \u00d7 3 \u00f7 4 '
          '\u00ac p \u2227 q \u2228 r \u2283 s \u2261 t';
      expect(values(source), [
        'a',
        '<=',
        'b',
        '>=',
        'c',
        '!=',
        'd',
        '^',
        '2',
        '*',
        '3',
        '/',
        '4',
        'not',
        'p',
        'and',
        'q',
        'or',
        'r',
        'impl',
        's',
        'eqv',
        't',
      ]);
      expect(
        meaningful(source)
            .where(
              (token) => const {
                'not',
                'and',
                'or',
                'impl',
                'eqv',
              }.contains(token.value),
            )
            .map((token) => token.type),
        everyElement('KEYWORD'),
      );
    });

    test('recognizes integers, reals, exponents, and both string quotes', () {
      final tokens = meaningful('42 3.14 1.5E-3 100e2 \'hello\' "world"');
      expect(tokens.map((token) => token.type), [
        'INTEGER_LIT',
        'REAL_LIT',
        'REAL_LIT',
        'REAL_LIT',
        'STRING_LIT',
        'STRING_LIT',
      ]);
      expect(tokens.map((token) => token.value), [
        '42',
        '3.14',
        '1.5E-3',
        '100e2',
        'hello',
        'world',
      ]);
    });
  });

  group('ALGOL programs and errors', () {
    test('tokenizes a complete program with positions and EOF', () {
      final tokens = tokenizeAlgol('''begin
  integer x;
  x := 42
end''');
      expect(tokens.first.value, 'begin');
      expect(tokens.first.line, 1);
      expect(tokens.first.column, 1);
      final assignment = tokens.firstWhere((token) => token.type == 'ASSIGN');
      expect(assignment.line, 3);
      expect(assignment.column, 5);
      expect(tokens.last.type, 'EOF');
      expect(tokens.last.line, 4);
    });

    test('returns EOF for empty input and rejects invalid characters', () {
      expect(tokenizeAlgol('').map((token) => token.type), ['EOF']);
      expect(() => tokenizeAlgol(r'x _ y'), throwsA(isA<LexerError>()));
    });
  });
}
