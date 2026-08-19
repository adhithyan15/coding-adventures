# cowsay (Perl)

A configurable ASCII cow that speaks or thinks — the Perl port of `cowsay`,
the third language in the cowsay-through-`paint-vm-ascii` rollout (after
csharp and fsharp).

See [`code/specs/cowsay-paintvm-pipeline.md`](../../../specs/cowsay-paintvm-pipeline.md)
for the full design.

## What it does

Parses its CLI spec from [`code/specs/cowsay.json`](../../../specs/cowsay.json)
via `CodingAdventures::CliBuilder`, formats a message into a speech or
thought bubble above an ASCII cow loaded from
[`code/specs/cows/*.cow`](../../../specs/cows/), and prints the result —
byte-identical to the existing python/go/rust/typescript/ruby/elixir/csharp/
fsharp ports for the same flags and message.

Instead of printing the composed text directly, it builds a PaintScene (one
`glyph_run` instruction per line, one glyph placement per non-space
character) and renders that scene through `CodingAdventures::PaintVmAscii`.

This PR also brought `perl/paint-vm-ascii` up from a `rect`-only stub to the
full `P2D02-paint-vm-ascii.md` contract (`rect`/`line`/`glyph_run`/`group`/
`clip`/`layer`) — see that package's own CHANGELOG.

## Dependencies

- `cli-builder`
- `paint-instructions`
- `paint-vm-ascii`
- `JSON::PP` (core since Perl 5.14) — decodes `cowsay.json`

## Usage

```bash
perl cowsay.pl "Hello, World!"
perl cowsay.pl --think -f tux "beep boop"
perl cowsay.pl -b "resistance is futile"
perl cowsay.pl -l
```

Flags match `code/specs/cowsay.json`: `-e/--eyes`, `-T/--tongue`,
`-f/--file` (cow name), `-l/--list`, `-n/--nowrap`, `-W/--width`,
`--think`, and the mood shortcuts `-b`, `-d`, `-g`, `-p`, `-s`, `-t`, `-w`,
`-y` (borg/dead/greedy/paranoid/stoned/tired/wired/youthful — none of these
mood flags have `--long` forms in `cowsay.json`, short-only).

## Development

```bash
bash BUILD
```

Installs `Test2::V0` and runs the test suite via `prove`. Perl testing is
skipped on Windows CI (see `BUILD_windows`), matching every other Perl
package in this repo.
