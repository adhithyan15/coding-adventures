# cowsay (Haskell)

A configurable ASCII cow that speaks or thinks — the Haskell port of
`cowsay`, the fourth program in the repository to render through the
`paint-vm-ascii` backend (after the C#, F#, and Perl ports in this same
rollout).

See [`code/specs/cowsay-paintvm-pipeline.md`](../../../specs/cowsay-paintvm-pipeline.md)
for the full design.

## What it does

Parses its CLI spec from [`code/specs/cowsay.json`](../../../specs/cowsay.json)
via `CliBuilder`, formats a message into a speech or thought bubble above an
ASCII cow loaded from [`code/specs/cows/*.cow`](../../../specs/cows/), and
prints the result — byte-identical to the existing
python/go/rust/typescript/ruby/elixir/csharp/fsharp/perl ports for the same
flags and message.

Instead of printing the composed text directly, it builds a `PaintScene`
(one `PaintGlyphRun` per line, one `PaintGlyphPlacement` per non-space
character) and renders that scene through
`CodingAdventures.PaintVmAscii.render`. This is also the PR that brought
`haskell/paint-instructions` and `haskell/paint-vm-ascii` up to the full
P2D02 contract (rect/line/glyph_run/group/clip/layer) — previously those
packages only supported plain rectangles.

## Dependencies

- `cli-builder`
- `json-value`
- `paint-instructions`
- `paint-vm-ascii`

## Usage

```bash
cabal run cowsay -- "Hello, World!"
cabal run cowsay -- --think -f tux "beep boop"
cabal run cowsay -- -b "resistance is futile"
cabal run cowsay -- -l
```

Flags match `code/specs/cowsay.json`: `-e/--eyes`, `-T/--tongue`,
`-f/--file` (cow name), `-l/--list`, `-n/--nowrap`, `-W/--width`,
`--think`, and the mood shortcuts `-b/--borg`, `-d/--dead`, `-g/--greedy`,
`-p/--paranoid`, `-s/--stoned`, `-t/--tired`, `-w/--wired`, `-y/--youthful`.

## Development

```bash
cabal test all
```

See [lessons.md](../../../../lessons.md) for the Haskell-specific
`parseArgs`-expects-a-program-name-placeholder convention this port relies
on, and the local `cabal test` environment caveat noted there.
