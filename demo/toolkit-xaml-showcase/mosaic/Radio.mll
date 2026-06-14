// Radio.mll — layout for the Radio (v0.3 rewrite).
//
// Pre-v0.3 the layout fanned out into a `Row` containing two
// `HostButton`s wrapped in an `If/Else` (one with a `•` glyph, one
// blank) plus a sibling `Text` for the label. That fake-radio
// pattern lost native a11y role, focus ring, group-mutex visual,
// and arrow-key navigation.
//
// v0.3 is a one-line wrapper around the UI29-2 kernel primitive
// `HostRadio`. The native widget owns label wiring and (where the
// platform supports it) the group-mutex behaviour.

layout Radio {
  HostRadio [ radio ] (
    label    : slot: label ,
    checked  : slot: checked ,
    value    : slot: value ,
    group    : slot: group ,
    disabled : slot: disabled ,
    onSelect : emit: onSelect
  )
}
