// Checkbox.mll — layout for the Checkbox (v0.3 rewrite).
//
// Pre-v0.3 the layout fanned out into a `Row` containing two
// `HostButton`s wrapped in an `If/Else` (one with a `✓` glyph, one
// blank) plus a sibling `Text` for the label. That fake-checkbox
// pattern lost native a11y role, focus ring, tri-state, and
// keyboard semantics.
//
// v0.3 is a one-line wrapper: the layout root is the UI29-2 kernel
// primitive `HostCheckbox`. Every backend lowers it to its
// platform's actual checkbox widget. The `label:` slot lives on
// the native widget so the input + label are wired together (DOM
// `<label><input/> body</label>`, SwiftUI `Toggle(label, isOn:)`,
// Qt `CheckBox { text: ... }`, WinUI `<CheckBox Content="..."/>`),
// which means clicking the label toggles the box for free.

layout Checkbox {
  HostCheckbox [ checkbox ] (
    label         : slot: label ,
    checked       : slot: checked ,
    disabled      : slot: disabled ,
    indeterminate : slot: indeterminate ,
    onToggle      : emit: onChange
  )
}
