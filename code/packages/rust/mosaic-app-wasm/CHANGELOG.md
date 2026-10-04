# Changelog

## Unreleased

### Changed — the executable-list parity test reads the shared Compose file (UI89 §3.8)

- The Compose platform library's rules moved into `MosaicFileEffects.kt`,
  the half Compose Desktop and Android share, so the node test that pins
  `EXECUTABLE_EXTENSIONS` to the Kotlin list reads that file, and now fails
  plainly if the list is not found rather than comparing against nothing.

### Changed — device names, active content and non-ASCII extensions (UI87 §3.1)

- `isPlainFileName` refuses Windows device names (`CON`, `NUL.txt`,
  `com1.json`, `COM¹`, `CONIN$`, …), compared on the part before the first
  dot with trailing spaces removed and ASCII letters folded to upper case. New exports:
  `RESERVED_DEVICE_NAMES` and `isReservedDeviceName`.
- `EXECUTABLE_EXTENSIONS` adds documents that run code when opened (web
  pages and SVG, `.mht`, `.website`, Office macro formats with `.xlsb` and
  `.xla`), Python scripts and bytecode, and `.iqy`, `.slk`, `.rdp`; `hasExecutableExtension` also counts any extension with a
  non-ASCII character (a Cyrillic lookalike of `.exe`, or `.exe` with a
  combining mark). The same rules land in every native host library.
- A node test covers each; a Rust test in `mosaic-app-bindings` pins the
  device-name list to the native libraries'.

### Changed — one stricter plain-name rule on every host (UI87 §3.1)

Following the SwiftUI library's security review, `files.save` suggested names
are checked the same way by the Compose and SwiftUI libraries and the browser
executor. Newly refused: a leading `.` (dot-files such as `.zshrc`), line and
paragraph separators (U+2028/U+2029), leading or trailing whitespace of any
kind (a no-break space included), and runs of two or more whitespace
characters. Characters are checked by code point, so a format character
outside the BMP (U+E0001) is caught too. When the app names no accepted type,
a name ending in an extension that runs when opened (`.command`, `.terminal`,
`.webloc`, `.exe`, `.desktop`, … — one shared list) is refused.

- `isPlainFileName` gains the same checks, and `EXECUTABLE_EXTENSIONS` /
  `hasExecutableExtension` are exported. A node test pins the list to the
  Compose library's.
- Security review, round 2: Swift checks the dot rules and the extension by
  scalar (a Prepend letter or combining mark merged with `.` into one
  grapheme hid `run\u{0D4E}.terminal` and `.\u{0301}zshrc` from it);
  surrogates, unassigned and private-use code points are refused everywhere;
  blank-rendering characters count as whitespace; the executable list gains
  macOS location files and installers, Windows launchers, credential-leaking
  shell files and disk images, and Linux packages; extensions are folded
  through upper case (`ſ`).
- Security review, round 3: invisible (default-ignorable) characters are
  dropped before the padding and dot rules, so a variation selector between
  single spaces no longer splits a run, while an emoji's own selector
  (`❤️ list.txt`) is still fine.
- The legacy `file.save` alias refuses a `mimeType`/`extension` pair whose
  extension is on the executable list (the dialog may append it).

### Added — the browser answers the standard `files.*` kinds (UI87 §7)

- `mosaic-file-effects.mjs` answers `files.open` and `files.save`, the kinds
  every Mosaic backend answers, with UI59's `accept` MIME list (the same
  MIME-to-extension table as the Compose library) and `mimeType` in the open
  result. `file.open` / `file.save` stay as aliases with their original results.
- Suggested names follow the shared rule (`isPlainFileName`): no separators,
  `:`, control or format characters, or trailing dot or space; with `accept`
  the name must end in an accepted extension. Checked before any dialog opens.
- Without the File System Access API, `files.save` downloads the bytes and
  reports `ok { name, download: true }`, so an app can say "downloaded" rather
  than claim a durable save. Narrowed after security review, since no dialog
  shows the name: only for a known accepted type with a matching extension
  (never an untyped `.exe`), at most one download per 5 s (one gesture cannot
  start a burst), Blob typed as the accepted MIME type, URL always revoked.
  Names may not contain runs of spaces (the `Invoice.pdf<spaces>.exe`
  disguise). `file.save` keeps its explicit degradation.
- Five new tests (15 total), run in the VisiCalc workflow.
- The type declaration for `createBrowserFileEffects` includes its optional
  `environment` argument (a test passes a fake with the pickers).

### Added — the loader sends the browser's UTC offset

`mosaic-host.mjs` adds `utcOffsetMinutes` (`-getTimezoneOffset()`) to the
default start context. It is left out when implausible, so an app never fails
to start over it, and a caller's explicit context overrides it.

- Add reusable browser file.open/file.save handlers with explicit outcomes,
  gesture-bound pickers, bounded opaque bytes and completion retry without I/O replay.

- Add UI47 protocol-2 effect completion with pending-work checkpoint protection,
  explicit terminal outcomes and shared native/WASM conformance coverage.

## 0.1.0

- Add scalar WASM lifecycle exports and a reusable JavaScript host over MosaicRuntime.
- Validate compiled conformance and VisiCalc adapters through the shared protocol.
