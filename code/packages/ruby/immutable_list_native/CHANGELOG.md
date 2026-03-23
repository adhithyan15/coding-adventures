# Changelog

All notable changes to the `coding_adventures_immutable_list_native` gem will be
documented in this file.

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
