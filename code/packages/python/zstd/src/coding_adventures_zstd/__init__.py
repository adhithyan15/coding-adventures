"""zstd — Zstandard (ZStd) lossless compression algorithm — CMP07.

Zstandard (RFC 8878) is a high-ratio, fast compression format created by
Yann Collet at Facebook (2015). It combines:

- **LZ77 back-references** (via LZSS token generation) to exploit repetition
  in the data — the same "copy from earlier in the output" trick as DEFLATE,
  but with a 32 KB sliding window.
- **FSE (Finite State Entropy)** coding instead of Huffman for the sequence
  descriptor symbols. FSE is an asymmetric numeral system that approaches
  the Shannon entropy limit in a single pass.
- **Predefined decode tables** (RFC 8878 Appendix B) so short frames need no
  table description overhead.

Frame Layout (RFC 8878 §3)
===========================

.. code-block:: text

    ┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
    │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
    │ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
    └────────┴─────┴──────────────────────┴────────┴──────────────────┘

Each **block** has a 3-byte header::

    bit 0       = Last_Block flag
    bits [2:1]  = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
    bits [23:3] = Block_Size

Compression Strategy
====================

1. Split data into 128 KB blocks (MAX_BLOCK_SIZE).
2. For each block, try:

   a. **RLE** — all bytes identical → 5 bytes total.
   b. **Compressed** (LZ77 + FSE) — if output < input length.
   c. **Raw** — verbatim copy as fallback.

Series
======

.. code-block:: text

    CMP00 (LZ77)     — Sliding-window back-references
    CMP01 (LZ78)     — Explicit dictionary (trie)
    CMP02 (LZSS)     — LZ77 + flag bits
    CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
    CMP04 (Huffman)  — Entropy coding
    CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
    CMP06 (Brotli)   — DEFLATE + context modelling + static dict
    CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed  ← this package

Examples
========

    >>> from coding_adventures_zstd import compress, decompress
    >>> data = b"the quick brown fox jumps over the lazy dog"
    >>> compressed = compress(data)
    >>> decompress(compressed) == data
    True
"""

from __future__ import annotations

from dataclasses import dataclass

import coding_adventures_lzss as lzss

__all__ = ["compress", "decompress"]

# =============================================================================
# Constants
# =============================================================================
#
# MAGIC is the ZStd frame identifier. Every valid ZStd frame starts with these
# 4 bytes in little-endian order (0x28, 0xB5, 0x2F, 0xFD). The value was chosen
# to be unlikely to appear at the start of plaintext files.

MAGIC: int = 0xFD2FB528

# Maximum block size: 128 KB.
# ZStd allows blocks up to 128 KB. Larger inputs are split across multiple
# blocks. The spec maximum is min(WindowSize, 128 KB).
MAX_BLOCK_SIZE: int = 128 * 1024

# Security: cap decompressed output at 256 MB to prevent decompression bombs.
# A decompression bomb is a tiny compressed file that expands to enormous data,
# causing memory exhaustion. This is a defence-in-depth limit — real ZStd
# implementations use the FCS field for pre-allocation.
MAX_OUTPUT: int = 256 * 1024 * 1024

# =============================================================================
# LL / ML / OF Code Tables (RFC 8878 §3.1.1.3)
# =============================================================================
#
# These tables map a *code number* to a (baseline, extra_bits) pair.
#
# To decode a value: value = baseline + read(extra_bits)
# To encode a value: find last code whose baseline ≤ value; extra = value - baseline
#
# For example, LL code 17 means literal_length = 18 + read(1 extra bit),
# covering literal lengths 18 and 19.

# Literal Length codes: (baseline, extra_bits) for codes 0..35
# Codes 0..15 cover individual values 0..15 (no extra bits needed).
# Codes 16..35 cover grouped ranges with increasing extra bit counts.
LL_CODES: list[tuple[int, int]] = [
    # Codes 0-15: individual values
    (0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0),
    (6, 0), (7, 0), (8, 0), (9, 0), (10, 0), (11, 0),
    (12, 0), (13, 0), (14, 0), (15, 0),
    # Codes 16-19: pairs (1 extra bit each)
    (16, 1), (18, 1), (20, 1), (22, 1),
    # Codes 20-21: quads (2 extra bits each)
    (24, 2), (28, 2),
    # Codes 22-23: octets (3 extra bits each)
    (32, 3), (40, 3),
    # Codes 24-25: 16-tuples / 64-tuples
    (48, 4), (64, 6),
    # Codes 26-35: increasingly wide ranges
    (128, 7), (256, 8), (512, 9), (1024, 10), (2048, 11), (4096, 12),
    (8192, 13), (16384, 14), (32768, 15), (65536, 16),
]

# Match Length codes: (baseline, extra_bits) for codes 0..52
# Minimum match length in ZStd is 3 (not 0). Code 0 = match length 3.
ML_CODES: list[tuple[int, int]] = [
    # Codes 0-31: individual values 3-34
    (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0),
    (9, 0), (10, 0), (11, 0), (12, 0), (13, 0), (14, 0),
    (15, 0), (16, 0), (17, 0), (18, 0), (19, 0), (20, 0),
    (21, 0), (22, 0), (23, 0), (24, 0), (25, 0), (26, 0),
    (27, 0), (28, 0), (29, 0), (30, 0), (31, 0), (32, 0),
    (33, 0), (34, 0),
    # Codes 32-35: pairs (1 extra bit each)
    (35, 1), (37, 1), (39, 1), (41, 1),
    # Codes 36-37: quads (2 extra bits each)
    (43, 2), (47, 2),
    # Codes 38-39: octets (3 extra bits each)
    (51, 3), (59, 3),
    # Codes 40-41: 16-tuples (4 extra bits each)
    (67, 4), (83, 4),
    # Codes 42-43: 32-tuples / 128-tuples
    (99, 5), (131, 7),
    # Codes 44-52: increasingly wide ranges
    (259, 8), (515, 9), (1027, 10), (2051, 11),
    (4099, 12), (8195, 13), (16387, 14), (32771, 15), (65539, 16),
]

# =============================================================================
# FSE Predefined Distributions (RFC 8878 Appendix B)
# =============================================================================
#
# "Predefined_Mode" means no per-frame table description is transmitted.
# The decoder builds the same table from these fixed distributions.
#
# Entries of -1 mean "probability 1/table_size" — these symbols each get
# exactly one slot in the decode table. Their encoder state never needs
# extra bits because the table walk visits them at the "high" end indices.

# Predefined normalised distribution for Literal Length FSE.
# Table accuracy log = 6 → 64 slots.
# The distribution is derived from statistics over large corpora of text.
LL_NORM: list[int] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1,
    2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
]
LL_ACC_LOG: int = 6  # table_size = 64

# Predefined normalised distribution for Match Length FSE.
# Table accuracy log = 6 → 64 slots.
ML_NORM: list[int] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1,
    -1, -1, -1, -1, -1,
]
ML_ACC_LOG: int = 6

# Predefined normalised distribution for Offset FSE.
# Table accuracy log = 5 → 32 slots.
OF_NORM: list[int] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
]
OF_ACC_LOG: int = 5  # table_size = 32


# =============================================================================
# FSE Decode Table Builder
# =============================================================================
#
# An FSE decode table maps a state index to (symbol, nb, base):
#   - sym:  the decoded symbol
#   - nb:   number of extra bits to read for the NEXT state
#   - base: added to those bits to form the next state
#
# The table has `sz = 1 << acc_log` entries. Valid encoder states are in
# [sz, 2*sz); valid table indices are [0, sz).


def _build_decode_table(norm: list[int], acc_log: int) -> list[dict]:
    """Build an FSE decode table from a normalised probability distribution.

    The algorithm:
      1. Place symbols with probability -1 (very rare) at the TOP of the table
         (high indices). These symbols each get exactly 1 slot.
      2. Spread remaining symbols using a deterministic step function derived
         from the table size. This ensures each symbol occupies the correct
         fraction of slots.
      3. Assign ``nb`` (bits to read) and ``base`` to each slot so that the
         decoder can reconstruct the next state.

    The step function ``step = (sz >> 1) + (sz >> 3) + 3`` is co-prime to ``sz``
    when ``sz`` is a power of two (which it always is in ZStd), ensuring that
    the walk visits every slot exactly once.

    Args:
        norm: Normalised probability distribution. -1 entries are "very rare".
        acc_log: Accuracy log; table has 2**acc_log entries.

    Returns:
        List of dicts ``{sym, nb, base}`` indexed by FSE state.
    """
    sz = 1 << acc_log
    step = (sz >> 1) + (sz >> 3) + 3

    # Initialise table with placeholder dicts.
    tbl: list[dict] = [{"sym": 0, "nb": 0, "base": 0} for _ in range(sz)]

    # sym_next[s] tracks the "next state counter" for symbol s during Phase 3.
    # After Phase 2, sym_next[s] = number of times symbol s appears in tbl.
    sym_next = [0] * len(norm)

    # ── Phase 1: symbols with probability -1 go to the HIGH end of the table ──
    #
    # Why "high end"? The spreading step function walks upward from pos=0.
    # By reserving the high indices for rare symbols, they don't interfere
    # with the main spreading walk. Each -1 symbol gets exactly one slot.
    high = sz - 1
    for s, c in enumerate(norm):
        if c == -1:
            tbl[high]["sym"] = s
            if high > 0:
                high -= 1
            sym_next[s] = 1

    # ── Phase 2: spread remaining symbols using the step function ────────────
    #
    # Two-pass approach: first symbols with count > 1, then count == 1.
    # This matches the reference implementation's deterministic ordering and
    # ensures the same spread across Python, Rust, and C implementations.
    pos = 0
    for pass_num in range(2):
        for s, c in enumerate(norm):
            if c <= 0:
                continue
            cnt = int(c)
            # Pass 0: handle symbols with count > 1 first.
            # Pass 1: handle symbols with count == 1.
            if (pass_num == 0) != (cnt > 1):
                continue
            sym_next[s] = cnt
            for _ in range(cnt):
                tbl[pos]["sym"] = s
                pos = (pos + step) & (sz - 1)
                # Skip positions that are reserved for -1 symbols (> high).
                while pos > high:
                    pos = (pos + step) & (sz - 1)

    # ── Phase 3: assign nb (state bits) and base ─────────────────────────────
    #
    # For each slot i in the table:
    #   The symbol s = tbl[i].sym has been seen sym_next[s] times so far.
    #   ns = current sym_next[s] (starts at count, increments each time s appears)
    #   nb = acc_log - floor(log2(ns))     — how many bits to read for next state
    #   base = ns * (1 << nb) - sz         — offset for next state reconstruction
    #
    # The decoder computes: next_state = base + read(nb bits)
    # This lands in [sz, 2*sz) (the valid encoder state range) because:
    #   ns * (1<<nb) is the smallest power-of-2-multiple of ns that's >= sz.
    #   So base + any nb-bit value covers exactly cnt encoder states.
    sn = list(sym_next)  # working copy of sym_next for Phase 3
    for i in range(sz):
        s = tbl[i]["sym"]
        ns = sn[s]
        sn[s] += 1
        # floor(log2(ns)) via bit_length: log2(ns) = bit_length(ns) - 1
        nb = acc_log - (ns.bit_length() - 1)
        base = ns * (1 << nb) - sz
        tbl[i]["nb"] = nb
        tbl[i]["base"] = base

    return tbl


# =============================================================================
# FSE Encode Symbol Table Builder
# =============================================================================
#
# The encoder needs the inverse of the decode table: given a symbol, how do
# we update state and how many bits do we flush?
#
# Returns (ee_list, st_list) where:
#   ee_list[sym] = {"delta_nb": int, "delta_fs": int}
#   st_list[slot] = encoder output state (in [sz, 2*sz))
#
# Encoder state update for symbol `s` at state E:
#   nb_out = (E + delta_nb) >> 16      (number of state bits to flush)
#   emit low nb_out bits of E
#   slot_i = (E >> nb_out) + delta_fs
#   new_E = st_list[slot_i]            (in [sz, 2*sz))


def _build_encode_sym(
    norm: list[int], acc_log: int
) -> tuple[list[dict], list[int]]:
    """Build FSE encode tables from a normalised distribution.

    Returns:
        Tuple of (ee_list, st_list) where:
          ee_list[sym]: dict with ``delta_nb`` and ``delta_fs`` for encoding sym
          st_list[slot]: encoder output state (in [sz, 2*sz))
    """
    sz = 1 << acc_log

    # ── Step 1: compute cumulative sums (prefix sums of counts) ──────────────
    #
    # cumul[s] = sum of counts for all symbols before s.
    # This gives each symbol a contiguous range [cumul[s], cumul[s]+cnt) of
    # "encode slots". The encoder uses these slots to index st_list.
    cumul = [0] * len(norm)
    total = 0
    for s, c in enumerate(norm):
        cumul[s] = total
        cnt = 1 if c == -1 else max(0, int(c))
        total += cnt

    # ── Step 2: build the spread table ───────────────────────────────────────
    #
    # Same spreading algorithm as _build_decode_table: determines which symbol
    # occupies each table index. We need this to map table indices → encode slots.
    step = (sz >> 1) + (sz >> 3) + 3
    spread = [0] * sz
    idx_high = sz - 1

    # Phase 1: -1 symbols at the high end
    for s, c in enumerate(norm):
        if c == -1:
            spread[idx_high] = s
            if idx_high > 0:
                idx_high -= 1
    idx_limit = idx_high

    # Phase 2: spread remaining symbols
    pos = 0
    for pass_num in range(2):
        for s, c in enumerate(norm):
            if c <= 0:
                continue
            cnt = int(c)
            if (pass_num == 0) != (cnt > 1):
                continue
            for _ in range(cnt):
                spread[pos] = s
                pos = (pos + step) & (sz - 1)
                while pos > idx_limit:
                    pos = (pos + step) & (sz - 1)

    # ── Step 3: build the state table ─────────────────────────────────────────
    #
    # Iterate spread in INDEX ORDER (i = 0, 1, ..., sz-1).
    # For each table index i, determine which "occurrence" j this is for symbol s:
    #   sym_occ[s] counts how many times s has appeared so far in index order.
    #   encode slot = cumul[s] + j
    #   encoder output state = i + sz   (so the decoder at index i will decode s)
    #
    # The encoder at encode slot `cumul[s]+j` will output state `i+sz`, which
    # the decoder reads as state index i and recovers symbol s.
    sym_occ = [0] * len(norm)
    st: list[int] = [0] * sz
    for i in range(sz):
        s = spread[i]
        j = sym_occ[s]
        sym_occ[s] += 1
        slot = cumul[s] + j
        st[slot] = i + sz  # encode slot → decoder table index + sz

    # ── Step 4: build FseEe entries ───────────────────────────────────────────
    #
    # For symbol s with count cnt:
    #   mbo = max_bits_out:
    #     if cnt == 1: mbo = acc_log          (full-width, any state maps to this sym)
    #     else:        mbo = acc_log - floor(log2(cnt))
    #   delta_nb = (mbo << 16) - (cnt << mbo)
    #   delta_fs = cumul[s] - cnt
    #
    # The encoder computes nb_out = (E + delta_nb) >> 16, which correctly gives
    # the number of bits to flush from state E for symbol s.
    ee: list[dict] = [{"delta_nb": 0, "delta_fs": 0} for _ in range(len(norm))]
    for s, c in enumerate(norm):
        cnt = 1 if c == -1 else max(0, int(c))
        if cnt == 0:
            continue
        # mbo: the maximum number of bits we'd ever flush for this symbol.
        # When cnt == 1, the symbol has a unique encode slot, so we always
        # flush exactly acc_log bits regardless of state.
        mbo = acc_log if cnt == 1 else acc_log - (cnt.bit_length() - 1)
        ee[s]["delta_nb"] = (mbo << 16) - (cnt << mbo)
        ee[s]["delta_fs"] = cumul[s] - cnt

    return ee, st


# =============================================================================
# Reverse Bit Writer
# =============================================================================
#
# ZStd's sequence bitstream is written *backwards* relative to the data flow:
# the encoder writes bits that the decoder will read LAST first. This allows
# the decoder to read a forward-only stream while decoding sequences in order.
#
# Byte layout: [byte0, byte1, ..., byteN] where byteN is the LAST byte written,
# and it contains a **sentinel bit** (the highest set bit) that marks the end
# of meaningful data. The decoder initialises by finding this sentinel.
#
# Bit layout within each byte: LSB = first bit written.
#
# Example: write bits 1, 0, 1, 1 (4 bits) then flush:
#   reg = 0b1011, bits = 4
#   flush: sentinel at bit 4 → last byte = 0b0001_1011 = 0x1B
#   buf = [0x1B]
#
# The decoder reads: find MSB (bit 4 = sentinel), then read bits 3..0 =
# 0b1011 = the original 4 bits.


class _RevBitWriter:
    """Accumulates bits for the ZStd backward bitstream.

    Bits are packed LSB-first into bytes. A sentinel bit marks the end.
    """

    def __init__(self) -> None:
        self.reg: int = 0    # accumulation register (LSB side)
        self.bits: int = 0   # number of valid bits in reg
        self.buf: list[int] = []  # accumulated complete bytes

    def add_bits(self, val: int, nb: int) -> None:
        """Add the low ``nb`` bits of ``val`` to the stream.

        Args:
            val: Value to write (only low nb bits are used).
            nb: Number of bits to write (0 is a no-op).
        """
        if nb == 0:
            return
        mask = (1 << nb) - 1
        self.reg |= (val & mask) << self.bits
        self.bits += nb
        # Flush complete bytes as they fill up.
        while self.bits >= 8:
            self.buf.append(self.reg & 0xFF)
            self.reg >>= 8
            self.bits -= 8

    def flush(self) -> None:
        """Flush remaining bits with a sentinel bit.

        The sentinel is a '1' placed at position ``self.bits`` in the last byte.
        The decoder finds the sentinel by scanning for the highest set bit in
        the last byte. This tells it exactly how many valid data bits are present.

        Example: 4 data bits remain → sentinel at bit 4 → last_byte bit pattern:
          bits [3:0] = data, bit [4] = 1, bits [7:5] = 0.
        """
        sentinel = 1 << self.bits  # one bit above all data bits
        last_byte = (self.reg & 0xFF) | sentinel
        self.buf.append(last_byte)
        self.reg = 0
        self.bits = 0

    def finish(self) -> bytes:
        """Return the accumulated bitstream as bytes."""
        return bytes(self.buf)


# =============================================================================
# Reverse Bit Reader
# =============================================================================
#
# Mirrors RevBitWriter: reads bits from the END of the buffer going backwards.
# The stream is laid out so that the LAST bits written by the encoder are at
# the END of the byte buffer (in the sentinel-containing last byte). The reader
# initialises at the last byte and reads backward toward byte 0.
#
# Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side of a
# 64-bit integer). ``read_bits(n)`` extracts the top n bits and shifts left by n.
#
# Why left-aligned? The writer accumulates bits LSB-first. Within each flushed
# byte, bit 0 = earliest written, bit 7 = latest written. To read the LATEST
# bits first (which correspond to the highest byte positions), we need a
# left-aligned register so that reading from the top gives the highest-position
# bits first.


class _RevBitReader:
    """Reads bits from a ZStd backward bitstream.

    Initialise from the last byte (which contains the sentinel), then call
    ``read_bits(n)`` to extract bits in the reverse of write order.
    """

    def __init__(self, data: bytes) -> None:
        """Initialise the reader.

        Args:
            data: The entire bitstream bytes. Must not be empty.

        Raises:
            ValueError: If the data is empty or the sentinel byte is zero.
        """
        if not data:
            raise ValueError("empty bitstream")
        last = data[-1]
        if last == 0:
            raise ValueError("bitstream last byte is zero (no sentinel)")

        # Find the sentinel: highest set bit in the last byte.
        # sentinel_pos = index of sentinel bit (0 = LSB, 7 = MSB)
        # valid_bits  = number of data bits BELOW the sentinel
        #
        # Example: last = 0b00011110
        #   bit_length() = 5 (highest set bit is bit 4)
        #   sentinel_pos = 4
        #   valid_bits   = 4 (bits [3:0] are data)
        sentinel_pos = last.bit_length() - 1  # = floor(log2(last))
        valid_bits = sentinel_pos  # data bits below sentinel

        # Place the valid bits of the sentinel byte at the TOP (MSB side) of
        # a 64-bit register. Bits fill from the top downward.
        mask = (1 << valid_bits) - 1 if valid_bits > 0 else 0
        if valid_bits > 0:
            self.reg: int = (last & mask) << (64 - valid_bits)
        else:
            self.reg = 0
        self.bits: int = valid_bits     # how many valid bits are in the register
        self.pos: int = len(data) - 1  # next byte to load (decrement toward 0)
        self.data = data

        # Pre-fill the register from earlier bytes.
        self._reload()

    def _reload(self) -> None:
        """Load more bytes from the stream into the register.

        Each new byte is placed just BELOW the currently loaded bits (in a
        left-aligned register, that means at position ``64 - bits - 8``).
        We load until we have at least 56 valid bits or reach the start.
        """
        while self.bits <= 56 and self.pos > 0:
            self.pos -= 1
            shift = 64 - self.bits - 8
            self.reg |= self.data[self.pos] << shift
            self.bits += 8

    def read_bits(self, nb: int) -> int:
        """Read ``nb`` bits from the top of the register.

        The most recently written bits are at the top, mirroring the encoder's
        backward order.

        Args:
            nb: Number of bits to read (0 returns 0 without consuming anything).

        Returns:
            The extracted ``nb``-bit value.
        """
        if nb == 0:
            return 0
        # Extract the top nb bits.
        val = self.reg >> (64 - nb)
        # Shift the register left, consuming those bits.
        # Use the mask to stay within 64 bits (Python integers don't overflow,
        # but we must truncate to maintain the 64-bit register model).
        self.reg = (self.reg << nb) & 0xFFFF_FFFF_FFFF_FFFF
        self.bits -= nb
        if self.bits < 24:
            self._reload()
        return val


# =============================================================================
# FSE Encode / Decode Helpers
# =============================================================================


def _fse_encode_sym(
    state: int,
    sym: int,
    ee: list[dict],
    st: list[int],
    bw: _RevBitWriter,
) -> int:
    """Encode one symbol into the backward bitstream, returning the new state.

    The encoder maintains state in ``[sz, 2*sz)``. To emit symbol ``sym``:
      1. Compute how many bits to flush: ``nb = (state + delta_nb) >> 16``
      2. Write the low ``nb`` bits of state to the bitstream.
      3. New state = ``st[(state >> nb) + delta_fs]``

    Args:
        state: Current encoder state in [sz, 2*sz).
        sym: Symbol to encode.
        ee: Encode table from _build_encode_sym.
        st: State table from _build_encode_sym.
        bw: Bit writer to flush bits into.

    Returns:
        New encoder state.
    """
    e = ee[sym]
    nb = (state + e["delta_nb"]) >> 16
    bw.add_bits(state, nb)
    slot_i = (state >> nb) + e["delta_fs"]
    return st[slot_i]


def _fse_decode_sym(
    state: int, de: list[dict], br: _RevBitReader
) -> tuple[int, int]:
    """Decode one symbol from the backward bitstream, returning (sym, new_state).

    Steps:
      1. Look up de[state] to get sym, nb, and base.
      2. Read nb bits from the bitstream.
      3. New state = base + those bits.

    Args:
        state: Current decoder state (index into decode table).
        de: Decode table from _build_decode_table.
        br: Bit reader.

    Returns:
        Tuple of (decoded_symbol, new_state).
    """
    e = de[state]
    sym = e["sym"]
    next_state = e["base"] + br.read_bits(e["nb"])
    return sym, next_state


# =============================================================================
# LL / ML Code Computation
# =============================================================================


def _ll_to_code(ll: int) -> int:
    """Map a literal length value to its LL code number (0..35).

    Performs a linear scan over LL_CODES, returning the last code whose
    baseline is ≤ ll. Codes are sorted by increasing baseline, so the last
    matching code is the tightest fit.

    Args:
        ll: Literal length value.

    Returns:
        LL code number in [0, 35].
    """
    code = 0
    for i, (base, _bits) in enumerate(LL_CODES):
        if base <= ll:
            code = i
        else:
            break
    return code


def _ml_to_code(ml: int) -> int:
    """Map a match length value to its ML code number (0..52).

    Args:
        ml: Match length value (minimum 3).

    Returns:
        ML code number in [0, 52].
    """
    code = 0
    for i, (base, _bits) in enumerate(ML_CODES):
        if base <= ml:
            code = i
        else:
            break
    return code


# =============================================================================
# Sequence Data Type
# =============================================================================


@dataclass
class _Seq:
    """One ZStd sequence: (literal_length, match_length, match_offset).

    A sequence means: emit ``ll`` literal bytes from the literals section,
    then copy ``ml`` bytes starting ``off`` positions back in the output buffer.
    After all sequences, any remaining literals in the literals buffer are appended.

    Attributes:
        ll: Literal length (bytes to copy from literal section before this match).
        ml: Match length (bytes to copy from output history).
        off: Match offset (1-indexed: 1 = last byte written).
    """

    ll: int
    ml: int
    off: int


def _tokens_to_seqs(tokens: list) -> tuple[bytes, list[_Seq]]:
    """Convert LZSS tokens into ZStd sequences and a flat literals buffer.

    LZSS produces a stream of Literal(byte) and Match(offset, length) tokens.
    ZStd groups consecutive literals before each match into a single sequence.
    Any trailing literals (after the last match) stay in the literals buffer
    without a corresponding sequence entry — the decompressor emits them after
    the last sequence.

    Args:
        tokens: LZSS token list from lzss.encode().

    Returns:
        Tuple of (lits_bytes, seqs_list).
    """
    lits: list[int] = []
    seqs: list[_Seq] = []
    lit_run: int = 0  # count of literals accumulated since last match

    for tok in tokens:
        if isinstance(tok, lzss.Literal):
            lits.append(tok.byte)
            lit_run += 1
        elif isinstance(tok, lzss.Match):
            seqs.append(_Seq(ll=lit_run, ml=tok.length, off=tok.offset))
            lit_run = 0
        # Trailing literals stay in `lits`; no sequence for them.

    return bytes(lits), seqs


# =============================================================================
# Literals Section Encoding / Decoding
# =============================================================================
#
# ZStd supports Huffman-coded or raw literals. We use Raw_Literals (type=0),
# the simplest form: no Huffman table, bytes are stored verbatim.
#
# Header format (RFC 8878 §3.1.1.2.1):
#   bits [1:0] = Literals_Block_Type (0 = Raw)
#   bits [3:2] = Size_Format:
#     0b00 or 0b10 → 1-byte header: size in bits [7:3]  (5-bit, max 31)
#     0b01          → 2-byte LE:    size in bits [11:4] (12-bit, max 4095)
#     0b11          → 3-byte LE:    size in bits [19:4] (16-bit, max 65535)


def _encode_literals_section(lits: bytes) -> bytes:
    """Encode the literals section header + verbatim bytes.

    Uses Raw_Literals format (no Huffman, simplest header).

    Args:
        lits: Raw literal bytes to encode.

    Returns:
        Header bytes followed by the literal bytes.
    """
    n = len(lits)
    out = bytearray()

    if n <= 31:
        # 1-byte header: bits [7:3] = size, bits [3:2] = 00 (size_format),
        # bits [1:0] = 00 (Raw type). Net: header = (n << 3).
        out.append((n << 3) & 0xFF)
    elif n <= 4095:
        # 2-byte LE header: size_format=01, type=00 → low nibble = 0b0100 = 0x04
        hdr = (n << 4) | 0x04
        out.append(hdr & 0xFF)
        out.append((hdr >> 8) & 0xFF)
    else:
        # 3-byte LE header: size_format=11, type=00 → low nibble = 0b1100 = 0x0C
        hdr = (n << 4) | 0x0C
        out.append(hdr & 0xFF)
        out.append((hdr >> 8) & 0xFF)
        out.append((hdr >> 16) & 0xFF)

    out.extend(lits)
    return bytes(out)


def _decode_literals_section(data: bytes) -> tuple[bytes, int]:
    """Decode the literals section, returning (literals, bytes_consumed).

    Only Raw_Literals (type=0) is supported — the format our encoder produces.

    Args:
        data: Bytes starting at the literals section.

    Returns:
        Tuple of (literal_bytes, header_plus_literal_byte_count).

    Raises:
        ValueError: On empty input, unsupported type, or truncated data.
    """
    if not data:
        raise ValueError("empty literals section")

    b0 = data[0]
    ltype = b0 & 0b11         # bits [1:0] = Literals_Block_Type
    size_format = (b0 >> 2) & 0b11  # bits [3:2] = Size_Format

    if ltype != 0:
        raise ValueError(
            f"unsupported literals type {ltype} (only Raw=0 supported)"
        )

    # Decode literal count and header byte count based on size_format.
    if size_format in (0, 2):
        # 1-byte header: 5-bit size in bits [7:3]
        n = b0 >> 3
        header_bytes = 1
    elif size_format == 1:
        # 2-byte header: 12-bit size spanning bytes 0 and 1
        if len(data) < 2:
            raise ValueError("truncated literals header (2-byte)")
        # Bits [11:8] from byte1, bits [7:4] from byte0.
        # Byte0 bit layout: [type=2b][sf=2b][size[3:0]]
        # Combined: n = (b0 >> 4) | (b1 << 4)
        n = (b0 >> 4) | (data[1] << 4)
        header_bytes = 2
    else:  # size_format == 3
        # 3-byte header: 20-bit size spanning bytes 0, 1, and 2
        if len(data) < 3:
            raise ValueError("truncated literals header (3-byte)")
        n = (b0 >> 4) | (data[1] << 4) | (data[2] << 12)
        header_bytes = 3

    end = header_bytes + n
    if end > len(data):
        raise ValueError(
            f"literals data truncated: need {end} bytes, have {len(data)}"
        )

    return data[header_bytes:end], end


# =============================================================================
# Sequence Count Encoding / Decoding
# =============================================================================
#
# Sequence count uses a variable-length encoding (RFC 8878 §3.1.2.2):
#   0..127:     1 byte  = count
#   128..32767: 2 bytes where byte0 has bit 7 set
#               byte0 = 0x80 | ((count - 0x80) >> 8)
#               byte1 = (count - 0x80) & 0xFF
#               decode: count = (((b0 & 0x7F) << 8) | b1) + 0x80
#   32768+:     3 bytes = [0xFF, (count-0x7F00)&0xFF, (count-0x7F00)>>8]
#
# The 2-byte encoding ensures byte0 always has bit 7 set (>= 0x80), which
# is the signal to the decoder that this is a 2-byte (not 1-byte) count.
# This supports all counts 128..32767 correctly, including multiples of 256.


def _encode_seq_count(count: int) -> bytes:
    """Encode a sequence count to 1, 2, or 3 bytes (RFC 8878 §3.1.2.2).

    Args:
        count: Number of sequences (≥ 0).

    Returns:
        1–3 bytes encoding the count.
    """
    if count == 0:
        return b"\x00"
    if count < 128:
        return bytes([count])
    if count < 0x7F80:
        # 2-byte encoding: always has b0 in [0x80..0xFE] so the decoder can
        # distinguish from a 1-byte value (b0 < 128) and 3-byte (b0 == 0xFF).
        # Encodes counts 128..32639. Above that, b0 would reach 0xFF.
        raw = count - 0x80
        b0 = 0x80 | (raw >> 8)
        b1 = raw & 0xFF
        return bytes([b0, b1])
    # 3-byte encoding: first byte 0xFF, then (count - 0x7F00) as LE u16
    r = count - 0x7F00
    return bytes([0xFF, r & 0xFF, (r >> 8) & 0xFF])


def _decode_seq_count(data: bytes) -> tuple[int, int]:
    """Decode a sequence count, returning (count, bytes_consumed).

    Args:
        data: Bytes starting at the sequence count field.

    Returns:
        Tuple of (sequence_count, bytes_consumed).

    Raises:
        ValueError: On empty or truncated data.
    """
    if not data:
        raise ValueError("empty sequence count field")
    b0 = data[0]
    if b0 < 128:
        # 1-byte encoding: value is the count directly.
        return b0, 1
    if b0 < 0xFF:
        # 2-byte encoding: b0 has bit 7 set.
        # count = (((b0 & 0x7F) << 8) | b1) + 0x80
        if len(data) < 2:
            raise ValueError("truncated sequence count (2-byte)")
        count = (((b0 & 0x7F) << 8) | data[1]) + 0x80
        return count, 2
    # 3-byte encoding: b0 == 0xFF
    if len(data) < 3:
        raise ValueError("truncated sequence count (3-byte)")
    return 0x7F00 + data[1] + (data[2] << 8), 3


# =============================================================================
# Sequences Section Encoding
# =============================================================================
#
# Layout:
#   [sequence_count: 1-3 bytes]
#   [symbol_compression_modes: 1 byte]   (0x00 = all Predefined)
#   [FSE bitstream: variable]
#
# Symbol compression modes byte:
#   bits [7:6] = LL mode
#   bits [5:4] = OF mode
#   bits [3:2] = ML mode
#   bits [1:0] = reserved (0)
# Mode 0 = Predefined, Mode 1 = RLE, Mode 2 = FSE_Compressed, Mode 3 = Repeat.
# We always write 0x00 (all Predefined).
#
# The FSE bitstream is a BACKWARD bit-stream:
#   - Sequences are encoded in REVERSE ORDER (last sequence first).
#   - For each sequence:
#       OF extra bits (of_code bits), ML extra bits, LL extra bits
#       then FSE encode ML, OF, LL  (reverse of decode order LL, OF, ML)
#   - After all sequences, flush the final FSE states:
#       (state_of - sz_of) as OF_ACC_LOG bits
#       (state_ml - sz_ml) as ML_ACC_LOG bits
#       (state_ll - sz_ll) as LL_ACC_LOG bits
#   - Add sentinel and flush.
#
# The decoder does the mirror:
#   1. Read LL_ACC_LOG bits → initial state_ll
#   2. Read ML_ACC_LOG bits → initial state_ml
#   3. Read OF_ACC_LOG bits → initial state_of
#   4. For each sequence:
#       decode LL symbol (state transition + read LL extra bits)
#       decode OF symbol (state transition + read OF extra bits)
#       decode ML symbol (state transition + read ML extra bits)
#   5. Apply each sequence to the output buffer.


def _encode_sequences_section(seqs: list[_Seq]) -> bytes:
    """Encode sequences using predefined FSE tables to a bitstream.

    Sequences are encoded in reverse order so the decoder can process them
    in forward order from a backward bitstream.

    Args:
        seqs: List of ZStd sequences to encode.

    Returns:
        FSE bitstream bytes (without the count or modes byte).
    """
    # Build encode tables from the predefined distributions.
    ee_ll, st_ll = _build_encode_sym(LL_NORM, LL_ACC_LOG)
    ee_ml, st_ml = _build_encode_sym(ML_NORM, ML_ACC_LOG)
    ee_of, st_of = _build_encode_sym(OF_NORM, OF_ACC_LOG)

    sz_ll = 1 << LL_ACC_LOG
    sz_ml = 1 << ML_ACC_LOG
    sz_of = 1 << OF_ACC_LOG

    # FSE encoder states start at table_size.
    # Valid encoder state range is [sz, 2*sz).
    state_ll = sz_ll
    state_ml = sz_ml
    state_of = sz_of

    bw = _RevBitWriter()

    # Encode sequences in REVERSE ORDER.
    # The backward bitstream reverses the order when read, so encoding last-to-first
    # means the decoder processes first-to-last.
    for seq in reversed(seqs):
        ll_code = _ll_to_code(seq.ll)
        ml_code = _ml_to_code(seq.ml)

        # Offset encoding (RFC 8878 §3.1.1.3.2.1):
        #   raw_off = match_offset + 3   (the +3 avoids codes 0,1,2 reserved for
        #                                 "repeat offset" in the full spec)
        #   of_code = floor(log2(raw_off))   — the code number
        #   of_extra = raw_off - (1 << of_code)  — the extra bits
        #
        # Decoding inverse: raw_off = (1 << of_code) | of_extra; offset = raw_off - 3
        raw_off = seq.off + 3
        of_code = (raw_off.bit_length() - 1) if raw_off > 1 else 0
        of_extra = raw_off - (1 << of_code)

        # Write extra bits FIRST in the backward stream.
        # In the backward stream, writing order is REVERSE of reading order.
        # Decoder reads: OF extra, ML extra, LL extra (after decoding symbols).
        # So encoder writes: LL extra, ML extra, OF extra (reverse).
        # But ZStd spec says write: OF extra, ML extra, LL extra.
        # Wait — let's match the Rust reference exactly:
        #   Rust writes: of_extra, ml_extra, ll_extra  (as bw.add_bits)
        # These will be read in reverse by the decoder.
        bw.add_bits(of_extra, of_code)                       # OF extra bits
        ml_extra = seq.ml - ML_CODES[ml_code][0]
        bw.add_bits(ml_extra, ML_CODES[ml_code][1])          # ML extra bits
        ll_extra = seq.ll - LL_CODES[ll_code][0]
        bw.add_bits(ll_extra, LL_CODES[ll_code][1])          # LL extra bits

        # FSE encode symbols in REVERSE of decode order.
        # Decode order: LL, OF, ML (symbols).
        # Encode order (reversed): ML, OF, LL.
        state_ml = _fse_encode_sym(state_ml, ml_code, ee_ml, st_ml, bw)
        state_of = _fse_encode_sym(state_of, of_code, ee_of, st_of, bw)
        state_ll = _fse_encode_sym(state_ll, ll_code, ee_ll, st_ll, bw)

    # Flush final FSE states at the END of the bitstream.
    # The decoder reads these FIRST (it initialises from the top of the stream).
    # The final state minus sz gives the initial table index for the decoder.
    bw.add_bits(state_of - sz_of, OF_ACC_LOG)
    bw.add_bits(state_ml - sz_ml, ML_ACC_LOG)
    bw.add_bits(state_ll - sz_ll, LL_ACC_LOG)
    bw.flush()  # add sentinel bit

    return bw.finish()


# =============================================================================
# Block-Level Compress / Decompress
# =============================================================================


def _compress_block(block: bytes) -> bytes | None:
    """Compress one block into ZStd compressed block format.

    Uses LZSS to generate LZ77 tokens, converts to ZStd sequences, then
    FSE-encodes them.

    Args:
        block: Raw block data (≤ MAX_BLOCK_SIZE bytes).

    Returns:
        Compressed bytes, or None if compression is not beneficial (i.e.
        compressed form is ≥ original size).
    """
    # Use LZSS to generate LZ77 back-reference tokens.
    # Window = 32 KB, max match = 255, min match = 3.
    # Larger window (32 KB vs. default 4 KB) gives better compression ratio.
    tokens = lzss.encode(block, 32768, 255, 3)

    # Convert LZSS tokens to ZStd (literals, sequences) representation.
    lits, seqs = _tokens_to_seqs(tokens)

    # If no back-references were found, LZ77 had nothing to compress.
    # A compressed block with 0 sequences still has overhead, so fall back to raw.
    if not seqs:
        return None

    out = bytearray()

    # Encode the literals section (Raw_Literals format).
    out.extend(_encode_literals_section(lits))

    # Encode the sequence count (variable-length 1–3 bytes).
    out.extend(_encode_seq_count(len(seqs)))

    # Symbol compression modes byte: 0x00 = all Predefined FSE tables.
    out.append(0x00)

    # Encode the FSE bitstream for sequences.
    out.extend(_encode_sequences_section(seqs))

    compressed = bytes(out)
    if len(compressed) >= len(block):
        return None  # Not beneficial — caller should use raw or RLE
    return compressed


def _decompress_block(data: bytes, out: bytearray) -> None:
    """Decompress one ZStd compressed block, appending to ``out``.

    Parses the literals section, sequences section, and applies sequences
    to reconstruct the original data.

    Args:
        data: Compressed block data (without the 3-byte block header).
        out: Output buffer to append decompressed bytes to.

    Raises:
        ValueError: On malformed data, unsupported modes, or bad back-references.
    """
    # ── Parse literals section ────────────────────────────────────────────────
    lits, lit_consumed = _decode_literals_section(data)
    pos = lit_consumed

    # ── Check for sequences section ───────────────────────────────────────────
    if pos >= len(data):
        # Block has only literals, no sequences.
        out.extend(lits)
        return

    # ── Parse sequence count ─────────────────────────────────────────────────
    n_seqs, sc_bytes = _decode_seq_count(data[pos:])
    pos += sc_bytes

    if n_seqs == 0:
        # No sequences — all content is in the literals section.
        out.extend(lits)
        return

    # ── Parse symbol compression modes byte ──────────────────────────────────
    if pos >= len(data):
        raise ValueError("missing symbol compression modes byte")
    modes_byte = data[pos]
    pos += 1

    # Extract mode for each table from the modes byte.
    # bits [7:6] = LL mode, bits [5:4] = OF mode, bits [3:2] = ML mode
    ll_mode = (modes_byte >> 6) & 3
    of_mode = (modes_byte >> 4) & 3
    ml_mode = (modes_byte >> 2) & 3
    if ll_mode != 0 or of_mode != 0 or ml_mode != 0:
        raise ValueError(
            f"unsupported FSE modes: LL={ll_mode} OF={of_mode} ML={ml_mode} "
            f"(only Predefined=0 supported)"
        )

    # ── Parse FSE bitstream ───────────────────────────────────────────────────
    bitstream = data[pos:]
    br = _RevBitReader(bitstream)

    # Build decode tables from the predefined distributions.
    dt_ll = _build_decode_table(LL_NORM, LL_ACC_LOG)
    dt_ml = _build_decode_table(ML_NORM, ML_ACC_LOG)
    dt_of = _build_decode_table(OF_NORM, OF_ACC_LOG)

    # Initialise FSE states.
    # The encoder wrote these last (at the "top" of the backward stream),
    # so they are the FIRST things the decoder reads.
    state_ll = br.read_bits(LL_ACC_LOG)
    state_ml = br.read_bits(ML_ACC_LOG)
    state_of = br.read_bits(OF_ACC_LOG)

    lit_pos = 0  # current position in the literals buffer

    for _ in range(n_seqs):
        # ── Decode FSE symbols (state transitions) ────────────────────────────
        # Decode order: LL, OF, ML  (encoder used ML, OF, LL in reverse).
        ll_code, state_ll = _fse_decode_sym(state_ll, dt_ll, br)
        of_code, state_of = _fse_decode_sym(state_of, dt_of, br)
        ml_code, state_ml = _fse_decode_sym(state_ml, dt_ml, br)

        # Validate code indices before table lookups.
        if ll_code >= len(LL_CODES):
            raise ValueError(f"invalid LL code {ll_code}")
        if ml_code >= len(ML_CODES):
            raise ValueError(f"invalid ML code {ml_code}")

        # ── Read extra bits for each field ────────────────────────────────────
        # Extra bits follow the FSE symbol decoding in the stream.
        ll_base, ll_extra_bits = LL_CODES[ll_code]
        ml_base, ml_extra_bits = ML_CODES[ml_code]

        ll = ll_base + br.read_bits(ll_extra_bits)
        ml = ml_base + br.read_bits(ml_extra_bits)

        # Offset extra bits: of_code bits give the fractional part.
        # Raw offset = (1 << of_code) | extra_bits
        # Match offset = raw - 3  (inverse of the +3 applied during encoding)
        of_extra = br.read_bits(of_code)
        of_raw = (1 << of_code) | of_extra
        offset = of_raw - 3

        # ── Emit literal bytes ────────────────────────────────────────────────
        lit_end = lit_pos + ll
        if lit_end > len(lits):
            raise ValueError(
                f"literal run {ll} overflows literals buffer "
                f"(pos={lit_pos} len={len(lits)})"
            )
        out.extend(lits[lit_pos:lit_end])
        lit_pos = lit_end

        # ── Copy match bytes from back-reference ──────────────────────────────
        # offset is 1-indexed: offset=1 means last byte written.
        # Security check: offset=0 is invalid; offset > out_len would read before start.
        if offset == 0 or offset > len(out):
            raise ValueError(
                f"bad match offset {offset} (output len {len(out)})"
            )
        copy_start = len(out) - offset
        for i in range(ml):
            out.append(out[copy_start + i])

    # ── Emit remaining literals after the last sequence ───────────────────────
    out.extend(lits[lit_pos:])


# =============================================================================
# Public API
# =============================================================================


def compress(data: bytes) -> bytes:
    """Compress ``data`` to ZStd format (RFC 8878).

    The output is a valid ZStd frame that can be decompressed by the ``zstd``
    CLI tool or any conforming implementation (provided it supports predefined
    FSE tables, which all do).

    Strategy:
      - Frame header: magic + FHD (single-segment, 8-byte FCS) + content size.
      - Each 128 KB block is attempted as: RLE → Compressed → Raw (fallback).

    Args:
        data: Input bytes to compress.

    Returns:
        Compressed bytes in ZStd frame format.

    Example::

        >>> compressed = compress(b"hello " * 100)
        >>> len(compressed) < 600
        True
    """
    out = bytearray()

    # ── ZStd frame header ─────────────────────────────────────────────────────
    #
    # Magic number (4 bytes LE): marks this as a ZStd frame.
    out.extend(MAGIC.to_bytes(4, "little"))

    # Frame Header Descriptor (FHD) = 0xE0:
    #   bits [7:6] = 11  → FCS_Field_Size = 8 bytes (u64 content size)
    #   bit  [5]   = 1   → Single_Segment_Flag (no Window_Descriptor follows)
    #   bit  [4]   = 0   → Content_Checksum_Flag (no checksum)
    #   bits [3:2] = 00  → reserved
    #   bits [1:0] = 00  → Dict_ID_Flag = 0 (no dictionary ID)
    # Combined: 0b1110_0000 = 0xE0
    out.append(0xE0)

    # Frame_Content_Size (8 bytes LE): uncompressed data length.
    # Allows decoders to pre-allocate output buffers.
    out.extend(len(data).to_bytes(8, "little"))

    # ── Blocks ────────────────────────────────────────────────────────────────
    #
    # Special case: empty input gets one empty raw block.
    if not data:
        # Last=1, Type=Raw(00), Size=0 → header bits: size=0, type=00, last=1
        # 3-byte LE of (0 << 3) | (0b00 << 1) | 1 = 0x000001
        hdr = 0b001  # last=1, type=00, size=0
        out.extend(hdr.to_bytes(3, "little"))
        return bytes(out)

    offset = 0
    while offset < len(data):
        end = min(offset + MAX_BLOCK_SIZE, len(data))
        block = data[offset:end]
        last = end == len(data)
        block_len = len(block)

        # Block header layout:
        #   bits [23:3] = Block_Size (up to 128 KB fits in 17 bits)
        #   bits [2:1]  = Block_Type: 00=Raw, 01=RLE, 10=Compressed, 11=Reserved
        #   bit  [0]    = Last_Block flag

        # ── Try RLE block ─────────────────────────────────────────────────────
        # If all bytes in the block are identical, a single RLE byte + 4-byte
        # overhead encodes it far more efficiently than even a compressed block.
        if block and all(b == block[0] for b in block):
            # Type=01 (RLE), Last=last, Size=block_len
            hdr = (block_len << 3) | (0b01 << 1) | (1 if last else 0)
            out.extend(hdr.to_bytes(3, "little"))
            out.append(block[0])

        else:
            # ── Try compressed block ──────────────────────────────────────────
            maybe_compressed = _compress_block(block)
            if maybe_compressed is not None:
                # Type=10 (Compressed), size = compressed block data length
                hdr = (len(maybe_compressed) << 3) | (0b10 << 1) | (1 if last else 0)
                out.extend(hdr.to_bytes(3, "little"))
                out.extend(maybe_compressed)
            else:
                # ── Raw block (fallback) ──────────────────────────────────────
                # Type=00 (Raw), size = original block length
                hdr = (block_len << 3) | (0b00 << 1) | (1 if last else 0)
                out.extend(hdr.to_bytes(3, "little"))
                out.extend(block)

        offset = end

    return bytes(out)


def decompress(data: bytes) -> bytes:
    """Decompress a ZStd frame, returning the original data.

    Accepts any valid ZStd frame with:
      - Single-segment or multi-segment layout
      - Raw, RLE, or Compressed blocks
      - Predefined FSE modes (no per-frame table description)

    Args:
        data: Compressed bytes (a valid ZStd frame).

    Returns:
        Original uncompressed bytes.

    Raises:
        ValueError: If the input is truncated, has a bad magic number, or
            contains unsupported features (non-predefined FSE tables, Huffman
            literals, reserved block types).

    Example::

        >>> original = b"hello, world!"
        >>> decompress(compress(original)) == original
        True
    """
    if len(data) < 5:
        raise ValueError(f"frame too short: {len(data)} bytes")

    # ── Validate magic ────────────────────────────────────────────────────────
    magic = int.from_bytes(data[0:4], "little")
    if magic != MAGIC:
        raise ValueError(f"bad magic: {magic:#010x} (expected {MAGIC:#010x})")

    pos = 4

    # ── Parse Frame Header Descriptor ─────────────────────────────────────────
    fhd = data[pos]
    pos += 1

    # FCS_Field_Size: bits [7:6] of FHD.
    #   00 → 0 bytes (or 1 if Single_Segment)
    #   01 → 2 bytes (actual value = stored + 256)
    #   10 → 4 bytes
    #   11 → 8 bytes
    fcs_flag = (fhd >> 6) & 3

    # Single_Segment_Flag: bit 5. When set, Window_Descriptor is omitted.
    single_seg = (fhd >> 5) & 1

    # Dict_ID_Flag: bits [1:0]. Maps to {0:0, 1:1, 2:2, 3:4} bytes of dict ID.
    dict_flag = fhd & 3

    # ── Skip Window Descriptor ────────────────────────────────────────────────
    # Present only when Single_Segment_Flag = 0.
    if single_seg == 0:
        pos += 1  # skip 1-byte Window_Descriptor

    # ── Skip Dict ID ─────────────────────────────────────────────────────────
    # dict_flag maps to 0, 1, 2, or 4 bytes of dictionary ID.
    dict_id_bytes = [0, 1, 2, 4][dict_flag]
    pos += dict_id_bytes

    # ── Skip Frame Content Size ───────────────────────────────────────────────
    # We read FCS but don't validate against actual output (the blocks are
    # authoritative). fcs_flag=0 with single_seg=1 means 1 byte.
    if fcs_flag == 0:
        fcs_bytes = 1 if single_seg == 1 else 0
    elif fcs_flag == 1:
        fcs_bytes = 2
    elif fcs_flag == 2:
        fcs_bytes = 4
    else:
        fcs_bytes = 8
    pos += fcs_bytes

    # ── Decode Blocks ─────────────────────────────────────────────────────────
    out = bytearray()

    while True:
        # Each block has a 3-byte little-endian header.
        if pos + 3 > len(data):
            raise ValueError("truncated block header")

        hdr = int.from_bytes(data[pos:pos + 3], "little")
        pos += 3

        last = bool(hdr & 1)        # bit 0 = Last_Block flag
        btype = (hdr >> 1) & 3     # bits [2:1] = Block_Type
        bsize = hdr >> 3           # bits [23:3] = Block_Size

        if btype == 0:
            # ── Raw block: bsize verbatim bytes ──────────────────────────────
            if pos + bsize > len(data):
                raise ValueError(
                    f"raw block truncated: need {bsize} bytes at pos {pos}"
                )
            if len(out) + bsize > MAX_OUTPUT:
                raise ValueError(
                    f"decompressed size exceeds limit of {MAX_OUTPUT} bytes"
                )
            out.extend(data[pos:pos + bsize])
            pos += bsize

        elif btype == 1:
            # ── RLE block: 1 byte repeated bsize times ───────────────────────
            if pos >= len(data):
                raise ValueError("RLE block missing byte")
            if len(out) + bsize > MAX_OUTPUT:
                raise ValueError(
                    f"decompressed size exceeds limit of {MAX_OUTPUT} bytes"
                )
            byte = data[pos]
            pos += 1
            out.extend(bytes([byte]) * bsize)

        elif btype == 2:
            # ── Compressed block: FSE/literals encoded ───────────────────────
            if pos + bsize > len(data):
                raise ValueError(
                    f"compressed block truncated: need {bsize} bytes at pos {pos}"
                )
            block_data = data[pos:pos + bsize]
            pos += bsize
            _decompress_block(block_data, out)

        else:
            # btype == 3: Reserved — must not appear in valid frames.
            raise ValueError("reserved block type 3")

        if last:
            break

    return bytes(out)
