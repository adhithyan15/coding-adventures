# hash-functions

Pure non-cryptographic hash functions implemented from scratch in TypeScript.

This package covers the same educational surface as the Python and Rust
`hash-functions` packages: FNV-1a, DJB2, polynomial rolling hash, MurmurHash3,
and small analysis helpers for avalanche and bucket distribution.

## Usage

```ts
import { fnv1a32, fnv1a64, murmur3_32 } from "@coding-adventures/hash-functions";

fnv1a32("hello");       // 1335831723
fnv1a64("hello");       // 11831194018420276491n
murmur3_32("abc");      // 3017643002
```

## Development

```bash
bash BUILD
```
