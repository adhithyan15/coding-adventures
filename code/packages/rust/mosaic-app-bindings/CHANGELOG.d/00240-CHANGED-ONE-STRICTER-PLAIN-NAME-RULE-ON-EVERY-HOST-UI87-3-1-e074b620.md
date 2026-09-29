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

- Kotlin: `mosaicIsPlainFileName`, `MOSAIC_EXECUTABLE_EXTENSIONS`,
  `mosaicHasExecutableExtension`; Swift: the same as `mosaicIsPlainFileName`,
  `mosaicExecutableExtensions`, `mosaicHasExecutableExtension`. A Rust test
  pins the two executable lists together; the Kotlin test and the Swift
  harness check every newly refused name.
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

