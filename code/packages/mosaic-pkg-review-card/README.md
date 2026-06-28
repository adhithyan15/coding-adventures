# mosaic-pkg-review-card

A reusable Mosaic component package for spaced-repetition review cards.

This package owns the target-neutral `ReviewCard` component. Apps such as
Engram can consume it without copying review-card UI across HTML, Electron,
SwiftUI macOS/iOS, XAML, Qt, Compose, and Flutter targets.

The package is intentionally component-scoped: it does not own decks,
scheduling, persistence, navigation, or app shell layout. Those belong in an
app package. The component receives slots and emits review events; the host app
decides how those events flow into its business logic core.
