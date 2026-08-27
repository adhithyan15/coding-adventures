# cowsay (Lua)

A configurable ASCII cow that speaks or thinks -- the Lua port of `cowsay`,
the ninth and final language in the cowsay-through-`paint-vm-ascii` rollout
(after csharp, fsharp, perl, haskell, java, kotlin, dart, and swift).

See [`code/specs/cowsay-paintvm-pipeline.md`](../../../specs/cowsay-paintvm-pipeline.md)
for the full design.

## What it does

Parses its CLI spec from [`code/specs/cowsay.json`](../../../specs/cowsay.json)
via `coding_adventures.cli_builder`, formats a message into a speech or
thought bubble above an ASCII cow loaded from
[`code/specs/cows/*.cow`](../../../specs/cows/), and prints the result --
byte-identical to the existing python/go/rust/typescript/ruby/elixir/csharp/
fsharp/perl/haskell/java/kotlin/dart/swift ports for the same flags and
message (verified directly against the merged Perl port for several flag
combinations -- see cowsay.lua's tests).

Instead of printing the composed text directly, it builds a `PaintScene`
(one `glyph_run` instruction per line, one glyph placement per non-space
character) via `coding_adventures.paint_instructions` and renders that scene
through `coding_adventures.paint_vm_ascii`.

Neither `paint_instructions` nor `paint_vm_ascii` were built from scratch
for this PR -- both already existed on `main` (added for other producers,
e.g. `barcode-2d`) but only implemented `rect`. This PR extended both
additively to the full `P2D02-paint-vm-ascii.md` contract
(`rect`/`line`/`glyph_run`/`group`/`clip`/`layer`) -- see those packages' own
CHANGELOGs.

## Known, deliberate divergence from the Perl port

For a message containing non-ASCII characters, this port's output does
**not** byte-match the merged Perl port. This is intentional: Perl's
`cowsay.pl` never decodes `@ARGV` or `STDIN` as UTF-8 (only the *source
file* is parsed as UTF-8, via `use utf8;` -- external input is a different
thing entirely), so `length()`/`sprintf('%-*s', ...)` there operate on raw
UTF-8 *bytes* for actual command-line/stdin input, and printing those bytes
back out through a UTF-8-encoding STDOUT handle re-encodes them, producing
mojibake. This Lua port decodes UTF-8 correctly throughout (see
`cowsay.lua`'s module doc comment) and does not reproduce that bug -- the
byte-identical requirement is about not regressing relative to a working
reference, not about bug-for-bug parity with a reference that is itself
wrong for this input class. Every ASCII flag/message combination checked
during development matched exactly.

## Dependencies

- `coding_adventures.cli_builder`
- `coding_adventures.paint_instructions`
- `coding_adventures.paint_vm_ascii`

Like [`code/programs/lua/parrot`](../parrot/), this program reaches its
sibling packages by adding their `src/` directories to `package.path`
directly (see `main.lua`'s module path setup) rather than depending on a
`luarocks`-installed rock -- the whole monorepo is checked out together, so
there is no need to publish these packages to install them for this
program's own use.

## Usage

```bash
lua main.lua "Hello, World!"
lua main.lua --think -f tux "beep boop"
lua main.lua -b "resistance is futile"
lua main.lua -l
```

Flags match `code/specs/cowsay.json`: `-e/--eyes`, `-T/--tongue`,
`-f/--file` (cow name), `-l` (list), `-n` (nowrap), `-W` (width),
`--think`, and the mood shortcuts `-b`, `-d`, `-g`, `-p`, `-s`, `-t`, `-w`,
`-y` (borg/dead/greedy/paranoid/stoned/tired/wired/youthful -- short-only,
no `--long` forms, matching `cowsay.json`).

## Development

```bash
bash BUILD
```

Runs the test suite via `busted` (from `tests/`, matching
`code/programs/lua/parrot`'s convention). `cowsay.lua`'s tests exercise the
formatting/composition logic directly; `test_main.lua` additionally spawns
`lua ../main.lua` as a real subprocess to cover CLI wiring (argv parsing,
exit codes, help/version/list/error dispatch, the path-traversal-safe
`-f`/`--file` handling) end-to-end.

Lua testing is skipped on Windows CI (see `BUILD_windows`), matching every
other Lua package in this repo, and this program's own subprocess-spawning
`test_main.lua` suite in particular assumes a POSIX shell.
