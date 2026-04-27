# @coding-adventures/argon2i

Pure-TypeScript Argon2i (RFC 9106) — the data-independent variant of the
Argon2 memory-hard password hashing function.

Argon2i picks reference blocks from a deterministic pseudo-random stream
that does *not* depend on the password or any memory contents.  The
resulting memory access pattern is constant across secrets, which
defeats side-channel observers at the cost of making the variant the
easiest for GPUs/ASICs to parallelise.  For password hashing prefer
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
import { argon2i, argon2iHex } from "@coding-adventures/argon2i";

const tag = argon2i(
  new TextEncoder().encode("password"),
  new TextEncoder().encode("some-random-salt"),
  3, 65536, 4, 32,
);
```

## API

| Function | Returns | Description |
|---|---|---|
| `argon2i(...)` | `Uint8Array` | `tagLength` bytes |
| `argon2iHex(...)` | `string` | lowercase hex |

Both share the signature:

```ts
argon2i(
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
cd ../argon2i && npm install
npx vitest run --coverage
```

The suite includes the RFC 9106 §5.2 `argon2i` vector plus a spread of
parameter-edge cases.

## Part of coding-adventures

An educational computing stack built from logic gates up through
interpreters and compilers.  Argon2 sits on top of BLAKE2b (HF06); the
sibling packages `argon2d` and `argon2id` expose the other two variants.
