# cowsay (Swift)

A configurable ASCII cow that speaks or thinks — the Swift port of `cowsay`,
the tenth program in the repository to render through the `PaintVmAscii`
backend (after the C#, F#, Perl, Haskell, Java, Kotlin, and Dart ports in
this same rollout).

See [`code/specs/cowsay-paintvm-pipeline.md`](../../../specs/cowsay-paintvm-pipeline.md)
for the full design.

## What it does

Parses its CLI spec from [`code/specs/cowsay.json`](../../../specs/cowsay.json)
via `CliBuilder`, formats a message into a speech or thought bubble above an
ASCII cow loaded from [`code/specs/cows/*.cow`](../../../specs/cows/), and
prints the result — byte-identical to the existing
python/go/rust/typescript/ruby/elixir/csharp/fsharp/perl/haskell/java/kotlin/dart
ports for the same flags and message.

Instead of printing the composed text directly, it builds a `PaintScene`
(one `glyphRun` per line, one `PaintGlyphPlacement` per non-space
character) and renders that scene through `PaintVmAscii`'s `render`. This
is also the PR that built `PaintVmAscii` from scratch, implementing the
full P2D02 contract (rect/line/glyph_run/group/clip/layer) —
`PaintInstructions` already existed but had no ASCII backend, and (unlike
every other language in this rollout) wasn't even a real sum type yet —
see that package's own CHANGELOG.

## Dependencies

- `cli-builder` (`CliBuilder`)
- `PaintInstructions`
- `PaintVmAscii`

## Usage

```sh
swift run Cowsay 'Hello, World!'
swift run Cowsay --think -f tux 'beep boop'
swift run Cowsay -b 'resistance is futile'
swift run Cowsay -l
```

Flags match `code/specs/cowsay.json`: `-e/--eyes`, `-T/--tongue`,
`-f/--file` (cow name), `-l/--list`, `-n/--nowrap`, `-W/--width`,
`--think`, and the mood shortcuts `-b/--borg`, `-d/--dead`, `-g/--greedy`,
`-p/--paranoid`, `-s/--stoned`, `-t/--tired`, `-w/--wired`, `-y/--youthful`.

## A note on argv

Unlike every other port in this rollout, Swift's `CommandLine.arguments`
already includes the executable path at index 0 (the same convention as
C's `argv`/Go's `os.Args`) — matching what `CliBuilder`'s `Parser` expects
directly, with no placeholder to prepend. Kotlin's `args`, Dart's `args`,
and Java's `args` all exclude the program name and need one synthesized;
Swift doesn't.

## Development

```sh
swift test
```
