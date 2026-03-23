# Changelog

All notable changes to `@coding-adventures/immutable-list-native` will be documented in this file.

## [0.1.0] - 2026-03-23

### Added
- Initial release of the native ImmutableList Node.js addon.
- Constructor: `new ImmutableList()` for empty lists.
- Constructor: `new ImmutableList(["a", "b", "c"])` for creating from arrays.
- `push(value)` -- append element, returning a new list.
- `get(index)` -- retrieve element by index (returns undefined if out of bounds).
- `set(index, value)` -- replace element at index, returning a new list.
- `pop()` -- remove last element, returning [newList, removedValue].
- `length()` -- return element count.
- `isEmpty()` -- return true if empty.
- `toArray()` -- collect all elements into a plain JS array.
- Comprehensive vitest test suite covering construction, mutation, queries, persistence, large lists, and edge cases.
- BUILD and BUILD_windows scripts for the build tool.
