# coding_adventures_sha512

SHA-512 cryptographic hash function (FIPS 180-4) implemented from scratch in pure Dart with zero external dependencies and zero host-crypto imports.

This Dart package implements the standard SHA-512 algorithm adhering to the `HF04-sha512` contract, featuring both one-shot hashing functions and a streaming, non-destructive `Sha512Hasher`.

## Features

- **Exact 64-Bit Arithmetic on VM and Web**: Employs `{hi32, lo32}` unsigned 32-bit integer pairs (`Word64`), guaranteeing bit-identical behavior on 64-bit Dart VM and 53-bit JavaScript web runtimes.
- **Full HF04 Streaming Contract**: Retains $\le 1$ partial 128-byte block, prevents caller buffer aliasing, supports non-destructive snapshots, and allows independent cloning (`cloneHasher()` / `copy()`).
- **128-Bit FIPS Domain Bound**: Rejects message bit-lengths $\ge 2^{128}$ fail-closed before state mutation.
- **Standing Rule #20 Compliance**: 0 double-quote string literals in operational logic; 100% of error/diagnostic strings centralized in `Sha512Messages`.

## API

| Function / Class | Description |
|---|---|
| `sha512(List<int> data)` → `Uint8List` | Computes the 64-byte SHA-512 digest. |
| `sha512Hex(List<int> data)` → `String` | Computes the 128-character lowercase hex digest. |
| `Sha512Hasher` | Streaming hasher with `update`, `digest`, `hexDigest`, and `copy` / `cloneHasher`. |
| `encodeBitLength(BigInt high, BigInt low)` → `Uint8List` | Encodes the 128-bit message length field. |
| `digestLength` | Constant `64` bytes. |
| `Sha512Exception` | Exception thrown on validation or domain overflow errors. |

## 6-Class SHA-512 Resilience Showcase

1. **Counter Wraparound Past $2^{61}$ Bytes**: Correctly carries the 64-bit lower byte counter into the upper counter when bit-length exceeds $2^{64}$ bits ($2^{61}$ bytes).
2. **FIPS $2^{128}$-Bit Overflow Failure-Atomicity**: Bounds check rejects lengths $\ge 2^{128}$ bits before state mutation, preserving hasher integrity.
3. **Padding-Boundary Splits**: Robustly pads across single-block and double-block padding boundaries (`111, 112, 127, 128, 129, 239, 240, 255, 256, 257` bytes).
4. **Buffer Aliasing & Post-Snapshot Continuation**: Clones state on snapshots so callers can continue streaming without corrupting previous digests.
5. **Fail-Closed Input Validation**: Synchronously rejects invalid byte integers (`(b & ~0xFF) != 0`) and negative/out-of-bounds indices.
6. **Payload-Blind Error Diagnostics**: Isolates all error messages in `Sha512Messages` without leaking caller inputs or lengths.

## Usage

```dart
import 'dart:convert';
import 'package:coding_adventures_sha512/coding_adventures_sha512.dart';

void main() {
  final input = utf8.encode('abc');
  final hex = sha512Hex(input);
  print(hex);

  final hasher = Sha512Hasher();
  hasher.update(utf8.encode('ab'));
  hasher.update(utf8.encode('c'));
  assert(hasher.hexDigest() == hex);
}
```
