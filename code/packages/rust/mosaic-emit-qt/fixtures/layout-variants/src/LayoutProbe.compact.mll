// The compact layout (UI48 §7.10): the same interface, stacked, which Qt
// lowers to a ColumnLayout. Its root is the QML type `LayoutProbeCompact`,
// declared in the generated CMakeLists.txt, in `LayoutProbe.compact.qml`.
layout LayoutProbe {
  Column [ root ] {
    Text [ heading ] ( content: slot: title )
    HostButton [ pick ] ( label: "Pick", onClick: emit: onPick )
  }
}
