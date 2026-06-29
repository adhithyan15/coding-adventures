# mosaic-pkg-rating-controls

A reusable Mosaic component package for spaced-repetition answer grading.

`RatingControls` owns the target-neutral Again / Hard / Good / Easy control
row. Apps and higher-level review surfaces can consume it without copying the
same button layout and event wiring across HTML, Electron, SwiftUI, XAML, Qt,
Compose, and Flutter targets.

The package is intentionally component-scoped. It does not own scheduling,
review sessions, undo, burying, or card content. Hosts provide labels and route
the emitted review events into their business logic core.
