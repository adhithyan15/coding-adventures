# CodingAdventures::EcmascriptES1Lexer (Perl)

A grammar-driven ECMAScript 1 (1997) tokenizer. Reads the shared `ecmascript/es1.tokens` grammar file, compiles the token definitions into Perl regexes, and tokenizes ES1 source into a flat list of typed tokens.

## Usage

```perl
use CodingAdventures::EcmascriptES1Lexer;

my $tokens = CodingAdventures::EcmascriptES1Lexer->tokenize('var x = 1;');
for my $tok (@$tokens) {
    printf "%s  %s  (line %d, col %d)\n",
        $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col};
}
```

## Dependencies

- `CodingAdventures::GrammarTools` -- parses `es1.tokens`

## Running tests

```bash
prove -l -v t/
```
