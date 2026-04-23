import 'package:coding_adventures_json_parser/json_parser.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

void main() {
  group('parseJson', () {
    test('parses primitive values', () {
      expect(parseJson('"hello"').ruleName, 'value');
      expect(collectTokens(parseJson('"hello"'), type: 'STRING'), hasLength(1));
      expect(collectTokens(parseJson('42'), type: 'NUMBER'), hasLength(1));
      expect(collectTokens(parseJson('true'), type: 'TRUE'), hasLength(1));
      expect(collectTokens(parseJson('false'), type: 'FALSE'), hasLength(1));
      expect(collectTokens(parseJson('null'), type: 'NULL'), hasLength(1));
    });

    test('parses objects and arrays', () {
      final objectAst = parseJson('{"name": "Alice", "age": 30}');
      expect(findNodes(objectAst, 'object'), hasLength(1));
      expect(findNodes(objectAst, 'pair'), hasLength(2));

      final arrayAst = parseJson('[1, 2, 3]');
      expect(findNodes(arrayAst, 'array'), hasLength(1));
      expect(collectTokens(arrayAst, type: 'NUMBER'), hasLength(3));
    });

    test('parses nested structures', () {
      final ast = parseJson(
        '{"users": [{"name": "Alice"}, {"name": "Bob", "active": true}]}',
      );

      expect(ast.ruleName, 'value');
      expect(findNodes(ast, 'object'), hasLength(3));
      expect(findNodes(ast, 'array'), hasLength(1));
      expect(findNodes(ast, 'pair'), hasLength(4));
    });

    test('tracks source positions across pretty-printed JSON', () {
      final ast = parseJson('''
{
  "name": "Alice",
  "scores": [1, 2]
}
''');

      expect(ast.startLine, 1);
      expect(ast.startColumn, 1);
      expect(ast.endLine, 4);
      expect(ast.endColumn, 1);
    });

    test('rejects invalid JSON', () {
      expect(() => parseJson(''), throwsA(isA<GrammarParseError>()));
      expect(() => parseJson('{"a": 1,}'), throwsA(isA<GrammarParseError>()));
      expect(() => parseJson('[1, 2,]'), throwsA(isA<GrammarParseError>()));
    });
  });
}
