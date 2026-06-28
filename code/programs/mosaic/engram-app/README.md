# engram-app

Engram's Mosaic app package.

This package is the product assembly layer. It exports `EngramApp`, owns the
app/root surface, and depends on reusable Mosaic component packages such as
`mosaic-pkg-review-card` and `mosaic-pkg-session-progress`.
The review card composes further Mosaic packages such as
`mosaic-pkg-rating-controls`; Engram does not fork those components into the app
package.

Reusable UI components should live under `code/packages/mosaic-pkg-*`. Engram
itself should grow here as an app package that composes those components and
binds them to the shared Rust business logic core through host shells.

## Current surface

- `EngramApp.mil` defines the app-facing review slots and events.
- `EngramApp.mll` owns the product shell and mounts
  `pkg::mosaic-pkg-session-progress::SessionProgress` plus
  `pkg::mosaic-pkg-review-card::ReviewCard`.
- `EngramApp.dark.msl` owns app-shell styling only.
- Package artifact builds inline component-package styles through the full
  dependency chain.

## Running the smoke test

```bash
cd code/programs/mosaic/engram-app
cargo test
```
