/// Port of the reference `rust/python-parser` release suites
/// (`release_probes.rs`, 13 tests; `tokenize_differential.rs`, 3
/// tests), plus lane extras. The reference reads `releases.csv`,
/// `pair-evidence.csv`, grammar headers, and fixture files from disk;
/// this lane transcribes those committed artifacts instead (lane
/// tests never read outside the package): the 11-release inventory
/// is pinned, and the 6 CPython differential fixtures are embedded
/// verbatim below.
library;

import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_python_lexer/python_lexer.dart';
import 'package:coding_adventures_python_parser/python_parser.dart';
import 'package:test/test.dart';

const _py2Releases = ['2.2', '2.4', '2.6', '2.7.18'];

List<String> _py3Releases() => [
      for (final r in supportedPythonParserReleases)
        if (!_py2Releases.contains(r)) r,
    ];

PythonParse _parse(String release, String source) =>
    parsePythonAt(source, release, const FrontendBudgets());

PythonParse? _parseOrNull(String release, String source) {
  try {
    return _parse(release, source);
  } on PythonFrontendError {
    return null;
  }
}

/// Transcribed `tokenize/<case>.py` sources and `<case>.pytokens`
/// expectations (recorded by `tokenize_oracle.py` under host CPython).
const _tokenizeCases = <String, (String, String)>{
  '01_print_arith': (
    'print(1 + 2 * 3 - 8 // 4 % 3)\nprint(-7 // -2, +5, - -6)\n',
    'NAME\t1\t1\tprint\nLPAREN\t1\t6\t(\nINT\t1\t7\t1\nPLUS\t1\t9\t+\nINT\t1\t11\t2\nSTAR\t1\t13\t*\nINT\t1\t15\t3\nMINUS\t1\t17\t-\nINT\t1\t19\t8\nFLOOR_DIV\t1\t21\t//\nINT\t1\t24\t4\nPERCENT\t1\t26\t%\nINT\t1\t28\t3\nRPAREN\t1\t29\t)\nNEWLINE\nNAME\t2\t1\tprint\nLPAREN\t2\t6\t(\nMINUS\t2\t7\t-\nINT\t2\t8\t7\nFLOOR_DIV\t2\t10\t//\nMINUS\t2\t13\t-\nINT\t2\t14\t2\nCOMMA\t2\t15\t,\nPLUS\t2\t17\t+\nINT\t2\t18\t5\nCOMMA\t2\t19\t,\nMINUS\t2\t21\t-\nMINUS\t2\t23\t-\nINT\t2\t24\t6\nRPAREN\t2\t25\t)\nNEWLINE\nEOF\n'
  ),
  '02_numbers': (
    'print(0, 00, 0_0, 7, 1_000_000, 9223372036854775807)\nprint(1_2_3 * 4)\n',
    'NAME\t1\t1\tprint\nLPAREN\t1\t6\t(\nINT\t1\t7\t0\nCOMMA\t1\t8\t,\nINT\t1\t10\t00\nCOMMA\t1\t12\t,\nINT\t1\t14\t0_0\nCOMMA\t1\t17\t,\nINT\t1\t19\t7\nCOMMA\t1\t20\t,\nINT\t1\t22\t1_000_000\nCOMMA\t1\t31\t,\nINT\t1\t33\t9223372036854775807\nRPAREN\t1\t52\t)\nNEWLINE\nNAME\t2\t1\tprint\nLPAREN\t2\t6\t(\nINT\t2\t7\t1_2_3\nSTAR\t2\t13\t*\nINT\t2\t15\t4\nRPAREN\t2\t16\t)\nNEWLINE\nEOF\n'
  ),
  '03_operators': (
    'a **= b <<= c >>= d //= e\nx ** y // z << w >> v == u != t <= s >= r -> q\nm += 1; n -= 2; o *= 3; p /= 4; k %= 5; j &= 6; i |= 7; h ^= 8; g @= 9\nf := e ... @ d ~ c ^ b | a & z < y > x = w . v [ u ] { t } : s , r / q\n',
    'NAME\t1\t1\ta\nDOUBLE_STAR_EQUALS\t1\t3\t**=\nNAME\t1\t7\tb\nLEFT_SHIFT_EQUALS\t1\t9\t<<=\nNAME\t1\t13\tc\nRIGHT_SHIFT_EQUALS\t1\t15\t>>=\nNAME\t1\t19\td\nFLOOR_DIV_EQUALS\t1\t21\t//=\nNAME\t1\t25\te\nNEWLINE\nNAME\t2\t1\tx\nDOUBLE_STAR\t2\t3\t**\nNAME\t2\t6\ty\nFLOOR_DIV\t2\t8\t//\nNAME\t2\t11\tz\nLEFT_SHIFT\t2\t13\t<<\nNAME\t2\t16\tw\nRIGHT_SHIFT\t2\t18\t>>\nNAME\t2\t21\tv\nEQUALS_EQUALS\t2\t23\t==\nNAME\t2\t26\tu\nNOT_EQUALS\t2\t28\t!=\nNAME\t2\t31\tt\nLESS_EQUALS\t2\t33\t<=\nNAME\t2\t36\ts\nGREATER_EQUALS\t2\t38\t>=\nNAME\t2\t41\tr\nARROW\t2\t43\t->\nNAME\t2\t46\tq\nNEWLINE\nNAME\t3\t1\tm\nPLUS_EQUALS\t3\t3\t+=\nINT\t3\t6\t1\nSEMICOLON\t3\t7\t;\nNAME\t3\t9\tn\nMINUS_EQUALS\t3\t11\t-=\nINT\t3\t14\t2\nSEMICOLON\t3\t15\t;\nNAME\t3\t17\to\nSTAR_EQUALS\t3\t19\t*=\nINT\t3\t22\t3\nSEMICOLON\t3\t23\t;\nNAME\t3\t25\tp\nSLASH_EQUALS\t3\t27\t/=\nINT\t3\t30\t4\nSEMICOLON\t3\t31\t;\nNAME\t3\t33\tk\nPERCENT_EQUALS\t3\t35\t%=\nINT\t3\t38\t5\nSEMICOLON\t3\t39\t;\nNAME\t3\t41\tj\nAMP_EQUALS\t3\t43\t&=\nINT\t3\t46\t6\nSEMICOLON\t3\t47\t;\nNAME\t3\t49\ti\nPIPE_EQUALS\t3\t51\t|=\nINT\t3\t54\t7\nSEMICOLON\t3\t55\t;\nNAME\t3\t57\th\nCARET_EQUALS\t3\t59\t^=\nINT\t3\t62\t8\nSEMICOLON\t3\t63\t;\nNAME\t3\t65\tg\nAT_EQUALS\t3\t67\t@=\nINT\t3\t70\t9\nNEWLINE\nNAME\t4\t1\tf\nWALRUS\t4\t3\t:=\nNAME\t4\t6\te\nELLIPSIS\t4\t8\t...\nAT\t4\t12\t@\nNAME\t4\t14\td\nTILDE\t4\t16\t~\nNAME\t4\t18\tc\nCARET\t4\t20\t^\nNAME\t4\t22\tb\nPIPE\t4\t24\t|\nNAME\t4\t26\ta\nAMP\t4\t28\t&\nNAME\t4\t30\tz\nLESS_THAN\t4\t32\t<\nNAME\t4\t34\ty\nGREATER_THAN\t4\t36\t>\nNAME\t4\t38\tx\nEQUALS\t4\t40\t=\nNAME\t4\t42\tw\nDOT\t4\t44\t.\nNAME\t4\t46\tv\nLBRACKET\t4\t48\t[\nNAME\t4\t50\tu\nRBRACKET\t4\t52\t]\nLBRACE\t4\t54\t{\nNAME\t4\t56\tt\nRBRACE\t4\t58\t}\nCOLON\t4\t60\t:\nNAME\t4\t62\ts\nCOMMA\t4\t64\t,\nNAME\t4\t66\tr\nSLASH\t4\t68\t/\nNAME\t4\t70\tq\nNEWLINE\nEOF\n'
  ),
  '04_keywords': (
    'False None True and as assert async await break class continue def del elif else\nexcept finally for from global if import in is lambda nonlocal not or pass raise\nreturn try while with yield match case type _ print lazy\n',
    'KEYWORD\t1\t1\tFalse\nKEYWORD\t1\t7\tNone\nKEYWORD\t1\t12\tTrue\nKEYWORD\t1\t17\tand\nKEYWORD\t1\t21\tas\nKEYWORD\t1\t24\tassert\nKEYWORD\t1\t31\tasync\nKEYWORD\t1\t37\tawait\nKEYWORD\t1\t43\tbreak\nKEYWORD\t1\t49\tclass\nKEYWORD\t1\t55\tcontinue\nKEYWORD\t1\t64\tdef\nKEYWORD\t1\t68\tdel\nKEYWORD\t1\t72\telif\nKEYWORD\t1\t77\telse\nNEWLINE\nKEYWORD\t2\t1\texcept\nKEYWORD\t2\t8\tfinally\nKEYWORD\t2\t16\tfor\nKEYWORD\t2\t20\tfrom\nKEYWORD\t2\t25\tglobal\nKEYWORD\t2\t32\tif\nKEYWORD\t2\t35\timport\nKEYWORD\t2\t42\tin\nKEYWORD\t2\t45\tis\nKEYWORD\t2\t48\tlambda\nKEYWORD\t2\t55\tnonlocal\nKEYWORD\t2\t64\tnot\nKEYWORD\t2\t68\tor\nKEYWORD\t2\t71\tpass\nKEYWORD\t2\t76\traise\nNEWLINE\nKEYWORD\t3\t1\treturn\nKEYWORD\t3\t8\ttry\nKEYWORD\t3\t12\twhile\nKEYWORD\t3\t18\twith\nKEYWORD\t3\t23\tyield\nNAME\t3\t29\tmatch\nNAME\t3\t35\tcase\nNAME\t3\t40\ttype\nNAME\t3\t45\t_\nNAME\t3\t47\tprint\nNAME\t3\t53\tlazy\nNEWLINE\nEOF\n'
  ),
  '05_layout': (
    '# comment\n\nprint(1 +\n      2)  # trailing\nif x:\n    print(3)\n\n    # indented comment\nprint(4 \\\n      - 5)\n',
    'NAME\t3\t1\tprint\nLPAREN\t3\t6\t(\nINT\t3\t7\t1\nPLUS\t3\t9\t+\nINT\t4\t7\t2\nRPAREN\t4\t8\t)\nNEWLINE\nKEYWORD\t5\t1\tif\nNAME\t5\t4\tx\nCOLON\t5\t5\t:\nNEWLINE\nINDENT\nNAME\t6\t5\tprint\nLPAREN\t6\t10\t(\nINT\t6\t11\t3\nRPAREN\t6\t12\t)\nNEWLINE\nDEDENT\nNAME\t9\t1\tprint\nLPAREN\t9\t6\t(\nINT\t9\t7\t4\nMINUS\t10\t7\t-\nINT\t10\t9\t5\nRPAREN\t10\t10\t)\nNEWLINE\nEOF\n'
  ),
  '06_no_trailing_newline': (
    'print(1)',
    'NAME\t1\t1\tprint\nLPAREN\t1\t6\t(\nINT\t1\t7\t1\nRPAREN\t1\t8\t)\nNEWLINE\nEOF\n'
  ),
};

/// Our token stream in the fixture's line format.
String _stream(String release, String source) {
  final (_, tokens) =
      tokenizePythonAt(source, release, const FrontendBudgets());
  final out = StringBuffer();
  for (final token in tokens) {
    final kind = token.type;
    if (kind == 'NEWLINE' ||
        kind == 'INDENT' ||
        kind == 'DEDENT' ||
        kind == 'EOF') {
      out.writeln(kind);
    } else {
      out.writeln('$kind\t${token.line}\t${token.column}\t${token.value}');
    }
  }
  return out.toString();
}

void main() {
  group('python-parser releases', () {
    test('supported releases are exactly the paired inventory rows', () {
      // Pinned transcription of the paired `releases.csv` rows, in
      // inventory (date) order.
      expect(
          supportedPythonParserReleases,
          equals([
            '2.2',
            '2.4',
            '3.0a1',
            '2.6',
            '3.6.0',
            '2.7.18',
            '3.5.10',
            '3.12.3',
            '3.13.16',
            '3.11.17',
            '3.16.0a1',
          ]));
      expect(supportedPythonParserReleases, equals(supportedPythonReleases));
      expect(supportedPythonParserReleases, contains(defaultPythonRelease));
    });

    test('deduplicated classes follow the evidence', () {
      (String, String) cls(String r) => pythonGrammarClasses(r)!;
      expect(cls('3.12.3'), equals(cls('3.13.16')));
      expect(cls('3.13.16'), equals(cls('3.16.0a1')));
      expect(cls('3.11.17').$1, isNot(equals(cls('3.12.3').$1)));
      expect(cls('3.0a1').$1, isNot(equals(cls('3.11.17').$1)));
      final grammar = cls('3.5.10').$2;
      for (final r in _py3Releases()) {
        if (r == '3.0a1') continue;
        expect(cls(r).$2, equals(grammar), reason: r);
      }
      expect(cls('3.0a1').$2, isNot(equals(grammar)));
      for (final release in _py2Releases) {
        expect(cls('2.2').$2, equals(cls(release).$2), reason: release);
      }
      expect(cls('2.2').$2, isNot(equals(grammar)));
      expect(cls('2.2').$2, isNot(equals(cls('3.0a1').$2)));
      expect(cls('2.2').$1, isNot(equals(cls('2.4').$1)));
      expect(cls('2.4').$1, isNot(equals(cls('2.6').$1)));
      expect(cls('2.6').$1, equals(cls('2.7.18').$1));
      expect(cls('3.5.10').$1, isNot(equals(cls('3.0a1').$1)));
      expect(cls('3.6.0').$1, isNot(equals(cls('3.5.10').$1)));
      expect(pythonGrammarClasses('3.12.2'), isNull);
      expect(pythonGrammarClasses('3.12'), isNull);
    });

    test('release selection is exact and typed', () {
      expect(resolvePythonRelease(null), equals(defaultPythonRelease));
      expect(defaultPythonRelease, equals('3.13.16'));
      expect(resolvePythonRelease('3.0a1'), equals('3.0a1'));

      // A real row without a pair is not silently replaced.
      expect(
        () => resolvePythonRelease('3.12.2'),
        throwsA(
            const PythonReleaseNotAvailable(release: '3.12.2', stage: 'none')),
      );
      // Release 3.0 is a row; the legacy 3.0 minor-line pair is not
      // its pair and must not be reached through it.
      expect(() => resolvePythonRelease('3.0'),
          throwsA(isA<PythonReleaseNotAvailable>()));

      // An empty string is an error, not the default.
      expect(
        () => resolvePythonRelease(''),
        throwsA(const PythonUnknownRelease(requested: '', suggestions: [])),
      );

      // A minor line is not a tagged release; the error lists up to
      // five rows sharing the longest prefix.
      try {
        resolvePythonRelease('3.12');
        fail('3.12 must be unknown');
      } on PythonUnknownRelease catch (e) {
        expect(e.suggestions, isNotEmpty);
        expect(e.suggestions.length, lessThanOrEqualTo(5));
        for (final s in e.suggestions) {
          expect(s.startsWith('3.12'), isTrue, reason: '$s');
        }
      }

      // A hostile-length request is truncated before it is echoed.
      final long = '9' * 10000;
      try {
        resolvePythonRelease(long);
        fail('long ID must be unknown');
      } on PythonUnknownRelease catch (e) {
        expect(e.requested.length, lessThan(100));
        expect(e.suggestions, isEmpty);
      }

      // Control characters in a request are escaped, never echoed raw.
      try {
        resolvePythonRelease('x\nINFO: ok\x1b[2J');
        fail('a control-character ID must be unknown');
      } on PythonUnknownRelease catch (e) {
        expect(e.requested, equals(r'x\nINFO: ok\u{1b}[2J'));
        expect(
            e
                .toString()
                .codeUnits
                .any((c) => (c >= 0x00 && c <= 0x1F) || c == 0x7F),
            isFalse);
      }

      // Every entry point refuses before doing any work.
      const budgets = FrontendBudgets();
      expect(() => tokenizePythonAt('print(1)', '2.7.17', budgets),
          throwsA(isA<PythonReleaseNotAvailable>()));
      expect(() => parsePythonAt('print(1)', 'python-3.13.16', budgets),
          throwsA(isA<PythonUnknownRelease>()));
      expect(tokenGrammarForRelease('3.12'), isNull);
    });

    test('py int print probes for every release', () {
      const positiveAll = [
        'print(1 + 2)',
        'print(3 * 4)',
        'print(-7 // 2)',
        'print(-7 % 2)',
        'print(7 / 2)',
        'print()',
        'print(1,)',
        'print(1, 2)',
        'print((2))',
        'print(- 3)',
        'print(+-+3)',
        'print(2 * -3)',
        'print(1 +\n2 *\n3)',
        'print(\n1\n)',
        'print(1); print(2)',
        'print(1);',
        '\n\nprint(1)\n\n',
        '# leading comment\nprint(1) # comment\nprint(2)\n',
        'print(1 + \\\n 2)',
        'print(0)',
        'print(00)',
        'print(9223372036854775807)',
        'abs(1)',
        '1 + 2',
        'print(print(1))',
      ];
      const underscores = ['print(1_000)', 'print(0_0)', 'print(1_2_3 * 4)'];
      const floats = [
        'print(1.5)',
        'print(1e3)',
        'print(.5)',
        'print(1.)',
        'print(1. + 2)',
        'print(1.e5)',
        'print(1.5e-3 * 2E+2)',
        'print(00.5)',
        'print(0123e1)',
        'print(-0.0)',
        'print(7.5 // 2)',
      ];
      const floatUnderscores = [
        'print(1_000.5)',
        'print(1e1_0)',
        'print(.0_1)'
      ];
      const floatNegative = [
        'print(1j)',
        'print(1.5j)',
        'print(1_.5)',
        'print(1._5)',
        'print(1e)',
        'print(1e+)',
        'print(1e_1)',
        'print(1.5.5)',
        'print(1..5)',
        'print(1.5if)',
        'print(1.real)',
      ];
      const negativeAll = [
        'print(',
        'print(1',
        'print(1) print(2)',
        'print(1);;',
        ';print(1)',
        'print(1 +)',
        'print(0x10)',
        'print(\'a\')',
        'print(012)',
        'print(1__0)',
        'print(1_)',
        'print 1',
        'x = 1',
        'print(x[0])',
        'print(a.b)',
        'print(2 ** 3)',
        'print(~1)',
        'print(1 < 2)',
        'print(())',
        'print(1, end=None)',
        'print(*a)',
        '  print(1)',
        'if 1:\n    print(1)',
        'print(1) \\',
        'print(1 + \\ \n2)',
      ];
      var checked = 0;
      final releases = _py3Releases();
      for (final release in releases) {
        final early = release == '3.0a1' || release == '3.5.10';
        for (final source in positiveAll) {
          try {
            final ast = _parse(release, source).ast;
            expect(ast.ruleName, equals('file'), reason: '$release: $source');
          } on PythonFrontendError catch (e) {
            fail('$release: $source rejected: $e');
          }
          checked++;
        }
        for (final source in underscores) {
          try {
            _parse(release, source);
            expect(!early, isTrue, reason: '$release must not lex $source');
          } on PythonLexError {
            expect(early, isTrue, reason: '$release: $source');
          } on PythonFrontendError catch (e) {
            fail('$release: $source: $e');
          }
          checked++;
        }
        final floatOk = release != '3.0a1';
        for (final source in floats) {
          try {
            _parse(release, source);
            expect(floatOk, isTrue, reason: '$release must not parse $source');
          } on PythonLexError {
            expect(!floatOk, isTrue, reason: '$release: $source');
          } on PythonParseError {
            expect(!floatOk, isTrue, reason: '$release: $source');
          } on PythonFrontendError catch (e) {
            fail('$release: $source: $e');
          }
          checked++;
        }
        for (final source in floatUnderscores) {
          try {
            _parse(release, source);
            expect(floatOk && !early, isTrue,
                reason: '$release must not lex $source');
          } on PythonLexError {
            expect(!floatOk || early, isTrue, reason: '$release: $source');
          } on PythonParseError {
            expect(!floatOk || early, isTrue, reason: '$release: $source');
          } on PythonFrontendError catch (e) {
            fail('$release: $source: $e');
          }
          checked++;
        }
        for (final source in [...negativeAll, ...floatNegative]) {
          try {
            _parse(release, source);
            fail('$release: $source accepted');
          } on PythonLexError {
            // Expected.
          } on PythonParseError {
            // Expected.
          } on PythonFrontendError catch (e) {
            fail('$release: $source: $e');
          }
          checked++;
        }
      }
      expect(
          checked,
          equals(releases.length *
              (positiveAll.length +
                  underscores.length +
                  floats.length +
                  floatUnderscores.length +
                  negativeAll.length +
                  floatNegative.length)));
    });

    test('py2 int print probes for every python 2 release', () {
      const positive = [
        'print 1 + 2',
        'print 3 * 4',
        'print -7 / 2',
        'print -7 // 2',
        'print -7 % 2',
        'print',
        'print (2)',
        'print (1 + 2) * 3',
        'print - 3',
        'print +-+3',
        'print 2 * -3',
        'print (1 +\n2 *\n3)',
        'print 1; print 2',
        'print 1;',
        '\n\nprint 1\n\n',
        '# leading comment\nprint 1 # comment\nprint 2\n',
        'print 1 + \\\n 2',
        'print 0',
        'print 9223372036854775807',
        'print 0x10',
        'print 010',
        'print 00',
        'print 1L',
        'print 0XffL - 07l',
        'print 9223372036854775808',
        'print 1, 2',
        'print 1,',
        'abs(1)',
        '1 + 2',
        'print f(1)',
      ];
      const negative = [
        'print(',
        'print (1',
        'print 1 print 2',
        'print 1;;',
        ';print 1',
        'print 1 +',
        'print 08',
        'print 0x',
        'print 1LL',
        'print 0x1Fg',
        'print 1_000',
        'print 1.5',
        'print \'a\'',
        'print >>f, 1',
        'print ()',
        'print (1, 2)',
        'print 2 ** 3',
        'print ~1',
        'print 1 <> 2',
        'print `1`',
        'print x[0]',
        'print 1,,',
        'x = 1',
        '  print 1',
        'if 1:\n    print 1',
        'print 1 \\',
      ];
      var checked = 0;
      for (final release in _py2Releases) {
        for (final source in positive) {
          try {
            final ast = _parse(release, source).ast;
            expect(ast.ruleName, equals('file'), reason: '$release: $source');
          } on PythonFrontendError catch (e) {
            fail('$release: $source rejected: $e');
          }
          checked++;
        }
        for (final source in negative) {
          try {
            _parse(release, source);
            fail('$release: $source accepted');
          } on PythonLexError {
            // Expected.
          } on PythonParseError {
            // Expected.
          } on PythonFrontendError catch (e) {
            fail('$release: $source: $e');
          }
          checked++;
        }
        // `print(1)` is the statement `print (1)`: one `print_stmt`
        // node and no call suffix.
        final ast = _parse(release, 'print(1)').ast;
        final stack = [ast];
        var statements = 0;
        var suffixes = 0;
        while (stack.isNotEmpty) {
          final node = stack.removeLast();
          if (node.ruleName == 'print_stmt') statements++;
          if (node.ruleName == 'suffix') suffixes++;
          for (final child in node.children) {
            if (child is ASTNode) stack.add(child);
          }
        }
        expect((statements, suffixes), equals((1, 0)), reason: release);
      }
      expect(checked,
          equals(_py2Releases.length * (positive.length + negative.length)));
      // PEP 3127 (2.6): `0o`/`0b` parse from 2.6 on and are lexer
      // errors in the 2.2 and 2.4 pairs.
      for (final source in ['print 0o17', 'print 0b101L + 0O7']) {
        for (final release in ['2.2', '2.4']) {
          expect(_parseOrNull(release, source), isNull,
              reason: '$release $source');
          try {
            _parse(release, source);
            fail('expected Lex');
          } on PythonLexError {
            // Expected.
          }
        }
        for (final release in ['2.6', '2.7.18']) {
          expect(_parseOrNull(release, source), isNotNull,
              reason: '$release $source');
        }
      }
      // A Python 2 program is not Python 3 (and vice versa).
      expect(_parseOrNull('3.13.16', 'print 1'), isNull);
      expect(_parseOrNull('2.7.18', 'print(1, end=None)'), isNull);
    });

    test('per release operator sets follow the tokenizer', () {
      const budgets = FrontendBudgets();
      bool lexes(String release, String source) {
        try {
          tokenizePythonAt(source, release, budgets);
          return true;
        } on PythonFrontendError {
          return false;
        }
      }

      List<String> kinds(String release, String source) {
        final (_, tokens) = tokenizePythonAt(source, release, budgets);
        return [for (final t in tokens) t.type];
      }

      expect(
          kinds('3.0a1', 'a := 1').sublist(1, 3), equals(['COLON', 'EQUALS']));
      expect(kinds('3.11.17', 'a := 1')[1], equals('WALRUS'));
      expect(kinds('3.0a1', 'a @= 1').sublist(1, 3), equals(['AT', 'EQUALS']));
      expect(kinds('3.11.17', 'a @= 1')[1], equals('AT_EQUALS'));
      expect(!lexes('3.11.17', 'a ! b') && lexes('3.12.3', 'a ! b'), isTrue);
      for (final release in ['3.5.10', '3.6.0']) {
        expect(kinds(release, 'a @= 1')[1], equals('AT_EQUALS'),
            reason: release);
        expect(kinds(release, '1 <> 2')[1], equals('LEGACY_NOT_EQUALS'),
            reason: release);
        try {
          _parse(release, 'print(1 <> 2)');
          fail('expected Parse');
        } on PythonParseError {
          // Expected.
        }
      }
      String kind(String release) {
        final (_, tokens) = tokenizePythonAt('async\n', release, budgets);
        return tokens[0].type;
      }

      expect(kind('3.0a1'), equals('NAME'));
      expect(kind('3.13.16'), equals('KEYWORD'));
      for (final release in supportedPythonParserReleases) {
        final (_, tokens) = tokenizePythonAt('print\n', release, budgets);
        expect(tokens[0].type,
            equals(_py2Releases.contains(release) ? 'KEYWORD' : 'NAME'),
            reason: release);
      }
      expect(kinds('2.2', '1 <> 2')[1], equals('LEGACY_NOT_EQUALS'));
      expect(kinds('2.7.18', '`1`')[0], equals('BACKTICK'));
      expect(lexes('3.0a1', '`1`'), isFalse);
      expect(!lexes('2.2', '@') && lexes('2.4', '@') && lexes('2.7.18', '@'),
          isTrue);
      expect(kinds('2.2', 'with')[0], equals('NAME'));
      expect(kinds('2.4', 'with')[0], equals('NAME'));
      expect(kinds('2.6', 'with')[0], equals('KEYWORD'));
      expect(kinds('2.7.18', 'with')[0], equals('KEYWORD'));
    });

    test('legacy minor line api is unchanged', () {
      expect(parsePython('x = 1\n', '3.12').ruleName, equals('file'));
      expect(() => parsePython('print(1)\n', '3.13.16'), throwsArgumentError);
    });

    test('hostile probes return errors in process', () {
      // The reference runs these in a child process so a stack
      // overflow or abort shows up as a failed child; Dart has no
      // thread-stack tuning and no aborting overflow, so they run
      // in-process instead. A hung or crashed run fails the suite
      // the same way a failed child fails the harness.
      const budgets = FrontendBudgets();
      for (final release in supportedPythonParserReleases) {
        // Deep nesting stops at the parser's depth guard.
        final deep = 'print(${'(' * 5000}1${')' * 5000})';
        try {
          parsePythonAt(deep, release, budgets);
          fail('$release: deep input parsed');
        } on PythonBudgetExceeded catch (e) {
          expect(e.which, equals('parser rule depth'));
        }
        // The release grammars admit far more nesting than the
        // legacy 3.12 grammar's 9 levels under the same cap.
        var admitted = 0;
        for (var depth = 1; depth < 60; depth++) {
          final source = 'print(${'(' * depth}1${')' * depth})';
          try {
            parsePythonAt(source, release, budgets);
            admitted = depth;
          } on PythonBudgetExceeded {
            break;
          } on PythonFrontendError catch (e) {
            fail('$release depth $depth: $e');
          }
        }
        expect(admitted, greaterThanOrEqualTo(15),
            reason: '$release: $admitted');
        // Oversized input is refused before lexing.
        final big = 'print(${'1' * (64 * 1024)})';
        try {
          parsePythonAt(big, release, budgets);
          fail('$release: oversized input parsed');
        } on PythonBudgetExceeded catch (e) {
          expect(e.which, equals('source bytes'));
          expect(e.limit, equals(65536));
        }
        // A long flat operator chain parses or fails cleanly; it must
        // not recurse per operator.
        final chain = 'print(1${' + 1' * 7000})';
        expect(chain.length, lessThan(64 * 1024));
        _parseOrNull(release, chain);
        // A long unary chain is recursion, so it stops at the depth
        // guard.
        final unary = 'print(${'-' * 5000}1)';
        expect(_parseOrNull(release, unary), isNull, reason: release);
        // Unterminated constructs and junk are ordinary errors.
        for (final junk in [
          'print(1 + ',
          'print((((',
          '\u{0}\u{1}',
          '€€€',
          'print(1) \\',
          '\t print(1)'
        ]) {
          expect(_parseOrNull(release, junk), isNull,
              reason: '$release: $junk');
        }
        // A trusted caller may raise the source budget, never the
        // depth cap.
        const trusted = FrontendBudgets(
            maxSourceBytes: 9223372036854775807,
            maxRuleDepth: 9223372036854775807,
            maxTokens: 9223372036854775807);
        try {
          parsePythonAt(deep, release, trusted);
          fail('$release: deep input parsed under trusted budgets');
        } on PythonBudgetExceeded catch (e) {
          expect(e.which, equals('parser rule depth'));
          expect(e.limit, equals(190));
        }
      }
    });

    test('host releases lex exactly as cpython tokenize', () {
      const hostReleases = ['3.11.17', '3.12.3', '3.13.16'];
      for (final entry in _tokenizeCases.entries) {
        for (final release in hostReleases) {
          expect(_stream(release, entry.value.$1), equals(entry.value.$2),
              reason: '$release: ${entry.key}');
        }
      }
    });

    test('the newest release lexes like the default', () {
      for (final entry in _tokenizeCases.entries) {
        expect(_stream('3.16.0a1', entry.value.$1),
            equals(_stream('3.13.16', entry.value.$1)),
            reason: entry.key);
      }
    });

    test('release 3.0a1 differs only where its tokenizer did', () {
      for (final entry in _tokenizeCases.entries) {
        final source = entry.value.$1;
        final expected = entry.value.$2;
        switch (entry.key) {
          case '02_numbers':
            try {
              _stream('3.0a1', source);
              fail('expected Lex');
            } on PythonLexError catch (e) {
              expect(e.line, equals(1));
            }
          case '03_operators':
            final ours = _stream('3.0a1', source);
            expect(ours.contains('COLON\t4\t3\t:\nEQUALS\t4\t4\t='), isTrue);
            expect(ours.contains('AT\t3\t67\t@\nEQUALS\t3\t68\t='), isTrue);
            expect(ours.contains('WALRUS'), isFalse);
            expect(ours.contains('AT_EQUALS'), isFalse);
          case '04_keywords':
            final ours = _stream('3.0a1', source);
            final renamed = expected
                .replaceAll('KEYWORD\t1\t31\tasync', 'NAME\t1\t31\tasync')
                .replaceAll('KEYWORD\t1\t37\tawait', 'NAME\t1\t37\tawait');
            expect(renamed, isNot(equals(expected)));
            expect(ours, equals(renamed));
          default:
            expect(_stream('3.0a1', source), equals(expected),
                reason: entry.key);
        }
      }
    });

    test('budgets compare by value and clamp to ceilings', () {
      const def = FrontendBudgets();
      expect(def.maxSourceBytes, equals(65536));
      expect(def.maxRuleDepth, equals(190));
      expect(def.maxTokens, equals(16384));
      expect(def, equals(const FrontendBudgets()));
      expect(def.hashCode, equals(const FrontendBudgets().hashCode));
      expect(def, isNot(equals(const FrontendBudgets(maxTokens: 1))));
      expect(def.sourceLimit, equals(65536));
      expect(def.tokenLimit, equals(16384));
      expect(def.depthLimit, equals(190));
      const huge = FrontendBudgets(
          maxSourceBytes: 9223372036854775807,
          maxRuleDepth: 9223372036854775807,
          maxTokens: 9223372036854775807);
      expect(huge.sourceLimit, equals(1048576));
      expect(huge.tokenLimit, equals(262144));
      expect(huge.depthLimit, equals(190));
      const small = FrontendBudgets(maxSourceBytes: 8, maxTokens: 8);
      expect(small.sourceLimit, equals(8));
      expect(small.tokenLimit, equals(8));
    });

    test('errors compare by value and render exactly', () {
      const unknown =
          PythonUnknownRelease(requested: 'x', suggestions: ['y', 'z']);
      expect(
          unknown,
          equals(const PythonUnknownRelease(
              requested: 'x', suggestions: ['y', 'z'])));
      expect(
          unknown.hashCode,
          equals(const PythonUnknownRelease(
              requested: 'x', suggestions: ['y', 'z']).hashCode));
      expect(
          unknown,
          isNot(equals(
              const PythonUnknownRelease(requested: 'x', suggestions: ['y']))));
      expect(unknown.toString(),
          equals('unknown Python release "x" (did you mean y, z?)'));
      expect(
          const PythonUnknownRelease(requested: '', suggestions: []).toString(),
          equals('unknown Python release ""'));
      const missing = PythonReleaseNotAvailable(release: 'r', stage: 'none');
      expect(missing,
          equals(const PythonReleaseNotAvailable(release: 'r', stage: 'none')));
      expect(
          missing.hashCode,
          equals(const PythonReleaseNotAvailable(release: 'r', stage: 'none')
              .hashCode));
      expect(
          missing,
          isNot(equals(
              const PythonReleaseNotAvailable(release: 'r', stage: 'P'))));
      expect(missing.toString(),
          equals('Python r has no grammar pair yet (grammar_stage none)'));
      const budget =
          PythonBudgetExceeded(release: 'r', which: 'tokens', limit: 8);
      expect(
          budget,
          equals(const PythonBudgetExceeded(
              release: 'r', which: 'tokens', limit: 8)));
      expect(
          budget.hashCode,
          equals(const PythonBudgetExceeded(
                  release: 'r', which: 'tokens', limit: 8)
              .hashCode));
      expect(
          budget,
          isNot(equals(const PythonBudgetExceeded(
              release: 'r', which: 'tokens', limit: 9))));
      expect(
          budget.toString(), equals('Python r: tokens exceeds the limit of 8'));
      const lex =
          PythonLexError(release: 'r', line: 1, column: 2, message: 'm');
      expect(
          lex,
          equals(const PythonLexError(
              release: 'r', line: 1, column: 2, message: 'm')));
      expect(
          lex.hashCode,
          equals(const PythonLexError(
                  release: 'r', line: 1, column: 2, message: 'm')
              .hashCode));
      expect(
          lex,
          isNot(equals(const PythonLexError(
              release: 'r', line: 1, column: 3, message: 'm'))));
      expect(lex.toString(), equals('Python r lex error at 1:2: m'));
      const parse = PythonParseError(
          release: 'r', line: 3, column: 4, message: 'Expected X');
      expect(
          parse,
          equals(const PythonParseError(
              release: 'r', line: 3, column: 4, message: 'Expected X')));
      expect(
          parse.hashCode,
          equals(const PythonParseError(
                  release: 'r', line: 3, column: 4, message: 'Expected X')
              .hashCode));
      expect(
          parse,
          isNot(equals(const PythonParseError(
              release: 'r', line: 3, column: 4, message: 'Expected Y'))));
      expect(
          parse.toString(), equals('Python r parse error at 3:4: Expected X'));
      const different =
          PythonLexError(release: 'r', line: 1, column: 2, message: 'm');
      expect(parse, isNot(equals(different)));
    });

    test('echo escapes every control shape', () {
      String requested(String id) {
        try {
          resolvePythonRelease(id);
          fail('expected UnknownRelease for $id');
        } on PythonUnknownRelease catch (e) {
          return e.requested;
        }
      }

      expect(requested('a\tb\rc\nd\\e\'f"g'),
          equals('a\\tb\\rc\\nd\\\\e\\\'f\\"g'));
      expect(requested('\x00\x1f\x7f'), equals(r'\u{0}\u{1f}\u{7f}'));
      expect(requested('\x85\x9f'), equals(r'\u{85}\u{9f}'));
      expect(requested(String.fromCharCode(0xD800)), equals(r'\u{d800}'));
    });

    test('truncate respects rune boundaries and widths', () {
      String requested(String id) {
        try {
          resolvePythonRelease(id);
          fail('expected UnknownRelease');
        } on PythonUnknownRelease catch (e) {
          return e.requested;
        }
      }

      // 40 two-byte chars = 80 bytes > 64: kept whole chars only.
      final twoByte = 'é' * 40;
      expect(requested(twoByte), equals('${'é' * 32}...'));
      // 20 astral chars = 80 bytes: kept whole runes only.
      final astral = '🎉' * 20;
      expect(requested(astral), equals('${'🎉' * 16}...'));
      // Mixed widths: 63 ASCII + one 2-byte char would exceed.
      final mixed = '${'a' * 63}é';
      expect(requested(mixed), equals('${'a' * 63}...'));
      // Exactly at the cap echoes whole, without the marker.
      final exact = requested('9' * 64);
      expect(exact.length, equals(64));
      expect(exact.endsWith('...'), isFalse);
    });

    test('lone surrogates cannot crash id handling', () {
      final lone = String.fromCharCode(0xD800);
      try {
        resolvePythonRelease('3.12$lone');
        fail('expected UnknownRelease');
      } on PythonUnknownRelease catch (e) {
        expect(e.requested, contains(r'\u{d800}'));
      }
      const budgets = FrontendBudgets();
      try {
        tokenizePythonAt('print(1)', '3.1$lone', budgets);
        fail('expected UnknownRelease');
      } on PythonUnknownRelease {
        // Expected (prefix '3.1' matches nothing... or suggests).
      }
    });

    test('token budget refuses dense input before parsing', () {
      const budgets = FrontendBudgets();
      for (final release in ['3.0a1', '3.13.16']) {
        // `print(1+1+…)`: two tokens per two bytes, well inside 64 KiB.
        final dense = 'print(1${'+1' * 20000})';
        expect(dense.length, lessThan(defaultMaxSourceBytes));
        try {
          parsePythonAt(dense, release, budgets);
          fail('$release: dense input parsed');
        } on PythonBudgetExceeded catch (e) {
          expect(e.which, equals('tokens'));
          expect(e.limit, equals(defaultMaxTokens));
        }
        const trusted = FrontendBudgets(maxTokens: 9223372036854775807);
        final (_, tokens) = tokenizePythonAt(dense, release, trusted);
        expect(tokens.length, greaterThan(defaultMaxTokens));
        expect(tokens.length, lessThanOrEqualTo(maxTrustedTokens));
      }
      expect(maxTrustedSourceBytes, equals(1024 * 1024));
    });

    test('every error names the release and a one-based position', () {
      try {
        _parse('3.13.16', "print(1)\nprint('x')");
        fail('expected PythonLexError');
      } on PythonLexError catch (e) {
        expect(e.release, equals('3.13.16'));
        expect(e.line, equals(2));
        expect(e.column, equals(7));
        // The Dart engine is payload-blind by standing rule, so its
        // message carries no offending character (the reference pins
        // "Unexpected sequence '\''"); the release and 1-based
        // position are identical.
        expect(e.message, equals('Unexpected character.'));
      }
      try {
        _parse('3.0a1', 'print(1)\nprint 2');
        fail('expected PythonParseError');
      } on PythonParseError catch (e) {
        expect(e.release, equals('3.0a1'));
        expect(e.line, equals(2));
        expect(e.column, greaterThanOrEqualTo(1));
        expect(e.message.startsWith('Expected '), isTrue, reason: e.message);
        expect(
            e.toString().startsWith('Python 3.0a1 parse error at 2:'), isTrue);
      }
    });
  });
}
