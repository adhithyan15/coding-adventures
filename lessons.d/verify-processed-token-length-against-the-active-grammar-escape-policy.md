---
category: Rust
---

# Verify processed token length against the active grammar escape policy

The first D1 producer passed five budget unit tests and nine CLI tests, but the
astral-string fixture assumed ES2025 decodes backslash+n. Its generated grammar
sets escapes=none: quote stripping retains those two bytes, so the processed
value has six bytes while the consumed quoted string has eight. The full Origin
comparison already matched the unchanged legacy Token. Preserve the original
101/9P1F receipt, specify the correction first, then assert the actual legacy
value explicitly and correct only the extra numeric literal. Never alter lexer
semantics or raw ranges to satisfy a mistaken expected processed length.

The new ingestion path also made the old tokenizer import unused. Remove that
import before strict lint; a passing non-strict test run is not lint acceptance.

Windows rg does not expand wildcards in positional path arguments. Discover
actual files with rg --files, or use -g with an existing directory, rather than
retrying guessed glob paths or specification filenames.
