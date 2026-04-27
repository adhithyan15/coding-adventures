# @coding-adventures/argon2d

Pure-TypeScript Argon2d (RFC 9106) — the data-dependent variant of the
Argon2 memory-hard password hashing function.

Argon2d picks reference blocks using the previous block's contents, which
maximises GPU/ASIC resistance at the cost of leaking a timing
side-channel through memory access patterns.  Use Argon2d in contexts
where side-channel attacks are not in the threat model (e.g.
proof-of-work).  For password hashing prefer
[`argon2id`](../argon2id) (the RFC-recommended default).

Depends on [`@coding-adventures/blake2b`](../blake2b) for the outer
BLAKE2b calls (`H0` and the variable-length `H'`).  The compression
function's inner round is an Argon2-specific modification of BLAKE2b
(with an integer multiplication term added to each `G_B` addition), so
it is inlined in this package rather than imported.

See [`code/specs/KD03-argon2.md`](../../specs/KD03-argon2.md) for the
full algorithm walk-through.

## Usage

```ts
import { argon2d, argon2dHex } from "@coding-adventures/argon2d";

const tag = argon2d(
  new TextEncoder().encode("password"),
  new TextEncoder().encode("some-random-salt"),
  3, 65536, 4, 32,
);
```

## API

| Function | Returns | Description |
|---|---|---|
| `argon2d(...)` | `Uint8Array` | `tagLength` bytes |
| `argon2dHex(...)` | `string` | lowercase hex |

Both share the signature:

```ts
argon2d(
  password: Uint8Array,
  salt: Uint8Array,
  timeCost: number,
  memoryCost: number,
  parallelism: number,
  tagLength: number,
  options?: { key?: Uint8Array; associatedData?: Uint8Array; version?: number },
): Uint8Array
```

## Running the tests

```bash
cd ../blake2b && npm install
cd ../argon2d && npm install
npx vitest run --coverage
```

The suite includes the RFC 9106 §5.1 `argon2d` vector plus a spread of
parameter-edge cases.

## Part of coding-adventures

An educational computing stack built from logic gates up through
interpreters and compilers.  Argon2 sits on top of BLAKE2b (HF06); the
sibling packages `argon2i` and `argon2id` expose the other two variants.
