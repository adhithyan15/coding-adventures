/// Unit tests for `coding_adventures_macronib_lexer`: all 4
/// reference tests, plus the corners the reference leaves uncovered
/// — keyword promotion across the whole table, the raw factory's
/// unpromoted output, the nullable twin, error positions, and empty
/// input.
///
/// The two grammar-conformance tests read the canonical `.tokens`
/// sources (relative to the package root, like the reference reads
/// them relative to its manifest dir) and parse them with the
/// in-lane `grammar_tools` parser.
library;

import 'dart:io';

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_macronib_lexer/macronib_lexer.dart';
import 'package:coding_adventures_macronib_lexer/src/_grammar.dart';
import 'package:coding_adventures_nib_lexer/nib_lexer.dart';
import 'package:test/test.dart';

/// The six MacroNib-only rules: five assembler-style directives and
/// the quoted-include string.
const only = [
  'DOT_INCLUDE',
  'DOT_IFDEF',
  'DOT_ENDIF',
  'DOT_ELSE',
  'DOT_SET',
  'STR_LIT',
];

TokenGrammar parse(String language, String file) {
  final path = '../../../grammars/$language/$file';
  return parseTokenGrammar(File(path).readAsStringSync());
}

(String, String, bool) shape(TokenDefinition definition) =>
    (definition.name, definition.pattern, definition.isRegex);

void main() {
  group('reference', () {
    test('directives and quoted include lex as own tokens', () {
      for (final (text, kind) in [
        ('.include', 'DOT_INCLUDE'),
        ('.ifdef', 'DOT_IFDEF'),
        ('.endif', 'DOT_ENDIF'),
        ('.else', 'DOT_ELSE'),
        ('.set', 'DOT_SET'),
        ('"helper.nib"', 'STR_LIT'),
      ]) {
        final tokens = tokenizeMacronib(text);
        expect(tokens.length, equals(2));
        expect(tokens[0].value, equals(text));
        expect(tokens[0].type, equals(kind));
        expect(tokens[1].type, equals('EOF'));
      }
    });

    test('ordinary nib tokens keep their values and types', () {
      const source = 'fn main() { let x: u4 = 7; }';
      final reference = tokenizeNib(source);
      final actual = tokenizeMacronib(source);
      expect(actual, equals(reference));
    });

    test('every non-directive rule matches nib in order', () {
      final nib = parse('nib', 'nib.tokens');
      final macroNib = parse('macronib', 'macronib.tokens');
      expect(
        macroNib.definitions
            .where((d) => !only.contains(d.name))
            .map(shape)
            .toList(),
        equals(nib.definitions.map(shape).toList()),
      );
      for (final extra in only) {
        expect(
          macroNib.definitions.any((d) => d.name == extra),
          isTrue,
          reason: 'missing $extra',
        );
      }
      expect(macroNib.keywords, equals(nib.keywords));
      expect(
        macroNib.skipDefinitions.map(shape).toList(),
        equals(nib.skipDefinitions.map(shape).toList()),
      );
      expect(macroNib.caseSensitive, equals(nib.caseSensitive));
      expect(macroNib.caseInsensitive, equals(nib.caseInsensitive));
    });

    test('compiled grammar matches source file', () {
      final source = parse('macronib', 'macronib.tokens');
      final compiled = tokenGrammar;
      expect(
        compiled.definitions.map(shape).toList(),
        equals(source.definitions.map(shape).toList()),
      );
      expect(compiled.keywords, equals(source.keywords));
      expect(
        compiled.skipDefinitions.map(shape).toList(),
        equals(source.skipDefinitions.map(shape).toList()),
      );
    });
  });

  group('extras', () {
    test('every keyword promotes across the table', () {
      for (final kw in [
        'fn',
        'let',
        'static',
        'const',
        'return',
        'for',
        'while',
        'in',
        'if',
        'else',
        'true',
        'false',
      ]) {
        final tokens = tokenizeMacronib(kw);
        expect(tokens[0].type, equals(kw));
        expect(tokens[0].value, equals(kw));
      }
    });

    test('non-keyword names keep the NAME type', () {
      final tokens = tokenizeMacronib('main');
      expect(tokens[0].type, equals('NAME'));
    });

    test('factory output is raw: keywords stay KEYWORD', () {
      // The reference factory returns the raw engine without the
      // post-pass — only the one-shots promote.
      final tokens = createMacronibLexer('fn main() {}').tokenize();
      expect(tokens[0].type, equals('KEYWORD'));
      expect(tokens[0].value, equals('fn'));
    });

    test('nullable twin returns null on invalid input', () {
      expect(tryTokenizeMacronib('fn main() {}'), isNotNull);
      expect(tryTokenizeMacronib('@'), isNull);
    });

    test('errors carry positions', () {
      // `@` has no token in this grammar.
      expect(() => tokenizeMacronib('@'), throwsA(isA<LexerError>()));
      try {
        tokenizeMacronib('ab@cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), equals((1, 3)));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeMacronib('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, equals('EOF'));
    });
  });
}
