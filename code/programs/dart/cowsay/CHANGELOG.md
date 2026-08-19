# Changelog

## 0.1.0

- add the Dart port of `cowsay`, parsing `code/specs/cowsay.json` via
  `cli-builder` and rendering `code/specs/cows/*.cow` templates
- ninth program in the repository to render through `paint-vm-ascii`
  (after the C#, F#, Perl, Haskell, Java, and Kotlin ports): the composed
  bubble+cow text is converted into a `PaintScene` of `PaintGlyphRun`
  instructions and rendered via `render` instead of being printed directly
  (see `code/specs/cowsay-paintvm-pipeline.md`)
- this is also the PR that built `coding_adventures_paint_vm_ascii` from
  scratch, implementing the full P2D02 contract
  (rect/line/glyph_run/group/clip/layer) —
  `coding_adventures_paint_instructions` already existed but had no ASCII
  backend
- support eyes/tongue/cowfile/nowrap/width/think flags and the eight mood
  shortcuts (`--borg`, `--dead`, `--greedy`, `--paranoid`, `--stoned`,
  `--tired`, `--wired`, `--youthful`)
- support `--list` to enumerate available `.cow` files
- `loadCow` validates the user-supplied `-f`/`--file` flag against path
  traversal and rooted-path overrides before reading a file, mirroring the
  fix applied to every other port's `loadCow` after `/security-review`
- explicitly forces UTF-8 encoding on stdout/stderr and writes output with
  a literal `\n` (never `print`/`writeln`, which translate a trailing
  newline to `Platform.lineTerminator` — CRLF on Windows) for LF-only
  output, applying the JVM ports' encoding/newline lesson from the start
- catches `FileSystemException` in `main` alongside `CliBuilderError` (the
  gap found and fixed via `/security-review` in the Java port), applied
  here from the start
