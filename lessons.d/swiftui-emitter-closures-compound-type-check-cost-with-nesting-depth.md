---
category: Mosaic compiler pipeline
---

# SwiftUI emitter closures compound type-check cost with nesting depth

Adding one more view to Trestle's `If`/`Else` view chain (Checklists, C3a
of #14018) made the generated SwiftUI release build fail: "the compiler is
unable to type-check this expression in reasonable time". Every local check
(emitter tests, native-complete reports, control contracts) was green, because
none of them compiled the generated Swift in release mode.

**Cause:** `If` was lowered to an immediately-invoked closure with the
`if`/`else` directly inside. Swift type-checks a multi-statement closure
together with its enclosing expression (SE-0326), so nesting compounds.

**Fix:** put the branch in a local `func _mosaicBranch()`, as nodes already use
`func _mosaicNode()`. A local function's body is checked on its own.
`tests/nested_if_chain_typechecks.rs` pins a 10-deep chain.

**Do differently:**
- When a change deepens a generated SwiftUI tree (a new view in a chain, a new
  wrapper), run `swift build -c release` on the emitted project locally. Swift
  is on the dev Mac; it takes about 20 s.
- In the emitter, never leave control flow directly in an expression-embedded
  closure; wrap it in a local function.
