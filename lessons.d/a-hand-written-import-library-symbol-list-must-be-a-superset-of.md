---
category: Native extensions & FFI
---

# A hand-written import-library symbol list must be a superset of the bridge's declarations, or a lagging crate fails to link on Windows

On Windows each Ruby native crate generates an MSVC import library for Ruby's DLL from a
hand-written list of `rb_*` symbols in its `build.rs`. The crate links `ruby-bridge`, which
declares the Ruby C API it uses; anything the bridge references and the list omits is an
undefined symbol at link time (`rust-lld: undefined symbol: rb_str_new`). The lists drifted:
`bitset_native`, `tree_set_native` and `board_vm_native` were fixed one at a time, and then
`ruby/graph_native` failed the same way in the next full build, while 12 other crates still had
the same gap (up to 13 symbols behind) and were simply not yet built on Windows.

Do not fix one crate per failure. The invariant is "the list contains every symbol the bridge
declares": derive the bridge's `extern` functions and data (`rb_c*` / `rb_e*`) from
`ruby-bridge/src/lib.rs`, compare each crate's list, and complete all of them in one change. Adding
names is harmless as long as libruby exports them, which can be checked on Linux against
`nm -D libruby.so`. Better still: generate the list from the bridge so it cannot drift.
