# Versioned F# Grammar System

## Overview

This spec defines a versioned F# lexer/parser grammar set covering F# 1.0
through F# 10. Each version gets its own standalone `.tokens` and `.grammar`
file pair. The archive of formal language specs starts at F# 2.0, but we also
include F# 1.0 as the historical starting point so the file set mirrors the
full language timeline.

The implementation is intentionally conservative: the lexical and syntactic
surface here is the shared core that has remained stable across the language's
major releases. Later versions mostly add RFC-driven features and compiler
behavior changes rather than wholesale grammar rewrites, so the versioned
files stay separate even when the core syntax is the same.

## Why these versions?

- **F# 1.0** - the original release on .NET
- **F# 2.0** - first archived language specification
- **F# 3.0** - the big language update era
- **F# 3.1** - post-3.0 refinement release
- **F# 4.0** - major language/runtime refresh
- **F# 4.1** - spec archive update
- **F# 4.5 / 4.6 / 4.7** - RFC-driven feature waves
- **F# 5 / 6 / 7 / 8 / 9 / 10** - modern .NET releases and current docs

## File Naming Convention

| Version | Tokens File | Grammar File | Notes |
|---------|-------------|--------------|-------|
| 1.0 | `fsharp1.0.tokens` | `fsharp1.0.grammar` | Historical starting point |
| 2.0 | `fsharp2.0.tokens` | `fsharp2.0.grammar` | First archived spec |
| 3.0 | `fsharp3.0.tokens` | `fsharp3.0.grammar` | Major language update |
| 3.1 | `fsharp3.1.tokens` | `fsharp3.1.grammar` | Refinement release |
| 4.0 | `fsharp4.0.tokens` | `fsharp4.0.grammar` | Major refresh |
| 4.1 | `fsharp4.1.tokens` | `fsharp4.1.grammar` | Spec archive update |
| 4.5 | `fsharp4.5.tokens` | `fsharp4.5.grammar` | RFC-era release |
| 4.6 | `fsharp4.6.tokens` | `fsharp4.6.grammar` | RFC-era release |
| 4.7 | `fsharp4.7.tokens` | `fsharp4.7.grammar` | RFC-era release |
| 5 | `fsharp5.tokens` | `fsharp5.grammar` | Modern .NET naming |
| 6 | `fsharp6.tokens` | `fsharp6.grammar` | Modern .NET naming |
| 7 | `fsharp7.tokens` | `fsharp7.grammar` | Modern .NET naming |
| 8 | `fsharp8.tokens` | `fsharp8.grammar` | Current language guide era |
| 9 | `fsharp9.tokens` | `fsharp9.grammar` | Current language guide era |
| 10 | `fsharp10.tokens` | `fsharp10.grammar` | Current release track |

## Magic Comments

Every file includes version metadata:

```text
# F# 10 lexical grammar
# @version 1
# @fsharp_version 10
```

```text
# F# 10 parser grammar
# @version 1
# @fsharp_version 10
```

## Lexical Shape

The token files share the same broad lexical model:

- `NAME` for ordinary identifiers
- `TYPEVAR` for apostrophe-prefixed type variables such as `'"'"'T`
- `NEWLINE` tokens emitted by the shared lexer engine
- `(* ... *)` and `// ...` comments skipped by the lexer
- `[` `< ... >` `]` attribute sections
- F# pipelines, arrows, list-cons, range, and reference operators

## Grammar Shape

The parser files share the same conservative syntax core:

- top-level compilation units made of declarations and newline separators
- attribute sections before bindings
- `let`, `use`, `type`, `module`, `namespace`, `open`, and `do` forms
- a small but useful expression grammar for applications, `if`, `match`,
  `fun`, `function`, `let ... in ...`, `for`, and `while`
- basic patterns, types, records, unions, tuples, lists, and arrays

This is enough to parse the sample programs in the repository and provides a
stable base for expanding the F# surface later.
