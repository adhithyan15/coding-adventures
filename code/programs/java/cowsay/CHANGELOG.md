# Changelog

## 0.1.0

- add the Java port of `cowsay`, parsing `code/specs/cowsay.json` via
  `cli-builder` and rendering `code/specs/cows/*.cow` templates
- fifth program in the repository to render through `paint-vm-ascii`
  (after the C#, F#, Perl, and Haskell ports): the composed bubble+cow text
  is converted into a `PaintScene` of `PaintGlyphRun` instructions and
  rendered via `PaintVmAscii.render` instead of being printed directly (see
  `code/specs/cowsay-paintvm-pipeline.md`)
- this is also the PR that built `java/paint-vm-ascii` from scratch,
  implementing the full P2D02 contract (rect/line/glyph_run/group/clip/layer)
  — `java/paint-instructions` already existed but had no ASCII backend
- support eyes/tongue/cowfile/nowrap/width/think flags and the eight mood
  shortcuts (`--borg`, `--dead`, `--greedy`, `--paranoid`, `--stoned`,
  `--tired`, `--wired`, `--youthful`)
- support `--list` to enumerate available `.cow` files
- `loadCow` validates the user-supplied `-f`/`--file` flag against path
  traversal and rooted-path overrides before reading a file, mirroring the
  fix applied to every other port's `loadCow` after `/security-review`
- explicitly forces UTF-8 encoding and LF-only output on stdout/stderr
  rather than relying on JVM/platform defaults
