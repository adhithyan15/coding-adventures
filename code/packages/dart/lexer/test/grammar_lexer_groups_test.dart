/// Tests for the [GrammarLexer] group/callback engine: mirrors of the
/// reference engine's context and group tests, plus gap-flag, lookbehind,
/// peek, keyword, string, error-vocabulary, and reuse extras.
library;

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

TokenDefinition _def(String name, String pattern, bool isRegex,
        {String? alias}) =>
    TokenDefinition(
        name: name,
        pattern: pattern,
        isRegex: isRegex,
        lineNumber: 1,
        alias: alias);

/// The reference `make_group_grammar` in Dart form.
TokenGrammar makeGroupGrammar() => TokenGrammar(
      escapeMode: 'none',
      skipDefinitions: [_def('WS', r'[ \t\r\n]+', true)],
      definitions: [
        _def('TEXT', r'[^<]+', true),
        _def('OPEN_TAG', '<', false),
      ],
      groups: {
        'tag': PatternGroup(name: 'tag', definitions: [
          _def('TAG_NAME', r'[a-zA-Z_][a-zA-Z0-9_]*', true),
          _def('EQUALS', '=', false),
          _def('VALUE', r'"[^"]*"', true),
          _def('TAG_CLOSE', '>', false),
        ]),
      },
    );

/// A fresh context over [source] as if a token just ended at [pos].
LexerContext makeContext(String source, int pos,
        {Set<String>? groups, List<String>? stack, Token? previous}) =>
    LexerContext(
      groupNames: groups ?? {'default', 'tag'},
      groupStack: stack ?? ['default'],
      source: source,
      posAfterToken: pos,
      previousToken: previous,
    );

/// (type, value) pairs excluding EOF.
List<(String, String)> pairs(List<Token> tokens) =>
    tokens.where((t) => t.type != 'EOF').map((t) => (t.type, t.value)).toList();

/// Types excluding EOF.
List<String> types(List<Token> tokens) =>
    pairs(tokens).map((p) => p.$1).toList();

/// The tag push/pop callback shared by several tests.
void tagCallback(Token token, LexerContext ctx) {
  if (token.type == 'OPEN_TAG') {
    ctx.pushGroup('tag');
  } else if (token.type == 'TAG_CLOSE') {
    ctx.popGroup();
  }
}

void main() {
  group('LexerContext units', () {
    test('push records a Push action', () {
      final ctx = makeContext('x', 1);
      ctx.pushGroup('tag');
      expect(ctx.actions, [const PushGroup('tag')]);
    });

    test('push of unknown group throws ArgumentError', () {
      final ctx = makeContext('x', 1);
      expect(
          () => ctx.pushGroup('nonexistent'),
          throwsA(predicate((e) =>
              e is ArgumentError &&
              (e.message as String).contains('Unknown pattern group'))));
    });

    test('push error names the group and sorted available groups', () {
      final ctx = makeContext('x', 1, groups: {'default', 'tag', 'comment'});
      try {
        ctx.pushGroup('nope');
        fail('expected ArgumentError');
      } on ArgumentError catch (e) {
        expect(
            e.message,
            'Unknown pattern group: "nope". Available groups: '
            '["comment", "default", "tag"]');
      }
    });

    test('pop records a Pop action', () {
      final ctx = makeContext('x', 1);
      ctx.popGroup();
      expect(ctx.actions, [const PopGroup()]);
    });

    test('activeGroup reads the stack top', () {
      expect(makeContext('x', 1).activeGroup, 'default');
      expect(makeContext('x', 1, stack: ['default', 'tag']).activeGroup, 'tag');
    });

    test('groupStackDepth reads the stack length', () {
      expect(makeContext('x', 1).groupStackDepth, 1);
      expect(makeContext('x', 1, stack: ['default', 'tag']).groupStackDepth, 2);
    });

    test('emit records an Emit action', () {
      final ctx = makeContext('x', 1);
      const synthetic =
          Token(type: 'MARKER', value: '[start]', line: 1, column: 1);
      ctx.emit(synthetic);
      expect(ctx.actions, [const EmitToken(synthetic)]);
    });

    test('suppress sets the flag', () {
      final ctx = makeContext('x', 1);
      expect(ctx.suppressed, isFalse);
      ctx.suppress();
      expect(ctx.suppressed, isTrue);
    });

    test('peek reads runes past the token', () {
      final ctx = makeContext('hello', 3);
      expect(ctx.peek(1), 'l');
      expect(ctx.peek(2), 'o');
      expect(ctx.peek(3), '');
      expect(ctx.peek(0), '');
      expect(ctx.peek(-2), '');
    });

    test('peek handles astral characters', () {
      final ctx = makeContext('a\u{1F600}b', 1);
      expect(ctx.peek(1), '\u{1F600}');
      expect(ctx.peek(2), 'b');
      expect(ctx.peek(3), '');
    });

    test('peekStr reads a run', () {
      final ctx = makeContext('hello world', 5);
      expect(ctx.peekStr(6), ' world');
      expect(ctx.peekStr(100), ' world');
      expect(ctx.peekStr(0), '');
      expect(ctx.peekStr(-1), '');
    });

    test('setSkipEnabled records the action', () {
      final ctx = makeContext('x', 1);
      ctx.setSkipEnabled(false);
      expect(ctx.actions, [const SetSkipEnabled(false)]);
      ctx.setSkipEnabled(true);
      expect(ctx.actions[1], const SetSkipEnabled(true));
    });

    test('multiple pushes record in order', () {
      final ctx = makeContext('x', 1);
      ctx.pushGroup('tag');
      ctx.pushGroup('tag');
      expect(ctx.actions, [const PushGroup('tag'), const PushGroup('tag')]);
    });

    test('actions compare by value', () {
      expect(const PushGroup('a'), const PushGroup('a'));
      expect(const PushGroup('a'), isNot(const PushGroup('b')));
      expect(const PushGroup('a').hashCode, const PushGroup('a').hashCode);
      expect(const PushGroup('a').toString(), 'PushGroup(a)');
      expect(const PopGroup(), const PopGroup());
      expect(const PopGroup().hashCode, const PopGroup().hashCode);
      expect(const PopGroup().toString(), 'PopGroup()');
      const tok = Token(type: 'T', value: 'v', line: 1, column: 1);
      expect(const EmitToken(tok), const EmitToken(tok));
      expect(const EmitToken(tok).hashCode, const EmitToken(tok).hashCode);
      expect(const EmitToken(tok).toString(), startsWith('EmitToken('));
      expect(const SetSkipEnabled(true), const SetSkipEnabled(true));
      expect(const SetSkipEnabled(true), isNot(const SetSkipEnabled(false)));
      expect(const SetSkipEnabled(true).hashCode,
          const SetSkipEnabled(true).hashCode);
      expect(const SetSkipEnabled(true).toString(), 'SetSkipEnabled(true)');
      expect(const PushGroup('a'), isNot(const PopGroup()));
      expect(const PopGroup(), isNot('not an action'));
    });
  });

  group('groups', () {
    test('no callback uses the default group', () {
      final tokens = GrammarLexer('hello', makeGroupGrammar()).tokenize();
      expect(tokens[0].type, 'TEXT');
      expect(tokens[0].value, 'hello');
    });

    test('callback pushes and pops groups', () {
      final lexer = GrammarLexer('<div>hello', makeGroupGrammar());
      lexer.setOnToken(tagCallback);
      expect(pairs(lexer.tokenize()), [
        ('OPEN_TAG', '<'),
        ('TAG_NAME', 'div'),
        ('TAG_CLOSE', '>'),
        ('TEXT', 'hello'),
      ]);
    });

    test('callback handles attributes', () {
      final lexer = GrammarLexer('<div class="main">', makeGroupGrammar());
      lexer.setOnToken(tagCallback);
      // Divergence note: the reference renders this `("Equals", "=")`
      // through its TokenType enum Display; this lane keeps grammar
      // names verbatim, like the flat engine.
      expect(pairs(lexer.tokenize()), [
        ('OPEN_TAG', '<'),
        ('TAG_NAME', 'div'),
        ('TAG_NAME', 'class'),
        ('EQUALS', '='),
        ('VALUE', '"main"'),
        ('TAG_CLOSE', '>'),
      ]);
    });

    test('nested tags balance pushes and pops', () {
      final grammar = TokenGrammar(
        escapeMode: 'none',
        skipDefinitions: [_def('WS', r'[ \t\r\n]+', true)],
        definitions: [
          _def('TEXT', r'[^<]+', true),
          _def('CLOSE_TAG_START', '</', false),
          _def('OPEN_TAG', '<', false),
        ],
        groups: {
          'tag': PatternGroup(name: 'tag', definitions: [
            _def('TAG_NAME', r'[a-zA-Z_][a-zA-Z0-9_]*', true),
            _def('TAG_CLOSE', '>', false),
            _def('SLASH', '/', false),
          ]),
        },
      );
      final lexer = GrammarLexer('<a>text<b>inner</b></a>', grammar);
      lexer.setOnToken((token, ctx) {
        if (token.type == 'OPEN_TAG' || token.type == 'CLOSE_TAG_START') {
          ctx.pushGroup('tag');
        } else if (token.type == 'TAG_CLOSE') {
          ctx.popGroup();
        }
      });
      expect(pairs(lexer.tokenize()), [
        ('OPEN_TAG', '<'),
        ('TAG_NAME', 'a'),
        ('TAG_CLOSE', '>'),
        ('TEXT', 'text'),
        ('OPEN_TAG', '<'),
        ('TAG_NAME', 'b'),
        ('TAG_CLOSE', '>'),
        ('TEXT', 'inner'),
        ('CLOSE_TAG_START', '</'),
        ('TAG_NAME', 'b'),
        ('TAG_CLOSE', '>'),
        ('CLOSE_TAG_START', '</'),
        ('TAG_NAME', 'a'),
        ('TAG_CLOSE', '>'),
      ]);
    });

    test('suppress omits the token', () {
      final lexer = GrammarLexer('<hello', makeGroupGrammar());
      lexer.setOnToken((token, ctx) {
        if (token.type == 'OPEN_TAG') ctx.suppress();
      });
      expect(types(lexer.tokenize()), ['TEXT']);
    });

    test('emit injects without re-triggering', () {
      final lexer = GrammarLexer('<hello', makeGroupGrammar());
      var calls = 0;
      lexer.setOnToken((token, ctx) {
        calls++;
        if (token.type == 'OPEN_TAG') {
          ctx.emit(Token(
              type: 'MARKER',
              value: '[start]',
              line: token.line,
              column: token.column));
        }
      });
      expect(pairs(lexer.tokenize()), [
        ('OPEN_TAG', '<'),
        ('MARKER', '[start]'),
        ('TEXT', 'hello'),
      ]);
      // OPEN_TAG + TEXT only: the emitted MARKER never re-fires.
      expect(calls, 2);
    });

    test('suppress plus emit replaces the token', () {
      final lexer = GrammarLexer('<hello', makeGroupGrammar());
      lexer.setOnToken((token, ctx) {
        if (token.type == 'OPEN_TAG') {
          ctx.suppress();
          ctx.emit(Token(
              type: 'REPLACED',
              value: '<',
              line: token.line,
              column: token.column));
        }
      });
      expect(pairs(lexer.tokenize()), [
        ('REPLACED', '<'),
        ('TEXT', 'hello'),
      ]);
    });

    test('pop at the bottom is a no-op', () {
      final lexer = GrammarLexer('hello', makeGroupGrammar());
      lexer.setOnToken((_, ctx) => ctx.popGroup());
      final tokens = lexer.tokenize();
      expect(tokens[0].type, 'TEXT');
    });

    test('skip toggle preserves significant whitespace', () {
      final grammar = TokenGrammar(
        escapeMode: 'none',
        skipDefinitions: [_def('WS', r'[ \t]+', true)],
        definitions: [
          _def('TEXT', r'[^<]+', true),
          _def('START', '<!', false),
        ],
        groups: {
          'raw': PatternGroup(name: 'raw', definitions: [
            _def('RAW_TEXT', r'[^>]+', true),
            _def('END', '>', false),
          ]),
        },
      );
      final lexer = GrammarLexer('<! hello world >after', grammar);
      lexer.setOnToken((token, ctx) {
        if (token.type == 'START') {
          ctx.pushGroup('raw');
          ctx.setSkipEnabled(false);
        } else if (token.type == 'END') {
          ctx.popGroup();
          ctx.setSkipEnabled(true);
        }
      });
      expect(pairs(lexer.tokenize()), [
        ('START', '<!'),
        ('RAW_TEXT', ' hello world '),
        ('END', '>'),
        ('TEXT', 'after'),
      ]);
    });

    test('group-less grammars behave flatly', () {
      final grammar = TokenGrammar(definitions: [
        _def('NAME', r'[a-zA-Z_][a-zA-Z0-9_]*', true),
        _def('NUMBER', r'[0-9]+', true),
        _def('PLUS', '+', false),
      ]);
      final lexer = GrammarLexer('x + 1', grammar);
      // Divergence note: the reference renders these through its
      // TokenType Display (`Name`, `Plus`, `Number`); this lane keeps
      // grammar names verbatim.
      expect(pairs(lexer.tokenize()), [
        ('NAME', 'x'),
        ('PLUS', '+'),
        ('NUMBER', '1'),
      ]);
    });

    test('clearing the callback stops firing', () {
      final lexer = GrammarLexer('hello', makeGroupGrammar());
      final seen = <String>[];
      lexer.setOnToken((token, _) => seen.add(token.type));
      lexer.setOnToken(null);
      lexer.tokenize();
      expect(seen, isEmpty);
    });

    test('fresh lexers start from default', () {
      for (var i = 0; i < 2; i++) {
        final lexer = GrammarLexer('<div', makeGroupGrammar());
        lexer.setOnToken((token, ctx) {
          if (token.type == 'OPEN_TAG') ctx.pushGroup('tag');
        });
        expect(types(lexer.tokenize()), contains('TAG_NAME'));
      }
    });

    test('multiple pushes stack without crashing', () {
      final lexer = GrammarLexer('<div', makeGroupGrammar());
      lexer.setOnToken((token, ctx) {
        if (token.type == 'OPEN_TAG') {
          ctx.pushGroup('tag');
          ctx.pushGroup('tag');
        }
      });
      expect(types(lexer.tokenize()), contains('TAG_NAME'));
    });

    test('callbacks observe group transitions', () {
      final seen = <(String, String, int)>[];
      final lexer = GrammarLexer('<div>', makeGroupGrammar());
      lexer.setOnToken((token, ctx) {
        seen.add((token.type, ctx.activeGroup, ctx.groupStackDepth));
        tagCallback(token, ctx);
      });
      lexer.tokenize();
      expect(seen, [
        ('OPEN_TAG', 'default', 1),
        ('TAG_NAME', 'tag', 2),
        ('TAG_CLOSE', 'tag', 2),
      ]);
    });

    test('unknown group push throws out of tokenize', () {
      final lexer = GrammarLexer('hello', makeGroupGrammar());
      lexer.setOnToken((_, ctx) => ctx.pushGroup('nope'));
      expect(() => lexer.tokenize(), throwsArgumentError);
    });

    test('previous token tracks emissions, not suppressions', () {
      final seen = <String?>[];
      final lexer = GrammarLexer('<hello', makeGroupGrammar());
      lexer.setOnToken((token, ctx) {
        seen.add(ctx.previousToken?.type);
        if (token.type == 'OPEN_TAG') ctx.suppress();
      });
      lexer.tokenize();
      // OPEN_TAG saw nothing before it; TEXT saw nothing either,
      // because the suppressed OPEN_TAG never emitted.
      expect(seen, [null, null]);
    });

    test('previous token tracks emissions in order', () {
      final seen = <String?>[];
      final lexer = GrammarLexer('<a><b', makeGroupGrammar());
      lexer.setOnToken((token, ctx) {
        seen.add(ctx.previousToken?.type);
      });
      lexer.tokenize();
      expect(seen[0], isNull);
      expect(seen[1], 'OPEN_TAG');
    });
  });

  group('keywords', () {
    TokenGrammar sqlGrammar({bool caseInsensitive = false}) => TokenGrammar(
          caseInsensitive: caseInsensitive,
          definitions: [_def('NAME', r'[a-zA-Z_][a-zA-Z0-9_]*', true)],
          skipDefinitions: [_def('WS', r'[ \t\n]+', true)],
          keywords: caseInsensitive ? ['SELECT', 'FROM'] : ['select'],
        );

    test('case-insensitive keywords promote and normalize', () {
      for (final word in ['select', 'SELECT', 'Select']) {
        final tokens =
            GrammarLexer(word, sqlGrammar(caseInsensitive: true)).tokenize();
        expect(tokens[0].type, 'KEYWORD', reason: word);
        expect(tokens[0].value, 'SELECT', reason: word);
      }
    });

    test('case-insensitive non-keywords keep case', () {
      final tokens =
          GrammarLexer('myTable', sqlGrammar(caseInsensitive: true)).tokenize();
      expect(tokens[0].type, 'NAME');
      expect(tokens[0].value, 'myTable');
    });

    test('case-sensitive keywords keep value and case', () {
      final promoted = GrammarLexer('select', sqlGrammar()).tokenize();
      expect(promoted[0].type, 'KEYWORD');
      expect(promoted[0].value, 'select');
      final plain = GrammarLexer('SELECT', sqlGrammar()).tokenize();
      expect(plain[0].type, 'NAME');
      expect(plain[0].value, 'SELECT');
    });

    test('keyword sequence mixes promotion and names', () {
      final tokens = GrammarLexer(
              'select id from users', sqlGrammar(caseInsensitive: true))
          .tokenize();
      expect(pairs(tokens), [
        ('KEYWORD', 'SELECT'),
        ('NAME', 'id'),
        ('KEYWORD', 'FROM'),
        ('NAME', 'users'),
      ]);
    });

    test('reserved keywords throw payload-blind errors', () {
      final grammar = TokenGrammar(
        definitions: [_def('NAME', r'[a-zA-Z_][a-zA-Z0-9_]*', true)],
        reservedKeywords: ['goto'],
      );
      expect(
          () => GrammarLexer('goto', grammar).tokenize(),
          throwsA(predicate((e) =>
              e is LexerError &&
              e.message ==
                  'Reserved keyword cannot be used as an identifier.' &&
              e.line == 1 &&
              e.column == 1)));
    });

    test('context keywords flag names', () {
      final grammar = TokenGrammar(
        definitions: [_def('NAME', r'[a-zA-Z_][a-zA-Z0-9_]*', true)],
        contextKeywords: ['async'],
      );
      final tokens = GrammarLexer('async x', grammar).tokenize();
      expect(tokens[0].flags, TOKEN_CONTEXT_KEYWORD);
      expect(tokens[1].flags, TOKEN_PRECEDED_BY_SPACE);
    });
  });

  group('strings', () {
    TokenGrammar stringGrammar({String? escapeMode}) => TokenGrammar(
          escapeMode: escapeMode,
          definitions: [_def('STRING', r'"([^"\\]|\\.)*"', true)],
        );

    test('escapes decode by default', () {
      final tokens =
          GrammarLexer(r'"a\nb\tc\\d\"efq\qg\rz"', stringGrammar()).tokenize();
      expect(tokens[0].value, 'a\nb\tc\\d"efqqg\rz');
      // Escaped single-quote via a normally-quoted literal (a raw
      // string cannot hold both quote kinds).
      final quoted = GrammarLexer('"a\\\'b"', stringGrammar()).tokenize();
      expect(quoted[0].value, "a'b");
    });

    test('escapes mode none keeps raw inner text', () {
      final tokens =
          GrammarLexer(r'"a\nb"', stringGrammar(escapeMode: 'none')).tokenize();
      expect(tokens[0].value, r'a\nb');
    });

    test('triple quotes strip three', () {
      final grammar = TokenGrammar(definitions: [
        _def('STRING', r'"""[^"]*"""', true),
        _def('NAME', r'[a-z]+', true),
      ]);
      final tokens = GrammarLexer('"""hi"""', grammar).tokenize();
      expect(tokens[0].value, 'hi');
    });

    test('unquoted string-typed matches stay raw', () {
      final grammar = TokenGrammar(definitions: [
        _def('XSTRING', r'ab|"', true),
      ]);
      final tokens = GrammarLexer('ab "', grammar).tokenize();
      expect(tokens[0].value, 'ab');
    });
  });

  group('gaps and newlines', () {
    test('space gap flags the next token', () {
      final tokens = GrammarLexer('a b', makeGroupGrammar()).tokenize();
      // 'a b' lexes as one TEXT (no tag chars) with no preceding gap.
      expect(tokens[0].flags, isNull);
      final lexer = GrammarLexer('<a> <b>', makeGroupGrammar());
      lexer.setOnToken(tagCallback);
      final tagged = lexer.tokenize();
      final secondOpen =
          tagged.firstWhere((t) => t.value == '<' && t.column > 1);
      expect(secondOpen.flags, TOKEN_PRECEDED_BY_SPACE);
    });

    test('newline gap flags the next token doubly', () {
      final lexer = GrammarLexer('<a>\n<b>', makeGroupGrammar());
      lexer.setOnToken(tagCallback);
      final tokens = lexer.tokenize();
      final secondOpen =
          tokens.firstWhere((t) => t.value == '<' && t.line == 2);
      expect(secondOpen.flags,
          TOKEN_PRECEDED_BY_NEWLINE | TOKEN_PRECEDED_BY_SPACE);
    });

    test('builtin newline never fires the callback', () {
      final seen = <String>[];
      final grammar = TokenGrammar(definitions: [
        _def('NAME', r'[a-z]+', true),
      ]);
      final lexer = GrammarLexer('a\nb', grammar);
      lexer.setOnToken((token, _) => seen.add(token.type));
      final tokens = lexer.tokenize();
      expect(types(tokens), ['NAME', 'NEWLINE', 'NAME']);
      expect(seen, ['NAME', 'NAME']);
      expect(tokens[1].value, r'\n');
    });

    test('builtin whitespace path skips without skip patterns', () {
      final grammar = TokenGrammar(definitions: [
        _def('NAME', r'[a-z]+', true),
      ]);
      final tokens = GrammarLexer('a \t\r b', grammar).tokenize();
      expect(types(tokens), ['NAME', 'NAME']);
      // The builtin `\r` arms the newline bit too.
      expect(
          tokens[1].flags, TOKEN_PRECEDED_BY_NEWLINE | TOKEN_PRECEDED_BY_SPACE);
    });

    test('suppressed gap lands on the replacement', () {
      final lexer = GrammarLexer('<a >', makeGroupGrammar());
      lexer.setOnToken((token, ctx) {
        tagCallback(token, ctx);
        if (token.type == 'TAG_CLOSE') {
          ctx.suppress();
          ctx.emit(
              const Token(type: 'REPLACED', value: '>', line: 1, column: 4));
        }
      });
      final tokens = lexer.tokenize();
      final replaced = tokens.firstWhere((t) => t.type == 'REPLACED');
      // The skipped space before `>` rides onto the replacement.
      expect(replaced.flags, TOKEN_PRECEDED_BY_SPACE);
    });

    test('suppressed gap with no replacement flows onward', () {
      final lexer = GrammarLexer('a <b', makeGroupGrammar());
      lexer.setOnToken((token, ctx) {
        if (token.type == 'OPEN_TAG') ctx.suppress();
      });
      final tokens = lexer.tokenize();
      // The suppressed `<` itself separates, so TEXT 'b' takes a gap.
      final last = tokens.firstWhere((t) => t.value == 'b');
      expect(last.flags, TOKEN_PRECEDED_BY_SPACE);
    });

    test('suppressed newline gap flows onward', () {
      final lexer = GrammarLexer(
          'x\ny z',
          TokenGrammar(
              definitions: [_def('NAME', r'[a-z]+', true)],
              skipDefinitions: [_def('WS', r'\s+', true)]));
      lexer.setOnToken((token, ctx) {
        if (token.value == 'y') ctx.suppress();
      });
      final tokens = lexer.tokenize();
      // Suppressing 'y' restores its newline gap onto 'z'.
      final z = tokens.firstWhere((t) => t.value == 'z');
      expect(z.flags, TOKEN_PRECEDED_BY_NEWLINE | TOKEN_PRECEDED_BY_SPACE);
    });

    test('suppress-first repays nothing to the emit', () {
      final lexer = GrammarLexer('<b', makeGroupGrammar());
      lexer.setOnToken((token, ctx) {
        if (token.type == 'OPEN_TAG') {
          ctx.suppress();
          ctx.emit(
              const Token(type: 'REPLACED', value: '<', line: 1, column: 1));
        }
      });
      final tokens = lexer.tokenize();
      // No gap preceded the first token, so the emit keeps its flags.
      expect(tokens.firstWhere((t) => t.type == 'REPLACED').flags, isNull);
    });
  });

  group('errors and edges', () {
    test('unexpected sequence is payload-blind with position', () {
      final lexer = GrammarLexer(
          'a @', TokenGrammar(definitions: [_def('NAME', r'[a-z]+', true)]));
      expect(
          () => lexer.tokenize(),
          throwsA(predicate((e) =>
              e is LexerError &&
              e.message == 'Unexpected sequence.' &&
              e.line == 1 &&
              e.column == 3)));
    });

    test('aliases resolve to the emitted type', () {
      final grammar = TokenGrammar(definitions: [
        _def('HEX', r'0x[0-9a-f]+', true, alias: 'NUMBER'),
        _def('NAME', r'[a-z]+', true),
      ]);
      final tokens = GrammarLexer('0xff', grammar).tokenize();
      expect(tokens[0].type, 'NUMBER');
      expect(tokens[0].value, '0xff');
    });

    test('indentation and layout modes are rejected', () {
      for (final mode in ['indentation', 'layout']) {
        final grammar = TokenGrammar(
          mode: mode,
          definitions: [_def('NAME', r'[a-z]+', true)],
        );
        expect(() => GrammarLexer('x', grammar).tokenize(),
            throwsA(isA<UnsupportedError>()),
            reason: mode);
      }
    });

    test('empty literal definitions never match', () {
      // Security review: `startsWith('', pos)` is always true, so an
      // empty literal used to zero-width-match and hang the loop.
      final grammar = TokenGrammar(definitions: [
        _def('EMPTY', '', false),
        _def('NAME', r'[a-z]+', true),
      ]);
      final tokens = GrammarLexer('ab', grammar).tokenize();
      expect(types(tokens), ['NAME']);
    });

    test('empty source yields EOF only', () {
      final tokens = GrammarLexer('', makeGroupGrammar()).tokenize();
      expect(tokens.length, 1);
      expect(tokens[0].type, 'EOF');
    });

    test('second tokenize call sees spent state', () {
      // The cursor stays spent like the reference; only the stack, skip
      // flag, and gaps reset.
      final lexer = GrammarLexer('<a>', makeGroupGrammar());
      lexer.setOnToken(tagCallback);
      expect(lexer.tokenize().length, greaterThan(1));
      final again = lexer.tokenize();
      expect(again.length, 1);
      expect(again[0].type, 'EOF');
    });

    test('case-insensitive patterns follow caseSensitive only', () {
      // Reference parity: pattern compilation obeys `caseSensitive`,
      // not `caseInsensitive` (which governs keywords alone).
      final grammar = TokenGrammar(
        caseSensitive: true,
        caseInsensitive: true,
        definitions: [_def('AB', 'ab', false)],
        keywords: ['SELECT'],
      );
      expect(() => GrammarLexer('AB', grammar).tokenize(),
          throwsA(isA<LexerError>()));
      final tokens = GrammarLexer('ab', grammar).tokenize();
      expect(tokens[0].value, 'ab');
    });
  });

  group('hooks', () {
    TokenGrammar hookGrammar() => TokenGrammar(
          definitions: [
            _def('WORD', r'[a-z]+', true),
            _def('BANG', '!', false),
          ],
        );

    test('pre hook transforms the source before lexing', () {
      final lexer = GrammarLexer('hello world', hookGrammar())
        ..addPreTokenize((src) => src.replaceAll('world', 'there'));
      expect(pairs(lexer.tokenize()), [('WORD', 'hello'), ('WORD', 'there')]);
    });

    test('pre hooks compose left to right', () {
      final seen = <String>[];
      final lexer = GrammarLexer('ab', hookGrammar())
        ..addPreTokenize((src) {
          seen.add('first:$src');
          return '$src-c';
        })
        ..addPreTokenize((src) {
          seen.add('second:$src');
          return src.replaceAll('-', '');
        });
      final tokens = lexer.tokenize();
      // The second hook observed the FIRST hook's output (`ab-c`),
      // proving left-to-right composition; the final source `abc`
      // lexes as one word.
      expect(seen, ['first:ab', 'second:ab-c']);
      expect(types(tokens), ['WORD']);
      expect(tokens[0].value, 'abc');
    });

    test('post hook transforms the token list', () {
      final lexer = GrammarLexer('hi!', hookGrammar())
        ..addPostTokenize(
            (tokens) => tokens.where((t) => t.type != 'BANG').toList());
      expect(pairs(lexer.tokenize()), [('WORD', 'hi')]);
      expect(lexer.tokenize().last.type, 'EOF');
    });

    test('post hooks compose left to right and see EOF', () {
      final seen = <int>[];
      final lexer = GrammarLexer('ab', hookGrammar())
        ..addPostTokenize((tokens) {
          seen.add(tokens.length);
          return tokens;
        })
        ..addPostTokenize((tokens) {
          seen.add(tokens.length);
          return [
            for (final t in tokens)
              if (t.type != 'EOF')
                Token(
                    type: t.type,
                    value: '${t.value}!',
                    line: t.line,
                    column: t.column,
                    flags: t.flags)
              else
                t,
          ];
        });
      final tokens = lexer.tokenize();
      // Both hooks observed WORD + EOF.
      expect(seen, [2, 2]);
      expect(pairs(tokens), [('WORD', 'ab!')]);
    });

    test('no hooks leaves the stream untouched', () {
      final tokens = GrammarLexer('hi!', hookGrammar()).tokenize();
      expect(tokens.map((t) => t.type).toList(), ['WORD', 'BANG', 'EOF']);
    });
  });
}
