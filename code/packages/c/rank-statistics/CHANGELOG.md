# Changelog

## 0.1.0

- Add ISO C17 finite-sample median and exact distribution-free median interval.
- Add installed CMake package and shared-oracle tests.
- Expand API, allocator ownership, finite-input, and numeric-boundary checks.
- Link the POSIX rank-statistics test with libm after its source inputs so
  `nextafter` resolves under both GCC and Clang.
