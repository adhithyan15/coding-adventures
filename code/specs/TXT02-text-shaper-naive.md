# TXT02 — text-shaper-naive: Minimal Device-Independent TextShaper

## Overview

TXT02 is the **first concrete implementation** of the `TextShaper`
interface defined in TXT00. It is the simplest possible shaper that
produces correct output for basic Latin text: one codepoint maps to
one glyph via the font's `cmap`, advance widths come straight from
`hmtx`, and kerning comes from the legacy `kern` table when present.

There is no GSUB. There is no GPOS. No ligatures. No contextual forms.
No mark positioning. No complex scripts. The shaper's job in this tier
is purely mechanical: **translate codepoints to glyph IDs and stamp
advances on each one**.

```
Input:   "Hello"  (5 codepoints)
Output:  ShapedRun with 5 glyphs, each glyph_id from cmap,
         x_advance from hmtx, adjacent pairs kerned via `kern` table.
```

The point of TXT02 is twofold:

1. **Unblock the pipeline.** With TXT00 + TXT01 + TXT02, the
   device-independent stack is end-to-end complete for basic Markdown
   rendering: layout can measure, shape, and emit `PaintGlyphRun`
   instructions that downstream rasterizers understand.

2. **Establish the test bed for TXT04.** When the hand-rolled
   HarfBuzz-equivalent shaper (TXT04) lands, its output on basic Latin
   must be a superset of TXT02's — same glyph IDs, same advances
   (before GPOS), same clusters. TXT02 is the correctness floor that
   TXT04 later extends.

TXT02 is explicitly a **stepping stone**, not a destination. It will
be superseded by TXT04 for anything that needs real typography.
CommonMark rendering of plain prose is in-scope for TXT02; typesetting
a book jacket is not.

---

## Scope and explicit non-goals

### What TXT02 handles

- Basic Latin (ISO-15924 script `"Latn"`).
- Left-to-right horizontal text (direction `"ltr"`).
- Codepoints in the font's `cmap` Format 4 subtable (the BMP, U+0000
  through U+FFFF).
- The legacy `kern` Format 0 table when present in the font.
- The `.notdef` glyph (glyph_id 0) for codepoints the font cannot map.

### What TXT02 refuses

The shaper MUST throw `UnsupportedScriptError` when asked for:
- Any script other than `"Latn"` or `null` (null means auto-detect;
  TXT02 always auto-detects to `"Latn"` if every character is in the
  BMP Latin ranges, otherwise errors).
- Any direction other than `"ltr"` (RTL, TTB, BTT all refused).

The refusal is deliberate. A silent fallback ("we'll pretend Arabic is
Latin") would produce visually wrong but structurally valid output,
which is the worst failure mode. A loud refusal makes the consumer pick
a real shaper.

### What TXT02 silently ignores

- `ShapeOptions.language` — no language-specific behavior since there
  is no GSUB/GPOS. Pass it in; TXT02 ignores it.
- `ShapeOptions.features` — all feature toggles are silently dropped,
  except for `{ "kern": false }` which disables the kern pass.
  Consumers that depend on other OpenType features must use a real
  shaper.

### What TXT02 does NOT handle

- **GSUB substitutions.** No ligatures ("fi" stays two glyphs), no
  contextual forms, no stylistic sets.
- **GPOS positioning.** No mark positioning, no cursive attachment,
  no advanced kerning beyond legacy `kern` format 0.
- **Complex scripts.** Arabic, Indic, Thai, Hebrew, CJK vertical — all
  rejected.
- **Bidi resolution.** The layout engine must split mixed-direction
  runs before calling this shaper.
- **Unicode normalization.** Input is assumed to already be in NFC.
  TXT02 does NOT re-normalize.
- **Surrogate pairs / supplementary plane.** `cmap` Format 4 only
  covers the BMP. Codepoints above U+FFFF resolve to `.notdef` until
  TXT02 gains Format 12 support (future minor revision).
- **Font fallback.** If the font lacks a glyph, `.notdef` is emitted.
  Cross-font fallback is a layout-engine concern.

---

## The FontHandle type

TXT02 reuses the same `FontHandle` shape as TXT01 (see TXT01 §"The
FontHandle type for this backend"). Concretely:

```
TXT02 FontHandle  ≔  a reference to a font_parser::FontFile plus a
                     font_ref string.

In Rust, TXT02's shaper is constructible either:
  - from a TXT01 FontParserMetrics handle (shares the FontFile ref
    and the font_ref string — the typical path)
  - directly from a &FontFile plus bytes or a caller-supplied id
    (independent of whether the caller needs metrics)
```

A TXT02 shaper MUST produce `font_ref` values with the
`"font-parser:"` scheme prefix, identical to TXT01's. This is what
makes TXT01 and TXT02 **coherent partners** — a producer using both
sees a single font binding, and downstream paint backends route
consistently.

The contract:

> When a consumer pairs a TXT01 `FontParserMetrics` with a TXT02
> `NaiveShaper` over the same `FontFile`, both MUST agree on
> `font_ref(handle)`. The shared string is the paint backend's routing
> key.

Adapters are encouraged to expose a single constructor that builds
both TXT01 and TXT02 at once, sharing state. The language-specific
packages describe how.

---

## Shaping algorithm

The shaper walks the input text once, codepoint by codepoint, and
emits one glyph per codepoint. Then it walks the glyph list and
adjusts advances using the `kern` table if kerning is enabled.

### Phase 1 — cmap lookup

```
for each codepoint c at byte offset b in text:
    gid = font_parser::glyph_id(font, c)
          .unwrap_or(0)   // 0 = .notdef
    gm  = font_parser::glyph_metrics(font, gid)
          .unwrap_or(.notdef_metrics)
    emit Glyph {
        glyph_id:  gid,
        cluster:   b,
        x_advance: gm.advance_width × size / units_per_em,
        y_advance: 0,
        x_offset:  0,
        y_offset:  0,
    }
```

Key points:

- `cluster` is the **UTF-8 byte offset** of the codepoint in the
  original string. For a codepoint at byte offset 3 that occupies
  two bytes, the next codepoint's cluster is 5.
- `.notdef_metrics` is a synthesized fallback: `advance_width =
  units_per_em / 2` (half an em, the conventional fallback width).
- All per-glyph values are already scaled to user-space units at the
  requested `size` — consumers do NOT re-scale by `units_per_em`.

### Phase 2 — kerning pass

If `ShapeOptions.features` does not contain `{ "kern": false }` AND
the font has a `kern` table with a Format 0 subtable, the shaper
walks adjacent glyph pairs:

```
for i in 0 .. glyphs.len() - 1:
    left  = glyphs[i].glyph_id
    right = glyphs[i+1].glyph_id
    k = font_parser::kerning(font, left, right)   // in design units
    if k != 0:
        glyphs[i].x_advance += k × size / units_per_em
```

The kerning value is **added to the left glyph's advance**, not
applied as an offset on the right glyph. This matches the
interpretation used by CSS Canvas, SVG, and most rendering engines.

If the font has no `kern` table, or only has format ≥ 1, or the
consumer disables kerning via features, this phase is skipped and
advances stay as emitted from Phase 1.

### Phase 3 — totalize

```
x_advance_total = sum of x_advance across all glyphs
```

This is pre-computed and stored on the `ShapedRun` so consumers
don't re-sum (TXT00 requires it).

---

## cluster field semantics

The `cluster` field on each glyph is a **byte offset into the input
text string** (UTF-8 encoding assumed). This definition is chosen
to match HarfBuzz's default and is the convention TXT04 will inherit.

- For a codepoint that maps to one glyph, `cluster = byte_offset`.
- For a codepoint that maps to zero glyphs (outside the font), a
  single `.notdef` glyph is emitted with `cluster = byte_offset`.
- TXT02 never produces **multiple glyphs from one codepoint**
  (no decomposition, no mark sequences). Every output glyph's
  cluster is the starting byte of the single codepoint it
  represents.

A consumer doing cursor positioning or hit-testing walks the glyphs
from left to right accumulating advances until the x-coordinate
matches the hit position, then reads `cluster` to get the byte
offset into the source text. This is unambiguous in TXT02's 1:1
mapping.

### Languages without byte strings

In TypeScript/JavaScript, strings are UTF-16 internally. TXT02
implementations in those languages SHOULD report cluster as the
**UTF-16 code-unit offset**, not a byte offset. This is the
language-idiomatic choice and matches what `string.charAt(cluster)`
expects. The TXT00 field contract specifies "byte offset into the
original text string" — adapters in UTF-16 languages interpret
"byte" as "code unit of the language's native string encoding".

This divergence is acceptable because:
- Cluster values are only meaningful to the caller that produced
  the text string.
- A TypeScript caller never compares clusters with a Rust caller.
- The cluster's only purpose is index-into-string, and the
  language's native indexing unit is the right choice.

Adapters MUST document this clearly in their READMEs.

---

## ShapeOptions handling

```
ShapeOptions field    TXT02 behavior
──────────────────────────────────────────────────────────────────────
script                Accepted only: "Latn", null.
                      Any other → UnsupportedScriptError.
                      null → auto-detect: if every codepoint is in
                      U+0000..U+024F (Basic + Latin Extended A/B) or
                      common punctuation/whitespace, treat as "Latn".
                      Otherwise error.

language              Silently ignored.

direction             Accepted only: "ltr".
                      Any other → UnsupportedScriptError.

features              Silently ignored, with ONE exception:
                      { "kern": false } disables the Phase 2 kerning
                      pass. All other feature tags are dropped.
```

The auto-detection for `script: null` is intentionally conservative.
A string containing a single codepoint outside the allowed range
triggers an error, forcing the caller to either (a) provide a real
script code that TXT02 doesn't support and get a clean error, or
(b) switch to a real shaper. Both outcomes are better than silently
shaping Cyrillic text as Latin.

---

## Error conditions

| Error                      | When                                                                                  |
|----------------------------|---------------------------------------------------------------------------------------|
| `UnsupportedScriptError`   | Script is not `"Latn"` or `null`, or auto-detect finds non-Latin characters           |
| `UnsupportedScriptError`   | Direction is not `"ltr"` (same error type reused for simplicity; message distinguishes) |
| `ShapingFailedError`       | Font has no `cmap` Format 4 subtable AND no Format 0 subtable (malformed font)        |

Missing `kern` table is NOT an error. The shaper just skips Phase 2.

Missing glyphs (codepoint not in `cmap`) are NOT an error. The shaper
emits `.notdef` (glyph_id 0) with the synthesized fallback advance.
This matches the TXT00 contract.

Missing per-glyph metrics (glyph_id out of range) are NOT an error.
The shaper substitutes `.notdef_metrics`. This should never actually
happen for well-formed fonts but is handled defensively.

---

## Package layout

One package per supported language, mirroring TXT01:

```
text-shaper-naive    (TS, Python, Ruby, Go, Perl, Lua, Haskell,
                      Swift, C#, F#, Elixir, Rust)
```

Each package:

- Depends on that language's `font-parser`, `text-interfaces` (TXT00),
  and optionally `text-metrics-font-parser` (TXT01) — the last only if
  exposing the convenience co-constructor.
- Exposes a constructor that takes either a `FontFile`+bytes or a
  TXT01 `FontParserMetrics` handle.
- Exposes a single `shape(text, handle, size, options) → ShapedRun`
  method matching the TXT00 trait.
- Exposes `font_ref(handle) → string` returning the same string as
  the paired TXT01 instance.

### Rust reference signature

```rust
pub struct NaiveShaper<'a> {
    file: &'a font_parser::FontFile,
    font_ref: String,
    // Cache of (codepoint → glyph_id) to avoid redundant cmap walks
    // for strings with repeated characters. Optional; implementations
    // MAY skip this if the font_parser::glyph_id call is already fast.
    cmap_cache: RefCell<HashMap<u32, u16>>,
}

impl<'a> NaiveShaper<'a> {
    pub fn from_file(file: &'a font_parser::FontFile, bytes: &[u8]) -> Self;
    pub fn with_id(file: &'a font_parser::FontFile, id: impl Into<String>) -> Self;
    pub fn from_metrics(metrics: &TXT01::FontParserMetrics<'a>) -> Self;
}

impl<'a> text_interfaces::TextShaper for NaiveShaper<'a> {
    type Handle = &'a font_parser::FontFile;
    fn shape(&self,
             text: &str,
             handle: Self::Handle,
             size: f32,
             options: &ShapeOptions)
             -> Result<ShapedRun, ShapingError>;
    fn font_ref(&self, handle: Self::Handle) -> &str;
}
```

Other languages follow the same shape with idiomatic naming.

---

## Testing strategy

Every TXT02 package MUST include the following tests:

1. **Single-character shaping.** Input `"A"` produces one glyph with
   the expected glyph_id, correct advance from `hmtx`, cluster=0.

2. **Multi-character advance summation.** Input `"Hello"` produces
   five glyphs. The sum of `x_advance` equals `x_advance_total`.

3. **cmap miss.** A character not in the font (e.g., U+2603 ☃ in a
   Latin-only font) produces a `.notdef` glyph (glyph_id 0).

4. **Kerning presence.** Input `"AV"` in Inter (which has kern data
   for this pair) produces a smaller `x_advance_total` than `"AV"` in
   the same font with `{ "kern": false }` in features. Assert strict
   inequality.

5. **Kerning absence.** A font without a `kern` table produces the
   same output whether kerning is enabled or disabled.

6. **Script refusal.** `shape("مرحبا", font, 16, { script: "Arab" })`
   throws `UnsupportedScriptError`. `shape("مرحبا", font, 16, { script:
   null })` also throws (auto-detect refuses non-Latin).

7. **Direction refusal.** `direction: "rtl"` throws
   `UnsupportedScriptError`.

8. **Cluster offsets.** Input `"a é b"` (with `é` = U+00E9, 2 bytes
   in UTF-8) produces glyphs with clusters `[0, 1, 2, 4, 5]` in
   byte-indexed languages, or `[0, 1, 2, 3, 4]` in UTF-16-indexed
   languages.

9. **font_ref consistency.** A TXT02 shaper built from a TXT01
   metrics handle returns the same `font_ref` as the metrics handle.

10. **Scale correctness.** `shape("A", font, 32, opts).glyphs[0].
    x_advance` equals `2 × shape("A", font, 16, opts).glyphs[0].
    x_advance` (advance scales linearly with size).

Coverage target: **90%+**. The shaper is small enough that this is
easy to hit.

### Cross-shaper correctness test (future)

When TXT04 (hand-rolled HarfBuzz-like) lands, a shared correctness
suite MUST assert that TXT02 and TXT04 produce identical output on
basic Latin input without any features enabled. This is the
regression lock that prevents TXT04 from accidentally diverging on
the simple case while improving the complex cases.

---

## Non-goals

TXT02 explicitly does NOT cover:

**OpenType shaping features.** GSUB (ligatures, contextual forms,
stylistic sets) and GPOS (mark positioning, cursive attachment,
advanced kerning) are deferred to TXT04.

**Complex scripts.** Any script requiring reordering, cluster merging,
or mark positioning is out of scope: Arabic, Indic scripts
(Devanagari, Bengali, Tamil, etc.), Thai, Hebrew, Mongolian vertical,
CJK vertical.

**Font fallback.** If the font lacks a glyph, TXT02 emits `.notdef`.
A wrapping shaper (not specified here) can implement "try the primary
font, fall back to a secondary font" on top of TXT02.

**Normalization.** Input must be NFC-normalized before calling shape.
Repeated normalization in the shaper would be wasted work in 99% of
cases; pushing it to the caller (or to the layout engine) is correct.

**Variable font instancing.** A variable font must be "frozen" to a
specific instance before being passed to the shaper. Axis
interpolation is not TXT02's job.

**Rasterization.** Turning glyph IDs into pixels is the paint
backend's job (via FNT02 + FNT03). TXT02 stops at glyph IDs and
positions.

---

## Relationship to sibling specs

| Spec  | Relationship                                                                           |
|-------|----------------------------------------------------------------------------------------|
| TXT00 | Provides the `TextShaper` trait that TXT02 implements.                                 |
| TXT01 | **Coherent partner.** TXT01 + TXT02 over the same `FontFile` form a complete device-independent metrics + shaping stack. Share `font_ref`. |
| TXT03 | Orthogonal sibling. Device-dependent shapers (CoreText, DirectWrite, Pango). A caller picks TXT02 OR a TXT03 shaper, not both. |
| TXT04 | Supersedes TXT02 for advanced typography. On basic Latin, TXT04's output MUST match TXT02's (regression lock). |
| TXT05 | Supplies the `FontFile`. FontResolver's job.                                           |
| FNT00 | Upstream dependency — `font-parser` for cmap, hmtx, kern.                              |
| FNT02 | **Downstream pairing.** The paint backend that receives TXT02-produced `PaintGlyphRun`s uses glyph-parser (FNT02) to look up outlines. The `font-parser:` font_ref prefix is how the paint backend knows to use FNT02's rasterizer. |

### The complete device-independent pipeline as of TXT02

```
CommonMark parser
    │
    ▼
document-ast
    │
    ▼
document-ast-to-layout         ─ positional layout
    │ (calls)
    ▼
 ┌──────────────────────┐
 │  TXT01 FontMetrics   │       ─ for line-height / ascent / descent
 │  TXT02 TextShaper    │       ─ for per-string glyph runs
 └──────────────────────┘
    │
    ▼
PositionedTree → PaintScene    ─ emits PaintGlyphRun (P2D00)
    │
    ▼
PaintVM (P2D01)                ─ dispatches "glyph_run"
    │
    ▼  font_ref = "font-parser:..."
FNT02 (glyph-parser)           ─ looks up glyph outline
    │
    ▼
FNT03 (rasterizer)             ─ rasterizes outline
    │
    ▼
pixels
```

This pipeline works on any platform — macOS, Linux, Windows, iOS,
Android, WASM — with no OS font calls. Same bytes in, same pixels out.
The LaTeX use case depends on this property. The CommonMark use case
does not require it but benefits from the deterministic testability.

For the native-platform quality path (CoreText on macOS, DirectWrite
on Windows), the layout engine substitutes a TXT03 shaper in place of
TXT02. Everything else in the pipeline stays the same.

---

## Open questions

- Whether to honor the `{ "liga": true }` feature as a hint to emit
  a warning ("this shaper doesn't support ligatures; use TXT04 if
  you need them"). The current design silently ignores it. A warning
  might be helpful during TXT04 migration; noisy thereafter.

- Whether the `.notdef` advance fallback (half an em) should be
  configurable. The current choice matches conventional typesetting
  practice but makes `.notdef` glyphs visually conspicuous — some
  consumers may want them to be zero-width. Deferred until a real
  caller complains.

- Whether to support `cmap` Format 12 (supplementary plane, emoji
  range) in v0.1.0 or defer to a minor revision. Deferring keeps the
  initial implementation small; adding it later is backward-compatible.
  Current recommendation: defer.
