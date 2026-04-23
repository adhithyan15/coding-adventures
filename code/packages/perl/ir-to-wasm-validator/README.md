# ir-to-wasm-validator

`ir-to-wasm-validator` is the small adapter that turns exceptions from the
lowering pass into a list of diagnostics.

That keeps the higher-level compiler packages on a uniform contract:

- `[]` means the IR can be lowered
- `[ { rule => ..., message => ... } ]` means lowering would fail

## Development

```bash
bash BUILD
```
