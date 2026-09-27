### Added — the resolver's contract is now enforced, not assumed (#14886)

`LayoutPackageResolver` promises backends "a layout tree containing no qualified
tags", and every emitter is written against that guarantee — **none of the eight
calls `package_ref()` or `component()` to check**. #14884 was the worked
example: a `.mll` with a qualified `Cell` reached the XAML emitter with no
resolver pass and `main` went red.

Measured what all eight actually do when handed `pkg::mosaic-pkg-grid::Grid`
directly:

| backend | behaviour |
| --- | --- |
| compose, swiftui, html, react, webcomponent, qt, flutter, xaml | `UnknownPrimitive("pkg::mosaic-pkg-grid::Grid")` |

**The failure mode worth fearing does not exist.** A backend that stripped the
qualifier and emitted a structurally plausible element for the wrong component
would fail *quietly* — that is the shape of #14867, which #14886 was opened to
rule out. All eight fail loudly and name the offending tag.

So the gap was never the behaviour; it was that nothing pinned it. This test is
that pin, and it fails if any emitter starts accepting a qualified tag.

It asserts on the **error** rather than on the absence of the component in the
output: a backend that emitted an empty string would satisfy "no `Grid` in the
output" while having silently dropped the component. Falsified by pointing the
fixture at a tag the emitters do accept, which fails with `compose ACCEPTED a
qualified tag and emitted 3298 bytes`.

