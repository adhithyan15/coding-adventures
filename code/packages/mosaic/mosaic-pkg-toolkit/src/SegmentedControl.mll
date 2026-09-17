// SegmentedControl.mll — layout for the SegmentedControl.
//
//   Row [ segmented ]
//     For (each: slot: options, as: option, index: i)
//       If (when: i == selectedIndex)
//         HostButton [ segmented-option-selected ] (...)
//       Else
//         HostButton [ segmented-option ] (...)
//
// Why this shape removes TaskApp's 36 parts
// -----------------------------------------
//
// TaskApp draws its switcher as a six-way If/Else chain *around* the
// buttons: each branch re-declares all six buttons, and two parts cannot
// share a name, so every branch needs its own suffixed copy (`seg-list-on`,
// `seg-list-off`, `seg-list-off2`, ... 36 in all). Adding a view is a
// Cartesian expansion.
//
// Here the If/Else sits *inside* the loop and chooses between exactly two
// parts per option. The part count is 2 however many options there are:
// the same pattern Tabs and ListGroup use.
//
// Why `HostButton` and not the toolkit's own `Button`
// ---------------------------------------------------
//
// It could be: a package may place its own components by their qualified
// name (`pkg::mosaic-pkg-toolkit::Button ( variant : "primary" , ... )`),
// which the resolver inlines, and that compiles on every backend. (A bare
// `Button` does NOT work; it is read as the legacy kernel primitive of
// that name, which the pipeline emitters reject.)
//
// What stops it is the accessible name. `Button` has no `a11y-label`
// slot, so an option placed through it would lose the host-supplied name,
// and with it the only way the selected state reaches a screen reader
// today. Keeping the name wins over reuse; moving onto `Button` once it
// can carry a name is tracked separately. The two parts use the same
// property set as the other toolkit buttons so they read as one family.
//
// `selectedIndex`, not `selected-index`
// -------------------------------------
//
// Inside a `when:` or parenthesized expression a slot is referenced by its
// camelCase identifier. The kebab-case spelling compiles but emits
// `selected - index` (a subtraction of two undefined names) on the web
// backends; Notes.mll records where that was found live.
//
// Keyboard: each option is a native button, so Tab traversal, focus rings
// and Enter/Space activation are the platform's own. Arrow-key traversal
// within the group is not expressible in the kernel today; it is tracked
// with the selected-state gap described in SegmentedControl.mil.

layout SegmentedControl {
  Row [ segmented ] {
    For ( each: slot: options , as: option , index: i ) {
      If ( when: i == selectedIndex ) {
        HostButton [ segmented-option-selected ] (
          label : ( option[0] ) ,
          a11y-label : ( option[1] ) ,
          disabled : slot: disabled ,
          onClick : emit: onSelect
        )
      }
      Else {
        HostButton [ segmented-option ] (
          label : ( option[0] ) ,
          a11y-label : ( option[1] ) ,
          disabled : slot: disabled ,
          onClick : emit: onSelect
        )
      }
    }
  }
}
