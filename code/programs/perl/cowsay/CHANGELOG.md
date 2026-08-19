# Changelog

## 0.1.0

- add the Perl port of `cowsay`, parsing `code/specs/cowsay.json` via
  `cli-builder` and rendering `code/specs/cows/*.cow` templates
- third program in the repository to render through `paint-vm-ascii` (after
  the csharp and fsharp ports): the composed bubble+cow text is converted
  into a PaintScene of `glyph_run` instructions and rendered via
  `CodingAdventures::PaintVmAscii->render` instead of being printed directly
  (see `code/specs/cowsay-paintvm-pipeline.md`)
- support eyes/tongue/cowfile/nowrap/width/think flags and the eight mood
  shortcuts (`-b`, `-d`, `-g`, `-p`, `-s`, `-t`, `-w`, `-y`)
- support `--list`/`-l` to enumerate available `.cow` files
- `load_cow` validates the user-supplied `-f`/`--file` flag against path
  traversal and rooted-path overrides before reading a file, mirroring the
  fix applied to the C# and F# pilots after `/security-review`
- note: unlike the C#/F# `CliBuilder` ports, this Perl `CliBuilder::Parser`
  does not expect a leading program-name placeholder in argv — `@ARGV` is
  passed straight through (see the regression test locking this in)
