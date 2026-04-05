# CodingAdventures::EcmascriptES3Lexer (Perl)

A grammar-driven ECMAScript 3 (1999) tokenizer. Reads the shared `ecmascript/es3.tokens` grammar file and tokenizes ES3 source into a flat list of typed tokens.

ES3 adds over ES1: strict equality (===, !==), try/catch/finally/throw, instanceof, and regex literals.

## Usage

```perl
use CodingAdventures::EcmascriptES3Lexer;

my $tokens = CodingAdventures::EcmascriptES3Lexer->tokenize('var x = 1;');
```

## Dependencies

- `CodingAdventures::GrammarTools` -- parses `es3.tokens`

## Running tests

```bash
prove -l -v t/
```
