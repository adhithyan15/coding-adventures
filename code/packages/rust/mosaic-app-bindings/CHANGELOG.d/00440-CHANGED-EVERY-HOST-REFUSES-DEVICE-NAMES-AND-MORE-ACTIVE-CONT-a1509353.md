### Changed — every host refuses device names and more active-content extensions (UI87 §3.1)

- Every platform library (Compose, SwiftUI, XAML, Qt, Flutter) refuses
  Windows device names as `suggestedName` (`CON`, `NUL.txt`, `com1.json`,
  `COM¹`, `CONIN$`, …): on Windows those are the console, the null device or
  a port, never a file. Compared on the part before the first dot with
  trailing spaces removed and ASCII letters folded to upper case -- only
  those, because .NET's upper-casing leaves `ı` alone where the others make
  it `I`, so `CONıN$` would otherwise pass on XAML alone; one list on every host, the
  browser executor's included, pinned by
  `every_host_refuses_the_same_windows_device_names`.
- The executable list (used when the app names no accepted type) adds
  documents that run code when opened -- web pages and SVG, `.mht`,
  `.website`, the Office macro formats (`.xlsb` and `.xla` included) -- Python
  scripts and bytecode, and shortcuts that fetch or connect (`.iqy`, `.slk`,
  `.rdp`). An extension with
  any non-ASCII character counts as executable too: a Cyrillic lookalike of
  `.exe`, or `.exe` with a combining mark, is an extension no list can name.
- Each harness (Compose test, SwiftUI checks, XAML harness, Flutter harness,
  Qt driver) checks the device names, near misses that still pass, the new
  extensions and ordinary documents. Run locally on every host -- the
  SwiftUI checks with Swift 6.1 on Linux.
- UI87 §3.1 now records why the libraries add no Mark-of-the-Web or
  quarantine attribute themselves, and that one the OS or browser adds is
  left in place. Qt spells the superscript device names as `\u00B9`-style
  escapes, so its source stays ASCII.
