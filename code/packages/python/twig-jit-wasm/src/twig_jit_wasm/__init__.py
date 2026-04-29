"""Twig source → JIT-specialised WebAssembly via jit-core + wasm-backend.

See package README for the broader pipeline diagram and motivation.
"""

from __future__ import annotations

from twig_jit_wasm.runner import (
    TwigJITRunner,
    compile_to_iir,
    run_with_jit,
)

__all__ = [
    "TwigJITRunner",
    "compile_to_iir",
    "run_with_jit",
]
