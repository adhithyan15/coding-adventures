/// Validated loader for the generated TRIG-REF-1 vector source (PHY00a 6.5).
///
/// This package declares no filesystem capability, so the vectors in
/// `code/specs/fixtures/trig-reference-v1/*.tsv` reach the tests as base64
/// constants in `generated/trig_reference_vectors.dart`.  Constants are data
/// from outside the code, so they are checked before any row is trusted: a size
/// cap, strict UTF-8, exactly five tab-separated fields, a closed set of ops and
/// flags, 16-digit lowercase hex tokens, and the manifest's row counts.
library;

import 'dart:convert';

/// Largest decoded group accepted (the biggest real group is about 1.2 MB).
const int maxGroupBytes = 1500000;

/// The closed set of operations a row may name (PHY00a 6.2).
const Set<String> knownOps = {
  'sin',
  'cos',
  'tan',
  'radians',
  'degrees',
  'sqrt',
  'atan',
  'atan2',
};

/// The closed set of flags a row may carry (PHY00a 6.2).
const Set<String> knownFlags = {
  'nan-class',
  'nonfinite=unrepresentable',
  'nonfinite=rejects',
  'out-of-domain',
  'pole-sentinel',
  'beam-raises',
  'signed-zero',
};

final RegExp _hex16 = RegExp(r'^[0-9a-f]{16}$');

/// One vector: [op] applied to [x] must give the bits [y].
class Vector {
  Vector(this.op, this.x, this.y, this.cr, this.flags);

  /// Operation name, one of [knownOps].
  final String op;

  /// Input bits as 16 hex digits, or `y,x` (33 characters) for `atan2`.
  final String x;

  /// Reference result bits, or the literal `error`.
  final String y;

  /// Correctly rounded result bits, or `-` when none is claimed.
  final String cr;

  /// Flags from [knownFlags].
  final Set<String> flags;
}

/// Decodes one group, validating it, and returns its rows in file order.
///
/// [rows] and [opRows] are the manifest's counts for the group; any mismatch
/// throws a [FormatException].
List<Vector> decodeGroup(
  String name,
  String encoded, {
  required int rows,
  required Map<String, int> opRows,
}) {
  final bytes = base64Decode(encoded);
  if (bytes.length > maxGroupBytes) {
    throw FormatException('$name: group exceeds $maxGroupBytes bytes');
  }
  final text = utf8.decode(bytes, allowMalformed: false);
  if (text.contains('\r') || !text.endsWith('\n')) {
    throw FormatException('$name: lines must end with a single \\n');
  }
  final lines = text.substring(0, text.length - 1).split('\n');
  if (lines.length != rows) {
    throw FormatException('$name: ${lines.length} rows, expected $rows');
  }
  final out = <Vector>[];
  final counts = <String, int>{};
  for (var i = 0; i < lines.length; i++) {
    final where = '$name:${i + 1}';
    final fields = lines[i].split('\t');
    if (fields.length != 5) {
      throw FormatException('$where: expected 5 fields, got ${fields.length}');
    }
    final op = fields[0];
    if (!knownOps.contains(op)) {
      throw FormatException('$where: unknown op "$op"');
    }
    final x = fields[1];
    final inputOk = op == 'atan2'
        ? x.length == 33 &&
            x[16] == ',' &&
            _hex16.hasMatch(x.substring(0, 16)) &&
            _hex16.hasMatch(x.substring(17))
        : _hex16.hasMatch(x);
    if (!inputOk) {
      throw FormatException('$where: malformed input "$x"');
    }
    final y = fields[2];
    if (!(_hex16.hasMatch(y) || (y == 'error' && op == 'sqrt'))) {
      throw FormatException('$where: malformed result "$y"');
    }
    final cr = fields[3];
    if (!(_hex16.hasMatch(cr) || cr == '-')) {
      throw FormatException('$where: malformed cr "$cr"');
    }
    final flags = fields[4].isEmpty ? <String>{} : fields[4].split(',').toSet();
    for (final flag in flags) {
      if (!knownFlags.contains(flag)) {
        throw FormatException('$where: unknown flag "$flag"');
      }
    }
    counts[op] = (counts[op] ?? 0) + 1;
    out.add(Vector(op, x, y, cr, flags));
  }
  if (counts.length != opRows.length ||
      opRows.entries.any((e) => counts[e.key] != e.value)) {
    throw FormatException('$name: per-op row counts $counts != $opRows');
  }
  return out;
}
