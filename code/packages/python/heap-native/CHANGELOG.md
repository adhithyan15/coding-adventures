# Changelog

## [0.1.0] - 2026-04-10

### Added

- Initial release of the native heap extension
- `MinHeap` and `MaxHeap` classes wrapping the Rust `heap` crate via `python-bridge`
- Module functions: `heapify`, `heap_sort`, `nlargest`, and `nsmallest`
- Python-object comparison support inside the native heap wrapper
