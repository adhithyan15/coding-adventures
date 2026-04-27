"""cas-complex: complex number IR support.

Provides ``ImaginaryUnit`` as a pre-bound IR symbol, arithmetic
normalization to rectangular form, and utility heads for complex
decomposition and transformation.
"""
from __future__ import annotations

from cas_complex.constants import IMAGINARY_UNIT
from cas_complex.handlers import (
    IMAGINARY_POWER_HOOK,
    build_complex_handler_table,
)
from cas_complex.normalize import contains_imaginary, normalize_complex, split_rect
from cas_complex.parts import conjugate, im_part, re_part
from cas_complex.polar import arg, polar_form, rect_form
from cas_complex.power import reduce_imaginary_power

__all__ = [
    "IMAGINARY_UNIT",
    "IMAGINARY_POWER_HOOK",
    "build_complex_handler_table",
    "normalize_complex",
    "contains_imaginary",
    "split_rect",
    "re_part",
    "im_part",
    "conjugate",
    "arg",
    "rect_form",
    "polar_form",
    "reduce_imaginary_power",
]
