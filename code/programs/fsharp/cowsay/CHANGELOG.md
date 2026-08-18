# Changelog

## 0.1.0

- add the F# port of `cowsay`, parsing `code/specs/cowsay.json` via
  `cli-builder` and rendering `code/specs/cows/*.cow` templates
- second program in the repository to render through `paint-vm-ascii`
  (after the C# pilot): the composed bubble+cow text is converted into a
  `PaintScene` of `PaintGlyphRun` instructions and rendered via
  `PaintVmAscii.renderToAscii` instead of being printed directly (see
  `code/specs/cowsay-paintvm-pipeline.md`)
- support eyes/tongue/cowfile/nowrap/width/think flags and the eight mood
  shortcuts (`--borg`, `--dead`, `--greedy`, `--paranoid`, `--stoned`,
  `--tired`, `--wired`, `--youthful`)
- support `--list` to enumerate available `.cow` files
- `loadCow` validates the user-supplied `-f`/`--file` flag against path
  traversal and rooted-path overrides before reading a file, mirroring the
  fix applied to the C# pilot after `/security-review`
