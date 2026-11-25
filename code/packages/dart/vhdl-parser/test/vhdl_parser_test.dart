import 'dart:math' as math;

import 'package:coding_adventures_vhdl_lexer/coding_adventures_vhdl_lexer.dart'
    show VhdlLexerException, tokenizeVhdl;
import 'package:coding_adventures_vhdl_parser/coding_adventures_vhdl_parser.dart';
import 'package:test/test.dart';

Token _kw(String value, [int column = 1]) =>
    Token(type: 'KEYWORD', value: value, line: 1, column: column);

Token _tok(String type, String value, [int column = 1]) =>
    Token(type: type, value: value, line: 1, column: column);

/// Wraps [statements] in an architecture (not a counted nesting level).
String _arch(String statements) =>
    'architecture a of e is begin $statements end architecture a;';

/// Wraps [body] in a process, which opens exactly one nesting level.
String _proc(String body) => _arch('p: process begin $body end process p;');

Matcher _nestingTooDeep() => throwsA(
      isA<VhdlParseException>()
          .having(
            (e) => e.errorCode,
            'errorCode',
            VhdlParserErrorCode.nestingTooDeep,
          )
          .having((e) => e.errorId, 'errorId', errorIdNestingTooDeep),
    );

Matcher _syntaxError() => throwsA(
      isA<VhdlParseException>().having(
        (e) => e.errorCode,
        'errorCode',
        VhdlParserErrorCode.syntaxError,
      ),
    );

String _nestedIfs(int n) => '${'if a then ' * n}x <= 1; ${'end if; ' * n}';

void main() {
  group('5-Class VHDL Parser Security Resilience Showcase', () {
    test('Class 1: Version Resolution & Domain Isolation', () {
      expect(resolveVhdlParserVersion(), equals('2008'));
      expect(resolveVhdlParserVersion(null), equals('2008'));
      expect(resolveVhdlParserVersion('  '), equals('2008'));
      expect(supportedVhdlParserVersions,
          equals(['1987', '1993', '2002', '2008', '2019']));

      for (final year in supportedVhdlParserVersions) {
        expect(resolveVhdlParserVersion(year), equals(year));
        expect(resolveVhdlParserVersion('vhdl$year'), equals(year));
        expect(resolveVhdlParserVersion(' VHDL$year '), equals(year));
        expect(
          parseVhdl('ENTITY e IS END ENTITY e;', version: year).ruleName,
          equals('design_file'),
        );
        expect(loadVhdlParserGrammar(version: year).rules, isNotEmpty);
      }

      for (final invalid in ['vhdl', '2005', 'verilog', 'vhdl-2008', '08']) {
        expect(
          () => resolveVhdlParserVersion(invalid),
          throwsA(
            isA<InvalidVhdlParserVersionException>().having(
              (e) => e.errorId,
              'errorId',
              errorIdInvalidVersion,
            ),
          ),
        );
        expect(
          () => parseVhdl('entity e is end;', version: invalid),
          throwsA(isA<InvalidVhdlParserVersionException>()),
        );
      }
    });

    test('Class 2: CWE-400 Resource Exhaustion via Token Ceiling', () {
      final oversizedTokens = List.generate(
        maxTokenCount + 1,
        (i) => _tok('NAME', 'n$i', i + 1),
      );

      expect(
        () => parseVhdlTokens(oversizedTokens),
        throwsA(
          isA<VhdlParseException>()
              .having(
                (e) => e.errorCode,
                'errorCode',
                VhdlParserErrorCode.tokenCountExceeded,
              )
              .having((e) => e.errorId, 'errorId', errorIdTokenCountExceeded),
        ),
      );
    });

    test('Class 3a: CWE-674 Parenthesis Depth (no delimiter-only bypass)', () {
      String parens(int n) => _arch('y <= ${'(' * n}a${')' * n};');

      // At the ceiling: stack-safe success.
      expect(parseVhdl(parens(maxNestingDepth)).ruleName, 'design_file');
      expect(() => parseVhdl(parens(maxNestingDepth + 1)), _nestingTooDeep());
    });

    test('Class 3b: CWE-674 Keyword-Only Nesting (if/loop/case/generate)', () {
      // The process itself opens one level.
      final room = maxNestingDepth - 1;
      expect(parseVhdl(_proc(_nestedIfs(room))).ruleName, 'design_file');
      expect(() => parseVhdl(_proc(_nestedIfs(room + 1))), _nestingTooDeep());

      String loops(int n) =>
          '${'for i in 0 to 1 loop ' * n}x <= 1; ${'end loop; ' * n}';
      expect(parseVhdl(_proc(loops(room))).ruleName, 'design_file');
      expect(() => parseVhdl(_proc(loops(room + 1))), _nestingTooDeep());

      String cases(int n) => '${'case s is when others => ' * n}x <= 1; '
          '${'end case; ' * n}';
      expect(parseVhdl(_proc(cases(room))).ruleName, 'design_file');
      expect(() => parseVhdl(_proc(cases(room + 1))), _nestingTooDeep());

      String generates(int n) => '${'g: for i in 0 to 1 generate ' * n}'
          'y <= a; ${'end generate; ' * n}';
      expect(
          parseVhdl(_arch(generates(maxNestingDepth))).ruleName, 'design_file');
      expect(() => parseVhdl(_arch(generates(maxNestingDepth + 1))),
          _nestingTooDeep());

      // Unclosed openers still count: no `end` is required to hit the ceiling.
      expect(
        () => parseVhdl(_proc('if a then ' * (maxNestingDepth + 1))),
        _nestingTooDeep(),
      );
    });

    test('Class 3c: CWE-674 Prefix-Operator Chains and Mixed Nesting', () {
      final room = maxNestingDepth - 1;
      expect(
        parseVhdl(_proc('x := ${'not ' * room}a;')).ruleName,
        'design_file',
      );
      expect(
        () => parseVhdl(_proc('x := ${'not ' * (room + 1)}a;')),
        _nestingTooDeep(),
      );
      expect(
        () => parseVhdl(_proc('x := ${'- ' * (room + 1)}a;')),
        _nestingTooDeep(),
      );
      expect(
        () => parseVhdl(_proc('x := ${'abs ' * (room + 1)}a;')),
        _nestingTooDeep(),
      );

      // process (1) + ifs (31) + parens (32) == ceiling, one more is rejected.
      String mixed(int ifs, int parens) => _proc(
            '${'if a then ' * ifs}x := ${'(' * parens}a${')' * parens}; '
            '${'end if; ' * ifs}',
          );
      expect(parseVhdl(mixed(31, 32)).ruleName, 'design_file');
      expect(() => parseVhdl(mixed(31, 33)), _nestingTooDeep());
      expect(() => parseVhdl(mixed(32, 32)), _nestingTooDeep());
    });

    test('Class 3d: Balanced Constructs Never Leak Depth', () {
      // Far more sequential blocks than the ceiling, all properly closed.
      final sequentialIfs = _proc('if a then x <= 1; end if; ' * 200);
      expect(parseVhdl(sequentialIfs).ruleName, 'design_file');

      final sequentialProcesses =
          _arch('process begin x <= 1; end process; ' * 200);
      expect(parseVhdl(sequentialProcesses).ruleName, 'design_file');

      // `if ... generate` opens exactly one level through its `generate`.
      final sequentialGenerates =
          _arch('g: if c = 1 generate y <= a; end generate g; ' * 200);
      expect(parseVhdl(sequentialGenerates).ruleName, 'design_file');

      // `elsif ... then` must not disturb the unresolved-`if` bookkeeping.
      final elsifs = _proc(
        'if a then x <= 1; elsif b then x <= 2; elsif c then x <= 3; end if; ' *
            100,
      );
      expect(parseVhdl(elsifs).ruleName, 'design_file');

      // `end postponed process` closes the process it names.
      final postponed = 'postponed process begin end postponed process; ' * 200;
      expect(() => parseVhdl(_arch(postponed)), _syntaxError());
    });

    test('Class 3e: Stray Closers Cannot Hide Later Nesting', () {
      // The process level is cancelled by the first stray `end if`, so the
      // deep run that follows is measured from depth zero and still rejected.
      final strayEnds =
          _proc('end if; ' * 200 + _nestedIfs(maxNestingDepth + 1));
      expect(() => parseVhdl(strayEnds), _nestingTooDeep());

      // Closers that name no opener are ignored by the depth counter.
      final tokens = <Token>[
        _kw('end'),
        _kw('record'),
        _kw('end'),
        _kw('postponed'),
        _kw('record'),
        _kw('end'),
      ];
      expect(() => parseVhdlTokens(tokens), _syntaxError());
    });

    test('Class 3f: Stack Overflow Backstop', () {
      // Any StackOverflowError escaping the grammar parser is mapped to the
      // typed nesting error rather than crashing the caller.
      final options = GrammarParserOptions(
        trace: true,
        traceWriter: (_) => throw StackOverflowError(),
      );
      expect(
        () => parseVhdl('entity e is end entity e;', options: options),
        _nestingTooDeep(),
      );
    });

    test('Class 4: CWE-209 Payload-Blind Diagnostics & Confidentiality', () {
      try {
        parseVhdl('entity secret_payload is bogus bogus end;');
        fail('Expected VhdlParseException on invalid syntax');
      } on VhdlParseException catch (e) {
        expect(e.errorCode, equals(VhdlParserErrorCode.syntaxError));
        expect(e.errorId, equals(errorIdSyntaxError));
        expect(e.message, equals(messageSyntaxError));
        final str = e.toString();
        expect(str, contains(messageSyntaxError));
        expect(str.contains('secret'), isFalse);
        expect(str.contains('bogus'), isFalse);
        expect(e.line, isNotNull);
        expect(e.column, isNotNull);
      }

      expect(
        const VhdlParseException.tokenCountExceeded().toString(),
        equals(messageTokenCountExceeded),
      );
      expect(formatLocation('msg', 3, null), equals('msg'));
    });

    test('Class 5: Case-Folded Tokens & AST Structure', () {
      final parser = createVhdlParser('ENTITY Foo IS END ENTITY Foo;');
      expect(parser.version, equals('2008'));
      expect(
        parser.tokens.map((t) => t.value).toList(),
        equals(['entity', 'foo', 'is', 'end', 'entity', 'foo', ';', '']),
      );
      final ast = parser.parse();
      expect(ast.ruleName, equals('design_file'));
      expect(ast.children, isNotEmpty);
    });
  });

  group('VHDL Parser Comprehensive Syntax Coverage', () {
    test('Accepts representative design units', () {
      const designs = <String>[
        'entity e is end entity e;',
        'library ieee; use ieee.std_logic_1164.all; '
            'entity e is port (a, b : in std_logic; y : out std_logic); '
            'end entity e;',
        'architecture a of e is signal s : std_logic; begin y <= a and b; '
            'end architecture a;',
        'architecture a of e is begin process (clk) begin '
            "if clk = '1' then q <= d; elsif r = '1' then q <= '0'; "
            'else q <= q; end if; end process; end architecture a;',
        'architecture a of e is begin process (s) begin case s is '
            'when a => x <= 1; when others => x <= 0; end case; '
            'end process; end architecture a;',
        'architecture a of e is begin process begin '
            'while c < 10 loop c := c + 1; end loop; end process; '
            'end architecture a;',
        'package p is constant w : integer := 8; end package p;',
        'architecture a of e is component c is port (x : in std_logic); '
            'end component c; begin u: c port map (x => y); '
            'end architecture a;',
        'architecture a of e is begin process begin x := not a; '
            'y := - b + abs c; end process; end architecture a;',
      ];
      for (final design in designs) {
        expect(
          parseVhdl(design).ruleName,
          equals('design_file'),
          reason: 'Design should parse: $design',
        );
      }
    });

    test('Rejects malformed designs with typed syntax errors', () {
      for (final design in ['entity', 'entity e is', 'end;', 'architecture']) {
        expect(
          () => parseVhdl(design),
          _syntaxError(),
          reason: 'Design should fail: $design',
        );
      }
    });

    test('Lexical failures surface as typed lexer exceptions', () {
      expect(
          () => parseVhdl('entity e is ~'), throwsA(isA<VhdlLexerException>()));
    });

    test('parseVhdlTokens agrees with parseVhdl', () {
      const source = 'entity e is end entity e;';
      expect(
        parseVhdlTokens(tokenizeVhdl(source)).toString(),
        equals(parseVhdl(source).toString()),
      );
    });

    test('Parser options and parser factory', () {
      final traceOutput = <String>[];
      final options = GrammarParserOptions(
        trace: true,
        traceWriter: (msg) => traceOutput.add(msg),
      );

      final parser = createVhdlParser(
        'entity e is end entity e;',
        version: 'vhdl1993',
        options: options,
      );
      expect(parser.version, equals('1993'));
      expect(parser.parse().ruleName, equals('design_file'));
      expect(traceOutput, isNotEmpty);
    });
  });

  group('Deterministic PRNG Property & Fuzz Tests', () {
    test('200-case PRNG property test verifies valid design generation', () {
      final rng = math.Random(42);
      final names = ['e', 'cpu', 'alu', 'Counter', 'ADDER'];
      final types = ['std_logic', 'integer', 'bit'];

      for (var i = 0; i < 200; i++) {
        final name = names[rng.nextInt(names.length)];
        final type = types[rng.nextInt(types.length)];
        final version = supportedVhdlParserVersions[
            rng.nextInt(supportedVhdlParserVersions.length)];
        final design = switch (rng.nextInt(3)) {
          0 => 'entity $name is end entity $name;',
          1 => 'entity $name is port (a, b : in $type; y : out $type); '
              'end entity $name;',
          _ => 'architecture a of $name is signal s : $type; begin '
              'y <= a and b; end architecture a;',
        };
        expect(
          parseVhdl(design, version: version).ruleName,
          equals('design_file'),
          reason: 'Failed case $i ($version): $design',
        );
        expect(
          parseVhdl(design.toUpperCase(), version: version).ruleName,
          equals('design_file'),
          reason: 'Upper-cased case $i ($version): $design',
        );
      }
    });

    test(
      '10,000-iteration mutation fuzz loop ensures memory and exception safety',
      () {
        final rng = math.Random(1337);
        final seeds = [
          'entity e is end entity e;',
          'architecture a of e is begin y <= a and b; end architecture a;',
          'architecture a of e is begin process begin '
              'if a then x <= 1; end if; end process; end architecture a;',
          'package p is constant w : integer := 8; end package p;',
        ];
        const alphabet =
            'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_#"\'-<=>:;,.()&+*/ \t\n';

        for (var iter = 0; iter < 10000; iter++) {
          final chars = seeds[rng.nextInt(seeds.length)].split('');

          final numMutations = rng.nextInt(3) + 1;
          for (var m = 0; m < numMutations && chars.isNotEmpty; m++) {
            final pos = rng.nextInt(chars.length);
            switch (rng.nextInt(4)) {
              case 0:
                chars[pos] = alphabet[rng.nextInt(alphabet.length)];
              case 1:
                chars.removeAt(pos);
              case 2:
                chars.insert(pos, alphabet[rng.nextInt(alphabet.length)]);
              default:
                chars.insert(pos, rng.nextBool() ? '(' : ')');
            }
          }

          try {
            expect(parseVhdl(chars.join()).ruleName, equals('design_file'));
          } on VhdlParseException {
            // Expected typed syntax error or limit violation.
          } on VhdlLexerException {
            // Expected typed lexical error.
          }
        }
      },
    );
  });
}
