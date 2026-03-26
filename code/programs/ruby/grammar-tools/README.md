# grammar-tools (Ruby program)

A standalone CLI for validating `.tokens` and `.grammar` files. This program
wraps the `CodingAdventures::GrammarTools` library behind a `cli_builder`-powered
interface.

## Usage

```bash
ruby main.rb validate css.tokens css.grammar
ruby main.rb validate-tokens css.tokens
ruby main.rb validate-grammar css.grammar
ruby main.rb --help
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more validation errors |
| 2 | Usage error |
