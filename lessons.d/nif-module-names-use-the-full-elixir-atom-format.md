---
category: Elixir
---

# NIF module names use the full Elixir atom format

: `b"Elixir.CodingAdventures.GF256Native\0"`, not `"gf256_native"`. Otherwise Erlang raises `:bad_lib`.
