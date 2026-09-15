---
category: Native extensions & FFI
---

# OTP 26 Linux

: `enif_get_int64`/`enif_make_int64` are NOT reliably exported from `beam.smp` — declaring them gives `undefined symbol` at NIF load. Use `enif_get_long`/`enif_make_long` (always exported); on 64-bit POSIX they're equivalent.
