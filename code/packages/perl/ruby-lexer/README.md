# CodingAdventures::RubyLexer (Perl)

A grammar-driven Ruby tokenizer. Reads the shared `ruby.tokens` grammar file, compiles the token definitions into Perl regexes, and tokenizes Ruby source into a flat list of typed tokens.

## What it does

Given `def greet(name)`, produces:

| type   | value   | line | col |
|--------|---------|------|-----|
| DEF    | `def`   | 1    | 1   |
| NAME   | `greet` | 1    | 5   |
| LPAREN | `(`     | 1    | 10  |
| NAME   | `name`  | 1    | 11  |
| RPAREN | `)`     | 1    | 15  |
| EOF    |         | 1    | 16  |

Whitespace is consumed silently. The last token is always `EOF`.

## Token types

### Literals
| Token  | Example |
|--------|---------|
| NAME   | `my_var`, `_private`, `MyClass` |
| NUMBER | `42`, `0` |
| STRING | `"hello"`, `"a\nb"` |

### Keywords
| Token   | Keyword   |
|---------|-----------|
| DEF     | `def`     |
| END     | `end`     |
| CLASS   | `class`   |
| MODULE  | `module`  |
| IF      | `if`      |
| ELSIF   | `elsif`   |
| ELSE    | `else`    |
| UNLESS  | `unless`  |
| WHILE   | `while`   |
| UNTIL   | `until`   |
| FOR     | `for`     |
| DO      | `do`      |
| RETURN  | `return`  |
| BEGIN   | `begin`   |
| RESCUE  | `rescue`  |
| ENSURE  | `ensure`  |
| REQUIRE | `require` |
| PUTS    | `puts`    |
| YIELD   | `yield`   |
| THEN    | `then`    |
| TRUE    | `true`    |
| FALSE   | `false`   |
| NIL     | `nil`     |
| AND     | `and`     |
| OR      | `or`      |
| NOT     | `not`     |

### Operators
| Token          | Symbol |
|----------------|--------|
| EQUALS_EQUALS  | `==`   |
| DOT_DOT        | `..`   |
| HASH_ROCKET    | `=>`   |
| NOT_EQUALS     | `!=`   |
| LESS_EQUALS    | `<=`   |
| GREATER_EQUALS | `>=`   |
| EQUALS         | `=`    |
| PLUS           | `+`    |
| MINUS          | `-`    |
| STAR           | `*`    |
| SLASH          | `/`    |
| LESS_THAN      | `<`    |
| GREATER_THAN   | `>`    |

### Delimiters
| Token  | Symbol |
|--------|--------|
| LPAREN | `(`    |
| RPAREN | `)`    |
| COMMA  | `,`    |
| COLON  | `:`    |

## Usage

```perl
use CodingAdventures::RubyLexer;

my $tokens = CodingAdventures::RubyLexer->tokenize('def greet(name)');
for my $tok (@$tokens) {
    printf "%s  %s  (line %d, col %d)\n",
        $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col};
}
```

## How it fits in the stack

```
ruby.tokens  (code/grammars/)
    ↓  parsed by CodingAdventures::GrammarTools
TokenGrammar
    ↓  compiled to Perl qr// rules
CodingAdventures::RubyLexer  ← you are here
    ↓  feeds
ruby_parser  (future)
```

## Dependencies

- `CodingAdventures::GrammarTools` — parses `ruby.tokens`
- `CodingAdventures::Lexer` — general-purpose rule-driven lexer (transitive)

## Running tests

```bash
prove -l -v t/
```
