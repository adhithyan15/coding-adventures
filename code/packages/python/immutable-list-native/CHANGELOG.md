# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added
- Initial release of the Python native extension for ImmutableList.
- `ImmutableList()` constructor for empty lists.
- `ImmutableList.from_list(items)` class method to build from Python lists.
- `push(value)` -- append element, returning a new list.
- `get(index)` -- retrieve element with negative index support.
- `set(index, value)` -- replace element, returning a new list.
- `pop()` -- remove last element, returning (new_list, value) tuple.
- `len()` and `__len__` -- element count.
- `is_empty()` -- emptiness check.
- `to_list()` -- convert to plain Python list.
- `__getitem__` -- bracket notation with negative index support.
- `__iter__` -- iteration protocol.
- `__eq__` -- element-wise equality comparison.
- `__repr__` -- human-readable string representation.
- `ImmutableListError` exception for error conditions (e.g., pop on empty).
- Full test suite covering all methods, edge cases, large lists, and
  structural sharing verification.
