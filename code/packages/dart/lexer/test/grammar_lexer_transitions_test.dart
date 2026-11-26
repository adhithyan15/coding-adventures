/// Tests for declarative mode transitions: mirrors of the
/// reference engine's F10 tests, plus condition, value-guard,
/// refuse, and malformed-artifact extras.
library;

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

List<String> names(List<Token> tokens) => [
      for (final t in tokens)
        if (t.type != 'EOF') t.type
    ];

TokenGrammar regexDivGrammar() => parseTokenGrammar('''
NAME = /[a-z]+/
REGEX = /\\/[a-z]+\\//
SLASH_EQUALS = "/="
SLASH = "/"
EQUALS = "="
PLUS = "+"

group div:
  SLASH_EQUALS = "/="
  SLASH = "/"

start_mode: default
transitions:
  on NAME -> set-mode div
  on (EQUALS | PLUS | SLASH | SLASH_EQUALS | REGEX) -> set-mode default
''');

TokenGrammar rubyLikeNewlineGrammar(String extraRules) {
  final grammar = parseTokenGrammar('''
start_mode: beg
NAME = /[a-z]+/
QUESTION = "?"
SEMI = ";"
BANG = "!"
group beg:
  CHAR = /\\?[a-z]/
group hold:
  HASH = "#"
transitions:
$extraRules''');
  // The validator must accept `NEWLINE` as a trigger even though no
  // pattern declares it: it is a built-in emitted token.
  expect(validateTokenGrammar(grammar), isEmpty);
  return grammar;
}

void main() {
  group('transitions (F10 mirrors)', () {
    test('flat mode inherits default patterns', () {
      // In `div` mode a NAME has no override, so it must be matched
      // via inheritance from the default group. If inheritance were
      // off, the second identifier would fail to lex.
      final tokens = GrammarLexer('ab cd', regexDivGrammar()).tokenize();
      expect(names(tokens), equals(['NAME', 'NAME']));
    });

    test('slash equals override beats inherited regex', () {
      // `a/=b` — in div mode the `/=` override wins over the inherited
      // `REGEX`/`SLASH` patterns.
      final tokens = GrammarLexer('a/=b', regexDivGrammar()).tokenize();
      expect(names(tokens), equals(['NAME', 'SLASH_EQUALS', 'NAME']));
    });

    test('no transitions is backward compatible', () {
      // The SAME tokens without a transitions table: `REGEX` greedily
      // wins, so `a/b/c` lexes as NAME REGEX NAME. This pins that an
      // empty table changes nothing.
      final grammar = parseTokenGrammar(
        'NAME = /[a-z]+/\nREGEX = /\\/[a-z]+\\//\nSLASH = "/"\n',
      );
      final tokens = GrammarLexer('a/b/c', grammar).tokenize();
      expect(names(tokens), equals(['NAME', 'REGEX', 'NAME']));
    });

    test('push region stays exclusive', () {
      // A `push`ed region is NOT a flat mode: it does NOT inherit
      // default, preserving exclusivity. Here the `tag` region only
      // knows `WORD` and `CLOSE`; the default `BANG` is invisible
      // inside it, so a `!` inside the region is a lex error.
      final grammar = parseTokenGrammar('''
OPEN = "<"
BANG = "!"
group tag:
  WORD = /[a-z]+/
  CLOSE = ">"
transitions:
  on OPEN -> push tag
  on CLOSE -> pop
''');
      // Inside the region, `!` (a default token) must NOT match.
      expect(
        () => GrammarLexer('<a!>', grammar).tokenize(),
        throwsA(isA<LexerError>()),
      );
      // A well-formed region tokenizes; after `>` we pop back to
      // default where `!` is valid again.
      final tokens = GrammarLexer('<ab>!', grammar).tokenize();
      expect(names(tokens), equals(['OPEN', 'WORD', 'CLOSE', 'BANG']));
    });

    test('start mode is respected', () {
      // Starting directly in `div` means the first `/` is a division
      // slash.
      final base = regexDivGrammar();
      final grammar = TokenGrammar(
        definitions: base.definitions,
        groups: base.groups,
        startMode: 'div',
        transitions: base.transitions,
      );
      final tokens = GrammarLexer('a/b', grammar).tokenize();
      expect(names(tokens), equals(['NAME', 'SLASH', 'NAME']));
    });

    test('newline set mode resets line start state', () {
      final grammar = rubyLikeNewlineGrammar(
        '  on NAME -> set-mode default\n  on (SEMI | NEWLINE) -> set-mode beg\n',
      );
      // Line 1 ends in a value (`x`), so `?a` after it is QUESTION
      // NAME on the same line; after the NEWLINE we are back in `beg`
      // and `?a` is a CHAR.
      final tokens = GrammarLexer('x ?a\n?a', grammar).tokenize();
      expect(
        names(tokens),
        equals(['NAME', 'QUESTION', 'NAME', 'NEWLINE', 'CHAR']),
      );
      // `;` (an ordinary declared token) and NEWLINE behave identically.
      final tokens2 = GrammarLexer('x;?a', grammar).tokenize();
      expect(names(tokens2), equals(['NAME', 'SEMI', 'CHAR']));
    });

    test('newline in mode guard is respected', () {
      final grammar = rubyLikeNewlineGrammar(
        '  on NAME -> set-mode default\n  on BANG -> set-mode hold\n  on NEWLINE in default -> set-mode beg\n',
      );
      // In `default` the guarded rule fires: next line starts in `beg`.
      final tokens = GrammarLexer('x\n?a', grammar).tokenize();
      expect(names(tokens), equals(['NAME', 'NEWLINE', 'CHAR']));
      // In `hold` the guard does not match, so the mode survives the
      // line break: `?a` is QUESTION NAME (hold inherits default's
      // patterns).
      final tokens2 = GrammarLexer('!\n?a', grammar).tokenize();
      expect(
        names(tokens2),
        equals(['BANG', 'NEWLINE', 'QUESTION', 'NAME']),
      );
    });

    test('newline pops a pushed region', () {
      // A line comment as a nested region: `#` pushes `comment`, and
      // the end of the line pops it.
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
HASH = "#"
group comment:
  TEXT = /[^\\n]+/
transitions:
  on HASH -> push comment
  on NEWLINE in comment -> pop
''');
      expect(validateTokenGrammar(grammar), isEmpty);
      final tokens = GrammarLexer('a # hi there\nb', grammar).tokenize();
      expect(
        names(tokens),
        equals(['NAME', 'HASH', 'TEXT', 'NEWLINE', 'NAME']),
      );
      expect(tokens[2].value, equals('hi there'));
    });

    test('newline pushes a region', () {
      // NEWLINE can also push: every line after the first starts in an
      // exclusive `head` region that only accepts a LABEL, which pops
      // back.
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
group head:
  LABEL = /[a-z]+:/
transitions:
  on NEWLINE in default -> push head
  on LABEL -> pop
''');
      final tokens = GrammarLexer('a\nb: c', grammar).tokenize();
      expect(names(tokens), equals(['NAME', 'NEWLINE', 'LABEL', 'NAME']));
      // `head` is exclusive (push target), so a line that does not
      // start with a label is an error — proof the region really was
      // entered.
      expect(
        () => GrammarLexer('a\nb c', grammar).tokenize(),
        throwsA(isA<LexerError>()),
      );
    });

    test('table without newline rule is unchanged across lines', () {
      // A grammar WITH a transition table but no NEWLINE rule: the
      // mode set by `a` (div) survives the line break. So `/b/` on the
      // next line is SLASH NAME SLASH, not a REGEX.
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
REGEX = /\\/[a-z]+\\//
group div:
  SLASH = "/"
transitions:
  on NAME -> set-mode div
''');
      final tokens = GrammarLexer('a\n/b/', grammar).tokenize();
      expect(
        names(tokens),
        equals(['NAME', 'NEWLINE', 'SLASH', 'NAME', 'SLASH']),
      );
    });

    test('newline rule cannot fire when skip consumes newlines', () {
      // A skip pattern that eats `\n` means no NEWLINE is ever
      // emitted, so an `on NEWLINE` rule has nothing to fire on and
      // the mode carries over.
      final grammar = parseTokenGrammar('''
start_mode: beg
NAME = /[a-z]+/
QUESTION = "?"
group beg:
  CHAR = /\\?[a-z]/
skip:
  WS = /[ \\t\\n]+/
transitions:
  on NAME -> set-mode default
  on NEWLINE -> set-mode beg
''');
      final tokens = GrammarLexer('x\n?a', grammar).tokenize();
      expect(names(tokens), equals(['NAME', 'QUESTION', 'NAME']));
    });

    test('newline emitted while skip disabled fires multi action rule', () {
      // With skip disabled (by `!`), the newline-eating skip pattern
      // is not consulted, so the `\n` becomes a built-in NEWLINE —
      // whose rule both resets the mode AND re-enables skip, in order.
      final grammar = parseTokenGrammar('''
start_mode: beg
NAME = /[a-z]+/
QUESTION = "?"
BANG = "!"
group beg:
  CHAR = /\\?[a-z]/
skip:
  WS = /[ \\t\\n]+/
transitions:
  on NAME -> set-mode default
  on BANG -> disable-skip
  on NEWLINE -> set-mode beg, enable-skip
''');
      expect(validateTokenGrammar(grammar), isEmpty);
      final tokens = GrammarLexer('x!\n?a \n ?b', grammar).tokenize();
      // After the first NEWLINE, skip is back on: the later ` \n ` is
      // skipped trivia (no second NEWLINE), and `?b` follows `?a`'s
      // CHAR in `beg` mode (no NAME switched it to default).
      expect(
        names(tokens),
        equals(['NAME', 'BANG', 'NEWLINE', 'CHAR', 'CHAR']),
      );
    });

    test('newline transition runs alongside a callback', () {
      // A registered callback is still not invoked for the built-in
      // NEWLINE (unchanged), but the declarative table is.
      final grammar = rubyLikeNewlineGrammar(
        '  on NAME -> set-mode default\n  on NEWLINE -> set-mode beg\n',
      );
      final seen = <String>[];
      final lexer = GrammarLexer('x\n?a', grammar);
      lexer.setOnToken((token, ctx) {
        seen.add(token.type);
      });
      final tokens = lexer.tokenize();
      expect(names(tokens), equals(['NAME', 'NEWLINE', 'CHAR']));
      expect(seen, equals(['NAME', 'CHAR']));
    });

    test('newline state resets between tokenize calls', () {
      // The mode a NEWLINE rule sets must not leak into the next run:
      // the stack resets to start_mode at EOF, as for any other
      // transition.
      final grammar = rubyLikeNewlineGrammar(
        // `on SEMI -> set-mode beg` keeps `beg` a flat (inheriting) mode.
        '  on NAME -> set-mode default\n  on SEMI -> set-mode beg\n  on NEWLINE -> set-mode hold\n',
      );
      final lexer = GrammarLexer('x\n', grammar);
      final tokens = lexer.tokenize();
      expect(names(tokens), equals(['NAME', 'NEWLINE']));
      // The NEWLINE rule left the lexer in `hold` at EOF; the reset
      // puts the register back to `beg`, the declared start mode.
      expect(lexer.activeGroup, equals('beg'));
      expect(lexer.groupStackDepth, equals(1));
    });
  });

  group('transitions extras', () {
    test('value guard fires only for the named keyword', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
SLASH = "/"
keywords:
  return
  yield
group div:
  DIV = "/"
transitions:
  on KEYWORD="return" -> set-mode div
''');
      expect(validateTokenGrammar(grammar), isEmpty);
      // `return` switches to div, where `/` reads as DIV ...
      final switched = GrammarLexer('return /', grammar).tokenize();
      expect(names(switched), equals(['KEYWORD', 'DIV']));
      // ... but `yield`, though also a keyword, does not switch.
      final plain = GrammarLexer('yield /', grammar).tokenize();
      expect(names(plain), equals(['KEYWORD', 'SLASH']));
      // Nor does an ordinary name.
      final name = GrammarLexer('max /', grammar).tokenize();
      expect(names(name), equals(['NAME', 'SLASH']));
    });

    test('first matching rule wins', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
SLASH = "/"
group div:
  DIV = "/"
group re:
  REGEX = "/"
transitions:
  on NAME -> set-mode div
  on NAME -> set-mode re
''');
      // The first rule wins: `/` after a NAME is DIV, not REGEX.
      final tokens = GrammarLexer('a /', grammar).tokenize();
      expect(names(tokens), equals(['NAME', 'DIV']));
    });

    test('conditions gate on space and line facts', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
SLASH = "/"
group div:
  DIV = "/"
transitions:
  on NAME if space_before -> set-mode div
''');
      // `a` at input start has no space before: no switch, SLASH stays.
      final noSpace = GrammarLexer('a/', grammar).tokenize();
      expect(names(noSpace), equals(['NAME', 'SLASH']));
      // `a` after a space switches: DIV wins.
      final space = GrammarLexer('x a/', grammar).tokenize();
      expect(names(space), equals(['NAME', 'NAME', 'DIV']));
    });

    test('negated line start condition', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
SLASH = "/"
group div:
  DIV = "/"
transitions:
  on NAME if !line_start -> set-mode div
''');
      // `a` at input start and `x` at a line start do not switch.
      final atStarts = GrammarLexer('a\nx /', grammar).tokenize();
      expect(names(atStarts), equals(['NAME', 'NEWLINE', 'NAME', 'SLASH']));
      // Mid-line `x` switches: `/` reads as DIV.
      final midLine = GrammarLexer('q x /', grammar).tokenize();
      expect(names(midLine), equals(['NAME', 'NAME', 'DIV']));
    });

    test('astral next char is never ascii whitespace', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
SLASH = "/"
TEXT = /\u{1F389}/
group div:
  DIV = "/"
transitions:
  on NAME if space_after -> set-mode div
''');
      // A BMP space after NAME switches: `/` reads as DIV.
      final space = GrammarLexer('a /', grammar).tokenize();
      expect(names(space), equals(['NAME', 'DIV']));
      // An astral emoji after NAME does not switch (exercising the
      // surrogate-pair decode of the lookahead character): `/` stays
      // SLASH. A buggy "astral counts as space" reading would give DIV.
      final emoji = GrammarLexer('a\u{1F389}/', grammar).tokenize();
      expect(names(emoji), equals(['NAME', 'TEXT', 'SLASH']));
    });

    test('lone surrogate lookahead fails closed', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
SLASH = "/"
group div:
  DIV = "/"
transitions:
  on NAME if space_after -> set-mode div
''');
      // A lone high surrogate followed by a non-surrogate reads as one
      // non-whitespace unit (no switch), then the surrogate itself is
      // unlexable and the run fails closed with LexerError only.
      expect(
        () => GrammarLexer('a\ud800/', grammar).tokenize(),
        throwsA(isA<LexerError>()),
      );
    });

    test('line start condition sees real line starts', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
BANG = "!"
group head:
  HEAD = /[a-z]+/
transitions:
  on BANG if line_start -> set-mode head
''');
      // `!` at a line start switches to head, where `go` is HEAD.
      final atStart = GrammarLexer('a\n!go', grammar).tokenize();
      expect(
        names(atStart),
        equals(['NAME', 'NEWLINE', 'BANG', 'HEAD']),
      );
      // Mid-line `!` does not switch: `go` stays NAME.
      final midLine = GrammarLexer('a !go', grammar).tokenize();
      expect(names(midLine), equals(['NAME', 'BANG', 'NAME']));
    });

    test('refuse fails lexing with a payload-blind message', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
BANG = "!"
transitions:
  on BANG -> refuse
''');
      try {
        GrammarLexer('a !', grammar).tokenize();
        fail('expected LexerError');
      } on LexerError catch (e) {
        expect(e.message, contains('not supported in this mode'));
        expect(e.message, isNot(contains('!')));
        expect((e.line, e.column), equals((1, 3)));
      }
    });

    test('pop never pops the last mode', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
BANG = "!"
transitions:
  on BANG -> pop
''');
      final lexer = GrammarLexer('a ! b', grammar);
      final tokens = lexer.tokenize();
      expect(names(tokens), equals(['NAME', 'BANG', 'NAME']));
      expect(lexer.activeGroup, equals('default'));
      expect(lexer.groupStackDepth, equals(1));
    });

    test('unknown start mode falls back to default', () {
      // A malformed compiled artifact (validation would reject the
      // source) still tokenizes instead of throwing at construction.
      final grammar = TokenGrammar(
        definitions: [
          TokenDefinition(
              name: 'NAME', pattern: '[a-z]+', isRegex: true, lineNumber: 1),
        ],
        startMode: 'nope',
      );
      final lexer = GrammarLexer('ab', grammar);
      expect(lexer.activeGroup, equals('default'));
      expect(names(lexer.tokenize()), equals(['NAME']));
    });

    test('unknown mode falls back to default patterns', () {
      // Same fail-soft contract mid-stream: `set-mode` to an
      // undeclared mode lexes on with default patterns rather than
      // throwing a lookup error.
      final grammar = TokenGrammar(
        definitions: [
          TokenDefinition(
              name: 'NAME', pattern: '[a-z]+', isRegex: true, lineNumber: 1),
          TokenDefinition(
              name: 'BANG', pattern: '!', isRegex: false, lineNumber: 2),
        ],
        transitions: [
          ModeTransition(
            onTokens: ['BANG'],
            actions: [SetMode('nope')],
            lineNumber: 3,
          ),
        ],
      );
      final tokens = GrammarLexer('a ! b', grammar).tokenize();
      expect(names(tokens), equals(['NAME', 'BANG', 'NAME']));
    });
  });
}
