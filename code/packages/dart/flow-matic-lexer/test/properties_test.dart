/// Property tests for `coding_adventures_flow_matic_lexer`, using
/// the in-lane `pbt` package: the engine contract over hostile text,
/// keyword folding, name shapes, and API agreement — all with fixed
/// seeds.
library;

import 'package:coding_adventures_flow_matic_lexer/flow_matic_lexer.dart';
import 'package:coding_adventures_flow_matic_lexer/src/_grammar.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

Gen<String> wordGen(int minLen, int maxLen) {
  const alphabet = 'abcdefghijklmnopqrstuvwxyz0123456789-';
  return Gen.list(minLen, maxLen, smallInt(0, alphabet.length - 1))
      .map((idx) => idx.map((i) => alphabet[i]).join());
}

/// Markup-dense hostile text: every FLOW-MATIC sigil plus whitespace
/// and a little out-of-alphabet noise.
Gen<String> hostileFlowMatic(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '.',
    ';',
    '-',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    'COMPARE',
    'compare',
    'GO',
    'TO',
    'INPUT',
    'INVENTORY',
    '@',
    '=',
    '+',
    '*',
    '/',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

int countType(List<Token> tokens, String type) =>
    tokens.where((t) => t.type == type).length;

void main() {
  group('properties', () {
    test('tokenize fails closed with LexerError only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(3701),
          PbtOptions(numTests: 400, seed: BigInt.from(3701)),
          (rng) => hostileFlowMatic(0, 60).sample(rng),
          (input) {
            try {
              tokenizeFlowMatic(input);
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
          BigInt.from(3702),
          PbtOptions(numTests: 400, seed: BigInt.from(3702)),
          (rng) => hostileFlowMatic(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeFlowMatic(input);
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
          BigInt.from(3703),
          PbtOptions(numTests: 200, seed: BigInt.from(3703)),
          (rng) => hostileFlowMatic(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeFlowMatic(input);
            final b = tryTokenizeFlowMatic(input);
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
          BigInt.from(3704),
          PbtOptions(numTests: 400, seed: BigInt.from(3704)),
          (rng) => hostileFlowMatic(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeFlowMatic(input);
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

    test('every keyword folds to uppercase in any case', () {
      check(
        'keyword-fold',
        checkProperty<int>(
          BigInt.from(3705),
          PbtOptions(numTests: 300, seed: BigInt.from(3705)),
          (rng) => smallInt(0, tokenGrammar.keywords.length - 1).sample(rng),
          (index) {
            final keyword = tokenGrammar.keywords[index];
            final mixed = keyword
                .split('')
                .asMap()
                .entries
                .map((e) => e.key.isEven
                    ? e.value.toLowerCase()
                    : e.value.toUpperCase())
                .join();
            for (final spelled in [keyword, keyword.toLowerCase(), mixed]) {
              final tokens = tryTokenizeFlowMatic(spelled);
              if (tokens == null ||
                  tokens.length != 2 ||
                  tokens[0].type != 'KEYWORD' ||
                  tokens[0].value != keyword) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('hyphenated names lex whole with case kept', () {
      check(
        'name-whole',
        checkProperty<String>(
          BigInt.from(3706),
          PbtOptions(numTests: 200, seed: BigInt.from(3706)),
          // Strict NAME shape: `q`-led (never a keyword — none starts
          // with Q), lowercase alphanumerics, single hyphens with
          // alnum tails, never trailing. Segments use a hyphen-free
          // alphabet (`wordGen` itself emits `-`).
          (rng) {
            String seg(int min, int max) => Gen.list(min, max, smallInt(0, 35))
                .map((idx) => idx
                    .map((i) => 'abcdefghijklmnopqrstuvwxyz0123456789'[i])
                    .join())
                .sample(rng);
            final head = seg(0, 3);
            final tail = seg(1, 3);
            final extra = Gen.boolean().sample(rng) ? '-${seg(1, 3)}' : '';
            return 'q$head-$tail$extra';
          },
          (name) {
            final tokens = tryTokenizeFlowMatic(name);
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'NAME' &&
                tokens[0].value == name;
          },
        ),
      );
    });

    test('numbers lex whole', () {
      check(
        'number-whole',
        checkProperty<String>(
          BigInt.from(3707),
          PbtOptions(numTests: 200, seed: BigInt.from(3707)),
          (rng) => smallInt(0, 99999).map((i) => i.toString()).sample(rng),
          (literal) {
            final tokens = tryTokenizeFlowMatic(literal);
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'NUMBER' &&
                tokens[0].value == literal;
          },
        ),
      );
    });

    test('newlines never surface as tokens', () {
      check(
        'newline-invisible',
        checkProperty<String>(
          BigInt.from(3708),
          PbtOptions(numTests: 200, seed: BigInt.from(3708)),
          (rng) {
            // Strict NAME shape, hyphen-free segments (see name-whole).
            final head = Gen.list(0, 3, smallInt(0, 35))
                .map((idx) => idx
                    .map((i) => 'abcdefghijklmnopqrstuvwxyz0123456789'[i])
                    .join())
                .sample(rng);
            return 'q$head';
          },
          (body) {
            final flat = tryTokenizeFlowMatic('INPUT $body.');
            final wrapped = tryTokenizeFlowMatic('INPUT\n$body\n.');
            if (flat == null || wrapped == null) return false;
            String signature(List<Token> tokens) => tokens
                .where((t) => t.type != 'EOF')
                .map((t) => '${t.type}=${t.value}')
                .join('|');
            return signature(flat) == signature(wrapped);
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(3709),
          PbtOptions(numTests: 400, seed: BigInt.from(3709)),
          (rng) => hostileFlowMatic(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeFlowMatic(input);
            try {
              final hard = tokenizeFlowMatic(input);
              if (soft == null) return false;
              if (soft.length != hard.length) return false;
              for (var i = 0; i < hard.length; i++) {
                if (soft[i] != hard[i]) return false;
              }
              return true;
            } on LexerError {
              return soft == null;
            }
          },
        ),
      );
    });
  });
}
