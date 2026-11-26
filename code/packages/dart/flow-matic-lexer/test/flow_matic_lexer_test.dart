/// Unit tests for `coding_adventures_flow_matic_lexer`: all 15
/// reference tests, plus the corners the reference leaves uncovered —
/// error positions, the nullable API shape, and empty input.
library;

import 'package:coding_adventures_flow_matic_lexer/flow_matic_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

List<(String, String)> pairs(String src) => [
      for (final t in tokenizeFlowMatic(src))
        if (t.type != 'EOF') (t.type, t.value)
    ];

List<String> kinds(String src) => [for (final (k, _) in pairs(src)) k];

void main() {
  group('names', () {
    test('hyphenated name is one token', () {
      expect(pairs('PRODUCT-NO'), [('NAME', 'PRODUCT-NO')]);
      expect(pairs('UNPRICED-INV'), [('NAME', 'UNPRICED-INV')]);
    });

    test('single letters and file names are names', () {
      expect(kinds('A B FILE-A FILE-B'), ['NAME', 'NAME', 'NAME', 'NAME']);
      expect(pairs('FILE-A')[0].$2, 'FILE-A');
    });

    test('lowercase name still matches', () {
      expect(pairs('product-no'), [('NAME', 'product-no')]);
    });
  });

  group('keywords', () {
    test('verbs are uppercase keywords', () {
      expect(pairs('compare'), [('KEYWORD', 'COMPARE')]);
      expect(pairs('Transfer'), [('KEYWORD', 'TRANSFER')]);
      expect(pairs('COMPARE'), [('KEYWORD', 'COMPARE')]);
    });

    test('hyphenated verbs promote to keywords', () {
      expect(pairs('WRITE-ITEM'), [('KEYWORD', 'WRITE-ITEM')]);
      expect(pairs('READ-ITEM'), [('KEYWORD', 'READ-ITEM')]);
      expect(pairs('CLOSE-OUT'), [('KEYWORD', 'CLOSE-OUT')]);
    });

    test('keyword prefix word stays a name', () {
      expect(pairs('INVENTORY'), [('NAME', 'INVENTORY')]);
    });
  });

  group('punctuation', () {
    test('operation label is paren number paren', () {
      expect(pairs('(0)'), [('LPAREN', '('), ('NUMBER', '0'), ('RPAREN', ')')]);
    });

    test('field qualifier is paren name paren', () {
      expect(kinds('(A)'), ['LPAREN', 'NAME', 'RPAREN']);
    });

    test('period and semicolon', () {
      expect(kinds('. ;'), ['PERIOD', 'SEMICOLON']);
    });
  });

  group('clauses', () {
    test('compare clause', () {
      expect(pairs('COMPARE PRODUCT-NO (A) WITH PRODUCT-NO (B)'), [
        ('KEYWORD', 'COMPARE'),
        ('NAME', 'PRODUCT-NO'),
        ('LPAREN', '('),
        ('NAME', 'A'),
        ('RPAREN', ')'),
        ('KEYWORD', 'WITH'),
        ('NAME', 'PRODUCT-NO'),
        ('LPAREN', '('),
        ('NAME', 'B'),
        ('RPAREN', ')'),
      ]);
    });

    test('if branch clause', () {
      expect(pairs('IF GREATER GO TO OPERATION 10'), [
        ('KEYWORD', 'IF'),
        ('KEYWORD', 'GREATER'),
        ('KEYWORD', 'GO'),
        ('KEYWORD', 'TO'),
        ('KEYWORD', 'OPERATION'),
        ('NUMBER', '10'),
      ]);
    });

    test('read item end of data clause', () {
      expect(kinds('READ-ITEM A ; IF END OF DATA GO TO OPERATION 14'), [
        'KEYWORD',
        'NAME',
        'SEMICOLON',
        'KEYWORD',
        'KEYWORD',
        'KEYWORD',
        'KEYWORD',
        'KEYWORD',
        'KEYWORD',
        'KEYWORD',
        'NUMBER',
      ]);
    });

    test('newlines are whitespace', () {
      final oneLine = kinds('INPUT INVENTORY FILE-A ; HSP D .');
      final wrapped = kinds('INPUT INVENTORY FILE-A ;\n   HSP D .');
      expect(wrapped, oneLine);
    });

    test('canonical program head tokenizes', () {
      final src = '\n'
          '(0)  INPUT INVENTORY FILE-A PRICE FILE-B ; OUTPUT PRICED-INV FILE-C\n'
          '       UNPRICED-INV FILE-D ; HSP D .\n'
          '(1)  COMPARE PRODUCT-NO (A) WITH PRODUCT-NO (B) ;\n'
          '       IF GREATER GO TO OPERATION 10 ;\n'
          '       IF EQUAL GO TO OPERATION 5 ;\n'
          '       OTHERWISE GO TO OPERATION 2 .';
      final toks = tokenizeFlowMatic(src);
      expect(toks.last.type, 'EOF');
      expect(toks.where((t) => t.type == 'EOF'), hasLength(1));
      expect(toks.where((t) => t.type == 'PERIOD'), hasLength(2));
      for (final word in ['GREATER', 'EQUAL', 'OTHERWISE']) {
        expect(toks.any((t) => t.type == 'KEYWORD' && t.value == word), isTrue);
      }
    });
  });

  group('errors', () {
    test('unknown character is an error', () {
      expect(tryTokenizeFlowMatic('(0) @ .'), isNull);
    });

    test('errors carry positions', () {
      try {
        tokenizeFlowMatic('AB@CD');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), (1, 3));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('nullable API mirrors the throwing API', () {
      expect(tryTokenizeFlowMatic('INPUT A.'), isNotNull);
      expect(() => tokenizeFlowMatic('(0) @ .'), throwsA(isA<LexerError>()));
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeFlowMatic('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, 'EOF');
    });

    test('factory tokenizes directly', () {
      final tokens = createFlowMaticLexer('INPUT A.').tokenize();
      expect(tokens.last.type, 'EOF');
      expect(() => createFlowMaticLexer('@').tokenize(),
          throwsA(isA<LexerError>()));
    });
  });
}
