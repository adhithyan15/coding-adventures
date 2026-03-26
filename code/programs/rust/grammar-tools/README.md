# grammar-tools (Rust program)

A standalone CLI for validating `.tokens` and `.grammar` files. This program
wraps `grammar-tools` (the Rust library package) behind a
`cli-builder` interface.

## Usage

```bash
grammar-tools validate css.tokens css.grammar
grammar-tools validate-tokens css.tokens
grammar-tools validate-grammar css.grammar
grammar-tools --help
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more validation errors |
| 2 | Usage error |
