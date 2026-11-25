/// Ed25519 digital signatures (RFC 8032) — implemented from scratch.
///
/// Ed25519 is an elliptic curve digital signature algorithm (EdDSA)
/// over the twisted Edwards curve `-x² + y² = 1 + d·x²·y²` (mod p,
/// where p = 2^255 - 19). It provides fast signing and verification,
/// compact 32-byte keys and 64-byte signatures, 128-bit security, and
/// deterministic signing (no random nonce needed).
///
/// ## Field representation
///
/// The reference stores GF(p) elements as five 64-bit limbs in radix
/// 2^51 with hand-rolled carry chains. This port uses exact [BigInt]
/// integers reduced mod p instead: the observable behavior —
/// encodings, vectors, verification verdicts — is identical, while
/// limb packing, addition chains, and schoolbook division have no
/// reason to exist here.
///
/// Uses the in-lane SHA-512 implementation for all hashing.
library;

import 'dart:typed_data';

import 'package:coding_adventures_sha512/coding_adventures_sha512.dart';

import 'curve.dart';

/// Generates an Ed25519 key pair from a 32-byte [seed].
///
/// Returns `(publicKey, secretKey)` where the public key is 32 bytes
/// and the secret key is 64 bytes (seed || public key).
///
/// Throws [ArgumentError] if [seed] is not exactly 32 bytes (the
/// reference takes a fixed-size array, which has no unrepresentable
/// lengths in Dart).
(Ed25519PublicKey, Ed25519SecretKey) generateKeypair(Uint8List seed) {
  _checkLength(seed, 32, 'seed');
  final h = sha512(seed);
  final a = _clamp(h);

  final bigA = ExtPoint.base().scalarMul(a.toList());
  final publicKey = Ed25519PublicKey._(bigA.encode().toList());

  final secret = Uint8List(64);
  secret.setRange(0, 32, seed);
  secret.setRange(32, 64, publicKey.bytes);
  return (publicKey, Ed25519SecretKey._(secret));
}

/// Signs [message] with a 64-byte Ed25519 [secretKey].
///
/// Returns the 64-byte signature (R || S). Signing is deterministic.
Ed25519Signature sign(List<int> message, Ed25519SecretKey secretKey) {
  final seed = secretKey.bytes.sublist(0, 32);
  final publicKey = secretKey.bytes.sublist(32, 64);

  final h = sha512(seed);
  final a = _clamp(h);
  final prefix = h.sublist(32, 64);

  // r = SHA-512(prefix || message) mod L.
  final rHash = sha512([...prefix, ...message]);
  final r = scReduce(rHash);

  // R = r · B.
  final bigR = ExtPoint.base().scalarMul(r.toList());
  final rBytes = bigR.encode();

  // k = SHA-512(R || public_key || message) mod L.
  final kHash = sha512([...rBytes.toList(), ...publicKey, ...message]);
  final k = scReduce(kHash);

  // S = (r + k·a) mod L.
  final s = scMulAdd(r.toList(), k.toList(), a.toList());

  final sig = Uint8List(64);
  sig.setRange(0, 32, rBytes.toList());
  sig.setRange(32, 64, s.toList());
  return Ed25519Signature._(sig);
}

/// Returns whether [bytes] encode a canonical, non-identity,
/// prime-order Ed25519 public key.
///
/// Generated keys live in the large prime-order subgroup. This check
/// rejects non-canonical encodings, low-order points, and points with
/// a torsion component before an external key is granted verification
/// authority.
bool isValidPublicKey(Uint8List bytes) {
  if (bytes.length != 32) return false;
  return decodePrimeOrderNonIdentity(bytes) != null;
}

/// Verifies an Ed25519 [signature] over [message] for [publicKey].
///
/// Returns `true` if the signature is valid. Never throws on invalid
/// input: malformed keys and signatures simply fail verification.
bool verify(
  List<int> message,
  Ed25519Signature signature,
  Ed25519PublicKey publicKey,
) {
  final rBytes = signature.bytes.sublist(0, 32);
  final sBytes = signature.bytes.sublist(32, 64);

  if (!scIsCanonical(sBytes)) {
    return false;
  }

  final bigR = decodeNonSmallOrder(rBytes);
  if (bigR == null) {
    return false;
  }

  final bigA = decodeNonSmallOrder(publicKey.bytes);
  if (bigA == null) {
    return false;
  }

  final kHash = sha512([...rBytes, ...publicKey.bytes, ...message]);
  final k = scReduce(kHash);

  final lhs = ExtPoint.base().scalarMul(sBytes);
  final rhs = bigR.add(bigA.scalarMul(k.toList()));

  return pointEq(lhs, rhs);
}

/// A 32-byte Ed25519 public key.
class Ed25519PublicKey {
  /// The 32 key bytes.
  final Uint8List bytes;

  Ed25519PublicKey._(List<int> bytes) : bytes = Uint8List.fromList(bytes);

  /// Parses 32 bytes into a public key container.
  ///
  /// This checks the length only; use [isValidPublicKey] for the
  /// cryptographic validity check.
  ///
  /// Throws [ArgumentError] unless [bytes] has length 32.
  factory Ed25519PublicKey(Uint8List bytes) {
    _checkLength(bytes, 32, 'public key');
    return Ed25519PublicKey._(bytes);
  }

  @override
  bool operator ==(Object other) =>
      other is Ed25519PublicKey && _bytesEqual(bytes, other.bytes);

  @override
  int get hashCode => Object.hashAll(bytes);

  @override
  String toString() => 'Ed25519PublicKey(${bytes.length} bytes)';
}

/// A 64-byte Ed25519 secret key (seed || public key).
class Ed25519SecretKey {
  /// The 64 key bytes.
  final Uint8List bytes;

  Ed25519SecretKey._(List<int> bytes) : bytes = Uint8List.fromList(bytes);

  /// Parses 64 bytes into a secret key container.
  ///
  /// Throws [ArgumentError] unless [bytes] has length 64.
  factory Ed25519SecretKey(Uint8List bytes) {
    _checkLength(bytes, 64, 'secret key');
    return Ed25519SecretKey._(bytes);
  }

  /// Value equality for tests and debugging. Not constant-time: never
  /// use this to compare secrets in a security decision.
  @override
  bool operator ==(Object other) =>
      other is Ed25519SecretKey && _bytesEqual(bytes, other.bytes);

  @override
  int get hashCode => Object.hashAll(bytes);
}

/// A 64-byte Ed25519 signature (R || S).
class Ed25519Signature {
  /// The 64 signature bytes.
  final Uint8List bytes;

  Ed25519Signature._(List<int> bytes) : bytes = Uint8List.fromList(bytes);

  /// Parses 64 bytes into a signature container.
  ///
  /// Throws [ArgumentError] unless [bytes] has length 64.
  factory Ed25519Signature(Uint8List bytes) {
    _checkLength(bytes, 64, 'signature');
    return Ed25519Signature._(bytes);
  }

  @override
  bool operator ==(Object other) =>
      other is Ed25519Signature && _bytesEqual(bytes, other.bytes);

  @override
  int get hashCode => Object.hashAll(bytes);
}

Uint8ListBytes _clamp(Uint8List hash) {
  final clamped = Uint8ListBytes(32);
  for (var i = 0; i < 32; i++) {
    clamped[i] = hash[i];
  }
  clamped[0] &= 248;
  clamped[31] &= 127;
  clamped[31] |= 64;
  return clamped;
}

void _checkLength(List<int> bytes, int want, String what) {
  if (bytes.length != want) {
    throw ArgumentError('$what must be exactly $want bytes');
  }
}

bool _bytesEqual(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}
