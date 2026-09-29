### Added — every native host sends its UTC offset (UI38 "Local time")

The SwiftUI, Compose, Qt, Flutter and XAML host templates add
`utcOffsetMinutes` to the start context, read from the platform's time zone
API in minutes east of UTC. Apps such as Journal can then file work under the
user's local day. A value outside the runtime's −840..=840 is **left out**, as
the wasm loader does, and the app falls back to UTC. On Unix a custom POSIX
`TZ` string can say anything (`TZ=XYZ-15`), and the runtime would refuse it,
so the app would not start.
The bindings test pins the line for each backend, and all five
effect-completion drivers still build and run.

