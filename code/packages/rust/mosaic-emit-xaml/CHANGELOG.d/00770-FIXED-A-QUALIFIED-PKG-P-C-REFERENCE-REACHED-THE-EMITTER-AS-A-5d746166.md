### Fixed — a qualified `pkg::P::C` reference reached the emitter as an unknown component (#14867 follow-up)

`ComponentRegistry::lookup` matched the raw tag, and the emitted element was
built from that same raw tag. Both are wrong for a qualified reference:

- The registry is keyed by **bare** export names — `build_self_package_registry`
  inserts `manifest.components.exports` verbatim — so `pkg::mosaic-pkg-grid::Cell`
  never matched the `Cell` that was registered, and the emit failed with
  `UnknownComponent`.
- Had it matched, `format!("<{prefix}:{tag}/>")` would have produced
  `<grid:pkg::mosaic-pkg-grid::Cell/>`, which is not well-formed XML.

`emit_component_reference` now splits a qualified tag with
`LayoutNode::package_ref()` and uses the component half for both the registry
key and the element name. When the tag names a package explicitly, that package
must match the registration's `package_name`, so `pkg::other-pkg::Cell` does not
silently resolve to `mosaic-pkg-grid`'s `Cell`.

**This did not affect a full build**, and no shipped artifact changes. The
package resolver inlines every `pkg::P::C` node before emit — its own contract
is that it "leaves backend emitters with a layout tree containing no qualified
tags" — and both `mosaic-compile` and the artifact builder run it. The path
that breaks is the emitter driven *without* that pass: a direct `from_pipeline`
call over a package's own `.mll`, which is how `tests/pkg_grid_compiles_to_xaml.rs`
exercises PR-5's registry.

That test is how this surfaced. #14867 qualified `Cell` in `Grid.mll` to recover
style that seven backends were dropping; the test compiles `Grid.mll` straight
into the emitter, so the newly-qualified tag reached a code path that had never
seen one and `main` went red.

None of the other seven emitters call `package_ref()` or `component()` either.
They are not broken today for the same reason XAML's full build is not — the
resolver runs first — but none of them would survive being driven directly with
a qualified tag. Filed separately rather than changed speculatively here.

