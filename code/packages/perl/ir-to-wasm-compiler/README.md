# ir-to-wasm-compiler

`ir-to-wasm-compiler` lowers the repo's generic `compiler-ir` into the plain
Perl WebAssembly module structure used by the local encoder, parser, validator,
and runtime.

## Example

```perl
use CodingAdventures::IrToWasmCompiler qw(compile new_function_signature);

my $module = compile(
    $ir_program,
    [ new_function_signature('_start', 0, '_start') ],
);
```

## Development

```bash
bash BUILD
```
