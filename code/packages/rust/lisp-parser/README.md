# lisp-parser

Parses Lisp token streams into S-expression abstract syntax trees (ASTs). This is the second stage of the Lisp compilation pipeline: tokens go in, a tree structure comes out.

## Grammar

Lisp has one of the simplest grammars of any programming language -- just 6 rules:

```
program   = { sexpr } ;
sexpr     = atom | list | quoted ;
atom      = NUMBER | SYMBOL | STRING ;
list      = LPAREN { sexpr } RPAREN ;
quoted    = QUOTE sexpr ;
```

Dotted pairs (`(a . b)`) are supported inside lists.

## AST Structure

The parser produces an `SExpr` enum:

- `SExpr::Atom(AtomKind, String)` -- numbers, symbols, and strings
- `SExpr::List(Vec<SExpr>)` -- parenthesized lists
- `SExpr::DottedPair(Vec<SExpr>, Box<SExpr>)` -- dotted pairs
- `SExpr::Quoted(Box<SExpr>)` -- quoted forms (`'x`)

## Usage

```rust
use lisp_parser::parse;

let program = parse("(+ 1 2)").unwrap();
// program is a Vec<SExpr> with one List entry
```

## How It Fits in the Stack

```
Source --> [lisp-lexer] --> tokens --> [lisp-parser] --> AST --> [lisp-compiler] --> ...
```
