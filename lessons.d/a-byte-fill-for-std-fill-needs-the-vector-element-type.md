---
category: Cross-platform & Windows BUILD_windows
---

# A byte fill for `std::fill` needs the vector element type

MSVC `/W4 /WX` rejected the `std::fill` that restores erased page bytes. The
`UINT8_C(0xFF)` macro can still have an integer type, and `std::fill` instantiates
the conversion from that type to the vector's `std::uint8_t` element type.

Use an explicitly typed `std::uint8_t{0xFF}` fill value when erasing the byte
vector. Keep the erased byte unchanged and retain the warning policy. When a
standard-library template copies a fill value into a narrow element type, make
the value's type match the container element type instead of assuming a constant
macro is already typed narrowly enough.
