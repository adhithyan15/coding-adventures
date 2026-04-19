# TXT05 — FontResolver: Abstract font refs → concrete handles

## Overview

TXT05 defines the `FontResolver` interface — the layer that turns
abstract, author-facing font specifications ("Helvetica 12pt bold",
`font-family: sans-serif`) into concrete `FontHandle` values the
TXT00 trait family can operate on.

```
Author text               "font-family: Helvetica, Arial, sans-serif;
                           font-weight: 700; font-style: italic"
    │
    ▼
FontQuery                  { family_names: ["Helvetica", "Arial",
                             "sans-serif"],
                             weight: 700,
                             style: italic,
                             stretch: normal }
    │
    ▼
FontResolver  (TXT05)      picks the best match, returns a
    │                      backend-specific FontHandle
    ▼
FontHandle                 ready to pass to FontMetrics (TXT01,
                           TXT03a) and TextShaper (TXT02, TXT04)
```

The resolver is the **entry point** to the text pipeline: nothing
else turns a string like "Inter" into bytes or into an OS font
handle, so every downstream spec depends on one.

A key design property: **FontResolver is generic over the handle
type it returns**. The interface is not "produce a FontFile" or
"produce a CTFontRef" — it's "produce an `H`, where `H` is the
backend's handle type". Each implementation commits to one `H` and
partners with the matching FontMetrics and TextShaper backends.

This preserves the **font-binding invariant** from TXT00: a handle
from a font-parser resolver is only usable with font-parser-backed
FontMetrics and shaper. A handle from a CoreText resolver is only
usable with CoreText-backed FontMetrics and shaper. The type system
should make the mismatch a compile-time error in statically-typed
languages.

---

## The FontQuery type

```
FontQuery {
  family_names: Vec<String>,
  // Ordered fallback list, highest preference first.
  // Concrete family names ("Inter", "Helvetica Neue") may be mixed
  // with generic names ("sans-serif", "serif", "monospace",
  // "cursive", "fantasy", "system-ui"). The resolver walks left to
  // right and returns the first match. If nothing matches (and
  // there's no last-resort fallback — see below), it throws.

  weight: FontWeight,
  // CSS-style numeric weight, 1..=1000. Common values:
  //   100 Thin, 200 ExtraLight, 300 Light, 400 Regular (default),
  //   500 Medium, 600 SemiBold, 700 Bold, 800 ExtraBold, 900 Black.
  // Finer granularity allowed for variable fonts (e.g., 425).

  style: FontStyle,
  // One of: Normal, Italic, Oblique.
  // Oblique is a synthesized slant of the upright face; Italic is a
  // separately-designed face. The resolver MAY substitute one for
  // the other when the exact style is unavailable; this is called
  // out in the matching algorithm below.

  stretch: FontStretch,
  // CSS-style width category. Values:
  //   UltraCondensed, ExtraCondensed, Condensed, SemiCondensed,
  //   Normal (default), SemiExpanded, Expanded, ExtraExpanded,
  //   UltraExpanded.
  // Represented as an enum with a fixed numeric rank (1..=9) for
  // distance computation in the matching algorithm.
}
```

Notable omissions:

- **No `size`.** Glyph size is passed to `TextShaper.shape()` at
  shape time, not at resolution time. The resolver is purely about
  "which face". An exception: variable fonts with an `opsz`
  (optical size) axis may use a size hint. TXT05 v1 does not expose
  this; add later if needed.
- **No language / script.** Resolver picks a face based on
  family/weight/style. Picking a *different* face for different
  scripts (CJK fallback, emoji fallback) is a layout-engine
  concern — the layout engine runs the resolver multiple times
  with different family lists for different language runs.
- **No feature settings.** OpenType features are passed to the
  shaper, not the resolver. A feature-aware caller does its own
  "does this font have feature X" lookup after resolution.

These omissions keep FontQuery narrow and the interface stable.

### FontWeight as numeric, not enum

The weight is a raw number, not an enum. Rationale:

- Variable fonts support arbitrary weights along the `wght` axis;
  an enum would need a `Custom(u16)` escape hatch that most callers
  use incorrectly.
- CSS uses numeric values directly; matching the CSS model reduces
  impedance when resolving from CSS-style queries.
- Helper constants (`FontWeight::BOLD = 700`) give the ergonomics
  back where they matter.

---

## The FontResolver interface

```
trait FontResolver<H> {
  /// Resolve a query to a concrete FontHandle of type H.
  ///
  /// Returns the best available match per the matching algorithm.
  /// Throws FontResolutionError if nothing matches and no
  /// last-resort fallback is configured.
  fn resolve(&self, query: &FontQuery) -> Result<H, FontResolutionError>;

  /// (Optional) Check whether a family name is resolvable at all,
  /// without committing to a full resolution. Used by layout
  /// engines to probe for fallback before doing the work.
  ///
  /// Default implementation: call resolve() and return true on
  /// success. Backends MAY override with a faster check.
  fn has_family(&self, family: &str) -> bool { ... }

  /// (Optional) List all concrete family names the resolver knows
  /// about. Useful for debugging and for layout-engine font
  /// picker UIs. NOT stable across resolver versions or across
  /// platforms; callers use it for diagnostics only.
  fn list_families(&self) -> Vec<String> { ... }
}
```

The interface is intentionally small. Only `resolve()` is required;
the other methods have default implementations.

### What "best match" means

A resolver walks `family_names` in order. For the first family name
that has at least one registered face, it picks the face minimizing
a weighted sum of **weight distance**, **style distance**, and
**stretch distance**.

```
distance(query, face) =
    weight_distance(query.weight, face.weight) × W_weight
  + style_distance(query.style, face.style)    × W_style
  + stretch_distance(query.stretch, face.stretch) × W_stretch

weight_distance(qw, fw) = |qw − fw|
                          // Simple linear; CSS §5.2 specifies a
                          // slightly asymmetric rule (prefer going
                          // heavier when asked for 400-500 and
                          // missing; prefer going lighter when
                          // asked for >=500). Implementations
                          // SHOULD follow CSS §5.2 for
                          // interoperability; this spec documents
                          // the linear fallback.

style_distance(qs, fs) =  0 if qs == fs
                          1 if qs == Italic and fs == Oblique (or v.v.)
                         10 otherwise
                          // Italic ↔ Oblique are acceptable
                          // substitutes but strongly prefer exact.

stretch_distance(qst, fst) = |rank(qst) − rank(fst)|
                              where rank is 1..=9 per the enum
```

Default weights: `W_weight = 1`, `W_style = 100`, `W_stretch = 50`.
Style is weighted heavily so an exact-weight-wrong-style match loses
to a close-weight-correct-style match. Implementations MAY expose
these weights for tuning but MUST default to the values above.

If no face of the first family name is registered, the resolver
moves to the next family name. It does NOT return a weight-matched
face from a different family than requested — that would silently
substitute (e.g., "I asked for Helvetica Bold; you gave me Arial
Bold"), which is exactly the kind of quiet substitution the TXT00
design tries to avoid.

### Generic family handling

Generic families (`"sans-serif"`, `"serif"`, `"monospace"`,
`"cursive"`, `"fantasy"`, `"system-ui"`) are resolved through a
**generic-family map** configured on the resolver. The map
translates the generic name to a concrete family-name list, which
is then walked by the same algorithm.

Default generic-family map (platform-dependent, documented per
backend):

| Generic      | macOS default                          | Windows default                | Linux default        |
|--------------|----------------------------------------|--------------------------------|----------------------|
| serif        | Times New Roman, Times                 | Times New Roman                | DejaVu Serif, Liberation Serif |
| sans-serif   | Helvetica Neue, Helvetica, Arial       | Segoe UI, Arial                | DejaVu Sans, Liberation Sans |
| monospace    | Menlo, Courier New                     | Consolas, Courier New          | DejaVu Sans Mono     |
| cursive      | Snell Roundhand                        | Comic Sans MS                  | (no default)         |
| fantasy      | Papyrus                                | Impact                         | (no default)         |
| system-ui    | -apple-system, BlinkMacSystemFont      | Segoe UI                       | system-ui            |

A resolver MUST allow the generic-family map to be overridden at
construction time. Device-independent resolvers (font-parser)
default to an **empty** generic-family map — the caller must supply
one, since the resolver has no knowledge of what fonts are
"installed".

---

## Error conditions

| Error                       | When                                                                 |
|-----------------------------|----------------------------------------------------------------------|
| `FontResolutionError::NoFamilyFound` | None of the `family_names` resolve to any registered face  |
| `FontResolutionError::EmptyQuery`    | `family_names` is empty                                    |
| `FontResolutionError::InvalidWeight` | `weight` is outside 1..=1000                               |
| `FontResolutionError::LoadFailed`    | A matching face was found but its bytes could not be loaded (I/O error, malformed file, permission denied) |

`NoFamilyFound` is the common one. Callers catching it typically
either retry with a different generic-family fallback or surface a
user-facing error. Silently substituting a different font is
discouraged but permitted — it's a caller policy choice.

### Last-resort fallback

Resolvers MAY expose an opt-in "last-resort fallback": an always-
available face returned when nothing matches. This is useful for
display environments that must render *something* (browsers,
terminals).

```
resolver.with_last_resort("LastResort")
```

With this set, `resolve()` never throws `NoFamilyFound`; instead it
silently returns the last-resort face. Callers that need loud
failure MUST NOT configure a last-resort.

Default: no last-resort. Opt-in only.

---

## Per-backend behavior

### FontResolver-font-parser (device-independent)

Maintains an in-memory registry of parsed `FontFile` objects. The
caller "registers" fonts at setup time:

```
resolver.register(family: "Inter",
                  weight: 400, style: Normal,
                  bytes: inter_regular_ttf_bytes);
resolver.register(family: "Inter",
                  weight: 700, style: Normal,
                  bytes: inter_bold_ttf_bytes);
```

`resolve()` walks the registry and returns a `FontFile` reference
packaged in a `FontParserHandle` (as specified in TXT01).

Generic-family map defaults to empty — callers configure it
explicitly for reproducibility. A "register a font and give it a
generic alias" helper is provided:

```
resolver.register_with_generic("Inter",
                                variants,
                                as_generic: "sans-serif");
```

### FontResolver-coretext (macOS, iOS)

Translates `FontQuery` into a `CTFontDescriptor`, calls
`CTFontCreateWithFontDescriptor`, returns the resulting `CTFontRef`
as the handle. Generic-family map is populated from the
Apple-recommended defaults at construction time.

Uses the **system font registry**: anything installed on the
machine is resolvable. No explicit registration needed.

### FontResolver-directwrite (Windows)

Translates `FontQuery` into an `IDWriteFontSet::GetMatchingFonts`
call, returns the first `IDWriteFont` in the result set. Handles
wrap `IDWriteFont` + `IDWriteFontFace`.

Uses the Windows system font registry. Font substitution via
`DirectWriteFontFallback` is available but not used by default;
callers explicitly opt in.

### FontResolver-fontconfig (Linux)

Translates `FontQuery` into a `FcPattern`, calls `FcFontMatch`,
returns the best match. The handle wraps the `FcPattern` and the
font file path returned by fontconfig, plus (for shaping) a parsed
`FontFile` from font-parser.

Fontconfig handles generic-family resolution natively, so the
resolver's generic-family map is empty and the call is delegated.

### FontResolver-browser (TypeScript, browser only)

Queries `document.fonts` to find loaded faces. Returns a handle
that identifies the face by a CSS `font` shorthand string — the
Canvas backend uses this string directly; the SVG backend embeds it
into `font-family` / `font-weight` attributes.

Unlike other backends, no parsed font file is available — the
browser keeps the bytes private. This means the browser backend
cannot pair with `font-parser`-backed FontMetrics or TextShaper;
consumers must use the browser-backed metrics shim (which wraps
`FontFace.load()` and `ctx.measureText()`) and the browser-backed
shaper.

---

## Package layout

```
font-resolver-font-parser    (every language)
font-resolver-coretext       (Swift, Rust+FFI; macOS/iOS only)
font-resolver-directwrite    (C#, Rust+FFI; Windows only)
font-resolver-fontconfig     (every language via Rust+FFI; Linux)
font-resolver-browser        (TypeScript; browser only)
```

Each package:
- Depends on that language's `text-interfaces` (TXT00) for the
  FontQuery / FontResolver / FontResolutionError types.
- Depends on that language's `font-parser` (FNT00) if it needs to
  parse font bytes (true for font-parser resolver, fontconfig
  resolver; false for CoreText / DirectWrite / browser).
- Exposes a constructor with sensible defaults and a builder-style
  API for the generic-family map and last-resort config.
- Returns backend-specific handle types that satisfy the TXT00
  `FontHandle` contract.

### Rust reference signature

```rust
pub struct FontParserResolver {
    registry: Vec<RegisteredFace>,
    generics: HashMap<String, Vec<String>>,
    last_resort: Option<Arc<FontFile>>,
    weights: MatchingWeights,
}

impl FontParserResolver {
    pub fn new() -> Self;

    pub fn register(&mut self,
                    family: impl Into<String>,
                    weight: FontWeight,
                    style: FontStyle,
                    stretch: FontStretch,
                    bytes: Vec<u8>) -> Result<(), FontResolutionError>;

    pub fn with_generic(mut self,
                        generic: impl Into<String>,
                        families: Vec<String>) -> Self;

    pub fn with_last_resort(mut self, bytes: Vec<u8>) -> Self;
}

impl text_interfaces::FontResolver<FontParserHandle<'_>> for FontParserResolver {
    fn resolve(&self, query: &FontQuery)
        -> Result<FontParserHandle<'_>, FontResolutionError>;
}
```

Other languages follow idiomatic patterns.

---

## Testing strategy

Every FontResolver implementation MUST include the following
tests:

1. **Exact match.** Register an Inter Regular face; query for
   `{ families: ["Inter"], weight: 400, style: Normal }`; assert
   the returned handle parses back to that face.

2. **Weight substitution.** Register only Inter Regular (weight
   400); query for weight 500; assert Inter Regular is returned
   (closest match).

3. **Family fallback.** Register only Arial; query for `[
   "Helvetica", "Arial" ]`; assert Arial is returned.

4. **Generic family fallback.** Configure sans-serif →
   ["Helvetica", "Arial"]; query for `["sans-serif"]`; assert
   Helvetica is returned if registered, else Arial.

5. **No match error.** Query for an unregistered family with no
   generic fallback and no last-resort; assert
   `NoFamilyFound` is thrown.

6. **Italic/Oblique substitution.** Register Inter Regular
   (Normal) and Inter Italic; query for
   `{ style: Oblique }`; assert Italic is returned (distance 1
   from Oblique, preferred over Normal at distance 10).

7. **Stretch distance.** Register Inter Regular and Inter
   Condensed; query for `{ stretch: SemiCondensed }`; assert
   Condensed is returned (rank distance 1 vs 4).

8. **Empty family_names.** Throws `EmptyQuery`.

9. **Last-resort.** With last-resort configured, an unresolvable
   query returns the last-resort handle instead of throwing.

10. **Backend-binding invariant.** A handle produced by one
    resolver cannot be passed to a different backend's FontMetrics
    without a compile or runtime error. This test is most
    meaningful in statically-typed languages.

Coverage target: **90%+**.

### Cross-resolver conformance (future)

When FNT05 (the Coding Adventures Test Font) gains an SIL-style
filename, the same font can be registered with the font-parser
resolver AND installed as a system font for the CoreText /
DirectWrite / fontconfig resolvers. A shared test asserts that all
resolvers return handles with metrics matching the same expected
JSON — proving the interface is backend-agnostic.

---

## Non-goals

**Font downloading.** The resolver does not fetch fonts from URLs
or from a CDN. Downloading is the caller's job; the resolver takes
already-fetched bytes (or an OS-registered font).

**Font subsetting.** The resolver returns a full-face handle, not
a subset. Subsetting for PDF embedding is a downstream concern.

**Variable-font axis resolution beyond weight/style/stretch.**
Custom axes (`opsz`, `slnt`, `GRAD`) are out of scope for v1. A
future revision may extend FontQuery with a generic axes map
(`{"opsz": 12.0, "GRAD": 0.5}`).

**Language-aware fallback.** Picking a CJK font when the user asks
for Latin but the text contains Chinese is a **layout-engine**
concern. The layout engine runs the resolver per language run.

**Font loading caches.** Resolvers MAY cache loaded font files to
avoid re-parsing, but the caching policy is implementation-defined.
Consumers should not depend on specific caching behavior.

**Thread safety.** The default assumption is single-threaded use.
Thread-safe wrappers are a separate concern; each language's
package documents its own threading model.

---

## Relationship to sibling specs

| Spec  | Relationship                                                                         |
|-------|--------------------------------------------------------------------------------------|
| TXT00 | Defines `FontResolver`, `FontQuery`, `FontResolutionError`. TXT05 is the detailed spec for this interface (the TXT00 roadmap entry is a stub). |
| TXT01 | Downstream consumer — takes FontHandles from the font-parser resolver                |
| TXT02 | Downstream consumer — takes FontHandles from the font-parser resolver                |
| TXT03a| Downstream consumer — takes FontHandles from the CoreText resolver                   |
| TXT03b| Downstream consumer — takes FontHandles from the DirectWrite resolver                |
| TXT03c| Downstream consumer — takes FontHandles from the fontconfig resolver                 |
| TXT04 | Downstream consumer — takes FontHandles from the font-parser resolver (hand-rolled HarfBuzz-equivalent) |
| FNT00 | Upstream dependency for the font-parser resolver (parses font bytes)                 |
| FNT05 | Primary test fixture — registered with every resolver in tests                       |

### The full pipeline as of TXT05

```
CSS / CommonMark author styles
    │
    ▼ "font-family: sans-serif; font-weight: 700"
FontQuery
    │
    ▼
FontResolver (TXT05)              ─ picks device-indep or device-dep backend
    │
    ▼ FontHandle (opaque, backend-typed)
 ┌─────────────────────────┐
 │  FontMetrics (TXT01/03) │      ─ line heights
 │  TextShaper (TXT02/03/04)│     ─ positioned glyph runs
 └─────────────────────────┘
    │
    ▼ PaintGlyphRun with font_ref
Paint backend (P2D02..P2D05)
    │
    ▼
pixels
```

With TXT00 + TXT01 + TXT02 + TXT05 merged, the device-independent
text pipeline is **specified end to end**. Implementation PRs can
now fill in each layer without blocking on another spec.

---

## Open questions

- **Asynchronous resolution.** In browsers and in some server
  environments, font loading is naturally async (`FontFace.load()`,
  streaming font loaders). The current spec is synchronous. A
  future revision MAY add `resolve_async()` or split into
  `Resolver` vs. `AsyncResolver` traits. Deferred until a concrete
  async caller exists.

- **Font fallback as a first-class concept.** Language-aware
  fallback ("use Helvetica for Latin but Hiragino Sans for
  Japanese") is currently a layout-engine responsibility. It could
  be pushed into the resolver via a `FallbackChain` concept. Not
  done here because it significantly expands the interface surface
  and couples the resolver to script detection.

- **Whether to expose `FontDescriptor` as an intermediate type**
  (more metadata than a handle, richer than a query) so callers
  can do "list all faces with weight >= 700" queries. Useful for
  font-picker UIs but not essential for layout. Deferred.

- **Negative test for cross-backend handle passing.** Statically-
  typed languages (Rust, TypeScript, C#, Swift) can enforce the
  font-binding invariant via generic type parameters. Dynamically-
  typed languages (Python, Ruby, Lua) have to rely on runtime
  type tags. Whether to standardize a runtime tag mechanism in
  this spec or leave it to each backend is unresolved.
