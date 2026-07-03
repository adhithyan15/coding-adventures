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
// would then de-duplicate via styling anyway. Driving variants
// through part_name + .msl is the conventional mosstyle pattern
// (see mosaic-pkg-grid's parts) and is what mosstyle is designed for.

layout Button {
  HostButton [ button ] (
    label : slot: label ,
    disabled : slot: disabled ,
    onClick : emit: onClick
  )
}
