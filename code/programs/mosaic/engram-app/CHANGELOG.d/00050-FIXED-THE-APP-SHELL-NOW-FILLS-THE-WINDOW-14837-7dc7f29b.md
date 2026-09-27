### Fixed — the app shell now fills the window (#14837)

Engram's composition root measured `1280 x 776` in a `1280 x 900` window.
Reading the rendered PNG's alpha channel: the last painted row at x=640 was
**775**, and rows 776..899 were `(0, 0, 0, 0)` — genuinely unpainted, so
whatever composites behind the surface showed through.

Trestle has authored `min-height: "100vh"` on its app shell since it was built;
Engram simply never did. Both `.msl` files now do.

That alone fixed nothing, because `min-height` was **dropped by both of
Engram's gated backends**. Lowering it — Compose `fillMaxHeight()`, SwiftUI
`frame(maxHeight: .infinity)` — is the other half, and both halves were
falsified independently: with the Compose lowering disabled and the `.msl` line
kept, the last painted row goes straight back to 775.

Measured after: **899**. The window is fully painted.


