### Added — `HostScroll` honours its axis (UI61, #14854)

Flutter has no two-axis scroll view, so `both` composes the idiomatic pair: a
vertical `SingleChildScrollView` wrapping a horizontal one. The test asserts
that as real **nesting** — two widgets, one `scrollDirection` — rather than as
a string, because an emitter that merely wrote `scrollDirection:
Axis.horizontal` for `both` would scroll one way only and still satisfy any
assertion that looked for the word.

`vertical` emits a bare `SingleChildScrollView`, Flutter's own default, so all
158 existing tests passed through the change untouched.

