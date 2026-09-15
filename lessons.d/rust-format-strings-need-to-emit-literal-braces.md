---
category: Compiler / VM / language pipeline
---

# Rust format strings need `{{` / `}}` to emit literal braces

`format!("{ ... }")` is a format-string error — `{` inside `"..."` with no closing `}` fails at compile time with "expected `}`, found `\"`". Use `format!("{{...}}")` to produce the literal string `{...}`.
