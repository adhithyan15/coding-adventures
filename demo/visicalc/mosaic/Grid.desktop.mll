// Grid.desktop.mll — UI34 PR-4 (final).
//
// THE COLLAPSE — what this file used to be vs what it is now
// ----------------------------------------------------------
//
// Before UI34: 148 lines of hand-rolled
// HostTable + HostTableColGroup + HostTableHead + HostTableBody
// + nested For + Box[cell] + state-when-* + If/Else + HostInput
// composition.  Inlined here because mosaic-compile's package
// resolver wasn't wired through the demo build script.
//
// After UI34 (PR-3 #4969 + PR-4 #4974): one `pkg::mosaic-pkg-grid::Grid`
// reference.  mosaic-compile locates the package on its
// `--package-search-path`, recursively compiles
// `mosaic-pkg-grid`'s Grid + Cell triple, substitutes the resolved
// sub-tree at this call site, and rewires every slot/emit
// reference inside it to the visicalc consumer's matching slot /
// emit name.  The generated `src/components/Grid.tsx` is
// **byte-identical** to the pre-collapse output — same 74 lines
// of React, same behaviour, same per-cell state-when style spreads.
//
// What the collapse proves
// ------------------------
//
//   1. UI34's `pkg::P::C` syntax works end-to-end on the React
//      backend — grammar, AST, resolver, emitter all line up.
//   2. The package is now the single source of truth for what a
//      Grid is.  When `mosaic-pkg-grid` ships sticky-header
//      support or a new state-when predicate, this demo
//      automatically gets the new behaviour at the next
//      `bash scripts/build.sh`.  No `.mll` drift, no copy-paste
//      maintenance.
//   3. The other six VisiCalc demos (visicalc-html, -webcomp,
//      -swiftui, -qt, -flutter, -compose, -android) — all of
//      which currently hand-write their grid widget directly in
//      the host language — can follow the same path once their
//      respective backend emitters are confirmed to handle the
//      kernel primitives `mosaic-pkg-grid::Grid` expands into.
//
// How the bindings work
// ---------------------
//
// Every prop name on the left side of `:` is a slot or emit
// declared in `mosaic-pkg-grid`'s `Grid.mil`.  Every value on the
// right side is bound to a slot or emit declared in this demo's
// own `Grid.mil`.  Because the demo's `Grid.mil` declares slots
// with the SAME names as the package's, every binding is a
// pass-through (`viewport-rows: slot: viewport-rows`).
//
// The resolver's `rewrite_bindings` step walks the inlined Grid
// sub-tree and substitutes every `slot: viewport-rows` reference
// in the package's body with the call-site's
// `slot: viewport-rows` — which here is the SAME slot name
// because the consumer mirrors the package's interface, but
// the indirection is what makes a name-mismatched consumer
// (e.g. `viewport-rows: slot: my-data`) work too.

layout Grid {
  pkg::mosaic-pkg-grid::Grid (
    viewport-rows:    slot: viewport-rows ,
    column-headers:   slot: column-headers ,
    column-widths:    slot: column-widths ,
    selected-row:     slot: selected-row ,
    selected-col:     slot: selected-col ,
    edit-row:         slot: edit-row ,
    edit-col:         slot: edit-col ,
    edit-content:     slot: edit-content ,
    onNavigate:       emit: onNavigate ,
    onFormulaChange:  emit: onFormulaChange ,
    onEditCommit:     emit: onEditCommit ,
    onEditCancel:     emit: onEditCancel
  )
}
