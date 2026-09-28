### Added — `HostButton` `selected` lowers to `Semantics(selected:)` (UI86, #15420)

- **Where it goes:** `selected : …` adds `selected: …` to the button's
  `Semantics` node, after `enabled`, so the release-lane contract marker before
  it stays contiguous.
- **Buttons with no authored name:** they get the same node, named by their
  visible label, because a bare `Semantics` around an `ElevatedButton` is not
  guaranteed to land on the button's node.
- **Accepted values:** literals, slots, loop bindings and expressions, through
  `_mosaicTruthy`.
- **Predicate:** `host_button_selected_is_native(node)` is the artifact
  builder's check.

**Checked with Flutter 3.47.** A three-option `For` compiled with
`selected : ( i == selectedIndex )` passes `flutter analyze`. A widget test on
the generated widget found:
- only the selected option has `isSelected`;
- all three have `hasSelectedState` and `isButton`;
- a tap dispatches the event and does not change the flag.

