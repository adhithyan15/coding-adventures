/// Port of the reference `rust/toml-parser` unit suite (26 tests).
library;

import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_toml_parser/toml_parser.dart';
import 'package:test/test.dart';

/// All TOML documents parse to a root node with rule name "document",
/// since that is the start symbol of the grammar.
void assertDocumentRoot(ASTNode ast) {
  expect(
    ast.ruleName,
    equals('document'),
    reason: "Expected root rule 'document', got '${ast.ruleName}'",
  );
}

/// Recursively search the AST for a node with the given rule name.
/// Returns true if found anywhere in the tree.
bool findRule(ASTNode node, String targetRule) {
  if (node.ruleName == targetRule) {
    return true;
  }
  for (final child in node.children) {
    if (child is ASTNode && findRule(child, targetRule)) {
      return true;
    }
  }
  return false;
}

/// Count how many times a rule appears in the AST (recursively).
int countRule(ASTNode node, String targetRule) {
  var count = node.ruleName == targetRule ? 1 : 0;
  for (final child in node.children) {
    if (child is ASTNode) {
      count += countRule(child, targetRule);
    }
  }
  return count;
}

String nestedArraySource(int n) => 'a = ${'[' * n}1${']' * n}';

String nestedInlineTableSource(int n) {
  final buf = StringBuffer('a = ');
  for (var i = 0; i < n; i++) {
    if (i > 0) {
      buf.write('b = ');
    }
    buf.write('{ ');
  }
  buf.write('c = 1');
  for (var i = 0; i < n; i++) {
    buf.write(' }');
  }
  return buf.toString();
}

void main() {
  group('toml-parser', () {
    test('simple keyval parses', () {
      final ast = parseToml('title = "TOML Example"');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'keyval'), isTrue, reason: "Expected 'keyval'");
      expect(findRule(ast, 'key'), isTrue, reason: "Expected 'key'");
      expect(findRule(ast, 'value'), isTrue, reason: "Expected 'value'");
    });

    test(
        'fallible api distinguishes lexical and syntax errors '
        'without source echo', () {
      const lexicalSource = 'secret = "unterminated';
      expect(tryParseToml(lexicalSource), isNull);
      expect(tryCreateTomlParser(lexicalSource), isNull);
      try {
        parseToml(lexicalSource);
        fail('expected TomlParseError.lexical');
      } on TomlParseError catch (e) {
        expect(e, same(TomlParseError.lexical));
        expect(e.toString(), equals('TOML tokenization failed'));
        expect(e.toString(), isNot(contains('secret')));
      }

      const syntaxSource = 'secret =';
      expect(tryParseToml(syntaxSource), isNull);
      expect(tryCreateTomlParser(syntaxSource), isNotNull);
      try {
        parseToml(syntaxSource);
        fail('expected TomlParseError.syntax');
      } on TomlParseError catch (e) {
        expect(e, same(TomlParseError.syntax));
        expect(e.toString(), equals('TOML parsing failed'));
        expect(e.toString(), isNot(contains('secret')));
      }
    });

    test(
        'fallible factory and parser preserve valid ast shape '
        'and depth cap', () {
      final ast = tryCreateTomlParser('[server]\nport = 8080')!.parse();
      assertDocumentRoot(ast);
      expect(tryParseToml(nestedArraySource(5000)), isNull);
    });

    test('integer value parses', () {
      final ast = parseToml('port = 8080');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'keyval'), isTrue, reason: "Expected 'keyval'");
    });

    test('boolean value parses', () {
      final ast = parseToml('enabled = true');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'keyval'), isTrue, reason: "Expected 'keyval'");
    });

    test('float value parses', () {
      final ast = parseToml('pi = 3.14');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'keyval'), isTrue, reason: "Expected 'keyval'");
    });

    test('table header parses', () {
      final ast = parseToml('[server]\nhost = "localhost"');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'table_header'), isTrue,
          reason: "Expected 'table_header'");
      expect(findRule(ast, 'keyval'), isTrue, reason: "Expected 'keyval'");
    });

    test('array table header parses', () {
      final ast = parseToml('[[products]]\nname = "Hammer"');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'array_table_header'), isTrue,
          reason: "Expected 'array_table_header'");
    });

    test('dotted key parses', () {
      final ast = parseToml('a.b.c = 1');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'key'), isTrue, reason: "Expected 'key'");
      expect(findRule(ast, 'simple_key'), isTrue,
          reason: "Expected 'simple_key'");
    });

    test('inline table parses', () {
      final ast = parseToml('point = { x = 1, y = 2 }');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'inline_table'), isTrue,
          reason: "Expected 'inline_table'");
    });

    test('array value parses', () {
      final ast = parseToml('colors = ["red", "green", "blue"]');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'array'), isTrue, reason: "Expected 'array'");
    });

    test('multiline array parses', () {
      final ast = parseToml('colors = [\n  "red",\n  "green",\n  "blue",\n]');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'array'), isTrue, reason: "Expected 'array'");
    });

    test('multiple keyvals parse', () {
      final ast = parseToml('a = 1\nb = 2\nc = 3');
      assertDocumentRoot(ast);
      expect(countRule(ast, 'keyval'), equals(3));
    });

    test('datetime value parses', () {
      final ast = parseToml('dob = 1979-05-27T07:32:00Z');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'value'), isTrue, reason: "Expected 'value'");
    });

    test('empty document parses', () {
      assertDocumentRoot(parseToml(''));
    });

    test('comment only document parses', () {
      assertDocumentRoot(parseToml('# this is a comment\n# another comment'));
    });

    test('nested table header parses', () {
      final ast = parseToml('[a.b.c]\nkey = "value"');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'table_header'), isTrue,
          reason: "Expected 'table_header'");
    });

    test('literal string value parses', () {
      final ast = parseToml(r"path = 'C:\Users\Alice'");
      assertDocumentRoot(ast);
      expect(findRule(ast, 'value'), isTrue, reason: "Expected 'value'");
    });

    test('create parser factory works', () {
      final ast = createTomlParser('key = 42').parse();
      expect(ast.ruleName, equals('document'));
    });

    test('full document parses', () {
      const source = '# Configuration file\n'
          'title = "My App"\n'
          '\n'
          '[server]\n'
          'host = "localhost"\n'
          'port = 8080\n'
          'enabled = true\n'
          '\n'
          '[database]\n'
          'connection = "postgresql://localhost/mydb"\n'
          'pool_size = 5\n'
          '\n'
          '[[users]]\n'
          'name = "Alice"\n'
          'role = "admin"\n'
          '\n'
          '[[users]]\n'
          'name = "Bob"\n'
          'role = "user"\n';
      final ast = parseToml(source);
      assertDocumentRoot(ast);
      expect(findRule(ast, 'table_header'), isTrue,
          reason: 'Expected table headers');
      expect(findRule(ast, 'array_table_header'), isTrue,
          reason: 'Expected array-of-tables headers');
      expect(findRule(ast, 'keyval'), isTrue,
          reason: 'Expected key-value pairs');
      expect(countRule(ast, 'table_header'), equals(2));
      expect(countRule(ast, 'array_table_header'), equals(2));
    });

    test('inline table with multiple pairs parses', () {
      final ast =
          parseToml('person = { name = "Alice", age = 30, active = true }');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'inline_table'), isTrue,
          reason: "Expected 'inline_table'");
    });

    test('integer array parses', () {
      final ast = parseToml('ports = [8001, 8001, 8002]');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'array'), isTrue, reason: "Expected 'array'");
      expect(findRule(ast, 'array_values'), isTrue,
          reason: "Expected 'array_values'");
    });

    test('quoted table header parses', () {
      final ast = parseToml('["quoted key"]\nvalue = 1');
      assertDocumentRoot(ast);
      expect(findRule(ast, 'table_header'), isTrue,
          reason: "Expected 'table_header'");
    });

    test('deeply nested input returns error not overflow for every shape', () {
      // No thread-stack tuning exists in Dart, so both depth tests run
      // in-process: the guard must fail the parse with a clean error —
      // never a StackOverflowError — which proves it trips first.
      for (final src in [
        nestedArraySource(5000),
        nestedInlineTableSource(5000),
      ]) {
        expect(tryParseToml(src), isNull,
            reason: 'deeply-nested input must fail, not parse or crash');
        expect(() => parseToml(src), throwsA(same(TomlParseError.syntax)));
      }
    });

    test('cap trips before overflow on default stack for every shape', () {
      for (final src in [
        nestedArraySource(5000),
        nestedInlineTableSource(5000),
      ]) {
        expect(createTomlParser(src).parse, throwsA(isA<GrammarParseError>()),
            reason: 'deeply-nested input must error, not crash');
      }
    });

    test('reasonable nesting stays under the cap for every shape', () {
      expect(tryParseToml(nestedArraySource(10)), isNotNull);
      expect(tryParseToml(nestedInlineTableSource(10)), isNotNull);
    });
  });
}
