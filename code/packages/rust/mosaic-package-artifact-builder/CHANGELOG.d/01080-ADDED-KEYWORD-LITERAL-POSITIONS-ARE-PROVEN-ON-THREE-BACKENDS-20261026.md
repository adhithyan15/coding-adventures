### Added — keyword-literal positions are proven on three backends (E6 F1)

`keyword_literal_props_are_literal_on_react_swiftui_and_html` builds every
`moslayout_compiler::KEYWORD_LITERAL_PROPS` entry on React, SwiftUI and HTML.
Each entry sits inside a `For` whose binding has the same name as the
entry's enum value. The test asserts that no backend reads the value as a
variable.

- A control, `HostButton ( label: zqprobe )`, proves the detector sees a
  variable on each of the three.
- A table entry with no fixture fails the test.
- A backend that does not support a primitive (SwiftUI and `Path`) is
  skipped, since it cannot misread one.

The package resolver relies on the table to decide which bare NAMEs in a
dependency must name one of its slots.
