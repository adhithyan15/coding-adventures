// The default (wide) layout: the title and its button side by side, which
// Qt lowers to a RowLayout.
layout LayoutProbe {
  Row [ root ] {
    Text [ heading ] ( content: slot: title )
    HostButton [ pick ] ( label: "Pick", onClick: emit: onPick )
  }
}
