# CodingAdventures::TypescriptLexer (Perl)

A grammar-driven TypeScript tokenizer. Reads the shared `typescript.tokens` grammar file, compiles the token definitions into Perl regexes, and tokenizes TypeScript source into a flat list of typed tokens.

TypeScript is a strict superset of JavaScript. This lexer recognizes all JavaScript tokens plus TypeScript-specific keywords.

## What it does

Given `interface Foo { x: number; }`, produces:

| type      | value       | line | col |
|-----------|-------------|------|-----|
| INTERFACE | `interface` | 1    | 1   |
| NAME      | `Foo`       | 1    | 11  |
| LBRACE    | `{`         | 1    | 15  |
| NAME      | `x`         | 1    | 17  |
| COLON     | `:`         | 1    | 18  |
| NUMBER    | `number`    | 1    | 20  |
| SEMICOLON | `;`         | 1    | 26  |
| RBRACE    | `}`         | 1    | 28  |
| EOF       |             | 1    | 29  |

Whitespace is consumed silently. The last token is always `EOF`.

## Token types

### Inherited from JavaScript
All JavaScript tokens: NAME, NUMBER (literal), STRING (literal), LET, CONST, VAR, IF, ELSE, WHILE, FOR, DO, FUNCTION, RETURN, CLASS, IMPORT, EXPORT, FROM, AS, NEW, THIS, TYPEOF, INSTANCEOF, TRUE, FALSE, NULL, UNDEFINED, and all operators and delimiters.

### TypeScript-specific keywords
| Token      | Keyword      |
|------------|--------------|
| INTERFACE  | `interface`  |
| TYPE       | `type`       |
| ENUM       | `enum`       |
| NAMESPACE  | `namespace`  |
| DECLARE    | `declare`    |
| READONLY   | `readonly`   |
| PUBLIC     | `public`     |
| PRIVATE    | `private`    |
| PROTECTED  | `protected`  |
| ABSTRACT   | `abstract`   |
| IMPLEMENTS | `implements` |
| EXTENDS    | `extends`    |
| KEYOF      | `keyof`      |
| INFER      | `infer`      |
| NEVER      | `never`      |
| UNKNOWN    | `unknown`    |
| ANY        | `any`        |
| VOID       | `void`       |
| NUMBER     | `number`     |
| STRING     | `string`     |
| BOOLEAN    | `boolean`    |
| OBJECT     | `object`     |
| SYMBOL     | `symbol`     |
| BIGINT     | `bigint`     |

Note: `number`, `string`, etc. are TypeScript reserved type names. They produce keyword tokens (NUMBER, STRING, etc.). A numeric literal `42` also produces NUMBER — distinguish them by value (`"number"` vs `"42"`).

## Usage

```perl
use CodingAdventures::TypescriptLexer;

my $tokens = CodingAdventures::TypescriptLexer->tokenize('interface Foo { x: number }');
for my $tok (@$tokens) {
    printf "%s  %s  (line %d, col %d)\n",
        $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col};
}
```

## How it fits in the stack

```
typescript.tokens  (code/grammars/)
    ↓  parsed by CodingAdventures::GrammarTools
TokenGrammar
    ↓  compiled to Perl qr// rules
CodingAdventures::TypescriptLexer  ← you are here
    ↓  feeds
typescript_parser  (future)
```

## Dependencies

- `CodingAdventures::GrammarTools` — parses `typescript.tokens`
- `CodingAdventures::Lexer` — general-purpose rule-driven lexer (transitive)

## Running tests

```bash
prove -l -v t/
```
