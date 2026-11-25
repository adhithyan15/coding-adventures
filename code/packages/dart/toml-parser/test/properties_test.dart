/// Property tests for `coding_adventures_toml_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, positions, both exact depth
/// boundaries across wide ranges, valid-document parsing, and API
/// agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_toml_parser/toml_parser.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// TOML-dense hostile text: every TOML sigil plus whitespace and
/// out-of-alphabet noise.
Gen<String> hostileToml(int minLen, int maxLen) {
  const pieces = [
    '[',
    ']',
    '[[',
    ']]',
    '{',
    '}',
    '=',
    ',',
    '.',
    '"',
    "'",
    "'''",
    '"""',
    '#',
    'a',
    'Z',
    'key',
    'true',
    'false',
    '0',
    '9',
    '3.14',
    '-',
    '+',
    ':',
    'T',
    'Z',
    '_',
    '-',
    '\\',
    ';',
    r'$',
    '@',
    '!',
    '%',
    '\n',
    '\r',
    ' ',
    '\t',
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

String nestedArraySource(int n) => 'a = ${'[' * n}1${']' * n}';

String nestedInlineTableSource(int n) {
  final buf = StringBuffer('a = ');
  for (var i = 0; i < n; i++) {
    if (i > 0) {
      buf.write('b = ');
    }
    buf.write('{ ');
  }
  buf.write('c = 1');
  for (var i = 0; i < n; i++) {
    buf.write(' }');
  }
  return buf.toString();
}

void main() {
  group('properties', () {
    test('parse fails closed with TomlParseError only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(5801),
          PbtOptions(numTests: 400, seed: BigInt.from(5801)),
          (rng) => hostileToml(0, 60).sample(rng),
          (input) {
            try {
              parseToml(input);
            } on TomlParseError catch (e) {
              return e == TomlParseError.lexical || e == TomlParseError.syntax;
            }
            return true;
          },
        ),
      );
    });

    test('successful parses root at document', () {
      check(
        'root-shape',
        checkProperty<String>(
          BigInt.from(5802),
          PbtOptions(numTests: 400, seed: BigInt.from(5802)),
          (rng) => hostileToml(0, 60).sample(rng),
          (input) {
            final ast = tryParseToml(input);
            if (ast == null) return true;
            return ast.ruleName == 'document';
          },
        ),
      );
    });

    test('parse is deterministic', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(5803),
          PbtOptions(numTests: 200, seed: BigInt.from(5803)),
          (rng) => hostileToml(0, 60).sample(rng),
          (input) {
            final a = tryParseToml(input);
            final b = tryParseToml(input);
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
          BigInt.from(5804),
          PbtOptions(numTests: 400, seed: BigInt.from(5804)),
          (rng) => hostileToml(0, 60).sample(rng),
          (input) {
            final ast = tryParseToml(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('array boundary is exactly fifty-three levels', () {
      check(
        'depth-boundary-arrays',
        checkProperty<int>(
          BigInt.from(5805),
          PbtOptions(numTests: 60, seed: BigInt.from(5805)),
          (rng) => smallInt(0, 100).sample(rng),
          (depth) {
            return (tryParseToml(nestedArraySource(depth)) != null) ==
                (depth <= 53);
          },
        ),
      );
    });

    test('inline table boundary is exactly fifty-three levels', () {
      check(
        'depth-boundary-tables',
        checkProperty<int>(
          BigInt.from(5806),
          PbtOptions(numTests: 60, seed: BigInt.from(5806)),
          // Depth 0 degenerates to `a = c = 1`, which is invalid TOML.
          (rng) => smallInt(1, 100).sample(rng),
          (depth) {
            return (tryParseToml(nestedInlineTableSource(depth)) != null) ==
                (depth <= 53);
          },
        ),
      );
    });

    test('valid documents always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5807),
          PbtOptions(numTests: 100, seed: BigInt.from(5807)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const documents = [
              '',
              '# only a comment',
              'title = "TOML Example"',
              'a = 1\nb = 2\nc = 3',
              '[server]\nhost = "localhost"\nport = 8080',
              '[[products]]\nname = "Hammer"',
              'point = { x = 1, y = 2 }',
              'colors = ["red", "green", "blue"]',
              'pi = 3.14\nenabled = true\ndob = 1979-05-27T07:32:00Z',
              'a.b.c = 1\n["quoted key"]\nvalue = 2',
            ];
            return tryParseToml(documents[index]) != null;
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(5808),
          PbtOptions(numTests: 400, seed: BigInt.from(5808)),
          (rng) => hostileToml(0, 60).sample(rng),
          (input) {
            final soft = tryParseToml(input);
            try {
              final hard = parseToml(input);
              return soft != null && soft == hard;
            } on TomlParseError {
              return soft == null;
            }
          },
        ),
      );
    });
  });
}
