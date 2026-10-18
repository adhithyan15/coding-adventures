---
category: Rust
---

# Pass Rust filename globs to rg filters rather than PowerShell path arguments

During CV02 exploration I twice passed wildcard path arguments to rg, such as
javascript-ast/src/*.rs. PowerShell did not expand them and rg reported an
invalid Windows filename. The search produced no evidence despite an existing
matching source file.

Use rg -g '*.rs' with an existing directory as the path. Its filter performs
matching inside rg on every platform. Keep filters before -- and distinguish a
failed search command from a successful search with no matches.

The Timeline font-family scope audit repeated this with a `src/timeline*` path
even after reading this lesson. Use the existing `src` directory plus
`-g 'timeline*.rs'`; a partial result from other valid paths does not make the
failed multi-path search complete. No constructor inventory was accepted from
the failed call; the corrected directory/filter call supplies that evidence.

The same rule applies to script names: do not pass `code/scripts/closure*` as a
path. Search the real directory with `-g` filters, or locate candidates with
`rg --files code/scripts` first.
