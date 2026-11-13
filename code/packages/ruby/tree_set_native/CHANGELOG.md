# Changelog

## Unreleased

### Fixed

- Its own `rb_int2inum` declaration takes an `isize` (`intptr_t`) instead of `c_long`, which is 32 bits on 64-bit Windows and returned negative Integers as their unsigned 32-bit value.

## 0.1.0

- Added the native Ruby tree set implementation.
