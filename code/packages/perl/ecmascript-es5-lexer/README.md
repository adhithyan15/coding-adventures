# CodingAdventures::EcmascriptES5Lexer (Perl)

A grammar-driven ECMAScript 5 (2009) tokenizer. Reads the shared `ecmascript/es5.tokens` grammar file and tokenizes ES5 source into a flat list of typed tokens.

ES5 adds the `debugger` keyword over ES3 and retains all ES3 features.

## Usage

```perl
use CodingAdventures::EcmascriptES5Lexer;

my $tokens = CodingAdventures::EcmascriptES5Lexer->tokenize('var x = 1;');
```

## Dependencies

- `CodingAdventures::GrammarTools` -- parses `es5.tokens`

## Running tests

```bash
prove -l -v t/
```
