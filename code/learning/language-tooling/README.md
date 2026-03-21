# Language Tooling

This track covers the path from source text to executable behavior.

Relevant packages include:

- `grammar-tools`
- `lexer`
- `parser`
- `python-lexer`, `ruby-lexer`, `javascript-lexer`, `typescript-lexer`
- `python-parser`, `ruby-parser`, `javascript-parser`, `typescript-parser`
- `bytecode-compiler`
- `virtual-machine`
- `assembler`
- `jit-compiler`

## Why This Track Matters

A lot of programming education jumps straight from:

```text
I wrote some code
```

to:

```text
the machine ran it
```

without dwelling on the layers in between.

This repository is trying to make those layers visible.

## The Main Stages

### Grammars

Grammar files define the legal tokens and structural rules for a language subset.

This lets the repo share language definitions across multiple implementations.

### Lexing

Lexing turns raw text into tokens.

Example:

```text
x = 1 + 2
```

becomes something like:

```text
NAME("x"), EQUALS, NUMBER("1"), PLUS, NUMBER("2")
```

### Parsing

Parsing turns a token stream into structure, usually an abstract syntax tree.

At this point the machine knows that `1 + 2` is an expression rather than just a sequence of characters.

### Lowering Or Translation

After parsing, the repository follows two main execution stories:

- AST -> bytecode -> virtual machine
- AST -> assembly / machine-oriented representation -> ISA simulator

### Virtual Machines

The VM path is especially useful for understanding:

- bytecode design
- stacks
- runtime state
- instruction dispatch

### Assemblers And ISA Paths

The machine-oriented path is useful for understanding:

- instruction encoding
- assembly as a symbolic representation of lower-level behavior
- how software meets architectural state

## Why This Track Connects To Architecture

Language tooling and architecture are often taught separately, but this repository deliberately lets them touch.

That matters because:

- compilers target machine models
- VMs encode execution strategies
- instruction sets influence lowering decisions
- execution engines inherit constraints from the underlying architecture

In other words:

frontend work and architecture are different layers of the same story.

## Recommended Reading Order

1. `code/specs/02-lexer.md`
2. `code/specs/03-parser.md`
3. `code/specs/04-bytecode-compiler.md`
4. `code/specs/05-virtual-machine.md`
5. `code/specs/06-assembler.md`
6. `code/specs/14-grammar-driven-frontends.md`

This learning index is intentionally lightweight for now. The goal of this file is to give the language-tooling packages an explicit home in the learning tree so the educational structure matches the code structure.
