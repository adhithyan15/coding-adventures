/// Allocation-oriented educational AES modes: PKCS#7 padding plus
/// ECB, CBC, CTR, and GCM built over the in-lane AES block cipher.
///
/// ECB is insecure for real use (identical blocks encrypt
/// identically); CBC needs unpredictable IVs and resists padding
/// oracles poorly; CTR and GCM are the modern choices, with GCM
/// adding authentication. Key-length failures propagate from the
/// AES lane unchanged, matching the reference's `?`.
library;

import 'dart:typed_data';

import 'package:coding_adventures_aes/aes.dart' as aes;

import 'errors.dart';
import 'fixed.dart';

/// AES block size in bytes.
const int _blockSize = 16;

/// Applies PKCS#7 padding: 1–16 bytes so the result is a multiple
/// of 16 (a full block of 0x10 when already aligned, keeping
/// unpadding unambiguous).
Uint8List pkcs7Pad(List<int> data) {
  final padLen = _blockSize - (data.length % _blockSize);
  final out = Uint8List(data.length + padLen);
  out.setRange(0, data.length, data);
  for (var i = data.length; i < out.length; i++) {
    out[i] = padLen;
  }
  return out;
}

/// Removes PKCS#7 padding, validating the pad bytes with an
/// OR-accumulated diff so the scan takes the same time regardless
/// of which byte fails.
///
/// Throws [InvalidPaddedLength] unless the input is a non-empty
/// multiple of 16 bytes, and [InvalidPadding] on a bad pad value
/// or inconsistent pad bytes.
Uint8List pkcs7Unpad(List<int> data) {
  if (data.isEmpty || data.length % _blockSize != 0) {
    throw const InvalidPaddedLength();
  }
  final padLen = data.last;
  if (padLen < 1 || padLen > _blockSize) {
    throw const InvalidPadding();
  }
  var diff = 0;
  for (var i = data.length - padLen; i < data.length; i++) {
    diff |= data[i] ^ padLen;
  }
  if (diff != 0) {
    throw const InvalidPadding();
  }
  return Uint8List.fromList(data.sublist(0, data.length - padLen));
}

/// Encrypts with AES-ECB (INSECURE — educational only): pads, then
/// encrypts each block independently.
Uint8List ecbEncrypt(List<int> plaintext, List<int> key) {
  final padded = pkcs7Pad(plaintext);
  final result = Uint8List(padded.length);
  for (var offset = 0; offset < padded.length; offset += _blockSize) {
    final encrypted =
        aes.encryptBlock(padded.sublist(offset, offset + _blockSize), key);
    result.setRange(offset, offset + _blockSize, encrypted);
  }
  return result;
}

/// Decrypts AES-ECB (INSECURE — educational only), then unpads.
///
/// Throws [EcbInvalidCiphertext] unless the input is a non-empty
/// multiple of 16 bytes.
Uint8List ecbDecrypt(List<int> ciphertext, List<int> key) {
  if (ciphertext.isEmpty || ciphertext.length % _blockSize != 0) {
    throw const EcbInvalidCiphertext();
  }
  final result = Uint8List(ciphertext.length);
  for (var offset = 0; offset < ciphertext.length; offset += _blockSize) {
    final decrypted =
        aes.decryptBlock(ciphertext.sublist(offset, offset + _blockSize), key);
    result.setRange(offset, offset + _blockSize, decrypted);
  }
  return pkcs7Unpad(result);
}

/// Encrypts with AES-CBC: `C[0] = E(P[0] XOR IV)`,
/// `C[i] = E(P[i] XOR C[i-1])` over PKCS#7-padded input.
///
/// Throws [CbcInvalidIv] unless [iv] is 16 bytes.
Uint8List cbcEncrypt(List<int> plaintext, List<int> key, List<int> iv) {
  if (iv.length != _blockSize) {
    throw CbcInvalidIv(iv.length);
  }
  final padded = pkcs7Pad(plaintext);
  final result = Uint8List(padded.length);
  var prev = Uint8List.fromList(iv);
  for (var offset = 0; offset < padded.length; offset += _blockSize) {
    final xored = Uint8List(_blockSize);
    for (var i = 0; i < _blockSize; i++) {
      xored[i] = padded[offset + i] ^ prev[i];
    }
    final encrypted = aes.encryptBlock(xored, key);
    result.setRange(offset, offset + _blockSize, encrypted);
    prev = encrypted;
  }
  return result;
}

/// Decrypts AES-CBC, then unpads.
///
/// Throws [CbcInvalidIv] unless [iv] is 16 bytes, and
/// [CbcInvalidCiphertext] unless the input is a non-empty multiple
/// of 16 bytes.
Uint8List cbcDecrypt(List<int> ciphertext, List<int> key, List<int> iv) {
  if (iv.length != _blockSize) {
    throw CbcInvalidIv(iv.length);
  }
  if (ciphertext.isEmpty || ciphertext.length % _blockSize != 0) {
    throw const CbcInvalidCiphertext();
  }
  final result = Uint8List(ciphertext.length);
  var prev = Uint8List.fromList(iv);
  for (var offset = 0; offset < ciphertext.length; offset += _blockSize) {
    final decrypted =
        aes.decryptBlock(ciphertext.sublist(offset, offset + _blockSize), key);
    for (var i = 0; i < _blockSize; i++) {
      result[offset + i] = decrypted[i] ^ prev[i];
    }
    prev = Uint8List.fromList(ciphertext.sublist(offset, offset + _blockSize));
  }
  return pkcs7Unpad(result);
}

Uint8List _counterBlock(List<int> nonce, int counter) {
  final block = Uint8List(16);
  block.setRange(0, 12, nonce);
  block[12] = (counter >> 24) & 0xff;
  block[13] = (counter >> 16) & 0xff;
  block[14] = (counter >> 8) & 0xff;
  block[15] = counter & 0xff;
  return block;
}

/// Encrypts with AES-CTR: keystream blocks from nonce || BE counter
/// (starting at 1), XORed over the input with no padding. Never
/// reuse a nonce with a key.
///
/// Throws [CtrInvalidNonce] unless [nonce] is 12 bytes.
Uint8List ctrEncrypt(List<int> plaintext, List<int> key, List<int> nonce) {
  if (nonce.length != 12) {
    throw CtrInvalidNonce(nonce.length);
  }
  final result = Uint8List(plaintext.length);
  var counter = 1;
  for (var offset = 0; offset < plaintext.length; offset += _blockSize) {
    final keystream = aes.encryptBlock(_counterBlock(nonce, counter), key);
    final end = offset + _blockSize < plaintext.length
        ? offset + _blockSize
        : plaintext.length;
    for (var i = offset; i < end; i++) {
      result[i] = plaintext[i] ^ keystream[i - offset];
    }
    counter = (counter + 1) & 0xffffffff;
  }
  return result;
}

/// Decrypts AES-CTR: identical to encryption (stream-cipher
/// property).
Uint8List ctrDecrypt(List<int> ciphertext, List<int> key, List<int> nonce) =>
    ctrEncrypt(ciphertext, key, nonce);

Uint8List _incrementCounter(Uint8List block) {
  final result = Uint8List.fromList(block);
  for (var i = 15; i >= 12; i--) {
    result[i] = (result[i] + 1) & 0xff;
    if (result[i] != 0) {
      break;
    }
  }
  return result;
}

Uint8List _ghash(Uint8List h, List<int> aad, List<int> ciphertext) {
  final output = Uint8List(16);
  // Addressable Dart lists always fit the GCM length fields (at most
  // 2^32 bytes << 2^61), so the fixed boundary cannot fail here.
  ghashFixed(h, aad, ciphertext, output);
  return output;
}

/// Encrypts with AES-GCM, returning `(ciphertext, tag)`.
///
/// `H = E(0^128)`; `J0 = IV || 1`; CTR from J0+1; tag over AAD and
/// ciphertext XOR `E(J0)`.
///
/// Throws [GcmInvalidIv] unless [iv] is 12 bytes.
(Uint8List, Uint8List) gcmEncrypt(
  List<int> plaintext,
  List<int> key,
  List<int> iv,
  List<int> aad,
) {
  if (iv.length != 12) {
    throw GcmInvalidIv(iv.length);
  }
  final h = aes.encryptBlock(Uint8List(16), key);
  final j0 = Uint8List(16);
  j0.setRange(0, 12, iv);
  j0[15] = 1;

  final ciphertext = Uint8List(plaintext.length);
  var counter = j0;
  for (var offset = 0; offset < plaintext.length; offset += _blockSize) {
    counter = _incrementCounter(counter);
    final keystream = aes.encryptBlock(counter, key);
    final end = offset + _blockSize < plaintext.length
        ? offset + _blockSize
        : plaintext.length;
    for (var i = offset; i < end; i++) {
      ciphertext[i] = plaintext[i] ^ keystream[i - offset];
    }
  }

  final ghashResult = _ghash(h, aad, ciphertext);
  final encJ0 = aes.encryptBlock(j0, key);
  final tag = Uint8List(16);
  for (var i = 0; i < 16; i++) {
    tag[i] = ghashResult[i] ^ encJ0[i];
  }
  return (ciphertext, tag);
}

/// Decrypts AES-GCM, verifying the tag (constant-time
/// OR-accumulated compare) before releasing plaintext.
///
/// Throws [GcmInvalidIv] unless [iv] is 12 bytes,
/// [GcmAuthenticationFailed] on tag mismatch, and [ArgumentError]
/// unless [tag] is 16 bytes.
Uint8List gcmDecrypt(
  List<int> ciphertext,
  List<int> key,
  List<int> iv,
  List<int> aad,
  List<int> tag,
) {
  if (iv.length != 12) {
    throw GcmInvalidIv(iv.length);
  }
  if (tag.length != 16) {
    throw ArgumentError('tag must be exactly 16 bytes');
  }
  final h = aes.encryptBlock(Uint8List(16), key);
  final j0 = Uint8List(16);
  j0.setRange(0, 12, iv);
  j0[15] = 1;

  final ghashResult = _ghash(h, aad, ciphertext);
  final encJ0 = aes.encryptBlock(j0, key);
  var diff = 0;
  for (var i = 0; i < 16; i++) {
    diff |= (ghashResult[i] ^ encJ0[i]) ^ tag[i];
  }
  if (diff != 0) {
    throw const GcmAuthenticationFailed();
  }

  final plaintext = Uint8List(ciphertext.length);
  var counter = j0;
  for (var offset = 0; offset < ciphertext.length; offset += _blockSize) {
    counter = _incrementCounter(counter);
    final keystream = aes.encryptBlock(counter, key);
    final end = offset + _blockSize < ciphertext.length
        ? offset + _blockSize
        : ciphertext.length;
    for (var i = offset; i < end; i++) {
      plaintext[i] = ciphertext[i] ^ keystream[i - offset];
    }
  }
  return plaintext;
}
