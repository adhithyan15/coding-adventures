# Changelog

All notable changes to the `coding_adventures_immutable_list_native` gem will be
documented in this file.

## Unreleased - complete the Windows import-library symbol list

- `ext/**/build.rs` generates the MSVC import library for Ruby's DLL from a hand-written list of
  `rb_*` symbols. The list is now a superset of every symbol `ruby-bridge` declares (28 functions
  and 4 data symbols), so a symbol the bridge uses can never be missing at link time. Before, a
  crate whose list lagged the bridge failed to link on Windows with `rust-lld: undefined symbol:
  rb_str_new` / `rb_hash_new` / ... (`ruby/graph_native` in the scheduled full build; the same cause
  as the earlier `bitset_native`, `tree_set_native` and `board_vm_native` fixes). Every added name
  is exported by libruby. Validated by `cargo check` on Linux and by checking each name against
  libruby's exports; the Windows link itself is proven by the next daily Windows full build.

## [0.1.0] - 2026-03-23

### Added
- Initial release of the ImmutableList native extension for Ruby
- Wraps the Rust `immutable-list` crate via `ruby-bridge` (no Magnus, no rb-sys)
- `ImmutableList.new` creates an empty persistent list
- `ImmutableList.from_array(arr)` creates a list from a Ruby Array of Strings
- `push(value)` appends an element, returning a new list (original unchanged)
- `get(index)` retrieves an element by index, returns nil if out of bounds
- `set(index, value)` replaces an element, returning a new list (original unchanged)
- `pop` removes the last element, returns `[new_list, removed_value]`
- `size` returns the number of elements
- `empty?` returns true if the list has no elements
- `to_a` converts to a Ruby Array
- `each` yields each element to a block
- `inspect` / `to_s` returns a human-readable string representation
- `==` compares two lists for element-wise equality
- Cross-platform support: Linux, macOS, and Windows (MinGW Ruby + MSVC Rust)
- Comprehensive minitest test suite covering all methods and immutability invariants
