# mosaic-pkg-review-actions

A reusable Mosaic component package for Anki-style review actions.

`ReviewActions` owns the target-neutral Undo / Bury / Suspend / Mark action
strip. Apps can consume it without copying the same button layout and event
wiring across HTML, Electron, SwiftUI, XAML, Qt, Compose, and Flutter targets.

The package is intentionally component-scoped. It does not own scheduling,
review sessions, card content, or the collection browser. Hosts provide labels
and route emitted events into their shared business logic core.
