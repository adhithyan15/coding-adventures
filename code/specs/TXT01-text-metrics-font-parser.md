# TXT01 — text-metrics-font-parser: FontMetrics over font-parser

## Overview

TXT01 is the **first concrete implementation** of the `FontMetrics` interface
defined in TXT00. It is a thin adapter over the existing `font-parser`
package (FNT00), producing numeric font metrics directly from OpenType/
TrueType binary tables with no OS involvement.

```
┌─────────────────────────────────┐
│  Layout / TextShaper consumer   │
└────────────────┬────────────────┘
                 │ calls
                 ▼
┌─────────────────────────────────┐
│  TXT00 FontMetrics trait        │
└────────────────┬────────────────┘
                 │ implemented by
                 ▼
┌─────────────────────────────────┐
│  TXT01 text-metrics-font-parser │
└────────────────┬────────────────┘
                 │ delegates table lookups to
                 ▼
┌─────────────────────────────────┐
│  FNT00 font-parser              │
└─────────────────────────────────┘
```

This is the **device-independent** metrics path. Given the same font
bytes, every machine that runs this code returns bit-identical metric
values — the same property LaTeX needs for reproducible typesetting.

TXT01 is deliberately small. It contains no novel algorithm; every
interesting number already exists in `font-parser`. The job of this
spec (and its implementations) is to expose those numbers through the
TXT00-shaped interface and to bind them to a `font_ref` string that
downstream rasterizers can route on.

---

## Scope

TXT01 implements **only** the `FontMetrics` interface from TXT00. It
does not implement `TextShaper` — that is TXT02's job. A package that
follows this spec exports exactly the methods listed below and nothing
more.

Per-glyph measurements (advance widths, kerning) are **shaper territory**
even though `font-parser` exposes them. TXT01 adapters MUST NOT surface
`glyph_metrics()` or `kerning()` through the FontMetrics trait. If a
consumer wants per-glyph data, it must go through a `TextShaper`.

The rationale is strict separation of concerns: future replacements of
FontMetrics (e.g. a CoreText-backed one) may not be able to answer
per-glyph questions in isolation from the shaper. Keeping the TXT01
surface narrow preserves substitutability.

---

## The FontHandle type for this backend

```
FontHandle  for  text-metrics-font-parser
  ≔  a reference (or pointer, or language-appropriate handle) to a
     parsed font file from font-parser, PLUS its font_ref string.

In Rust:

  pub struct FontParserHandle<'a> {
      pub file: &'a font_parser::FontFile,
      pub font_ref: String,   // "font-parser:<key>", see below
      // Cached metrics — lazily computed on first access, then frozen.
      cached: OnceCell<font_parser::FontMetrics>,
  }
```

Equivalent struct shapes apply in other languages. The crucial
properties are:

- The handle **borrows** the parsed font; it does not own font bytes.
  Ownership lives one layer up (in the font loader / FontResolver).
- The handle carries its `font_ref` string so callers do not need to
  recompute it on every access.
- Cached metrics are allowed and recommended — a parsed
  `font_parser::FontMetrics` is ~40 bytes and immutable for the
  font's lifetime.

### Why not expose `FontFile` directly?

The TXT00 `FontHandle` type is implementation-defined, not universal.
A CoreText-backed FontMetrics cannot use a `FontFile`. Keeping each
backend's handle private (behind a type it also defines) means the
adapter layer is the single point where font-parser and TXT00
vocabulary meet.

Callers hold handles opaquely and pass them back to the same
FontMetrics instance that minted them. That matches the font-binding
invariant laid out in TXT00.

---

## The font_ref scheme

Every handle produced by this backend has a `font_ref` of the form:

```
font-parser:<key>
```

Where `<key>` is a **stable identifier** for the font bytes. Adapters
MUST use one of the following choices, in preference order:

1. **Content hash** — `blake2b(font_bytes)[0..16]` as lowercase hex.
   This is the canonical form. It is reproducible, collision-resistant,
   and makes identical font bytes produce identical font_refs across
   processes.

2. **Caller-supplied id** — if the caller wants a human-readable key
   (`"inter-regular-v3.19"`), they supply it at load time and the
   adapter prefixes `font-parser:` to it. The caller is responsible
   for uniqueness; adapters MUST NOT attempt to detect collisions.

A paint backend that sees `font_ref = "font-parser:abc123..."` MUST
route this glyph run through the `glyph-parser` (FNT02) /
`glyph-rasterizer` (FNT03) pipeline — NOT through CoreText or
DirectWrite. The paint backend MUST reject any `font-parser:`
font_ref it has not been given the matching font bytes for, by
throwing `UnsupportedFontBindingError` (from TXT00).

This routing key is the concrete realization of the **font-binding
invariant** from TXT00 §"The font-binding invariant". It is the
single piece of wire data that keeps the device-independent path
and the device-dependent path from contaminating each other.

### Why blake2b and not sha256

`blake2b` is already the content-hash primitive of record in this
repo (see `HF06-blake2b.md` and the `blake2b` packages across nine
languages). Using it here avoids pulling a second hash implementation
into every TXT01 package. 16 hex bytes (64 bits of entropy) is
sufficient for font content-addressing — far below the birthday
collision threshold for any realistic font set.

---

## Method-by-method implementation

Each method in the TXT00 `FontMetrics` interface maps to a
straightforward call into `font-parser`. Cached metrics are denoted
`m = font_parser::font_metrics(handle.file)` and are computed once
per handle.

### `units_per_em(font)`

```
→  m.units_per_em   (u16 in font-parser, widened to int)
```

Direct passthrough. Guaranteed non-zero for any font that parsed
successfully.

### `ascent(font)`

```
→  m.ascender       (i16 in font-parser)
```

`font_parser::font_metrics` already prefers `OS/2.typoAscender` over
`hhea.ascender` when both are present, which matches TXT00's
expected semantics. The sign is already positive (distance *above*
the baseline).

### `descent(font)`

```
→  abs(m.descender)
```

**Sign conversion required.** `font-parser` returns the signed
`hhea.descender` / `OS/2.typoDescender` value, which is conventionally
negative (e.g., `-512` for Inter Regular). TXT00 specifies that
descent is returned as a non-negative "distance below baseline".
Adapters MUST take the absolute value.

A font with `descender == 0` is valid (no glyphs below the baseline,
like an all-caps display font). Return `0`, not an error.

### `line_gap(font)`

```
→  m.line_gap       (i16 in font-parser; typically 0)
```

Passthrough, unsigned at the TXT00 layer. Most modern fonts have
`line_gap == 0`; extra line spacing is a consumer concern (CSS
`line-height`, TeX `\baselineskip`).

### `x_height(font)`

```
→  m.x_height      (Option<i16> in Rust, null in TS/Python/etc.)
```

`font-parser` already returns `None` when the OS/2 table is absent
or version < 2. Propagate that.

### `cap_height(font)`

```
→  m.cap_height    (Option<i16>)
```

Same handling as `x_height`.

### `family_name(font)`

```
→  m.family_name   (String)
```

Direct passthrough. `font-parser` reads this from `name` table
nameID 1, platform 3 encoding 1 (UTF-16 BE). Callers should treat
this as display-only — do NOT use it for font resolution (TXT05's
job) or for `font_ref` construction.

---

## Package layout

TXT01 has one package per supported language. The naming convention is:

```
text-metrics-font-parser    (TypeScript, Python, Ruby, Go, Perl, Lua,
                             Haskell, Swift, C#, F#, Elixir, Rust)

coding_adventures_text_metrics_font_parser   (for languages that use
                                               snake_case package names)
```

Each package:

- Depends on that language's `font-parser` package.
- Depends on that language's `text-interfaces` package (TXT00).
- Depends on that language's `blake2b` package for the default
  content-hashed `font_ref` path.
- Exposes a single constructor that takes a `FontFile` (or raw bytes
  plus the language's `font-parser` load call) and returns a value
  implementing the TXT00 `FontMetrics` trait.
- Exposes an explicit constructor variant that takes a caller-supplied
  id string to bypass hashing (useful for tests and for cases where
  the caller already has a content-addressed identifier).

### Rust reference layout

```
code/packages/rust/text-metrics-font-parser/
├── Cargo.toml              # depends on font-parser, text-interfaces, blake2b
├── CHANGELOG.md
├── README.md
├── BUILD
├── required_capabilities.json
└── src/
    └── lib.rs
```

Reference API (other languages follow the same shape with idiomatic
naming):

```rust
pub struct FontParserMetrics<'a> {
    file: &'a font_parser::FontFile,
    font_ref: String,
    cached: OnceCell<font_parser::FontMetrics>,
}

impl<'a> FontParserMetrics<'a> {
    /// Construct from an already-parsed FontFile. The font_ref is
    /// derived from the content hash of the original bytes, which
    /// must be supplied (font-parser does not retain them).
    pub fn from_file(file: &'a font_parser::FontFile, bytes: &[u8])
        -> Self { ... }

    /// Construct with a caller-supplied id. The final font_ref is
    /// "font-parser:<id>". The caller owns collision avoidance.
    pub fn with_id(file: &'a font_parser::FontFile, id: impl Into<String>)
        -> Self { ... }
}

impl<'a> text_interfaces::FontMetrics for FontParserMetrics<'a> {
    type Handle = &'a font_parser::FontFile;

    fn units_per_em(&self, _: Self::Handle) -> u32 { ... }
    fn ascent       (&self, _: Self::Handle) -> i32 { ... }
    fn descent      (&self, _: Self::Handle) -> i32 { ... }  // abs()
    fn line_gap     (&self, _: Self::Handle) -> i32 { ... }
    fn x_height     (&self, _: Self::Handle) -> Option<i32> { ... }
    fn cap_height   (&self, _: Self::Handle) -> Option<i32> { ... }
    fn family_name  (&self, _: Self::Handle) -> &str { ... }
}
```

### Cross-language consistency

A given font file's metrics values MUST match across all TXT01
implementations to the bit. This is testable via a shared fixture:
a fixed font file (e.g., Inter Regular, a known-good open-license
font) and a JSON file of expected metric values. Every language's
package runs the same fixture through its adapter and asserts
equality. Divergences indicate a bug in that language's
`font-parser` port, not in TXT01.

---

## Testing strategy

Every TXT01 package MUST include the following tests:

1. **Passthrough correctness.** For a known font (Inter Regular,
   ships as a test fixture), each method returns the documented
   expected value. Fixture values are committed to the repo and
   shared across language packages.

2. **Descent sign conversion.** A font with a negative `hhea.descender`
   produces a positive `descent()` result. A font with
   `descender == 0` produces `0`.

3. **Optional metrics.** A synthetic "OS/2-less" font (old-style
   TrueType with no OS/2 table) produces `None` / `null` from
   `x_height()` and `cap_height()`, and does not throw.

4. **font_ref stability.** Two `FontParserMetrics` instances
   constructed from the same bytes produce the same `font_ref`
   string. A one-byte change in the font bytes produces a
   different `font_ref`.

5. **Caching.** Two consecutive `ascent(h)` calls on the same
   handle do not both call `font_parser::font_metrics`. Observable
   via a mock or via a counter in a test-only wrapper.

6. **Interface conformance.** The adapter satisfies the TXT00
   `FontMetrics` trait's signature exactly — tested via a
   compile-time check in Rust, a structural type check in
   TypeScript, and runtime assertion in dynamic languages.

Coverage target: **95%+**, per the repo-wide standard for library
packages. Since TXT01 is a thin adapter, hitting this is trivial.

### Integration test — layout line height

A cross-language integration test computes the CSS-style line
height for a known font at a known size and asserts it matches
across languages:

```
font            = Inter Regular
size            = 16 (user-space units)
units_per_em    = 2048
ascent          = 1984 design units  → 1984 × 16 / 2048 = 15.5 px
descent         = 494  design units  → 494  × 16 / 2048 ≈ 3.86 px
line_gap        = 0
line_height     ≈ 15.5 + 3.86 + 0    = 19.36 px
```

(Actual Inter values; the test asserts to three decimal places.)

---

## Error conditions

TXT01 is thin enough that it introduces no new error types. It
propagates errors from `font-parser`:

| Source                     | When                                                                 | TXT00-layer mapping             |
|----------------------------|----------------------------------------------------------------------|---------------------------------|
| `FontError::TableNotFound` | The font is missing a required table (`head`, `hhea`, `name`)        | `FontResolutionError` at load time, not observable at the FontMetrics layer |
| `FontError::BadMagic`      | The `head` table's magic number is wrong (not a valid OpenType file) | Same — caught at load time      |
| `FontError::Utf16Decode`   | The `name` table has malformed UTF-16 BE data                        | Fall back to an empty string for `family_name`; do NOT propagate |

Once a `FontFile` has been successfully parsed and a
`FontParserMetrics` is built from it, every method is infallible.
No TXT01 method returns `Result` / throws. This is a deliberate
choice: metrics retrieval on a known-good parsed font is pure
reading of already-validated fields.

---

## Non-goals

TXT01 explicitly does NOT cover:

**Glyph-level metrics.** Per-glyph advance widths, kerning pairs,
and glyph bounding boxes live in `TextShaper` output, not in
`FontMetrics`. Adapters MUST NOT leak `font_parser::glyph_metrics`
or `font_parser::kerning` through this interface.

**Font loading I/O.** Reading font bytes from disk, network, or a
system font directory is `FontResolver`'s job (TXT05). TXT01
assumes a parsed `FontFile` already exists.

**Font file parsing.** That is `font-parser` (FNT00). TXT01 is an
adapter, not a parser.

**Variable-font axis resolution.** OpenType variable fonts expose
multiple instances along design axes (weight, width, optical size).
Selecting a specific instance is FontResolver's job. Once a variable
font has been "instanced" to a concrete set of metrics, TXT01 reads
them like any static font.

**CFF / PostScript-flavored OpenType fonts.** As of this writing,
`font-parser` v0.1.0 only handles TrueType-flavored outlines
(`sfntVersion == 0x00010000`). TXT01 inherits that limitation.
When `font-parser` gains CFF support, TXT01 gets CFF support
automatically — no adapter changes needed.

**Color fonts (COLRv0 / COLRv1 / SVG / sbix).** Color layer data
is a rendering concern owned by glyph-parser / rasterizer. TXT01
is metrics-only.

---

## Relationship to sibling specs

| Spec  | Relationship                                                                     |
|-------|----------------------------------------------------------------------------------|
| TXT00 | Provides the `FontMetrics` trait that TXT01 implements.                          |
| TXT02 | Will consume TXT01 indirectly — naive shaper needs cmap + glyph metrics + kerning from `font-parser` directly, and line-height metrics via TXT01. |
| TXT03 | Orthogonal sibling — device-dependent `FontMetrics` adapters (CoreText, DirectWrite). A caller picks one or the other; they are not composed. |
| TXT05 | Supplies the `FontFile` that TXT01 wraps. FontResolver handles "Helvetica" → bytes; TXT01 takes it from there. |
| FNT00 | Upstream dependency. `font-parser` is the table parser; TXT01 is the trait adapter. |

The device-independent font pipeline, end to end:

```
  user names a font           ─ "Inter Regular"
        │
        ▼
  FontResolver (TXT05)        ─ resolves to bytes on disk, loads them
        │
        ▼
  font-parser (FNT00)         ─ parses tables into a FontFile
        │
        ▼
  TXT01 adapter               ─ wraps FontFile, mints font_ref
        │                         "font-parser:abc123..."
        ▼
  Layout / TextShaper         ─ reads metrics, shapes text
        │
        ▼
  PaintGlyphRun (P2D00)       ─ carries font_ref through
        │
        ▼
  Paint backend               ─ sees "font-parser:abc123...",
                                routes to glyph-parser rasterizer
```

No step in this pipeline calls an OS font API. The same bytes in
produce the same pixels out on macOS, Windows, Linux, iOS, Android,
and WASM.

---

## Open questions

- Whether to expose a "metrics fingerprint" alongside `font_ref` —
  a content hash of the parsed metrics values only (ignoring glyph
  data). Useful for cache invalidation at the layout layer. Deferred
  until a concrete caller needs it.

- Whether adapters should accept `Arc<FontFile>` (or
  language-equivalent reference counting) in addition to borrowed
  references. The current design assumes the caller owns the
  `FontFile` and outlives the adapter. Some consumers (async
  servers) may need shared ownership. Add when a real call site
  asks for it; do not speculatively generalize.

- Whether the `font_ref` key should include font version
  (`head.fontRevision` or the `name` table version string). Two
  versions of the same font have different bytes and therefore
  different content hashes, so the question is cosmetic. Leaving
  it out keeps keys shorter.
