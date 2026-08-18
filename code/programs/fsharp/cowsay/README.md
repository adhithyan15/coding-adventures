# cowsay (F#)

A configurable ASCII cow that speaks or thinks — the F# port of `cowsay`,
the second program in the repository to render through the `paint-vm-ascii`
backend (after the C# pilot in this same rollout).

See [`code/specs/cowsay-paintvm-pipeline.md`](../../../specs/cowsay-paintvm-pipeline.md)
for the full design.

## What it does

Parses its CLI spec from [`code/specs/cowsay.json`](../../../specs/cowsay.json)
via `CodingAdventures.CliBuilder.FSharp`, formats a message into a speech or
thought bubble above an ASCII cow loaded from
[`code/specs/cows/*.cow`](../../../specs/cows/), and prints the result —
byte-identical to the existing python/go/rust/typescript/ruby/elixir/csharp
ports for the same flags and message.

Instead of printing the composed text directly, it builds a `PaintScene`
(one `PaintGlyphRun` per line, one `PaintGlyphPlacement` per non-space
character) and renders that scene through
`CodingAdventures.PaintVmAscii.renderToAscii`.

## Dependencies

- `cli-builder`
- `paint-instructions` (transitively, via `paint-vm-ascii`)
- `paint-vm-ascii`

## Usage

```bash
dotnet run --project code/programs/fsharp/cowsay -- "Hello, World!"
dotnet run --project code/programs/fsharp/cowsay -- --think -f tux "beep boop"
dotnet run --project code/programs/fsharp/cowsay -- -b "resistance is futile"
dotnet run --project code/programs/fsharp/cowsay -- -l
```

Flags match `code/specs/cowsay.json`: `-e/--eyes`, `-T/--tongue`,
`-f/--file` (cow name), `-l/--list`, `-n/--nowrap`, `-W/--width`,
`--think`, and the mood shortcuts `-b/--borg`, `-d/--dead`, `-g/--greedy`,
`-p/--paranoid`, `-s/--stoned`, `-t/--tired`, `-w/--wired`, `-y/--youthful`.

## Development

```bash
bash BUILD
```

Runs the test suite with an 80% line-coverage gate via coverlet, same as
every other F# package/program in this repo.
