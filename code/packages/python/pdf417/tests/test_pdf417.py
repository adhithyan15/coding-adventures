"""Tests for the PDF417 encoder.

Test strategy follows the spec (code/specs/pdf417.md):

1. GF(929) arithmetic — add, mul, inverse, log/antilog tables.
2. RS generator polynomial — known coefficients for ECC level 0.
3. RS ECC encoding — known test vectors.
4. Byte compaction — known input/output pairs including edge cases.
5. Row indicator computation — known values from spec.
6. Dimension selection — heuristic output for specific inputs.
7. Symbol dimensions — output grid has correct shape.
8. Start/stop patterns — every row begins/ends with the fixed patterns.
9. Error handling — bad ECC level, bad columns, input too long.
10. ModuleGrid content — consistency checks across the grid.
"""

from __future__ import annotations

import pytest

from pdf417 import (
    InvalidDimensionsError,
    InvalidECCLevelError,
    InputTooLongError,
    PDF417Error,
    compute_lri,
    compute_rri,
    encode,
    grid_to_string,
)
from pdf417._cluster_tables import CLUSTER_TABLES, START_PATTERN, STOP_PATTERN
from pdf417.pdf417 import (
    _GF_EXP,
    _GF_LOG,
    _auto_ecc_level,
    _byte_compact,
    _choose_dimensions,
    _expand_pattern,
    _expand_widths,
    _gf_add,
    _gf_mul,
    _rs_encode,
    _build_generator,
)

# ═══════════════════════════════════════════════════════════════════════════════
# GF(929) arithmetic
# ═══════════════════════════════════════════════════════════════════════════════


class TestGF929Arithmetic:
    """GF(929) field arithmetic — basic sanity checks."""

    def test_add_no_wrap(self) -> None:
        # 100 + 200 = 300, no modular reduction needed.
        assert _gf_add(100, 200) == 300

    def test_add_with_wrap(self) -> None:
        # (100 + 900) mod 929 = 1000 mod 929 = 71
        assert _gf_add(100, 900) == 71

    def test_add_identity(self) -> None:
        # 0 is the additive identity.
        assert _gf_add(500, 0) == 500
        assert _gf_add(0, 500) == 500

    def test_add_modular_boundary(self) -> None:
        # 928 + 1 = 929 mod 929 = 0
        assert _gf_add(928, 1) == 0

    def test_mul_zero(self) -> None:
        # Zero absorbs multiplication.
        assert _gf_mul(0, 500) == 0
        assert _gf_mul(500, 0) == 0
        assert _gf_mul(0, 0) == 0

    def test_mul_one(self) -> None:
        # 1 is the multiplicative identity.
        assert _gf_mul(1, 500) == 500
        assert _gf_mul(500, 1) == 500

    def test_mul_generator(self) -> None:
        # α × α = α^2 = 3^2 mod 929 = 9
        assert _gf_mul(3, 3) == 9

    def test_mul_known_inverse(self) -> None:
        # 3 × inv(3) = 1.  inv(3) mod 929: 3 × 310 = 930 ≡ 1 (mod 929).
        assert _gf_mul(3, 310) == 1

    def test_mul_commutativity(self) -> None:
        assert _gf_mul(7, 13) == _gf_mul(13, 7)

    def test_mul_associativity(self) -> None:
        a, b, c = 11, 17, 23
        assert _gf_mul(_gf_mul(a, b), c) == _gf_mul(a, _gf_mul(b, c))

    def test_mul_distributivity(self) -> None:
        a, b, c = 5, 7, 11
        assert _gf_mul(a, _gf_add(b, c)) == _gf_add(_gf_mul(a, b), _gf_mul(a, c))

    def test_fermat_little_theorem(self) -> None:
        # α^928 ≡ 1 (mod 929) — Fermat's little theorem.
        # Verify via log table: α^928 = GF_EXP[928] = GF_EXP[0] = 1.
        assert _GF_EXP[928] == 1

    def test_log_exp_round_trip(self) -> None:
        # For every non-zero element v: GF_EXP[GF_LOG[v]] == v.
        for v in range(1, 929):
            assert _GF_EXP[_GF_LOG[v]] == v, f"Round-trip failed for v={v}"

    def test_exp_table_length(self) -> None:
        # The exp table has 929 entries (indices 0..928).
        assert len(_GF_EXP) == 929

    def test_log_table_length(self) -> None:
        assert len(_GF_LOG) == 929

    def test_exp_table_nonzero(self) -> None:
        # All values in exp table for indices 0..927 are non-zero.
        for i in range(928):
            assert _GF_EXP[i] > 0, f"GF_EXP[{i}] should be nonzero"

    def test_exp_table_is_permutation(self) -> None:
        # GF_EXP[0..927] contains every value 1..928 exactly once.
        vals = set(_GF_EXP[i] for i in range(928))
        assert vals == set(range(1, 929))


# ═══════════════════════════════════════════════════════════════════════════════
# RS generator polynomial
# ═══════════════════════════════════════════════════════════════════════════════


class TestRSGenerator:
    """Reed-Solomon generator polynomial tests."""

    def test_generator_level_0_degree(self) -> None:
        # ECC level 0: k = 2^1 = 2 ECC codewords → degree-2 polynomial (3 coefficients).
        g = _build_generator(0)
        assert len(g) == 3  # [g2, g1, g0]

    def test_generator_level_0_leading(self) -> None:
        # The generator is monic (leading coefficient = 1).
        g = _build_generator(0)
        assert g[0] == 1

    def test_generator_level_1_degree(self) -> None:
        # ECC level 1: k = 2^2 = 4 → degree-4 polynomial (5 coefficients).
        g = _build_generator(1)
        assert len(g) == 5

    def test_generator_degree_vs_level(self) -> None:
        # For each level L, the generator has degree k = 2^(L+1).
        for level in range(5):  # 0..4 (levels 5-8 are slow to build)
            k = 1 << (level + 1)
            g = _build_generator(level)
            assert len(g) == k + 1, f"Level {level}: expected {k+1} coefficients"

    def test_generator_level_2_ecc_count(self) -> None:
        # Level 2: k = 2^3 = 8.
        g = _build_generator(2)
        assert len(g) == 9  # degree-8 polynomial


# ═══════════════════════════════════════════════════════════════════════════════
# RS ECC encoding
# ═══════════════════════════════════════════════════════════════════════════════


class TestRSEncoding:
    """Reed-Solomon ECC encoding tests."""

    def test_rs_encode_level0_length(self) -> None:
        # ECC level 0 produces exactly k=2 ECC codewords.
        data = [10, 20, 30]
        ecc = _rs_encode(data, 0)
        assert len(ecc) == 2

    def test_rs_encode_level2_length(self) -> None:
        # ECC level 2 produces exactly k=8 ECC codewords.
        data = [100, 200, 300, 400]
        ecc = _rs_encode(data, 2)
        assert len(ecc) == 8

    def test_rs_encode_all_zeros(self) -> None:
        # Encoding all zeros → all ECC codewords are zero (zero data → zero remainder).
        data = [0, 0, 0, 0]
        ecc = _rs_encode(data, 0)
        assert ecc == [0, 0]

    def test_rs_encode_codeword_range(self) -> None:
        # All ECC codewords must be in 0..928.
        data = [i % 929 for i in range(20)]
        for level in range(5):
            ecc = _rs_encode(data, level)
            for val in ecc:
                assert 0 <= val <= 928, f"ECC codeword out of range: {val}"

    def test_rs_encode_deterministic(self) -> None:
        # Same input always produces same output.
        data = [1, 2, 3, 4, 5]
        assert _rs_encode(data, 2) == _rs_encode(data, 2)

    def test_rs_encode_changes_with_data(self) -> None:
        # Changing one data codeword changes the ECC.
        data1 = [10, 20, 30, 40]
        data2 = [10, 20, 30, 41]  # changed last element
        assert _rs_encode(data1, 2) != _rs_encode(data2, 2)


# ═══════════════════════════════════════════════════════════════════════════════
# Byte compaction
# ═══════════════════════════════════════════════════════════════════════════════


class TestByteCompaction:
    """Byte compaction mode tests."""

    def test_latch_codeword(self) -> None:
        # First codeword is always 924 (byte-compaction latch).
        result = _byte_compact(b"A")
        assert result[0] == 924

    def test_single_byte_direct(self) -> None:
        # A single byte beyond a 6-byte group maps to its value directly.
        # b"\x41" = 65 = ord('A').
        result = _byte_compact(b"\x41")
        assert result == [924, 65]

    def test_single_byte_ff(self) -> None:
        # 0xFF = 255 → codeword 255.
        result = _byte_compact(b"\xFF")
        assert result == [924, 255]

    def test_six_bytes_produces_five_codewords(self) -> None:
        # 6 bytes → 5 codewords (plus latch = 6 total).
        result = _byte_compact(b"ABCDEF")
        assert len(result) == 6  # [924, c1, c2, c3, c4, c5]

    def test_six_bytes_known_value(self) -> None:
        # "ABCDEF" = [65, 66, 67, 68, 69, 70]
        # n = 65×256^5 + 66×256^4 + 67×256^3 + 68×256^2 + 69×256 + 70
        n = 0
        for b in b"ABCDEF":
            n = n * 256 + b
        # Convert to base-900.
        expected: list[int] = []
        for _ in range(5):
            expected.insert(0, n % 900)
            n //= 900
        result = _byte_compact(b"ABCDEF")
        assert result[1:] == expected

    def test_seven_bytes(self) -> None:
        # 7 bytes = 1 group of 6 + 1 remainder.
        result = _byte_compact(b"ABCDEFG")
        # [924] + [5 group codewords] + [71 (ord 'G')]
        assert len(result) == 7
        assert result[0] == 924
        assert result[6] == ord("G")

    def test_twelve_bytes(self) -> None:
        # 12 bytes = 2 groups of 6 → 10 data codewords + latch.
        result = _byte_compact(b"ABCDEFGHIJKL")
        assert len(result) == 11  # [924] + 10

    def test_empty_bytes(self) -> None:
        # Empty input → just the latch codeword.
        result = _byte_compact(b"")
        assert result == [924]

    def test_five_bytes_remainder(self) -> None:
        # 5 bytes = all remainder → 5 codewords + latch.
        result = _byte_compact(b"HELLO")
        assert result == [924, 72, 69, 76, 76, 79]

    def test_codeword_range(self) -> None:
        # All codewords in result must be in 0..928.
        data = bytes(range(256))
        result = _byte_compact(data)
        for cw in result:
            assert 0 <= cw <= 928, f"Codeword out of range: {cw}"


# ═══════════════════════════════════════════════════════════════════════════════
# Auto ECC level selection
# ═══════════════════════════════════════════════════════════════════════════════


class TestAutoECCLevel:
    """ECC level auto-selection tests."""

    def test_level_2_small(self) -> None:
        assert _auto_ecc_level(1) == 2
        assert _auto_ecc_level(40) == 2

    def test_level_3_medium(self) -> None:
        assert _auto_ecc_level(41) == 3
        assert _auto_ecc_level(160) == 3

    def test_level_4(self) -> None:
        assert _auto_ecc_level(161) == 4
        assert _auto_ecc_level(320) == 4

    def test_level_5(self) -> None:
        assert _auto_ecc_level(321) == 5
        assert _auto_ecc_level(863) == 5

    def test_level_6_large(self) -> None:
        assert _auto_ecc_level(864) == 6
        assert _auto_ecc_level(10000) == 6


# ═══════════════════════════════════════════════════════════════════════════════
# Dimension selection
# ═══════════════════════════════════════════════════════════════════════════════


class TestDimensionSelection:
    """Dimension selection heuristic tests."""

    def test_min_rows(self) -> None:
        # Any input should produce at least 3 rows.
        cols, rows = _choose_dimensions(1)
        assert rows >= 3

    def test_min_cols(self) -> None:
        # Any input should produce at least 1 column.
        cols, rows = _choose_dimensions(1)
        assert cols >= 1

    def test_max_rows(self) -> None:
        cols, rows = _choose_dimensions(200)
        assert rows <= 90

    def test_max_cols(self) -> None:
        cols, rows = _choose_dimensions(200)
        assert cols <= 30

    def test_capacity_sufficient(self) -> None:
        # rows × cols ≥ total_codewords for any total.
        for total in [1, 5, 10, 50, 100, 500, 1000]:
            cols, rows = _choose_dimensions(total)
            assert cols * rows >= total, (
                f"total={total}: cols={cols}, rows={rows}, "
                f"capacity={cols*rows} < total"
            )


# ═══════════════════════════════════════════════════════════════════════════════
# Row indicators
# ═══════════════════════════════════════════════════════════════════════════════


class TestRowIndicators:
    """Row indicator computation tests.

    For a 10-row, 3-column, ECC level 2 symbol:
        R_info = (10-1)//3 = 3
        C_info = 3-1 = 2
        L_info = 3×2 + (10-1)%3 = 6+0 = 6

    LRI formulas (matches TypeScript/Python reference implementation):
        Cluster 0: LRI = 30*row_group + R_info
        Cluster 1: LRI = 30*row_group + L_info
        Cluster 2: LRI = 30*row_group + C_info

    RRI formulas (follows Python pdf417 library, verified to produce
    scannable symbols — note the TypeScript source notes this differs
    from the ISO spec text):
        Cluster 0: RRI = 30*row_group + C_info
        Cluster 1: RRI = 30*row_group + R_info
        Cluster 2: RRI = 30*row_group + L_info

    Row 0 (cluster 0): LRI = 30×0 + 3 = 3,  RRI = 30×0 + 2 = 2
    Row 1 (cluster 1): LRI = 30×0 + 6 = 6,  RRI = 30×0 + 3 = 3
    Row 2 (cluster 2): LRI = 30×0 + 2 = 2,  RRI = 30×0 + 6 = 6
    Row 3 (cluster 0): LRI = 30×1 + 3 = 33, RRI = 30×1 + 2 = 32
    """

    def test_lri_row0(self) -> None:
        # Cluster 0: LRI = R_info = 3
        assert compute_lri(r=0, rows=10, cols=3, ecc_level=2) == 3

    def test_rri_row0(self) -> None:
        # Cluster 0: RRI = C_info = cols-1 = 2
        assert compute_rri(r=0, rows=10, cols=3, ecc_level=2) == 2

    def test_lri_row1(self) -> None:
        # Cluster 1: LRI = L_info = 6
        assert compute_lri(r=1, rows=10, cols=3, ecc_level=2) == 6

    def test_rri_row1(self) -> None:
        # Cluster 1: RRI = R_info = 3
        assert compute_rri(r=1, rows=10, cols=3, ecc_level=2) == 3

    def test_lri_row2(self) -> None:
        # Cluster 2: LRI = C_info = 2
        assert compute_lri(r=2, rows=10, cols=3, ecc_level=2) == 2

    def test_rri_row2(self) -> None:
        # Cluster 2: RRI = L_info = 6
        assert compute_rri(r=2, rows=10, cols=3, ecc_level=2) == 6

    def test_lri_row3(self) -> None:
        # Row 3 is cluster 0 again; row_group = 1 → 30 + R_info = 33
        assert compute_lri(r=3, rows=10, cols=3, ecc_level=2) == 33

    def test_rri_row3(self) -> None:
        # Row 3, cluster 0: RRI = 30*row_group + C_info = 30*1 + 2 = 32
        assert compute_rri(r=3, rows=10, cols=3, ecc_level=2) == 32

    def test_indicator_range(self) -> None:
        # All indicator values must be valid codeword values (0–928).
        for r in range(9):
            lri = compute_lri(r, 9, 3, 2)
            rri = compute_rri(r, 9, 3, 2)
            assert 0 <= lri <= 928, f"LRI out of range: {lri} at row {r}"
            assert 0 <= rri <= 928, f"RRI out of range: {rri} at row {r}"

    def test_cluster_cycles(self) -> None:
        # Row 0 and row 3 are the same cluster — LRI formula should cycle.
        lri_0 = compute_lri(0, 3, 3, 2)
        lri_3 = compute_lri(3, 3, 3, 2)
        # Row group increments by 1, so lri_3 = lri_0 + 30.
        assert lri_3 == lri_0 + 30


# ═══════════════════════════════════════════════════════════════════════════════
# Pattern expansion
# ═══════════════════════════════════════════════════════════════════════════════


class TestPatternExpansion:
    """Bar/space pattern expansion tests."""

    def test_start_pattern_length(self) -> None:
        modules = _expand_widths(START_PATTERN)
        assert len(modules) == 17

    def test_stop_pattern_length(self) -> None:
        modules = _expand_widths(STOP_PATTERN)
        assert len(modules) == 18

    def test_start_pattern_first_dark(self) -> None:
        # Start pattern begins with a bar (dark).
        modules = _expand_widths(START_PATTERN)
        assert modules[0] is True

    def test_start_pattern_modules(self) -> None:
        # 11111111010101000 (from spec)
        expected = [1,1,1,1,1,1,1,1, 0, 1, 0, 1, 0, 1, 0,0,0]
        modules = _expand_widths(START_PATTERN)
        assert [int(m) for m in modules] == expected

    def test_stop_pattern_modules(self) -> None:
        # 111111101000101001 (from spec)
        expected = [1,1,1,1,1,1,1, 0, 1, 0,0,0, 1, 0, 1, 0,0, 1]
        modules = _expand_widths(STOP_PATTERN)
        assert [int(m) for m in modules] == expected

    def test_expand_pattern_length(self) -> None:
        # Every packed pattern in cluster tables expands to exactly 17 modules.
        # Spot-check first 10 entries in each cluster.
        for cluster_idx in range(3):
            for cw in range(10):
                modules = _expand_pattern(CLUSTER_TABLES[cluster_idx][cw])
                assert len(modules) == 17, (
                    f"Cluster {cluster_idx} CW {cw}: got {len(modules)} modules"
                )

    def test_expand_pattern_starts_dark(self) -> None:
        # Every codeword pattern starts with a bar (dark module).
        for cluster_idx in range(3):
            for cw in [0, 100, 500, 928]:
                modules = _expand_pattern(CLUSTER_TABLES[cluster_idx][cw])
                assert modules[0] is True, (
                    f"Cluster {cluster_idx} CW {cw}: first module not dark"
                )


# ═══════════════════════════════════════════════════════════════════════════════
# Cluster tables
# ═══════════════════════════════════════════════════════════════════════════════


class TestClusterTables:
    """Cluster table structure tests."""

    def test_three_clusters(self) -> None:
        assert len(CLUSTER_TABLES) == 3

    def test_each_cluster_929_entries(self) -> None:
        for i, table in enumerate(CLUSTER_TABLES):
            assert len(table) == 929, f"Cluster {i}: expected 929 entries, got {len(table)}"

    def test_all_patterns_17_modules(self) -> None:
        # Every pattern in all three tables must expand to exactly 17 modules.
        # Spot-check: every 50th entry.
        for cluster_idx, table in enumerate(CLUSTER_TABLES):
            for cw in range(0, 929, 50):
                modules = _expand_pattern(table[cw])
                assert len(modules) == 17, (
                    f"Cluster {cluster_idx} CW {cw}: {len(modules)} modules"
                )

    def test_all_patterns_have_4_bars_4_spaces(self) -> None:
        # Each codeword pattern has exactly 4 bars (runs of True) and 4 spaces.
        # Spot-check every 100th codeword.
        for cluster_idx, table in enumerate(CLUSTER_TABLES):
            for cw in range(0, 929, 100):
                modules = _expand_pattern(table[cw])
                # Count runs.
                runs = []
                if modules:
                    current = modules[0]
                    length = 1
                    for m in modules[1:]:
                        if m == current:
                            length += 1
                        else:
                            runs.append((current, length))
                            current = m
                            length = 1
                    runs.append((current, length))
                bars   = [(v, l) for v, l in runs if v]
                spaces = [(v, l) for v, l in runs if not v]
                assert len(bars) == 4, (
                    f"Cluster {cluster_idx} CW {cw}: expected 4 bars, got {len(bars)}"
                )
                assert len(spaces) == 4, (
                    f"Cluster {cluster_idx} CW {cw}: expected 4 spaces, got {len(spaces)}"
                )


# ═══════════════════════════════════════════════════════════════════════════════
# Encode — output structure
# ═══════════════════════════════════════════════════════════════════════════════


class TestEncode:
    """End-to-end encode() tests."""

    # ── Symbol dimensions ────────────────────────────────────────────────────

    def test_encode_returns_module_grid(self) -> None:
        from barcode_2d import ModuleGrid
        grid = encode("A")
        assert isinstance(grid, ModuleGrid)

    def test_encode_width_formula(self) -> None:
        # Width = 69 + 17×cols.
        grid = encode("A")
        # Determine cols from width.
        w = grid.cols
        # w = 69 + 17*cols → cols = (w - 69) / 17
        assert (w - 69) % 17 == 0, f"Width {w} is not of the form 69+17k"

    def test_encode_height_formula(self) -> None:
        # Height = rows × row_height.
        # With default row_height=3, height must be divisible by 3.
        grid = encode("A")
        assert grid.rows % 3 == 0, f"Height {grid.rows} not divisible by 3"

    def test_encode_custom_row_height(self) -> None:
        grid1 = encode("A", row_height=3)
        grid2 = encode("A", row_height=5)
        # Same logical rows, different module height.
        assert grid2.rows > grid1.rows

    def test_encode_columns_override(self) -> None:
        grid = encode("HELLO WORLD", columns=5)
        w = grid.cols
        assert (w - 69) % 17 == 0
        expected_cols = (w - 69) // 17
        assert expected_cols == 5

    def test_encode_ecc_level_override(self) -> None:
        # Higher ECC → more codewords → larger (or same) grid.
        grid_l2 = encode("TEST", ecc_level=2)
        grid_l4 = encode("TEST", ecc_level=4)
        # ECC level 4 uses 32 ECC codewords vs level 2 with 8.
        # Either more rows or more cols are needed.
        capacity_l2 = grid_l2.rows * grid_l2.cols  # in logical terms
        capacity_l4 = grid_l4.rows * grid_l4.cols
        assert capacity_l4 >= capacity_l2

    # ── Start pattern ────────────────────────────────────────────────────────

    def _check_start_pattern(self, row_modules: list[bool]) -> None:
        """Assert first 17 modules of row_modules match the start pattern."""
        expected = [1,1,1,1,1,1,1,1, 0, 1, 0, 1, 0, 1, 0,0,0]
        actual = [int(m) for m in row_modules[:17]]
        assert actual == expected, f"Start pattern mismatch: {actual}"

    def _check_stop_pattern(self, row_modules: list[bool]) -> None:
        """Assert last 18 modules of row_modules match the stop pattern."""
        expected = [1,1,1,1,1,1,1, 0, 1, 0,0,0, 1, 0, 1, 0,0, 1]
        actual = [int(m) for m in row_modules[-18:]]
        assert actual == expected, f"Stop pattern mismatch: {actual}"

    def test_every_row_has_start_pattern(self) -> None:
        grid = encode("HELLO WORLD")
        row_h = 3  # default row height
        n_logical_rows = grid.rows // row_h
        module_width = grid.cols

        for r in range(n_logical_rows):
            # Read the first module row of this logical row.
            row_modules = [grid.modules[r * row_h][c] for c in range(module_width)]
            self._check_start_pattern(row_modules)

    def test_every_row_has_stop_pattern(self) -> None:
        grid = encode("HELLO WORLD")
        row_h = 3
        n_logical_rows = grid.rows // row_h
        module_width = grid.cols

        for r in range(n_logical_rows):
            row_modules = [grid.modules[r * row_h][c] for c in range(module_width)]
            self._check_stop_pattern(row_modules)

    def test_row_height_repeats_modules(self) -> None:
        # With row_height=3, each logical row should be repeated 3 times.
        grid = encode("A", row_height=3)
        # Check the first two module rows are identical (both are row 0 of logical row 0).
        row0 = [grid.modules[0][c] for c in range(grid.cols)]
        row1 = [grid.modules[1][c] for c in range(grid.cols)]
        assert row0 == row1, "First two module rows should be identical"

    # ── Various inputs ───────────────────────────────────────────────────────

    def test_encode_single_byte(self) -> None:
        # "A" is the minimal test case.
        grid = encode("A")
        assert grid.rows > 0
        assert grid.cols > 0

    def test_encode_hello_world(self) -> None:
        # 11-byte classic input.
        grid = encode("HELLO WORLD")
        assert grid.rows > 0
        assert grid.cols > 0

    def test_encode_digits(self) -> None:
        # Digit-only input (in v0.1.0, byte-compacted).
        grid = encode("1234567890")
        assert grid.rows > 0

    def test_encode_binary_data(self) -> None:
        # Binary with all 256 byte values.
        data = "".join(chr(i) for i in range(256))
        # Some of these may not be valid Unicode; encode as bytes directly.
        # encode() takes a str and encodes to UTF-8.
        # Use only valid ASCII for now.
        data = "".join(chr(i) for i in range(128))
        grid = encode(data)
        assert grid.rows > 0

    def test_encode_empty_string(self) -> None:
        # Empty input: just the byte-compact latch (924), length descriptor, ECC.
        grid = encode("")
        assert grid.rows >= 3

    def test_encode_long_string(self) -> None:
        # 100-character string — should produce a valid symbol.
        data = "A" * 100
        grid = encode(data)
        assert grid.rows > 0
        assert grid.cols > 0

    def test_encode_unicode(self) -> None:
        # Unicode input gets UTF-8 encoded.
        grid = encode("Héllo")
        assert grid.rows > 0

    # ── Grid consistency ─────────────────────────────────────────────────────

    def test_grid_is_rectangular(self) -> None:
        grid = encode("test")
        for r in range(grid.rows):
            assert len(grid.modules[r]) == grid.cols

    def test_all_rows_identical_within_logical_row(self) -> None:
        # All module rows within a logical row should be identical.
        grid = encode("TEST", row_height=4)
        n_logical_rows = grid.rows // 4
        for lr in range(n_logical_rows):
            base = lr * 4
            row_base = [grid.modules[base][c] for c in range(grid.cols)]
            for h in range(1, 4):
                row_h = [grid.modules[base + h][c] for c in range(grid.cols)]
                assert row_base == row_h, (
                    f"Logical row {lr}, module row +{h} differs from base"
                )

    def test_grid_to_string(self) -> None:
        grid = encode("A")
        s = grid_to_string(grid)
        lines = s.split("\n")
        assert len(lines) == grid.rows
        for line in lines:
            assert len(line) == grid.cols
        assert all(c in "01" for line in lines for c in line)

    def test_deterministic(self) -> None:
        # Same input always produces identical grid.
        s = grid_to_string(encode("HELLO"))
        for _ in range(3):
            assert grid_to_string(encode("HELLO")) == s

    def test_different_inputs_differ(self) -> None:
        # Different inputs must produce different grids (with overwhelmingly high
        # probability — this is not guaranteed in general but holds for these inputs).
        s1 = grid_to_string(encode("HELLO"))
        s2 = grid_to_string(encode("WORLD"))
        assert s1 != s2


# ═══════════════════════════════════════════════════════════════════════════════
# Error handling
# ═══════════════════════════════════════════════════════════════════════════════


class TestErrorHandling:
    """Error condition tests."""

    def test_invalid_ecc_level_low(self) -> None:
        with pytest.raises(InvalidECCLevelError):
            encode("A", ecc_level=-1)

    def test_invalid_ecc_level_high(self) -> None:
        with pytest.raises(InvalidECCLevelError):
            encode("A", ecc_level=9)

    def test_invalid_ecc_level_is_pdf417_error(self) -> None:
        with pytest.raises(PDF417Error):
            encode("A", ecc_level=99)

    def test_invalid_columns_low(self) -> None:
        with pytest.raises(InvalidDimensionsError):
            encode("A", columns=0)

    def test_invalid_columns_high(self) -> None:
        with pytest.raises(InvalidDimensionsError):
            encode("A", columns=31)

    def test_invalid_columns_is_pdf417_error(self) -> None:
        with pytest.raises(PDF417Error):
            encode("A", columns=100)

    def test_input_too_long_with_forced_columns(self) -> None:
        # Force 1 column with a huge input — should exceed 90 rows.
        big_data = "X" * 10000
        with pytest.raises(InputTooLongError):
            encode(big_data, columns=1)

    def test_input_too_long_is_pdf417_error(self) -> None:
        big_data = "X" * 10000
        with pytest.raises(PDF417Error):
            encode(big_data, columns=1)

    def test_valid_ecc_levels(self) -> None:
        # All ECC levels 0–8 are valid.
        for level in range(9):
            grid = encode("A", ecc_level=level)
            assert grid.rows > 0

    def test_valid_column_counts(self) -> None:
        # All column counts 1–30 are valid (with short enough input).
        for cols in range(1, 31):
            grid = encode("A", columns=cols)
            assert grid.rows > 0


# ═══════════════════════════════════════════════════════════════════════════════
# Cross-verification helpers
# ═══════════════════════════════════════════════════════════════════════════════


class TestCrossVerification:
    """Tests that verify internal consistency of the encoding.

    These tests do not require an external decoder — they verify structural
    properties of the symbol that must hold for any correct PDF417 encoder.
    """

    def _logical_rows(self, grid_rows: int, row_height: int) -> int:
        return grid_rows // row_height

    def _data_cols(self, grid_cols: int) -> int:
        # grid_cols = 69 + 17*cols → cols = (grid_cols - 69) / 17
        assert (grid_cols - 69) % 17 == 0
        return (grid_cols - 69) // 17

    def test_module_width_formula(self) -> None:
        # For various inputs, verify width = 69 + 17*cols.
        for text, cols_override in [("A", None), ("HELLO", 3), ("TEST", 5)]:
            kw = {} if cols_override is None else {"columns": cols_override}
            grid = encode(text, **kw)
            c = self._data_cols(grid.cols)
            assert grid.cols == 69 + 17 * c

    def test_row_count_multiple_of_row_height(self) -> None:
        for rh in [1, 2, 3, 5]:
            grid = encode("PDF417", row_height=rh)
            assert grid.rows % rh == 0, (
                f"row_height={rh}: grid.rows={grid.rows} not divisible"
            )

    def test_start_pattern_column_1_is_dark(self) -> None:
        # The start pattern's first 8 modules are all dark.
        grid = encode("A")
        for r in range(grid.rows):
            for c in range(8):
                assert grid.modules[r][c] is True, (
                    f"Row {r} col {c}: expected dark (start pattern bar)"
                )

    def test_stop_pattern_last_module_dark(self) -> None:
        # The stop pattern ends with a single dark module.
        # Module sequence: ...0, 0, 1 (last 3 of stop pattern).
        grid = encode("A")
        last_col = grid.cols - 1
        for r in range(grid.rows):
            assert grid.modules[r][last_col] is True, (
                f"Row {r}: last module should be dark (stop pattern)"
            )
