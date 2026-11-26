// Unit tests for the slice-2 audit envelopes: the 4 reference tests from
// `audit.rs` mirrored case-for-case, plus lane extras (message copies,
// the Null denied constructor, sink draining, equality, and mapping a
// denied envelope).

import 'dart:typed_data';

import 'package:coding_adventures_capability_cage/coding_adventures_capability_cage.dart';
import 'package:test/test.dart';

CapabilityViolationError _violation() => const CapabilityViolationError(
      category: Category.net,
      action: Action.connect,
      target: 'api.example.test:443',
      message: 'net connect denied',
    );

void main() {
  group('audit', () {
    test('allowed operation records capability and output', () {
      final operation = Operation<Uint8List>.allowed(
        Category.fs,
        Action.read,
        './input.txt',
        Uint8List.fromList([104, 101, 108, 108, 111]),
      );

      expect(operation.isAllowed, isTrue);
      expect(
        operation.output,
        equals(Uint8List.fromList([104, 101, 108, 108, 111])),
      );
      expect(
        operation.auditRecord().capabilityKey(),
        equals('fs:read:./input.txt'),
      );
      expect(operation.auditRecord().decision.wire, equals('allowed'));
    });

    test('denied operation carries violation message without output', () {
      final operation = Operation<Null>(
        record: OperationRecord.denied(_violation()),
      );

      expect(operation.isAllowed, isFalse);
      expect(operation.output, isNull);
      expect(
        operation.auditRecord().capabilityKey(),
        equals('net:connect:api.example.test:443'),
      );
      expect(
        operation.auditRecord().message,
        equals('net connect denied'),
      );
    });

    test('operation output maps without changing audit record', () {
      final operation =
          Operation<List<int>>.allowed(Category.fs, Action.list, './fixtures', [
        1,
        2,
        3,
      ]).map((items) => items.length);

      expect(operation.output, equals(3));
      expect(
        operation.auditRecord().capabilityKey(),
        equals('fs:list:./fixtures'),
      );
    });

    test('vec sink records ordered operation records', () {
      final sink = VecAuditSink();
      Operation<int>.allowed(Category.time, Action.read, 'clock', 123)
          .recordTo(sink);
      Operation<Null>(
        record: OperationRecord(
          category: Category.stdout,
          action: Action.write,
          target: 'terminal',
          decision: OperationDecision.denied,
          message: 'stdout denied',
        ),
      ).recordTo(sink);

      final records = sink.records();
      expect(records, hasLength(2));
      expect(records[0].decision, equals(OperationDecision.allowed));
      expect(records[1].decision, equals(OperationDecision.denied));
      expect(records[1].message, equals('stdout denied'));
    });

    // --- Lane extras ------------------------------------------------------

    test('withMessage returns a copy carrying the message', () {
      final original = OperationRecord.allowed(
        Category.fs,
        Action.read,
        './a.txt',
      );
      final withMessage = original.withMessage('note');

      expect(original.message, isNull);
      expect(withMessage.message, equals('note'));
      expect(withMessage.capabilityKey(), equals(original.capabilityKey()));
      expect(withMessage.decision, equals(original.decision));
    });

    test('denied constructor builds a Null envelope', () {
      final operation = Operation.denied(_violation());

      expect(operation.isAllowed, isFalse);
      expect(operation.output, isNull);
      expect(
        operation.auditRecord().capabilityKey(),
        equals('net:connect:api.example.test:443'),
      );
    });

    test('mapping a denied envelope keeps null output', () {
      final operation = Operation.denied(_violation());
      final mapped = operation.map((_) => 42);

      expect(mapped.output, isNull);
      expect(mapped.isAllowed, isFalse);
      expect(
        mapped.auditRecord(),
        equals(operation.auditRecord()),
      );
    });

    test('noop sink discards records', () {
      const sink = NoopAuditSink();
      Operation<int>.allowed(Category.time, Action.read, 'clock', 1)
          .recordTo(sink);
      // No observable state: completing without throwing is the assertion.
    });

    test('takeRecords drains the sink', () {
      final sink = VecAuditSink();
      Operation<int>.allowed(Category.time, Action.read, 'clock', 1)
          .recordTo(sink);
      Operation<int>.allowed(Category.time, Action.read, 'clock', 2)
          .recordTo(sink);

      final drained = sink.takeRecords();
      expect(drained, hasLength(2));
      expect(sink.records(), isEmpty);
      expect(
        drained.map((r) => r.capabilityKey()),
        equals(['time:read:clock', 'time:read:clock']),
      );
    });

    test('records support value equality', () {
      final a = OperationRecord.allowed(Category.fs, Action.read, './a.txt');
      final b = OperationRecord.allowed(Category.fs, Action.read, './a.txt');
      final c = OperationRecord.allowed(Category.fs, Action.read, './b.txt');

      expect(a, equals(b));
      expect(a.hashCode, equals(b.hashCode));
      expect(a, isNot(equals(c)));
    });

    test('string forms, equality, and hash codes', () {
      final record = OperationRecord.allowed(Category.fs, Action.read, './a');
      expect(
        record.toString(),
        equals('OperationRecord(fs:read:./a, allowed)'),
      );
      expect(
        record.withMessage('m').toString(),
        equals('OperationRecord(fs:read:./a, allowed, m)'),
      );

      final one = Operation<int>.allowed(Category.fs, Action.read, './a', 1);
      final two = Operation<int>.allowed(Category.fs, Action.read, './a', 1);
      final three = Operation<int>.allowed(Category.fs, Action.read, './a', 2);
      expect(one, equals(two));
      expect(one.hashCode, equals(two.hashCode));
      expect(one, isNot(equals(three)));
      expect(one.toString(), contains('Operation('));

      final sinkA = VecAuditSink();
      final sinkB = VecAuditSink();
      expect(sinkA, equals(sinkB));
      expect(sinkA.hashCode, equals(sinkB.hashCode));
      one.recordTo(sinkA);
      expect(sinkA, isNot(equals(sinkB)));
      one.recordTo(sinkB);
      expect(sinkA, equals(sinkB));
      expect(sinkA.hashCode, equals(sinkB.hashCode));
    });

    test('decision wire forms and predicates', () {
      expect(OperationDecision.allowed.wire, equals('allowed'));
      expect(OperationDecision.denied.wire, equals('denied'));
      expect(OperationDecision.allowed.toString(), equals('allowed'));
      expect(OperationDecision.allowed.isAllowed, isTrue);
      expect(OperationDecision.denied.isAllowed, isFalse);
    });
  });
}
