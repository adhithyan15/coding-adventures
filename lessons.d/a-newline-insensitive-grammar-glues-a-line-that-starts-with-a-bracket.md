---
category: Compiler / VM / language pipeline
---

# A newline-insensitive grammar glues a line that starts with a bracket onto the expression before it, so the lexer must mark where a newline ends a statement

**What went wrong.** The legacy `ruby.grammar` treats newlines as
whitespace, and `factor` takes any number of `[ … ]` index suffixes.  So

```ruby
x = 5
[1].each { |v| puts v }
```

parsed as `x = 5[1].each { … }`.  The C backend failed with `undefined method
[]` (#18778), and `puts` ⏎ `[7].each { … }` parsed as `puts([7].each { … })`,
which printed the wrong thing on every backend with no error at all.  The
same gluing still happens for a line that starts with `-` or `+`
(`x = 5` ⏎ `-1` reads as `x = 5 - 1`).

**Fix.** Ruby ends a statement at a newline that follows a complete
expression.  `ruby-lexer`'s `tag_line_start_brackets` re-tags such a `[`
(`Newline` token(s) between it and an expression-ending token) with the
value `[ line start`, keeping the `LBracket` type, and the grammar's
`index_suffix` and paren-less-call argument refuse that value with a
negative lookahead.  `array_literal` matches by type, so it still accepts the
token.  The grammar is shared by six languages' parsers, and only Rust's
grammar tool supports adjacency predicates (`@no_newline_before`), so a
lexer tag plus a plain lookahead was the portable choice: the other lexers
never produce the value, so nothing changes for them.

**Do differently.** When a grammar ignores newlines, list every token that
can both start a statement and continue an expression (`[`, `(`, `-`, `+`,
`*`, `/`, `::`), and decide for each where a newline ends the statement.  An
end-to-end corpus should put such a token at the start of a line after an
assignment and after a bare call.
