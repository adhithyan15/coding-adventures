---
category: Rust
---

# Bind parser signature edits to the owning dialect before replacing text

The Wardley multiline ownership repair used a global replacement of a common
return signature followed by prepare_line_grammar_source. It also changed the
Cynefin and Treeview helper signatures, whose side vectors still contain plain
strings. The first green attempt failed compilation with six type errors;
the independent typed geometric controls compiled and passed two tests.

Restore those two untouched dialect signatures, bind edits to named function
boundaries, and require a complete expected match count before writing. Keep
the compiling semantic red distinct from this later implementation compile
failure. Do not repair unrelated dialect implementations to fit an accidental
signature edit.
