# wasm-module-encoder

`wasm-module-encoder` turns the structured Perl module shape used by the local
Wasm stack into raw `.wasm` bytes.

It is the mirror image of `wasm-module-parser`:

`module hashref -> wasm-module-encoder -> bytes -> wasm-module-parser -> module hashref`

## Example

```perl
use CodingAdventures::WasmModuleEncoder qw(encode_module);

my $bytes = encode_module({
    types => [{ params => [], results => [0x7F] }],
    functions => [0],
    exports => [{ name => 'answer', desc => { kind => 'func', idx => 0 } }],
    codes => [{ locals => [], body => "\x41\x07\x0f\x0b" }],
});
```

## Development

```bash
bash BUILD
```
