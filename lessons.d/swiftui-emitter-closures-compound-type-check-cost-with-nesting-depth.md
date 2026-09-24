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

**Compose hit the same wall differently.** A plain Kotlin `if` keeps every
branch in the enclosing lambda, so the whole chain became one JVM method and
exceeded the 64 KB limit (`MethodTooLargeException`). Each branch now runs in
a non-inline `_MosaicBranch { }` composable, which gives it a method of its own.

**Do differently:**
- When a change grows or deepens a generated app tree (a new view in a chain,
  a new wrapper), compile the emitted native projects locally before pushing:
  `swift build -c release` (SwiftUI) and `gradle compileKotlin` (Compose).
  Swift, Java 21 and Gradle are on the dev Mac; each takes about 20 s. The
  emitter tests and native-complete reports do not compile anything.
- In an emitter, never let one construct absorb an unbounded subtree. Give
  each branch its own function or lambda.
