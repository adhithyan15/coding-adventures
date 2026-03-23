"""
ImmutableList -- Native Extension (Rust-backed via python-bridge)
=================================================================

A persistent immutable list backed by a Rust implementation using a 32-way
trie with structural sharing. Every "mutation" (push, set, pop) returns a
new list while the original remains unchanged. The new and old lists share
most of their internal memory via Arc reference counting in Rust.

All list operations run in Rust. Only the method call boundary crosses
between Python and Rust.
"""

# The native .so/.dylib/.pyd is compiled from Rust and placed in this
# directory by the build process. It exports the ImmutableList class and
# ImmutableListError exception via PyInit_immutable_list_native.
from immutable_list_native.immutable_list_native import (  # type: ignore[import]
    ImmutableList,
    ImmutableListError,
)

__all__ = [
    "ImmutableList",
    "ImmutableListError",
]
