### Added — `[styles] version` gates the mosstyle v2 property table (UI90 §6 step 2)

- `build_package` checks every `.msl` file of each export, theme and
  variant files included, with `mosstyle_compiler::v2::check` before it
  writes anything:
  - A package with `[styles] version = 2` fails on any finding, and the
    error lists all of them.
  - A v1 package builds exactly as before.
- `BuildResult` gains `style_version` and `style_findings`, one
  `src/<file>: <message>` line per finding, sorted. Callers can report a
  v1 package's distance to v2 from these.
- A file that does not parse is skipped here, because the compile step
  reports it with its real error.
- `style_language_version_tests` covers these cases:
  - v1 warns, with and without an explicit `version = 1`;
  - v2 fails before anything is written;
  - clean v2 passes;
  - theme files are checked, and another component's files are not.
