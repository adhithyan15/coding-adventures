### Added -- Qt is wired into style-drop reporting (#12022)

`styleDegradations` now carries Qt's dropped style properties alongside XAML's,
SwiftUI's and Compose's. Qt derives its drops by running the real emit with
read-recording armed, so unlike the other three it is passed the interface and
the layout, not just the stylesheet.

Across the four Mosaic product packages that is 649 properties that previously
vanished without a word: VisiCalc 89, Venture 16, Engram 188, Trestle 356. No
emitted QML changes.

**Flutter is now the last backend without drop reporting**, and its empty
`styleDegradations` still means "nobody looked" rather than "nothing was lost".
The test pinning that distinction moved its exemplar from Qt to Flutter, and a
new test pins BOTH directions for Qt: `box-shadow` is reported, `color` is not.

