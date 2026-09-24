---
category: CI & GitHub Actions
---

# An emitter fix that retires a pinned style drop leaves every other package's pin stale until something rebuilds it

Mosaic packages pin known native style drops in `ALLOWED_STYLE_DROPS`, and
`no_pinned_style_drop_has_silently_been_fixed` fails when a pinned drop stops
happening. That check only runs when the package itself is rebuilt. A Flutter
emitter change that started lowering `font-size` merged green, because diff-based
CI never rebuilt the five packages that pinned `(Flutter, "font-size")`:
review-card, session-progress, collection-actions, note-editor and review-history.

The stale pins surfaced days later on an unrelated Qt emitter PR. That PR
rebuilt every package downstream of `mosaic-package-artifact-builder`, and all
five gates failed at once. None of the failures had anything to do with the Qt
change.

**What to do:** when an emitter change removes a drop, grep every
`native_complete_gate.rs` for that `(Backend, "property")` pin and delete it in
the same PR. When your PR touches `mosaic-emit-*` or the artifact builder,
run `cargo test` in every Mosaic package that depends on them *before* pushing.
Expect stale pins from someone else's earlier fix, and remove them.
