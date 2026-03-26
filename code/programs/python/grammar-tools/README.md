# grammar-tools (Python program)

A standalone CLI for validating `.tokens` and `.grammar` files. This program
wraps the `grammar_tools` library behind a `cli_builder`-powered interface.

## Usage

```bash
python main.py validate css.tokens css.grammar
python main.py validate-tokens css.tokens
python main.py validate-grammar css.grammar
python main.py --help
```

## Commands

| Command | Args | Description |
|---------|------|-------------|
| `validate` | `<tokens> <grammar>` | Cross-validate a pair of grammar files |
| `validate-tokens` | `<tokens>` | Validate just a `.tokens` file |
| `validate-grammar` | `<grammar>` | Validate just a `.grammar` file |

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more validation errors |
| 2 | Usage error |

## Where this fits

This program lives in `code/programs/python/grammar-tools/`. The library it
wraps is at `code/packages/python/grammar-tools/`.
