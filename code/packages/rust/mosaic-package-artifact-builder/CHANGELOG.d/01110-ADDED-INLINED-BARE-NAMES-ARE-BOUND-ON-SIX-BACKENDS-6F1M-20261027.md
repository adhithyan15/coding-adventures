### Added — inlined bare names are bound on six backends (UI29 §3.4, 6f1m)

`tests/inlined_bare_names_are_bound.rs` composes a consumer that mounts a
package `Card` and emits it on React, SwiftUI, Compose, Flutter, Web
Components and Qt. The fixture has:

- an own-slot bare name (`content: heading`), which must arrive as the
  bound value;
- a callee loop that shadows that slot inside its body;
- a consumer loop variable `row` bound into a callee loop that also binds
  `row`;
- consumer children spliced into a mount inside that loop.

Each backend's output must hold the renamed callee loop (`rowIn1`) beside
the consumer's `row`, and must not read the callee's slot name. Where `tsc`
is installed, the React output is type-checked against a stub of React's
types. On macOS with `swiftc`, the SwiftUI output is type-checked too. A
missing compiler skips only its check.
