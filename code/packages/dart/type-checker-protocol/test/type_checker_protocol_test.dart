/// Unit tests for `coding_adventures_type_checker_protocol`.
///
/// Mirrors the TypeScript reference's `type-checker-protocol.test.ts`,
/// then covers the behaviors the reference suite leaves implicit: wildcard
/// routing, decline chaining, hook order, argument forwarding, error-list
/// freshness, kind normalization (through dispatch behavior), and the
/// diagnostic value type.
library;

import 'package:coding_adventures_type_checker_protocol/coding_adventures_type_checker_protocol.dart';
import 'package:test/test.dart';

class SimpleNode {
  String kind;
  String? value;
  SimpleNode(this.kind, [this.value]);
}

class SuccessChecker implements TypeChecker<SimpleNode, SimpleNode> {
  @override
  TypeCheckResult<SimpleNode> check(SimpleNode ast) {
    return TypeCheckResult(
      typedAst: SimpleNode(ast.kind, ast.value ?? 'ok'),
      errors: const [],
      ok: true,
    );
  }
}

class RuleDrivenChecker extends GenericTypeChecker<SimpleNode> {
  RuleDrivenChecker() {
    registerHook('node', 'literal', (node, args) {
      node.value = 'checked';
      return null;
    });
    registerHook('node', 'broken', (node, args) {
      error('bad node: ${node.kind}', node);
      return null;
    });
  }

  @override
  void run(SimpleNode ast) {
    dispatch('node', ast);
  }

  @override
  String? nodeKind(SimpleNode node) => node.kind;

  @override
  (int, int) locate(Object? subject) => (7, 9);
}

void main() {
  group('type-checker-protocol', () {
    test('supports simple protocol-shaped checkers', () {
      final result = SuccessChecker().check(SimpleNode('literal'));
      expect(result.ok, isTrue);
      expect(result.errors, isEmpty);
      expect(result.typedAst.value, 'ok');
    });

    test('dispatches full nodes through the generic checker hooks', () {
      final result = RuleDrivenChecker().check(SimpleNode('literal'));
      expect(result.ok, isTrue);
      expect(result.typedAst.value, 'checked');
    });

    test('reports diagnostics through the shared error helper', () {
      final result = RuleDrivenChecker().check(SimpleNode('broken'));
      expect(result.ok, isFalse);
      expect(
          result.errors,
          equals([
            const TypeErrorDiagnostic(
                message: 'bad node: broken', line: 7, column: 9),
          ]));
    });

    test('ignores unhandled node kinds cleanly', () {
      final result = RuleDrivenChecker().check(SimpleNode('unknown', 'x'));
      expect(result.ok, isTrue);
      expect(result.typedAst.value, 'x');
    });
  });

  group('EXTRA: dispatch routing', () {
    test('wildcard hooks fire when no kind hook matches', () {
      final checker = _RecordingChecker();
      final result = checker.check(SimpleNode('anything'));
      expect(result.ok, isTrue);
      expect(checker.calls, ['wildcard:anything']);
    });

    test('kind hooks beat wildcard hooks', () {
      final checker = _RecordingChecker()..addKindHook('special');
      final result = checker.check(SimpleNode('special'));
      expect(result.ok, isTrue);
      // Kind hook handled it, so the wildcard never ran.
      expect(checker.calls, ['kind:special']);
    });

    test('declined hooks fall through to the next hook', () {
      final checker = _ChainChecker(declines: 2, answer: 'third');
      final result = checker.check(SimpleNode('literal'));
      expect(result.ok, isTrue);
      expect(checker.seen, isTrue);
      expect(result.typedAst.value, 'third');
    });

    test('all-declined dispatch returns null and runs nothing else', () {
      final checker = _ChainChecker(declines: 3, answer: null);
      final result = checker.check(SimpleNode('literal'));
      expect(result.ok, isTrue);
      expect(result.typedAst.value, isNull);
    });

    test('a null hook result still counts as handled', () {
      // Like the reference (`result !== NOT_HANDLED`), only the sentinel
      // declines: a hook returning null stops the chain.
      final checker = _NullStopsChecker();
      checker.check(SimpleNode('literal'));
      expect(checker.secondRan, isFalse);
    });

    test('hooks run in registration order', () {
      final checker = _OrderChecker();
      checker.check(SimpleNode('literal'));
      expect(checker.order, ['first', 'second']);
    });

    test('dispatch forwards extra args to every hook', () {
      final checker = _ArgsChecker();
      checker.check(SimpleNode('literal'));
      expect(checker.received, [
        [1, 'two']
      ]);
    });

    test('null node kinds only reach wildcard hooks', () {
      final checker = _RecordingChecker();
      checker.check(_NullKindNode());
      expect(checker.calls, ['wildcard:null-kind']);
    });
  });

  group('EXTRA: check lifecycle', () {
    test('errors reset between checks', () {
      final checker = RuleDrivenChecker();
      expect(checker.check(SimpleNode('broken')).ok, isFalse);
      final again = checker.check(SimpleNode('literal'));
      expect(again.ok, isTrue);
      expect(again.errors, isEmpty);
    });

    test('each result holds its own error list copy', () {
      final checker = RuleDrivenChecker();
      final first = checker.check(SimpleNode('broken'));
      first.errors.clear();
      final second = checker.check(SimpleNode('broken'));
      expect(second.errors, hasLength(1));
    });

    test('typedAst is the input tree itself', () {
      final checker = RuleDrivenChecker();
      final node = SimpleNode('literal');
      final result = checker.check(node);
      expect(result.typedAst, same(node));
    });

    test('default locate pins diagnostics to (1, 1)', () {
      final checker = _DefaultLocateChecker();
      final result = checker.check(SimpleNode('boom'));
      expect(
          result.errors,
          equals([
            const TypeErrorDiagnostic(message: 'boom', line: 1, column: 1),
          ]));
    });
  });

  group('EXTRA: kind normalization', () {
    // Each pair below must route identically: the registrations use the
    // first spelling and the dispatched node the second.
    final cases = {
      'kebab-case': 'kebab case',
      'snake_case': 'snake__case',
      '__padded__': 'padded',
      'MixedCase123': 'MixedCase123',
      'dots.and:colons': 'dots and colons',
      'café': 'caf',
      'a\u{1F600}b': 'a b',
      '!!!': '???',
    };
    for (final entry in cases.entries) {
      test('registers ${entry.key} matches ${entry.value}', () {
        final checker = _ProbeChecker(entry.key);
        final result = checker.check(SimpleNode(entry.value));
        expect(result.ok, isTrue);
        expect(checker.hit, isTrue);
      });
    }

    test('distinct normalized kinds do not collide', () {
      final checker = _ProbeChecker('alpha');
      final result = checker.check(SimpleNode('beta'));
      expect(result.ok, isTrue);
      expect(checker.hit, isFalse);
    });

    test('empty kind matches only empty and blank spellings', () {
      final checker = _ProbeChecker('');
      checker.check(SimpleNode(''));
      expect(checker.hit, isTrue);
      checker.check(SimpleNode('   '));
      expect(checker.hit, isTrue);
      checker.check(SimpleNode('x'));
      expect(checker.hit, isFalse);
    });
  });

  group('EXTRA: TypeErrorDiagnostic value type', () {
    test('compares by value', () {
      const a = TypeErrorDiagnostic(message: 'm', line: 1, column: 2);
      const b = TypeErrorDiagnostic(message: 'm', line: 1, column: 2);
      const c = TypeErrorDiagnostic(message: 'm', line: 1, column: 3);
      expect(a, equals(b));
      expect(a.hashCode, b.hashCode);
      expect(a == c, isFalse);
      expect(a == 'm', isFalse);
      expect(a.toString(), contains('m'));
    });
  });
}

/// Records which hook tier handled each node.
class _RecordingChecker extends GenericTypeChecker<SimpleNode> {
  final List<String> calls = [];

  _RecordingChecker() {
    registerHook('node', '*', (node, args) {
      calls.add('wildcard:${node.kind}');
      return null;
    });
  }

  void addKindHook(String kind) {
    registerHook('node', kind, (node, args) {
      calls.add('kind:${node.kind}');
      return null;
    });
  }

  @override
  void run(SimpleNode ast) {
    dispatch('node', ast);
  }

  @override
  String? nodeKind(SimpleNode node) =>
      node.kind == 'null-kind' ? null : node.kind;
}

class _NullKindNode extends SimpleNode {
  _NullKindNode() : super('null-kind');
}

/// Declines [declines] times, then answers (or declines forever when
/// [answer] is null).
class _ChainChecker extends GenericTypeChecker<SimpleNode> {
  final int declines;
  final String? answer;
  var seen = false;

  _ChainChecker({required this.declines, required this.answer}) {
    for (var i = 0; i < declines; i++) {
      registerHook('node', 'literal', (node, args) => notHandled());
    }
    if (answer != null) {
      registerHook('node', 'literal', (node, args) {
        seen = true;
        node.value = answer;
        return answer;
      });
    }
  }

  @override
  void run(SimpleNode ast) {
    dispatch('node', ast);
  }

  @override
  String? nodeKind(SimpleNode node) => node.kind;
}

class _NullStopsChecker extends GenericTypeChecker<SimpleNode> {
  var secondRan = false;

  _NullStopsChecker() {
    registerHook('node', 'literal', (node, args) => null);
    registerHook('node', 'literal', (node, args) {
      secondRan = true;
      return null;
    });
  }

  @override
  void run(SimpleNode ast) {
    dispatch('node', ast);
  }

  @override
  String? nodeKind(SimpleNode node) => node.kind;
}

class _OrderChecker extends GenericTypeChecker<SimpleNode> {
  final List<String> order = [];

  _OrderChecker() {
    registerHook('node', 'literal', (node, args) {
      order.add('first');
      return notHandled();
    });
    registerHook('node', 'literal', (node, args) {
      order.add('second');
      return null;
    });
  }

  @override
  void run(SimpleNode ast) {
    dispatch('node', ast);
  }

  @override
  String? nodeKind(SimpleNode node) => node.kind;
}

class _ArgsChecker extends GenericTypeChecker<SimpleNode> {
  final List<List<Object?>> received = [];

  _ArgsChecker() {
    registerHook('node', 'literal', (node, args) {
      received.add(List.of(args));
      return null;
    });
  }

  @override
  void run(SimpleNode ast) {
    dispatch('node', ast, [1, 'two']);
  }

  @override
  String? nodeKind(SimpleNode node) => node.kind;
}

class _DefaultLocateChecker extends GenericTypeChecker<SimpleNode> {
  _DefaultLocateChecker() {
    registerHook('node', 'boom', (node, args) {
      error('boom', node);
      return null;
    });
  }

  @override
  void run(SimpleNode ast) {
    dispatch('node', ast);
  }

  @override
  String? nodeKind(SimpleNode node) => node.kind;
}

/// Fires [hit] when a node routes to the registered [kind].
class _ProbeChecker extends GenericTypeChecker<SimpleNode> {
  var hit = false;

  _ProbeChecker(String kind) {
    registerHook('node', kind, (node, args) {
      hit = true;
      return 'hit';
    });
  }

  @override
  void run(SimpleNode ast) {
    // Reset per check so one checker answers repeated probes.
    hit = false;
    dispatch('node', ast);
  }

  @override
  String? nodeKind(SimpleNode node) => node.kind;
}
