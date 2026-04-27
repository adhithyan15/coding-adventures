# nib-wasm-compiler

`nib-wasm-compiler` is Perl's end-to-end Nib to WebAssembly orchestration
package.

It combines the frontend, semantic pass, IR lowering, and Wasm backend into a
single package:

`Nib source -> parser -> type checker -> compiler IR -> Wasm module -> bytes`

## Example

```perl
use CodingAdventures::NibWasmCompiler qw(compile_source);

my $result = compile_source('fn main() -> u4 { return 7; }');
print length($result->{binary});
```

## Development

```bash
bash BUILD
```
