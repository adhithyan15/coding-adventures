# cowsay (Kotlin)

A configurable ASCII cow that speaks or thinks — the Kotlin port of `cowsay`,
the sixth program in the repository to render through the `paint-vm-ascii`
backend (after the C#, F#, Perl, Haskell, and Java ports in this same
rollout).

See [`code/specs/cowsay-paintvm-pipeline.md`](../../../specs/cowsay-paintvm-pipeline.md)
for the full design.

## What it does

Parses its CLI spec from [`code/specs/cowsay.json`](../../../specs/cowsay.json)
via `com.codingadventures.clibuilder`, formats a message into a speech or
thought bubble above an ASCII cow loaded from
[`code/specs/cows/*.cow`](../../../specs/cows/), and prints the result —
byte-identical to the existing
python/go/rust/typescript/ruby/elixir/csharp/fsharp/perl/haskell/java
ports for the same flags and message.

Instead of printing the composed text directly, it builds a `PaintScene`
(one `PaintGlyphRun` per line, one `PaintGlyphPlacement` per non-space
character) and renders that scene through
`com.codingadventures.paintvmascii.render`. This is also the PR that built
`kotlin/paint-vm-ascii` from scratch, implementing the full P2D02 contract
(rect/line/glyph_run/group/clip/layer) — `kotlin/paint-instructions`
already existed but had no ASCII backend before this.

## Dependencies

- `com.codingadventures:cli-builder`
- `com.codingadventures:paint-instructions`
- `com.codingadventures:paint-vm-ascii`

## Usage

```sh
gradle run --args="'Hello, World!'"
gradle run --args="--think -f tux 'beep boop'"
gradle run --args="-b 'resistance is futile'"
gradle run --args="-l"
```

Flags match `code/specs/cowsay.json`: `-e/--eyes`, `-T/--tongue`,
`-f/--file` (cow name), `-l/--list`, `-n/--nowrap`, `-W/--width`,
`--think`, and the mood shortcuts `-b/--borg`, `-d/--dead`, `-g/--greedy`,
`-p/--paranoid`, `-s/--stoned`, `-t/--tired`, `-w/--wired`, `-y/--youthful`.

## Development

```sh
gradle test
```
