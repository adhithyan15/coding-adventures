# reed-solomon — Java

Reed-Solomon error-correcting codes over GF(256) for Java.

## What This Is

Reed-Solomon adds redundancy bytes to a message so that a decoder can recover
the original even when some bytes are corrupted:

- Encode `k` message bytes + `nCheck` redundancy bytes = `n = k + nCheck` codeword
- Decode corrects up to `t = nCheck / 2` byte errors

## Quick Start

```java
import com.codingadventures.reedsolomon.ReedSolomon;

byte[] message = {72, 101, 108, 108, 111};  // "Hello"
int nCheck = 8;                              // t = 4 errors correctable

byte[] codeword = ReedSolomon.encode(message, nCheck);
// codeword[0..4] == message (systematic encoding)

// Corrupt 4 bytes — still recoverable
codeword[0] ^= 0xFF;
codeword[2] ^= 0xAA;
codeword[4] ^= 0xBB;
codeword[6] ^= 0xCC;

byte[] recovered = ReedSolomon.decode(codeword, nCheck);
// recovered equals message
```

## Decoding Pipeline

1. Compute syndromes — if all zero, no errors
2. Berlekamp-Massey → error locator polynomial Λ(x)
3. Chien search → error positions
4. Forney algorithm → error magnitudes
5. Apply corrections

## Constraints

- `nCheck` must be even and ≥ 2
- `message.length + nCheck ≤ 255` (GF(256) block size limit)

## Exceptions

- `RsTooManyErrorsException` — more than `t = nCheck/2` errors
- `RsInvalidInputException` — invalid parameters

## Spec

See `code/specs/MA02-reed-solomon.md` for the full specification.

## Tests

```
gradle test
```

Part of the [MA series](../../../../specs/MA02-reed-solomon.md) — the math foundation
for 2D barcodes.
