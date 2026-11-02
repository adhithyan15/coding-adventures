### Harden SHA-512 corpus readers across twelve existing lanes

- Reconcile the established SHA-512 implementations against a bounded neutral
  HF04 corpus covering byte-exact digests, incremental behavior, ownership,
  finalization, and allocation-free 128-bit length accounting.
- Make every native test loader read at most 4,097 bytes from the real fixture
  path, reject the extra byte before unbounded allocation, and validate the
  entire corpus strictly before decoding any row.
- Preserve stable failure behavior and empty declared authority while adding
  oversized-file regressions for C#, Elixir, F#, Go, Haskell, Lua, Perl,
  Python, Ruby, Rust, Swift, and TypeScript.
