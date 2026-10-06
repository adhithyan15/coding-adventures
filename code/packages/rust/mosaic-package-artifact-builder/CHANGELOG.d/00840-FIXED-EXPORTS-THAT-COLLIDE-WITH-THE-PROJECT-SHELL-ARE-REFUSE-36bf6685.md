### Fixed — exports that collide with the project shell are refused (UI32 §3.7)

- A project build refuses an export that collides with the backend's project
  shell (UI32 §3.7). Before, each of these built without a word and produced
  a broken project:
  - `MosaicApp` on Compose or Flutter, and `MosaicHost` on Qt, declared the
    shell's own type a second time.
  - `App` on SwiftUI and `Main` on Compose lost their file to the shell's
    `App.swift` / `Main.kt`, or replaced it, leaving no entry point.
- Names: `check_exports_against_shell` checks every export against the
  reserved names layout variants are already checked against, before
  anything is written, in project builds only (a flat build has no shell).
- Files: `write_file` refuses to write over a file an export generated, once
  the exports are written. The shells' copies of each export into their
  source sets go through `write_export_mirror`, which also refuses to land on
  a file the shell already wrote and protects the copy. Paths compare without
  regard to case, so `MAIN.kt` and `Main.kt` collide on every OS, as they do
  on macOS and Windows. The deliberate rewrites of an export's own files --
  Qt's strict re-emission of its root, XAML's re-emission of its root's view
  models and helpers -- go through `write_export_file`.
- Checked against every package and fixture in the repository, on Compose,
  Flutter, SwiftUI, Qt and XAML project builds: none collides.
- Two exports whose names differ only in letter case (`Card`, `CARD`) are
  refused before any I/O, on flat builds too: on a case-insensitive
  filesystem each wrote its files over the other's.
- The write recording is cleared by a guard however a build ends, so a build
  that fails part-way leaves no guard behind on its thread; a path that
  exists is keyed by its fully resolved form, so a link to a protected file
  is that file.
- Tests: an export named like a shell type is refused on Compose, Flutter, Qt
  and XAML (and still builds flat); `App` on SwiftUI and `Main` / `MAIN` on
  Compose are refused with the export's own file left as generated.
