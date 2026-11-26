/// Unit tests for `coding_adventures_macrooct_lexer`: all 11
/// reference tests, plus the corners the reference leaves uncovered
/// — the doc-comment example, keyword promotion across the whole
/// table, the raw factory's unpromoted output, the nullable twin,
/// and empty input.
///
/// The two drift-guard tests read the canonical `.tokens` sources
/// (relative to the package root, like the reference reads them
/// relative to its manifest dir) and parse them with the in-lane
/// `grammar_tools` parser.
library;

import 'dart:io';

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_macrooct_lexer/macrooct_lexer.dart';
import 'package:coding_adventures_macrooct_lexer/src/_grammar.dart';
import 'package:test/test.dart';

/// The names MacroOct adds. Everything else must match Oct exactly.
const macrooctOnly = [
  'AT_INCLUDE',
  'AT_DEFINE',
  'AT_IF',
  'AT_ELSE',
  'AT_END',
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
    test('tokenizes plain oct source unchanged', () {
      final tokens = tokenizeMacrooct('fn main() { let x: u8 = 5; }');
      final values = tokens.map((t) => t.value);
      expect(values, contains('fn'));
      expect(values, contains('main'));
      expect(values, contains('5'));
      expect(values, contains('u8'));
    });

    test('keyword type names are promoted', () {
      final tokens = tokenizeMacrooct('fn main() {}');
      final fnTok = tokens.firstWhere((t) => t.value == 'fn');
      expect(fnTok.type, equals('fn'));
    });

    test('each directive lexes as one token of its own type', () {
      // One token, not `@` followed by a word: the dialect classifies a
      // line by its first token's value, so a split spelling would make
      // every directive unrecognisable.
      for (final (src, name) in [
        ('@include', 'AT_INCLUDE'),
        ('@define', 'AT_DEFINE'),
        ('@if', 'AT_IF'),
        ('@else', 'AT_ELSE'),
        ('@end', 'AT_END'),
      ]) {
        final tokens = tokenizeMacrooct(src);
        expect(tokens.length, equals(2),
            reason: '$src must lex as one token plus EOF');
        expect(tokens[0].value, equals(src));
        expect(tokens[0].type, equals(name));
        expect(tokens[1].type, equals('EOF'));
      }
    });

    test('the stream ends with an eof sentinel', () {
      for (final src in ['', 'fn main() {}', '@end', '@end\n']) {
        final tokens = tokenizeMacrooct(src);
        expect(tokens.last.type, equals('EOF'),
            reason: '$src must end with the EOF sentinel');
        expect(
          tokens.sublist(0, tokens.length - 1).every((t) => t.type != 'EOF'),
          isTrue,
          reason: '$src must carry exactly one EOF, at the end',
        );
      }
      // And on a source with no trailing newline, the sentinel really
      // does share the last line — the case that would silently eat it.
      final tokens = tokenizeMacrooct('@if 1\nfn main() {}\n@end');
      final end = tokens.firstWhere((t) => t.value == '@end');
      expect(tokens.last.line, equals(end.line));
    });

    test('at if does not shadow at include', () {
      // `@if` is listed after `@include`, but the two only diverge at
      // their third character: `@include` must not lex as AT_IF
      // followed by garbage.
      final tokens = tokenizeMacrooct('@include "ports.oct"');
      expect(tokens[0].type, equals('AT_INCLUDE'));
      expect(tokens[1].type, equals('STR_LIT'));
      expect(tokens[1].value, equals('"ports.oct"'));
    });

    test('the keyword if and the directive at if are different tokens', () {
      // The practical dividend of the `@` prefix: MacroOct never has to
      // decide whether `if` at the start of a line means Oct's
      // conditional or the preprocessor's conditional directive.
      final tokens = tokenizeMacrooct('@if 1\nif x == 1 { }\n@end');
      expect(tokens[0].type, equals('AT_IF'));
      final octIf = tokens.firstWhere((t) => t.value == 'if');
      expect(octIf.type, equals('if'));
    });

    test('a string literal stops at the end of its line', () {
      // An unterminated quote must not swallow the rest of the file.
      // `STR_LIT` excludes `\n`, so this input has no STR_LIT at all
      // and fails to lex — reported here, on this line.
      expect(tryTokenizeMacrooct('@include "oops\nfn main() {}'), isNull);
    });

    test('tokens carry the line the engine groups them by', () {
      // MacroOct's grammar skips newlines (Oct's does), so nothing marks
      // an end of line in the token stream. The preprocessor groups by
      // `Token.line` instead, which makes this field load-bearing rather
      // than merely diagnostic.
      final tokens = tokenizeMacrooct('@if 1\nfn main() { }\n@end\n');
      expect(tokens[0].line, equals(1));
      expect(tokens[1].line, equals(1));
      expect(tokens.where((t) => t.line == 2).length, greaterThanOrEqualTo(5));
      final end = tokens.firstWhere((t) => t.value == '@end');
      expect(end.line, equals(3));
      // `@end` is alone on its line — nothing else shares it, which is
      // what makes "the run of tokens on an AT_* token's line" a sound
      // definition of "the directive".
      expect(tokens.where((t) => t.line == 3).length, equals(1));
    });

    test('a lex failure is an error not a panic', () {
      // The engine's no-panic contract reaches through into this lane,
      // because an included file's text is chosen by the program being
      // compiled.
      expect(tryTokenizeMacrooct('fn main() { let x: u8 = \$; }'), isNull);
    });

    test('macrooct agrees with oct on every non-directive rule', () {
      final oct = parse('oct', 'oct.tokens');
      final macrooct = parse('macrooct', 'macrooct.tokens');

      final octRules = oct.definitions.map(shape).toList();
      final macrooctRules = macrooct.definitions
          .where((d) => !macrooctOnly.contains(d.name))
          .map(shape)
          .toList();
      expect(
        macrooctRules,
        equals(octRules),
        reason: 'macrooct.tokens has drifted from oct.tokens on a '
            'non-directive rule. Every rule except $macrooctOnly must '
            'match Oct exactly, in Oct order (first-match-wins, so order '
            'is semantics). Update macrooct.tokens to match; do NOT '
            'change oct.tokens, which PREP01 holds fixed as the reference.',
      );

      // The additions must actually be present — otherwise deleting all
      // the directive rules would make the filter vacuous and this test
      // pass.
      final names = macrooct.definitions.map((d) => d.name).toList();
      for (final extra in macrooctOnly) {
        expect(names, contains(extra),
            reason: 'macrooct.tokens lost its $extra rule');
      }

      // Keywords and skip rules are part of the lexical language too.
      expect(
        macrooct.keywords,
        equals(oct.keywords),
        reason: "MacroOct must promote exactly Oct's keywords — a "
            'directive is spelled with a leading `@`, so MacroOct needs '
            'no keyword of its own',
      );
      expect(
        macrooct.skipDefinitions.map(shape).toList(),
        equals(oct.skipDefinitions.map(shape).toList()),
        reason: 'MacroOct must skip exactly what Oct skips',
      );
      expect(macrooct.caseSensitive, equals(oct.caseSensitive));
      expect(macrooct.caseInsensitive, equals(oct.caseInsensitive));
    });

    test('the compiled grammar matches the macrooct tokens file', () {
      final fromFile = parse('macrooct', 'macrooct.tokens');
      final compiled = tokenGrammar;
      expect(
        compiled.definitions.map(shape).toList(),
        equals(fromFile.definitions.map(shape).toList()),
        reason: 'lib/src/_grammar.dart is stale — regenerate it from '
            'code/grammars/macrooct/macrooct.tokens',
      );
      expect(compiled.keywords, equals(fromFile.keywords));
      expect(
        compiled.skipDefinitions.map(shape).toList(),
        equals(fromFile.skipDefinitions.map(shape).toList()),
      );
    });
  });

  group('extras', () {
    test('doc example: directive and guarded code on separate lines', () {
      final tokens =
          tokenizeMacrooct('@if 1\nfn main() { out(1, 42); }\n@end\n');
      expect(tokens[0].value, equals('@if'));
      expect(tokens[0].line, equals(1));
      expect(tokens.any((t) => t.value == 'fn' && t.line == 2), isTrue);
    });

    test('every keyword promotes across the table', () {
      for (final kw in [
        'fn',
        'let',
        'static',
        'if',
        'else',
        'while',
        'loop',
        'break',
        'return',
        'true',
        'false',
        'in',
        'out',
        'adc',
        'sbb',
        'rlc',
        'rrc',
        'ral',
        'rar',
        'carry',
        'parity',
      ]) {
        final tokens = tokenizeMacrooct(kw);
        expect(tokens[0].type, equals(kw));
        expect(tokens[0].value, equals(kw));
      }
    });

    test('non-keyword names keep the NAME type', () {
      final tokens = tokenizeMacrooct('main');
      expect(tokens[0].type, equals('NAME'));
    });

    test('factory output is raw: keywords stay KEYWORD', () {
      // The reference factory returns the raw engine without the
      // post-pass — only the one-shots promote.
      final tokens = createMacrooctLexer('fn main() {}').tokenize();
      expect(tokens[0].type, equals('KEYWORD'));
      expect(tokens[0].value, equals('fn'));
    });

    test('nullable twin agrees with the throwing API', () {
      final hard = tokenizeMacrooct('@if 1\nfn main() {}\n@end\n');
      expect(tryTokenizeMacrooct('@if 1\nfn main() {}\n@end\n'), equals(hard));
      expect(tryTokenizeMacrooct('\$'), isNull);
    });

    test('errors carry positions', () {
      // `$` has no token in this grammar.
      expect(() => tokenizeMacrooct('\$'), throwsA(isA<LexerError>()));
      try {
        tokenizeMacrooct('ab\$cd');
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect((e.line, e.column), equals((1, 3)));
        expect(e.errorId, isNotEmpty);
      }
    });

    test('empty input yields only EOF', () {
      final tokens = tokenizeMacrooct('');
      expect(tokens, hasLength(1));
      expect(tokens.single.type, equals('EOF'));
    });
  });
}
