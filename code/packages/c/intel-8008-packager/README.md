# intel-8008-packager (C)

An Intel HEX ROM image encoder/decoder for the Intel 8008, in **pure ISO C17**.
A faithful port of the Rust
[`intel-8008-packager`](../../rust/intel-8008-packager) crate.

## What it does

Converts raw binary machine code into the Intel HEX format used by EPROM
programmers, and parses Intel HEX back to binary for round-trip verification.

Each record is `:LLAAAATTDD...CC` — start code, byte count, 16-bit big-endian
load address, record type (`00` data / `01` EOF), data bytes, and a checksum
(the two's complement of the field byte-sum, so every record byte sums to 0
mod 256).

## API

- `pak_encode_hex(binary, len, origin, &out, &out_len)` — encode to a malloc'd
  Intel HEX string (caller frees). 16 bytes per data record + a trailing EOF.
- `pak_decode_hex(text, &decoded)` — parse to `PakDecoded { origin, binary,
  binary_len }`; release with `pak_decoded_free`.
- `pak_error_message(status)` — a representative static message for any status.

## Design notes

- **Status codes, not `Result`.** Rust's `PackagerError(String)` becomes a
  `PakStatus`; the dynamic Rust messages become one representative static string
  per code, each carrying the same keyword (`checksum`, `overlap`, `EOF`, …).
  Results (`String` / `Vec<u8>`) become malloc'd buffers the caller frees.
- **Strict, hardened decoder.** Rejects a missing `:`, non-hex or odd-length
  bodies, records shorter than their claimed byte count, bad checksums,
  unsupported record types, overlapping/duplicate data records, a missing EOF
  record, over-long lines (> 1024 chars), and any decoded span larger than the
  8008's 16 KB address space — all before or without unbounded allocation.
- **Overflow-guarded** growable buffers and address arithmetic throughout.

## Usage

```c
#include "intel_8008_packager.h"

uint8_t rom[3] = {0x06, 0x00, 0xFF};   /* MVI B,0; HLT */
char *hex = NULL; size_t hex_len = 0;
if (pak_encode_hex(rom, 3, 0, &hex, &hex_len) == PAK_OK) {
    PakDecoded d;
    if (pak_decode_hex(hex, &d) == PAK_OK) { /* d.binary == rom */ }
    pak_decoded_free(&d);
    free(hex);
}
```

## Building

```sh
sh BUILD           # POSIX: GCC and/or Clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
