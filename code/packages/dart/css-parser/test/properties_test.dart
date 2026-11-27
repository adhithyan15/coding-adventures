/// Property tests for `coding_adventures_css_parser`, using the
/// in-lane `pbt` package: the engine contract over hostile text, the
/// calc depth boundary across a wide range, positions, valid-rule
/// parsing, and factory agreement — all with fixed seeds.
///
/// The reference offers no `Result` twin (callers use factory +
/// `parse()`), so this lane has no nullable API either; [soft]
/// is a test-local helper, not a product API.
library;

import 'package:coding_adventures_css_parser/css_parser.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

/// Test-local nullable twin: `null` on [LexerError] or
/// [GrammarParseError].
ASTNode? soft(String input) {
  try {
    return parseCss(input);
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// CSS-dense hostile text: every CSS sigil plus whitespace and
/// out-of-alphabet noise.
Gen<String> hostileCss(int minLen, int maxLen) {
  const pieces = [
    '{',
    '}',
    '(',
    ')',
    '[',
    ']',
    ':',
    '::',
    ';',
    ',',
    '.',
    '#',
    '@media',
    '@supports',
    '@',
    'calc(',
    ':not(',
    '>',
    '+',
    '~',
    '*',
    '=',
    '/',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '9',
    '-',
    '_',
    '"',
    "'",
    '!',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

/// Every token in [node], in tree order.
List<Token> treeTokens(ASTNode node) {
  final out = <Token>[];
  void walk(Object current) {
    if (current is Token) {
      out.add(current);
    } else if (current is ASTNode) {
      for (final child in current.children) {
        walk(child);
      }
    }
  }

  walk(node);
  return out;
}

void main() {
  group('properties', () {
    test('parse fails closed with engine errors only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(5501),
          PbtOptions(numTests: 400, seed: BigInt.from(5501)),
          (rng) => hostileCss(0, 60).sample(rng),
          (input) {
            try {
              parseCss(input);
            } on LexerError {
              return true;
            } on GrammarParseError {
              return true;
            }
            return true;
          },
        ),
      );
    });

    test('successful parses root at stylesheet', () {
      check(
        'root-shape',
        checkProperty<String>(
          BigInt.from(5502),
          PbtOptions(numTests: 400, seed: BigInt.from(5502)),
          (rng) => hostileCss(0, 60).sample(rng),
          (input) {
            final ast = soft(input);
            if (ast == null) return true;
            return ast.ruleName == 'stylesheet';
          },
        ),
      );
    });

    test('parse is deterministic', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(5503),
          PbtOptions(numTests: 200, seed: BigInt.from(5503)),
          (rng) => hostileCss(0, 60).sample(rng),
          (input) {
            final a = soft(input);
            final b = soft(input);
            if (a == null || b == null) return a == null && b == null;
            return a == b;
          },
        ),
      );
    });

    test('tree tokens keep sane positions', () {
      check(
        'positions',
        checkProperty<String>(
          BigInt.from(5504),
          PbtOptions(numTests: 400, seed: BigInt.from(5504)),
          (rng) => hostileCss(0, 60).sample(rng),
          (input) {
            final ast = soft(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('calc boundary is exactly seventy-nine levels', () {
      check(
        'depth-boundary',
        checkProperty<int>(
          BigInt.from(5505),
          PbtOptions(numTests: 100, seed: BigInt.from(5505)),
          (rng) => smallInt(0, 100).sample(rng),
          (depth) {
            final src = 'a{width:${'calc(' * depth}1${')' * depth};}';
            return (soft(src) != null) == (depth <= 79);
          },
        ),
      );
    });

    test('valid rules always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5506),
          PbtOptions(numTests: 100, seed: BigInt.from(5506)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const rules = [
              'body { color: red; }',
              'p { color: red; margin: 0; }',
              '.warning { color: orange; }',
              '#main { color: black; }',
              '@media screen { body { color: red; } }',
              'div p { color: red; }',
              'a{width:calc(1);}',
              '',
              'h1 { color: blue; } p { color: red; }',
              ':not(a){color:red;}',
            ];
            return soft(rules[index]) != null;
          },
        ),
      );
    });

    test('qualified nesting trips exactly at thirty-four', () {
      check(
        'qual-boundary',
        checkProperty<int>(
          BigInt.from(5507),
          PbtOptions(numTests: 100, seed: BigInt.from(5507)),
          (rng) => smallInt(0, 60).sample(rng),
          (depth) {
            final src = '${'.a{' * depth}color:red;${'}' * depth}';
            return (soft(src) != null) == (depth <= 33);
          },
        ),
      );
    });

    test('at-rule nesting trips exactly at forty', () {
      check(
        'atrule-boundary',
        checkProperty<int>(
          BigInt.from(5508),
          PbtOptions(numTests: 100, seed: BigInt.from(5508)),
          (rng) => smallInt(0, 60).sample(rng),
          (depth) {
            final src = '${'@media{' * depth}a{color:red;}${'}' * depth}';
            return (soft(src) != null) == (depth <= 39);
          },
        ),
      );
    });

    test('factory agrees with the one-shot', () {
      check(
        'factory-agreement',
        checkProperty<String>(
          BigInt.from(5509),
          PbtOptions(numTests: 400, seed: BigInt.from(5509)),
          (rng) => hostileCss(0, 60).sample(rng),
          (input) {
            final oneShot = soft(input);
            ASTNode viaFactory;
            try {
              viaFactory = createCssParser(input).parse();
            } on LexerError {
              return oneShot == null;
            } on GrammarParseError {
              return oneShot == null;
            }
            return oneShot != null && oneShot == viaFactory;
          },
        ),
      );
    });
  });
}
