# Changelog — font-parser-node (Rust)

## [0.2.0] — 2026-05-19

### Fixed — latent `napi_value` → `napi_ref` storage bug

`FONT_FILE_CTOR` stored the class constructor as a raw `napi_value`
(a scope-local N-API handle valid only inside the current handle
scope).  When `fp.load(buffer)` reached `napi_new_instance` with
that stale handle, the call would return `napi_invalid_arg`
(status 1) and the wrapper would throw
`"load(): failed to create FontFile wrapper"`.

The bug never fired in practice because every existing test path
hit `fp::load(&bytes)` (the pure-Rust parser) first and rejected
with `InvalidMagic` / `BufferTooShort` / etc. — control never
reached `napi_new_instance` with a valid font.  The first consumer
to call `fp.load` with a real TTF/OTF would have hit it.

Fix.  Match the matrix-rust-napi Phase 4 pattern (PR #3551):

* `FONT_FILE_CTOR` (`AtomicUsize` of `napi_value`) →
  `FONT_FILE_CTOR_REF` (`AtomicUsize` of `napi_ref`).
* In `napi_register_module_v1`, after `napi_define_class`, wrap the
  local handle in a persistent `napi_ref` via
  `napi_create_reference(env, ff_class, /* refcount */ 1, &mut ref)`
  and store the ref.
* In `napi_load`, replace the bare `load_ctor()` lookup with a new
  `resolve_ctor(env)` helper that calls
  `napi_get_reference_value(env, ref, &mut value)` to recover a
  scope-bound `napi_value` for the current callback, throwing a
  precise JS error on null / failure.

Verified end-to-end on darwin-arm64 with a real system TTF:

```
$ node -e "
    const fp = require('./font_parser_native.node');
    const ttf = require('fs').readFileSync('/System/Library/Fonts/...ttf');
    const font = fp.load(ttf);
    console.log('fp.load OK; unitsPerEm =', fp.fontMetrics(font).unitsPerEm);
  "
fp.load OK; unitsPerEm = 2048
```

Without the fix the original code threw the bogus
"failed to create FontFile wrapper" error here.

See `lessons.md` "N-API: `napi_value` is a local handle; storing it
across calls requires `napi_ref`" for the underlying lesson.

## [0.1.0] — 2026-04-01

### Added

- Initial release — Node.js N-API addon wrapping the Rust `font-parser` core.

- **`load(buffer: Buffer) → FontFile`** — Parses a font from a Node.js
  Buffer. Returns an opaque JS object wrapping `Box<FontFile>`. Throws an
  Error on parse failure or wrong argument type.

- **`fontMetrics(font) → object`** — Returns an object with camelCase keys:
  `unitsPerEm`, `ascender`, `descender`, `lineGap`, `xHeight` (number | null),
  `capHeight` (number | null), `numGlyphs`, `familyName`, `subfamilyName`.

- **`glyphId(font, codepoint: number) → number | null`** — Maps a Unicode
  codepoint to a glyph ID. Returns `null` if unmapped.

- **`glyphMetrics(font, glyphId: number) → object | null`** — Returns an
  object with `advanceWidth` and `leftSideBearing`. Returns `null` for
  out-of-range glyph IDs.

- **`kerning(font, left: number, right: number) → number`** — Returns the
  kern value for a pair of glyph IDs; 0 when not found.

### Implementation notes

- Uses `napi_wrap` with a GC finalizer to store `Box<FontFile>` in a JS object.
  The finalizer calls `Box::from_raw` to drop Rust memory when GC collects.
- `napi_get_buffer_info` extracts raw bytes from a Node.js Buffer without copy.
- `napi_new_instance` + stored constructor VALUE (`FONT_FILE_CTOR`) creates
  new FontFile JS instances.
- `napi_register_module_v1` entry point — the ABI-stable N-API module
  registration function (replaces the old `NODE_MODULE_INIT` / `napi_module`).
- Targets N-API v4 (Node.js 10.16+) for maximum compatibility.
- `crate-type = ["cdylib"]`, lib name `font_parser_native` — rename to
  `font_parser_native.node` after `cargo build --release`.
