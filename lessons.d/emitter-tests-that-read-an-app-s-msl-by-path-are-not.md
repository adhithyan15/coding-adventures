---
category: Mosaic compiler pipeline
---

# Emitter tests that read an app's .msl by path are not rebuilt when only the app changes; run all mosaic-emit-* tests after editing an app's styles

**What went wrong.** C3c (#15962) gave Trestle's Checklists controls clones
of `add-btn` and the outline chip styles, so the app gained 4 pressed and 6
hover states. Two emitter tests pin those counts:

- `mosaic-emit-swiftui/tests/task_app_press_compiles_to_swiftui.rs` (exactly
  2 pressed surfaces);
- `mosaic-emit-xaml/tests/task_app_hover_compiles_to_xaml.rs` (exactly 8
  `IsPointerOver` bindings).

They read `code/programs/mosaic/task-app/src/TaskApp.*` through a relative
path from `CARGO_MANIFEST_DIR`, not through a Cargo dependency. So the build
tool's graph does not know that the emitter crates depend on the app. #15962
only touched the app, CI never ran those tests, and the PR merged green.
Main was then red for them, and the failure surfaced on the NEXT PRs that
touched any `mosaic-emit-*` crate (#15966, #15967, #15968), none of which
had caused it.

**The fix.** #15970 updates the counts (6 and 14) and names the new surfaces,
so the tests still say which controls they are checking.

**Do differently.** After editing an app's `.mil`, `.mll` or `.msl`, grep for
tests that read it by path and run them:

    grep -rln "task-app/src\|TaskApp.light.msl" code/packages/rust/*/tests

or simply run every emitter suite:

    cargo test -p mosaic-emit-compose -p mosaic-emit-swiftui -p mosaic-emit-xaml \
      -p mosaic-emit-qt -p mosaic-emit-flutter -p mosaic-emit-react \
      -p mosaic-emit-html -p mosaic-emit-webcomponent -p mosaic-emit-paint

When a PR fails a test in a crate it did not touch, check main first
(`git checkout origin/main` and run the test). The break may already be
there.
