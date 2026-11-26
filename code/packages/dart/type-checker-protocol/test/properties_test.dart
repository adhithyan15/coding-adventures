/// Property tests for `coding_adventures_type_checker_protocol`, using the
/// in-lane `pbt` package. Every property builds randomized hook sets and
/// nodes, then checks the dispatch contract (self-routing, first-match-wins,
/// wildcard fallback, argument forwarding, check freshness) with fixed seeds.
library;

import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_type_checker_protocol/coding_adventures_type_checker_protocol.dart';
import 'package:test/test.dart';

class _Node {
  final String kind;
  _Node(this.kind);
}

/// A hook registration: [kind] may be any string (including `'*'`); only
/// [behavior] decides what the hook does:
///
/// * 0 — decline via `notHandled()`;
/// * 1 — return [marker];
/// * 2 — record `error('e\$marker')`, then decline;
/// * 3 — record `error('e\$marker')`, then return [marker].
class _HookSpec {
  final String kind;
  final int behavior;
  final int marker;
  const _HookSpec(this.kind, this.behavior, this.marker);
}

class _FlexChecker extends GenericTypeChecker<_Node> {
  Object? lastResult;
  List<Object?>? lastArgs;
  List<Object?> callArgs = const [];

  _FlexChecker(List<_HookSpec> specs, String phase) {
    for (final spec in specs) {
      registerHook(phase, spec.kind, (node, args) {
        lastArgs = List.of(args);
        switch (spec.behavior) {
          case 0:
            return notHandled();
          case 1:
            return spec.marker;
          case 2:
            error('e${spec.marker}', node);
            return notHandled();
          default:
            error('e${spec.marker}', node);
            return spec.marker;
        }
      });
    }
  }

  @override
  void run(_Node ast) {
    lastResult = dispatch('node', ast, callArgs);
  }

  @override
  String? nodeKind(_Node node) => node.kind;
}

/// The dispatch rule, stated independently: scan [specs] in order for the
/// first behavior in {1, 3}; its marker wins. Every behavior in {2, 3} met
/// before the winner (or all of them, when nobody wins) records one error.
(Object?, int) _firstWinsModel(List<_HookSpec> specs) {
  var errors = 0;
  for (final spec in specs) {
    if (spec.behavior == 2 || spec.behavior == 3) errors++;
    if (spec.behavior == 1 || spec.behavior == 3) {
      return (spec.marker, errors);
    }
  }
  return (null, errors);
}

/// Arbitrary kind strings: empty, blanks, punctuation, high bytes.
Gen<String> _anyKind() => Gen.bytes(0, 6).map(String.fromCharCodes);

/// Alphanumeric-only strings, which normalize to themselves.
Gen<String> _alnumKind() => Gen.list(
    1,
    5,
    Gen.integer(BigInt.zero, BigInt.from(61)).map((n) {
      final v = n.toInt();
      if (v < 10) return String.fromCharCode(0x30 + v);
      if (v < 36) return String.fromCharCode(0x41 + v - 10);
      return String.fromCharCode(0x61 + v - 36);
    })).map((chars) => chars.join());

Gen<_HookSpec> _hookSpec(Gen<String> kinds) => Gen.tuple3(
        kinds,
        Gen.integer(BigInt.zero, BigInt.from(3)).map((n) => n.toInt()),
        Gen.u8())
    .map((t) => _HookSpec(t.$1, t.$2, t.$3));

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('a hook always routes to its own kind spelling', () {
      final gen = _anyKind();
      final verdict = checkProperty<String>(
        BigInt.from(21),
        PbtOptions(numTests: 200, seed: BigInt.from(21)),
        (rng) => gen.sample(rng),
        (kind) {
          final checker = _FlexChecker(
              [_HookSpec(kind, 1, 7), _HookSpec('*', 1, 9)], 'node');
          checker.check(_Node(kind));
          // The kind hook wins with 7 (the wildcard's 9 must not leak in).
          return checker.lastResult == 7;
        },
      );
      _check('self-routing', verdict);
    });

    test('first non-declined hook wins (model)', () {
      final kinds = _anyKind();
      final gen = Gen.tuple2(kinds, Gen.list(0, 5, _hookSpec(kinds)));
      final verdict = checkProperty<(String, List<_HookSpec>)>(
        BigInt.from(22),
        PbtOptions(numTests: 200, seed: BigInt.from(22)),
        (rng) => gen.sample(rng),
        (input) {
          // All hooks share the node's exact kind spelling, so routing is
          // by order alone (normalization is consistent by construction).
          final nodeKind = input.$1;
          final specs = input.$2
              .map((s) => _HookSpec(nodeKind, s.behavior, s.marker))
              .toList();
          final checker = _FlexChecker(specs, 'node');
          final result = checker.check(_Node(nodeKind));
          final (wantValue, wantErrors) = _firstWinsModel(specs);
          return checker.lastResult == wantValue &&
              result.errors.length == wantErrors &&
              result.ok == (wantErrors == 0);
        },
      );
      _check('first-wins', verdict);
    });

    test('unmatched nodes fall through to wildcard hooks (model)', () {
      final gen =
          Gen.tuple2(_alnumKind(), Gen.list(0, 4, _hookSpec(_alnumKind())));
      final verdict = checkProperty<(String, List<_HookSpec>)>(
        BigInt.from(23),
        PbtOptions(numTests: 200, seed: BigInt.from(23)),
        (rng) => gen.sample(rng),
        (input) {
          // Alphanumeric spellings normalize to themselves, so kind hooks
          // can only collide by exact equality; force the miss.
          final nodeKind = input.$1;
          final kindSpecs = input.$2
              .map((s) => _HookSpec(s.kind == nodeKind ? '${s.kind}x' : s.kind,
                  s.behavior, s.marker))
              .toList();
          final wildSpecs = [
            _HookSpec('*', 0, 0),
            _HookSpec('*', 1, 42),
          ];
          final checker = _FlexChecker([...kindSpecs, ...wildSpecs], 'node');
          checker.check(_Node(nodeKind));
          // No kind hook can match; the first wildcard declines, the second
          // answers 42.
          return checker.lastResult == 42;
        },
      );
      _check('wildcard-fallback', verdict);
    });

    test('dispatch forwards args intact', () {
      final gen = Gen.list(0, 4, Gen.u8());
      final verdict = checkProperty<List<int>>(
        BigInt.from(24),
        PbtOptions(numTests: 200, seed: BigInt.from(24)),
        (rng) => gen.sample(rng),
        (args) {
          final checker = _FlexChecker([_HookSpec('k', 1, 0)], 'node');
          checker.callArgs = List.of(args);
          checker.check(_Node('k'));
          final received = checker.lastArgs;
          return received != null &&
              received.length == args.length &&
              [for (var i = 0; i < args.length; i++) received[i] == args[i]]
                  .every((e) => e);
        },
      );
      _check('args-forwarding', verdict);
    });

    test('checks are fresh: errors never leak across runs', () {
      final gen = Gen.tuple2(_anyKind(), _anyKind());
      final verdict = checkProperty<(String, String)>(
        BigInt.from(25),
        PbtOptions(numTests: 200, seed: BigInt.from(25)),
        (rng) => gen.sample(rng),
        (input) {
          final checker = _FlexChecker([_HookSpec(input.$1, 2, 5)], 'node');
          final first = checker.check(_Node(input.$1));
          final second = checker.check(_Node('${input.$1}\u{0}'));
          // First run records exactly one error; the second run starts
          // empty (its own outcome depends only on its own dispatch).
          final secondErrors = second.errors.length;
          final third = checker.check(_Node('zzz-unrelated-alnum'));
          return first.errors.length == 1 &&
              secondErrors <= 1 &&
              third.errors.isEmpty &&
              third.ok &&
              identical(third.typedAst.kind, 'zzz-unrelated-alnum');
        },
      );
      _check('freshness', verdict);
    });

    test('typedAst is identical to the input and ok matches errors', () {
      final gen = Gen.tuple2(_anyKind(), Gen.list(0, 4, _hookSpec(_anyKind())));
      final verdict = checkProperty<(String, List<_HookSpec>)>(
        BigInt.from(26),
        PbtOptions(numTests: 200, seed: BigInt.from(26)),
        (rng) => gen.sample(rng),
        (input) {
          final node = _Node(input.$1);
          final checker = _FlexChecker(input.$2, 'node');
          final result = checker.check(node);
          return identical(result.typedAst, node) &&
              result.ok == result.errors.isEmpty;
        },
      );
      _check('result-shape', verdict);
    });
  });
}
