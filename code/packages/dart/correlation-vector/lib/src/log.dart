/// The map that holds all CV entries for one pipeline run.
library;

import 'dart:convert';

import 'package:coding_adventures_sha256/coding_adventures_sha256.dart';

import 'limits.dart';
import 'types.dart';

/// The checked state of a log: its [GraphLimits] plus the running
/// transaction [Usage].
class _CheckedState {
  _CheckedState({required this.limits, required this.usage});
  final GraphLimits limits;
  Usage usage;
}

/// The map that holds all CV entries for one pipeline run.
///
/// Travels alongside the data being processed, accumulating every
/// entity's history. Slice 1 covers mint, mutate, query, and checked
/// limits; serialization, graph validation, checked export, and the
/// journal land in slice 2.
class CVLog {
  CVLog._({
    required this.enabled,
    required BigInt? compactSequence,
    required _CheckedState? checked,
  })  : _compactSequence = compactSequence,
        _checked = checked;

  /// Creates a new, empty log. When [enabled] is false, write
  /// operations still mint IDs (and advance counters) but store
  /// nothing.
  factory CVLog({bool enabled = true}) =>
      CVLog._(enabled: enabled, compactSequence: null, checked: null);

  /// Creates an opt-in compact-v1 log: IDs are fixed-length
  /// `cv1.<16 hex>` and unique within the log; parent edges carry
  /// ancestry. The default remains hierarchical.
  factory CVLog.compact({bool enabled = true}) =>
      CVLog._(enabled: enabled, compactSequence: BigInt.zero, checked: null);

  /// Constructs a checked, fully recorded compact log with explicit
  /// [limits]. Throws [CvError] when the limits are invalid.
  factory CVLog.newCheckedCompact(GraphLimits limits) {
    limits.validate();
    final log = CVLog.compact(enabled: true);
    log._checked = _CheckedState(limits: limits, usage: const Usage());
    return log;
  }

  final Map<String, CVEntry> _entries = {};
  final List<String> _passOrder = [];
  final Set<String> _passSources = {};
  final Map<String, int> _baseCounters = {};
  final Map<String, int> _childCounters = {};
  BigInt? _compactSequence;
  _CheckedState? _checked;
  bool enabled;

  /// Immutable graph storage. Mutations must pass through methods.
  Map<String, CVEntry> get entries => Map.unmodifiable(_entries);

  /// Declared first-occurrence order of recorded stages.
  List<String> get passOrder => List.unmodifiable(_passOrder);

  bool get isEnabled => enabled;

  /// Toggles recording. A checked log cannot discard evidence by
  /// disabling storage halfway through a run.
  void setEnabled(bool value) {
    if (_checked != null && !value) {
      throw const CvError('cannot disable checked CV recording');
    }
    enabled = value;
  }

  // ------------------------------------------------------------------
  // ID generation
  // ------------------------------------------------------------------

  /// The 8-hex base for a root CV: SHA-256 of `"source:location"`
  /// truncated to 8 chars, or `"00000000"` for synthetic entities.
  static String originBase(Origin? origin) {
    if (origin == null) {
      return '00000000';
    }
    final hash = sha256Hex(utf8.encode('${origin.source}:${origin.location}'));
    return hash.substring(0, 8);
  }

  String _nextOriginId(Origin? origin) {
    if (_compactSequence != null) {
      return _nextCompactId();
    }
    return _nextRootId(originBase(origin));
  }

  String _nextRootId(String base) {
    if (_compactSequence != null) {
      // Defensive mirror of the reference's belt-and-braces check;
      // the sole caller (_nextOriginId) routes compact logs first.
      return _nextCompactId(); // coverage:ignore-line
    }
    final current = _baseCounters[base] ?? 0;
    final next = current + 1;
    if (next > 0xFFFFFFFF) {
      // TODO(slice-2): unreachable without import paths (requires a
      // u32-maxed counter); import tests will cover it.
      throw const CvError(
          'root identity sequence exhausted'); // coverage:ignore-line
    }
    final id = '$base.$next';
    _checkAllocationCollision(id);
    _baseCounters[base] = next;
    return id;
  }

  String _nextChildId(String parentId) {
    if (_compactSequence != null) {
      return _nextCompactId();
    }
    final current = _childCounters[parentId] ?? 0;
    final next = current + 1;
    if (next > 0xFFFFFFFF) {
      // TODO(slice-2): unreachable without import paths (requires a
      // u32-maxed counter); import tests will cover it.
      throw const CvError(
          'child identity sequence exhausted'); // coverage:ignore-line
    }
    final id = '$parentId.$next';
    _checkAllocationCollision(id);
    _childCounters[parentId] = next;
    return id;
  }

  String _nextCompactId() {
    // Unreachable when null: every caller checks compact mode first
    // (mirrors the reference's expect on the same option).
    final last = _compactSequence!;
    final next = last + BigInt.one;
    if (next > _u64max) {
      // TODO(slice-2): unreachable without import paths (requires a
      // u64-maxed sequence); import tests will cover it.
      throw const CvError(
          'compact identity sequence exhausted'); // coverage:ignore-line
    }
    final id = 'cv1.${next.toRadixString(16).padLeft(16, '0')}';
    _checkAllocationCollision(id);
    _compactSequence = next;
    return id;
  }

  static final BigInt _u64max = BigInt.parse('ffffffffffffffff', radix: 16);

  void _checkAllocationCollision(String id) {
    if (_entries.containsKey(id)) {
      // coverage:ignore-start
      // TODO(slice-2): unreachable without import paths (freshly
      // minted IDs cannot collide); import tests will cover it.
      throw const CvError(
          'identity allocation would overwrite an existing entry');
      // coverage:ignore-end
    }
  }

  /// The sequence number of a `cv1.<16 hex>` identity. Only the happy
  /// path runs in slice 1 (checked parent checks over minted IDs);
  /// malformed inputs arrive via snapshots in slice 2.
  static BigInt _compactIdSequence(String id) {
    if (!id.startsWith('cv1.')) {
      // TODO(slice-2): snapshot-only input; import tests cover it.
      throw const CvError(
          'invalid compact identity prefix'); // coverage:ignore-line
    }
    final sequence = _fixedHexSequence(id.substring(4));
    if (sequence == BigInt.zero) {
      // TODO(slice-2): snapshot-only input; import tests cover it.
      throw const CvError(
          'compact identity zero is reserved'); // coverage:ignore-line
    }
    return sequence;
  }

  static BigInt _fixedHexSequence(String text) {
    final valid = text.length == 16 &&
        text.codeUnits
            .every((c) => (c >= 0x30 && c <= 0x39) || (c >= 0x61 && c <= 0x66));
    if (!valid) {
      // coverage:ignore-start
      // TODO(slice-2): snapshot-only input; import tests cover it.
      throw const CvError(
          'compact identity sequence must be sixteen lowercase hex digits');
      // coverage:ignore-end
    }
    return BigInt.parse(text, radix: 16);
  }

  void _recordSource(String source) {
    if (_passSources.add(source)) {
      _passOrder.add(source);
    }
  }

  /// Deep-freezes an origin's metadata on the way in. The reference
  /// takes ownership of the map (callers cannot reuse it); Dart
  /// cannot move, so the tree is copied and every map and list is
  /// made unmodifiable — stored provenance can never be mutated
  /// through a caller-retained reference, and reads ([get],
  /// [lineage], [entries]) can safely hand out the live entries.
  /// Cyclic meta (unrepresentable upstream, where values are trees)
  /// and non-string keys fail closed with [CvError].
  static Origin? _storedOrigin(Origin? origin) => origin == null
      ? null
      : Origin(
          source: origin.source,
          location: origin.location,
          timestamp: origin.timestamp,
          meta: _freezeMeta(origin.meta));

  /// Deep-copies [meta] into unmodifiable maps and lists. Iterative
  /// (deep inputs must not overflow the stack) with memoization
  /// (shared substructure is copied once, so adversarial DAGs stay
  /// linear) and ancestor tracking (cycles fail closed). Scalars —
  /// including values outside the JSON vocabulary, which the checked
  /// preflight reports — pass through untouched.
  static Map<String, Object?> _freezeMeta(Map<String, Object?> meta) {
    final memo = Map<Object, Object?>.identity();
    final active = Set<Object>.identity();
    // (node, exit): enter-frames push their exit marker plus their
    // children's enter-frames, so every descendant completes (and
    // memoizes) before its parent's exit resolves it.
    final stack = <(Object, bool)>[(meta, false)];
    while (stack.isNotEmpty) {
      final (node, exit) = stack.removeLast();
      if (exit) {
        if (node is Map) {
          final out = <String, Object?>{};
          for (final entry in node.entries) {
            if (entry.key is! String) {
              throw const CvError('CV metadata keys must be strings');
            }
            out[entry.key as String] = _resolveFrozen(entry.value, memo);
          }
          memo[node] = Map<String, Object?>.unmodifiable(out);
        } else {
          final list = node as List;
          memo[node] = List<Object?>.unmodifiable([
            for (var i = 0; i < list.length; i++) _resolveFrozen(list[i], memo)
          ]);
        }
        active.remove(node);
        continue;
      }
      if (memo.containsKey(node)) {
        continue;
      }
      if (!active.add(node)) {
        throw const CvError('CV metadata contains a cycle');
      }
      stack.add((node, true));
      final children = node is Map ? node.values : (node as List).toList();
      for (final child in children) {
        if (child is Map || child is List) {
          stack.add((child, false));
        }
      }
    }
    return memo[meta] as Map<String, Object?>;
  }

  static Object? _resolveFrozen(Object? child, Map<Object, Object?> memo) =>
      (child is Map || child is List) ? memo[child]! : child;

  void _commitUsage(Usage? usage) {
    if (usage != null) {
      _checked!.usage = usage;
    }
  }

  // ------------------------------------------------------------------
  // Checked preflights (no-ops on generic logs; journal coupling
  // lands with the journal in slice 2)
  // ------------------------------------------------------------------

  Usage? _prospectiveNode(List<String> parents, Origin? origin) {
    final state = _checked;
    if (state == null) {
      return null;
    }
    if (!enabled) {
      // TODO(slice-2): reachable only via snapshot import with
      // enabled:false; import tests will cover it.
      throw const CvError(
          'checked CV recording is disabled'); // coverage:ignore-line
    }
    var usage = state.usage;
    usage = usage.copyWith(
        nodes: addBounded(usage.nodes, 1, state.limits.maxNodes, 'nodes'));
    usage = usage.copyWith(
        edges: addBounded(usage.edges, parents.length, state.limits.maxEdges,
            'parent edges'));
    final work = Work(state.limits.maxWork);
    work.take(1);
    final last = _compactSequence;
    if (last == null) {
      // TODO(slice-2): checked logs always carry compact state in
      // slice 1; import tests will cover it.
      throw const CvError(
          'checked CV identity state is missing'); // coverage:ignore-line
    }
    for (final parent in parents) {
      work.take(1);
      if (!_entries.containsKey(parent)) {
        throw const CvError('unknown checked CV parent');
      }
      if (_compactIdSequence(parent) > last) {
        throw const CvError('checked CV parent exceeds allocation state');
      }
    }
    final payload = _Payload(usage: usage, limits: state.limits, work: work);
    payload.origin(origin);
    return payload.usage;
  }

  Usage? _prospectiveEvent(
      String id, String source, String tag, Map<String, Object?> meta) {
    final state = _checked;
    if (state == null) {
      return null;
    }
    if (!enabled) {
      // TODO(slice-2): reachable only via snapshot import with
      // enabled:false; import tests will cover it.
      throw const CvError(
          'checked CV recording is disabled'); // coverage:ignore-line
    }
    final entry = _entries[id];
    if (entry == null) {
      throw const CvError('unknown checked CV identity');
    }
    if (entry.deleted != null) {
      throw const CvError('checked CV identity is already deleted');
    }
    var usage = state.usage;
    usage = usage.copyWith(
        events: addBounded(usage.events, 1, state.limits.maxEvents, 'events'));
    final work = Work(state.limits.maxWork);
    work.take(1);
    final payload = _Payload(usage: usage, limits: state.limits, work: work);
    payload.text(source);
    payload.text(tag);
    payload.meta(meta);
    if (!_passSources.contains(source)) {
      payload.text(source);
    }
    return payload.usage;
  }

  // ------------------------------------------------------------------
  // Write operations
  // ------------------------------------------------------------------

  /// Creates a new root CV and returns its ID (`"{base}.{n}"`, or
  /// `cv1.<seq>` in compact mode). Throws [StateError] on allocation
  /// failure; use [tryCreate] for the [CvError].
  String create(Origin? origin) {
    try {
      return tryCreate(origin);
    } on CvError catch (e) {
      throw StateError('CV identity allocation failed: ${e.message}');
    }
  }

  /// Fallible root allocation. Errors leave state unchanged.
  String tryCreate(Origin? origin) {
    // Freeze before the preflight: cyclic meta must fail here, not
    // hang the accounting walk (and never reach storage).
    final stored = _storedOrigin(origin);
    final usage = _prospectiveNode(const [], stored);
    final id = _nextOriginId(stored);
    if (enabled) {
      _entries[id] = CVEntry(
          id: id,
          parentIds: const [],
          origin: stored,
          contributions: const [],
          deleted: null);
    }
    _commitUsage(usage);
    return id;
  }

  /// Records that [source] did [tag] to [cvId]. Errors only when the
  /// entity was deleted (missing IDs and disabled logs succeed
  /// silently).
  void contribute(
      String cvId, String source, String tag, Map<String, Object?> meta) {
    final frozen = _freezeMeta(meta);
    final usage = _prospectiveEvent(cvId, source, tag, frozen);
    if (!enabled) {
      return;
    }
    final entry = _entries[cvId];
    if (entry == null) {
      return;
    }
    if (entry.deleted != null) {
      throw CvError(
          'cannot contribute to deleted CV $cvId: entity has been removed');
    }
    // Note: the reference converts the contribution index to u64 for
    // the journal record; lengths always fit on 64-bit targets, and
    // the journal lands in slice 2.
    _entries[cvId] = CVEntry(
        id: entry.id,
        parentIds: entry.parentIds,
        origin: entry.origin,
        contributions: List.unmodifiable([
          ...entry.contributions,
          Contribution(source: source, tag: tag, meta: frozen)
        ]),
        deleted: entry.deleted);
    _recordSource(source);
    _commitUsage(usage);
  }

  /// Creates a child of [parentCvId] (`"{parent}.{m}"`, or the next
  /// compact ID). Throws [StateError] on allocation failure; use
  /// [tryDerive] for the [CvError].
  String derive(String parentCvId, Origin? origin) {
    try {
      return tryDerive(parentCvId, origin);
    } on CvError catch (e) {
      throw StateError('CV identity allocation failed: ${e.message}');
    }
  }

  /// Fallible one-parent derivation. Allocation failure leaves state
  /// unchanged. Checked logs require a stored parent.
  String tryDerive(String parentCvId, Origin? origin) {
    final stored = _storedOrigin(origin);
    final usage = _prospectiveNode([parentCvId], stored);
    final id = _nextChildId(parentCvId);
    if (enabled) {
      _entries[id] = CVEntry(
          id: id,
          parentIds: List.unmodifiable([parentCvId]),
          origin: stored,
          contributions: const [],
          deleted: null);
    }
    _commitUsage(usage);
    return id;
  }

  /// Merges [parentCvIds] into one CV, preserving parent order and
  /// duplicates. Throws [StateError] on allocation failure; use
  /// [tryMerge] for the [CvError].
  String merge(List<String> parentCvIds, Origin? origin) {
    try {
      return tryMerge(parentCvIds, origin);
    } on CvError catch (e) {
      throw StateError('CV identity allocation failed: ${e.message}');
    }
  }

  /// Fallible merge. Allocation failure leaves state unchanged.
  String tryMerge(List<String> parentCvIds, Origin? origin) {
    final stored = _storedOrigin(origin);
    final usage = _prospectiveNode(parentCvIds, stored);
    final id = _nextOriginId(stored);
    if (enabled) {
      _entries[id] = CVEntry(
          id: id,
          parentIds: List.unmodifiable(List.of(parentCvIds)),
          origin: stored,
          contributions: const [],
          deleted: null);
    }
    _commitUsage(usage);
    return id;
  }

  /// Records [cvId]'s intentional removal. Throws [StateError] on
  /// failure; use [tryDelete] for the [CvError].
  void delete(
      String cvId, String source, String reason, Map<String, Object?> meta) {
    try {
      tryDelete(cvId, source, reason, meta);
    } on CvError catch (e) {
      throw StateError('CV deletion failed: ${e.message}');
    }
  }

  /// Fallible tombstone recording. Generic logs replace an existing
  /// tombstone; checked logs reject a second deletion (via the
  /// already-deleted preflight).
  void tryDelete(
      String cvId, String source, String reason, Map<String, Object?> meta) {
    final frozen = _freezeMeta(meta);
    final usage = _prospectiveEvent(cvId, source, reason, frozen);
    if (!enabled) {
      return;
    }
    final entry = _entries[cvId];
    if (entry != null) {
      _entries[cvId] = CVEntry(
          id: entry.id,
          parentIds: entry.parentIds,
          origin: entry.origin,
          contributions: entry.contributions,
          deleted:
              DeletionRecord(source: source, reason: reason, meta: frozen));
    }
    if (_checked != null) {
      _recordSource(source);
    }
    _commitUsage(usage);
  }

  /// Records that [source] examined [cvId] without changing it (a
  /// `"passthrough"` contribution). Generic logs silently ignore
  /// missing or deleted IDs; checked logs throw [StateError] there.
  void passthrough(String cvId, String source) {
    if (!enabled) {
      return;
    }
    if (_checked != null) {
      try {
        tryPassthrough(cvId, source);
      } on CvError catch (e) {
        throw StateError('CV passthrough failed: ${e.message}');
      }
    } else {
      try {
        tryPassthrough(cvId, source);
      } on CvError {
        // Ignored: recording that we saw a gone entity is harmless.
      }
    }
  }

  /// Fallible no-change event.
  void tryPassthrough(String cvId, String source) {
    contribute(cvId, source, 'passthrough', const {});
  }

  // ------------------------------------------------------------------
  // Read operations (direct walks; validation wrappers land with
  // graph validation in slice 2)
  // ------------------------------------------------------------------

  /// The full entry for [cvId], or `null` when missing (or when
  /// disabled, since nothing was stored).
  CVEntry? get(String cvId) => _entries[cvId];

  /// Ancestor IDs nearest-first (BFS over parent edges, parent-list
  /// order within each level). Empty when [cvId] is unknown.
  List<String> ancestors(String cvId) {
    if (!_entries.containsKey(cvId)) {
      return [];
    }
    final visited = <String>{cvId};
    final queue = <String>[];
    for (final parent in _entries[cvId]!.parentIds) {
      if (visited.add(parent)) {
        queue.add(parent);
      }
    }
    final result = <String>[];
    while (queue.isNotEmpty) {
      final current = queue.removeAt(0);
      result.add(current);
      final entry = _entries[current];
      if (entry == null) {
        continue;
      }
      for (final parent in entry.parentIds) {
        if (visited.add(parent)) {
          queue.add(parent);
        }
      }
    }
    return result;
  }

  /// Every ID with [cvId] in its ancestor chain, via a reverse index
  /// (children sorted by identity). Empty when [cvId] is unknown.
  List<String> descendants(String cvId) {
    if (!_entries.containsKey(cvId)) {
      return [];
    }
    final children = <String, List<String>>{};
    for (final entry in _entries.values) {
      for (final parent in entry.parentIds) {
        (children[parent] ??= []).add(entry.id);
      }
    }
    for (final list in children.values) {
      list.sort();
    }
    final visited = <String>{cvId};
    final queue = <String>[cvId];
    final result = <String>[];
    while (queue.isNotEmpty) {
      final current = queue.removeAt(0);
      for (final child in children[current] ?? const []) {
        if (visited.add(child)) {
          queue.add(child);
          result.add(child);
        }
      }
    }
    return result;
  }

  /// The ordered contributions for [cvId] (call order). Empty when
  /// unknown or disabled. The deletion record is NOT included.
  List<Contribution> history(String cvId) =>
      List.of(_entries[cvId]?.contributions ?? const []);

  /// The entity plus all ancestors, oldest first (deterministic
  /// topological order: every parent precedes its child). Empty when
  /// [cvId] is unknown. Dangling parent IDs (only constructible via
  /// unchecked derive-from-missing, which graph validation rejects
  /// in slice 2) are skipped rather than crashing the walk.
  List<CVEntry> lineage(String cvId) {
    if (!_entries.containsKey(cvId)) {
      return [];
    }
    final selected = <String>{cvId, ...ancestors(cvId)}
      ..removeWhere((id) => !_entries.containsKey(id));
    // Kahn's algorithm over the reachable set, ready set kept sorted
    // for determinism (mirrors the reference's BTreeSet).
    final indegree = <String, int>{};
    final children = <String, List<String>>{};
    for (final id in selected) {
      indegree[id] = 0;
    }
    for (final id in selected) {
      for (final parent in _entries[id]?.parentIds ?? const []) {
        if (!selected.contains(parent)) {
          continue;
        }
        indegree[id] = indegree[id]! + 1;
        (children[parent] ??= []).add(id);
      }
    }
    final ready = [
      for (final id in selected)
        if (indegree[id] == 0) id
    ]..sort();
    final result = <CVEntry>[];
    while (ready.isNotEmpty) {
      final id = ready.removeAt(0);
      result.add(_entries[id]!);
      for (final child in children[id] ?? const []) {
        indegree[child] = indegree[child]! - 1;
        if (indegree[child] == 0) {
          ready.add(child);
          ready.sort();
        }
      }
    }
    return result;
  }
}

/// The metadata preflight: walks origins/meta/values charging bytes,
/// values, depth, and work against [limits]. Mirrors the reference's
/// iterative accounting exactly (including its depth numbering: meta
/// maps charge at depth 2 for their values).
class _Payload {
  _Payload({required this.usage, required this.limits, required this.work});

  Usage usage;
  final GraphLimits limits;
  final Work work;

  void _bytes(int amount) {
    usage = usage.copyWith(
        bytes: addBounded(
            usage.bytes, amount, limits.maxMetadataBytes, 'metadata bytes'));
  }

  void text(String text) {
    if (text.length > limits.maxMetadataBytes) {
      throw const CvError('CV metadata bytes limit exceeded');
    }
    _bytes(stringBytes(text));
  }

  void value(Object? value, int depth) {
    final stack = [(value, depth)];
    while (stack.isNotEmpty) {
      final (current, currentDepth) = stack.removeLast();
      work.take(1);
      usage = usage.copyWith(
          values: addBounded(
              usage.values, 1, limits.maxMetadataValues, 'metadata values'));
      if (currentDepth > limits.maxMetadataDepth) {
        throw const CvError('CV metadata depth limit exceeded');
      }
      if (current == null) {
        _bytes(4);
      } else if (current is bool) {
        _bytes(current ? 4 : 5);
      } else if (current is num) {
        _bytes('$current'.length);
      } else if (current is String) {
        text(current);
      } else if (current is List) {
        _bytes(2);
        _bytes(current.length - 1 < 0 ? 0 : current.length - 1);
        _guardPending(stack.length, current.length);
        for (var i = current.length - 1; i >= 0; i--) {
          work.take(1);
          stack.add((current[i], currentDepth + 1));
        }
      } else if (current is Map) {
        _bytes(2);
        _bytes(current.length - 1 < 0 ? 0 : current.length - 1);
        _guardPending(stack.length, current.length);
        final keys = current.keys.toList().reversed;
        for (final key in keys) {
          // JSON objects have string keys; anything else is
          // unrepresentable upstream and fails closed here. (Empty
          // map literals infer Map<dynamic, dynamic> but are still
          // valid objects when their keys — if any — are strings.)
          if (key is! String) {
            // coverage:ignore-start
            // Defensive: _freezeMeta rejects non-string keys at every
            // entry point, so the accounting walk only ever sees
            // string keys. Kept so a future direct caller fails
            // closed instead of crashing on `text(key)`.
            throw CvError('CV metadata value has unsupported type: '
                '${current.runtimeType}');
            // coverage:ignore-end
          }
          work.take(1);
          text(key);
          _bytes(1);
          work.take(1);
          stack.add((current[key], currentDepth + 1));
        }
      } else {
        throw CvError('CV metadata value has unsupported type: '
            '${current.runtimeType}');
      }
    }
  }

  void _guardPending(int pending, int additional) {
    // Overflow-free form of the reference's checked accumulation:
    // usage stays within its cap by construction, so both
    // differences are non-negative and no addition can wrap. (The
    // reference's separate accounting-overflow error needs
    // near-2^64 magnitudes, unrepresentable in Dart ints.)
    final remaining = limits.maxMetadataValues - usage.values;
    if (pending > remaining || additional > remaining - pending) {
      throw const CvError('CV metadata values limit exceeded');
    }
  }

  void meta(Map<String, Object?> meta) {
    work.take(1);
    usage = usage.copyWith(
        values: addBounded(
            usage.values, 1, limits.maxMetadataValues, 'metadata values'));
    if (limits.maxMetadataDepth == 0) {
      throw const CvError('CV metadata depth limit exceeded');
    }
    _bytes(2);
    _bytes(meta.length - 1 < 0 ? 0 : meta.length - 1);
    _guardPending(0, meta.length);
    for (final entry in meta.entries) {
      work.take(1);
      text(entry.key);
      _bytes(1);
      value(entry.value, 2);
    }
  }

  void origin(Origin? origin) {
    if (origin == null) {
      return;
    }
    text(origin.source);
    text(origin.location);
    final timestamp = origin.timestamp;
    if (timestamp != null) {
      text(timestamp);
    }
    meta(origin.meta);
  }
}
