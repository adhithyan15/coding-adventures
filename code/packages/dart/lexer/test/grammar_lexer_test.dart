import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_lexer/src/messages.dart';
import 'package:test/test.dart';

void main() {
  group('grammarTokenize', () {
    test('tokenizes a minimal grammar with skip patterns', () {
      final grammar = parseTokenGrammar('''
NUMBER = /[0-9]+/
PLUS = "+"
skip:
  SPACE = /[ \t]+/
''');

      final tokens = grammarTokenize('12 + 34', grammar);
      expect(tokens.map((token) => token.type).toList(), [
        'NUMBER',
        'PLUS',
        'NUMBER',
        'EOF',
      ]);
      expect(tokens.map((token) => token.value).toList(), [
        '12',
        '+',
        '34',
        '',
      ]);
    });

    test('respects aliases and string unescaping by default', () {
      final grammar = parseTokenGrammar(
        'STRING_DQ = /"([^"\\\\]|\\\\.)*"/ -> STRING',
      );

      final tokens = grammarTokenize(r'"line1\nline2"', grammar);
      expect(tokens.first.type, 'STRING');
      expect(tokens.first.value, 'line1\nline2');
    });

    test('keeps raw string escapes when escapes none is enabled', () {
      final grammar = parseTokenGrammar('''
escapes: none
STRING = /"([^"\\\\]|\\\\.)*"/
''');

      final tokens = grammarTokenize(r'"line1\nline2"', grammar);
      expect(tokens.first.value, r'line1\nline2');
    });

    test('strips single-quoted and named string delimiters', () {
      final grammar = parseTokenGrammar('''
escapes: none
LITERAL_STRING = /'[^']*'/
''');

      final token = grammarTokenize("'hello'", grammar).first;
      expect(token.type, 'LITERAL_STRING');
      expect(token.value, 'hello');
    });

    test('strips triple delimiters from multiline string types', () {
      final grammar = parseTokenGrammar(r'''
escapes: none
ML_BASIC_STRING = /"""[\s\S]*?"""/
''');

      final token = grammarTokenize('"""hello\nworld"""', grammar).first;
      expect(token.type, 'ML_BASIC_STRING');
      expect(token.value, 'hello\nworld');
    });

    test('emits NEWLINE when newline is not skipped', () {
      final grammar = parseTokenGrammar('NUMBER = /[0-9]+/');

      final tokens = grammarTokenize('1\n2', grammar);
      expect(tokens.map((token) => token.type).toList(), [
        'NUMBER',
        'NEWLINE',
        'NUMBER',
        'EOF',
      ]);
    });
  });

  group('grammarTokenize linear-time matching (CWE-400)', () {
    test('lexes 256 KB of repeated tokens well inside the time bound', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
PLUS = "+"
''');
      final source = 'a+' * (128 * 1024);
      expect(source.length, equals(256 * 1024));

      final stopwatch = Stopwatch()..start();
      final tokens = grammarTokenize(source, grammar);
      stopwatch.stop();

      expect(tokens.length, equals(2 * 128 * 1024 + 1));
      expect(tokens.last.type, equals('EOF'));
      expect(tokens.last.column, equals(source.length + 1));
      expect(
        stopwatch.elapsed,
        lessThan(const Duration(seconds: 5)),
        reason: 'Lexing must be linear; substring-per-attempt was quadratic.',
      );
    });

    test('literal tokens match without copying the remaining source', () {
      final grammar = parseTokenGrammar('''
PLUS = "+"
MINUS = "-"
''');
      final tokens = grammarTokenize('+-' * (64 * 1024), grammar);
      expect(tokens.length, equals(2 * 64 * 1024 + 1));
      expect(tokens[1].type, equals('MINUS'));
    });
  });

  group('grammarTokenize astral code points', () {
    test('advances UTF-16 position per code point but column per code point',
        () {
      final grammar = parseTokenGrammar('''
STRING = /"[^"]*"/
PLUS = "+"
NAME = /[a-z]+/
''');

      final tokens = grammarTokenize('"\u{1F600}"+x', grammar);
      expect(tokens.map((t) => t.type).toList(),
          ['STRING', 'PLUS', 'NAME', 'EOF']);
      expect(tokens[0].value, equals('\u{1F600}'));
      expect(tokens.map((t) => t.column).toList(), [1, 4, 5, 6]);
    });

    test('consecutive astral tokens and a token after each one', () {
      final grammar = parseTokenGrammar('''
EMOJI = /[\u{1F600}-\u{1F64F}]/
NAME = /[a-z]+/
''');

      final tokens = grammarTokenize('\u{1F600}\u{1F601}a\u{1F602}b', grammar);
      expect(tokens.map((t) => t.value).toList(),
          ['\u{1F600}', '\u{1F601}', 'a', '\u{1F602}', 'b', '']);
      expect(tokens.map((t) => t.column).toList(), [1, 2, 3, 4, 5, 6]);
    });

    test('skip patterns may consume astral code points', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
skip:
  FILL = /[ \u{1F600}]+/
''');

      final tokens = grammarTokenize('a \u{1F600} \u{1F600}b', grammar);
      expect(tokens.map((t) => t.value).toList(), ['a', 'b', '']);
      expect(tokens[1].column, equals(6));
    });

    test('lone surrogate fails with a typed, payload-blind error', () {
      final grammar = parseTokenGrammar('NAME = /[a-z]+/');
      expect(
        () => grammarTokenize('a\uD83D', grammar),
        throwsA(
          isA<LexerError>()
              .having((e) => e.message, 'message', messageUnexpectedCharacter)
              .having((e) => e.column, 'column', 2),
        ),
      );
    });
  });

  group('grammarTokenize diagnostics (Standing Rule #20)', () {
    test('unexpected character error never echoes the character', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
skip:
  SPACE = /[ ]+/
''');
      try {
        grammarTokenize('abc Z', grammar);
        fail('Expected LexerError');
      } on LexerError catch (e) {
        expect(e.message, equals(messageUnexpectedCharacter));
        expect(e.errorId, equals(errorIdLexicalError));
        expect(e.line, equals(1));
        expect(e.column, equals(5));
        expect(e.toString().contains('Z'), isFalse);
        expect(
          e.toString(),
          equals(formatLexerError(messageUnexpectedCharacter, 1, 5)),
        );
      }
    });

    test('reserved keyword error never echoes the keyword', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
reserved:
  secretword
''');
      try {
        grammarTokenize('secretword', grammar);
        fail('Expected LexerError');
      } on LexerError catch (e) {
        expect(e.message, equals(messageReservedKeyword));
        expect(e.toString().contains('secretword'), isFalse);
        expect(e.column, equals(1));
      }
    });
  });

  group('grammarTokenize literal matching', () {
    test('case-sensitive literals only match exact case', () {
      final grammar = parseTokenGrammar('''
ARROW = "->"
NAME = /[a-z]+/
''');
      expect(
        grammarTokenize('a->b', grammar).map((t) => t.type).toList(),
        ['NAME', 'ARROW', 'NAME', 'EOF'],
      );
    });

    test('case-insensitive literals match any case and stop at end', () {
      final grammar = parseTokenGrammar('''
# @case_insensitive true
ARROW = "ab"
skip:
  SPACE = /[ ]+/
''');
      final tokens = grammarTokenize('AB aB', grammar);
      expect(tokens.map((t) => t.value).toList(), ['AB', 'aB', '']);
      expect(() => grammarTokenize('a', grammar), throwsA(isA<LexerError>()));
    });

    test('regexes see the real preceding text (context-aware word boundary)',
        () {
      final grammar = parseTokenGrammar('''
X = /x/
FOO = /\\bfoo/
''');
      expect(
        grammarTokenize('foo', grammar).map((t) => t.type).toList(),
        ['FOO', 'EOF'],
      );
      expect(
          () => grammarTokenize('xfoo', grammar), throwsA(isA<LexerError>()));
    });
  });

  group('grammarTokenize string escape decoding', () {
    test('decodes every supported escape and keeps unknown ones literal', () {
      final grammar = parseTokenGrammar(
        r'STRING = /"([^"\\]|\\.)*"/',
      );
      final tokens = grammarTokenize(
        r'"a\n\t\r\b\f\"\\\/\u0041\uZZZZ\q!"',
        grammar,
      );
      expect(
        tokens.first.value,
        equals('a\n\t\r\b\f"\\/A\\uZZZZq!'),
      );
    });

    test('keeps a trailing backslash when nothing follows it', () {
      final grammar = parseTokenGrammar('LIT = /"[^"]*"/ -> STRING');
      expect(grammarTokenize(r'"a\"', grammar).first.value, equals(r'a\'));
    });
  });

  group('grammarTokenize mode: indentation', () {
    final indentationGrammar = parseTokenGrammar('''
mode: indentation
NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
INT = /[0-9]+/
STRING = /"([^"\\\\\\n]|\\\\.)*"/
LPAREN = "("
RPAREN = ")"
LBRACK = "["
RBRACK = "]"
LBRACE = "{"
RBRACE = "}"
COLON = ":"
EQUALS = "="
COMMA = ","
skip:
  COMMENT = /#[^\\n]*/
  SPACE = /[ ]+/
''');

    test('tokenizes nested indentation blocks and multi-level dedents', () {
      const source = '''
def foo():
    if 1:
        x = 1
y = 2
''';
      final tokens = grammarTokenize(source, indentationGrammar);
      final types = tokens.map((t) => t.type).toList();
      expect(types, [
        'NAME',
        'NAME',
        'LPAREN',
        'RPAREN',
        'COLON',
        'NEWLINE',
        'INDENT',
        'NAME',
        'INT',
        'COLON',
        'NEWLINE',
        'INDENT',
        'NAME',
        'EQUALS',
        'INT',
        'NEWLINE',
        'DEDENT',
        'DEDENT',
        'NAME',
        'EQUALS',
        'INT',
        'NEWLINE',
        'EOF',
      ]);
    });

    test('skips blank and comment-only lines without emitting indent/dedent',
        () {
      const source = '''
x = 1

    # indented comment
y = 2
''';
      final tokens = grammarTokenize(source, indentationGrammar);
      final types = tokens.map((t) => t.type).toList();
      expect(types, [
        'NAME',
        'EQUALS',
        'INT',
        'NEWLINE',
        'NAME',
        'EQUALS',
        'INT',
        'NEWLINE',
        'EOF',
      ]);
    });

    test('suppresses newlines and ignores indentation inside brackets', () {
      const source = '''
items = [
    1,
    2,
]
''';
      final tokens = grammarTokenize(source, indentationGrammar);
      final types = tokens.map((t) => t.type).toList();
      expect(types, [
        'NAME',
        'EQUALS',
        'LBRACK',
        'INT',
        'COMMA',
        'INT',
        'COMMA',
        'RBRACK',
        'NEWLINE',
        'EOF',
      ]);
    });

    test(
        'flushes trailing NEWLINE and DEDENTs at EOF when trailing newline is missing',
        () {
      const source = 'if 1:\n    x = 1';
      final tokens = grammarTokenize(source, indentationGrammar);
      final types = tokens.map((t) => t.type).toList();
      expect(types, [
        'NAME',
        'INT',
        'COLON',
        'NEWLINE',
        'INDENT',
        'NAME',
        'EQUALS',
        'INT',
        'NEWLINE',
        'DEDENT',
        'EOF',
      ]);
    });

    test('handles empty input and whitespace-only input', () {
      expect(
          grammarTokenize('', indentationGrammar).map((t) => t.type).toList(),
          ['EOF']);
      expect(
          grammarTokenize('   \n  \n', indentationGrammar)
              .map((t) => t.type)
              .toList(),
          ['EOF']);
      expect(
          grammarTokenize('# comment\n', indentationGrammar)
              .map((t) => t.type)
              .toList(),
          ['EOF']);
    });

    test('skips inter-token whitespace without a WHITESPACE skip', () {
      // The reference's built-in whitespace rule applies always in
      // indentation mode; grammars without a whitespace skip (every
      // python*.tokens) rely on it. Mid-line tabs and carriage
      // returns are skipped too — only indentation-position tabs
      // are an error.
      final noWhitespaceSkip = parseTokenGrammar('''
mode: indentation
NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
INT = /[0-9]+/
EQUALS = "="
skip:
  COMMENT = /#[^\\n]*/
''');
      final tokens = grammarTokenize('x = 1\r\ny\t=\t2\n', noWhitespaceSkip);
      expect(tokens.map((t) => t.type).toList(), [
        'NAME',
        'EQUALS',
        'INT',
        'NEWLINE',
        'NAME',
        'EQUALS',
        'INT',
        'NEWLINE',
        'EOF',
      ]);
    });

    test('rejects tab character in leading indentation', () {
      const source = '\tx = 1\n';
      expect(
        () => grammarTokenize(source, indentationGrammar),
        throwsA(
          isA<LexerError>().having(
            (e) => e.message,
            'message',
            contains(messageTabInIndentation),
          ),
        ),
      );
    });

    test('rejects inconsistent dedent', () {
      const source = '''
if 1:
    x = 1
  y = 2
''';
      expect(
        () => grammarTokenize(source, indentationGrammar),
        throwsA(
          isA<LexerError>().having(
            (e) => e.message,
            'message',
            contains(messageInconsistentDedent),
          ),
        ),
      );
    });

    test(
        'does not treat bracket characters inside string literals as delimiters',
        () {
      const source = '''
x = "("
y = 1
''';
      final tokens = grammarTokenize(source, indentationGrammar);
      final types = tokens.map((t) => t.type).toList();
      expect(types, [
        'NAME',
        'EQUALS',
        'STRING',
        'NEWLINE',
        'NAME',
        'EQUALS',
        'INT',
        'NEWLINE',
        'EOF',
      ]);
      expect(tokens[2].value, '(');
    });

    test('rejects unexpected character in indentation mode', () {
      const source = '?';
      expect(
        () => grammarTokenize(source, indentationGrammar),
        throwsA(
          isA<LexerError>().having(
            (e) => e.message,
            'message',
            contains(messageUnexpectedCharacter),
          ),
        ),
      );
    });
  });
}
