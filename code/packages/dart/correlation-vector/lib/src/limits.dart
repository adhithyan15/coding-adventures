/// Finite resource limits ([GraphLimits]), transaction accounting
/// ([Usage], [Work]), and the metadata preflight ([_Payload]) that
/// checked logs run before every mutation.
library;

import 'dart:convert';

import 'types.dart';

/// Finite resource limits for checked graph operations. Counts
/// include repeated parent edges and deletion records; payload bytes
/// include encoded text/meta.
class GraphLimits {
  const GraphLimits({
    this.maxNodes = 1000000,
    this.maxEdges = 4000000,
    this.maxEvents = 4000000,
    this.maxMetadataValues = 1000000,
    this.maxMetadataDepth = 64,
    this.maxMetadataBytes = 128 * 1024 * 1024,
    this.maxInputBytes = 128 * 1024 * 1024,
    this.maxWork = 64000000,
    this.maxOutputBytes = 512 * 1024 * 1024,
  });

  final int maxNodes;
  final int maxEdges;
  final int maxEvents;
  final int maxMetadataValues;
  final int maxMetadataDepth;
  final int maxMetadataBytes;
  final int maxInputBytes;
  final int maxWork;
  final int maxOutputBytes;

  /// Validates configuration before constructing a checked log.
  /// Counts may be zero; metadata depth cannot exceed the safe hard
  /// ceiling (serialization is recursive).
  void validate() {
    if (maxMetadataDepth > 64) {
      throw const CvError('CV metadata depth limit cannot exceed 64');
    }
  }

  @override
  bool operator ==(Object other) =>
      other is GraphLimits &&
      other.maxNodes == maxNodes &&
      other.maxEdges == maxEdges &&
      other.maxEvents == maxEvents &&
      other.maxMetadataValues == maxMetadataValues &&
      other.maxMetadataDepth == maxMetadataDepth &&
      other.maxMetadataBytes == maxMetadataBytes &&
      other.maxInputBytes == maxInputBytes &&
      other.maxWork == maxWork &&
      other.maxOutputBytes == maxOutputBytes;

  @override
  int get hashCode => Object.hash(
        maxNodes,
        maxEdges,
        maxEvents,
        maxMetadataValues,
        maxMetadataDepth,
        maxMetadataBytes,
        maxInputBytes,
        maxWork,
        maxOutputBytes,
      );
}

/// The running transaction account of a checked log.
class Usage {
  const Usage({
    this.nodes = 0,
    this.edges = 0,
    this.events = 0,
    this.values = 0,
    this.bytes = 0,
  });

  final int nodes;
  final int edges;
  final int events;
  final int values;
  final int bytes;

  Usage copyWith({
    int? nodes,
    int? edges,
    int? events,
    int? values,
    int? bytes,
  }) =>
      Usage(
        nodes: nodes ?? this.nodes,
        edges: edges ?? this.edges,
        events: events ?? this.events,
        values: values ?? this.values,
        bytes: bytes ?? this.bytes,
      );

  @override
  bool operator ==(Object other) =>
      other is Usage &&
      other.nodes == nodes &&
      other.edges == edges &&
      other.events == events &&
      other.values == values &&
      other.bytes == bytes;

  @override
  int get hashCode => Object.hash(nodes, edges, events, values, bytes);
}

/// A per-operation work budget. Unlike [Usage] (committed on
/// success), work is spent during a single preflight or query.
class Work {
  Work(this.cap) : used = 0;
  final int cap;
  int used;

  void take(int amount) {
    used = addBounded(used, amount, cap, 'operation work');
  }

  /// Reserves the visits spent so far a second time (used before a
  /// conversion phase that re-walks budgeted structure).
  void reserveConversion() {
    take(used);
  }
}

/// Adds [amount] to [current], failing on overflow or when the sum
/// would exceed [cap]. Returns the new total (Dart ints wrap
/// silently, so the overflow check is explicit, like the
/// reference's `checked_add`).
int addBounded(int current, int amount, int cap, String kind) {
  final next = current + amount;
  // Wrap-around (Dart ints are 64-bit and wrap) or a negative amount
  // can only come from accounting bugs; the reference reports the
  // same overflow for both.
  if (amount < 0 || next < current) {
    throw CvError('CV $kind accounting overflow');
  }
  if (next > cap) {
    throw CvError('CV $kind limit exceeded');
  }
  return next;
}

/// JSON string bytes without constructing an escaped copy. Mirrors
/// the reference's accounting exactly over the UTF-8 encoding:
/// quotes/backslash and `\n\r\t\b\f` cost 2, other controls cost 6
/// (`\u00XX`), every other byte costs 1, plus 2 for the surrounding
/// quotes. Lone surrogates (unrepresentable in the reference's
/// `&str`, and unencodable by `utf8.encode`) count as U+FFFD,
/// deterministically.
int stringBytes(String text) {
  var bytes = 2;
  final units = text.codeUnits;
  var i = 0;
  final sanitized = StringBuffer();
  while (i < units.length) {
    final unit = units[i];
    if (unit >= 0xD800 && unit <= 0xDBFF && i + 1 < units.length) {
      final next = units[i + 1];
      if (next >= 0xDC00 && next <= 0xDFFF) {
        sanitized.writeCharCode(unit);
        sanitized.writeCharCode(next);
        i += 2;
        continue;
      }
    }
    if (unit >= 0xD800 && unit <= 0xDFFF) {
      sanitized.writeCharCode(0xFFFD);
      i += 1;
      continue;
    }
    sanitized.writeCharCode(unit);
    i += 1;
  }
  for (final byte in utf8.encode(sanitized.toString())) {
    final width = byte == 0x22 ||
            byte == 0x5C ||
            byte == 0x0A ||
            byte == 0x0D ||
            byte == 0x09 ||
            byte == 0x08 ||
            byte == 0x0C
        ? 2
        : (byte <= 0x1F ? 6 : 1);
    final next = bytes + width;
    if (next < bytes) {
      throw const CvError('CV text accounting overflow');
    }
    bytes = next;
  }
  return bytes;
}
