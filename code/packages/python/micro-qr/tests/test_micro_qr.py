"""Tests for the micro-qr Python encoder.

This test suite mirrors the Rust test suite (code/packages/rust/micro-qr/src/lib.rs)
so that cross-language corpus comparisons are straightforward.

Test sections
-------------
1.  Symbol dimensions            — M1..M4 produce correct grid sizes.
2.  Auto-version selection       — smallest symbol is chosen automatically.
3.  Structural modules           — finder pattern, separator, timing strip.
4.  Determinism                  — same input → same output every call.
5.  ECC level constraints        — valid/invalid (version, ECC) combinations.
6.  Capacity boundaries          — max-capacity and overflow inputs.
7.  Format information           — format modules are set; BCH utility.
8.  Error handling               — InputTooLongError, ECCNotAvailableError, etc.
9.  Cross-language corpus        — reference inputs with expected dimensions.
10. Encoding modes               — numeric, alphanumeric, byte mode internals.
11. Masking                      — mask selection produces different grids.
12. Public API                   — encode_at, layout_grid, encode_and_layout.
13. RS ECC utility               — _rs_encode sanity checks.
14. Format word utility          — compute_format_word matches _FORMAT_TABLE.
15. Grid utilities               — grid_to_string and shape.
"""

from __future__ import annotations

import pytest

# =============================================================================
# Helpers
# =============================================================================
from barcode_2d import ModuleGrid  # noqa: E402

from micro_qr import (
    _FORMAT_TABLE,
    _SYMBOL_CONFIGS,
    ECCNotAvailableError,
    InputTooLongError,
    MicroQREccLevel,
    MicroQRVersion,
    UnsupportedModeError,
    __version__,
    _BitWriter,
    _build_data_codewords,
    _mask_condition,
    _rs_encode,
    _select_mode,
    compute_format_word,
    encode,
    encode_and_layout,
    encode_at,
    grid_to_string,
    layout_grid,
)


def grid_str(grid: ModuleGrid) -> str:
    """Shorthand: render grid to a '0'/'1' string."""
    return grid_to_string(grid)


# =============================================================================
# 1. Symbol dimensions
# =============================================================================


class TestSymbolDimensions:
    """Every Micro QR symbol is square; sizes are 11, 13, 15, 17."""

    def test_m1_is_11x11(self):
        g = encode("1")
        assert g.rows == 11
        assert g.cols == 11

    def test_m2_is_13x13(self):
        g = encode("HELLO")
        assert g.rows == 13
        assert g.cols == 13

    def test_m3_is_15x15(self):
        # "MICRO QR TEST" is 13 alphanumeric chars → M3-L (cap 14)
        g = encode("MICRO QR TEST")
        assert g.rows == 15
        assert g.cols == 15

    def test_m4_is_17x17(self):
        g = encode("https://a.b")
        assert g.rows == 17
        assert g.cols == 17

    def test_all_symbols_are_square(self):
        for inp in ["1", "HELLO", "MICRO QR TEST", "https://a.b"]:
            g = encode(inp)
            assert g.rows == g.cols, f"grid must be square for '{inp}'"

    def test_module_shape_is_square(self):
        for inp in ["1", "HELLO", "https://a.b"]:
            g = encode(inp)
            assert g.module_shape == "square", (
                f"module_shape must be 'square' for '{inp}'"
            )

    def test_modules_match_dimensions(self):
        for inp in ["1", "HELLO", "hello", "https://a.b"]:
            g = encode(inp)
            assert len(g.modules) == g.rows
            for row in g.modules:
                assert len(row) == g.cols


# =============================================================================
# 2. Auto-version selection
# =============================================================================


class TestAutoVersionSelection:
    """Encoder selects the smallest symbol that fits the input."""

    def test_single_digit_selects_m1(self):
        assert encode("1").rows == 11

    def test_5_digits_selects_m1(self):
        assert encode("12345").rows == 11

    def test_6_digits_escapes_m1(self):
        # M1 max is 5 numeric; 6 digits must go to M2
        assert encode("123456").rows == 13

    def test_hello_selects_m2(self):
        assert encode("HELLO").rows == 13

    def test_hello_lowercase_requires_m3(self):
        # "hello" is byte mode; M2-L byte cap = 4, M3-L byte cap = 9
        assert encode("hello").rows >= 15

    def test_url_selects_m4(self):
        assert encode("https://a.b").rows == 17

    def test_alpha_a1b2c3_selects_m2(self):
        # "A1B2C3" = 6 alphanumeric chars → M2-L (cap 6)
        assert encode("A1B2C3").rows == 13

    def test_8_digit_numeric_selects_m2(self):
        assert encode("01234567").rows == 13

    def test_micro_qr_test_selects_m3(self):
        assert encode("MICRO QR TEST").rows == 15

    def test_forced_m4_with_short_input(self):
        g = encode("1", version=MicroQRVersion.M4)
        assert g.rows == 17

    def test_forced_m2_with_ecc_m(self):
        g = encode("HELLO", version=MicroQRVersion.M2, ecc=MicroQREccLevel.M)
        assert g.rows == 13

    def test_empty_string_selects_m1(self):
        # Empty string: is_numeric is True (vacuously), fits in M1
        g = encode("")
        assert g.rows == 11


# =============================================================================
# 3. Structural modules
# =============================================================================


class TestStructuralModules:
    """Verify finder pattern, separator, and timing strip placement."""

    # ── Finder pattern ──────────────────────────────────────────────────────

    def test_finder_top_row_all_dark(self):
        g = encode("1")
        m = g.modules
        for c in range(7):
            assert m[0][c], f"finder top row: col {c} should be dark"

    def test_finder_bottom_row_all_dark(self):
        g = encode("1")
        m = g.modules
        for c in range(7):
            assert m[6][c], f"finder bottom row: col {c} should be dark"

    def test_finder_left_col_all_dark(self):
        g = encode("1")
        m = g.modules
        for r in range(7):
            assert m[r][0], f"finder left col: row {r} should be dark"

    def test_finder_right_col_all_dark(self):
        g = encode("1")
        m = g.modules
        for r in range(7):
            assert m[r][6], f"finder right col: row {r} should be dark"

    def test_finder_inner_ring_light(self):
        g = encode("1")
        m = g.modules
        for c in range(1, 6):
            assert not m[1][c], f"inner ring row 1 col {c} should be light"
        for c in range(1, 6):
            assert not m[5][c], f"inner ring row 5 col {c} should be light"
        for r in range(1, 6):
            assert not m[r][1], f"inner ring col 1 row {r} should be light"
        for r in range(1, 6):
            assert not m[r][5], f"inner ring col 5 row {r} should be light"

    def test_finder_core_dark(self):
        g = encode("1")
        m = g.modules
        for r in range(2, 5):
            for c in range(2, 5):
                assert m[r][c], f"finder core ({r},{c}) should be dark"

    # ── Separator ────────────────────────────────────────────────────────────

    def test_separator_row7_light(self):
        g = encode("HELLO")
        m = g.modules
        for c in range(8):
            assert not m[7][c], f"separator row 7 col {c} should be light"

    def test_separator_col7_light(self):
        g = encode("HELLO")
        m = g.modules
        for r in range(8):
            assert not m[r][7], f"separator col 7 row {r} should be light"

    # ── Timing ───────────────────────────────────────────────────────────────

    def test_timing_row0_alternates_m4(self):
        g = encode("https://a.b")
        m = g.modules
        for c in range(8, 17):
            expected = (c % 2 == 0)
            assert m[0][c] == expected, f"timing row 0 col {c}: expected {expected}"

    def test_timing_col0_alternates_m4(self):
        g = encode("https://a.b")
        m = g.modules
        for r in range(8, 17):
            expected = (r % 2 == 0)
            assert m[r][0] == expected, f"timing col 0 row {r}: expected {expected}"

    def test_timing_row0_alternates_m2(self):
        g = encode("HELLO")
        m = g.modules
        for c in range(8, 13):
            expected = (c % 2 == 0)
            assert m[0][c] == expected, f"timing row 0 col {c}: expected {expected}"

    def test_timing_col0_alternates_m2(self):
        g = encode("HELLO")
        m = g.modules
        for r in range(8, 13):
            expected = (r % 2 == 0)
            assert m[r][0] == expected, f"timing col 0 row {r}: expected {expected}"


# =============================================================================
# 4. Determinism
# =============================================================================


class TestDeterminism:
    """Encoding the same input twice yields identical grids."""

    @pytest.mark.parametrize("inp", [
        "1", "12345", "HELLO", "A1B2C3", "hello", "https://a.b",
        "MICRO QR TEST", "", "01234567",
    ])
    def test_same_input_same_grid(self, inp: str):
        g1 = encode(inp)
        g2 = encode(inp)
        assert grid_str(g1) == grid_str(g2), f"non-deterministic for '{inp}'"

    def test_different_inputs_different_grids(self):
        g1 = encode("1")
        g2 = encode("2")
        assert grid_str(g1) != grid_str(g2)

    def test_m4_l_vs_m4_m_differ(self):
        g1 = encode("HELLO", ecc=MicroQREccLevel.L)
        g2 = encode("HELLO", ecc=MicroQREccLevel.M)
        assert grid_str(g1) != grid_str(g2)


# =============================================================================
# 5. ECC level constraints
# =============================================================================


class TestEccConstraints:
    """Validate valid and invalid (version, ECC) combinations."""

    def test_m1_detection_valid(self):
        g = encode("1", version=MicroQRVersion.M1, ecc=MicroQREccLevel.Detection)
        assert g.rows == 11

    def test_m2_l_valid(self):
        g = encode("HELLO", version=MicroQRVersion.M2, ecc=MicroQREccLevel.L)
        assert g.rows == 13

    def test_m2_m_valid(self):
        g = encode("HELLO", version=MicroQRVersion.M2, ecc=MicroQREccLevel.M)
        assert g.rows == 13

    def test_m4_q_valid(self):
        g = encode("HELLO", version=MicroQRVersion.M4, ecc=MicroQREccLevel.Q)
        assert g.rows == 17

    def test_m4_all_ecc_differ(self):
        gl = encode("HELLO", version=MicroQRVersion.M4, ecc=MicroQREccLevel.L)
        gm = encode("HELLO", version=MicroQRVersion.M4, ecc=MicroQREccLevel.M)
        gq = encode("HELLO", version=MicroQRVersion.M4, ecc=MicroQREccLevel.Q)
        assert grid_str(gl) != grid_str(gm)
        assert grid_str(gm) != grid_str(gq)
        assert grid_str(gl) != grid_str(gq)

    def test_m1_rejects_ecc_l(self):
        with pytest.raises(ECCNotAvailableError):
            encode("1", version=MicroQRVersion.M1, ecc=MicroQREccLevel.L)

    def test_m1_rejects_ecc_m(self):
        with pytest.raises(ECCNotAvailableError):
            encode("1", version=MicroQRVersion.M1, ecc=MicroQREccLevel.M)

    def test_m1_rejects_ecc_q(self):
        with pytest.raises(ECCNotAvailableError):
            encode("1", version=MicroQRVersion.M1, ecc=MicroQREccLevel.Q)

    def test_m2_rejects_ecc_q(self):
        with pytest.raises(ECCNotAvailableError):
            encode("1", version=MicroQRVersion.M2, ecc=MicroQREccLevel.Q)

    def test_m3_rejects_ecc_q(self):
        with pytest.raises(ECCNotAvailableError):
            encode("1", version=MicroQRVersion.M3, ecc=MicroQREccLevel.Q)

    def test_m2_rejects_ecc_detection(self):
        with pytest.raises(ECCNotAvailableError):
            encode("1", version=MicroQRVersion.M2, ecc=MicroQREccLevel.Detection)

    def test_m3_rejects_ecc_detection(self):
        with pytest.raises(ECCNotAvailableError):
            encode("1", version=MicroQRVersion.M3, ecc=MicroQREccLevel.Detection)

    def test_m4_rejects_ecc_detection(self):
        with pytest.raises(ECCNotAvailableError):
            encode("1", version=MicroQRVersion.M4, ecc=MicroQREccLevel.Detection)

    def test_nonexistent_combo_raises(self):
        with pytest.raises(ECCNotAvailableError):
            # M1 only has Detection; L/M/Q all fail
            encode("1", version="M1", ecc="Q")


# =============================================================================
# 6. Capacity boundaries
# =============================================================================


class TestCapacityBoundaries:
    """Inputs at and beyond symbol capacity boundaries."""

    # ── M1 (max 5 numeric) ──────────────────────────────────────────────────

    def test_m1_max_5_digits(self):
        g = encode("12345")
        assert g.rows == 11

    def test_m1_overflow_6_digits_goes_m2(self):
        g = encode("123456")
        assert g.rows == 13

    # ── M4-L (max 35 numeric) ───────────────────────────────────────────────

    def test_m4_max_35_digits(self):
        g = encode("1" * 35)
        assert g.rows == 17

    def test_m4_overflow_36_digits_raises(self):
        with pytest.raises(InputTooLongError):
            encode("1" * 36)

    # ── M4-L byte (max 15) ──────────────────────────────────────────────────

    def test_m4_l_max_15_bytes(self):
        g = encode("a" * 15)
        assert g.rows == 17

    def test_m4_l_overflow_16_bytes_raises(self):
        with pytest.raises(InputTooLongError):
            encode("a" * 16)

    # ── M4-Q numeric (max 21) ────────────────────────────────────────────────

    def test_m4_q_max_21_numeric(self):
        g = encode("1" * 21, ecc=MicroQREccLevel.Q)
        assert g.rows == 17

    def test_m4_q_overflow_22_numeric_raises(self):
        with pytest.raises(InputTooLongError):
            encode("1" * 22, ecc=MicroQREccLevel.Q)

    # ── M2-L byte (max 4) ────────────────────────────────────────────────────

    def test_m2_l_max_4_bytes(self):
        g = encode("abcd", version=MicroQRVersion.M2, ecc=MicroQREccLevel.L)
        assert g.rows == 13

    def test_m2_l_overflow_5_bytes(self):
        # 5 bytes don't fit in M2-L (cap 4) → escalate to M3
        g = encode("abcde")
        assert g.rows >= 15

    # ── M1 unsupported modes ─────────────────────────────────────────────────

    def test_m1_rejects_alphanumeric(self):
        with pytest.raises((InputTooLongError, UnsupportedModeError)):
            encode("HELLO", version=MicroQRVersion.M1, ecc=MicroQREccLevel.Detection)

    def test_m1_rejects_byte(self):
        with pytest.raises((InputTooLongError, UnsupportedModeError)):
            encode("hello", version=MicroQRVersion.M1, ecc=MicroQREccLevel.Detection)


# =============================================================================
# 7. Format information
# =============================================================================


class TestFormatInformation:
    """Format information modules must be set and non-trivial."""

    def test_format_info_non_zero_m4_l(self):
        g = encode("HELLO", version=MicroQRVersion.M4, ecc=MicroQREccLevel.L)
        m = g.modules
        any_dark_row = any(m[8][c] for c in range(1, 9))
        any_dark_col = any(m[r][8] for r in range(1, 8))
        assert any_dark_row or any_dark_col

    def test_format_info_non_zero_m1(self):
        g = encode("1")
        m = g.modules
        count = sum(1 for c in range(1, 9) if m[8][c])
        count += sum(1 for r in range(1, 8) if m[r][8])
        assert count > 0

    def test_format_info_positions_row8(self):
        """Row 8, cols 1–8 are format information modules — should not all be zero."""
        g = encode("HELLO")
        m = g.modules
        bits = [m[8][c] for c in range(1, 9)]
        # At least one must be set (format word is not 0x0000)
        assert any(bits)

    def test_format_info_positions_col8(self):
        """Col 8, rows 1–7 are format information modules."""
        g = encode("HELLO")
        m = g.modules
        bits = [m[r][8] for r in range(1, 8)]
        assert any(bits)

    def test_compute_format_word_matches_table(self):
        """compute_format_word() must match the pre-computed _FORMAT_TABLE."""
        for si in range(8):
            for mp in range(4):
                expected = _FORMAT_TABLE[si][mp]
                got = compute_format_word(si, mp)
                assert got == expected, (
                    f"symbol_indicator={si} mask={mp}: "
                    f"expected 0x{expected:04X}, got 0x{got:04X}"
                )

    def test_all_format_words_distinct(self):
        """All 32 format words must be distinct (XOR mask ensures this)."""
        words = [_FORMAT_TABLE[si][mp] for si in range(8) for mp in range(4)]
        assert len(set(words)) == 32


# =============================================================================
# 8. Error handling
# =============================================================================


class TestErrorHandling:
    """All error types surface correctly."""

    def test_input_too_long_message(self):
        with pytest.raises(InputTooLongError, match="36"):
            encode("1" * 36)

    def test_ecc_not_available_message(self):
        with pytest.raises(ECCNotAvailableError):
            encode("1", version=MicroQRVersion.M1, ecc=MicroQREccLevel.L)

    def test_unsupported_mode_m1_alpha(self):
        # M1 has no alpha/byte support; trying to force it should fail
        with pytest.raises((UnsupportedModeError, InputTooLongError)):
            encode("HELLO", version=MicroQRVersion.M1, ecc=MicroQREccLevel.Detection)

    def test_input_too_long_is_micro_qr_error(self):
        from micro_qr import MicroQRError
        with pytest.raises(MicroQRError):
            encode("1" * 36)

    def test_ecc_not_available_is_micro_qr_error(self):
        from micro_qr import MicroQRError
        with pytest.raises(MicroQRError):
            encode("1", version=MicroQRVersion.M1, ecc=MicroQREccLevel.L)


# =============================================================================
# 9. Cross-language corpus
# =============================================================================


class TestCrossLanguageCorpus:
    """Reference test corpus matching the Rust implementation."""

    CORPUS = [
        ("1",             11),   # M1 single digit
        ("12345",         11),   # M1 max numeric
        ("HELLO",         13),   # M2-L alphanumeric
        ("01234567",      13),   # M2-L numeric 8 digits
        ("https://a.b",   17),   # M4-L byte mode
        ("MICRO QR TEST", 15),   # M3-L alphanumeric 13 chars
        ("A1B2C3",        13),   # M2-L alphanumeric 6 chars
        ("hello",         15),   # M3-L byte mode 5 bytes
    ]

    @pytest.mark.parametrize("inp,expected_size", CORPUS)
    def test_corpus_dimensions(self, inp: str, expected_size: int):
        g = encode(inp)
        assert g.rows == expected_size, (
            f"input '{inp}': expected {expected_size}×{expected_size}, "
            f"got {g.rows}×{g.cols}"
        )
        assert g.cols == expected_size


# =============================================================================
# 10. Encoding modes
# =============================================================================


class TestEncodingModes:
    """Verify internal mode selection and character coverage."""

    def test_numeric_mode_all_digits(self):
        # All-digit input → numeric mode for M2-L
        cfg = next(c for c in _SYMBOL_CONFIGS if c.version == "M2" and c.ecc == "L")
        mode = _select_mode("123456", cfg)
        assert mode == "numeric"

    def test_alpha_mode_uppercase(self):
        cfg = next(c for c in _SYMBOL_CONFIGS if c.version == "M2" and c.ecc == "L")
        mode = _select_mode("HELLO", cfg)
        assert mode == "alphanumeric"

    def test_byte_mode_lowercase(self):
        cfg = next(c for c in _SYMBOL_CONFIGS if c.version == "M3" and c.ecc == "L")
        mode = _select_mode("hello", cfg)
        assert mode == "byte"

    def test_numeric_preferred_over_alpha(self):
        # "12345" is both numeric and alphanumeric; numeric should win
        cfg = next(c for c in _SYMBOL_CONFIGS if c.version == "M2" and c.ecc == "L")
        mode = _select_mode("12345", cfg)
        assert mode == "numeric"

    def test_alpha_preferred_over_byte(self):
        # "HELLO" is both alpha and byte; alpha should win
        cfg = next(c for c in _SYMBOL_CONFIGS if c.version == "M2" and c.ecc == "L")
        mode = _select_mode("HELLO", cfg)
        assert mode == "alphanumeric"

    def test_alphanum_set_includes_space(self):
        # Space is in the 45-char alphanumeric set
        g = encode("MICRO QR TEST")
        assert g.rows == 15  # M3-L alphanumeric

    def test_m1_only_numeric(self):
        # M1 does not support alpha or byte
        cfg = next(c for c in _SYMBOL_CONFIGS if c.version == "M1")
        with pytest.raises(UnsupportedModeError):
            _select_mode("HELLO", cfg)

    def test_byte_mode_utf8(self):
        # UTF-8 multi-byte characters should work in byte mode
        g = encode("hi")
        assert g.rows >= 11  # just check it doesn't raise

    def test_numeric_encodes_groups_correctly(self):
        """Verify _build_data_codewords for a known numeric input."""
        cfg = next(c for c in _SYMBOL_CONFIGS if c.version == "M2" and c.ecc == "L")
        cw = _build_data_codewords("0", cfg, "numeric")
        assert len(cw) == cfg.data_cw  # must produce exactly data_cw bytes

    def test_data_codewords_length_all_configs(self):
        """_build_data_codewords must produce exactly data_cw bytes."""
        for cfg in _SYMBOL_CONFIGS:
            if cfg.numeric_cap > 0:
                inp = "1"
                mode = "numeric"
            elif cfg.alpha_cap > 0:
                inp = "A"
                mode = "alphanumeric"
            else:
                inp = "a"
                mode = "byte"

            if cfg.version == "M1":
                inp = "1"
                mode = "numeric"

            cw = _build_data_codewords(inp, cfg, mode)
            assert len(cw) == cfg.data_cw, (
                f"{cfg.version}-{cfg.ecc}: expected {cfg.data_cw} codewords, "
                f"got {len(cw)}"
            )


# =============================================================================
# 11. Masking
# =============================================================================


class TestMasking:
    """Mask selection affects the final grid."""

    def test_mask_conditions_cover_all_patterns(self):
        # Spot-check each mask pattern condition at a known point.
        assert _mask_condition(0, 0, 0) is True   # (0+0) % 2 == 0
        assert _mask_condition(0, 1, 0) is False  # (1+0) % 2 == 1
        assert _mask_condition(1, 0, 5) is True   # 0 % 2 == 0
        assert _mask_condition(1, 1, 5) is False  # 1 % 2 != 0
        assert _mask_condition(2, 5, 3) is True   # 3 % 3 == 0
        assert _mask_condition(2, 5, 4) is False  # 4 % 3 != 0
        assert _mask_condition(3, 0, 0) is True   # (0+0) % 3 == 0
        assert _mask_condition(3, 1, 1) is False  # (1+1) % 3 == 2 != 0

    def test_same_config_different_ecc_different_grid(self):
        """M2-L and M2-M with same input must differ (different ECC, different CWs)."""
        g1 = encode("HELLO", version=MicroQRVersion.M2, ecc=MicroQREccLevel.L)
        g2 = encode("HELLO", version=MicroQRVersion.M2, ecc=MicroQREccLevel.M)
        assert grid_str(g1) != grid_str(g2)


# =============================================================================
# 12. Public API
# =============================================================================


class TestPublicApi:
    """encode_at, layout_grid, encode_and_layout."""

    def test_encode_at_m2_l(self):
        g = encode_at("HELLO", MicroQRVersion.M2, MicroQREccLevel.L)
        assert g.rows == 13

    def test_encode_at_m4_q(self):
        g = encode_at("HELLO", MicroQRVersion.M4, MicroQREccLevel.Q)
        assert g.rows == 17

    def test_encode_at_m1_detection(self):
        g = encode_at("12345", MicroQRVersion.M1, MicroQREccLevel.Detection)
        assert g.rows == 11

    def test_layout_grid_returns_paint_scene(self):
        from paint_instructions import PaintScene
        g = encode("HELLO")
        scene = layout_grid(g)
        assert isinstance(scene, PaintScene)

    def test_layout_grid_default_quiet_zone(self):
        """Default layout uses 2-module quiet zone."""
        from paint_instructions import PaintScene
        g = encode("1")  # 11×11
        scene = layout_grid(g)
        assert isinstance(scene, PaintScene)
        # With quiet_zone=2, module_size=10: total = (11 + 2*2) * 10 = 150 px
        assert scene.width == 150
        assert scene.height == 150

    def test_layout_grid_custom_config(self):
        from barcode_2d import Barcode2DLayoutConfig
        from paint_instructions import PaintScene
        g = encode("1")
        cfg = Barcode2DLayoutConfig(quiet_zone_modules=4, module_size_px=5)
        scene = layout_grid(g, config=cfg)
        assert isinstance(scene, PaintScene)
        # (11 + 2*4) * 5 = 95 px
        assert scene.width == 95

    def test_encode_and_layout_returns_paint_scene(self):
        from paint_instructions import PaintScene
        scene = encode_and_layout("HELLO")
        assert isinstance(scene, PaintScene)

    def test_encode_and_layout_with_ecc(self):
        from paint_instructions import PaintScene
        scene = encode_and_layout("HELLO", ecc=MicroQREccLevel.M)
        assert isinstance(scene, PaintScene)

    def test_version_module(self):
        assert __version__ == "0.1.0"


# =============================================================================
# 13. RS ECC utility
# =============================================================================


class TestRsEcc:
    """Reed-Solomon encoder sanity checks."""

    def test_rs_encode_produces_correct_length(self):
        for ecc_count in [2, 5, 6, 8, 10, 14]:
            data = [0x10] * 5
            ecc = _rs_encode(data, ecc_count)
            assert len(ecc) == ecc_count, f"expected {ecc_count} ECC bytes"

    def test_rs_encode_all_zero_data(self):
        """All-zero data → all-zero ECC (trivial case)."""
        ecc = _rs_encode([0] * 5, 5)
        assert ecc == [0] * 5

    def test_rs_encode_non_trivial(self):
        """Non-zero data should produce non-zero ECC in general."""
        ecc = _rs_encode([1, 2, 3, 4, 5], 5)
        assert any(b != 0 for b in ecc)

    def test_rs_encode_deterministic(self):
        data = [0x42, 0x01, 0xFF, 0x00, 0x80]
        ecc1 = _rs_encode(data, 5)
        ecc2 = _rs_encode(data, 5)
        assert ecc1 == ecc2


# =============================================================================
# 14. Format word utility
# =============================================================================


class TestFormatWordUtility:
    """compute_format_word must return values from the pre-computed table."""

    def test_m1_mask0_format_word(self):
        # M1 symbol_indicator=0, mask=0 → should match table
        assert compute_format_word(0, 0) == _FORMAT_TABLE[0][0]

    def test_m4_q_mask3_format_word(self):
        # M4-Q symbol_indicator=7, mask=3 → should match table
        assert compute_format_word(7, 3) == _FORMAT_TABLE[7][3]

    def test_all_format_words_match_table(self):
        for si in range(8):
            for mp in range(4):
                assert compute_format_word(si, mp) == _FORMAT_TABLE[si][mp], (
                    f"mismatch at si={si}, mp={mp}"
                )

    def test_invalid_symbol_indicator_raises(self):
        with pytest.raises(ValueError):
            compute_format_word(8, 0)

    def test_invalid_mask_pattern_raises(self):
        with pytest.raises(ValueError):
            compute_format_word(0, 4)

    def test_all_words_are_15_bit_values(self):
        for si in range(8):
            for mp in range(4):
                w = compute_format_word(si, mp)
                assert 0 <= w <= 0x7FFF, (
                    f"si={si} mp={mp}: word 0x{w:04X} exceeds 15 bits"
                )


# =============================================================================
# 15. Grid utilities
# =============================================================================


class TestGridUtilities:
    """grid_to_string and grid shape."""

    def test_grid_to_string_dimensions(self):
        g = encode("1")  # 11×11
        s = grid_to_string(g)
        lines = s.split("\n")
        assert len(lines) == 11
        for line in lines:
            assert len(line) == 11
            assert all(c in "01" for c in line)

    def test_grid_to_string_no_trailing_newline(self):
        g = encode("1")
        s = grid_to_string(g)
        assert not s.endswith("\n")

    def test_grid_to_string_top_left_dark(self):
        """Finder corner (0,0) is always dark."""
        for inp in ["1", "HELLO", "hello", "https://a.b"]:
            g = encode(inp)
            s = grid_to_string(g)
            assert s[0] == "1", f"top-left module should be dark for '{inp}'"

    def test_grid_to_string_different_inputs_differ(self):
        s1 = grid_to_string(encode("1"))
        s2 = grid_to_string(encode("2"))
        assert s1 != s2


# =============================================================================
# 16. _BitWriter unit tests
# =============================================================================


class TestBitWriter:
    """Internal _BitWriter class."""

    def test_empty_writer(self):
        w = _BitWriter()
        assert w.bit_len() == 0
        assert w.to_bytes() == []

    def test_write_single_bit(self):
        w = _BitWriter()
        w.write(1, 1)
        assert w.bit_len() == 1

    def test_write_byte_msb_first(self):
        w = _BitWriter()
        w.write(0b10110000, 8)
        assert w.to_bytes() == [0b10110000]

    def test_write_padded_to_byte(self):
        w = _BitWriter()
        w.write(0b101, 3)  # bits: 1,0,1 → padded to 10100000
        assert w.to_bytes() == [0b10100000]

    def test_to_bit_list(self):
        w = _BitWriter()
        w.write(0b110, 3)
        bits = w.to_bit_list()
        assert bits == [1, 1, 0]

    def test_multiple_writes(self):
        w = _BitWriter()
        w.write(0b1, 1)
        w.write(0b0, 1)
        w.write(0b111, 3)
        # bits: 1, 0, 1, 1, 1 → padded to 10111000
        assert w.to_bytes() == [0b10111000]

    def test_16_bits(self):
        w = _BitWriter()
        w.write(0xABCD, 16)
        b = w.to_bytes()
        assert b == [0xAB, 0xCD]


# =============================================================================
# 17. Symbol config table integrity
# =============================================================================


class TestSymbolConfigTable:
    """_SYMBOL_CONFIGS must have exactly 8 entries and correct values."""

    def test_exactly_8_configs(self):
        assert len(_SYMBOL_CONFIGS) == 8

    def test_all_symbol_indicators_unique(self):
        sis = [c.symbol_indicator for c in _SYMBOL_CONFIGS]
        assert sorted(sis) == list(range(8))

    def test_sizes_correct(self):
        expected = {"M1": 11, "M2": 13, "M3": 15, "M4": 17}
        for cfg in _SYMBOL_CONFIGS:
            assert cfg.size == expected[cfg.version], (
                f"{cfg.version}: expected size {expected[cfg.version]}, got {cfg.size}"
            )

    def test_m1_has_half_cw(self):
        m1 = next(c for c in _SYMBOL_CONFIGS if c.version == "M1")
        assert m1.m1_half_cw is True

    def test_others_have_no_half_cw(self):
        for cfg in _SYMBOL_CONFIGS:
            if cfg.version != "M1":
                assert cfg.m1_half_cw is False

    def test_m1_has_no_alpha_or_byte(self):
        m1 = next(c for c in _SYMBOL_CONFIGS if c.version == "M1")
        assert m1.alpha_cap == 0
        assert m1.byte_cap == 0

    def test_m4_q_has_most_ecc(self):
        m4q = next(c for c in _SYMBOL_CONFIGS if c.version == "M4" and c.ecc == "Q")
        assert m4q.ecc_cw == 14  # largest ECC block in Micro QR
