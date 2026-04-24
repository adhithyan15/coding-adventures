# coding_adventures_reed_solomon

Reed-Solomon error-correcting codes over GF(256).

## What Is Reed-Solomon?

RS is a **block error-correcting code**: you add `nCheck` redundancy bytes
to a message and can recover the original even if up to `t = nCheck / 2`
bytes are corrupted.

| System | Role |
|--------|------|
| **QR codes** | Up to 30% of a QR symbol can be damaged and still decode |
| **CDs / DVDs** | Two-level RS corrects scratches (CIRC) |
| **Hard drives** | Sector-level error correction |
| **Voyager probes** | Transmit images across 20+ billion km |
| **RAID-6** | Two parity drives are exactly an RS code over GF(256) |

## Usage

```dart
import 'dart:typed_data';
import 'package:coding_adventures_reed_solomon/coding_adventures_reed_solomon.dart';

// Encode: add 8 check bytes (t = 4 errors correctable)
final message = Uint8List.fromList([1, 2, 3, 4, 5]);
final codeword = rsEncode(message, 8);
// codeword = [1, 2, 3, 4, 5, <8 check bytes>]

// Corrupt 3 bytes (within t = 4)
codeword[0] ^= 0xFF;
codeword[2] ^= 0xAA;
codeword[4] ^= 0x55;

// Decode: recover original message
final recovered = rsDecode(codeword, 8);
// recovered == [1, 2, 3, 4, 5]
```

## API

| Function | Returns | Notes |
|----------|---------|-------|
| `rsEncode(message, nCheck)` | `Uint8List` | Systematic codeword |
| `rsDecode(received, nCheck)` | `Uint8List` | Throws `TooManyErrorsException` or `InvalidInputException` |
| `rsBuildGenerator(nCheck)` | `Uint8List` | RS generator polynomial (LE) |
| `rsSyndromes(received, nCheck)` | `Uint8List` | Syndrome values |
| `rsErrorLocator(synds)` | `Uint8List` | Λ(x) via Berlekamp-Massey |

## Constraints

- `nCheck` must be even and ≥ 2
- `message.length + nCheck` must be ≤ 255
- `received.length` must be ≥ `nCheck`

## How It Fits in the Stack

```
MA00  polynomial       ← coefficient-array polynomial arithmetic
MA01  gf256            ← GF(2^8) field arithmetic
MA02  reed-solomon     ← THIS PACKAGE
MA03  qr-encoder       ← QR code generation, uses MA02 for error correction
```

## Running Tests

```bash
dart pub get
dart test
```

## Spec

[MA02-reed-solomon.md](../../../../specs/MA02-reed-solomon.md)
