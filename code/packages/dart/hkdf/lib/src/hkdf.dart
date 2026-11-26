/// HKDF (HMAC-based Extract-and-Expand Key Derivation Function) —
/// RFC 5869.
///
/// HKDF derives strong keys from input keying material in two
/// phases. Extract concentrates entropy (`PRK = HMAC(salt, IKM)`);
/// Expand stretches the PRK to any length by chaining
/// (`T(i) = HMAC(PRK, T(i-1) || info || i)`).
///
/// Two API families mirror the reference: the legacy
/// [HashAlgorithm] entry points ([hkdfExtract]/[hkdfExpand]/[hkdf])
/// and the fixed-memory [FixedHkdfHash] boundaries
/// ([hkdfExtractFixed]/[hkdfExpandFixed]) built for production
/// secret handling over the in-lane HMAC segments paths.
///
/// ## Memory hygiene
///
/// Fixed-path secrets live in [ZeroizingBytes] owners disposed in
/// `finally` blocks (Dart has no destructors): the PRK backing
/// array, the Expand previous-block owner, and every temporary HMAC
/// tag (wiped after copying into its destination). The legacy path
/// mirrors the reference exactly, which does not wipe its
/// intermediates. The zeroize lane docs' platform limits apply
/// unchanged.
library;

import 'dart:typed_data';

import 'package:coding_adventures_blake2b/blake2b.dart';
import 'package:coding_adventures_hmac/hmac.dart';
import 'package:coding_adventures_sha256/coding_adventures_sha256.dart'
    as sha256lane;
import 'package:coding_adventures_sha512/coding_adventures_sha512.dart'
    as sha512lane;
import 'package:coding_adventures_zeroize/zeroize.dart';

import 'errors.dart';

/// Supported hash algorithms for the legacy HKDF entry points.
///
/// ## BLAKE2b variant
///
/// Uses BLAKE2b's native keyed-hash mode as the PRF instead of HMAC.
/// Output is fixed at 32 bytes to match the PRK size the Signal
/// Protocol stack expects.
enum HashAlgorithm {
  /// SHA-256: 32-byte output, 64-byte block.
  sha256,

  /// SHA-512: 64-byte output, 128-byte block.
  sha512,

  /// BLAKE2b in keyed mode: 32-byte output, no pad construction.
  blake2b;

  /// The hash output length: PRK size, Expand block size, and
  /// default-salt length.
  int get hashLen => switch (this) {
        HashAlgorithm.sha256 => 32,
        HashAlgorithm.sha512 => 64,
        HashAlgorithm.blake2b => 32,
      };
}

/// Fixed-output hashes admitted by the fixed-memory Extract and
/// Expand boundaries. Deliberately separate from [HashAlgorithm]:
/// only production-grade fixed paths qualify.
enum FixedHkdfHash {
  /// HMAC-SHA-256 with a 32-byte PRK.
  sha256,

  /// HMAC-SHA-384 with a 48-byte PRK.
  sha384;

  /// The exact PRK and default-salt length for this hash.
  int get hashLen => switch (this) {
        FixedHkdfHash.sha256 => 32,
        FixedHkdfHash.sha384 => 48,
      };

  /// HMAC's inner/outer hash block length for this hash.
  int get blockLen => switch (this) {
        FixedHkdfHash.sha256 => 64,
        FixedHkdfHash.sha384 => 128,
      };

  /// Largest complete hash input representable by the FIPS encoded
  /// length field: u64::MAX/8 for SHA-256, u128::MAX/8 for SHA-384.
  BigInt get maxMessageBytes => switch (this) {
        FixedHkdfHash.sha256 => ((BigInt.one << 64) - BigInt.one) >> 3,
        FixedHkdfHash.sha384 => ((BigInt.one << 128) - BigInt.one) >> 3,
      };
}

/// Wipe-on-drop pseudorandom key from [hkdfExtractFixed] or
/// [HkdfPrk.importExact].
///
/// One fixed 48-byte backing array exposes only the selected hash's
/// prefix. Call `dispose()` when done (Dart has no destructors);
/// [asBytes] throws [StateError] afterwards. Retain this owner
/// rather than copying its secret slice into longer-lived storage.
class HkdfPrk {
  final FixedHkdfHash _algorithm;
  final ZeroizingBytes _bytes;

  HkdfPrk._(this._algorithm, Uint8List backing)
      : _bytes = ZeroizingBytes(backing);

  /// Imports an already-derived PRK into fixed wipe-on-drop storage.
  ///
  /// [bytes] must be exactly the selected hash's output length;
  /// validation happens before any secret copy, and the unused
  /// SHA-256 tail of the zero-initialized owner stays zero (the
  /// borrow itself remains caller-owned and is not wiped here).
  ///
  /// This is deliberately **not** HKDF-Extract: it establishes no
  /// entropy, provenance, or protocol suitability. Use
  /// [hkdfExtractFixed] for raw keying material and reserve this
  /// for values already known to be valid PRKs.
  ///
  /// Throws [HkdfPrkImportError] unless the length is exact.
  factory HkdfPrk.importExact(FixedHkdfHash algorithm, List<int> bytes) {
    if (bytes.length != algorithm.hashLen) {
      throw const HkdfPrkImportError();
    }
    final staging = ZeroizingBytes(Uint8List(48));
    try {
      staging.bytes.setRange(0, bytes.length, bytes);
      // Copy into the result slot so the staging owner still drops
      // (and scrubs) its own storage afterwards.
      return HkdfPrk._(algorithm, Uint8List.fromList(staging.bytes));
    } finally {
      staging.dispose();
    }
  }

  /// The hash algorithm bound to this PRK.
  FixedHkdfHash get algorithm => _algorithm;

  /// Borrows the selected hash's exact PRK bytes as a view: no copy
  /// is made. Do not mutate or retain past [dispose].
  ///
  /// Throws [StateError] after [dispose].
  Uint8List asBytes() =>
      Uint8List.sublistView(_bytes.bytes, 0, _algorithm.hashLen);

  /// Wipes the backing array. Idempotent.
  void dispose() => _bytes.dispose();

  /// Whether [dispose] has run.
  bool get isDisposed => _bytes.isDisposed;

  /// Renders metadata only — never PRK bytes.
  @override
  String toString() =>
      'HkdfPrk(algorithm: $_algorithm, length: ${_algorithm.hashLen})';
}

/// The pseudorandom function for [algorithm]: generic HMAC over the
/// in-lane hashes for SHA-256/SHA-512, native keyed BLAKE2b (32-byte
/// digest) otherwise. An empty BLAKE2b key falls back to an unkeyed
/// hash over a 32-zero prefix plus the message; longer keys clamp to
/// the 64-byte BLAKE2b limit.
Uint8List _prf(HashAlgorithm algorithm, List<int> key, List<int> message) {
  switch (algorithm) {
    case HashAlgorithm.sha256:
      return hmac(sha256lane.sha256, 64, key, message);
    case HashAlgorithm.sha512:
      return hmac(sha512lane.sha512, 128, key, message);
    case HashAlgorithm.blake2b:
      if (key.isEmpty) {
        return blake2b(
          [...Uint8List(32), ...message],
          Blake2bOptions(digestSize: 32),
        );
      }
      final clamped = key.length > 64 ? key.sublist(0, 64) : key;
      return blake2b(
        message,
        Blake2bOptions(
          key: clamped is Uint8List ? clamped : Uint8List.fromList(clamped),
          digestSize: 32,
        ),
      );
  }
}

/// Fixed-memory HKDF-Extract for SHA-256 and SHA-384 (RFC 5869 §2.2):
/// `PRK = HMAC-Hash(salt, IKM)`.
///
/// An empty [salt] becomes exactly HashLen zero bytes. The returned
/// [HkdfPrk] wipes its fixed backing storage on [HkdfPrk.dispose];
/// the temporary HMAC tag is wiped after copying into it.
///
/// Throws [HkdfInputTooLong] if an inner input would exceed the
/// encoded length bound, or [HkdfPrfRejectedKey] if the PRF rejects
/// its key (unreachable from public inputs: the effective salt is
/// never empty).
HkdfPrk hkdfExtractFixed(
  List<int> salt,
  List<int> ikm,
  FixedHkdfHash algorithm,
) {
  final effectiveSalt = salt.isEmpty ? Uint8List(algorithm.hashLen) : salt;
  final Uint8List tag;
  try {
    tag = switch (algorithm) {
      FixedHkdfHash.sha256 => hmacSha256(effectiveSalt, ikm),
      FixedHkdfHash.sha384 => hmacSha384(effectiveSalt, ikm),
    };
    // coverage:ignore-start
    // Both arms below are unreachable: the effective salt is
    // non-empty by construction, and the length bound needs 2^127+
    // bytes. They preserve the reference's total HkdfError mapping.
  } on HmacEmptyKey {
    throw const HkdfPrfRejectedKey();
  } on HmacInputTooLong {
    throw const HkdfInputTooLong();
    // coverage:ignore-end
  }
  try {
    return HkdfPrk.importExact(algorithm, tag);
  } finally {
    zeroizeBytes(tag);
  }
}

/// HKDF-Extract: concentrates [ikm] entropy into a HashLen-byte PRK
/// (`PRK = HMAC-Hash(salt, IKM)`). An empty [salt] becomes HashLen
/// zero bytes, per RFC 5869 §2.2.
Uint8List hkdfExtract(
  List<int> salt,
  List<int> ikm,
  HashAlgorithm algorithm,
) {
  final effectiveSalt = salt.isEmpty ? Uint8List(algorithm.hashLen) : salt;
  return _prf(algorithm, effectiveSalt, ikm);
}

/// Whether every inner HMAC input in fixed-memory Expand fits the
/// selected hash's encoded message-length field: one normalized key
/// block plus `T(i-1) || info || i` (later blocks only carry `T(i-1)`,
/// so single-block expansion keeps its slightly larger valid `info`
/// boundary). Runs before any caller output changes.
bool _fixedExpandInputFits(
  FixedHkdfHash algorithm,
  int infoLen,
  int blockCount,
) {
  final previousLen = blockCount > 1 ? algorithm.hashLen : 0;
  final inputLen =
      BigInt.from(algorithm.blockLen + previousLen + 1) + BigInt.from(infoLen);
  return inputLen <= algorithm.maxMessageBytes;
}

/// Fixed-memory HKDF-Expand for SHA-256 and SHA-384 (RFC 5869 §2.3)
/// into caller-provided [output]:
/// `T(i) = HMAC-Hash(PRK, T(i-1) || info || i)`.
///
/// The algorithm bound to [prk] prevents Extract/Expand mismatch.
/// Message segments avoid an aggregate allocation, one fixed 48-byte
/// owner holds `T(i-1)`, and each temporary tag is wiped after
/// copying. Length failures happen before [output] changes; an
/// unexpected HMAC failure wipes all caller output before returning.
///
/// Throws [HkdfOutputTooShort] when [output] is empty,
/// [HkdfOutputTooLong] past 255 × HashLen, or [HkdfInputTooLong]
/// when an inner input would exceed the length bound.
void hkdfExpandFixed(HkdfPrk prk, List<int> info, Uint8List output) {
  final algorithm = prk.algorithm;
  final hashLen = algorithm.hashLen;
  final maximum = 255 * hashLen;
  if (output.isEmpty) {
    throw const HkdfOutputTooShort();
  }
  if (output.length > maximum) {
    throw HkdfOutputTooLong(
      requested: output.length,
      maximum: maximum,
    );
  }
  final blockCount = (output.length + hashLen - 1) ~/ hashLen;
  if (!_fixedExpandInputFits(algorithm, info.length, blockCount)) {
    throw const HkdfInputTooLong();
  }

  final previous = ZeroizingBytes(Uint8List(48));
  try {
    var written = 0;
    for (var blockIndex = 0; blockIndex < blockCount; blockIndex++) {
      final List<int> previousBlock = blockIndex == 0
          ? <int>[]
          : Uint8List.sublistView(previous.bytes, 0, hashLen);
      final segments = [
        previousBlock,
        info,
        [blockIndex + 1]
      ];
      final remaining = output.length - written;
      final take = remaining < hashLen ? remaining : hashLen;
      final Uint8List tag;
      try {
        tag = switch (algorithm) {
          FixedHkdfHash.sha256 => hmacSha256Segments(prk.asBytes(), segments),
          FixedHkdfHash.sha384 => hmacSha384Segments(prk.asBytes(), segments),
        };
        // coverage:ignore-start
        // Unreachable: the PRK key is HashLen bytes (never empty)
        // and the preflight above bounds every inner input. The wipe
        // preserves the reference's scrub-on-unexpected-failure.
      } on HmacError catch (e) {
        zeroizeBytes(output);
        throw e is HmacEmptyKey
            ? const HkdfPrfRejectedKey()
            : const HkdfInputTooLong();
        // coverage:ignore-end
      }
      try {
        output.setRange(written, written + take, tag);
        previous.bytes.setRange(0, hashLen, tag);
      } finally {
        zeroizeBytes(tag);
      }
      written += take;
    }
  } finally {
    previous.dispose();
  }
}

/// HKDF-Expand: derives [length] bytes from [prk]
/// (`T(i) = HMAC-Hash(PRK, T(i-1) || info || i)`, first [length]
/// bytes kept).
///
/// Throws [HkdfOutputTooShort] when [length] is zero (or negative,
/// which the reference's unsigned length cannot express) and
/// [HkdfOutputTooLong] past 255 × HashLen.
Uint8List hkdfExpand(
  List<int> prk,
  List<int> info,
  int length,
  HashAlgorithm algorithm,
) {
  final hashLen = algorithm.hashLen;
  final maxLength = 255 * hashLen;
  if (length <= 0) {
    throw const HkdfOutputTooShort();
  }
  if (length > maxLength) {
    throw HkdfOutputTooLong(requested: length, maximum: maxLength);
  }
  final n = (length + hashLen - 1) ~/ hashLen;
  final okm = <int>[];
  var tPrev = <int>[];
  for (var i = 1; i <= n; i++) {
    // Counter is one byte: n <= 255 and i starts at 1, so the value
    // always fits.
    final message = [...tPrev, ...info, i];
    tPrev = _prf(algorithm, prk, message);
    okm.addAll(tPrev);
  }
  return Uint8List.fromList(okm.sublist(0, length));
}

/// HKDF extract-then-expand (RFC 5869 §2):
/// `OKM = HKDF-Expand(HKDF-Extract(salt, IKM), info, L)`.
///
/// Throws [HkdfError] when the output length is invalid.
Uint8List hkdf(
  List<int> salt,
  List<int> ikm,
  List<int> info,
  int length,
  HashAlgorithm algorithm,
) {
  final prk = hkdfExtract(salt, ikm, algorithm);
  return hkdfExpand(prk, info, length, algorithm);
}
