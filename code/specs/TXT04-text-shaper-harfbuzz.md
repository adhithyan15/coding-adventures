# TXT04 — text-shaper-harfbuzz: Hand-rolled Full OpenType Shaper

## Overview

TXT04 is the **device-independent, full-featured** text shaper
for the coding-adventures stack. It implements the full OpenType
shaping pipeline — cmap, GSUB (substitutions), GPOS (positioning),
complex-script handling — from first principles, parsing OpenType
tables directly. It is the hand-rolled equivalent of HarfBuzz.

Unlike TXT02 (the naive 1-codepoint-to-1-glyph shaper) and TXT03
(the native-OS shapers), TXT04 aims to be:

- **Reproducible.** Same font bytes + same input = same output,
  every time, on every machine. Property inherited from its
  TXT01 font-parser foundation.
- **Complete.** Supports ligatures, contextual alternates,
  stylistic sets, kerning pairs, mark positioning, cursive
  attachment, and enough complex-script machinery for Latin,
  Cyrillic, Greek, Arabic, Hebrew, Devanagari, and Thai. The
  exact feature set grows tier by tier.
- **A learning project.** Building a full shaper from first
  principles is a substantial undertaking. TXT04 is authored
  incrementally over a long timeline; each tier is a publishable
  milestone.

This spec is deliberately a **scaffold**: it defines the
interface (which is already fixed by TXT00), enumerates the
tiers of functionality, and calls out the implementation-time
decision points — but does NOT prescribe the full algorithm in
detail the way FNT02 does for glyph parsing. The algorithm is
the work; it will evolve as each tier is implemented.

---

## Relationship to TXT02

TXT04 **supersedes TXT02 for consumers that need advanced
typography**. On basic Latin text with no features enabled, TXT04
MUST produce **identical output to TXT02** — same glyph IDs, same
advances (before GPOS), same clusters. This is the regression
lock noted in TXT02's spec: TXT04 is TXT02's strict superset on
the easy case, and adds capability on the hard cases.

Callers swap TXT02 for TXT04 via dependency injection — they
conform to the same `TextShaper` trait. A layout engine that
only needs basic Latin rendering can stick with TXT02 for speed
and simplicity; a consumer rendering CJK or Arabic switches to
TXT04.

TXT02 is NOT deprecated when TXT04 lands. It remains the right
choice for:
- CI / test code that wants fast startup and tiny binary size
- Consumers that are certain their input is basic Latin
- Bootstrap / build-time text rendering (logos, splash screens)

TXT04 is the right choice for:
- CommonMark rendering where users may paste accented or non-Latin
  content
- LaTeX rendering (needs full ligature control and math fonts)
- Anything that claims "proper" typography

---

## Implementation tiers

Each tier is a separate implementation milestone. Ship a tier
in its own PR; do not attempt to land tiers 1–6 all at once.

### Tier 1 — Latin with GSUB ligatures

Minimum viable upgrade over TXT02:

- cmap lookup (Format 4 already in font-parser; TXT04 also adds
  Format 12 for supplementary-plane codepoints like emoji)
- GSUB lookup types 1 (single substitution) and 4 (ligature
  substitution). Covers "fi" → fi ligature, "ffi" → ffi, "ct" in
  display fonts.
- Standard Latin OpenType features: `liga` (standard ligatures,
  on by default), `kern` (legacy — route through GPOS later).
- Legacy `kern` table (format 0) as a fallback when GPOS is
  absent.

Deliverable: `shape("effective")` produces 7 glyphs where the
"ff" and "fi" become ligature glyphs.

### Tier 2 — GPOS positioning

- GPOS lookup types 1 (single adjustment), 2 (pair adjustment —
  proper kerning), 4 (mark-to-base attachment), 5 (mark-to-
  ligature), 6 (mark-to-mark).
- Replace the legacy `kern` fallback from Tier 1 with proper
  GPOS pair-adjust when the font provides both (prefer GPOS).

Deliverable: `shape("AV")` produces kerned output matching
HarfBuzz to within 1 design unit. Mark positioning for accented
Latin (`á`, `ñ`) places the mark at the correct offset.

### Tier 3 — Stylistic sets and alternates

- GSUB lookup types 2 (multiple substitution), 3 (alternate
  substitution), 5 (contextual substitution), 6 (chained
  contextual).
- `ShapeOptions.features` fully honored for tags like `ss01`,
  `calt`, `salt`.

Deliverable: Inter's `ss01` (single-storey 'a') and similar
stylistic sets work correctly when requested.

### Tier 4 — Complex scripts: Arabic, Hebrew, N'Ko

- Cursive connection via GSUB contextual substitutions (initial
  / medial / final / isolated forms).
- Right-to-left direction handling in `ShapeOptions.direction =
  "rtl"`.
- Arabic shaping state machine with the OpenType Arabic shaping
  model's joining classes.
- Bidi resolution remains the layout engine's job (UAX #9);
  TXT04 shapes one direction run at a time.

Deliverable: `shape("مرحبا")` produces correctly-joined Arabic
with matching HarfBuzz output byte-for-byte (given the same
font).

### Tier 5 — Indic scripts

- Devanagari, Bengali, Tamil, Telugu, Kannada, Gujarati.
- Reordering of matras (vowel marks) around the base consonant.
- Conjunct formation (half-forms, nukta handling).
- Unicode-standard normalization before shaping (explicitly).

Indic shaping is notoriously complex — it's one of the main
reasons HarfBuzz exists. Expect Tier 5 to take longer than
Tier 4 in calendar time.

Deliverable: Hindi paragraph rendering matches HarfBuzz. Tamil
renders correctly with all conjunct rules.

### Tier 6 — CJK and Southeast Asian scripts

- Chinese / Japanese / Korean: mostly 1:1 codepoint-to-glyph
  with occasional ligatures. Variable-font support for
  Japanese (`opsz`, `wght`) matters here.
- Thai / Lao: no spaces between words, upper/lower vowel marks.
- Tibetan, Khmer, Myanmar: stackable subjoined consonants.

At this tier, TXT04 is feature-complete for 95% of real-world
content. The remaining 5% (Mongolian traditional, Balinese,
Sundanese script) can follow if interest exists.

---

## Interface (inherited from TXT00)

Because TXT04 implements the same `TextShaper` trait as TXT02,
no new interface is specified here. See TXT00 §"Interface 2:
TextShaper" for the full definition.

TXT04 extends TXT02's behavior within the same signature:

| Feature                  | TXT02 behavior                       | TXT04 behavior                              |
|--------------------------|--------------------------------------|---------------------------------------------|
| ShapeOptions.script      | Latn / null only; throws otherwise   | All supported scripts (tier-dependent)     |
| ShapeOptions.direction   | ltr only                             | ltr, rtl (from tier 4)                     |
| ShapeOptions.features    | Silently ignored (except `kern`)     | Honored per OpenType feature registry      |
| GSUB substitution        | None                                 | Yes (tier-dependent which lookup types)    |
| GPOS positioning         | None (legacy `kern` only)            | Yes (from tier 2)                          |
| Mark positioning         | None                                 | Yes (from tier 2)                          |
| Multi-glyph clusters     | Never                                | Yes (ligatures → one cluster, many glyphs; decomposition → one cluster, many glyphs) |
| font_ref                 | `font-parser:<key>`                  | `font-parser:<key>` (same scheme — device-independent) |

Critically, TXT02 and TXT04 share the **same font binding**
(`font-parser:`). A consumer that held a `font-parser:<hash>`
font_ref from TXT02 can upgrade to TXT04 without re-resolving
the font — same bytes, same parsed tables, same glyph IDs
(by cmap; GSUB adds new glyphs from existing tables).

---

## Dependencies

TXT04 depends on:

- **font-parser (FNT00)** — for the font's header and table
  directory; specifically, TXT04 reads the following tables that
  FNT00 does not fully expose:
  - `GSUB` (glyph substitution)
  - `GPOS` (glyph positioning)
  - `GDEF` (glyph definition — classes, mark glyph sets)
  - Optionally `morx` / `mort` (Apple Advanced Typography — for
    fonts that use AAT instead of OpenType, like San Francisco)
- **text-interfaces (TXT00)** — for the TextShaper / ShapeOptions
  / ShapedRun types.
- **text-metrics-font-parser (TXT01)** — for FontMetrics; TXT04
  pairs with TXT01 (same font binding).
- **Unicode data tables** for:
  - Script property (per-codepoint script assignment)
  - Bidi mirrored / joining class / Indic positional category
  - NFC normalization (TXT04 assumes input is NFC but applies
    its own normalization for robustness when script-specific
    requirements demand it — e.g., for Tibetan).

The Unicode data tables should live in a separate package
(`unicode-tables` or similar), auto-generated from the Unicode
Character Database. They are a shared dependency of TXT04 and
any future Unicode-aware consumers (bidi resolver, line
breaker, segmenter).

---

## Implementation notes

### Parsing OpenType tables

TXT04's first implementation task, before any shaping logic, is
extending `font-parser` (FNT00) to read GSUB, GPOS, and GDEF.
These tables are substantially more complex than the metrics-only
tables in FNT00 v0.1:

- **GSUB / GPOS shared structure**: script list → feature list
  → lookup list. Each lookup is one of 7 (GSUB) or 9 (GPOS)
  types, each with multiple format variants.
- **Extension lookups** (LookupType 7/9): indirection into
  32-bit offsets for large tables.
- **Coverage tables** in three formats.
- **Class definition tables** in two formats.

Adding this to FNT00 is a sizable PR of its own. TXT04's tier-1
implementation must ship that PR first; alternatively, TXT04
can vendor its own minimal OpenType table parser internally.
Recommendation: extend font-parser — a single source of truth
for OpenType tables across the stack is valuable.

### Shaping engines

HarfBuzz organizes shaping around **shapers** — one per script
family. Each shaper understands its script's specific rules
(Arabic joining, Indic reordering, etc.). TXT04 should follow
this pattern:

```
trait ShapingEngine {
    fn shape_run(
        codepoints: &[u32],
        font: &FontFile,
        features: &FeatureSet,
        buffer: &mut ShapedBuffer,
    );
}

impl ShapingEngine for LatinShaper { ... }   // tier 1 + tier 2
impl ShapingEngine for ArabicShaper { ... }  // tier 4
impl ShapingEngine for IndicShaper { ... }   // tier 5
// ...

fn shape(...) -> ShapedRun {
    let engine = pick_engine_from_script(options.script, ...);
    engine.shape_run(...);
}
```

Each engine is tested independently. This isolates
implementation complexity and makes "add a new script" a
straightforward extension.

### Testing strategy

Ground-truth comparison against HarfBuzz is the primary
correctness test. For each tier:

1. Build a test corpus of strings exercising the tier's
   features (Latin ligatures for tier 1; Arabic paragraphs for
   tier 4; Hindi blocks for tier 5).
2. Shape each string with HarfBuzz (via a dev-time binding or
   pre-computed output JSON checked into the repo).
3. Shape the same string with TXT04.
4. Assert per-glyph equality (glyph ID, cluster, advance,
   x_offset, y_offset) within a tight tolerance (typically 0
   for IDs/clusters, ±1 design unit for offsets/advances).

A HarfBuzz output difference is an acceptance failure. The goal
is byte-for-byte parity on the supported feature set.

Within the repo, tests live per-tier so a regression on tier-1
Latin doesn't block work on tier-5 Indic.

### Performance expectations

TXT04 is not a performance project. Tier-1 Latin should be
within 3× HarfBuzz's throughput on reference hardware (Rust
baseline). Tier 5+ may be slower — that's acceptable as long as
it's correct and reproducible. If TXT04 turns out to be the
bottleneck in a hot path, the right fix is to cache shaped
runs at the layout layer, not to optimize TXT04 prematurely.

---

## Package layout

```
text-shaper-harfbuzz    (every language eventually; Rust first,
                         other languages via FFI shim)
```

The package name **`text-shaper-harfbuzz`** is chosen deliberately
— the "harfbuzz" in the name refers to the functionality tier
(feature parity with HarfBuzz), not a dependency on libharfbuzz.
This implementation is from scratch.

Rust is the primary implementation (the font-parser foundation is
Rust; the table-parsing code is Rust; memory safety matters for
the untrusted-input aspect of font parsing). Other languages
consume TXT04 through a C ABI cbindgen-generated from the Rust
crate, same pattern TXT01/TXT02 expected.

A pure-language port (e.g., a Haskell port that re-implements
from scratch for learning) is welcomed but not required. The
Rust implementation is the canonical one.

---

## Non-goals

- **Shaping engine plugin architecture** in v1 beyond per-script.
  Custom shaping engines plugged in by third parties is a
  flexibility concern deferred until multiple engines exist.
- **Font subsetting during shaping.** TXT04 shapes against the
  full font; producing a subset for embedding (PDF, web fonts)
  is a separate concern.
- **Variable-font axis interpolation during shaping.** Variable
  fonts must be pre-instanced before TXT04 shapes them. Custom
  axes can be added to TXT05 and TXT01 first, then TXT04 picks
  up the already-instanced font.
- **Graphite-format shaping** (SIL's alternative to OpenType).
  Graphite is used by some linguistic-community fonts (Awami
  Nastaliq). TXT04 v1 supports OpenType only; Graphite can be
  a separate future spec.
- **AAT (Apple Advanced Typography) shaping**. Apple-authored
  fonts (San Francisco, Hiragino, GeezaPro) include AAT tables
  (`morx`, `kerx`) alongside or instead of OpenType. Tier 1
  parses OpenType only; AAT support can be added later for
  compatibility when a user wants to render with a specific
  Apple font on a non-Apple platform.

---

## Relationship to sibling specs

| Spec  | Relationship                                                                       |
|-------|------------------------------------------------------------------------------------|
| TXT00 | Defines the `TextShaper` trait TXT04 implements.                                   |
| TXT01 | Coherent partner. TXT01 metrics + TXT04 shaper form the full device-independent stack. |
| TXT02 | **TXT04 supersedes** for advanced typography. On basic Latin without features, TXT04 output MUST match TXT02 byte-for-byte (regression lock). |
| TXT03 | Orthogonal sibling (device-dependent).                                             |
| TXT05 | Produces the font handles TXT04 consumes. font-parser resolver variant.            |
| FNT00 | Upstream dependency. TXT04 ships GSUB/GPOS/GDEF extensions to FNT00.               |
| FNT02 | Downstream via paint dispatch. Glyph IDs from TXT04 (including those generated by GSUB ligature substitution) must be resolvable by FNT02's `glyf` lookup. |
| FNT03 | Downstream. Rasterizes the outlines FNT02 returns.                                 |
| P2D06 | Paint backends route TXT04 output via the `font-parser:` scheme.                   |

### The full device-independent + advanced pipeline (TXT04 target)

```
CommonMark parser
    │
    ▼
document-ast
    │
    ▼  per-run language segmentation (bidi + script split)
PositionedNode tree
    │
    ▼  for each text run:
 ┌────────────────────────────┐
 │ FontResolver (TXT05a)      │  font-parser:<hash> binding
 │ TXT01 FontMetrics          │  line-height math
 │ TXT04 TextShaper           │  script-specific engine dispatch
 │                            │      ↓
 │                            │   LatinShaper / ArabicShaper / IndicShaper / …
 │                            │      ↓
 │                            │   GSUB → GPOS → Emit ShapedRun
 └────────────────────────────┘
    │
    ▼
UI04 layout-to-paint
    │
    ▼
PaintGlyphRun { font_ref: "font-parser:...", ... }
    │
    ▼
Paint backend → FNT02 (outlines) → FNT03 (coverage) → pixels
```

### Why not just vendor HarfBuzz?

HarfBuzz is excellent software — mature, fast, battle-tested,
shipped with every major OS and browser. Vendoring it would give
us state-of-the-art shaping for free.

But the coding-adventures repo exists to build from first
principles. TXT04 is explicitly a long-running learning project;
the journey is the destination. At the end of many tiers, the
repo will have its own full OpenType shaper, reproducible on
every platform, understandable by anyone reading the code. That
is the goal — not the fastest path to "it works".

For projects that want HarfBuzz's maturity *right now*, use
TXT03 (the native-OS path, which internally uses HarfBuzz on
Linux). TXT04 is for the long game.

---

## Open questions

- **Whether tier ordering is fixed.** The current tier 1 → 6
  order is from "easiest to implement" toward "most complex".
  A consumer with specific script needs (say, "I only care
  about Arabic") may want to pursue tier 4 first. Tier
  ordering is a recommendation, not a hard dependency, except
  that tier 2 (GPOS) depends on tier 1 (GSUB infrastructure)
  being in place.

- **Whether to split TXT04 into per-tier sub-specs** as they
  mature. The current format is "one document, each tier is a
  section". When a tier grows detailed enough to warrant its
  own spec (TXT04.4 for Arabic, TXT04.5 for Indic), it may
  graduate to a standalone document. Defer formatting decision
  until needed.

- **How aggressive to be about Unicode version matching.**
  HarfBuzz tracks Unicode updates closely; TXT04 will lag. Tier
  1 can target Unicode 15.0; later tiers can update as needed.
  Worth being explicit about the target Unicode version in
  each tier's v1.0 release.

- **Whether to expose a "feature capability" probe**
  (`supports_feature("liga")`). Useful for consumers to fall
  back to TXT02 gracefully if TXT04 hasn't implemented a
  needed feature yet. Add when a real consumer needs it.
