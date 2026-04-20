# @coding-adventures/argon2id

Pure-TypeScript Argon2id (RFC 9106) — the RFC-recommended memory-hard
password hashing function.

Argon2id is the hybrid variant: it uses the side-channel-resistant
*data-independent* addressing (Argon2i) for the first half of the first
pass, and the GPU/ASIC-resistant *data-dependent* addressing (Argon2d)
for everything after.  Pick this one unless you have a specific reason
to prefer [`argon2d`](../argon2d) or [`argon2i`](../argon2i).

Depends on [`@coding-adventures/blake2b`](../blake2b) for the outer
BLAKE2b calls (`H0` and the variable-length `H'`).  The compression
function's inner round is an Argon2-specific modification of BLAKE2b
(with an integer multiplication term added to each `G_B` addition), so
it is inlined in this package rather than imported.

See [`code/specs/KD03-argon2.md`](../../specs/KD03-argon2.md) for the
full algorithm walk-through.

## Usage

```ts
import { argon2id, argon2idHex } from "@coding-adventures/argon2id";

const tag = argon2id(
  new TextEncoder().encode("password"),
  new TextEncoder().encode("some-random-salt"),
  /*timeCost=*/    3,
  /*memoryCost=*/  65536, // 64 MiB
  /*parallelism=*/ 4,
  /*tagLength=*/   32,
);
```

With optional secret key (`K`) and associated data (`X`):

```ts
const tag = argon2id(
  pw, salt, 3, 65536, 4, 32,
  { key: serverSecret, associatedData: contextTag },
);
```

## API

| Function | Returns | Description |
|---|---|---|
| `argon2id(...)` | `Uint8Array` | `tagLength` bytes |
| `argon2idHex(...)` | `string` | lowercase hex |

Both share the signature:

```ts
argon2id(
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
cd ../argon2id && npm install
npx vitest run --coverage
```

The suite includes the RFC 9106 §5.3 `argon2id` vector plus a spread
of parameter-edge cases.

## Part of coding-adventures

An educational computing stack built from logic gates up through
interpreters and compilers.  Argon2 sits on top of BLAKE2b (HF06); the
sibling packages `argon2d` and `argon2i` expose the other two variants.
