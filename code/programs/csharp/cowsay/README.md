# cowsay (C#)

A configurable ASCII cow that speaks or thinks — the C# port of `cowsay`, and
the first program in the repository that renders through the `paint-vm-ascii`
backend instead of printing formatted text directly.

See [`code/specs/cowsay-paintvm-pipeline.md`](../../../specs/cowsay-paintvm-pipeline.md)
for the full design: why this port routes through PaintVM-ASCII, how the
composed bubble+cow text becomes a `PaintScene`, and the rollout plan for the
other languages still missing `cowsay`.

## What it does

Parses its CLI spec from [`code/specs/cowsay.json`](../../../specs/cowsay.json)
via `CodingAdventures.CliBuilder`, formats a message into a speech or thought
bubble above an ASCII cow loaded from
[`code/specs/cows/*.cow`](../../../specs/cows/), and prints the result — same
behavior as the existing python/go/rust/typescript/ruby/elixir ports.

The one thing this port does differently: instead of printing the composed
text directly, it builds a `PaintScene` (one `PaintGlyphRun` per line, one
`PaintGlyphPlacement` per non-space character) and renders that scene through
`CodingAdventures.PaintVmAscii.RenderToAscii`. The output is byte-identical to
what a direct print would have produced — this program exists to prove that
round trip holds, not to change cowsay's behavior.

## Dependencies

- `cli-builder`
- `paint-instructions` (transitively, via `paint-vm-ascii`)
- `paint-vm-ascii`

## Usage

```bash
dotnet run --project code/programs/csharp/cowsay -- "Hello, World!"
dotnet run --project code/programs/csharp/cowsay -- --think -f tux "beep boop"
dotnet run --project code/programs/csharp/cowsay -- --borg -f dragon "resistance is futile"
dotnet run --project code/programs/csharp/cowsay -- -l
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
every other C# package/program in this repo.
