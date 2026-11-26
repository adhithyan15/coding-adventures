/// AES block cipher (FIPS 197): SubBytes, ShiftRows, MixColumns,
/// AddRoundKey over a 4×4 column-major byte state, with 10/12/14
/// rounds for 128/192/256-bit keys.
///
/// State here is a flat 16-byte column-major buffer
/// (`state[row + 4 * col]`), which is exactly the reference's
/// `state[row][col]` matrix laid out in the same order the bytes
/// arrive. Two paths mirror the reference: the compatibility
/// implementation close to the FIPS pseudocode (table-indexed
/// S-box), and the fixed-memory AES-128 boundary with a table-free
/// arithmetic S-box circuit and wipe-on-drop scratch.
library;

import 'dart:typed_data';

import 'package:coding_adventures_zeroize/zeroize.dart';

import 'errors.dart';
import 'sbox.dart';

/// Lazily built global tables (Dart top-level finals initialize on
/// first use; no lock needed single-threaded).
final (Uint8List, Uint8List) _tables = buildSboxes();

/// The AES S-box (`SBOX[0x00] == 0x63`, FIPS 197 Figure 7).
/// Returned by copy so callers cannot mutate the shared tables.
Uint8List sbox() => Uint8List.fromList(_tables.$1);

/// The AES inverse S-box (`INV_SBOX[SBOX[b]] == b`).
/// Returned by copy so callers cannot mutate the shared tables.
Uint8List invSbox() => Uint8List.fromList(_tables.$2);

/// Live table access for internal round functions (read-only by
/// convention; never exposed).
Uint8List _sboxTable() => _tables.$1;
Uint8List _invSboxTable() => _tables.$2;

/// Round constants: Rcon[i] = x^(i-1) in GF(2^8). Index 0 unused
/// (NIST is 1-indexed); indices 1..14 cover AES-256.
const List<int> _rcon = [
  0x00,
  0x01,
  0x02,
  0x04,
  0x08,
  0x10,
  0x20,
  0x40,
  0x80,
  0x1b,
  0x36,
  0x6c,
  0xd8,
  0xab,
  0x4d,
];

/// Expands a 16-, 24-, or 32-byte key into Nr+1 (11/13/15) round
/// keys of 16 bytes each, flat column-major.
///
/// Throws [AesInvalidKeyLength] otherwise.
List<Uint8List> expandKey(List<int> key) {
  if (key.length != 16 && key.length != 24 && key.length != 32) {
    throw AesInvalidKeyLength(key.length);
  }
  final sb = _sboxTable();
  final nk = key.length ~/ 4;
  final nr = switch (nk) { 4 => 10, 6 => 12, _ => 14 };
  final totalWords = 4 * (nr + 1);

  final w = <List<int>>[];
  for (var i = 0; i < nk; i++) {
    w.add(List<int>.of(key.sublist(4 * i, 4 * i + 4)));
  }
  for (var i = nk; i < totalWords; i++) {
    var temp = List<int>.of(w[i - 1]);
    if (i % nk == 0) {
      temp = [temp[1], temp[2], temp[3], temp[0]];
      temp = [sb[temp[0]], sb[temp[1]], sb[temp[2]], sb[temp[3]]];
      temp[0] ^= _rcon[i ~/ nk];
    } else if (nk == 8 && i % nk == 4) {
      temp = [sb[temp[0]], sb[temp[1]], sb[temp[2]], sb[temp[3]]];
    }
    final prev = w[i - nk];
    w.add([
      prev[0] ^ temp[0],
      prev[1] ^ temp[1],
      prev[2] ^ temp[2],
      prev[3] ^ temp[3],
    ]);
  }

  // Pack into round keys: state[row][col] = w[4*rk + col][row],
  // i.e. flat byte row + 4*col.
  return List<Uint8List>.generate(nr + 1, (rk) {
    final out = Uint8List(16);
    for (var col = 0; col < 4; col++) {
      final word = w[4 * rk + col];
      for (var row = 0; row < 4; row++) {
        out[row + 4 * col] = word[row];
      }
    }
    return out;
  });
}

void _addRoundKey(Uint8List state, List<int> roundKey) {
  for (var i = 0; i < 16; i++) {
    state[i] ^= roundKey[i];
  }
}

void _subBytes(Uint8List state) {
  final sb = _sboxTable();
  for (var i = 0; i < 16; i++) {
    state[i] = sb[state[i]];
  }
}

void _invSubBytes(Uint8List state) {
  final isb = _invSboxTable();
  for (var i = 0; i < 16; i++) {
    state[i] = isb[state[i]];
  }
}

void _shiftRows(Uint8List state) {
  final copy = Uint8List.fromList(state);
  for (var r = 0; r < 4; r++) {
    for (var c = 0; c < 4; c++) {
      state[r + 4 * c] = copy[r + 4 * ((c + r) % 4)];
    }
  }
}

void _invShiftRows(Uint8List state) {
  final copy = Uint8List.fromList(state);
  for (var r = 0; r < 4; r++) {
    for (var c = 0; c < 4; c++) {
      state[r + 4 * c] = copy[r + 4 * ((c + 4 - r) % 4)];
    }
  }
}

void _mixColumn(List<int> s) {
  final s0 = s[0], s1 = s[1], s2 = s[2], s3 = s[3];
  s[0] = xtime(s0) ^ (xtime(s1) ^ s1) ^ s2 ^ s3;
  s[1] = s0 ^ xtime(s1) ^ (xtime(s2) ^ s2) ^ s3;
  s[2] = s0 ^ s1 ^ xtime(s2) ^ (xtime(s3) ^ s3);
  s[3] = (xtime(s0) ^ s0) ^ s1 ^ s2 ^ xtime(s3);
}

void _invMixColumn(List<int> s) {
  final s0 = s[0], s1 = s[1], s2 = s[2], s3 = s[3];
  s[0] = gfMul(0x0e, s0) ^ gfMul(0x0b, s1) ^ gfMul(0x0d, s2) ^ gfMul(0x09, s3);
  s[1] = gfMul(0x09, s0) ^ gfMul(0x0e, s1) ^ gfMul(0x0b, s2) ^ gfMul(0x0d, s3);
  s[2] = gfMul(0x0d, s0) ^ gfMul(0x09, s1) ^ gfMul(0x0e, s2) ^ gfMul(0x0b, s3);
  s[3] = gfMul(0x0b, s0) ^ gfMul(0x0d, s1) ^ gfMul(0x09, s2) ^ gfMul(0x0e, s3);
}

void _mixColumns(Uint8List state) {
  for (var col = 0; col < 4; col++) {
    final column = [
      state[4 * col],
      state[4 * col + 1],
      state[4 * col + 2],
      state[4 * col + 3],
    ];
    _mixColumn(column);
    for (var row = 0; row < 4; row++) {
      state[4 * col + row] = column[row];
    }
  }
}

void _invMixColumns(Uint8List state) {
  for (var col = 0; col < 4; col++) {
    final column = [
      state[4 * col],
      state[4 * col + 1],
      state[4 * col + 2],
      state[4 * col + 3],
    ];
    _invMixColumn(column);
    for (var row = 0; row < 4; row++) {
      state[4 * col + row] = column[row];
    }
  }
}

/// Encrypts one 16-byte block with a 16-, 24-, or 32-byte key
/// (AES-128/192/256: 10/12/14 rounds).
///
/// Throws [AesInvalidKeyLength] unless the key is a valid AES
/// length, and [ArgumentError] unless [block] is exactly 16 bytes
/// (the reference takes a fixed-size array, which has no
/// unrepresentable lengths in Dart).
Uint8List encryptBlock(List<int> block, List<int> key) {
  if (block.length != 16) {
    throw ArgumentError('block must be exactly 16 bytes');
  }
  final roundKeys = expandKey(key);
  final nr = roundKeys.length - 1;
  final state = Uint8List.fromList(block);
  _addRoundKey(state, roundKeys[0]);
  for (var round = 1; round < nr; round++) {
    _subBytes(state);
    _shiftRows(state);
    _mixColumns(state);
    _addRoundKey(state, roundKeys[round]);
  }
  _subBytes(state);
  _shiftRows(state);
  _addRoundKey(state, roundKeys[nr]);
  return state;
}

/// Decrypts one 16-byte block: inverse rounds in reverse
/// (InvShiftRows → InvSubBytes → AddRoundKey → InvMixColumns).
///
/// Throws [AesInvalidKeyLength] unless the key is a valid AES
/// length, and [ArgumentError] unless [block] is exactly 16 bytes.
Uint8List decryptBlock(List<int> block, List<int> key) {
  if (block.length != 16) {
    throw ArgumentError('block must be exactly 16 bytes');
  }
  final roundKeys = expandKey(key);
  final nr = roundKeys.length - 1;
  final state = Uint8List.fromList(block);
  _addRoundKey(state, roundKeys[nr]);
  for (var round = nr - 1; round >= 1; round--) {
    _invShiftRows(state);
    _invSubBytes(state);
    _addRoundKey(state, roundKeys[round]);
    _invMixColumns(state);
  }
  _invShiftRows(state);
  _invSubBytes(state);
  _addRoundKey(state, roundKeys[0]);
  return state;
}

/// Expands a 16-byte AES-128 key into eleven contiguous 16-byte
/// round keys (176 bytes), using the table-free circuit.
void _expandAes128KeyFixed(
  List<int> key,
  Uint8List expanded,
  Uint8List temp,
) {
  expanded.setRange(0, 16, key);
  var generated = 16;
  var roundConstant = 1;
  while (generated < expanded.length) {
    temp.setRange(0, 4, expanded.sublist(generated - 4, generated));
    if (generated % 16 == 0) {
      final rotated = [temp[1], temp[2], temp[3], temp[0]];
      temp.setRange(0, 4, [for (final b in rotated) sboxFixed(b)]);
      temp[0] ^= roundConstant;
      roundConstant = xtimeFixed(roundConstant);
    }
    for (var i = 0; i < 4; i++) {
      expanded[generated] = expanded[generated - 16] ^ temp[i];
      generated += 1;
    }
  }
}

void _addRoundKeyFixed(Uint8List state, List<int> roundKey) {
  for (var i = 0; i < 16; i++) {
    state[i] ^= roundKey[i];
  }
}

void _subBytesFixed(Uint8List state) {
  for (var i = 0; i < 16; i++) {
    state[i] = sboxFixed(state[i]);
  }
}

/// Fixed ShiftRows over the flat column-major state
/// (index = 4 * column + row) via a reusable scratch block.
void _shiftRowsFixed(Uint8List state, Uint8List scratch) {
  scratch.setRange(0, 16, state);
  state[1] = scratch[5];
  state[5] = scratch[9];
  state[9] = scratch[13];
  state[13] = scratch[1];
  state[2] = scratch[10];
  state[6] = scratch[14];
  state[10] = scratch[2];
  state[14] = scratch[6];
  state[3] = scratch[15];
  state[7] = scratch[3];
  state[11] = scratch[7];
  state[15] = scratch[11];
}

void _mixColumnsFixed(Uint8List state, Uint8List column) {
  for (var start = 0; start < 16; start += 4) {
    column.setRange(0, 4, state.sublist(start, start + 4));
    final s0 = column[0];
    final s1 = column[1];
    final s2 = column[2];
    final s3 = column[3];
    state[start] = xtimeFixed(s0) ^ (xtimeFixed(s1) ^ s1) ^ s2 ^ s3;
    state[start + 1] = s0 ^ xtimeFixed(s1) ^ (xtimeFixed(s2) ^ s2) ^ s3;
    state[start + 2] = s0 ^ s1 ^ xtimeFixed(s2) ^ (xtimeFixed(s3) ^ s3);
    state[start + 3] = (xtimeFixed(s0) ^ s0) ^ s1 ^ s2 ^ xtimeFixed(s3);
  }
}

/// Encrypts exactly one block with AES-128 into caller-owned
/// [output], using fixed memory and no S-box table lookups.
///
/// Unlike [encryptBlock], every copied secret lives in a fixed
/// wipe-on-drop owner (state, 176-byte expanded key, scratch
/// buffers — all scrubbed in `finally` blocks, since Dart has no
/// destructors), and SubBytes runs the fixed-work arithmetic
/// circuit. The borrowed [plaintext] and [key] remain caller-owned
/// and are not wiped here.
///
/// This is only AES-128 block encryption: no authentication,
/// nonces, modes, or padding.
///
/// Throws [ArgumentError] unless all three buffers are exactly 16
/// bytes (the reference takes fixed-size arrays, which admit no
/// other lengths).
void aes128EncryptBlockFixed(
  List<int> plaintext,
  List<int> key,
  Uint8List output,
) {
  if (plaintext.length != 16) {
    throw ArgumentError('plaintext must be exactly 16 bytes');
  }
  if (key.length != 16) {
    throw ArgumentError('key must be exactly 16 bytes');
  }
  if (output.length != 16) {
    throw ArgumentError('output must be exactly 16 bytes');
  }
  final expanded = ZeroizingBytes(Uint8List(176));
  final state = ZeroizingBytes(Uint8List(16));
  final keyTemp = ZeroizingBytes(Uint8List(4));
  final rowScratch = ZeroizingBytes(Uint8List(16));
  final column = ZeroizingBytes(Uint8List(4));
  try {
    state.bytes.setRange(0, 16, plaintext);
    _expandAes128KeyFixed(key, expanded.bytes, keyTemp.bytes);
    _addRoundKeyFixed(
        state.bytes, Uint8List.sublistView(expanded.bytes, 0, 16));
    for (var round = 1; round < 10; round++) {
      _subBytesFixed(state.bytes);
      _shiftRowsFixed(state.bytes, rowScratch.bytes);
      _mixColumnsFixed(state.bytes, column.bytes);
      final start = round * 16;
      _addRoundKeyFixed(state.bytes,
          Uint8List.sublistView(expanded.bytes, start, start + 16));
    }
    _subBytesFixed(state.bytes);
    _shiftRowsFixed(state.bytes, rowScratch.bytes);
    _addRoundKeyFixed(
        state.bytes, Uint8List.sublistView(expanded.bytes, 160, 176));
    output.setRange(0, 16, state.bytes);
  } finally {
    expanded.dispose();
    state.dispose();
    keyTemp.dispose();
    rowScratch.dispose();
    column.dispose();
  }
}
