/// Unit tests for `coding_adventures_xml_lexer`: all 38 reference
/// tests, plus the corners the reference leaves uncovered — lexer
/// errors, unclosed constructs at EOF, NEWLINE inside skip-disabled
/// groups, positions, and text that looks like markup but is not.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_xml_lexer/xml_lexer.dart';
import 'package:test/test.dart';

/// (type, value) pairs, excluding the final EOF token. Types are the
/// grammar names verbatim, like the reference's `effective_type_name`.
List<(String, String)> tokenPairs(List<Token> tokens) => [
      for (final t in tokens)
        if (t.type != 'EOF') (t.type, t.value)
    ];

/// Type names only, excluding EOF.
List<String> tokenTypes(List<Token> tokens) => [
      for (final t in tokens)
        if (t.type != 'EOF') t.type
    ];

List<String> textsOf(List<Token> tokens, String type) => [
      for (final (t, v) in tokenPairs(tokens))
        if (t == type) v
    ];

void main() {
  group('basic tags', () {
    test('simple element', () {
      expect(tokenPairs(tokenizeXml('<p>text</p>')), [
        ('OPEN_TAG_START', '<'),
        ('TAG_NAME', 'p'),
        ('TAG_CLOSE', '>'),
        ('TEXT', 'text'),
        ('CLOSE_TAG_START', '</'),
        ('TAG_NAME', 'p'),
        ('TAG_CLOSE', '>'),
      ]);
    });

    test('element with namespace', () {
      final tokens = tokenizeXml('<ns:tag>content</ns:tag>');
      expect(tokenTypes(tokens), [
        'OPEN_TAG_START',
        'TAG_NAME',
        'TAG_CLOSE',
        'TEXT',
        'CLOSE_TAG_START',
        'TAG_NAME',
        'TAG_CLOSE',
      ]);
      expect(tokenPairs(tokens)[1], ('TAG_NAME', 'ns:tag'));
    });

    test('explicitly empty element', () {
      expect(tokenPairs(tokenizeXml('<div></div>')), [
        ('OPEN_TAG_START', '<'),
        ('TAG_NAME', 'div'),
        ('TAG_CLOSE', '>'),
        ('CLOSE_TAG_START', '</'),
        ('TAG_NAME', 'div'),
        ('TAG_CLOSE', '>'),
      ]);
    });

    test('self-closing tag', () {
      expect(tokenPairs(tokenizeXml('<br/>')), [
        ('OPEN_TAG_START', '<'),
        ('TAG_NAME', 'br'),
        ('SELF_CLOSE', '/>'),
      ]);
    });

    test('self-closing tag with space', () {
      expect(tokenPairs(tokenizeXml('<br />')), [
        ('OPEN_TAG_START', '<'),
        ('TAG_NAME', 'br'),
        ('SELF_CLOSE', '/>'),
      ]);
    });
  });

  group('attributes', () {
    test('double-quoted attribute', () {
      expect(tokenPairs(tokenizeXml('<div class="main">')), [
        ('OPEN_TAG_START', '<'),
        ('TAG_NAME', 'div'),
        ('TAG_NAME', 'class'),
        ('ATTR_EQUALS', '='),
        ('ATTR_VALUE', '"main"'),
        ('TAG_CLOSE', '>'),
      ]);
    });

    test('single-quoted attribute', () {
      expect(tokenPairs(tokenizeXml("<div class='main'>")), [
        ('OPEN_TAG_START', '<'),
        ('TAG_NAME', 'div'),
        ('TAG_NAME', 'class'),
        ('ATTR_EQUALS', '='),
        ('ATTR_VALUE', "'main'"),
        ('TAG_CLOSE', '>'),
      ]);
    });

    test('multiple attributes', () {
      final pairs = tokenPairs(tokenizeXml('<a href="url" target="_blank">'));
      expect([
        for (final (t, v) in pairs)
          if (t == 'TAG_NAME') v
      ], [
        'a',
        'href',
        'target'
      ]);
      expect([
        for (final (t, v) in pairs)
          if (t == 'ATTR_VALUE') v
      ], [
        '"url"',
        '"_blank"'
      ]);
    });

    test('attribute on self-closing tag', () {
      final types = tokenTypes(tokenizeXml('<img src="photo.jpg"/>'));
      expect(types, contains('SELF_CLOSE'));
      expect(types, contains('ATTR_VALUE'));
    });

    test('quoted value wins over tag structure', () {
      final tokens = tokenizeXml('<a b="<">');
      expect(tokenPairs(tokens), contains(('ATTR_VALUE', '"<"')));
      // The `<` inside quotes never opens a nested tag: exactly one
      // opener (for `<a` itself).
      expect(
          tokenTypes(tokens).where((t) => t == 'OPEN_TAG_START'), hasLength(1));
    });
  });

  group('comments', () {
    test('simple comment', () {
      expect(tokenPairs(tokenizeXml('<!-- hello -->')), [
        ('COMMENT_START', '<!--'),
        ('COMMENT_TEXT', ' hello '),
        ('COMMENT_END', '-->'),
      ]);
    });

    test('comment preserves whitespace', () {
      expect(
          textsOf(tokenizeXml('<!--  spaces  and\ttabs  -->'), 'COMMENT_TEXT'),
          ['  spaces  and\ttabs  ']);
    });

    test('comment with dashes splits and reassembles', () {
      final texts = textsOf(tokenizeXml('<!-- a-b-c -->'), 'COMMENT_TEXT');
      expect(texts, [' a', '-', 'b', '-', 'c ']);
      expect(texts.join(), ' a-b-c ');
    });

    test('comment between elements', () {
      final types = tokenTypes(tokenizeXml('<a/><!-- mid --><b/>'));
      expect(types, contains('COMMENT_START'));
      expect(types, contains('COMMENT_END'));
    });

    test('empty comment', () {
      expect(tokenPairs(tokenizeXml('<!---->')), [
        ('COMMENT_START', '<!--'),
        ('COMMENT_END', '-->'),
      ]);
    });

    test('comment then cdata', () {
      final types = tokenTypes(tokenizeXml('<!-- comment --><![CDATA[data]]>'));
      expect(types, contains('COMMENT_START'));
      expect(types, contains('COMMENT_END'));
      expect(types, contains('CDATA_START'));
      expect(types, contains('CDATA_END'));
    });
  });

  group('cdata', () {
    test('simple cdata', () {
      expect(tokenPairs(tokenizeXml('<![CDATA[raw text]]>')), [
        ('CDATA_START', '<![CDATA['),
        ('CDATA_TEXT', 'raw text'),
        ('CDATA_END', ']]>'),
      ]);
    });

    test('cdata with angle brackets', () {
      expect(textsOf(tokenizeXml('<![CDATA[<not a tag>]]>'), 'CDATA_TEXT'),
          ['<not a tag>']);
    });

    test('cdata preserves whitespace', () {
      expect(
          textsOf(tokenizeXml('<![CDATA[  hello\n  world  ]]>'), 'CDATA_TEXT'),
          ['  hello\n  world  ']);
    });

    test('cdata with single bracket splits and reassembles', () {
      final texts = textsOf(tokenizeXml('<![CDATA[a]b]]>'), 'CDATA_TEXT');
      expect(texts, ['a', ']', 'b']);
      expect(texts.join(), 'a]b');
    });

    test('cdata inside element', () {
      final types =
          tokenTypes(tokenizeXml('<script><![CDATA[x < y]]></script>'));
      expect(types, contains('CDATA_START'));
      expect(types, contains('CDATA_TEXT'));
      expect(types, contains('CDATA_END'));
    });
  });

  group('processing instructions', () {
    test('xml declaration', () {
      expect(tokenPairs(tokenizeXml('<?xml version="1.0"?>')), [
        ('PI_START', '<?'),
        ('PI_TARGET', 'xml'),
        ('PI_TEXT', ' version="1.0"'),
        ('PI_END', '?>'),
      ]);
    });

    test('stylesheet pi', () {
      final types =
          tokenTypes(tokenizeXml('<?xml-stylesheet type="text/xsl"?>'));
      expect(types[0], 'PI_START');
      expect(types[1], 'PI_TARGET');
      expect(types.last, 'PI_END');
    });

    test('pi body question mark never retokenizes as target', () {
      final pairs = tokenPairs(tokenizeXml('<?t a?b?>'));
      expect([
        for (final (t, _) in pairs)
          if (t == 'PI_TARGET') t
      ], hasLength(1));
      expect(textsOf(tokenizeXml('<?t a?b?>'), 'PI_TEXT').join(), ' a?b');
    });
  });

  group('entities', () {
    test('named entity', () {
      expect(tokenPairs(tokenizeXml('a&amp;b')), [
        ('TEXT', 'a'),
        ('ENTITY_REF', '&amp;'),
        ('TEXT', 'b'),
      ]);
    });

    test('decimal char ref', () {
      expect(tokenPairs(tokenizeXml('&#65;')), [('CHAR_REF', '&#65;')]);
    });

    test('hex char ref', () {
      expect(tokenPairs(tokenizeXml('&#x41;')), [('CHAR_REF', '&#x41;')]);
    });

    test('multiple entities', () {
      expect(tokenTypes(tokenizeXml('&lt;hello&gt;')),
          ['ENTITY_REF', 'TEXT', 'ENTITY_REF']);
    });
  });

  group('documents', () {
    test('nested elements', () {
      final types = tokenTypes(tokenizeXml('<a><b>text</b></a>'));
      expect(types.where((t) => t == 'OPEN_TAG_START'), hasLength(2));
      expect(types.where((t) => t == 'CLOSE_TAG_START'), hasLength(2));
    });

    test('mixed content', () {
      expect(textsOf(tokenizeXml('<p>Hello <b>world</b>!</p>'), 'TEXT'),
          ['Hello ', 'world', '!']);
    });

    test('full document', () {
      final source = '<?xml version="1.0"?>'
          '<!-- A greeting -->'
          '<root lang="en">'
          '<greeting>Hello &amp; welcome</greeting>'
          '</root>';
      final tokens = tokenizeXml(source);
      final types = tokenTypes(tokens);
      expect(types, contains('PI_START'));
      expect(types, contains('PI_END'));
      expect(types, contains('COMMENT_START'));
      expect(types, contains('COMMENT_END'));
      expect(types.where((t) => t == 'OPEN_TAG_START'), hasLength(2));
      expect(types.where((t) => t == 'CLOSE_TAG_START'), hasLength(2));
      expect(types, contains('ENTITY_REF'));
      expect(tokens.last.type, 'EOF');
    });

    test('deeply nested', () {
      final tokens = tokenizeXml('<a><b><c>deep</c></b></a>');
      final types = tokenTypes(tokens);
      expect(types.where((t) => t == 'OPEN_TAG_START'), hasLength(3));
      expect(types.where((t) => t == 'CLOSE_TAG_START'), hasLength(3));
      expect(textsOf(tokens, 'TEXT'), ['deep']);
    });

    test('multiple self-closing', () {
      final types = tokenTypes(tokenizeXml('<br/><hr/><img/>'));
      expect(types.where((t) => t == 'SELF_CLOSE'), hasLength(3));
      expect(types.where((t) => t == 'OPEN_TAG_START'), hasLength(3));
    });

    test('tag name with special chars', () {
      final pairs = tokenPairs(tokenizeXml('<my-tag.v2>text</my-tag.v2>'));
      expect(pairs[1], ('TAG_NAME', 'my-tag.v2'));
      expect(pairs[5], ('TAG_NAME', 'my-tag.v2'));
    });
  });

  group('edges', () {
    test('empty string yields only EOF', () {
      final tokens = tokenizeXml('');
      expect(tokens, hasLength(1));
      expect(tokens[0].type, 'EOF');
    });

    test('text only', () {
      expect(tokenPairs(tokenizeXml('just text')), [('TEXT', 'just text')]);
    });

    test('whitespace between tags is skipped', () {
      expect(textsOf(tokenizeXml('<a> <b> </b> </a>'), 'TEXT'), isEmpty);
    });

    test('EOF is always last', () {
      expect(tokenizeXml('<root/>').last.type, 'EOF');
    });

    test('factory lexer tokenizes', () {
      final tokens = createXmlLexer('<p>hi</p>').tokenize();
      expect(tokens.length, greaterThanOrEqualTo(2));
      expect(tokens.last.type, 'EOF');
    });

    test('lone ampersand fails closed', () {
      expect(() => tokenizeXml('&'), throwsA(isA<LexerError>()));
      expect(() => tokenizeXml('a & b'), throwsA(isA<LexerError>()));
    });

    test('lexer error carries position and id', () {
      try {
        tokenizeXml('<a>&</a>');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect(e.line, 1);
        expect(e.column, 4);
        expect(e.errorId, isNotEmpty);
      }
    });

    test('unclosed constructs run to EOF without error', () {
      expect(tokenPairs(tokenizeXml('<!-- abc')),
          [('COMMENT_START', '<!--'), ('COMMENT_TEXT', ' abc')]);
      expect(tokenPairs(tokenizeXml('<div')),
          [('OPEN_TAG_START', '<'), ('TAG_NAME', 'div')]);
      expect(tokenPairs(tokenizeXml('<![CDATA[x')),
          [('CDATA_START', '<![CDATA['), ('CDATA_TEXT', 'x')]);
    });

    test('newline at group position becomes NEWLINE', () {
      // The engine checks bare newlines before grammar patterns (both
      // lanes, by construction) — even with skips disabled.
      expect(tokenPairs(tokenizeXml('<!--\n-->')), [
        ('COMMENT_START', '<!--'),
        ('NEWLINE', '\\n'),
        ('COMMENT_END', '-->'),
      ]);
    });

    test('equals and brackets stay text outside tags', () {
      expect(tokenPairs(tokenizeXml('1 + 1 = 2')), [('TEXT', '1 + 1 = 2')]);
      expect(tokenPairs(tokenizeXml('a > b')), [('TEXT', 'a > b')]);
    });

    test('positions track lines and columns', () {
      final tokens = tokenizeXml('<a>\n<b>\n');
      final opens = [
        for (final t in tokens)
          if (t.type == 'OPEN_TAG_START') t
      ];
      expect(opens, hasLength(2));
      expect((opens[0].line, opens[0].column), (1, 1));
      expect((opens[1].line, opens[1].column), (2, 1));
      expect(tokens.last.type, 'EOF');
      expect(tokens.last.line, 3);
    });
  });
}
