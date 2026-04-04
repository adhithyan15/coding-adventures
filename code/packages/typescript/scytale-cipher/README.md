# Scytale Cipher (TypeScript)

Ancient Spartan transposition cipher implementation in TypeScript.

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
