# Changelog

## 0.1.0

- add the Lua port of `cowsay`, parsing `code/specs/cowsay.json` via
  `coding_adventures.cli_builder` and rendering `code/specs/cows/*.cow`
  templates
- ninth and last program in the repository to route through
  `paint-vm-ascii` (after csharp, fsharp, perl, haskell, java, kotlin,
  dart, swift): the composed bubble+cow text is converted into a
  PaintScene of `glyph_run` instructions and rendered via
  `coding_adventures.paint_vm_ascii.render` instead of being printed
  directly (see `code/specs/cowsay-paintvm-pipeline.md`)
- support eyes/tongue/cowfile/nowrap/width/think flags and the eight mood
  shortcuts (`-b`, `-d`, `-g`, `-p`, `-s`, `-t`, `-w`, `-y`)
- support `-l`/`--list` (per `cowsay.json`, short-only) to enumerate
  available `.cow` files
- `load_cow` rejects any `-f`/`--file` value containing a path separator
  (forward or back slash) before joining it onto the cows directory,
  falling back to `default.cow` -- structurally equivalent to (and
  simpler than) the C#/F#/Perl pilots' "extract basename, verify resolved
  path stays within root" approach, since Lua's standard library has no
  path-canonicalization primitive to lean on for that approach; rejecting
  separators outright means there's no directory component left for a
  traversal or rooted-path override to hide in
- `find_repo_root` walks up from the running script's own directory (not
  the process's current working directory, which Lua's standard library
  cannot query) looking for the `CLAUDE.md` sentinel file -- the same
  pattern `code/programs/perl/cowsay`'s `find_repo_root` uses, called out
  there as a lesson from a prior, reverted Lua cowsay port's CI pathing
  problems (PR #1535); this port is strictly additive, touching no files
  outside the packages/programs it adds or extends
- `code/packages/lua/paint_instructions` and
  `code/packages/lua/paint_vm_ascii` (both pre-existing but rect-only)
  were extended to the full `P2D02-paint-vm-ascii.md` contract in a
  separate commit ahead of this one -- see those packages' own
  CHANGELOGs
- verified byte-identical to the merged Perl port for eight ASCII flag
  combinations (plain message, `--think -f tux`, `-b`, `-l`, `-W`
  (wrapping), `-d -T` (mode + custom tongue), piped stdin, `-n`
  (nowrap)) and for both path-traversal and absolute-path `-f`/`--file`
  attempts (both fall back to `default.cow` identically in both ports).
  Deliberately does **not** match Perl for non-ASCII input -- see the
  README's "Known, deliberate divergence" section: Perl's port never
  UTF-8-decodes `@ARGV`/`STDIN`, producing mojibake for non-ASCII
  messages, which this port does not reproduce
- note: like the Perl `CliBuilder::Parser`, this Lua
  `coding_adventures.cli_builder` does not expect a leading
  program-name placeholder in argv -- Lua's own `arg` global already
  excludes the program name (`arg[0]` is the script path, `arg[1..n]`
  are the real arguments), so `arg` is passed straight through
