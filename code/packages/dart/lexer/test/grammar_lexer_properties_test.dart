/// Property tests for the [GrammarLexer] group/callback engine, using
/// the in-lane `pbt` package: the fail-closed vocabulary over hostile
/// grammars and sources, group-stack balance, the EOF invariant,
/// callback-trace consistency, and purity — all with fixed seeds.
library;

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

/// Hostile but well-formed grammars: flat definitions, skips, and two
/// named groups over safe pieces.
Gen<TokenGrammar> grammarGen() {
  final defs = Gen.list(1, 4, Gen.integer(BigInt.zero, BigInt.from(1000000)));
  return Gen.tuple3(defs, Gen.boolean(), Gen.boolean()).map((t) {
    var counter = 0;
    TokenDefinition next(String prefix) {
      final pick = counter++;
      return TokenDefinition(
          name: '$prefix$pick',
          pattern: _pieces[t.$1[pick % t.$1.length].toInt() % _pieces.length],
          isRegex: true,
          lineNumber: 1);
    }

    final definitions = [for (var i = 0; i < 2; i++) next('D')];
    final skips = t.$2 ? <TokenDefinition>[] : [next('S')];
    final groups = {
      'g1': PatternGroup(
          name: 'g1', definitions: [for (var i = 0; i < 2; i++) next('G')]),
      'g2': PatternGroup(
          name: 'g2', definitions: [for (var i = 0; i < 1; i++) next('H')]),
    };
    return TokenGrammar(
      escapeMode: t.$3 ? 'none' : null,
      skipDefinitions: skips,
      definitions: definitions,
      groups: groups,
    );
  });
}

const _pieces = [
  'a',
  'b',
  '<',
  '>',
  '=',
  '"',
  ' ',
  '[ab]+',
  '[0-9]+',
  '[^<]+',
  'a*',
  'ab?',
];

Gen<String> sourceGen() {
  const alphabet = 'ab<>= "01xy\n\t;';
  return Gen.list(
          0, 30, Gen.integer(BigInt.zero, BigInt.from(alphabet.length - 1)))
      .map((idx) => idx.map((i) => alphabet[i.toInt()]).join());
}

void main() {
  group('engine properties', () {
    test('tokenize fails closed on hostile grammars and sources', () {
      check(
        'vocabulary',
        checkProperty<(TokenGrammar, String)>(
          BigInt.from(1501),
          PbtOptions(numTests: 200, seed: BigInt.from(1501)),
          (rng) => Gen.tuple2(grammarGen(), sourceGen()).sample(rng),
          (input) {
            try {
              GrammarLexer(input.$2, input.$1).tokenize();
              return true;
            } on LexerError {
              return true;
            } on ArgumentError {
              // Unknown-group pushes from callbacks; invalid patterns at
              // construction (none generated here, but the vocabulary is
              // closed over it).
              return true;
            } on UnsupportedError {
              return true;
            } catch (_) {
              return false;
            }
          },
        ),
      );
    });

    test('EOF is last and alone', () {
      check(
        'eof invariant',
        checkProperty<(TokenGrammar, String)>(
          BigInt.from(1502),
          PbtOptions(numTests: 200, seed: BigInt.from(1502)),
          (rng) => Gen.tuple2(grammarGen(), sourceGen()).sample(rng),
          (input) {
            List<Token> tokens;
            try {
              tokens = GrammarLexer(input.$2, input.$1).tokenize();
            } catch (_) {
              return true;
            }
            if (tokens.isEmpty) return false;
            if (tokens.last.type != 'EOF') return false;
            return tokens.take(tokens.length - 1).every((t) => t.type != 'EOF');
          },
        ),
      );
    });

    test('balanced scripts keep the stack valid', () {
      // Random push/pop scripts: the stack never underflows, the active
      // group is always known, and tokenize completes.
      check(
        'stack balance',
        checkProperty<(TokenGrammar, String, List<bool>)>(
          BigInt.from(1503),
          PbtOptions(numTests: 200, seed: BigInt.from(1503)),
          (rng) => Gen.tuple3(
                  grammarGen(), sourceGen(), Gen.list(0, 12, Gen.boolean()))
              .sample(rng),
          (input) {
            final script = input.$3;
            var step = 0;
            final lexer = GrammarLexer(input.$2, input.$1);
            lexer.setOnToken((token, ctx) {
              if (ctx.activeGroup != 'default' &&
                  ctx.activeGroup != 'g1' &&
                  ctx.activeGroup != 'g2') {
                throw StateError('unknown active group');
              }
              if (step >= script.length) return;
              if (script[step++]) {
                ctx.pushGroup(step.isEven ? 'g1' : 'g2');
              } else {
                ctx.popGroup();
              }
            });
            try {
              lexer.tokenize();
              return true;
            } on LexerError {
              return true;
            } catch (_) {
              return false;
            }
          },
        ),
      );
    });

    test('callback trace matches the action model', () {
      // Replays the recorded (token, activeGroup) trace against the
      // actions each callback recorded: the engine must have applied
      // pushes/pops in order.
      check(
        'trace model',
        checkProperty<(TokenGrammar, String, List<bool>)>(
          BigInt.from(1504),
          PbtOptions(numTests: 150, seed: BigInt.from(1504)),
          (rng) => Gen.tuple3(
                  grammarGen(), sourceGen(), Gen.list(0, 12, Gen.boolean()))
              .sample(rng),
          (input) {
            final script = input.$3;
            final trace = <(String, String)>[];
            var step = 0;
            final lexer = GrammarLexer(input.$2, input.$1);
            lexer.setOnToken((token, ctx) {
              trace.add((token.type, ctx.activeGroup));
              if (step >= script.length) return;
              if (script[step++]) {
                ctx.pushGroup('g1');
              } else {
                ctx.popGroup();
              }
            });
            try {
              lexer.tokenize();
            } catch (_) {
              return true;
            }
            // Model the stack from the script and check each observed
            // active group.
            final stack = ['default'];
            var modelStep = 0;
            for (var i = 0; i < trace.length; i++) {
              if (trace[i].$2 != stack.last) return false;
              if (modelStep >= script.length) continue;
              if (script[modelStep++]) {
                stack.add('g1');
              } else if (stack.length > 1) {
                stack.removeLast();
              }
            }
            return true;
          },
        ),
      );
    });

    test('tokenize is pure over fresh lexers', () {
      check(
        'purity',
        checkProperty<(TokenGrammar, String)>(
          BigInt.from(1505),
          PbtOptions(numTests: 150, seed: BigInt.from(1505)),
          (rng) => Gen.tuple2(grammarGen(), sourceGen()).sample(rng),
          (input) {
            List<Token>? first;
            Object? firstError;
            try {
              first = GrammarLexer(input.$2, input.$1).tokenize();
            } catch (e) {
              firstError = e;
            }
            try {
              final second = GrammarLexer(input.$2, input.$1).tokenize();
              if (firstError != null || !_equalTokens(first!, second)) {
                return false;
              }
            } catch (e) {
              if (firstError == null ||
                  e.runtimeType != firstError.runtimeType) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });
  });
}

bool _equalTokens(List<Token> a, List<Token> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}
