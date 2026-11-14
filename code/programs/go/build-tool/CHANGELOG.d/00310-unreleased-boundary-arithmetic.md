## Unreleased - source-input boundary sync (rust/arithmetic)

- The graph-diff conformance test pins the repository source-input boundary digest; it moved from
  `4569b531...` to `f338935e...` because `rust/arithmetic` stopped loading `rust_library.star`
  (#19311 turned its BUILD into plain shell commands), so it left the Starlark rust-library-rule
  consumer list. Test pin only; no tool behaviour change.
