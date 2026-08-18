# Changelog

## 0.1.0

- Add the Ruby port of `ct-compare`, completing the eleven-language set. Ruby
  was the only language missing it, which is why the D18T durable
  epoch-activation profile was hand-rolling its own constant-time loop.
- Add `ct_eq`, `ct_eq_fixed`, `ct_select_bytes`, and `ct_eq_u64`, matching the
  behaviour of the existing C, C++, C#, Elixir, F#, Go, Java, Kotlin, Python,
  Rust, and TypeScript ports.
- No data-dependent branches and no early exits: work depends only on input
  length, never on content. Length is deliberately not hidden, since operand
  lengths are public.
- Accept both byte strings and arrays of byte values, and compare by bytes
  rather than by encoding.
