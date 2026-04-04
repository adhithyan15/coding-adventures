# @coding-adventures/reed-solomon

Reed-Solomon error-correcting codes over GF(256). TypeScript implementation of [MA02](../../../specs/MA02-reed-solomon.md).

The math behind QR codes, CDs, DVDs, deep-space communication, and RAID-6.

## Install

```bash
npm install @coding-adventures/reed-solomon
```

## Quick Start

```ts
import { encode, decode } from "@coding-adventures/reed-solomon";

const message = new TextEncoder().encode("Hello, World!");
const nCheck = 8; // t = 4 errors correctable

// Encode: systematic — message bytes unchanged, check bytes appended
const codeword = encode(message, nCheck);
console.log(codeword.length); // message.length + 8

// Corrupt 4 bytes — still recoverable
codeword[0] ^= 0xff;
codeword[3] ^= 0xaa;
codeword[7] ^= 0x55;
codeword[10] ^= 0x0f;

const recovered = decode(codeword, nCheck);
// recovered deep-equals the original message
```

## API

### `encode(message, nCheck)`

Encode `message` with `nCheck` redundancy bytes. Returns a `Uint8Array` of
length `message.length + nCheck`. The first `message.length` bytes are the
original message (systematic encoding).

**Throws** `InvalidInputError` if `nCheck` is 0, odd, or `message.length + nCheck > 255`.

### `decode(received, nCheck)`

Decode a (possibly corrupted) codeword. Returns the recovered message bytes.
Corrects up to `t = nCheck / 2` byte errors.

**Throws** `TooManyErrorsError` if more than `t` errors are present.
**Throws** `InvalidInputError` if `nCheck` is 0/odd or received is too short.

### `syndromes(received, nCheck)`

Compute the `nCheck` syndrome values `S_j = received(α^j)` for `j = 1…nCheck`.
All-zero → no errors; any non-zero → errors detected.

### `buildGenerator(nCheck)`

Build the monic generator polynomial `g(x) = ∏(x + αⁱ)` for `i = 1…nCheck`.
Returns a little-endian `Uint8Array` of length `nCheck + 1`.

### `errorLocator(syndromes)`

Compute the error locator polynomial `Λ(x)` from a syndrome array using
Berlekamp-Massey. Returns `Λ` in little-endian form with `Λ[0] = 1`.

## Error Classes

```ts
class TooManyErrorsError extends Error {} // > t errors; unrecoverable
class InvalidInputError extends Error {}  // bad nCheck or codeword too long
```

## Correction Capacity

| `nCheck` | `t` (errors correctable) |
|----------|--------------------------|
| 2 | 1 |
| 4 | 2 |
| 8 | 4 |
| 16 | 8 |
| 32 | 16 |

## Stack

```
MA00  @coding-adventures/polynomial
MA01  @coding-adventures/gf256
MA02  @coding-adventures/reed-solomon  ← this package
```

## Specification

See [`code/specs/MA02-reed-solomon.md`](../../../specs/MA02-reed-solomon.md).
