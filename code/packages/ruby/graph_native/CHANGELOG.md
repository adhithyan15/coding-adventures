# Changelog

## Unreleased - complete the Windows import-library symbol list

- `ext/**/build.rs` generates the MSVC import library for Ruby's DLL from a hand-written list of
  `rb_*` symbols. The list is now a superset of every symbol `ruby-bridge` declares (28 functions
  and 4 data symbols), so a symbol the bridge uses can never be missing at link time. Before, a
  crate whose list lagged the bridge failed to link on Windows with `rust-lld: undefined symbol:
  rb_str_new` / `rb_hash_new` / ... (`ruby/graph_native` in the scheduled full build; the same cause
  as the earlier `bitset_native`, `tree_set_native` and `board_vm_native` fixes). Every added name
  is exported by libruby. Validated by `cargo check` on Linux and by checking each name against
  libruby's exports; the Windows link itself is proven by the next daily Windows full build.

## 0.1.0

- Add the Rust-backed Ruby `graph_native` package for DT00 graph.
