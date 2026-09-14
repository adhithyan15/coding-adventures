---
category: Elixir
---

# Don't use `:make` compiler in `mix.exs` when BUILD compiles the NIF externally

Mix tries to load `Mix.Tasks.Compile.Make` before `elixir_make` is built from deps and exits non-zero on the very first `mix` command. Just `cargo build --release` from BUILD and copy `.so` into `priv/`.
