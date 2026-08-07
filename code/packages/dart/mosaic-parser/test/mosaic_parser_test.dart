import 'package:coding_adventures_mosaic_parser/mosaic_parser.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

void main() {
  group('Mosaic declarations', () {
    test('parses a minimal component', () {
      final ast = parseMosaic('component Empty { Box { } }');
      expect(ast.ruleName, 'file');
      expect(findNodes(ast, 'component_decl'), hasLength(1));
      expect(findNodes(ast, 'node_element'), hasLength(1));
      expect(collectTokens(ast, type: 'NAME').map((token) => token.value), [
        'Empty',
        'Box',
      ]);
    });

    test('parses imports with and without aliases', () {
      final ast = parseMosaic('''
import Button from "./button.mosaic";
import Card as InfoCard from "./card.mosaic";
component Demo { Box { } }
''');
      expect(findNodes(ast, 'import_decl'), hasLength(2));
      expect(collectTokens(ast, type: 'STRING').map((token) => token.value), [
        './button.mosaic',
        './card.mosaic',
      ]);
    });

    test('parses primitive, component, and list slot types', () {
      final ast = parseMosaic('''
component Demo {
  slot title: text;
  slot action: Button;
  slot items: list<text>;
  slot count: number = 0;
  Box { }
}
''');
      expect(findNodes(ast, 'slot_decl'), hasLength(4));
      expect(findNodes(ast, 'list_type'), hasLength(1));
      expect(findNodes(ast, 'default_value'), hasLength(1));
    });
  });

  group('Mosaic node content', () {
    test('parses nested nodes, properties, and slot references', () {
      final ast = parseMosaic('''
component Card {
  slot title: text;
  Column {
    Row { Text { content: @title; style: heading.large; } }
    @title;
  }
}
''');
      expect(findNodes(ast, 'node_element'), hasLength(3));
      expect(findNodes(ast, 'property_assignment'), hasLength(2));
      expect(findNodes(ast, 'slot_ref'), hasLength(1));
      expect(findNodes(ast, 'slot_reference'), hasLength(1));
      expect(findNodes(ast, 'enum_value'), hasLength(1));
    });

    test('parses when and each blocks', () {
      final ast = parseMosaic('''
component ListView {
  slot visible: bool;
  slot items: list<text>;
  Column {
    when @visible { Text { content: "shown"; } }
    each @items as item { Text { content: @item; } }
  }
}
''');
      expect(findNodes(ast, 'when_block'), hasLength(1));
      expect(findNodes(ast, 'each_block'), hasLength(1));
      expect(findNodes(ast, 'slot_ref'), hasLength(3));
    });

    test('parses all literal property forms', () {
      final ast = parseMosaic('''
component Literals {
  Box {
    title: "hello";
    count: 3;
    padding: 16dp;
    background: #2563eb;
    align: center;
  }
}
''');
      expect(findNodes(ast, 'property_assignment'), hasLength(5));
      expect(collectTokens(ast, type: 'STRING'), hasLength(1));
      expect(collectTokens(ast, type: 'NUMBER'), hasLength(1));
      expect(collectTokens(ast, type: 'DIMENSION'), hasLength(1));
      expect(collectTokens(ast, type: 'COLOR_HEX'), hasLength(1));
    });
  });

  group('Mosaic parser behavior', () {
    test('tracks source spans across lines', () {
      final ast = parseMosaic('''component Demo {
  Box { }
}''');
      expect(ast.startLine, 1);
      expect(ast.startColumn, 1);
      expect(ast.endLine, 3);
      expect(ast.endColumn, 1);
    });

    test('exposes a configured parser factory', () {
      final parser = createMosaicParser('component Demo { Box { } }');
      expect(parser.parse().ruleName, 'file');
    });

    test('rejects incomplete and trailing source', () {
      expect(
        () => parseMosaic('component Broken {'),
        throwsA(isA<GrammarParseError>()),
      );
      expect(
        () => parseMosaic('component Demo { Box { } } trailing'),
        throwsA(isA<GrammarParseError>()),
      );
    });
  });
}
