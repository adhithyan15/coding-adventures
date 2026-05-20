// Cell.mll — layout for one editable spreadsheet cell.
//
// Decomposition (kernel primitives only):
//
//   Box [cell]                       ← stylable container, the only `part`
//     If (when: slot: is-editing)    ← conditional render (UI29 §3.2)
//       HostInput                    ← native single-line text editor
//     Else
//       Text                         ← static read-only display
//
// Why a Box wrapper?  mosstyle scoping: the `cell` part is what
// Cell.dark.msl targets for padding/background/outline.  Without the Box,
// the rendered child (HostInput vs Text) would have to wear those styles
// itself — which leaks Cell-shaped concerns into kernel primitives.
//
// Why `If` + `Else` and not a single `If`?  We must always render
// SOMETHING; an absent `Else` would yield an empty cell while not editing,
// which is the v1 default value's intended display path.
//
// IMPORTANT (UI29-P1, v0.1.0): `If` / `Else` / `HostInput` are kernel
// primitives whose grammar productions and backend lowerings are landing
// in parallel PRs.  This file is parsed by the existing moslayout grammar
// (every tag is a NAME token, so `If`/`Else`/`HostInput` look like any
// other primitive) — so the .mll compiles cleanly.  Whether an emitter
// can actually lower it depends on which U29-K-* PR has landed.

layout Cell {
  Box [ cell ] {
    If ( when: slot: is-editing ) {
      HostInput (
        value:    slot: value ,
        onCommit: emit: onCommit ,
        onCancel: emit: onCancel
      )
    }
    Else {
      Text ( content: slot: value )
    }
  }
}
