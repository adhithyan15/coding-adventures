# TE13 - .NET Paint IR and VM Foundations

## Goal

Port the active paint foundation stack to both C# and F# as pure in-language
implementations:

- `pixel-container`
- `paint-instructions`
- `paint-vm`

This tranche intentionally targets the active `paint-*` path and does not add
new `.NET` work on the deprecated `draw-instructions` branch of the graphics
stack.

## Scope

Add these publishable packages:

- `code/packages/csharp/pixel-container`
- `code/packages/csharp/paint-instructions`
- `code/packages/csharp/paint-vm`
- `code/packages/fsharp/pixel-container`
- `code/packages/fsharp/paint-instructions`
- `code/packages/fsharp/paint-vm`

Each package must include:

- native implementation code only
- tests
- `BUILD`
- `BUILD_windows`
- `README.md`
- `CHANGELOG.md`
- package metadata
- `required_capabilities.json`

## Dependency Order

The tranche should be built in this order:

1. `pixel-container`
2. `paint-instructions`
3. `paint-vm`

`paint-instructions` should depend on the local `.NET` `pixel-container`
package. `paint-vm` should depend on the local `.NET` `paint-instructions`
package.

## Functional Requirements

### pixel-container

Both implementations should:

- define a fixed RGBA8 pixel buffer with `width`, `height`, and byte storage
- expose container creation helpers
- expose per-pixel read and write helpers
- expose whole-buffer fill helpers
- define an image codec interface/contract for encode/decode over the pixel
  container

### paint-instructions

Both implementations should:

- define the core `PaintScene` and `PaintInstruction` model from P2D00
- include the active instruction families needed by the current VM and barcode
  layout path
- re-expose the local pixel container types to preserve the paint-stack import
  ergonomics
- include builder/helper functions so tests and downstream packages can create
  instructions without verbose constructor boilerplate

### paint-vm

Both implementations should:

- implement a dispatch-table VM over paint instruction kinds
- support handler registration and duplicate-handler rejection
- support immediate-mode execution over a supplied backend context
- support wildcard fallback dispatch
- support the patch/diff helpers needed by the current package contract
- support export hooks for backends that can render to an offscreen pixel buffer
- expose the package-specific error types described by P2D01

## Behavioral Notes

- The F# packages must be implemented directly in F# and must not wrap the C#
  packages.
- No external graphics, pixel, or rendering libraries may be used in these
  foundation packages.
- The VM package is an abstraction layer, not a concrete backend. Tests should
  use fake contexts rather than native rendering APIs.
- `draw-instructions` is out of scope for new `.NET` work in this tranche.

## Test Coverage Targets

Tests should cover at least:

- pixel buffer creation, bounds-safe reads, writes, and fills
- image codec interface shape and simple stub implementations
- instruction builders and representative instruction variants
- scene metadata and pixel-container re-exports from `paint-instructions`
- duplicate-handler, unknown-instruction, null-context, and export-not-supported
  VM errors
- VM execute ordering, handler precedence, wildcard fallback, and recursive
  container dispatch
- patch/diff behavior on unchanged, added, removed, and modified instruction
  sets

## Out of Scope

- concrete SVG, Canvas, Metal, Direct2D, or terminal backends
- `barcode-layout-1d`
- 1D symbology packages such as `code39`, `code128`, `itf`, or `codabar`
- geometry packages not required by the current active paint/barcode path
