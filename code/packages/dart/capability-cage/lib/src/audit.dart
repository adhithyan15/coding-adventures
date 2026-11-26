/// Audit envelope types for capability-checked operations.
///
/// Every operation the cage mediates produces an [OperationRecord]: the
/// category, action, target, the cage's [OperationDecision], and an
/// optional human-readable message. Records are plain data, safe to
/// persist in audit trails.
///
/// The [Operation] envelope pairs a record with the operation's output
/// (present only when the operation was allowed). Wrappers hand records
/// to an [AuditSink]; [VecAuditSink] keeps them in memory for tests and
/// embedded runtimes, while [NoopAuditSink] deliberately discards them.
library;

import 'category.dart';
import 'errors.dart';

/// The cage decision for one attempted operation.
enum OperationDecision {
  /// The manifest allowed the operation.
  allowed('allowed'),

  /// The manifest denied the operation.
  denied('denied');

  /// Lower-case wire form. Stable across versions.
  final String wire;

  /// Creates a decision with its wire form.
  const OperationDecision(this.wire);

  /// Whether this decision allowed the operation.
  bool get isAllowed => this == OperationDecision.allowed;

  @override
  String toString() => wire;
}

/// Metadata for a capability decision, safe to persist in audit trails.
class OperationRecord {
  /// Requested category.
  final Category category;

  /// Requested action.
  final Action action;

  /// Requested target.
  final String target;

  /// The cage's decision.
  final OperationDecision decision;

  /// Optional human-readable message (always present on denial).
  final String? message;

  /// Creates a record with every field explicit.
  const OperationRecord({
    required this.category,
    required this.action,
    required this.target,
    required this.decision,
    this.message,
  });

  /// Creates an allowed record for the given capability triple.
  factory OperationRecord.allowed(
    Category category,
    Action action,
    String target,
  ) =>
      OperationRecord(
        category: category,
        action: action,
        target: target,
        decision: OperationDecision.allowed,
      );

  /// Creates a denied record from a capability violation.
  factory OperationRecord.denied(CapabilityViolationError violation) =>
      OperationRecord(
        category: violation.category,
        action: violation.action,
        target: violation.target,
        decision: OperationDecision.denied,
        message: violation.message,
      );

  /// Returns a copy of this record carrying [message].
  ///
  /// The reference mutates the record in place (`with_message` takes
  /// `mut self`); records are immutable here, so this returns a copy.
  OperationRecord withMessage(String message) => OperationRecord(
        category: category,
        action: action,
        target: target,
        decision: decision,
        message: message,
      );

  /// Whether the operation was allowed.
  bool get isAllowed => decision.isAllowed;

  /// The `category:action:target` key for this record.
  String capabilityKey() => '$category:$action:$target';

  @override
  bool operator ==(Object other) =>
      other is OperationRecord &&
      category == other.category &&
      action == other.action &&
      target == other.target &&
      decision == other.decision &&
      message == other.message;

  @override
  int get hashCode => Object.hash(category, action, target, decision, message);

  @override
  String toString() => 'OperationRecord(${capabilityKey()}, $decision'
      '${message == null ? '' : ', $message'})';
}

/// Result envelope for one capability-checked operation.
///
/// Pairs the audit [record] with the operation's [output], which is
/// present only when the operation was allowed.
class Operation<T> {
  /// The audit record for this operation.
  final OperationRecord record;

  /// The operation's output, or null when denied.
  final T? output;

  /// Creates an envelope with every field explicit.
  const Operation({required this.record, this.output});

  /// Creates an allowed envelope carrying [output].
  factory Operation.allowed(
    Category category,
    Action action,
    String target,
    T output,
  ) =>
      Operation(
        record: OperationRecord.allowed(category, action, target),
        output: output,
      );

  /// Creates a denied envelope with no output.
  ///
  /// The reference restricts this constructor to `Operation<()>`; the
  /// Dart port returns [Operation] of [Null], and wrappers build denied
  /// envelopes of any output type directly through the const constructor.
  static Operation<Null> denied(CapabilityViolationError violation) =>
      Operation<Null>(
        record: OperationRecord.denied(violation),
        output: null,
      );

  /// Maps the output through [f], keeping the audit record unchanged.
  Operation<U> map<U>(U Function(T value) f) => Operation<U>(
        record: record,
        output: output == null ? null : f(output as T),
      );

  /// Whether the operation was allowed.
  bool get isAllowed => record.isAllowed;

  /// The audit record for this operation.
  OperationRecord auditRecord() => record;

  /// Hands the audit record to [sink].
  void recordTo(AuditSink sink) {
    sink.record(record);
  }

  @override
  bool operator ==(Object other) =>
      other is Operation<T> && record == other.record && output == other.output;

  @override
  int get hashCode => Object.hash(record, output);

  @override
  String toString() => 'Operation($record, $output)';
}

/// Minimal sink contract used by wrappers and host runtimes.
abstract class AuditSink {
  /// Records one operation record.
  void record(OperationRecord record);
}

/// Audit sink for callers that deliberately discard audit events.
class NoopAuditSink implements AuditSink {
  /// Creates a discarding sink.
  const NoopAuditSink();

  @override
  void record(OperationRecord record) {}
}

/// Deterministic in-memory sink for tests and embedded runtimes.
class VecAuditSink implements AuditSink {
  /// The records collected so far, in arrival order.
  final List<OperationRecord> _records = [];

  /// Creates an empty sink.
  VecAuditSink();

  /// The collected records, in arrival order (unmodifiable view).
  List<OperationRecord> records() => List<OperationRecord>.unmodifiable(
        _records,
      );

  /// Drains and returns the collected records.
  ///
  /// Maps the reference's `into_records` (which consumes the sink);
  /// without move semantics, draining is the faithful equivalent.
  List<OperationRecord> takeRecords() {
    final drained = List<OperationRecord>.of(_records);
    _records.clear();
    return drained;
  }

  @override
  void record(OperationRecord record) {
    _records.add(record);
  }

  @override
  bool operator ==(Object other) =>
      other is VecAuditSink &&
      _records.length == other._records.length &&
      _records.asMap().entries.every(
            (entry) => entry.value == other._records[entry.key],
          );

  @override
  int get hashCode => Object.hashAll(_records);
}
