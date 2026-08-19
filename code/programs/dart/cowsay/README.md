# cowsay (Dart)

A configurable ASCII cow that speaks or thinks — the Dart port of `cowsay`,
the ninth program in the repository to render through the `paint-vm-ascii`
backend (after the C#, F#, Perl, Haskell, Java, and Kotlin ports in this
same rollout).

See [`code/specs/cowsay-paintvm-pipeline.md`](../../../specs/cowsay-paintvm-pipeline.md)
for the full design.

## What it does

Parses its CLI spec from [`code/specs/cowsay.json`](../../../specs/cowsay.json)
via `coding_adventures_cli_builder`, formats a message into a speech or
thought bubble above an ASCII cow loaded from
[`code/specs/cows/*.cow`](../../../specs/cows/), and prints the result —
byte-identical to the existing
python/go/rust/typescript/ruby/elixir/csharp/fsharp/perl/haskell/java/kotlin
ports for the same flags and message.

Instead of printing the composed text directly, it builds a `PaintScene`
(one `PaintGlyphRun` per line, one `PaintGlyphPlacement` per non-space
character) and renders that scene through
`coding_adventures_paint_vm_ascii`'s `render`. This is also the PR that
built `coding_adventures_paint_vm_ascii` from scratch, implementing the
full P2D02 contract (rect/line/glyph_run/group/clip/layer) —
`coding_adventures_paint_instructions` already existed but had no ASCII
backend before this.

## Dependencies

- `coding_adventures_cli_builder`
- `coding_adventures_paint_instructions`
- `coding_adventures_paint_vm_ascii`

## Usage

```sh
dart run bin/cowsay.dart 'Hello, World!'
dart run bin/cowsay.dart --think -f tux 'beep boop'
dart run bin/cowsay.dart -b 'resistance is futile'
dart run bin/cowsay.dart -l
```

Flags match `code/specs/cowsay.json`: `-e/--eyes`, `-T/--tongue`,
`-f/--file` (cow name), `-l/--list`, `-n/--nowrap`, `-W/--width`,
`--think`, and the mood shortcuts `-b/--borg`, `-d/--dead`, `-g/--greedy`,
`-p/--paranoid`, `-s/--stoned`, `-t/--tired`, `-w/--wired`, `-y/--youthful`.

## Development

```sh
dart pub get
dart test
```
