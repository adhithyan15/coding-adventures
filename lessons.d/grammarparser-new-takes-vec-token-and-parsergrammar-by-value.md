---
category: Compiler / VM / language pipeline
---

# `GrammarParser::new` takes `Vec<Token>` and `ParserGrammar` by value

Do NOT borrow: `GrammarParser::new(&tokens, &grammar)` fails. Use `GrammarParser::new(tokens, grammar)`. If you need the token list after parsing, clone before passing.
