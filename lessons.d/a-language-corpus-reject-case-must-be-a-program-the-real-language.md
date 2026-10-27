---
category: Compiler / VM / language pipeline
---

# A language corpus .reject case must be a program the real language accepts; programs the language refuses belong in lexer or parser tests

**What went wrong.** R1b's plan listed `.reject` corpus cases for an
unterminated `=begin` and an indented `=begin`. Ruby itself refuses both
programs with a SyntaxError. `ruby-iir-compiler`'s
`corpus_expectations_agree_with_the_host_ruby_oracle` runs every corpus case
under host ruby and asserts that each `.reject` case *succeeds* there, so
the test failed on both new cases.

**Why the rule exists.** A `.reject` records "Ruby accepts this program,
but the subset refuses it, for this reason". A program that Ruby refuses
says nothing about the subset's boundary. The other language corpora use
the same convention.

**Fix.** The two programs moved out of the corpus and into the lexer tests
(`ruby-parser/tests/r1b_line_start.rs`). The corpus kept the `.reject`
cases that Ruby runs: a heredoc, and `puts 6 /2`.

**Next time.** Before adding a `.reject` case, run it under the oracle
interpreter and confirm that it exits 0. If the real language refuses the
program, test it where token or parse behavior is checked, not in the
subset corpus.
