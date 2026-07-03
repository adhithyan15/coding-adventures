// Column.mll — metadata-only marker for Column.
//
// Column produces no visible output; Grid reads its slot values from each
// child Column instance and threads them into per-row Cell renderings.
//
// The single empty `Box [column-marker]` is required because moslayout's
// current root-count rule (analyze() in moslayout-compiler) demands
// exactly one root node per layout.  A declaration-only component without
// any rendered tree is a v0.2.0 concern that needs a grammar/resolver
// extension allowing zero-root layouts.

layout Column {
  Box [ column-marker ]
}
