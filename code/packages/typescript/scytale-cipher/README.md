# Scytale Cipher (TypeScript)

Ancient Spartan transposition cipher implementation in TypeScript.

This package follows [CR02](../../../specs/CR02-scytale-cipher.md). Grid cells are Unicode scalar values rather than UTF-16 code units, uneven ciphertext columns are reconstructed explicitly, and only trailing U+0020 padding is removed. `bruteForce` rejects inputs above 4096 scalars before allocating quadratic candidate output. Production code is deterministic pure computation with no OS capabilities.

## Usage

```typescript
import { encrypt, decrypt, bruteForce } from "@coding-adventures/scytale-cipher";

const ct = encrypt("HELLO WORLD", 3);
// => "HLWLEOODL R "

const pt = decrypt(ct, 3);
// => "HELLO WORLD"

const results = bruteForce(ct);
// => [{key: 2, text: "..."}, {key: 3, text: "HELLO WORLD"}, ...]
```

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.
