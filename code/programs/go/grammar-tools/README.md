# grammar-tools (Go program)

A standalone binary for validating `.tokens` and `.grammar` files. This program
wraps the `grammar-tools` Go library behind a `cli-builder`-powered interface.

## Usage

```bash
./grammar-tools validate css.tokens css.grammar
./grammar-tools validate-tokens css.tokens
./grammar-tools validate-grammar css.grammar
./grammar-tools --help
```

## Building

```bash
go build -o grammar-tools .
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more validation errors |
| 2 | Usage error |
