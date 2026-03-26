# grammar-tools (TypeScript program)

A standalone CLI for validating `.tokens` and `.grammar` files. This program
wraps `@coding-adventures/grammar-tools` behind a `@coding-adventures/cli-builder`
interface.

## Usage

```bash
node index.ts validate css.tokens css.grammar
node index.ts validate-tokens css.tokens
node index.ts validate-grammar css.grammar
node index.ts --help
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more validation errors |
| 2 | Usage error |
