import 'package:coding_adventures_algol_parser/algol_parser.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

void main() {
  group('ALGOL parser configuration', () {
    test('loads the embedded algol60 grammar and exposes a parser', () {
      expect(loadAlgolParserGrammar().version, 1);
      expect(loadAlgolParserGrammar().rules.first.name, 'program');
      expect(createAlgolParser('begin end').parse().ruleName, 'program');
    });

    test('rejects unknown grammar versions', () {
      expect(
        () => parseAlgol('begin end', version: 'algol68'),
        throwsArgumentError,
      );
    });
  });

  group('ALGOL blocks and declarations', () {
    test('parses case-insensitive blocks and comments', () {
      final ast = parseAlgol('BEGIN COMMENT ignored; INTEGER x; x := 42 END');
      expect(ast.ruleName, 'program');
      expect(findNodes(ast, 'block'), hasLength(1));
      expect(findNodes(ast, 'type_decl'), hasLength(1));
      expect(findNodes(ast, 'assign_stmt'), hasLength(1));
    });

    test('parses scalar, own, and multidimensional array declarations', () {
      final ast = parseAlgol('''begin
integer x, y;
own real total;
integer array matrix[1:10, 1:20];
x := 1
end''');
      expect(findNodes(ast, 'type_decl'), hasLength(1));
      expect(findNodes(ast, 'own_decl'), hasLength(1));
      expect(findNodes(ast, 'array_decl'), hasLength(1));
      expect(findNodes(ast, 'bound_pair'), hasLength(2));
    });
  });

  group('ALGOL statements and expressions', () {
    test('parses arithmetic precedence and conditional expressions', () {
      final ast = parseAlgol('''begin
real x;
x := if 2 < 3 then 1 + 2 * 3 else 2 ^ 4
end''');
      expect(findNodes(ast, 'assign_stmt'), hasLength(1));
      expect(findNodes(ast, 'expression'), isNotEmpty);
      expect(collectTokens(ast, type: 'STAR'), hasLength(1));
      expect(collectTokens(ast, type: 'CARET'), hasLength(1));
    });

    test('parses publication-symbol boolean and relational operators', () {
      final ast = parseAlgol(
        'begin boolean ok; '
        'ok := \u00ac false \u2227 (1 \u2264 2) \u2228 true '
        'end',
      );
      expect(findNodes(ast, 'assign_stmt'), hasLength(1));
      expect(
        collectTokens(ast, type: 'KEYWORD')
            .map((token) => token.value)
            .where(const {'not', 'and', 'or'}.contains),
        ['not', 'and', 'or'],
      );
      expect(collectTokens(ast, type: 'LEQ').single.value, '<=');
    });

    test('parses for loops, goto, labels, and procedure calls', () {
      final ast = parseAlgol('''begin
integer i;
procedure tick(); begin end;
start: for i := 1 step 1 until 3 do tick();
goto start
end''');
      expect(findNodes(ast, 'procedure_decl'), hasLength(1));
      expect(findNodes(ast, 'for_stmt'), hasLength(1));
      expect(findNodes(ast, 'proc_stmt'), hasLength(1));
      expect(findNodes(ast, 'goto_stmt'), hasLength(1));
      expect(findNodes(ast, 'label'), isNotEmpty);
    });

    test('parses nested blocks and chained assignment', () {
      final ast = parseAlgol('begin integer x, y; begin x := y := 0 end end');
      expect(findNodes(ast, 'block'), hasLength(1));
      expect(findNodes(ast, 'compound_stmt'), hasLength(1));
      expect(findNodes(ast, 'left_part'), hasLength(2));
    });
  });

  group('ALGOL parser behavior', () {
    test('tracks the source span of a complete program', () {
      final ast = parseAlgol('''begin
  integer x;
  x := 1
end''');
      expect(ast.startLine, 1);
      expect(ast.startColumn, 1);
      expect(ast.endLine, 4);
      expect(ast.endColumn, 1);
    });

    test('rejects missing delimiters, empty input, and trailing source', () {
      expect(
        () => parseAlgol('begin integer x; x := 1'),
        throwsA(isA<GrammarParseError>()),
      );
      expect(() => parseAlgol(''), throwsA(isA<GrammarParseError>()));
      expect(
        () => parseAlgol('begin end trailing'),
        throwsA(isA<GrammarParseError>()),
      );
    });
  });
}
