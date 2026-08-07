# protobuf (C)

A **zero-dependency Protocol Buffers wire-format codec** in pure ISO C17. A
faithful port of the Rust [`protobuf`](../../rust/protobuf) crate. It implements
just the [wire format](https://protobuf.dev/programming-guides/encoding/) —
enough to encode and decode messages byte-for-byte compatibly with Google's
protobuf and any conforming implementation (e.g. the one Anki uses inside
`.apkg` files). There is **no** `.proto` compiler and no codegen: you hand-write
the handful of encode/decode calls for the messages you need.

## The wire format

A message is a flat sequence of `(tag, value)` records — no framing, no length
prefix, no field ordering. Each tag is a varint: `tag = (field_number << 3) |
wire_type`.

| wire type | name | payload |
|-----------|------|---------|
| 0 | Varint          | one LEB128 varint (ints, bools, enums) |
| 1 | Fixed64         | 8 little-endian bytes |
| 2 | LengthDelimited | a varint length `n`, then `n` bytes (string/bytes/message) |
| 5 | Fixed32         | 4 little-endian bytes |

A **varint** stores 7 bits per byte, little-endian, top bit = "more follow":
`300 → [0xAC, 0x02]`. A `u64` needs at most 10 bytes; an 11th means overflow.

## API

```c
#include "protobuf.h"

/* Encode: field 1 = varint 150, field 2 = string "hi" */
PbWriter w;
pb_writer_init(&w);
pb_varint(&w, 1, 150);
pb_string(&w, 2, "hi");
/* bytes are pb_writer_bytes(&w) / pb_writer_len(&w), or take ownership: */
size_t n;
uint8_t *msg = pb_writer_take(&w, &n);   /* caller frees */
pb_writer_free(&w);

/* Decode */
PbReader r;
PbField f;
int has;
pb_reader_init(&r, msg, n);
while (pb_reader_next_field(&r, &f, &has) == PB_OK && has) {
    if (f.number == 1) { /* f.value.varint */ }
}
free(msg);
```

- **Writer** — `pb_writer_init`/`_free`, `pb_write_varint`, `pb_varint`,
  `pb_bytes`, `pb_string`, `pb_message`, `pb_fixed32`, `pb_fixed64`;
  `pb_writer_bytes`/`_len` to borrow, `pb_writer_take` for `into_bytes`
  ownership transfer. The buffer grows with a `size_t`-overflow-guarded doubling
  and latches an `oom` flag on allocation failure.
- **Reader** — `pb_reader_init`, `pb_reader_is_empty`, `pb_reader_next_field`
  (yields unknown field numbers too, for forward compatibility) returning a
  `PbError`. Length-delimited payloads (`PbValue.bytes`) **borrow** the input.
- **Errors** — `PbError` (`PB_ERR_TRUNCATED_VARINT`, `PB_ERR_UNEXPECTED_EOF`,
  `PB_ERR_UNKNOWN_WIRE_TYPE`, `PB_ERR_ZERO_FIELD_NUMBER`) + `pb_error_message`.

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
