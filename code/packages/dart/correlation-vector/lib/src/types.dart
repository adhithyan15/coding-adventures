/// The provenance data types: [Origin], [Contribution],
/// [DeletionRecord], [CVEntry], and the [CvError] carrying the
/// reference's exact `Result<String, String>` failure strings.
library;

/// Failure from a fallible [CVLog] operation. The [message] is the
/// reference's exact string error (the `String` in its
/// `Result<_, String>`), so call sites can match on documented text.
class CvError implements Exception {
  const CvError(this.message);
  final String message;
  @override
  String toString() => message;
}

/// Where and when an entity was born.
///
/// Answers "where did this thing come from before we started
/// tracking it?" [meta] is free-form JSON-compatible context.
class Origin {
  const Origin({
    required this.source,
    required this.location,
    this.timestamp,
    Map<String, Object?> meta = const <String, Object?>{},
  }) : meta = meta;

  final String source;
  final String location;
  final String? timestamp;
  final Map<String, Object?> meta;

  @override
  bool operator ==(Object other) =>
      other is Origin &&
      other.source == source &&
      other.location == location &&
      other.timestamp == timestamp &&
      _mapsEqual(other.meta, meta);

  @override
  int get hashCode => Object.hash(source, location, timestamp, _mapHash(meta));

  @override
  String toString() => 'Origin($source, $location)';
}

/// A record of one stage processing one entity.
///
/// Order is semantically meaningful: the sequence in which stages
/// processed the entity.
class Contribution {
  const Contribution({
    required this.source,
    required this.tag,
    Map<String, Object?> meta = const <String, Object?>{},
  }) : meta = meta;

  final String source;
  final String tag;
  final Map<String, Object?> meta;

  @override
  bool operator ==(Object other) =>
      other is Contribution &&
      other.source == source &&
      other.tag == tag &&
      _mapsEqual(other.meta, meta);

  @override
  int get hashCode => Object.hash(source, tag, _mapHash(meta));

  @override
  String toString() => 'Contribution($source, $tag)';
}

/// A record that an entity was intentionally removed.
///
/// The entry remains in the log permanently — this is how "why did
/// this thing disappear?" stays answerable.
class DeletionRecord {
  const DeletionRecord({
    required this.source,
    required this.reason,
    Map<String, Object?> meta = const <String, Object?>{},
  }) : meta = meta;

  final String source;
  final String reason;
  final Map<String, Object?> meta;

  @override
  bool operator ==(Object other) =>
      other is DeletionRecord &&
      other.source == source &&
      other.reason == reason &&
      _mapsEqual(other.meta, meta);

  @override
  int get hashCode => Object.hash(source, reason, _mapHash(meta));

  @override
  String toString() => 'DeletionRecord($source, $reason)';
}

/// The full record for a single CV ID: stable [id], [parentIds]
/// (empty for roots), optional [origin], ordered [contributions],
/// and an optional [deleted] tombstone.
class CVEntry {
  const CVEntry({
    required this.id,
    required this.parentIds,
    required this.origin,
    required this.contributions,
    required this.deleted,
  });

  final String id;
  final List<String> parentIds;
  final Origin? origin;
  final List<Contribution> contributions;
  final DeletionRecord? deleted;

  @override
  bool operator ==(Object other) =>
      other is CVEntry &&
      other.id == id &&
      _listsEqual(other.parentIds, parentIds) &&
      other.origin == origin &&
      _listsEqual(other.contributions, contributions) &&
      other.deleted == deleted;

  @override
  int get hashCode => Object.hash(
        id,
        Object.hashAll(parentIds),
        origin,
        Object.hashAll(contributions),
        deleted,
      );

  @override
  String toString() => 'CVEntry($id)';
}

bool _mapsEqual(Map<String, Object?> a, Map<String, Object?> b) {
  if (a.length != b.length) return false;
  for (final key in a.keys) {
    if (!b.containsKey(key) || !_valuesEqual(a[key], b[key])) return false;
  }
  return true;
}

bool _listsEqual<T>(List<T> a, List<T> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// Deep equality for JSON-compatible values (the reference compares
/// `serde_json::Value` structurally). Any [Map] counts as an object
/// here — key-vocabulary enforcement belongs to the checked
/// preflight, not to equality. (Empty literals infer
/// `Map<dynamic, dynamic>` and must still compare structurally.)
bool _valuesEqual(Object? a, Object? b) {
  if (a is Map && b is Map) {
    if (a.length != b.length) {
      return false;
    }
    for (final key in a.keys) {
      if (!b.containsKey(key) || !_valuesEqual(a[key], b[key])) {
        return false;
      }
    }
    return true;
  }
  if (a is List && b is List) {
    if (a.length != b.length) return false;
    for (var i = 0; i < a.length; i++) {
      if (!_valuesEqual(a[i], b[i])) return false;
    }
    return true;
  }
  return a == b;
}

int _mapHash(Map<String, Object?> map) {
  final keys = map.keys.toList()..sort();
  return Object.hashAll(
      [for (final k in keys) Object.hash(k, _valueHash(map[k]))]);
}

int _valueHash(Object? value) {
  if (value is Map) {
    // Sort by rendered key: a total order even for dynamic key
    // types (plain sort() would throw on mixed keys), keeping the
    // ==/hashCode contract for every structurally equal pair.
    final keys = value.keys.toList()..sort((a, b) => '$a'.compareTo('$b'));
    return Object.hashAll(
        [for (final k in keys) Object.hash(k, _valueHash(value[k]))]);
  }
  if (value is List) return Object.hashAll(value.map(_valueHash));
  return value.hashCode;
}
