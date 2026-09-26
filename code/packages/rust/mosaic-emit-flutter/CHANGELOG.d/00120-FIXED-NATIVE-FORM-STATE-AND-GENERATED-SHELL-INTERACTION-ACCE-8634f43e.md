### Fixed - native form state and generated-shell interaction acceptance

Slot-backed `HostButton.disabled` and `HostInput.read-only` values now reach
Flutter's native `onPressed` and `readOnly` contracts, while `HostInput`
`onCommit` dispatches from `TextField.onSubmitted`. Direct `Row` inputs are
wrapped in `Expanded` so generated toolbars have a finite width and can render.
Generated `MosaicApp` shells also accept an injected `MosaicHost`, allowing
widget tests to exercise initial props, native input, event envelopes, and
host-driven prop refresh without replacing the generated component.

