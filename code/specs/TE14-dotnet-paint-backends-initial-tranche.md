# TE14 — .NET Paint Backends Initial Tranche

## Goal

Build the first real PaintVM backend implementations for the .NET paint stack
so the new C# and F# paint foundations can render concrete output, not just
define scenes and dispatch handlers.

This tranche focuses on the two most useful pure managed backends for the
current roadmap:

- `paint-vm-ascii` — terminal/text rendering for debugging, docs, and tests
- `paint-vm-svg` — string-based vector rendering for barcode and document output

These two backends are the best next step because they:

- avoid native graphics APIs and platform-specific bindings
- produce deterministic outputs that are easy to test
- directly support the upcoming `barcode-layout-1d` and barcode convenience
  packages
- exercise important parts of the paint IR such as clipping, grouping, glyph
  placement, and strict unsupported-feature failures

## Packages

The tranche adds four packages:

- `code/packages/csharp/paint-vm-ascii`
- `code/packages/csharp/paint-vm-svg`
- `code/packages/fsharp/paint-vm-ascii`
- `code/packages/fsharp/paint-vm-svg`

All four packages must be pure in-language implementations. The F# ports must
not wrap the C# versions.

## Dependency Shape

Each backend depends on the already-ported .NET paint foundations:

- `paint-vm`
- `paint-instructions`
- `pixel-container` when needed by the backend API

`paint-vm-svg` may also need `pixel-container` to support image instructions
that embed in-memory pixel buffers rather than URI references.

## Backend Scope

### `paint-vm-ascii`

The .NET ASCII backend should follow the established P2D02 behavior and the
TypeScript reference package:

- expose an options type with `ScaleX` and `ScaleY`
- expose a convenience `Render` API that returns a string
- expose a reusable VM factory and context type
- render:
  - `rect`
  - `line`
  - `glyph_run`
  - `group`
  - `clip`
  - plain `layer`
- fail loudly for unsupported features:
  - `gradient`
  - `image`
  - transformed groups/layers
  - filtered layers
  - non-default opacity or blend modes

Important behavioral rules:

- use box-drawing characters for strokes
- use `█` for fill
- preserve text over later non-text cell writes
- replace unsafe control/bidi glyphs with a safe fallback
- trim trailing spaces and trailing blank lines in final output

### `paint-vm-svg`

The .NET SVG backend should mirror the TypeScript package behavior:

- expose a context accumulator for `defs` and rendered elements
- expose a reusable VM factory
- expose a convenience `RenderToSvgString(scene)` API
- render to a complete `<svg ...>...</svg>` string

Initial supported instructions:

- `rect`
- `ellipse`
- `path`
- `line`
- `glyph_run`
- `group`
- `layer`
- `clip`
- `gradient`
- `image`

Important SVG rules:

- validate numeric values before interpolation
- XML-escape text and attribute values
- render a background rect unless the scene background is transparent
- emit gradients and clip paths via `<defs>`
- support either URI-backed images or embedded image data for pixel buffers
- preserve instruction ordering in emitted elements

## Testing

Every package must have:

- package-level BUILD scripts
- unit tests with coverage comfortably above 80%
- README and CHANGELOG updates

Minimum test shape:

- version test
- top-level convenience rendering test
- supported instruction rendering tests
- strict unsupported-instruction or unsupported-feature tests
- clipping/grouping tests
- escaping and numeric validation tests for SVG
- unsafe glyph replacement tests for ASCII

## Out of Scope

This tranche does not include:

- `paint-vm-canvas`
- native backends such as Direct2D, GDI, Metal, or Cairo
- image codecs beyond what is minimally needed for `paint-vm-svg`
- `barcode-layout-1d` itself

## Follow-on Work

Once these packages land, the next natural paint-related tranches are:

1. `barcode-layout-1d` in C# and F#
2. barcode convenience packages that target `paint-vm-svg`
3. raster and native backends for pixel output and UI integration
