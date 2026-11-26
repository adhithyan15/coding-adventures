/// Property tests for `coding_adventures_xml_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text
/// (EOF shape, determinism, monotonic positions, closed failure
/// vocabulary) and structural balance over generated well-formed
/// documents — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_xml_lexer/xml_lexer.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

Gen<String> wordGen(int minLen, int maxLen) {
  const alphabet = 'abcdefghijklmnopqrstuvwxyz0123456789';
  return Gen.list(minLen, maxLen, smallInt(0, alphabet.length - 1))
      .map((idx) => idx.map((i) => alphabet[i]).join());
}

/// Markup-dense hostile text: every XML sigil plus whitespace,
/// quotes, and a little Unicode.
Gen<String> hostileXml(int minLen, int maxLen) {
  const pieces = [
    '<',
    '>',
    '/',
    '=',
    '!',
    '?',
    '-',
    '[',
    ']',
    '&',
    ';',
    '#',
    '"',
    "'",
    ':',
    '.',
    ' ',
    '\n',
    '\r',
    '\t',
    'a',
    'Z',
    '0',
    '<!--',
    '-->',
    '<![CDATA[',
    ']]>',
    '<?',
    '?>',
    '</',
    '/>',
    '&amp;',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

List<Token>? tryTokenize(String source) {
  try {
    return tokenizeXml(source);
  } on LexerError {
    return null;
  }
}

int countType(List<Token> tokens, String type) =>
    tokens.where((t) => t.type == type).length;

/// A small well-formed document generator: elements with attributes,
/// comments, CDATA, PIs, entities, and nesting.
Gen<String> documentGen() {
  String element(String tag, String body) => '<$tag>$body</$tag>';
  // Tags, attributes, and PI targets must start with a letter.
  final tagGen = Gen.tuple2(smallInt(0, 25), wordGen(0, 4))
      .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}');
  final text = wordGen(0, 8);
  final entity = Gen.constant('&amp;');
  final comment = wordGen(0, 8).map((w) => '<!--$w-->');
  final cdata = wordGen(0, 8).map((w) => '<![CDATA[$w]]>');
  final pi = tagGen.map((w) => '<?$w data?>');
  final attr =
      Gen.tuple2(tagGen, wordGen(0, 6)).map((t) => ' ${t.$1}="${t.$2}"');
  final leaf = Gen.oneOf([text, entity, comment, cdata, pi]);
  final nested = Gen.tuple2(tagGen, leaf).map((t) => element(t.$1, t.$2));
  final withAttr = Gen.tuple3(tagGen, attr, leaf)
      .map((t) => '<${t.$1}${t.$2}>${t.$3}</${t.$1}>');
  return Gen.oneOf([leaf, nested, withAttr, Gen.constant('<br/>')]);
}

void main() {
  group('properties', () {
    test('tokenize fails closed with LexerError only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(2301),
          PbtOptions(numTests: 400, seed: BigInt.from(2301)),
          (rng) => hostileXml(0, 60).sample(rng),
          (input) {
            try {
              tokenizeXml(input);
            } on LexerError {
              return true;
            }
            return true;
          },
        ),
      );
    });

    test('successful runs end in exactly one EOF', () {
      check(
        'eof-shape',
        checkProperty<String>(
          BigInt.from(2302),
          PbtOptions(numTests: 400, seed: BigInt.from(2302)),
          (rng) => hostileXml(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenize(input);
            if (tokens == null) return true;
            return tokens.isNotEmpty &&
                tokens.last.type == 'EOF' &&
                countType(tokens, 'EOF') == 1;
          },
        ),
      );
    });

    test('tokenize is deterministic', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(2303),
          PbtOptions(numTests: 200, seed: BigInt.from(2303)),
          (rng) => hostileXml(0, 60).sample(rng),
          (input) {
            final a = tryTokenize(input);
            final b = tryTokenize(input);
            if (a == null || b == null) return a == null && b == null;
            if (a.length != b.length) return false;
            for (var i = 0; i < a.length; i++) {
              if (a[i] != b[i]) return false;
            }
            return true;
          },
        ),
      );
    });

    test('positions are sane and monotonic', () {
      check(
        'positions',
        checkProperty<String>(
          BigInt.from(2304),
          PbtOptions(numTests: 400, seed: BigInt.from(2304)),
          (rng) => hostileXml(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenize(input);
            if (tokens == null) return true;
            var line = 1;
            var column = 1;
            for (final t in tokens) {
              if (t.line < 1 || t.column < 1) return false;
              if (t.line < line) return false;
              if (t.line == line && t.column < column) return false;
              line = t.line;
              column = t.column;
            }
            return true;
          },
        ),
      );
    });

    test('delimiters pair up in well-formed documents', () {
      check(
        'delimiter-balance',
        checkProperty<String>(
          BigInt.from(2305),
          PbtOptions(numTests: 300, seed: BigInt.from(2305)),
          (rng) => documentGen().sample(rng),
          (input) {
            final tokens = tryTokenize(input);
            if (tokens == null) return false;
            // Every opener (`<x>`, `</x>`, `<x/>`) carries exactly one
            // closer (`>` or `/>`).
            final opens = countType(tokens, 'OPEN_TAG_START') +
                countType(tokens, 'CLOSE_TAG_START');
            final closes = countType(tokens, 'TAG_CLOSE') +
                countType(tokens, 'SELF_CLOSE');
            if (opens != closes) return false;
            if (countType(tokens, 'COMMENT_START') !=
                countType(tokens, 'COMMENT_END')) {
              return false;
            }
            if (countType(tokens, 'CDATA_START') !=
                countType(tokens, 'CDATA_END')) {
              return false;
            }
            if (countType(tokens, 'PI_START') != countType(tokens, 'PI_END')) {
              return false;
            }
            return countType(tokens, 'PI_TARGET') <=
                countType(tokens, 'PI_START');
          },
        ),
      );
    });

    test('entities lex with exact values', () {
      check(
        'entity-exactness',
        checkProperty<String>(
          BigInt.from(2306),
          PbtOptions(numTests: 200, seed: BigInt.from(2306)),
          // Entity names start with a letter (grammar rule).
          (rng) => Gen.tuple2(smallInt(0, 25), wordGen(0, 7))
              .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}')
              .sample(rng),
          (name) {
            final tokens = tokenizeXml('a&$name;b');
            if (tokens.length != 4) return false; // TEXT ENTITY TEXT EOF
            return tokens[0] ==
                    Token(type: 'TEXT', value: 'a', line: 1, column: 1) &&
                tokens[1].type == 'ENTITY_REF' &&
                tokens[1].value == '&$name;';
          },
        ),
      );
    });

    test('attribute values keep their quotes', () {
      check(
        'attr-quotes',
        checkProperty<(String, String)>(
          BigInt.from(2307),
          PbtOptions(numTests: 200, seed: BigInt.from(2307)),
          (rng) {
            // Attribute names start with a letter (grammar rule).
            final name = Gen.tuple2(smallInt(0, 25), wordGen(0, 5))
                .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}')
                .sample(rng);
            final value = wordGen(0, 8).sample(rng);
            return (name, value);
          },
          (input) {
            final (name, value) = input;
            for (final quote in ['"', "'"]) {
              final tokens = tokenizeXml('<a $name=$quote$value$quote>');
              final values = tokens.where((t) => t.type == 'ATTR_VALUE');
              if (values.length != 1) return false;
              if (values.single.value != '$quote$value$quote') {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('comment and cdata bodies reassemble exactly', () {
      check(
        'body-reassembly',
        checkProperty<String>(
          BigInt.from(2308),
          PbtOptions(numTests: 200, seed: BigInt.from(2308)),
          (rng) => wordGen(0, 12).sample(rng),
          (body) {
            String collect(String open, String close, String type) {
              final tokens = tokenizeXml('$open$body$close');
              return tokens
                  .where((t) => t.type == type)
                  .map((t) => t.value)
                  .join();
            }

            return collect('<!--', '-->', 'COMMENT_TEXT') == body &&
                collect('<![CDATA[', ']]>', 'CDATA_TEXT') == body;
          },
        ),
      );
    });
  });
}
