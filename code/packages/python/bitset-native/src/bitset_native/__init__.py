"""
Bitset -- Native Extension (Rust-backed via python-bridge)
==========================================================

A drop-in alternative to ``coding-adventures-bitset`` backed by a Rust
implementation for better performance on large bitsets.

All bit manipulation runs in Rust. Only the method call boundary crosses
between Python and Rust.
"""

# The native .so/.dylib/.pyd is compiled from Rust and placed in this
# directory by the build process. It exports the Bitset class and
# BitsetError exception via PyInit_bitset_native.
from bitset_native.bitset_native import (  # type: ignore[import]
    Bitset,
    BitsetError,
)

__all__ = [
    "Bitset",
    "BitsetError",
]
