# TXT03 — Native Text Shapers: CoreText, DirectWrite, Pango

## Overview

TXT03 is the **device-dependent** counterpart to TXT01 + TXT02.
Where the device-independent path produces reproducible, byte-
identical output across every platform (at the cost of having
to carry font bytes and implement shaping ourselves), the device-
dependent path delegates to the host OS's text stack and gets
platform-native rendering quality — system fonts, hinting,
subpixel antialiasing, language-tailored shaping — for free.

This spec defines **three parallel implementations** of the TXT00
`FontMetrics` AND `TextShaper` interfaces, one per platform:

| Sub-spec | Platform          | API           | Shaper output "binding" |
|----------|-------------------|---------------|-------------------------|
| TXT03a   | macOS / iOS       | CoreText      | `coretext:<id>`         |
| TXT03b   | Windows           | DirectWrite   | `directwrite:<id>`      |
| TXT03c   | Linux / BSD       | Pango + HarfBuzz + FreeType | `pango:<id>` |

A fourth sibling, **TXT03d — Canvas Text Backend**, lives in its own
spec file (`TXT03d-canvas-text-backend.md`) because the browser runtime
is materially different: Canvas 2D does not expose glyph IDs to JavaScript,
so TXT03d implements `FontMetrics` but deliberately does not implement
`TextShaper` — it emits `PaintText` (see P2D00 amendment) instead of
`PaintGlyphRun`. It is still a device-dependent backend in the same spirit
as a/b/c: the host runtime owns shaping and rasterization; we never parse
fonts ourselves.

Each sub-spec specifies:
- How to map a `FontQuery` (TXT05) into a native font handle
- How the `FontMetrics` methods read from that handle
- How `TextShaper.shape()` calls into the OS shaping API
- The glyph ID namespace produced by shaping (and its invariant:
  only usable via the matching OS drawing call)
- The `font_ref` scheme prefix and key format

The three sub-specs share so much infrastructure that they are
combined into a single spec for easier navigation. When
implementations start to diverge (native variable-font support,
OS-specific shaping features), each sub-spec may graduate to a
standalone document.

---

## Shared design commitments

### Same interface, same contract

Every TXT03 implementation satisfies the **exact same** TXT00
traits as TXT01 + TXT02: `FontMetrics` for global metrics,
`TextShaper` for positioned glyph runs. A layout engine written
against those traits can swap implementations without knowing
which backend is underneath.

### Bundled metrics + shaping

TXT03 implementations deliberately bundle `FontMetrics` and
`TextShaper` for the same font binding. Unlike the device-
independent pair (TXT01 metrics + TXT02 shaper can be swapped
independently because `font-parser` is the single source of
truth), the native APIs return metrics and glyph IDs from the
same internal font handle. Splitting them would force the OS to
parse the font twice, and even then CoreText's `CGGlyph` values
would not match DirectWrite's. The bundling is non-negotiable
for the native path.

Consumers receive two paired objects — a `NativeMetrics` and a
`NativeShaper` — constructed together. Each carries a reference
to the same underlying OS handle.

### The font-binding invariant, reinforced

Every TXT03 shaper produces glyph IDs that are **opaque tokens
bound to the OS font handle that produced them**. This is the
load-bearing rule from TXT00 §"The font-binding invariant":

- A `CGGlyph` value of 42 from CoreText is not the same glyph as
  a `u16` value of 42 from DirectWrite, even if both were
  "resolved" from the same font file on disk. The OSes may use
  different internal glyph tables, apply different synthetic
  glyph generation, and return different IDs.
- A paint backend that receives a `PaintGlyphRun` with
  `font_ref: "coretext:XYZ"` MUST use `CTFontDrawGlyphs` (or
  equivalent) with the matching `CTFontRef`. Feeding the glyph
  IDs to DirectWrite's `DrawGlyphRun` is undefined behaviour and
  MUST throw `UnsupportedFontBindingError`.

The paint backend's glyph_run dispatch (P2D06 amendment) routes
on the scheme prefix to enforce this.

### Not reproducible, but faithful

The output of a TXT03 shaper depends on:
- The OS version (CoreText in macOS 14 differs from macOS 10)
- The installed system fonts (a font named `"Helvetica"` may
  resolve differently on different machines)
- Platform-specific shaping features (CoreText's Arabic handling
  differs from Pango's)

This is the correct tradeoff for a native app. The alternative —
reproducible-everywhere via TXT01+TXT02 — is available in the
same codebase for consumers who need it (LaTeX, server-side
rendering, pixel-exact tests). TXT03 does NOT replace the
device-independent path; it complements it.

---

## TXT03a — CoreText (macOS / iOS)

### The OS handle

```
CoreText FontHandle  ≔  CTFontRef  (Core Foundation opaque pointer)
```

`CTFontRef` is the stateful font object for CoreText — it owns
the font's glyph-rendering machinery and is sized at creation
time. A `CTFontRef` for "Helvetica" at 16pt is a different handle
from one at 24pt; both reference the same underlying font file,
but their cached data differs.

### font_ref scheme

```
coretext:<id>

<id> is a stable identifier that the TXT03a backend uses to look
up the CTFontRef from its internal registry. Proposed format:
  "<ps_name>@<size>" — e.g. "Helvetica-Bold@16.0"

The paint backend's CoreText registry keeps a Dictionary<String,
CTFontRef> mapping <id> to the live handle. Resolvers and
backends share this registry.
```

### Metrics implementation

The `FontMetrics` methods map directly to CoreText functions:

```
impl FontMetrics for CoreTextMetrics {
    fn units_per_em(font) = CTFontGetUnitsPerEm(font)
    fn ascent       (font) = CTFontGetAscent(font) × units_per_em / size
    fn descent      (font) = CTFontGetDescent(font) × units_per_em / size
                             // already positive — CoreText returns unsigned distance
    fn line_gap     (font) = CTFontGetLeading(font) × units_per_em / size
    fn x_height     (font) = CTFontGetXHeight(font) × units_per_em / size
    fn cap_height   (font) = CTFontGetCapHeight(font) × units_per_em / size
    fn family_name  (font) = CFStringGetCStringPtr(CTFontCopyFamilyName(font))
}
```

Scaling note: CoreText's `CTFontGetAscent` and friends return
values in **the font's size at creation**, not design units.
To match TXT00's `FontMetrics` contract (which returns design
units), we multiply by `units_per_em / size`. This is a
no-op for callers that then scale back by `size / units_per_em`
to pixels — the two operations cancel — but the invariant
matters.

### Shaper implementation

```
impl TextShaper for CoreTextShaper {
    fn shape(text, font, size, options):
        // 1. Build a CFAttributedString with the font and
        //    language attributes.
        let attr_string = CFAttributedStringCreate(...)

        // 2. Build a CTLine from it. CoreText runs its full
        //    shaping pipeline here: cmap, kerning, ligatures,
        //    GSUB/GPOS, bidi (if options.direction permits).
        let ct_line = CTLineCreateWithAttributedString(attr_string)

        // 3. Enumerate the line's glyph runs. A single input
        //    string may produce multiple CTRuns if the text
        //    contains a font transition or a script boundary.
        let runs = CTLineGetGlyphRuns(ct_line)

        // 4. For each CTRun, extract:
        //    - CGGlyph[] via CTRunGetGlyphs
        //    - CGPoint[] positions via CTRunGetPositions
        //    - CGSize[]  advances via CTRunGetAdvances
        //    - CFIndex[] string-index mapping via CTRunGetStringIndices
        //
        //    Map these into TXT00's ShapedRun shape:
        //      cluster    = CFIndex (utf-16 index; caller converts)
        //      glyph_id   = CGGlyph as u32
        //      x_advance  = CGSize.width   (already in user-space)
        //      y_advance  = CGSize.height  (usually 0 for horizontal)
        //      x_offset   = CGPoint.x - running_pen   (pen-relative)
        //      y_offset   = CGPoint.y

        return ShapedRun {
            glyphs,
            x_advance_total: sum of all x_advance,
            font_ref: "coretext:<id>",
        }
}
```

### Cluster semantics

CTRun's `CTRunGetStringIndices` returns **UTF-16 code-unit
offsets** into the input string (because CFAttributedString is
UTF-16-internal). TXT02's spec already allows language-idiomatic
cluster units; TXT03a follows that — the cluster is a UTF-16
code-unit offset on every language that hosts TXT03a. Swift and
Objective-C use UTF-16 natively; Rust callers must either work
in UTF-16 strings or convert.

### Script / direction / features

- **Script**: CoreText auto-detects script from the input text.
  Callers can force a script via the `NSLanguage` attribute;
  TXT03a maps `ShapeOptions.script` to this attribute. `null`
  means "let CoreText choose".
- **Direction**: CoreText resolves bidi internally. TXT03a
  currently exposes only `ltr` in `ShapeOptions.direction`. RTL
  support requires passing the bidi level explicitly; deferred.
- **Features**: TXT03a maps OpenType feature tags to CoreText's
  `kCTFontFeatureTypeIdentifierKey` / `kCTFontFeatureSelectorIdentifierKey`
  dictionary, using the published mapping table from Apple's
  Font Feature Registry. Unknown tags are dropped silently.

### Resolver integration

TXT05's CoreText resolver (`font-resolver-coretext`) produces
`CTFontRef` handles. TXT03a takes one of these handles and
derives both a `CoreTextMetrics` and a `CoreTextShaper` sharing
it. The `font_ref` string is computed once at construction time
and returned verbatim from both `shape()` and `font_ref()`.

---

## TXT03b — DirectWrite (Windows)

### The OS handle

```
DirectWrite FontHandle  ≔  (IDWriteFontFace, em_size: f32)
```

DirectWrite splits the "which font" from the "at what size": an
`IDWriteFontFace` identifies the glyph geometry; the em size is
supplied per-draw-call. TXT03b bundles them because the shaper
needs the size at shape time (for subpixel positioning
decisions).

### font_ref scheme

```
directwrite:<id>

<id> format: "<postscript_name>@<em_size>". Example:
  "Segoe UI-Regular@16.0"

Paint backends (paint-vm-direct2d) maintain a registry mapping
<id> to the pre-created (IDWriteFontFace, em_size) pair, which
is what DrawGlyphRun needs at dispatch time.
```

### Metrics implementation

```
impl FontMetrics for DirectWriteMetrics {
    let m = IDWriteFontFace::GetMetrics()
    fn units_per_em(_) = m.designUnitsPerEm
    fn ascent       (_) = m.ascent
    fn descent      (_) = m.descent    // already unsigned
    fn line_gap     (_) = m.lineGap
    fn x_height     (_) = m.xHeight    // may be 0 if font doesn't define
    fn cap_height   (_) = m.capHeight  // may be 0
    fn family_name  (_) = IDWriteFontFamily::GetFamilyNames
                            (localized; picks en-us or first available)
}
```

DirectWrite's metrics are already in design units, matching
TXT00's contract directly. Zero values for `xHeight` and
`capHeight` are returned as `null` (per the TXT00 Option type),
matching the "font may not define" clause.

### Shaper implementation

DirectWrite exposes shaping through `IDWriteTextAnalyzer`, which
mirrors the steps of the HarfBuzz-style pipeline:

```
impl TextShaper for DirectWriteShaper {
    fn shape(text, font, size, options):
        // 1. Analyze script boundaries (handles per run).
        let analyzer = IDWriteTextAnalyzer::GetScriptAnalysis(text)

        // 2. For each script run, call GetGlyphs.
        let glyph_analysis = analyzer.GetGlyphs(
            text, text.len(), font.face, false, false,
            &script_analysis, options.locale_ptr,
            null, null, null, 0, MAX_GLYPH_COUNT)

        // 3. Call GetGlyphPlacements for advances + offsets.
        let placements = analyzer.GetGlyphPlacements(
            text, cluster_map, text_properties, text.len(),
            glyph_indices, glyph_properties, actual_glyph_count,
            font.face, em_size, false, false,
            &script_analysis, options.locale_ptr,
            null, null, 0,
            advances /* out */, offsets /* out */)

        // 4. Assemble ShapedRun:
        //      cluster    = cluster_map[i]  (UTF-16 code-unit offset)
        //      glyph_id   = glyph_indices[i] as u32
        //      x_advance  = advances[i]
        //      x_offset   = offsets[i].advanceOffset
        //      y_offset   = offsets[i].ascenderOffset

        return ShapedRun { glyphs, x_advance_total, font_ref: "directwrite:<id>" }
}
```

### Cluster semantics

DirectWrite's `cluster_map` is **UTF-16 code-unit indexed**, same
as CoreText. On Windows, this is also the native string
representation (wide strings).

### Script / direction / features

- **Script**: DirectWrite auto-detects script. `ShapeOptions.script`
  maps to the `DWRITE_SCRIPT_ANALYSIS` override.
- **Direction**: `DWRITE_READING_DIRECTION` set from
  `ShapeOptions.direction`. RTL and vertical are natively
  supported; TXT03b v1 exposes ltr only.
- **Features**: `IDWriteTypography` accepts OpenType feature
  tags directly via `DWRITE_FONT_FEATURE`. TXT03b maps
  `ShapeOptions.features` directly with no translation layer.

### Resolver integration

TXT05's DirectWrite resolver (`font-resolver-directwrite`)
produces `IDWriteFont` objects; TXT03b derives an
`IDWriteFontFace` from them via `CreateFontFace` and caches it.
Pairing with an em size produces the TXT03b handle.

---

## TXT03c — Pango (Linux / BSD)

### The OS handle

```
Pango FontHandle  ≔  PangoFont
```

Pango sits on top of HarfBuzz (for shaping) and FreeType (for
outlines / rasterization). A `PangoFont` is a configured font
instance; the associated `PangoContext` owns the FontConfig font
map.

### font_ref scheme

```
pango:<id>

<id> format: "<family>:<size>:<weight>:<style>". Example:
  "DejaVu Sans:16:700:normal"

Paint backends (paint-vm-cairo) maintain a
Dictionary<String, PangoFont> mapping.
```

### Metrics implementation

```
impl FontMetrics for PangoMetrics {
    let m = pango_font_get_metrics(font, language=NULL)
    fn units_per_em(_) = pango_font_get_hb_font(font)
                            → hb_face_get_upem(face)
    fn ascent       (_) = pango_font_metrics_get_ascent(m) / PANGO_SCALE
    fn descent      (_) = pango_font_metrics_get_descent(m) / PANGO_SCALE
    fn line_gap     (_) = 0  // Pango does not expose line_gap; HarfBuzz
                             // does via hb_font_get_extents
    fn x_height     (_) = m.x-height from OS/2 table via HarfBuzz
    fn cap_height   (_) = m.cap-height from OS/2 table via HarfBuzz
    fn family_name  (_) = pango_font_description_get_family(desc)
}
```

The `/ PANGO_SCALE` division (PANGO_SCALE = 1024) converts
Pango's fixed-point units to design units. The `line_gap`
workaround — going through HarfBuzz directly — is a quirk of
Pango's API; HarfBuzz exposes more detail than Pango wraps.

### Shaper implementation

Pango shapes via its `pango_itemize` / `pango_shape` pipeline,
which internally calls HarfBuzz:

```
impl TextShaper for PangoShaper {
    fn shape(text, font, size, options):
        // 1. Create a PangoContext with this font's font map.
        let ctx = pango_context_new()
        pango_context_set_font_description(ctx, font.desc)

        // 2. Itemize the text into runs (handles script & direction).
        let items = pango_itemize(ctx, text, 0, text.len(),
                                  NULL /* attrs */, NULL)

        // 3. For each item, call pango_shape_full to get the
        //    PangoGlyphString.
        let glyph_strings = items.map(|item| {
            let gs = pango_glyph_string_new()
            pango_shape_full(item.text, item.length, NULL, 0, item.analysis, gs)
            gs
        })

        // 4. Flatten glyph strings into TXT00 ShapedRun:
        //      cluster    = glyph_info.log_cluster  (byte offset, UTF-8)
        //      glyph_id   = glyph_info.glyph
        //      x_advance  = glyph_info.geometry.width / PANGO_SCALE
        //      x_offset   = glyph_info.geometry.x_offset / PANGO_SCALE
        //      y_offset   = glyph_info.geometry.y_offset / PANGO_SCALE

        return ShapedRun { glyphs, x_advance_total, font_ref: "pango:<id>" }
}
```

### Cluster semantics

Pango's `log_cluster` is a **UTF-8 byte offset**. Native on Linux
(char * strings are UTF-8) and matches TXT02's default
semantics. The Rust and Python ports use byte offsets directly;
TS/JS is an edge case since clusters from Pango would need to be
remapped to UTF-16 offsets if the TS caller passes a UTF-16
string.

### Script / direction / features

- **Script**: Pango auto-detects via `pango_itemize`.
  `ShapeOptions.script` can be forced by setting attributes on
  the PangoAttributeList passed to `pango_itemize`.
- **Direction**: `ShapeOptions.direction` maps to
  `PANGO_DIRECTION_*`. RTL and vertical are natively supported;
  TXT03c v1 exposes ltr + rtl (the easy win over TXT03a/b).
- **Features**: Pango accepts OpenType feature tags via
  `pango_attr_font_features_new`. Direct 1:1 mapping.

### Resolver integration

TXT05's fontconfig resolver (`font-resolver-fontconfig`) produces
`FcPattern` values; TXT03c converts these to `PangoFontDescription`
via `pango_fc_font_description_from_pattern` and then creates a
`PangoFont` through a `PangoContext`.

---

## Testing strategy

TXT03 implementations cannot be tested against committed
reference outputs the way TXT01+TXT02 can (their output depends
on OS version and installed fonts). Instead, tests verify:

1. **Round-trip consistency.** Shape a string, re-encode to
   P2D00 PaintGlyphRun via UI04, pass through the matching paint
   backend, assert no errors.

2. **font_ref scheme correctness.** The `font_ref` on every
   `ShapedRun` starts with the expected scheme prefix for that
   backend (`coretext:`, `directwrite:`, `pango:`).

3. **Font-binding invariant violation.** Attempt to feed a
   TXT03a shaper's output to a DirectWrite paint backend;
   assert `UnsupportedFontBindingError`.

4. **Metric sanity.** For a known system font ("Helvetica" on
   macOS, "Segoe UI" on Windows, "DejaVu Sans" on Linux),
   assert metrics are within reasonable ranges (ascent > 0,
   descent > 0, x-height < cap-height < ascent).

5. **Empty string.** `shape("")` produces a `ShapedRun` with
   `glyphs.is_empty()` and `x_advance_total == 0.0`.

6. **Single-char.** `shape("A")` produces exactly one glyph.

7. **Script auto-detection.** `shape("Hello")` with
   `script: null` succeeds. `shape("مرحبا")` with `script: null`
   succeeds (Arabic auto-detect). `shape("مرحبا")` with
   `script: "Latn"` succeeds (shaper doesn't enforce; OS may do
   its own fallback).

CI runs these tests only on the matching OS. A
non-macOS machine cannot test TXT03a; PR workflows matrix-
dispatch the suite across runners.

### Cross-backend comparison

A shared integration test renders the same PaintScene via the
full CoreText path (TXT03a → paint-vm-metal with `coretext:`
route) and the full font-parser path (TXT02 → paint-vm-metal
with `font-parser:` route) and compares pixel output with a
structural-similarity metric (SSIM > 0.92). The two paths produce
visually similar but not identical output — that's expected.
Identical pixels would imply one of the pipelines isn't actually
using its named backend.

---

## Package layout

```
text-native-shaper-coretext     (Swift, Rust+FFI; macOS/iOS only)
text-native-shaper-directwrite  (C#, Rust+FFI; Windows only)
text-native-shaper-pango        (Rust+FFI; Linux)
```

Each package:
- Depends on that language's `text-interfaces` (TXT00).
- Depends on the matching TXT05 resolver package (so the shaper
  and resolver share handles).
- Links to the platform's text stack (CoreText.framework;
  dwrite.dll; libpango / libharfbuzz / libfreetype).
- Exposes a single constructor that takes a resolved
  FontHandle from the TXT05 resolver and returns the paired
  (metrics, shaper) objects.

Rust is the workhorse implementation for all three; other
languages wrap the Rust crates via cbindgen-generated C ABIs
(same pattern TXT01/TXT02 expected to use for their own Rust
refs). Swift code on macOS may also talk to CoreText directly
without going through Rust, for simpler builds in iOS apps.

---

## Non-goals

**Cross-platform fallback.** TXT03a does not gracefully degrade
on Windows; it requires macOS. Callers that want "run on any
OS" use TXT02 (device-independent) or switch backends based on
`#[cfg(target_os)]`. A wrapping NativeShaper that picks
automatically based on platform is a layout-engine convenience
library, not part of TXT03.

**Font loading from bytes.** TXT03 shapers operate on OS-
registered fonts only. Loading a font from a byte buffer at
runtime (`FontFace.load()` in web parlance) is OS-specific and
deferred to a later revision of TXT05.

**Color font rendering.** The glyph IDs TXT03 produces include
color-layer glyphs; when the paint backend dispatches them, the
native API renders color correctly. But TXT03 itself does not
expose color-layer metadata to the caller.

**Variable-font axis control.** TXT03 exposes the default-axis
instance only in v1. Custom axis values (`wght: 425`) require
TXT05's variable-font extensions, deferred.

**Explicit bidi control.** TXT03a exposes ltr only; TXT03b and
TXT03c support ltr + rtl. Vertical (ttb/btt) is out of scope for
all three in v1.

---

## Relationship to sibling specs

| Spec  | Relationship                                                                                 |
|-------|----------------------------------------------------------------------------------------------|
| TXT00 | Defines the `FontMetrics` and `TextShaper` traits that TXT03 implements.                      |
| TXT01 | Orthogonal sibling (device-independent metrics). Same interface.                             |
| TXT02 | Orthogonal sibling (device-independent shaper). Same interface.                              |
| TXT04 | Orthogonal sibling (device-independent advanced shaper). Same interface.                     |
| TXT05 | Provides the OS font handles. One resolver per TXT03 sub-spec.                               |
| FNT00 | Not used — TXT03 backends do not parse fonts themselves.                                     |
| FNT02 | Not used — TXT03 backends ask the OS for glyph outlines / rasterization internally.         |
| FNT03 | Not used — same as above.                                                                    |
| P2D06 | Paint backends use the font_ref scheme prefix to route glyph_run to the matching native API. |
| UI04  | Layout engines accept a TXT03 shaper-and-metrics pair the same way they accept TXT01+TXT02.  |

### The device-dependent pipeline

```
Author / CSS                  "font-family: Helvetica; font-weight: 700"
    │
    ▼  FontResolver (TXT05a CoreText variant)
CTFontRef (macOS opaque)
    │
    ▼  TXT03a metrics + shaper
ShapedRun { glyphs: [...CGGlyph...], font_ref: "coretext:Helvetica-Bold@16.0" }
    │
    ▼  UI04 (layout-to-paint)
PaintGlyphRun { font_ref: "coretext:...", glyphs, fill }
    │
    ▼  PaintVM (P2D01 dispatch → P2D06 routing)
CTFontDrawGlyphs(CTFontRef, CGGlyph[], CGPoint[], count, CGContextRef)
    │
    ▼
native Metal textures / pixels
```

This path gets system-font quality with zero font-parsing code.
For Markdown rendering on a Mac, this is the fastest route to a
first working pixel. For LaTeX and server-side rendering, TXT02
remains the right choice.

---

## Open questions

- **Font-handle lifecycle.** CoreText `CTFontRef` is reference-
  counted via Core Foundation; DirectWrite uses COM
  `Release()`; Pango uses GObject. The `NativeMetrics` and
  `NativeShaper` types must ensure they outlive their handles.
  Each per-language package documents the lifetime model;
  Rust uses RAII drop impls, Swift uses ARC, C# uses IDisposable
  + SafeHandle.

- **Synchronous vs async.** CoreText text-stack calls may block
  on font-downloading for fonts not yet cached. Synchronous
  APIs are fine for server-side rendering but awkward in a UI
  event loop. A future TXT03 revision MAY add async variants;
  deferred until a UI consumer exists.

- **Whether TXT03 should also support raster glyph caching.** In
  the font-parser path, FNT03 produces a reusable coverage
  bitmap that the paint backend caches. On the native path, the
  OS does its own caching internally. Forcing a consistent
  cache API across both paths would add complexity without
  clear benefit. Left to paint backends to decide per-backend.

- **Whether to split into TXT03a / TXT03b / TXT03c files.** The
  three are combined here for readability; if a sub-spec grows
  (say, TXT03c gains vertical-script support), that sub-spec
  may graduate to its own file. Split is a formatting question,
  not a design one.
