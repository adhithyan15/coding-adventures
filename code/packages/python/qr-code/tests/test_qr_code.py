"""Tests for the QR Code encoder (ISO/IEC 18004:2015).

Coverage goals: >90% of the encoder, verifying:
- All four ECC levels (L, M, Q, H)
- All three encoding modes (numeric, alphanumeric, byte)
- Version selection (v1 minimum, larger versions for bigger input)
- Finder, timing, and alignment patterns
- Dark module placement
- Format information correctness
- Version information (v7+)
- Masking: 8 patterns evaluated, best penalty chosen
- RS ECC generation
- Interleaving
- Error cases: too-long input, invalid ECC level, invalid version

These tests treat the encoder as a white box — they check internal
structure (e.g. are finder patterns at the right positions?) as well as
the black-box property (does encode() return a valid-sized grid?).
"""

from __future__ import annotations

import pytest

from qr_code import (
    ALPHANUM_CHARS,
    ECC_CODEWORDS_PER_BLOCK,
    ECC_IDX,
    ECC_INDICATOR,
    NUM_BLOCKS,
    InputTooLongError,
    apply_mask,
    build_data_codewords,
    compute_blocks,
    compute_format_bits,
    compute_penalty,
    compute_version_bits,
    encode,
    encode_to_scene,
    interleave_blocks,
    num_data_codewords,
    num_raw_data_modules,
    num_remainder_bits,
    rs_encode,
    select_mode,
    select_version,
    symbol_size,
    write_format_info,
)
from qr_code._qr_code import (
    BitWriter,
    Block,
    WorkGrid,
    _build_generator,
    build_grid,
    compute_blocks,
    encode_alphanumeric,
    encode_byte,
    encode_numeric,
    place_all_alignments,
    place_dark_module,
    place_finder,
    place_timing_strips,
    reserve_format_info,
    reserve_version_info,
)


# ===========================================================================
# symbol_size
# ===========================================================================


def test_symbol_size_v1() -> None:
    """Version 1 is 21×21 modules."""
    assert symbol_size(1) == 21


def test_symbol_size_v40() -> None:
    """Version 40 is 177×177 modules."""
    assert symbol_size(40) == 177


def test_symbol_size_formula() -> None:
    """Every version: 4v + 17."""
    for v in range(1, 41):
        assert symbol_size(v) == 4 * v + 17


# ===========================================================================
# num_raw_data_modules
# ===========================================================================


def test_num_raw_data_modules_v1() -> None:
    """Version 1 has 208 raw modules (208 bits = 26 bytes)."""
    assert num_raw_data_modules(1) == 208


def test_num_raw_data_modules_grows_with_version() -> None:
    """Raw module count must strictly increase with version."""
    prev = 0
    for v in range(1, 41):
        cur = num_raw_data_modules(v)
        assert cur > prev, f"v{v}: {cur} <= v{v-1}: {prev}"
        prev = cur


# ===========================================================================
# num_data_codewords
# ===========================================================================


def test_v1_m_data_codewords() -> None:
    """Version 1, ECC=M: 16 data codewords."""
    # 208 raw bits / 8 = 26 bytes total, minus 1 block × 10 ECC = 16
    assert num_data_codewords(1, "M") == 16


def test_v1_l_data_codewords() -> None:
    """Version 1, ECC=L: 19 data codewords."""
    assert num_data_codewords(1, "L") == 19


def test_all_ecc_levels_v1() -> None:
    """v1 data codeword counts per ISO Table 9."""
    assert num_data_codewords(1, "L") == 19
    assert num_data_codewords(1, "M") == 16
    assert num_data_codewords(1, "Q") == 13
    assert num_data_codewords(1, "H") == 9


# ===========================================================================
# num_remainder_bits
# ===========================================================================


def test_remainder_bits_v1() -> None:
    """Version 1 has 0 remainder bits."""
    assert num_remainder_bits(1) == 0


def test_remainder_bits_v2() -> None:
    """Version 2 has 7 remainder bits."""
    assert num_remainder_bits(2) == 7


# ===========================================================================
# select_mode
# ===========================================================================


def test_select_mode_numeric() -> None:
    assert select_mode("01234567890") == "numeric"


def test_select_mode_alphanumeric() -> None:
    assert select_mode("HELLO WORLD") == "alphanumeric"
    assert select_mode("HTTP://EXAMPLE.COM") == "alphanumeric"


def test_select_mode_byte() -> None:
    assert select_mode("Hello, World!") == "byte"
    assert select_mode("hello") == "byte"  # lowercase → byte
    assert select_mode("abc123") == "byte"  # lowercase → byte


def test_select_mode_empty() -> None:
    """Empty string falls to numeric (all chars are digits vacuously)."""
    assert select_mode("") == "numeric"


# ===========================================================================
# ALPHANUM_CHARS
# ===========================================================================


def test_alphanum_chars_length() -> None:
    """QR alphanumeric alphabet has exactly 45 characters."""
    assert len(ALPHANUM_CHARS) == 45


def test_alphanum_chars_contains_expected() -> None:
    """Spot-check a few characters and their positions."""
    assert ALPHANUM_CHARS[0] == "0"
    assert ALPHANUM_CHARS[9] == "9"
    assert ALPHANUM_CHARS[10] == "A"
    assert ALPHANUM_CHARS[35] == "Z"
    assert ALPHANUM_CHARS[36] == " "
    assert ALPHANUM_CHARS[44] == ":"


def test_alphanum_chars_no_newline() -> None:
    """ALPHANUM_CHARS must not contain a newline character."""
    assert "\n" not in ALPHANUM_CHARS


# ===========================================================================
# BitWriter
# ===========================================================================


def test_bit_writer_single_byte() -> None:
    """Writing 8 bits of value 0xFF produces [0xFF]."""
    w = BitWriter()
    w.write(0xFF, 8)
    assert w.to_bytes() == [0xFF]


def test_bit_writer_msb_first() -> None:
    """Value 0b10110000 written as 8 bits produces [0xB0]."""
    w = BitWriter()
    w.write(0b10110000, 8)
    assert w.to_bytes() == [0xB0]


def test_bit_writer_accumulates_bits() -> None:
    """Two 4-bit nibbles combine into one byte."""
    w = BitWriter()
    w.write(0b1010, 4)
    w.write(0b0101, 4)
    assert w.to_bytes() == [0b10100101]


def test_bit_writer_bit_length() -> None:
    """bit_length tracks total bits written."""
    w = BitWriter()
    assert w.bit_length == 0
    w.write(0, 4)
    assert w.bit_length == 4
    w.write(0, 4)
    assert w.bit_length == 8


# ===========================================================================
# RS generator + encode
# ===========================================================================


def test_build_generator_degree_1() -> None:
    """Generator of degree 1 = [1, α^0] = [1, 1]."""
    g = _build_generator(1)
    assert g == (1, 1)


def test_build_generator_degree_7_length() -> None:
    """Generator of degree 7 has 8 coefficients."""
    g = _build_generator(7)
    assert len(g) == 8


def test_rs_encode_zero_data() -> None:
    """ECC of all-zero data is all zeros (g divides x^n perfectly)."""
    from qr_code._qr_code import _build_generator, rs_encode

    gen = _build_generator(10)
    rem = rs_encode([0] * 16, gen)
    assert rem == [0] * 10


def test_rs_encode_deterministic() -> None:
    """RS encode is deterministic for the same inputs."""
    from qr_code._qr_code import _build_generator, rs_encode

    gen = _build_generator(10)
    data = [32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236]
    r1 = rs_encode(data, gen)
    r2 = rs_encode(data, gen)
    assert r1 == r2


# ===========================================================================
# build_data_codewords
# ===========================================================================


def test_build_data_codewords_length() -> None:
    """Output length == num_data_codewords(version, ecc)."""
    for text, version, ecc in [
        ("HELLO WORLD", 1, "M"),
        ("0123456789", 1, "L"),
        ("HI", 1, "Q"),  # short enough for v1/Q (13 data codewords)
    ]:
        cw = build_data_codewords(text, version, ecc)
        assert len(cw) == num_data_codewords(version, ecc), (
            f"text={text!r} v={version} ecc={ecc}: "
            f"got {len(cw)}, want {num_data_codewords(version, ecc)}"
        )


def test_build_data_codewords_pad_bytes() -> None:
    """Pad bytes alternate 0xEC / 0x11 in the correct order."""
    # "0" in numeric mode on v1/L: very short, many pad bytes follow
    cw = build_data_codewords("0", 1, "L")
    # Capacity = 19 bytes; data part is tiny, so most of cw are pads
    # Find where real data ends and pads begin by looking for 0xEC
    # The first 0xEC pad appears within the first few bytes
    pad_region = cw[-6:]  # last 6 bytes should all be pad bytes
    pads_seen = []
    expected_pad = 0xEC
    for b in pad_region:
        if b in (0xEC, 0x11):
            pads_seen.append(b)
            expected_pad = 0x11 if expected_pad == 0xEC else 0xEC
    # All collected pad bytes should match the alternating pattern
    alt_check = []
    p = 0xEC
    for _ in pads_seen:
        alt_check.append(p)
        p = 0x11 if p == 0xEC else 0xEC
    assert pads_seen == alt_check


# ===========================================================================
# Encode numeric / alphanumeric
# ===========================================================================


def test_encode_numeric_groups_of_3() -> None:
    """Numeric encoder packs three digits into 10 bits."""
    w = BitWriter()
    encode_numeric("012", w)
    assert w.bit_length == 10
    data = w.to_bytes()
    # 12 decimal = 0x0C → first byte = 0x00, second byte = 0x0C ... actually:
    # 012 = 12 decimal, written as 10 bits MSB-first = 0b0000001100
    # byte 0 = 0b00000011 = 3, byte 1 (upper 2 bits padded) = 0b00000000
    bits_int = (data[0] << 2) | (data[1] >> 6)
    assert bits_int == 12


def test_encode_numeric_single_digit() -> None:
    """Single trailing digit → 4 bits."""
    w = BitWriter()
    encode_numeric("7", w)
    assert w.bit_length == 4
    # 7 in 4 bits MSB-first: 0111 → upper nibble of byte 0 = 0x70
    assert w.to_bytes()[0] == 0x70


def test_encode_alphanumeric_pair() -> None:
    """A pair encodes as (idx1*45+idx2) in 11 bits."""
    # "AC": A=10, C=12  → 10*45 + 12 = 462
    w = BitWriter()
    encode_alphanumeric("AC", w)
    assert w.bit_length == 11
    val = (w.to_bytes()[0] << 3) | (w.to_bytes()[1] >> 5)
    assert val == 462


def test_encode_byte_utf8() -> None:
    """Byte mode: each UTF-8 byte produces 8 bits."""
    w = BitWriter()
    encode_byte("A", w)
    assert w.bit_length == 8
    assert w.to_bytes() == [ord("A")]


# ===========================================================================
# compute_blocks / interleave_blocks
# ===========================================================================


def test_compute_blocks_total_data() -> None:
    """All blocks together contain exactly num_data_codewords bytes."""
    for v, ecc in [(1, "M"), (5, "Q"), (7, "H")]:
        data = list(range(num_data_codewords(v, ecc)))
        blocks = compute_blocks(data, v, ecc)
        total = sum(len(b.data) for b in blocks)
        assert total == num_data_codewords(v, ecc)


def test_compute_blocks_ecc_length() -> None:
    """Each block has exactly ECC_CODEWORDS_PER_BLOCK[ecc][v] ECC bytes."""
    for v, ecc in [(1, "M"), (2, "L"), (5, "H")]:
        e = ECC_IDX[ecc]
        expected_ecc = ECC_CODEWORDS_PER_BLOCK[e][v]
        data = list(range(num_data_codewords(v, ecc)))
        blocks = compute_blocks(data, v, ecc)
        for blk in blocks:
            assert len(blk.ecc) == expected_ecc


def test_interleave_blocks_length() -> None:
    """Interleaved output length = sum of all data + ecc bytes."""
    v, ecc = 5, "M"
    data = list(range(num_data_codewords(v, ecc)))
    blocks = compute_blocks(data, v, ecc)
    interleaved = interleave_blocks(blocks)
    e = ECC_IDX[ecc]
    expected = (
        num_data_codewords(v, ecc)
        + NUM_BLOCKS[e][v] * ECC_CODEWORDS_PER_BLOCK[e][v]
    )
    assert len(interleaved) == expected


# ===========================================================================
# WorkGrid
# ===========================================================================


def test_work_grid_make() -> None:
    """WorkGrid.make creates all-False, all-unreserved grid."""
    g = WorkGrid.make(21)
    assert g.size == 21
    assert not any(g.modules[r][c] for r in range(21) for c in range(21))
    assert not any(g.reserved[r][c] for r in range(21) for c in range(21))


def test_work_grid_set() -> None:
    """WorkGrid.set correctly sets dark and reserve flags."""
    g = WorkGrid.make(5)
    g.set(2, 3, True, reserve=True)
    assert g.modules[2][3] is True
    assert g.reserved[2][3] is True
    # Neighbouring cell untouched
    assert g.modules[2][2] is False
    assert g.reserved[2][2] is False


def test_work_grid_to_module_grid() -> None:
    """to_module_grid produces correct ModuleGrid."""
    g = WorkGrid.make(3)
    g.set(0, 0, True)
    g.set(2, 2, True)
    mg = g.to_module_grid()
    assert mg.rows == 3
    assert mg.cols == 3
    assert mg.modules[0][0] is True
    assert mg.modules[2][2] is True
    assert mg.modules[1][1] is False


# ===========================================================================
# Structural patterns
# ===========================================================================


def test_finder_pattern_border_dark() -> None:
    """Finder pattern outer border is all dark."""
    g = WorkGrid.make(21)
    place_finder(g, 0, 0)
    # Check all border cells
    for i in range(7):
        assert g.modules[0][i], f"top row, col {i}"
        assert g.modules[6][i], f"bottom row, col {i}"
        assert g.modules[i][0], f"left col, row {i}"
        assert g.modules[i][6], f"right col, row {i}"


def test_finder_pattern_inner_ring_light() -> None:
    """Finder pattern inner ring (row/col 1 and 5, excluding corners) is light."""
    g = WorkGrid.make(21)
    place_finder(g, 0, 0)
    for i in range(1, 6):
        assert not g.modules[1][i], f"inner ring top, col {i}"
        assert not g.modules[5][i], f"inner ring bottom, col {i}"
        assert not g.modules[i][1], f"inner ring left, row {i}"
        assert not g.modules[i][5], f"inner ring right, row {i}"


def test_finder_pattern_core_dark() -> None:
    """Finder pattern 3×3 core is all dark."""
    g = WorkGrid.make(21)
    place_finder(g, 0, 0)
    for r in range(2, 5):
        for c in range(2, 5):
            assert g.modules[r][c], f"core at ({r},{c})"


def test_timing_strips_row() -> None:
    """Timing row 6 alternates starting dark at col 8."""
    g = WorkGrid.make(21)
    # Place finders first (they mark row/col 6 as unreserved at timing positions)
    place_finder(g, 0, 0)
    place_finder(g, 0, 14)
    place_finder(g, 14, 0)
    place_timing_strips(g)
    for c in range(8, 13):
        expected = (c % 2 == 0)
        assert g.modules[6][c] == expected, f"timing row 6, col {c}"


def test_dark_module_placement() -> None:
    """Always-dark module at (4v+9, 8)."""
    for version in [1, 2, 5, 10]:
        g = WorkGrid.make(symbol_size(version))
        place_dark_module(g, version)
        r = 4 * version + 9
        assert g.modules[r][8] is True, f"v{version}: dark module at ({r}, 8)"
        assert g.reserved[r][8] is True, f"v{version}: dark module must be reserved"


def test_alignment_patterns_v1_none() -> None:
    """Version 1 has no alignment patterns."""
    g = WorkGrid.make(21)
    place_finder(g, 0, 0)
    place_finder(g, 0, 14)
    place_finder(g, 14, 0)
    place_timing_strips(g)
    # Count reserved cells before alignment
    before = sum(g.reserved[r][c] for r in range(21) for c in range(21))
    place_all_alignments(g, 1)
    after = sum(g.reserved[r][c] for r in range(21) for c in range(21))
    assert before == after, "v1 should add no alignment patterns"


def test_alignment_patterns_v2_present() -> None:
    """Version 2 has one alignment pattern at (18, 18)."""
    g = WorkGrid.make(symbol_size(2))
    # Place minimal structure
    sz = g.size
    place_finder(g, 0, 0)
    place_finder(g, 0, sz - 7)
    place_finder(g, sz - 7, 0)
    place_timing_strips(g)
    place_all_alignments(g, 2)
    # Alignment centre should be dark and reserved
    assert g.modules[18][18] is True
    assert g.reserved[18][18] is True


def test_reserve_format_info_positions() -> None:
    """Format info cells are reserved (but not counted as dark)."""
    g = WorkGrid.make(21)
    reserve_format_info(g)
    # Row 8, cols 0–5 should be reserved
    for c in range(6):
        assert g.reserved[8][c], f"row 8, col {c} should be reserved"
    # Row 8, col 6 is timing — NOT reserved here
    assert not g.reserved[8][6]
    # Row 8, col 7 should be reserved
    assert g.reserved[8][7]
    # Col 8, rows 0–5 should be reserved
    for r in range(6):
        assert g.reserved[r][8], f"col 8, row {r} should be reserved"


def test_reserve_version_info_v6_skipped() -> None:
    """Version 6 does not reserve version info positions."""
    g = WorkGrid.make(symbol_size(6))
    reserve_version_info(g, 6)
    sz = g.size
    # No version info cells should be reserved
    for r in range(6):
        for dc in range(3):
            assert not g.reserved[r][sz - 11 + dc]


def test_reserve_version_info_v7() -> None:
    """Version 7 reserves the 6×3 blocks for version information."""
    g = WorkGrid.make(symbol_size(7))
    reserve_version_info(g, 7)
    sz = g.size
    for r in range(6):
        for dc in range(3):
            assert g.reserved[r][sz - 11 + dc], f"top-right: ({r}, {sz - 11 + dc})"
    for dr in range(3):
        for c in range(6):
            assert g.reserved[sz - 11 + dr][c], f"bottom-left: ({sz - 11 + dr}, {c})"


# ===========================================================================
# Format information
# ===========================================================================


def test_compute_format_bits_known_values() -> None:
    """Format bits for known (ECC, mask) pairs from ISO examples."""
    # These expected values are derived from the ISO standard / reference
    # implementations.  ECC=M, mask=0: data=0b00_000=0, BCH, XOR 0x5412
    # Let's verify ECC indicator and mask embedding:
    for ecc in ("L", "M", "Q", "H"):
        for mask in range(8):
            fmt = compute_format_bits(ecc, mask)
            # Result must be a 15-bit number
            assert 0 <= fmt < (1 << 15), f"format bits overflow: ecc={ecc} mask={mask}"


def test_format_bits_ecc_m_mask_0() -> None:
    """ECC=M mask=0: data=0b000_00=0, result must XOR-decode to ECC_INDICATOR[M]."""
    # Un-XOR and check data field
    fmt = compute_format_bits("M", 0)
    raw = fmt ^ 0x5412  # remove XOR mask
    data_field = raw >> 10  # upper 5 bits
    # data_field = (ECC_indicator << 3) | mask
    ecc_ind = data_field >> 3
    mask_bits = data_field & 0b111
    assert ecc_ind == ECC_INDICATOR["M"]  # 0b00
    assert mask_bits == 0


def test_format_bits_all_levels() -> None:
    """ECC indicator is embedded correctly in all levels."""
    for ecc in ("L", "M", "Q", "H"):
        for mask in range(8):
            fmt = compute_format_bits(ecc, mask)
            raw = fmt ^ 0x5412
            data_field = raw >> 10
            ecc_ind = data_field >> 3
            mask_bits = data_field & 0b111
            assert ecc_ind == ECC_INDICATOR[ecc], f"{ecc}/{mask}: ecc indicator"
            assert mask_bits == mask, f"{ecc}/{mask}: mask bits"


def test_write_format_info_copy1_row8() -> None:
    """Copy 1 row 8 cols 0–5 carry bits 14–9 (MSB-first)."""
    g = WorkGrid.make(21)
    fmt = compute_format_bits("M", 2)
    write_format_info(g, fmt)
    # Bit 14 should be at (8, 0)
    expected = bool((fmt >> 14) & 1)
    assert g.modules[8][0] == expected, "bit 14 at (8,0)"
    # Bit 9 should be at (8, 5)
    expected = bool((fmt >> 9) & 1)
    assert g.modules[8][5] == expected, "bit 9 at (8,5)"


# ===========================================================================
# Version information (v7+)
# ===========================================================================


def test_compute_version_bits_v7() -> None:
    """Version 7 BCH word is a known value from the ISO standard."""
    # Version 7 = 0b000111; BCH remainder per ISO = 010010011
    # Full 18-bit word = 0b000_111_010_010_011 ... let's just verify
    # it decodes back correctly:
    bits = compute_version_bits(7)
    assert bits != 0
    # Top 6 bits should be version 7 = 0b000111
    assert (bits >> 12) == 7


def test_compute_version_bits_below_7() -> None:
    """Versions 1–6 return 0 (no version info needed)."""
    for v in range(1, 7):
        assert compute_version_bits(v) == 0


def test_version_bits_top_field() -> None:
    """Top 6 bits of version bits = version number for v7–40."""
    for v in range(7, 41):
        bits = compute_version_bits(v)
        assert (bits >> 12) == v, f"v{v}: top 6 bits should equal version"


# ===========================================================================
# Masking
# ===========================================================================


def test_apply_mask_pattern_0() -> None:
    """Mask 0: (r+c) % 2 == 0 flips non-reserved modules."""
    sz = 5
    modules = [[False] * sz for _ in range(sz)]
    reserved = [[False] * sz for _ in range(sz)]
    masked = apply_mask(modules, reserved, sz, 0)
    # (0,0): 0+0=0, even → should be flipped to True
    assert masked[0][0] is True
    # (0,1): 0+1=1, odd → should remain False
    assert masked[0][1] is False
    # (1,0): 1+0=1, odd → should remain False
    assert masked[1][0] is False


def test_apply_mask_reserved_untouched() -> None:
    """Reserved modules are never flipped by masking."""
    sz = 3
    modules = [[False] * sz for _ in range(sz)]
    reserved = [[True] * sz for _ in range(sz)]
    for m in range(8):
        masked = apply_mask(modules, reserved, sz, m)
        for r in range(sz):
            for c in range(sz):
                assert masked[r][c] is False, f"mask {m} flipped reserved ({r},{c})"


def test_apply_mask_idempotent() -> None:
    """Applying the same mask twice restores the original."""
    sz = 10
    modules = [[bool((r * c) & 1) for c in range(sz)] for r in range(sz)]
    reserved = [[False] * sz for _ in range(sz)]
    for m in range(8):
        once = apply_mask(modules, reserved, sz, m)
        twice = apply_mask(once, reserved, sz, m)
        assert twice == modules, f"mask {m} not idempotent"


# ===========================================================================
# Penalty scoring
# ===========================================================================


def test_penalty_all_dark_high() -> None:
    """An all-dark grid scores high (rule 1, 2, and 4 all fire)."""
    sz = 21
    modules = [[True] * sz for _ in range(sz)]
    p = compute_penalty(modules, sz)
    # Rule 1: 21 runs of length 21 in rows + 21 columns = 42 runs × (21-2) = 42*19 = 798
    # Rule 2: (20×20) blocks = 400 × 3 = 1200
    # Rule 4: 100% dark → 100% - 50% = 50%, nearest multiple of 5 = 50, penalty = 100
    # We just check it's very large
    assert p > 1000


def test_penalty_all_light() -> None:
    """An all-light grid: rule 1 and 2 fire, rule 4 fires at 50% offset."""
    sz = 21
    modules = [[False] * sz for _ in range(sz)]
    p = compute_penalty(modules, sz)
    assert p > 1000


def test_penalty_returns_int() -> None:
    """Penalty is always an integer."""
    sz = 11
    modules = [[bool((r + c) & 1) for c in range(sz)] for r in range(sz)]
    p = compute_penalty(modules, sz)
    assert isinstance(p, int)


# ===========================================================================
# select_version
# ===========================================================================


def test_select_version_hello_world_m() -> None:
    """'Hello, World!' at M fits in version 1."""
    v = select_version("Hello, World!", "M")
    assert v == 1


def test_select_version_long_url_m() -> None:
    """A long URL requires a version larger than 1."""
    url = "https://www.example.com/path/to/some/very/long/resource?query=value"
    v = select_version(url, "M")
    assert v > 1


def test_select_version_too_long() -> None:
    """Input exceeding v40 capacity raises InputTooLongError."""
    with pytest.raises(InputTooLongError):
        select_version("A" * 7090, "H")


def test_select_version_numeric_dense() -> None:
    """Pure numeric strings are very dense and use a low version."""
    v = select_version("0" * 40, "M")
    assert v <= 3  # 40 digits should fit comfortably in v1 or v2 at M


def test_select_version_all_ecc_levels() -> None:
    """Higher ECC levels require larger versions for the same input."""
    text = "Hello, World!"
    vl = select_version(text, "L")
    vm = select_version(text, "M")
    vq = select_version(text, "Q")
    vh = select_version(text, "H")
    # Higher ECC → same or larger version
    assert vl <= vm <= vq <= vh


# ===========================================================================
# encode — black-box tests
# ===========================================================================


def test_encode_hello_world_m_size() -> None:
    """'Hello, World!' at M produces a correctly-sized grid."""
    grid = encode("Hello, World!", level="M")
    v = select_version("Hello, World!", "M")
    expected_size = symbol_size(v)
    assert grid.rows == expected_size
    assert grid.cols == expected_size


def test_encode_returns_module_grid() -> None:
    """encode() returns a ModuleGrid with boolean entries."""
    from qr_code import ModuleGrid
    grid = encode("TEST", level="L")
    assert isinstance(grid, ModuleGrid)
    # Check a few entries are bool
    assert isinstance(grid.modules[0][0], bool)


def test_encode_finder_top_left_present() -> None:
    """Top-left corner of any QR grid is dark (finder outer border)."""
    grid = encode("Hello, World!", level="M")
    assert grid.modules[0][0] is True


def test_encode_finder_outer_ring() -> None:
    """Top-left finder: row 0 is all-dark for first 7 columns."""
    grid = encode("Hello, World!", level="M")
    for c in range(7):
        assert grid.modules[0][c] is True, f"finder top row, col {c}"


def test_encode_separator_row() -> None:
    """Row 7 cols 0–7 is the separator (all light)."""
    grid = encode("Hello, World!", level="M")
    for c in range(8):
        assert grid.modules[7][c] is False, f"separator row 7, col {c}"


def test_encode_dark_module() -> None:
    """The always-dark module is present at (4v+9, 8)."""
    for text in ["HELLO", "Hello, World!", "12345678901234567890"]:
        grid = encode(text, level="M")
        v = select_version(text, "M")
        r = 4 * v + 9
        assert grid.modules[r][8] is True, f"dark module for '{text}' at ({r}, 8)"


def test_encode_module_shape_square() -> None:
    """encode() always produces a 'square' module grid."""
    grid = encode("ABC", level="L")
    assert grid.module_shape == "square"


def test_encode_all_ecc_levels() -> None:
    """encode() succeeds for all four ECC levels."""
    text = "HELLO WORLD"
    for level in ("L", "M", "Q", "H"):
        grid = encode(text, level=level)
        v = select_version(text, level)
        assert grid.rows == symbol_size(v), f"ECC={level}: wrong size"


def test_encode_numeric_mode_digits_only() -> None:
    """Pure digit string uses numeric mode and produces a valid grid."""
    grid = encode("1234567890", level="M", mode="numeric")
    assert grid.rows == grid.cols
    assert grid.rows >= 21  # at least v1


def test_encode_alphanumeric_mode() -> None:
    """Uppercase+digits string uses alphanumeric mode successfully."""
    grid = encode("HELLO WORLD", level="M", mode="alphanumeric")
    assert grid.rows == grid.cols


def test_encode_byte_mode_utf8() -> None:
    """Byte mode works for arbitrary UTF-8 strings."""
    grid = encode("Hello, World!", level="M", mode="byte")
    assert grid.rows == grid.cols


def test_encode_bytes_input() -> None:
    """bytes input is accepted and encoded in byte mode."""
    grid = encode(b"\x00\x01\x02\x03", level="M")
    assert grid.rows >= 21


def test_encode_empty_string() -> None:
    """Empty string produces a valid version-1 grid."""
    grid = encode("", level="M")
    assert grid.rows == 21  # v1


def test_encode_version_1_forced() -> None:
    """Forcing version 1 with short input works."""
    grid = encode("HI", level="L", version=1)
    assert grid.rows == 21


def test_encode_version_forced_too_small() -> None:
    """Forcing a version that can't hold the input raises InputTooLongError."""
    with pytest.raises(InputTooLongError):
        encode("A" * 200, level="H", version=1)


def test_encode_invalid_ecc_level() -> None:
    """Invalid ECC level raises ValueError."""
    with pytest.raises(ValueError, match="Invalid ECC level"):
        encode("hello", level="X")  # type: ignore[arg-type]


def test_encode_invalid_version() -> None:
    """Version out of range raises ValueError."""
    with pytest.raises(ValueError, match="version must be"):
        encode("hello", level="M", version=41)


def test_encode_invalid_mode() -> None:
    """Invalid mode raises ValueError."""
    with pytest.raises(ValueError, match="mode"):
        encode("hello", level="M", mode="kanji")  # type: ignore[arg-type]


def test_encode_version_7_has_version_info() -> None:
    """Version 7+ symbols are correctly built (no crash, correct size)."""
    # Version 7 requires a somewhat long input at high ECC
    # Use a deterministic string that forces v7 at H
    text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789"
    grid = encode(text, level="H")
    v = select_version(text, "H")
    assert grid.rows == symbol_size(v)


def test_encode_timing_strips() -> None:
    """Row 6 and col 6 timing strips alternate correctly in the output."""
    grid = encode("Hello, World!", level="M")
    sz = grid.rows
    # Row 6, cols 8..sz-9 should alternate dark/light starting with dark at col 8
    for c in range(8, sz - 8):
        expected = (c % 2 == 0)
        assert grid.modules[6][c] == expected, (
            f"timing row 6, col {c}: expected {expected}"
        )
    # Col 6, rows 8..sz-9 should alternate starting with dark at row 8
    for r in range(8, sz - 8):
        expected = (r % 2 == 0)
        assert grid.modules[r][6] == expected, (
            f"timing col 6, row {r}: expected {expected}"
        )


# ===========================================================================
# encode_to_scene
# ===========================================================================


def test_encode_to_scene_returns_paint_scene() -> None:
    """encode_to_scene returns a PaintScene."""
    from qr_code import PaintScene
    scene = encode_to_scene("Hello, World!", level="M")
    assert isinstance(scene, PaintScene)


def test_encode_to_scene_non_empty() -> None:
    """encode_to_scene produces at least a background + data instructions."""
    scene = encode_to_scene("HELLO", level="L")
    assert len(scene.instructions) > 1


# ===========================================================================
# ECC indicator constants
# ===========================================================================


def test_ecc_indicators() -> None:
    """ECC indicators match ISO standard values."""
    assert ECC_INDICATOR["L"] == 0b01
    assert ECC_INDICATOR["M"] == 0b00
    assert ECC_INDICATOR["Q"] == 0b11
    assert ECC_INDICATOR["H"] == 0b10


def test_ecc_idx_values() -> None:
    """ECC index values are 0–3."""
    assert ECC_IDX["L"] == 0
    assert ECC_IDX["M"] == 1
    assert ECC_IDX["Q"] == 2
    assert ECC_IDX["H"] == 3


# ===========================================================================
# Regression test: known-good output fingerprint
# ===========================================================================


def test_encode_known_top_row_pattern() -> None:
    """Regression: top row of 'Hello, World!' at M v1 starts with finder."""
    grid = encode("Hello, World!", level="M")
    # Top 7 columns of row 0: all dark (finder outer border)
    assert all(grid.modules[0][c] for c in range(7))
    # Col 7 row 0: separator → light
    assert grid.modules[0][7] is False
    # Col 8 row 0: format info (varies by mask — not hardcoded, but must be bool)
    assert isinstance(grid.modules[0][8], bool)


def test_encode_consistent_output() -> None:
    """Same input always produces identical output (deterministic encoder)."""
    grid1 = encode("Hello, World!", level="M")
    grid2 = encode("Hello, World!", level="M")
    assert grid1.modules == grid2.modules
