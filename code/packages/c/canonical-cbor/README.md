# canonical-cbor (C)

A deterministic CBOR (RFC 8949) codec, in **pure ISO C17**. A faithful port of
the Rust [`canonical-cbor`](../../rust/canonical-cbor) crate.

## What it does

Encodes and decodes CBOR values in a **canonical** (deterministic) profile so
that `decode(encode(v))` round-trips and `encode(v)` is the same bytes on every
platform — required for AEAD-authenticated records, byte-for-byte sync conflict
detection, and COSE-Key / WebAuthn-PRF.

**Profile** (RFC 8949 §4.2.3, "length-first map key ordering"):

- Definite length only (indefinite-length items are rejected).
- Smallest-form integer encoding (expanded forms rejected by the decoder).
- Map keys sorted length-first, ties broken bytewise on the encoded key.
- No floats (rejected); tags pass through opaquely; no `undefined`.

## API

- **Constructors** — `cbor_unsigned`, `cbor_negative`, `cbor_bool`, `cbor_null`,
  `cbor_bytes`, `cbor_text`, `cbor_array` + `cbor_array_push`, `cbor_map` +
  `cbor_map_push`, `cbor_tag`. Every `CborValue` is heap-allocated and owns its
  children; release a whole tree with one `cbor_free`.
- `cbor_encode` — canonical bytes into a malloc'd buffer (caller frees).
- `cbor_decode` — a `CborStatus` code + an owned value tree.
- `cbor_equal` — deep structural equality.

## Design notes

- **Ownership.** `cbor_array_push` / `cbor_map_push` / `cbor_tag` take ownership
  of what you hand them (and free it on allocation failure), so a partially
  built tree never leaks. The `CborValue` struct is exposed for direct
  inspection of decoded values.
- **Faithful divergences.** Rust `Vec<u8>` → malloc'd buffer + length; Rust
  `Result` → `CborStatus` + out-param (with an extra `CBOR_ERR_ALLOC`).
- **Security-hardened decoder** (matching the Rust crate): recursion depth is
  capped (`CBOR_MAX_DECODE_DEPTH`), declared lengths are bounded by the
  remaining input *before* allocating, and cursor arithmetic is overflow-checked
  — so hostile inputs cannot exhaust the stack or force a giant allocation.
- **Overflow-guarded allocation** throughout (growable arrays cap doubling
  below `SIZE_MAX`).

## Usage

```c
#include "canonical_cbor.h"

CborValue *m = cbor_map();
cbor_map_push(m, cbor_text("count", 5), cbor_unsigned(42));

uint8_t *bytes = NULL;
size_t len = 0;
if (cbor_encode(m, &bytes, &len) == CBOR_OK) {
    /* bytes[0..len) is canonical CBOR */
    free(bytes);
}
cbor_free(m);
```

## Building

```sh
sh BUILD           # POSIX: GCC and/or Clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
