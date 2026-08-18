# grammar-tools

Pure-Perl grammar-tools implementation.

- `CodingAdventures::GrammarTools` — parses and validates `.tokens`/`.grammar`
  files into `TokenGrammar`/`ParserGrammar` objects.
- `CodingAdventures::GrammarTools::Compiler` — compiles those parsed objects
  into Perl source code that embeds the grammar as a native data structure,
  so downstream lexer/parser packages can `require` a generated module
  instead of reading and parsing a `.tokens`/`.grammar` file at runtime. See
  `code/programs/perl/grammar-tools/` for the CLI that drives this.

## Development

```bash
# Run tests
bash BUILD
```
