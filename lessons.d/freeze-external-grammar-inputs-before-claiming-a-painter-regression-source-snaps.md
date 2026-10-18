---
category: Rust
---

# Freeze external grammar inputs before claiming a painter regression source snapshot

The first normal Venn painter regression runner froze the Rust source forest,
both locks and its helper, but omitted Mermaid parser/lexer `include_str!`
inputs under `code/grammars/mermaid`. Its compiled result was a genuine intended
normal-position red with passing direct Scene controls and strict checks;
that result did not establish complete physical input attribution.

Keep the original receipt and its narrower observations unchanged. Inventory
external compile-time includes from the selected source/dependency cone, bind
their complete path membership and bodies before and after every command,
and run a fresh focused tests-only successor before implementation. Pin the
executed toolchain as well. Current clean Git state cannot retrospectively
prove what an omitted external file contained during an earlier execution.
