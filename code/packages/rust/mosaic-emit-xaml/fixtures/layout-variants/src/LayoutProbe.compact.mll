// The compact layout (UI48 §7.11): the same interface, stacked. Its control
// is `LayoutProbeCompact`, and it raises the default's `LayoutProbeEvent`
// rather than declaring a union of its own.
layout LayoutProbe {
  Column [ root ] {
    Text [ marker ] ( content: "Layout: compact" )
    Text [ status ] ( content: slot: status )
    Text [ platform ] ( content: slot: platform )
  }
}
