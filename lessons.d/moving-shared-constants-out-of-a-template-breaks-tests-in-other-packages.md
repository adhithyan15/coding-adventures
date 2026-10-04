---
category: Testing & coverage
---

# Moving shared constants out of a template breaks tests in other packages that read that template by path

**What went wrong.** PR #16624 split the Compose platform library: the
save rules and `MOSAIC_EXECUTABLE_EXTENSIONS` moved from
`templates/compose/MosaicPlatformEffects.kt` into the new shared
`MosaicFileEffects.kt`. Every check in `mosaic-app-bindings` and the builder
passed locally, but CI's `formula-edit` job failed. That job runs
`mosaic-app-wasm/js/file-effects.test.mjs`, which reads the Kotlin template
**by relative path** to pin the JS executable list to it. `indexOf` returned
-1, the slice was empty, and `deepStrictEqual` compared the JS list against
nothing.

**Fix.** Point the test at `MosaicFileEffects.kt`, and assert the
declaration is found, so a future move fails with a clear message rather
than an empty comparison.

**Do differently.** Before moving or renaming anything in a template (a
file, or a constant inside one), grep the whole repo for the file's path and
the constant's name, not just the owning package:
`git grep -n "<file name>\|<CONSTANT>" -- code`. Cross-language parity
tests live with the *other* language's package, and the build tool only runs
them when that package is affected, so the owning package's own tests won't
catch it.
