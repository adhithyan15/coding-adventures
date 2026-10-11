---
category: Testing & coverage
---

# Bare Ruby call parentheses can consume a grouping fixture

The arithmetic AST mutation matrix used `puts (2)` as a bare-call grouping
fixture. Ruby's parser consumed those parentheses as the call delimiters, so
there was no grouped `factor` to mutate; the fixture lookup failed after the
actual token guard repair. Use `puts 0 + (2)` to force an expression grouping
inside `method_call_no_paren`, and assert the call rule before mutating tokens.
Check the parsed shape as well as source acceptance when a test targets one
particular grammar role for punctuation.
