// Card.mll — layout for the Card component.
//
// Decomposition (kernel primitives only — Box, Column, Text):
//
//   Column [ card-root ]            ← vertical stack, the outer themable part
//     Box [ card-title ]            ← the title region (stylable)
//       Text ( content: slot: title )
//     Box [ card-body ]             ← the body region (stylable)
//       Text ( content: slot: body )
//     Box [ card-footer ]           ← the footer region (stylable)
//       Text ( content: slot: footer )
//
// Why a Box around each Text and not just three Texts?
// ----------------------------------------------------
// mosstyle targets `part` names, and parts are attached to layout nodes
// — typically containers, not leaf text nodes.  Wrapping each region in
// a Box gives the .msl four independently styleable parts (root + title
// + body + footer), which is what makes the component themable.  A bare
// Text would force the theme onto the kernel Text primitive itself,
// which leaks Card-shaped concerns into a kernel primitive.
//
// Why Column and not three Boxes stacked manually?
// ------------------------------------------------
// `Column` IS the kernel primitive for "stack vertically" — using it
// directly is preferred over re-deriving vertical stacking from raw
// Box positioning.  Every backend already knows how to lower Column
// (UI29 kernel v1), so we get correct flex/stack behaviour on every
// emitter for free.
//
// Every primitive used here (Box, Column, Text) is part of the UI29
// kernel v1 surface that has shipped in every backend — there are no
// `If` / `For` / `HostTable` / userland refs anywhere in this file.
// That makes Card the cleanest possible proof point that a userland
// package compiles end-to-end TODAY.

layout Card {
  Column [ card-root ] {
    Box [ card-title ] {
      Text ( content: slot: title )
    }
    Box [ card-body ] {
      Text ( content: slot: body )
    }
    Box [ card-footer ] {
      Text ( content: slot: footer )
    }
  }
}
