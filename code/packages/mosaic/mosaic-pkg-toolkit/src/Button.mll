// Button.mll — layout for the toolkit Button.
//
// Single-element wrapping the kernel `HostButton`. The Mosaic
// composition is intentionally minimal — every visual difference
// between variants lives in Button.{theme}.msl, not here. That keeps
// the .mll variant-agnostic and the styling backend-portable.
//
// Why not lower variants in the .mll via If/Else?
// -----------------------------------------------
// Could write:
//   If (when: slot: variant == "primary") {
//     HostButton [primary] (...)
//   } Else If ... { ... }
//
// That would produce per-variant XAML/JSX trees, which any backend
// would then de-duplicate via styling anyway. UI49 makes each value
// of a closed `one-of` slot a state owned by that slot. Driving
// variants and sizes through those states keeps this one tree and
// gives every backend the same typed axes.

layout Button {
  HostButton [ button ] (
    label : slot: label ,
    disabled : slot: disabled ,
    onClick : emit: onClick
  )
}
