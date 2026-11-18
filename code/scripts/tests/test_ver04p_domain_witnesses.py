"""Bounded exact-rational checks for VER04p §11.1.1."""

from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from ver04p_domain_witnesses import (  # noqa: E402
    dedupe_final_bits,
    f32_value,
    f64_value,
    require_exact_edge_value,
    require_lower_even_tie,
    require_unique_final_bits,
    verify_witnesses,
)


class Ver04pDomainWitnessTests(unittest.TestCase):
    def test_exact_boundary_witnesses(self) -> None:
        results = verify_witnesses()
        self.assertEqual(len(results), 8)

    def test_unrepresentable_integer_sources_are_not_float_edges(self) -> None:
        # Negative control for the original misclassification: these exact
        # integer sources lie halfway between adjacent target encodings.
        with self.assertRaisesRegex(ValueError, "is not the value encoded"):
            require_exact_edge_value(Fraction((1 << 24) + 1), 32, 0x4B800000)
        with self.assertRaisesRegex(ValueError, "is not the value encoded"):
            require_exact_edge_value(
                Fraction((1 << 53) + 1), 64, 0x4340000000000000
            )

    def test_binary64_midpoint_is_not_a_binary64_edge_value(self) -> None:
        midpoint = Fraction((1 << 53) + 1, 1 << 53)
        with self.assertRaisesRegex(ValueError, "is not the value encoded"):
            require_exact_edge_value(midpoint, 64, 0x3FF0000000000000)
        self.assertLess(f64_value(0x3FF0000000000000), midpoint)
        self.assertLess(midpoint, f64_value(0x3FF0000000000001))

    def test_post_encoding_dedup_has_a_duplicate_negative_control(self) -> None:
        # The exact representable value 2^24 and its documented integer
        # conversion tie witness 2^24+1 both finish at the lower F32 encoding.
        # The diagnostic checks this named tie; it does not round arbitrary
        # rational inputs or prove any source/compiler conversion.
        exact_sources = [Fraction(1 << 24), Fraction((1 << 24) + 1)]
        self.assertNotEqual(exact_sources[0], exact_sources[1])
        lower_bits = 0x4B800000
        upper_bits = 0x4B800001
        require_exact_edge_value(exact_sources[0], 32, lower_bits)
        require_lower_even_tie(lower_bits, upper_bits, exact_sources[1], 32)
        duplicate_final_bits = [lower_bits, lower_bits]
        with self.assertRaisesRegex(ValueError, "duplicate final bit pattern"):
            require_unique_final_bits(duplicate_final_bits)
        self.assertEqual(dedupe_final_bits(duplicate_final_bits), [lower_bits])
        require_unique_final_bits(dedupe_final_bits(duplicate_final_bits))

    def test_largest_f32_and_f64_finite_values_stay_exact(self) -> None:
        self.assertEqual(
            f32_value(0x7F7FFFFF), (1 << 128) - (1 << 104)
        )
        self.assertEqual(
            f64_value(0x7FEFFFFFFFFFFFFF), (1 << 1024) - (1 << 971)
        )


if __name__ == "__main__":
    unittest.main()
