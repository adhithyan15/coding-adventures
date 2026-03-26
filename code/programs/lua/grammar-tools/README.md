# grammar-tools (Lua program)

A standalone CLI for validating `.tokens` and `.grammar` files. This program
wraps `coding_adventures.grammar_tools` (the Lua library package) with a
plain argument-parsing interface (no cli-builder package exists for Lua).

## Usage

```bash
lua main.lua validate css.tokens css.grammar
lua main.lua validate-tokens css.tokens
lua main.lua validate-grammar css.grammar
lua main.lua --help
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more validation errors |
| 2 | Usage error |
