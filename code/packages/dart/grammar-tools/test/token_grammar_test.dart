import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:test/test.dart';

void main() {
  group('parseTokenGrammar', () {
    test('parses regex and literal token definitions', () {
      final grammar = parseTokenGrammar('''NUMBER = /[0-9]+/
PLUS = "+"
''');

      expect(grammar.definitions.length, 2);
      expect(
        grammar.definitions.first,
        const TokenDefinition(
          name: 'NUMBER',
          pattern: '[0-9]+',
          isRegex: true,
          lineNumber: 1,
        ),
      );
      expect(grammar.definitions[1].isRegex, isFalse);
      expect(grammar.definitions[1].pattern, '+');
    });

    test('literals end at the first quote, escapes blind', () {
      // No lane unescapes literal bodies, so the closer is the first
      // `"` — `"\"` is the one-backslash pattern (apl.tokens SCAN),
      // exactly like the Rust (`rest.find('"')`) and Go
      // (`strings.Index`) parsers. Escape-skipping would reject it.
      final grammar = parseTokenGrammar(r'SCAN = "\"');
      expect(grammar.definitions.length, 1);
      expect(grammar.definitions.first.isRegex, isFalse);
      expect(grammar.definitions.first.pattern, r'\');
    });

    test('parses keywords, reserved, skip, errors, and aliases', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
STRING_DQ = /"[^"]*"/ -> STRING
mode: indentation
escapes: none
case_sensitive: false
keywords:
  if
reserved:
  class
skip:
  SPACE = /[ \t]+/
errors:
  BAD = /@+/
context_keywords:
  async
soft_keywords:
  match
''');

      expect(grammar.mode, 'indentation');
      expect(grammar.escapeMode, 'none');
      expect(grammar.caseSensitive, isFalse);
      expect(grammar.keywords, ['if']);
      expect(grammar.reservedKeywords, ['class']);
      expect(grammar.skipDefinitions.single.name, 'SPACE');
      expect(grammar.errorDefinitions.single.name, 'BAD');
      expect(grammar.contextKeywords, ['async']);
      expect(grammar.softKeywords, ['match']);
      expect(grammar.definitions[1].alias, 'STRING');
    });

    test('parses pattern groups', () {
      final grammar = parseTokenGrammar('''
TEXT = /[^<]+/
group tag:
  TAG_NAME = /[a-z]+/
  EQUALS = "="
''');

      expect(grammar.groups.keys, ['tag']);
      expect(grammar.groups['tag']!.definitions.length, 2);
    });

    test('reports malformed definitions with line numbers', () {
      expect(
        () => parseTokenGrammar('NUMBER /[0-9]+/'),
        throwsA(
          isA<TokenGrammarError>().having(
            (error) => error.lineNumber,
            'lineNumber',
            1,
          ),
        ),
      );
    });
  });

  group('validateTokenGrammar', () {
    test('detects duplicate names and invalid mode', () {
      final grammar = parseTokenGrammar('''
name = /[a-z]+/
name = /[a-z0-9]+/
mode: weird
''');

      final issues = validateTokenGrammar(grammar);
      expect(
        issues.any((issue) => issue.contains('Duplicate token name')),
        isTrue,
      );
      expect(
        issues.any((issue) => issue.contains("Unknown lexer mode 'weird'")),
        isTrue,
      );
      expect(
        issues.any(
          (issue) => issue.contains("Token name 'name' should be UPPER_CASE"),
        ),
        isTrue,
      );
    });

    test('includes aliases in token names', () {
      final grammar = parseTokenGrammar('STRING_DQ = /"[^"]*"/ -> STRING');
      expect(grammar.tokenNames(), containsAll(['STRING_DQ', 'STRING']));
      expect(grammar.effectiveTokenNames(), {'STRING'});
    });
  });

  group('transitions section', () {
    test('parses start_mode directive', () {
      final grammar = parseTokenGrammar('start_mode: div\nNAME = /[a-z]+/\n');
      expect(grammar.startMode, equals('div'));
    });

    test('start_mode defaults to null', () {
      final grammar = parseTokenGrammar('NAME = /[a-z]+/\n');
      expect(grammar.startMode, isNull);
      expect(grammar.transitions, isEmpty);
    });

    test('missing start_mode name is an error', () {
      expect(
        () => parseTokenGrammar('start_mode:\n'),
        throwsA(
          isA<TokenGrammarError>().having(
            (e) => e.message,
            'message',
            contains("Missing mode name after 'start_mode:'"),
          ),
        ),
      );
    });

    test('parses a simple set-mode rule', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
group div:
  SLASH = "/"
transitions:
  on NAME -> set-mode div
''');
      expect(grammar.transitions.length, equals(1));
      expect(
        grammar.transitions.first,
        const ModeTransition(
          onTokens: ['NAME'],
          actions: [SetMode('div')],
          lineNumber: 5,
        ),
      );
    });

    test('parses alternation, value guard, mode guard, conditions', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
group div:
  SLASH = "/"
keywords:
  return
transitions:
  on (NAME | NUMBER) in default if space_before && !line_start -> push div, disable-skip
  on KEYWORD="return" -> set-mode div
''');
      expect(grammar.transitions.length, equals(2));
      final first = grammar.transitions[0];
      expect(first.onTokens, equals(['NAME', 'NUMBER']));
      expect(first.inMode, equals('default'));
      expect(
        first.conditions,
        equals([
          TransitionCondition.spaceBefore,
          TransitionCondition.notLineStart,
        ]),
      );
      expect(
        first.actions,
        equals([PushRegion('div'), DisableSkip()]),
      );
      final second = grammar.transitions[1];
      expect(second.onTokens, equals(['KEYWORD']));
      expect(second.onValue, equals('return'));
      expect(second.actions, equals([SetMode('div')]));
    });

    test('parses pop, enable-skip, and refuse actions', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
BANG = "!"
transitions:
  on NAME -> pop, enable-skip
  on BANG -> refuse
''');
      expect(
        grammar.transitions[0].actions,
        equals([PopRegion(), EnableSkip()]),
      );
      expect(
        grammar.transitions[1].actions,
        equals([RefuseToken()]),
      );
    });

    test('missing arrow is an error', () {
      expect(
        () => parseTokenGrammar('NAME = /[a-z]+/\ntransitions:\n  on NAME\n'),
        throwsA(
          isA<TokenGrammarError>().having(
            (e) => e.message,
            'message',
            contains("missing '->'"),
          ),
        ),
      );
    });

    test('missing on prefix is an error', () {
      expect(
        () => parseTokenGrammar(
          'NAME = /[a-z]+/\ntransitions:\n  NAME -> set-mode div\n',
        ),
        throwsA(
          isA<TokenGrammarError>().having(
            (e) => e.message,
            'message',
            contains("must start with 'on '"),
          ),
        ),
      );
    });

    test('empty actions are an error', () {
      expect(
        () => parseTokenGrammar(
          'NAME = /[a-z]+/\ntransitions:\n  on NAME ->\n',
        ),
        throwsA(
          isA<TokenGrammarError>().having(
            (e) => e.message,
            'message',
            contains('no actions'),
          ),
        ),
      );
    });

    test('unknown action is an error', () {
      expect(
        () => parseTokenGrammar(
          'NAME = /[a-z]+/\ntransitions:\n  on NAME -> explode\n',
        ),
        throwsA(
          isA<TokenGrammarError>().having(
            (e) => e.message,
            'message',
            contains("Unknown transition action 'explode'"),
          ),
        ),
      );
    });

    test('refuse with other actions is an error', () {
      expect(
        () => parseTokenGrammar(
          'NAME = /[a-z]+/\ntransitions:\n  on NAME -> refuse, pop\n',
        ),
        throwsA(
          isA<TokenGrammarError>().having(
            (e) => e.message,
            'message',
            contains("'refuse' must be a transition rule's only action"),
          ),
        ),
      );
    });

    test('unknown condition is an error', () {
      expect(
        () => parseTokenGrammar(
          'NAME = /[a-z]+/\ntransitions:\n  on NAME if soggy -> pop\n',
        ),
        throwsA(
          isA<TokenGrammarError>().having(
            (e) => e.message,
            'message',
            contains("Unknown transition condition 'soggy'"),
          ),
        ),
      );
    });

    test('if inside quotes or parens does not split', () {
      // `KEYWORD="if"` must not treat the quoted `if` as a condition
      // clause; the rule has no conditions and one value guard.
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
keywords:
  if
transitions:
  on KEYWORD="if" -> pop
''');
      expect(grammar.transitions.length, equals(1));
      final rule = grammar.transitions.first;
      expect(rule.onTokens, equals(['KEYWORD']));
      expect(rule.onValue, equals('if'));
      expect(rule.conditions, isEmpty);
    });

    test('reserved section names cannot be group names', () {
      for (final name in ['transitions', 'modes', 'start_mode']) {
        expect(
          () => parseTokenGrammar('group $name:\n  X = "x"\n'),
          throwsA(
            isA<TokenGrammarError>().having(
              (e) => e.message,
              'message',
              contains('Reserved group name'),
            ),
          ),
          reason: name,
        );
      }
    });
  });

  group('transitions validation', () {
    test('undeclared start mode is an issue', () {
      final grammar = parseTokenGrammar('start_mode: nope\nNAME = /[a-z]+/\n');
      expect(
        validateTokenGrammar(grammar),
        contains("start_mode 'nope' is not 'default' or a declared group"),
      );
    });

    test('too many rules trips the cap', () {
      final grammar = TokenGrammar(
        definitions: [
          const TokenDefinition(
              name: 'NAME', pattern: '[a-z]+', isRegex: true, lineNumber: 1),
        ],
        transitions: [
          for (var i = 0; i < maxTransitions + 1; i++)
            ModeTransition(
                onTokens: ['NAME'],
                actions: [SetMode('default')],
                lineNumber: i),
        ],
      );
      expect(
        validateTokenGrammar(grammar).any(
          (issue) => issue.contains('Too many transition rules'),
        ),
        isTrue,
      );
    });

    test('indentation grammars reject refuse and conditions', () {
      final grammar = parseTokenGrammar('''
mode: indentation
NAME = /[a-z]+/
BANG = "!"
transitions:
  on BANG -> refuse
  on NAME if space_before -> pop
''');
      final issues = validateTokenGrammar(grammar);
      expect(
        issues
            .where((issue) => issue.contains('do not apply transitions'))
            .length,
        equals(2),
      );
    });

    test('unknown trigger is an issue', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
transitions:
  on BOGUS -> pop
''');
      expect(
        validateTokenGrammar(grammar),
        contains(
          "Transition trigger 'BOGUS' (line 3) is not a token this grammar can emit",
        ),
      );
    });

    test('builtin triggers and keyword triggers pass', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
BANG = "!"
keywords:
  return
transitions:
  on NEWLINE -> pop
  on KEYWORD="return" -> pop
''');
      expect(validateTokenGrammar(grammar), isEmpty);
    });

    test('empty trigger set is an error', () {
      expect(
        () => parseTokenGrammar(
          'NAME = /[a-z]+/\ntransitions:\n  on () -> pop\n',
        ),
        throwsA(
          isA<TokenGrammarError>().having(
            (e) => e.message,
            'message',
            contains('no trigger tokens'),
          ),
        ),
      );
    });

    test('all-empty actions are an error', () {
      expect(
        () => parseTokenGrammar(
          'NAME = /[a-z]+/\ntransitions:\n  on NAME -> ,\n',
        ),
        throwsA(
          isA<TokenGrammarError>().having(
            (e) => e.message,
            'message',
            contains('no valid actions'),
          ),
        ),
      );
    });

    test('alias triggers validate clean', () {
      final grammar = parseTokenGrammar('''
STRING_DQ = /"[^"]*"/ -> STRING
transitions:
  on STRING -> pop
''');
      expect(validateTokenGrammar(grammar), isEmpty);
    });

    test('transition values support equality, hashing, printing', () {
      const actions = [
        SetMode('div'),
        PushRegion('tag'),
        PopRegion(),
        EnableSkip(),
        DisableSkip(),
        RefuseToken(),
      ];
      for (var i = 0; i < actions.length; i++) {
        for (var j = 0; j < actions.length; j++) {
          expect(actions[i] == actions[j], equals(i == j));
        }
      }
      expect(SetMode('div'), isNot(equals(SetMode('re'))));
      expect(PushRegion('a'), isNot(equals(PushRegion('b'))));
      for (final action in actions) {
        expect(action.hashCode, isA<int>());
        expect(action.toString(), isNotEmpty);
      }
      const rule = ModeTransition(
        onTokens: ['NAME'],
        actions: [SetMode('div')],
        lineNumber: 1,
      );
      expect(rule.hashCode, isA<int>());
      expect(
        rule,
        isNot(
          equals(
            const ModeTransition(
              onTokens: ['OTHER'],
              actions: [SetMode('div')],
              lineNumber: 1,
            ),
          ),
        ),
      );
    });

    test('undeclared guard and target modes are issues', () {
      final grammar = parseTokenGrammar('''
NAME = /[a-z]+/
transitions:
  on NAME in nowhere -> set-mode elsewhere
''');
      final issues = validateTokenGrammar(grammar);
      expect(
        issues,
        contains(
          "Transition guard 'in nowhere' (line 3) names an undeclared mode",
        ),
      );
      expect(
        issues,
        contains(
          "Transition action targets undeclared mode 'elsewhere' (line 3)",
        ),
      );
    });
  });

  group('inline flag groups', () {
    String patternOf(String body) =>
        parseTokenGrammar('X = /$body/').definitions.single.pattern;

    test('expands (?i:) literals to case pairs', () {
      expect(patternOf('(?i:erDiagram)'),
          equals('(?:[eE][rR][dD][iI][aA][gG][rR][aA][mM])'));
    });

    test('leaves escapes, classes without letters, and structure alone', () {
      expect(patternOf(r'(?i:\|\||only[ \t]+one)'),
          equals(r'(?:\|\||[oO][nN][lL][yY][ \t]+[oO][nN][eE])'));
    });

    test('expands letter ranges and singles inside classes', () {
      expect(patternOf('(?i:[a-z]+)'), equals('(?:[a-zA-Z]+)'));
      expect(patternOf('(?i:[^A-Z])'), equals('(?:[^A-Za-z])'));
      expect(patternOf('(?i:[ab])'), equals('(?:[aAbB])'));
      expect(patternOf('(?i:[0-9])'), equals('(?:[0-9])'));
    });

    test('leading bracket and mixed ranges in classes', () {
      // A leading ']' is a literal member, folded like the rest.
      expect(patternOf('(?i:[]ab]+)'), equals('(?:[]aAbB]+)'));
      // Mixed-case ranges already span cases and pass through.
      expect(patternOf('(?i:[A-z])'), equals('(?:[A-z])'));
    });

    test('nested plain groups stay insensitive without double expansion', () {
      expect(patternOf('(?i:a(b)c)'), equals('(?:[aA]([bB])[cC])'));
      expect(patternOf('(?i:a(?i:b)c)'), equals('(?:[aA](?:[bB])[cC])'));
    });

    test('text outside the group is untouched', () {
      expect(patternOf(r'x(?i:ab)y\d'), equals(r'x(?:[aA][bB])y\d'));
    });

    test('other inline flags fail fast with a clear error', () {
      for (final body in ['(?m:ab)', '(?s:x)', '(?i-m:x)']) {
        try {
          patternOf(body);
          fail('expected TokenGrammarError for $body');
        } on TokenGrammarError catch (e) {
          expect(e.toString(), contains('Unsupported inline flag group'));
        }
      }
    });

    test('non-capturing and lookahead groups pass through', () {
      expect(patternOf('(?:ab)(?=c)(?!d)'), equals('(?:ab)(?=c)(?!d)'));
    });

    test('unterminated classes fail fast', () {
      try {
        patternOf('(?i:[abc)');
        fail('expected TokenGrammarError');
      } on TokenGrammarError catch (e) {
        expect(e.toString(), contains('Unterminated character class'));
      }
    });

    test('patterns without (? pass through identically', () {
      const body = r'[a-z]+\d(\w)';
      expect(patternOf(body), equals(body));
    });

    test('backslash-dash normalizes to hex escape', () {
      expect(patternOf(r'\-\-'), equals(r'\x2D\x2D'));
      expect(patternOf(r'a\-b'), equals(r'a\x2Db'));
      // An escaped backslash followed by a dash is untouched (the
      // dash is already bare).
      expect(patternOf(r'\\-'), equals(r'\\-'));
      // Inside classes too (where a bare `-` rewrite could form a
      // range, so the hex form is required).
      expect(patternOf(r'[\-a]'), equals(r'[\x2Da]'));
    });

    test('payload escapes inside (?i:) fail fast', () {
      try {
        patternOf(r'(?i:\p{L}+)');
        fail('expected TokenGrammarError');
      } on TokenGrammarError catch (e) {
        expect(e.toString(), contains('needs explicit case handling'));
      }
    });

    test('every expanded pattern compiles as a RegExp', () {
      for (final body in [
        '(?i:erDiagram)',
        r'(?i:\|\||only[ \t]+one)',
        '(?i:[a-z]+)',
        '(?i:a(b)c)',
      ]) {
        final expanded = patternOf(body);
        expect(expanded.contains('?i:'), isFalse);
        // Must compile (the pre-fix failure was a FormatException).
        RegExp('(?:$expanded)', unicode: true, dotAll: true);
      }
    });

    test('expanded matching agrees case-insensitively', () {
      final expanded = patternOf('(?i:erDiagram)');
      final regex = RegExp('^(?:$expanded)');
      for (final spelling in ['erDiagram', 'ERDIAGRAM', 'ErDiAgRaM']) {
        expect(regex.hasMatch(spelling), isTrue, reason: spelling);
      }
      expect(regex.hasMatch('erX'), isFalse);
    });
  });
}
