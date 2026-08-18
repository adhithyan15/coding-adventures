# grammar-tools (Perl program)

A standalone CLI for validating and compiling `.tokens` and `.grammar`
files. This program wraps `CodingAdventures::GrammarTools` and
`CodingAdventures::GrammarTools::Compiler` (the Perl library package) with a
plain `@ARGV`-parsing interface — no cli-builder package exists for Perl.

## Usage

```bash
perl grammar-tools.pl validate ruby.tokens ruby.grammar
perl grammar-tools.pl validate-tokens ruby.tokens
perl grammar-tools.pl validate-grammar ruby.grammar
perl grammar-tools.pl compile-tokens ruby.tokens -o Grammar.pm
perl grammar-tools.pl compile-grammar ruby.grammar -o Grammar.pm
```

## Commands

| Command | Args | Description |
|---------|------|-------------|
| `validate` | `<tokens> <grammar>` | Cross-validate a pair of grammar files |
| `validate-tokens` | `<tokens>` | Validate just a `.tokens` file |
| `validate-grammar` | `<grammar>` | Validate just a `.grammar` file |
| `compile-tokens` | `<tokens> [-o out.pm]` | Compile a `.tokens` file to Perl source embedding a `TokenGrammar` |
| `compile-grammar` | `<grammar> [-o out.pm]` | Compile a `.grammar` file to Perl source embedding a `ParserGrammar` |

The compile commands do not run a validation step before compiling
(matching the Lua port) — there is no `--force` flag because there is
nothing to force past.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed / compilation succeeded |
| 1 | One or more validation errors / compile error |
| 2 | Usage error |

## Where this fits

This program lives in `code/programs/perl/grammar-tools/`. The library it
wraps is at `code/packages/perl/grammar-tools/`. Language ports that read
`.tokens`/`.grammar` files (e.g. `ruby-lexer`, `xml-lexer`) run
`compile-tokens`/`compile-grammar` at dev time and check in the generated
`_Grammar.pm` file, eliminating runtime file I/O and parsing.
