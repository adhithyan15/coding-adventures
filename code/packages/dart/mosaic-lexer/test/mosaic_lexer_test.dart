import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_mosaic_lexer/mosaic_lexer.dart';
import 'package:test/test.dart';

void main() {
  List<Token> meaningful(String source) => tokenizeMosaic(
        source,
      ).where((token) => token.type != 'EOF').toList(growable: false);

  List<String> types(String source) =>
      meaningful(source).map((token) => token.type).toList(growable: false);

  group('Mosaic keywords and names', () {
    test('classifies structural and type keywords', () {
      expect(
        types(
          'component slot import from as text number bool image color '
          'node list true false when each',
        ),
        List<String>.filled(16, 'KEYWORD'),
      );
    });

    test('keeps component and hyphenated property names intact', () {
      final tokens = meaningful('ProfileCard avatar-url corner-radius');
      expect(tokens.map((token) => token.type), everyElement('NAME'));
      expect(tokens.map((token) => token.value), [
        'ProfileCard',
        'avatar-url',
        'corner-radius',
      ]);
    });
  });

  group('Mosaic literals and punctuation', () {
    test('recognizes dimensions before bare numbers', () {
      final tokens = meaningful('16dp 50% -1.5sp 42 -3.25');
      expect(tokens.map((token) => token.type), [
        'DIMENSION',
        'DIMENSION',
        'DIMENSION',
        'NUMBER',
        'NUMBER',
      ]);
      expect(tokens.map((token) => token.value), [
        '16dp',
        '50%',
        '-1.5sp',
        '42',
        '-3.25',
      ]);
    });

    test('recognizes strings, colors, and every delimiter', () {
      final tokens = meaningful(r'"hello\nworld" #fff #2563eb {}<>:;,.=@');
      expect(tokens.first.type, 'STRING');
      expect(tokens.first.value, 'hello\nworld');
      expect(tokens[1].type, 'COLOR_HEX');
      expect(tokens[2].type, 'COLOR_HEX');
      expect(tokens.skip(3).map((token) => token.type), [
        'LBRACE',
        'RBRACE',
        'LANGLE',
        'RANGLE',
        'COLON',
        'SEMICOLON',
        'COMMA',
        'DOT',
        'EQUALS',
        'AT',
      ]);
    });
  });

  group('Mosaic documents', () {
    test('tokenizes a complete component', () {
      const source = '''
component ProfileCard {
  slot avatar-url: image;
  slot display-name: text;
  Column {
    Image { source: @avatar-url; corner-radius: 50%; }
    Text { content: @display-name; color: #2563eb; }
  }
}
''';
      final tokens = meaningful(source);
      expect(tokens.first.value, 'component');
      expect(tokens.any((token) => token.value == 'avatar-url'), isTrue);
      expect(tokens.any((token) => token.type == 'DIMENSION'), isTrue);
      expect(tokens.any((token) => token.type == 'COLOR_HEX'), isTrue);
      expect(tokens.last.type, 'RBRACE');
    });

    test('skips line and block comments and tracks positions', () {
      final tokens = tokenizeMosaic('''
// heading
component Demo { /* inline */
  Box { }
}
''');
      final component = tokens.first;
      final box = tokens.firstWhere((token) => token.value == 'Box');
      expect(component.value, 'component');
      expect(component.line, 2);
      expect(component.column, 1);
      expect(box.line, 3);
      expect(box.column, 3);
      expect(tokens.last.type, 'EOF');
    });

    test('returns EOF for empty or ignored input', () {
      expect(tokenizeMosaic('').map((token) => token.type), ['EOF']);
      expect(tokenizeMosaic(' // comment').map((token) => token.type), ['EOF']);
    });

    test('rejects characters outside the grammar', () {
      expect(() => tokenizeMosaic(r'$'), throwsA(isA<LexerError>()));
    });
  });
}
