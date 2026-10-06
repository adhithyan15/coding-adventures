// The compact layout (UI48 §7.9): the same interface, stacked. Its widget is
// `LayoutProbeCompact`, and it dispatches the default file's
// `LayoutProbeEvent` rather than declaring its own.
layout LayoutProbe {
  Column [ root ] {
    Text [ heading ] ( content: slot: title )
    HostButton [ pick ] ( label: "Pick", onClick: emit: onPick )
  }
}
