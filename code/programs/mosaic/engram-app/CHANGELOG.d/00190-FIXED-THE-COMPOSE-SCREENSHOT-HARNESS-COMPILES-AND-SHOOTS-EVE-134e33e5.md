### Fixed — the Compose screenshot harness compiles and shoots every screen

`conformance/compose/EngramScreenshots.kt` still called `MosaicApp(host)`.
The standard binding has taken the start response as a second argument since
it began rendering from it, so the harness no longer compiled. CI doesn't run
it, so nothing noticed. It now passes `host.props()`, as the Journal harness
does. After `01-decks` it also opens Study, Browse, Add, Stats and Options
through the nav (`02-study` … `06-options`). Each option is matched by the
toolkit SegmentedControl's `segmented-option` tag plus its label, because
"Decks" is also the deck list's label. This is how Compose renders of Engram
get checked, for example #15985's text-box change: only small padding shifts,
nothing broken.

