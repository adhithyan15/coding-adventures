# ALGOL 60 Parser (Elixir)

Thin wrapper around the grammar-driven parser engine for ALGOL 60 programs.

## Historical Significance

ALGOL 60 holds a unique place in programming language history: it was the
**first language formally specified using BNF** (Backus-Naur Form). John Backus
and Peter Naur published the ALGOL 60 Report in 1960, and the notation they
invented — `<non-terminal> ::= ...` — is still the standard way to describe
programming language grammars today.

Before BNF, language specifications were written in ambiguous English prose.
The ALGOL 60 Report changed that, establishing the idea that a language's syntax
could be a mathematical object, open to rigorous analysis. Every language
standard, RFC, and ISO specification that uses BNF notation is standing on the
foundation Backus and Naur built.

## Usage

```elixir
{:ok, ast} = CodingAdventures.AlgolParser.parse("begin integer x; x := 42 end")
# => %ASTNode{rule_name: "program", children: [
#      %ASTNode{rule_name: "block", children: [...]}
#    ]}
```

## Grammar Structure

The ALGOL 60 grammar is organized in four layers:

1. **Top level** — every program is a `block`
2. **Declarations** — `integer x`, `real y`, `array A[1:10]`, `procedure foo(a);`
3. **Statements** — assignment (`:=`), `if/then/else`, `for/do`, `goto`, procedure call
4. **Expressions** — arithmetic (with `div`, `mod`, `**`), boolean (`and`, `or`, `not`), relational (`<`, `=`, `<=`, etc.)

### Notable Design Decisions

| Feature | ALGOL 60 Choice | Modern Contrast |
|---------|-----------------|-----------------|
| Assignment | `:=` | C uses `=` (confusing with equality) |
| Equality | `=` | C uses `==` |
| Dangling else | Grammar-level resolution via `unlabeled_stmt` | C/Java use convention |
| Exponentiation | Left-associative `2^3^4 = 4096` | Math convention: right-associative |
| Integer division | `div` keyword | C uses `/` (type-dependent) |
| Comments | `comment ... ;` (statement-terminated) | C uses `/* */` |
| Call by name | Default parameter passing | C/Java use call by value |

## Sample Programs

```algol
begin
  integer x;
  x := 42
end
```

```algol
begin
  real pi;
  pi := 3.14159;
  if pi > 3.0 then pi := pi + 0.0
end
```

```algol
begin
  integer x, i;
  for i := 1 step 1 until 10 do
    x := x + i
end
```

## How It Works

1. `AlgolLexer.tokenize/1` converts the source string into a token list.
2. `GrammarParser.parse/2` uses `algol.grammar` to build an AST via recursive descent.
3. Both grammars are cached in `persistent_term` for fast repeated calls.

## Dependencies

- `grammar_tools` — parses `.grammar` files into `ParserGrammar` structs
- `lexer` — grammar-driven tokenization engine
- `parser` — grammar-driven parsing engine
- `algol_lexer` — ALGOL 60 tokenization
- `directed_graph` — transitive dependency of `grammar_tools`
