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
  // The Box's `state-when-*` predicates wire the call-site's
  // `is-selected` / `is-editing` slots into mosstyle's sub-part
  // state mechanism (Task #35 / UI28-1).  When the host (typically
  // Grid) supplies `is-selected: ( r == selectedRow && c == selectedCol )`
  // at the Cell call site, the boolean propagates to this Box and
  // the React / SwiftUI / Qt emitters fold the `cell:selected`
  // mosstyle block into the cell's rendered style attribute.  The
  // `editing` predicate is the same idea — the Cell visually
  // highlights while the host has it promoted to edit mode, in
  // ADDITION to the structural If branch that swaps Text for
  // HostInput.
  Box [ cell ] (
    // Use the `slot:` form (not the `( name )` expression form) so
    // the UI34 package resolver's `rewrite_bindings` step
    // substitutes the call-site's bound value at compile time.
    // With the expression form, identifiers inside the parentheses
    // pass through as raw text and would land in the emitter as
    // invalid JS identifiers (`is-selected` has a hyphen).  The
    // slot-ref form goes through the resolver's existing
    // SlotRef-rewrite path, so the call-site predicate
    // (`r == selectedRow && c == selectedCol`) ends up inlined
    // here verbatim.
    state-when-selected: slot: is-selected ,
    state-when-editing:  slot: is-editing
  ) {
    If ( when: slot: is-editing ) {
      // value: slot: edit-content — the host's live edit buffer,
      // not the underlying cell value.  A controlled HostInput
      // pointed at `slot: value` without an `onChange` companion
      // would freeze (React rejects keystrokes); pairing
      // edit-content with onChange lets the host's reducer track
      // the in-flight edit and the input accepts characters
      // normally.  When the user hits Enter, onCommit carries
      // the final value to the host's persistence layer.
      HostInput (
        value:    slot: edit-content ,
        onChange: emit: onChange ,
        onCommit: emit: onCommit ,
        onCancel: emit: onCancel
      )
    }
    Else {
      Text ( content: slot: value )
    }
  }
}
