---
category: BUILD files & dependency management
---

# A package with NO BUILD file is invisible to CI — it is never built and its tests never run

The build tool discovers packages by their BUILD file, so a crate without one is silently unwatched no matter how many tests it has. `lang-aot` hid three red suites this way; `twig-aot` (PR #11264) hid both a red suite and a test target that did not compile at all. A workspace-wide `cargo check` elsewhere keeps such a crate *compiling* while nothing ever compiles its **test targets** or runs its assertions — which is exactly how an unused import in a test file sits on main indefinitely. When adding a BUILD to a long-unwatched package, expect to find existing breakage, and list test targets explicitly (`--test a --test b`) rather than running the whole crate, so any target you must exclude is visible and named in a comment. Excluding a target skips only its RUN, never its COMPILE — the check step still compiles every target with `-D warnings`.
