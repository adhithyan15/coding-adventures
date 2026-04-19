# brainfuck-wasm-compiler

`brainfuck-wasm-compiler` is the Perl orchestration package for Brainfuck's
Wasm lane.

It bundles the whole path:

`Brainfuck source -> parser -> brainfuck-ir-compiler -> ir-to-wasm-compiler -> wasm bytes`

The package returns intermediate artifacts as well as the final binary so
tests and tooling can inspect the whole pipeline.

## Example

```perl
use CodingAdventures::BrainfuckWasmCompiler qw(compile_source);

my $result = compile_source('+++++.');
print length($result->{binary});
```

## Development

```bash
bash BUILD
```
