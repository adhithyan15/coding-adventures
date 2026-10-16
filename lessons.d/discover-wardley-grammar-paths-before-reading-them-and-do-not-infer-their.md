---
category: Repo policy / workflow reminders
---

# Discover Wardley grammar paths before reading them and do not infer their parent directory

A diagnostic read guessed code/grammars/wardley.grammar and wardley.tokens and
failed because the resources live under code/grammars/mermaid/. The follow-up
rg --files discovery found the real paths before any edit. Use a discovered path
or a saved manifest's exact file field; a familiar filename does not prove its
directory. Preserve failed helper diagnostics separately from compiler/test results.

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.
