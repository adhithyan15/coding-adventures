// The default (wide) layout: a marker naming it, and the runtime's status,
// side by side. The resize smoke looks for the marker's text.
layout LayoutProbe {
  Row [ root ] {
    Text [ marker ] ( content: "Layout: default" )
    Text [ status ] ( content: slot: status )
    Text [ platform ] ( content: slot: platform )
  }
}
