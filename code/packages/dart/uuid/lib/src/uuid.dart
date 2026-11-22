/// Universally Unique Identifiers (RFC 4122 / RFC 9562), in pure Dart.
///
/// A UUID is a 128-bit label that identifies information without a central
/// authority. This package is the Dart lane of [uuid.md](../../../specs/uuid.md)
/// and ports the reference behavior of the Rust `uuid` crate:
///
/// * **v1** — time-based (Gregorian-epoch timestamp, random clock sequence,
///   random node ID with the multicast bit set per RFC 4122 section 4.5).
/// * **v3** — name-based with MD5 (deterministic; kept for compatibility).
/// * **v4** — 122 bits of OS randomness.
/// * **v5** — name-based with SHA-1 (deterministic; preferred over v3).
/// * **v7** — Unix-millisecond timestamp plus randomness (RFC 9562; sorts by
///   creation time).
///
/// A [Uuid] wraps 16 bytes in network (big-endian) order. Every derived view —
/// string form, version, variant, integer value — is computed from those bytes.
///
/// ```dart
/// import 'package:coding_adventures_uuid/coding_adventures_uuid.dart';
///
/// final id = Uuid.v4();
/// print(id); // e.g. 550e8400-e29b-41d4-a716-446655440000
/// final stable = Uuid.v5(Uuid.namespaceDns, 'python.org');
/// final back = Uuid.parse(stable.toString());
/// assert(back == stable);
/// ```
library uuid;

import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:coding_adventures_md5/coding_adventures_md5.dart' as md5;
import 'package:coding_adventures_sha1/coding_adventures_sha1.dart' as sha1;

// ─── Error type ─────────────────────────────────────────────────────────────
//
// One error type for the whole package keeps call sites simple: parsing and
// checked construction either give back a UUID or throw a [UuidError] whose
// message says what was wrong with the input.

/// An error produced by UUID parsing or checked construction.
class UuidError implements Exception {
  /// Human-readable reason, including the offending input where it helps.
  final String message;

  /// Creates an error with the given [message].
  const UuidError(this.message);

  @override
  String toString() => 'UuidError: $message';
}

// ─── Uuid ───────────────────────────────────────────────────────────────────
//
// The 16 bytes are stored big-endian (most significant byte first), matching
// the wire format and the order bytes appear in the string form:
//
//   Byte index:  0  1  2  3  |  4  5  |  6  7  |  8  9  | 10 11 12 13 14 15
//   UUID field:  time_low     | t_mid  | t_hi+v | clk_seq|  node (48 bits)
//
// The byte buffer is copied on the way in ([Uuid.fromBytes]) and on the way
// out ([bytes]), so a [Uuid] is a value: nobody holding an alias can mutate it
// afterwards. Equality, ordering, and hashing all work on the bytes.

/// A 128-bit UUID stored as 16 bytes in network byte order (big-endian).
class Uuid implements Comparable<Uuid> {
  /// The raw bytes. Private and never shared, so instances are immutable.
  final Uint8List _bytes;

  /// Wraps [bytes] without copying. Callers must not retain the buffer.
  Uuid._(this._bytes);

  // ── Construction ─────────────────────────────────────────────────────────

  /// Creates a UUID from 16 raw bytes in network byte order.
  ///
  /// The bytes are copied and used as-is; no version or variant bits are set.
  /// This is the low-level constructor — prefer [v4], [v5], and friends.
  ///
  /// Throws [UuidError] if [bytes] does not hold exactly 16 bytes.
  factory Uuid.fromBytes(List<int> bytes) {
    if (bytes.length != 16) {
      throw UuidError('expected exactly 16 bytes, got ${bytes.length}: $bytes');
    }
    return Uuid._(Uint8List.fromList(bytes));
  }

  /// Returns a copy of the raw 16 bytes in network byte order.
  Uint8List get bytes => Uint8List.fromList(_bytes);

  /// Returns the UUID as a 128-bit unsigned integer (big-endian).
  ///
  /// Dart has no 128-bit int, so this is a [BigInt]. Byte 0 is the most
  /// significant byte: the nil UUID maps to 0 and the max UUID to 2^128 - 1.
  BigInt toBigInt() {
    var value = BigInt.zero;
    for (final byte in _bytes) {
      value = (value << 8) | BigInt.from(byte);
    }
    return value;
  }

  // ── Field accessors ──────────────────────────────────────────────────────
  //
  // RFC 4122 section 4.1.3: the version is the 4 high bits of byte 6.
  //   Byte 6 layout: [v v v v | t t t t]  (version nibble, then time_hi)
  // A right shift by 4 with a mask isolates the version nibble.

  /// Returns the version field (the high nibble of byte 6).
  ///
  /// Standard values are 1, 3, 4, 5, or 7; the nil UUID reports 0.
  int get version => (_bytes[6] >> 4) & 0xF;

  // RFC 4122 section 4.1.1: the variant lives in the high bits of byte 8.
  //   [1 0 x x | x x x x]  -> rfc4122   (what we generate)
  //   [0 x x x | x x x x]  -> ncs       (legacy)
  //   [1 1 0 x | x x x x]  -> microsoft (legacy Windows GUID)
  //   [1 1 1 x | x x x x]  -> reserved
  // Decode the top two bits; for 0b11 the third bit splits the last two rows.

  /// Returns the variant field as a human-readable string.
  ///
  /// One of `"rfc4122"`, `"ncs"`, `"microsoft"`, or `"reserved"`.
  String get variant {
    switch ((_bytes[8] >> 6) & 0x3) {
      case 0:
      case 1:
        return 'ncs'; // 0xxx xxxx — NCS backward compatibility
      case 2:
        return 'rfc4122'; // 10xx xxxx — RFC 4122 (what we generate)
      default:
        // 110x xxxx = Microsoft; 111x xxxx = reserved.
        return ((_bytes[8] >> 5) & 1) == 0 ? 'microsoft' : 'reserved';
    }
  }

  /// Returns true if this is the nil UUID (all 128 bits zero).
  ///
  /// The nil UUID is the UUID analog of a null pointer: conventionally it
  /// means "no UUID assigned".
  bool get isNil {
    for (final byte in _bytes) {
      if (byte != 0) return false;
    }
    return true;
  }

  /// Returns true if this is the max UUID (all 128 bits one).
  ///
  /// Introduced in RFC 9562 as a sentinel maximum value.
  bool get isMax {
    for (final byte in _bytes) {
      if (byte != 0xFF) return false;
    }
    return true;
  }

  // ── Display ──────────────────────────────────────────────────────────────
  //
  // RFC 4122 section 3 mandates the canonical 8-4-4-4-12 lowercase hex format.
  // Encode the 16 bytes as 32 hex characters, then insert hyphens between the
  // field groups.

  /// Formats as the standard 8-4-4-4-12 lowercase hex representation.
  ///
  /// Example: `"550e8400-e29b-41d4-a716-446655440000"`.
  @override
  String toString() {
    final h = _hexEncode(_bytes);
    return '${h.substring(0, 8)}-'
        '${h.substring(8, 12)}-'
        '${h.substring(12, 16)}-'
        '${h.substring(16, 20)}-'
        '${h.substring(20, 32)}';
  }

  /// Returns the UUID as 32 lowercase hex characters (no hyphens).
  String toHex() => _hexEncode(_bytes);

  /// Returns the UUID as a URN string.
  ///
  /// Example: `"urn:uuid:550e8400-e29b-41d4-a716-446655440000"`.
  String toUrn() => 'urn:uuid:$this';

  // ── Comparison ───────────────────────────────────────────────────────────
  //
  // Ordering is lexicographic on the raw bytes (equivalently, numeric order on
  // the 128-bit big-endian value). That is what makes v7 UUIDs sort
  // chronologically: their timestamp sits in the highest bits.

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! Uuid) return false;
    for (var i = 0; i < 16; i++) {
      if (_bytes[i] != other._bytes[i]) return false;
    }
    return true;
  }

  @override
  int get hashCode => Object.hashAll(_bytes);

  /// Compares UUIDs lexicographically by raw bytes.
  @override
  int compareTo(Uuid other) {
    for (var i = 0; i < 16; i++) {
      final cmp = _bytes[i].compareTo(other._bytes[i]);
      if (cmp != 0) return cmp;
    }
    return 0;
  }

  /// Less-than on raw bytes (see [compareTo]).
  bool operator <(Uuid other) => compareTo(other) < 0;

  /// Less-than-or-equal on raw bytes (see [compareTo]).
  bool operator <=(Uuid other) => compareTo(other) <= 0;

  /// Greater-than on raw bytes (see [compareTo]).
  bool operator >(Uuid other) => compareTo(other) > 0;

  /// Greater-than-or-equal on raw bytes (see [compareTo]).
  bool operator >=(Uuid other) => compareTo(other) >= 0;

  // ── Parsing ──────────────────────────────────────────────────────────────
  //
  // Four serialization forms are accepted (mirroring the reference lanes):
  //   Standard: "550e8400-e29b-41d4-a716-446655440000"   (36 chars, hyphens)
  //   Compact:  "550e8400e29b41d4a716446655440000"         (32 chars, no hyphens)
  //   Braced:   "{550e8400-e29b-41d4-a716-446655440000}"  (Microsoft GUID style)
  //   URN:      "urn:uuid:550e8400-e29b-41d4-a716-446655440000"
  //
  // Strategy: trim whitespace, strip the URN prefix and braces, delete every
  // hyphen, require exactly 32 hex digits, then decode two digits per byte.
  // Like the reference, hyphen *positions* are not validated — only the digit
  // count and alphabet matter.

  /// Parses a UUID from its string representation.
  ///
  /// Accepts standard (8-4-4-4-12), compact (no hyphens), braced, and URN
  /// forms. Case-insensitive; leading/trailing whitespace is ignored.
  ///
  /// Throws [UuidError] if the text is not a valid UUID representation.
  static Uuid parse(String text) {
    var s = text.trim();

    // Strip the URN prefix "urn:uuid:" (lowercase, as emitted by [toUrn]).
    const urnPrefix = 'urn:uuid:';
    if (s.startsWith(urnPrefix)) {
      s = s.substring(urnPrefix.length);
    }

    // Strip outer braces (Microsoft GUID style).
    if (s.startsWith('{')) s = s.substring(1);
    if (s.endsWith('}')) s = s.substring(0, s.length - 1);

    // Remove hyphens so what remains must be raw hex.
    final hex = s.replaceAll('-', '');

    if (hex.length != 32) {
      throw UuidError('expected 32 hex digits (after removing hyphens), '
          'got ${hex.length}: "$s"');
    }
    for (var i = 0; i < hex.length; i++) {
      if (!_isHexDigit(hex.codeUnitAt(i))) {
        throw UuidError('non-hex character in UUID string: "$s"');
      }
    }

    final bytes = Uint8List(16);
    for (var i = 0; i < 16; i++) {
      final high = _hexValue(hex.codeUnitAt(2 * i));
      final low = _hexValue(hex.codeUnitAt(2 * i + 1));
      bytes[i] = (high << 4) | low;
    }
    return Uuid._(bytes);
  }

  /// Returns true if [text] is a valid UUID string in any accepted format.
  static bool isValid(String text) {
    try {
      parse(text);
      return true;
    } on UuidError {
      return false;
    }
  }

  // ── Namespace constants ──────────────────────────────────────────────────
  //
  // RFC 4122 Appendix C fixes four namespace UUIDs for name-based (v3/v5)
  // generation. Two applications hashing the same name under different
  // namespaces get different UUIDs, so their stable IDs cannot collide.
  //
  // These are static final rather than const because Uint8List has no const
  // constructor. Each is still a distinct immutable value.

  /// Namespace UUID for fully qualified domain names (RFC 4122 Appendix C).
  ///
  /// `"6ba7b810-9dad-11d1-80b4-00c04fd430c8"`.
  static final Uuid namespaceDns = Uuid._(Uint8List.fromList([
    0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1,
    0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8, //
  ]));

  /// Namespace UUID for URLs (RFC 4122 Appendix C).
  ///
  /// `"6ba7b811-9dad-11d1-80b4-00c04fd430c8"`.
  static final Uuid namespaceUrl = Uuid._(Uint8List.fromList([
    0x6b, 0xa7, 0xb8, 0x11, 0x9d, 0xad, 0x11, 0xd1,
    0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8, //
  ]));

  /// Namespace UUID for ISO Object Identifiers (RFC 4122 Appendix C).
  ///
  /// `"6ba7b812-9dad-11d1-80b4-00c04fd430c8"`.
  static final Uuid namespaceOid = Uuid._(Uint8List.fromList([
    0x6b, 0xa7, 0xb8, 0x12, 0x9d, 0xad, 0x11, 0xd1,
    0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8, //
  ]));

  /// Namespace UUID for X.500 distinguished names (RFC 4122 Appendix C).
  ///
  /// `"6ba7b814-9dad-11d1-80b4-00c04fd430c8"`.
  static final Uuid namespaceX500 = Uuid._(Uint8List.fromList([
    0x6b, 0xa7, 0xb8, 0x14, 0x9d, 0xad, 0x11, 0xd1,
    0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8, //
  ]));

  /// The nil UUID: all 128 bits zero.
  ///
  /// `"00000000-0000-0000-0000-000000000000"`.
  static final Uuid nilUuid = Uuid._(Uint8List(16));

  /// The max UUID: all 128 bits one (RFC 9562 sentinel maximum).
  ///
  /// `"ffffffff-ffff-ffff-ffff-ffffffffffff"`.
  static final Uuid maxUuid = Uuid._(Uint8List.fromList(List.filled(16, 0xFF)));

  // ── Generation ───────────────────────────────────────────────────────────
  //
  // After building a raw 16-byte buffer from a hash or random source, two
  // fields must be stamped to identify the version and variant:
  //   Byte 6: high nibble = version.  (raw[6] & 0x0F) keeps the hash's low bits.
  //   Byte 8: high 2 bits = 10.       (raw[8] & 0x3F) keeps the hash's low bits.

  /// Stamps [version] and the RFC 4122 variant bits into [raw], in place.
  static void _setVersionVariant(Uint8List raw, int version) {
    raw[6] = (raw[6] & 0x0F) | (version << 4);
    raw[8] = (raw[8] & 0x3F) | 0x80;
  }

  /// Draws [n] bytes from the OS cryptographically secure RNG.
  ///
  /// `Random.secure()` bridges to the platform entropy source
  /// (getrandom/B CryptGenRandom/SecRandomCopyBytes depending on the host).
  static Uint8List _randomBytes(int n) {
    final rng = Random.secure();
    final out = Uint8List(n);
    for (var i = 0; i < n; i++) {
      out[i] = rng.nextInt(256);
    }
    return out;
  }

  // UUID v4: 122 bits of randomness with version/variant stamped in. No
  // timestamp, no MAC, no determinism. Collision probability for n UUIDs is
  // about n^2 / 2^123 — for a billion IDs, roughly 1.2e-19.

  /// Generates a UUID v4 (random) from the OS CSPRNG.
  static Uuid v4() {
    final raw = _randomBytes(16);
    _setVersionVariant(raw, 4);
    return Uuid._(raw);
  }

  // UUID v5: SHA-1(namespace || name), first 16 of the 20 digest bytes, then
  // stamp version 5. Deterministic: same inputs always give the same UUID.
  // Preferred over v3 for new systems.

  /// Generates a UUID v5 (SHA-1 name-based). Deterministic.
  ///
  /// The name is encoded as UTF-8 before hashing.
  ///
  /// Known vector: `v5(namespaceDns, "python.org")` is
  /// `"886313e1-3b8a-5372-9b90-0c9aee199e5d"`.
  static Uuid v5(Uuid namespace, String name) {
    final input = [...namespace._bytes, ...utf8.encode(name)];
    final digest = sha1.sum1(input); // 20 bytes; use the first 16.
    final raw = Uint8List.fromList(digest.sublist(0, 16));
    _setVersionVariant(raw, 5);
    return Uuid._(raw);
  }

  // UUID v3: MD5(namespace || name) gives exactly 16 bytes, used directly,
  // then stamp version 3. Kept for compatibility with older systems; prefer v5.

  /// Generates a UUID v3 (MD5 name-based). Deterministic.
  ///
  /// The name is encoded as UTF-8 before hashing.
  ///
  /// Known vector: `v3(namespaceDns, "python.org")` is
  /// `"6fa459ea-ee8a-3ca4-894e-db77e160355e"`.
  static Uuid v3(Uuid namespace, String name) {
    final input = [...namespace._bytes, ...utf8.encode(name)];
    final raw = md5.sumMd5(input); // Exactly 16 bytes.
    _setVersionVariant(raw, 3);
    return Uuid._(raw);
  }

  // UUID v1: a 60-bit timestamp (100ns intervals since 1582-10-15), a 14-bit
  // clock sequence, and a 48-bit node ID. Dart reports microsecond time, so
  // the 100ns count is microseconds * 10. The node is random (never a MAC
  // address) with the multicast bit set per RFC 4122 section 4.5, and the
  // clock sequence is freshly random per call, matching the reference lane.

  /// 100-nanosecond intervals between the Gregorian epoch (1582-10-15) and
  /// the Unix epoch (1970-01-01). RFC 4122 Appendix B.
  static const int gregorianOffset = 122192928000000000;

  /// Generates a UUID v1 (time-based with random node ID).
  static Uuid v1() {
    final t = DateTime.now().microsecondsSinceEpoch * 10 + gregorianOffset;

    final timeLow = t & 0xFFFFFFFF;
    final timeMid = (t >> 32) & 0xFFFF;
    final timeHi = (t >> 48) & 0x0FFF;
    final timeHiV = 0x1000 | timeHi; // version 1 in the high nibble.

    // 14-bit random clock sequence across bytes 8 and 9.
    final clockRaw = _randomBytes(2);
    final clockSeq = ((clockRaw[0] << 8) | clockRaw[1]) & 0x3FFF;
    final clockHi = 0x80 | (clockSeq >> 8); // variant bits + high sequence.
    final clockLo = clockSeq & 0xFF;

    // Random 48-bit node with the multicast bit set (LSB of first byte).
    final node = _randomBytes(6);
    node[0] |= 0x01;

    final raw = Uint8List(16);
    raw[0] = (timeLow >> 24) & 0xFF;
    raw[1] = (timeLow >> 16) & 0xFF;
    raw[2] = (timeLow >> 8) & 0xFF;
    raw[3] = timeLow & 0xFF;
    raw[4] = (timeMid >> 8) & 0xFF;
    raw[5] = timeMid & 0xFF;
    raw[6] = (timeHiV >> 8) & 0xFF;
    raw[7] = timeHiV & 0xFF;
    raw[8] = clockHi;
    raw[9] = clockLo;
    raw.setRange(10, 16, node);
    return Uuid._(raw);
  }

  // UUID v7 (RFC 9562): a 48-bit Unix-millisecond timestamp in the high bits,
  // then version 7, 12 random bits, the variant, and 62 random bits. The
  // timestamp orders UUIDs across milliseconds; within one millisecond the
  // random suffix decides the order.
  //
  //   raw[0..6]  = 48-bit ms timestamp (big-endian)
  //   raw[6]     = 0x70 | 4 random bits   (version nibble + rand_a slice)
  //   raw[7]     = 8 random bits           (rest of rand_a)
  //   raw[8]     = 0x80 | 6 random bits   (variant + rand_b slice)
  //   raw[9..16] = 56 random bits          (rest of rand_b)

  /// Generates a UUID v7 (Unix-time ordered random).
  ///
  /// v7 UUIDs sort chronologically as byte strings, which makes them good
  /// database primary keys (B-tree locality, unlike fully random v4).
  static Uuid v7() {
    final tsMs = DateTime.now().millisecondsSinceEpoch;
    final rand = _randomBytes(10); // 80 bits: rand_a (12) + rand_b (62).

    final raw = Uint8List(16);
    raw[0] = (tsMs >> 40) & 0xFF;
    raw[1] = (tsMs >> 32) & 0xFF;
    raw[2] = (tsMs >> 24) & 0xFF;
    raw[3] = (tsMs >> 16) & 0xFF;
    raw[4] = (tsMs >> 8) & 0xFF;
    raw[5] = tsMs & 0xFF;
    raw[6] = 0x70 | (rand[0] & 0x0F);
    raw[7] = rand[1];
    raw[8] = 0x80 | (rand[2] & 0x3F);
    raw.setRange(9, 16, rand.skip(3));
    return Uuid._(raw);
  }
}

// ─── Hex helpers ────────────────────────────────────────────────────────────
//
// Rolled by hand rather than imported: two tiny functions are clearer than a
// dependency, and they keep this package's runtime footprint to md5 + sha1.

const String _hexDigits = '0123456789abcdef';

/// Returns true if [unit] is an ASCII hex digit (0-9, a-f, A-F).
bool _isHexDigit(int unit) =>
    (unit >= 0x30 && unit <= 0x39) || // 0-9
    (unit >= 0x61 && unit <= 0x66) || // a-f
    (unit >= 0x41 && unit <= 0x46); // A-F

/// Converts one ASCII hex digit code unit to its 0-15 value.
///
/// The caller must have validated with [_isHexDigit] first.
int _hexValue(int unit) {
  if (unit <= 0x39) return unit - 0x30; // 0-9
  if (unit >= 0x61) return unit - 0x61 + 10; // a-f
  return unit - 0x41 + 10; // A-F
}

/// Encodes [bytes] as a lowercase hex string. `[0xDE, 0xAD]` -> `"dead"`.
String _hexEncode(Uint8List bytes) {
  final out = StringBuffer();
  for (final byte in bytes) {
    out.write(_hexDigits[(byte >> 4) & 0xF]);
    out.write(_hexDigits[byte & 0xF]);
  }
  return out.toString();
}
