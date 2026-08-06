// zstd.hpp — Zstandard (ZStd) lossless compression, in pure ISO C++17,
// header-only, in namespace ca::zstd. An educational RFC 8878 SUBSET port of
// the corrected Rust `zstd` crate (CMP07).
// ===========================================================================
//
// Zstandard (Yann Collet, Meta, 2015; RFC 8878, 2021) combines an **LZ77-style
// match-finder** (reused here from `cpp/lzss`, CMP02) with **FSE (Finite
// State Entropy) coding**, a table-based Asymmetric Numeral System (tANS,
// Jarek Duda 2013) that approaches the Shannon entropy limit more closely
// than Huffman coding without needing a fractional-bit representation.
//
//   Analogy — Huffman vs FSE:
//     Huffman:  every symbol gets a whole number of bits; a 90%-probable
//               symbol still costs at least 1 bit.
//     FSE:      symbols "share" byte boundaries via a finite state machine
//               over a probability table; that same 90%-probable symbol can
//               cost ~0.15 bits on average.
//
// # Educational subset (this port; see code/specs/CMP07-zstd.md)
//
// | Feature         | Full ZStd              | This implementation |
// |-----------------|-------------------------|----------------------|
// | Literals        | Huffman or raw          | Raw only             |
// | Sequences FSE   | Custom + predefined     | Predefined only      |
// | Block types     | Raw / RLE / Compressed  | All three            |
// | Dictionary      | Yes                     | No                   |
// | Checksums       | Optional                | Omitted (flag=0)     |
// | Window size     | Up to 8 MB              | Fixed (Single_Segment)|
//
// Even with these simplifications the wire format is REAL: this decoder
// accepts (and this encoder's output is accepted by) the reference `zstd`
// CLI. That interoperability is exactly what makes the FSE codec below
// tricky to get right — see "THE FSE BUG CLASS" below.
//
// # Frame layout (RFC 8878 §3)
//
//   ┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
//   │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
//   │ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
//   └────────┴─────┴──────────────────────┴────────┴──────────────────┘
//
// Each **block** has a 3-byte header:
//   bit 0       = Last_Block flag
//   bits [2:1]  = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
//   bits [23:3] = Block_Size
//
// # THE FSE BUG CLASS (read this before touching the sequences codec)
//
// An earlier repo-wide audit (2026-08, see lessons.md Lesson 96/97) found
// that EVERY existing language port of this package — including the Rust
// crate this port is translated from — independently invented the SAME three
// wrong conventions for the sequences-section FSE codec. All three are
// "internally self-consistent": an encoder and decoder that agree with each
// other on the wrong convention pass every round-trip test against
// themselves, and the bug is only visible when checked against an
// INDEPENDENT, spec-conformant implementation (the real `zstd` CLI — see
// TC-9 below). The three bugs, all avoided here:
//
//   1. FSE table-spread must be a SINGLE pass over symbols `0..maxSymbol`,
//      placing each symbol's full count immediately (`FSE_buildDTable_internal`'s
//      low-probability branch). NOT a fabricated two-pass split of
//      "count>1 symbols first, then count==1 symbols".
//   2. Per-sequence decode order: PEEK all three symbols (LL, ML, OF) from
//      the CURRENT states first (a bare table lookup — zero bits consumed),
//      THEN read extra bits in order OF, ML, LL, THEN update states in order
//      LL, ML, OF. The initial states (read once, before sequence 1) are
//      read in a DIFFERENT order: LL, OF, ML. RFC 8878 is genuinely
//      asymmetric here, not a typo.
//   3. The state-transition UPDATE is skipped entirely for the LAST sequence
//      in a block (no "next" sequence needs a prepared state). The
//      encoder's mirror-image first-processed symbol (semantically the last
//      real sequence, since the bitstream is built backwards) gets its
//      starting state from a direct formula (`FSE_initCState2`), writing
//      ZERO bits — not from a normal bit-flushing transition.
//
// See `detail::build_decode_table`, `detail::decompress_block`, and
// `detail::encode_sequences_section` below for where each rule is enforced,
// and `tests/zstd_test.cpp`'s `tc9_cli_interop` for the interop test that
// actually catches a violation of any of the three.
//
// # Public API
//
//   ca::zstd::compress(data)     -> std::vector<std::uint8_t>   (never throws)
//   ca::zstd::decompress(data)   -> std::vector<std::uint8_t>   (throws ZstdError)
//
// Matching `cpp/lzss`'s established convention: bytes are
// `std::vector<std::uint8_t>` (this repo pins -std=c++17, so `std::span` —
// a C++20 addition — is not available), and malformed/untrusted input on the
// decode path is reported via a thrown exception rather than a silent
// best-effort result (`ZstdError`, matching `cpp/wasm-leb128`'s
// `Error : public std::runtime_error` convention) — appropriate here because,
// unlike `lzss::decode`'s deliberately lenient token-level API, a ZStd frame
// carries untrusted, security-relevant framing (declared sizes, offsets,
// sequence counts) that must be validated, not silently clamped.
//
// # Series
//
//   CMP00 (LZ77)     — Sliding-window back-references
//   CMP01 (LZ78)     — Explicit dictionary (trie)
//   CMP02 (LZSS)     — LZ77 + flag bits              ← reused for match-finding
//   CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
//   CMP04 (Huffman)  — Entropy coding
//   CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
//   CMP06 (Brotli)   — DEFLATE + context modelling + static dict
//   CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed ← this package
#ifndef CA_ZSTD_HPP
#define CA_ZSTD_HPP

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

#include "lzss.hpp"

namespace ca {
namespace zstd {

using Bytes = std::vector<std::uint8_t>;

// Thrown by `decompress()` on any malformed, truncated, or unsupported-
// feature input. `compress()` never throws — every input has a valid
// encoding (worst case: a Raw block copy).
class ZstdError : public std::runtime_error {
public:
    explicit ZstdError(const std::string& message) : std::runtime_error(message) {}
};

namespace detail {

// ─── Constants ─────────────────────────────────────────────────────────────

// ZStd magic number: 0xFD2FB528, little-endian on the wire (`28 B5 2F FD`).
constexpr std::uint32_t MAGIC = 0xFD2FB528u;

// Maximum block size: 128 KB. Real zstd bounds the REGENERATED size of every
// block — Raw, RLE, and Compressed alike — to `min(Window_Size, 128 KB)`;
// this implementation always uses Window_Size = 8 MB, so 128 KB is the
// binding constraint. Enforced on both the encode side (block splitting)
// and the decode side (`Block_Size` validation, code/specs/CMP07-zstd.md's
// "Security Considerations" > Block size cap).
constexpr std::size_t MAX_BLOCK_SIZE = 128 * 1024;

// Decompression-bomb guard: maximum total decompressed output size (256 MB).
//
// A Compressed block's WIRE size is capped at MAX_BLOCK_SIZE (128 KB), but
// that says nothing about how large the block can EXPAND to: a single
// FSE-coded sequence's match length can be up to ~131 KB (ML code 52), and
// one 128 KB block can carry tens of thousands of sequences. This budget is
// therefore checked incrementally, at EVERY point output can grow —
// including inside the per-sequence loop of `decompress_block`, not only
// once per top-level Raw/RLE block.
constexpr std::size_t MAX_OUTPUT = 256 * 1024 * 1024;

// Sequences_Count is a 24-bit wire field (RFC 8878 §3.1.1.3.1); reject
// anything claiming more than that before doing any per-sequence work, as a
// defense-in-depth check independent of the fact that this port's own
// `encode_seq_count` can never itself produce a value this large.
constexpr std::size_t MAX_SEQ_COUNT = 8'388'607;

// Throws if adding `additional` more bytes to output (currently
// `current_size` bytes) would exceed MAX_OUTPUT. `additional` is always
// derived from bounded wire fields (LL/ML values, at most ~131070 per RFC
// 8878's code tables), so this never has to worry about `additional` itself
// overflowing before the comparison.
inline void check_output_budget(std::size_t current_size, std::size_t additional) {
    if (additional > MAX_OUTPUT || current_size > MAX_OUTPUT - additional) {
        throw ZstdError("decompressed size exceeds limit of " +
                         std::to_string(MAX_OUTPUT) + " bytes");
    }
}

// ─── LL / ML / OF code tables (RFC 8878 §3.1.1.3) ──────────────────────────
//
// These map a *code number* to a (baseline, extra_bits) pair. For example,
// LL code 17 means literal_length = 18 + read(1 extra bit) — it covers
// literal lengths 18 and 19. The FSE state machine tracks one code number
// per field; extra bits are read directly from the bitstream after state
// transitions (or, for the very first sequence, after peeking the initial
// states).

struct CodeInfo {
    std::uint32_t baseline;
    std::uint8_t extra_bits;
};

// Literal Length code table: codes 0..=35. Literal lengths 0..15 each have
// their own code (0 extra bits); larger lengths are grouped into ranges.
constexpr std::array<CodeInfo, 36> LL_CODES = {{
    {0, 0},  {1, 0},  {2, 0},  {3, 0},  {4, 0},  {5, 0},
    {6, 0},  {7, 0},  {8, 0},  {9, 0},  {10, 0}, {11, 0},
    {12, 0}, {13, 0}, {14, 0}, {15, 0},
    {16, 1}, {18, 1}, {20, 1}, {22, 1},
    {24, 2}, {28, 2},
    {32, 3}, {40, 3},
    {48, 4}, {64, 6},
    {128, 7}, {256, 8}, {512, 9}, {1024, 10}, {2048, 11}, {4096, 12},
    {8192, 13}, {16384, 14}, {32768, 15}, {65536, 16},
}};

// Match Length code table: codes 0..=52. ZStd's minimum match length is 3
// (not 0) — code 0 means match length 3.
constexpr std::array<CodeInfo, 53> ML_CODES = {{
    {3, 0},  {4, 0},  {5, 0},  {6, 0},  {7, 0},  {8, 0},
    {9, 0},  {10, 0}, {11, 0}, {12, 0}, {13, 0}, {14, 0},
    {15, 0}, {16, 0}, {17, 0}, {18, 0}, {19, 0}, {20, 0},
    {21, 0}, {22, 0}, {23, 0}, {24, 0}, {25, 0}, {26, 0},
    {27, 0}, {28, 0}, {29, 0}, {30, 0}, {31, 0}, {32, 0},
    {33, 0}, {34, 0},
    {35, 1}, {37, 1},  {39, 1},  {41, 1},
    {43, 2}, {47, 2},
    {51, 3}, {59, 3},
    {67, 4}, {83, 4},
    {99, 5}, {131, 7},
    {259, 8}, {515, 9}, {1027, 10}, {2051, 11},
    {4099, 12}, {8195, 13}, {16387, 14}, {32771, 15}, {65539, 16},
}};

// ─── FSE predefined distributions (RFC 8878 Appendix B) ────────────────────
//
// "Predefined_Mode" means no per-frame table description is transmitted; the
// decoder builds the same table from these fixed distributions. This
// educational subset supports ONLY Predefined_Mode (no custom FSE tables,
// no RLE-mode, no repeat-mode for the sequences section).
//
// Entries of -1 mean "probability ~1/table_size" (a "low probability"
// symbol) — these symbols get exactly one slot in the decode table and
// their encoder state transition always uses the full accuracy-log bits.

constexpr std::array<std::int16_t, 36> LL_NORM = {{
     4,  3,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  1,  1,  1,
     2,  2,  2,  2,  2,  2,  2,  2,  2,  3,  2,  1,  1,  1,  1,  1,
    -1, -1, -1, -1,
}};
constexpr std::uint8_t LL_ACC_LOG = 6;  // table_size = 64

constexpr std::array<std::int16_t, 53> ML_NORM = {{
     1,  4,  3,  2,  2,  2,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1, -1, -1,
    -1, -1, -1, -1, -1,
}};
constexpr std::uint8_t ML_ACC_LOG = 6;  // table_size = 64

constexpr std::array<std::int16_t, 29> OF_NORM = {{
     1,  1,  1,  1,  1,  1,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1, -1, -1, -1, -1, -1,
}};
constexpr std::uint8_t OF_ACC_LOG = 5;  // table_size = 32

// ─── FSE decode table entry ────────────────────────────────────────────────
//
// To decode a symbol from state S: `sym` is the output symbol; read `nb`
// bits from the bitstream as `bits`; new state = `base + bits`.
struct FseDe {
    std::uint8_t sym = 0;
    std::uint8_t nb = 0;
    std::uint16_t base = 0;
};

// Build an FSE decode table from a normalised probability distribution.
//
// Algorithm (verified against the real reference C source,
// `FSE_buildDTable_internal` in `github.com/facebook/zstd`'s
// `fse_decompress.c` — see the module banner's "THE FSE BUG CLASS"):
//  1. Symbols with probability -1 go at the TOP of the table (high indices);
//     each gets exactly one slot.
//  2. Remaining symbols are spread with a SINGLE pass over symbols in
//     ascending order `0..norm.size()`, placing each symbol's full count
//     immediately when encountered, advancing by a fixed step and skipping
//     any slot already claimed by a probability-(-1) symbol. `step =
//     (sz>>1) + (sz>>3) + 3` is co-prime to `sz` (a power of two), so this
//     walk visits every free slot exactly once.
//  3. Assign `nb` (extra bits to read) and `base` (next-state offset) to
//     each slot so `state = base + read(nb bits)` lands in `[sz, 2*sz)`,
//     the valid encoder state range.
inline std::vector<FseDe> build_decode_table(const std::int16_t* norm, std::size_t norm_len,
                                              std::uint8_t acc_log) {
    const std::size_t sz = std::size_t(1) << acc_log;
    const std::size_t step = (sz >> 1) + (sz >> 3) + 3;
    std::vector<FseDe> tbl(sz);
    std::vector<std::uint16_t> sym_next(norm_len, 0);

    // Phase 1: probability -1 symbols at the top.
    std::size_t high = sz - 1;
    bool high_valid = true;
    for (std::size_t s = 0; s < norm_len; ++s) {
        if (norm[s] == -1) {
            tbl[high].sym = static_cast<std::uint8_t>(s);
            sym_next[s] = 1;
            if (high == 0) {
                high_valid = false;
            } else {
                --high;
            }
        }
    }
    (void)high_valid;  // `high` only underflows if norm_len exceeds sz, which
                        // never happens for our three fixed, compile-time
                        // distributions — guarded defensively, not reachable.

    // Phase 2: SINGLE pass, ascending symbol order (see banner comment — no
    // fabricated "count>1 then count==1" two-pass split).
    std::size_t pos = 0;
    for (std::size_t s = 0; s < norm_len; ++s) {
        if (norm[s] <= 0) {
            continue;
        }
        const std::size_t cnt = static_cast<std::size_t>(norm[s]);
        sym_next[s] = static_cast<std::uint16_t>(cnt);
        for (std::size_t i = 0; i < cnt; ++i) {
            tbl[pos].sym = static_cast<std::uint8_t>(s);
            pos = (pos + step) & (sz - 1);
            while (pos > high) {
                pos = (pos + step) & (sz - 1);
            }
        }
    }

    // Phase 3: assign nb / base per slot, in ascending table-index order.
    std::vector<std::uint16_t> sn = sym_next;
    for (std::size_t i = 0; i < sz; ++i) {
        const std::size_t s = tbl[i].sym;
        const std::uint32_t ns = sn[s];
        sn[s] = static_cast<std::uint16_t>(sn[s] + 1);
        // floor(log2(ns)): position of the highest set bit.
        std::uint8_t log2ns = 0;
        for (std::uint32_t v = ns; v > 1; v >>= 1) {
            ++log2ns;
        }
        const std::uint8_t nb = static_cast<std::uint8_t>(acc_log - log2ns);
        const std::uint32_t base = (ns << nb) - static_cast<std::uint32_t>(sz);
        tbl[i].nb = nb;
        tbl[i].base = static_cast<std::uint16_t>(base);
    }

    return tbl;
}

// ─── FSE encode symbol table entry ─────────────────────────────────────────
//
// Given encoder state S for symbol `s`: `nb_out = (S + delta_nb) >> 16`
// (bits to emit); emit the low `nb_out` bits of S; `new_S =
// state_tbl[(S >> nb_out) + delta_fs]`. Precomputing `delta_nb`/`delta_fs`
// from the distribution keeps the hot encode loop to arithmetic + lookup.
struct FseEe {
    std::uint32_t delta_nb = 0;  // (max_bits_out << 16) - (count << max_bits_out)
    std::int32_t delta_fs = 0;   // cumulative_count_before_sym - count
};

// Build FSE encode tables from a normalised distribution: `ee[sym]` (the
// FseEe transform per symbol) and `st[slot]` (encoder state table, slot ->
// output state in `[sz, 2*sz)`).
//
// # The encode/decode symmetry
//
// The decoder assigns `(sym, nb, base)` to each table cell in ASCENDING
// INDEX order (`build_decode_table`'s Phase 3): for symbol `s`, the j-th
// cell (ascending index order) has `ns = count[s] + j`. The encoder must use
// the SAME indexing: slot `cumul[s] + j` maps to the j-th table cell for
// symbol `s` (ascending index order), and the encoder's output state after
// encoding `s` from that slot is `(that cell's index) + sz` — the state a
// decoder in that state would decode `s` from, using the same bits.
inline std::pair<std::vector<FseEe>, std::vector<std::uint16_t>> build_encode_sym(
    const std::int16_t* norm, std::size_t norm_len, std::uint8_t acc_log) {
    const std::uint32_t sz = std::uint32_t(1) << acc_log;

    // Step 1: cumulative sums (start offset per symbol among its own count).
    std::vector<std::uint32_t> cumul(norm_len, 0);
    std::uint32_t total = 0;
    for (std::size_t s = 0; s < norm_len; ++s) {
        cumul[s] = total;
        const std::uint32_t cnt = (norm[s] == -1) ? 1u : static_cast<std::uint32_t>(std::max<std::int16_t>(norm[s], 0));
        total += cnt;
    }

    // Step 2: spread table (index -> symbol), mirroring build_decode_table's
    // Phase 1+2 EXACTLY (must use the identical single-pass algorithm — see
    // the module banner).
    const std::size_t szc = static_cast<std::size_t>(sz);
    const std::size_t step = (szc >> 1) + (szc >> 3) + 3;
    std::vector<std::uint8_t> spread(szc, 0);
    std::size_t idx_high = szc - 1;
    for (std::size_t s = 0; s < norm_len; ++s) {
        if (norm[s] == -1) {
            spread[idx_high] = static_cast<std::uint8_t>(s);
            if (idx_high == 0) {
                break;
            }
            --idx_high;
        }
    }
    const std::size_t idx_limit = idx_high;

    std::size_t pos = 0;
    for (std::size_t s = 0; s < norm_len; ++s) {
        if (norm[s] <= 0) {
            continue;
        }
        const std::size_t cnt = static_cast<std::size_t>(norm[s]);
        for (std::size_t i = 0; i < cnt; ++i) {
            spread[pos] = static_cast<std::uint8_t>(s);
            pos = (pos + step) & (szc - 1);
            while (pos > idx_limit) {
                pos = (pos + step) & (szc - 1);
            }
        }
    }

    // Step 3: state table by iterating `spread` in ascending index order,
    // tracking how many occurrences of each symbol we've seen so far.
    std::vector<std::uint32_t> sym_occ(norm_len, 0);
    std::vector<std::uint16_t> st(szc, 0);
    for (std::size_t i = 0; i < szc; ++i) {
        const std::size_t s = spread[i];
        const std::uint32_t j = sym_occ[s]++;
        const std::size_t slot = static_cast<std::size_t>(cumul[s]) + j;
        st[slot] = static_cast<std::uint16_t>(i + szc);
    }

    // Step 4: FseEe entries (delta_nb / delta_fs) per symbol.
    std::vector<FseEe> ee(norm_len);
    for (std::size_t s = 0; s < norm_len; ++s) {
        const std::uint32_t cnt = (norm[s] == -1) ? 1u : static_cast<std::uint32_t>(std::max<std::int16_t>(norm[s], 0));
        if (cnt == 0) {
            continue;
        }
        std::uint32_t mbo;
        if (cnt == 1) {
            mbo = acc_log;
        } else {
            std::uint8_t log2cnt = 0;
            for (std::uint32_t v = cnt; v > 1; v >>= 1) {
                ++log2cnt;
            }
            mbo = static_cast<std::uint32_t>(acc_log) - log2cnt;
        }
        ee[s].delta_nb = (mbo << 16) - (cnt << mbo);
        ee[s].delta_fs = static_cast<std::int32_t>(cumul[s]) - static_cast<std::int32_t>(cnt);
    }

    return {ee, st};
}

// ─── Reverse bit-writer / reader ────────────────────────────────────────────
//
// ZStd's sequence bitstream is written BACKWARDS relative to data flow: the
// encoder writes bits the decoder will read LAST, first — this lets a
// forward-only decoder read a bitstream the encoder built without knowing
// decode order in advance.
//
// Byte layout: `[byte0, ..., byteN]` where `byteN` (the last byte written)
// contains a SENTINEL bit (its highest set bit) marking the end of
// meaningful data; the decoder finds this sentinel to initialise. Within
// each byte, bit 0 = earliest written (LSB-first accumulation).
//
// Example: write bits `1, 0, 1, 1` (4 bits), then flush:
//   reg = 0b1011, bits = 4 -> sentinel at bit 4 -> last byte = 0b0001_1011.
//   The decoder finds the sentinel (bit 4), then reads bits 3..0 = 0b1011.
class RevBitWriter {
public:
    // Adds the low `nb` bits of `val` to the stream.
    void add_bits(std::uint64_t val, std::uint8_t nb) {
        if (nb == 0) {
            return;
        }
        const std::uint64_t mask = (nb == 64) ? ~std::uint64_t(0) : ((std::uint64_t(1) << nb) - 1);
        reg_ |= (val & mask) << bits_;
        bits_ = static_cast<std::uint8_t>(bits_ + nb);
        while (bits_ >= 8) {
            buf_.push_back(static_cast<std::uint8_t>(reg_ & 0xFF));
            reg_ >>= 8;
            bits_ = static_cast<std::uint8_t>(bits_ - 8);
        }
    }

    // Flushes remaining bits with a sentinel bit and appends the result.
    void flush() {
        const std::uint8_t sentinel = static_cast<std::uint8_t>(1u << bits_);
        const std::uint8_t last_byte = static_cast<std::uint8_t>((reg_ & 0xFF) | sentinel);
        buf_.push_back(last_byte);
        reg_ = 0;
        bits_ = 0;
    }

    Bytes finish() && { return std::move(buf_); }

private:
    Bytes buf_;
    std::uint64_t reg_ = 0;  // accumulation register (bits fill from LSB)
    std::uint8_t bits_ = 0;  // number of valid bits currently in reg_
};

// Mirrors RevBitWriter: reads bits from the END of the buffer backwards. The
// register holds valid bits LEFT-ALIGNED (packed toward the MSB side), so
// `read_bits(n)` can always return the most-recently-written n bits by
// taking the top n bits and shifting left.
class RevBitReader {
public:
    explicit RevBitReader(const std::uint8_t* data, std::size_t len) : data_(data), len_(len) {
        if (len_ == 0) {
            throw ZstdError("empty bitstream");
        }
        const std::uint8_t last = data_[len_ - 1];
        if (last == 0) {
            throw ZstdError("bitstream last byte is zero (no sentinel)");
        }
        // sentinel_pos = index (0=LSB) of the highest set bit in `last`.
        std::uint8_t sentinel_pos = 0;
        for (std::uint8_t v = last; v > 1; v >>= 1) {
            ++sentinel_pos;
        }
        const std::uint8_t valid_bits = sentinel_pos;  // data bits below sentinel
        const std::uint64_t mask = (valid_bits == 0) ? 0 : ((std::uint64_t(1) << valid_bits) - 1);
        reg_ = (valid_bits == 0) ? 0 : ((static_cast<std::uint64_t>(last) & mask) << (64 - valid_bits));
        bits_ = valid_bits;
        pos_ = len_ - 1;  // sentinel byte consumed; reload() continues below it
        reload();
    }

    // Reads `nb` bits from the top of the register (returns 0 if nb == 0).
    std::uint64_t read_bits(std::uint8_t nb) {
        if (nb == 0) {
            return 0;
        }
        const std::uint64_t val = reg_ >> (64 - nb);
        reg_ = (nb == 64) ? 0 : (reg_ << nb);
        bits_ = static_cast<std::uint8_t>(bits_ > nb ? bits_ - nb : 0);
        if (bits_ < 24) {
            reload();
        }
        return val;
    }

private:
    // Loads more bytes into the register from the stream, going backward.
    void reload() {
        while (bits_ <= 56 && pos_ > 0) {
            --pos_;
            const std::uint8_t shift = static_cast<std::uint8_t>(64 - bits_ - 8);
            reg_ |= static_cast<std::uint64_t>(data_[pos_]) << shift;
            bits_ = static_cast<std::uint8_t>(bits_ + 8);
        }
    }

    const std::uint8_t* data_;
    std::size_t len_;
    std::uint64_t reg_ = 0;
    std::uint8_t bits_ = 0;
    std::size_t pos_ = 0;
};

// ─── FSE encode/decode helpers ──────────────────────────────────────────────

// Encodes one symbol into the backward bitstream and updates encoder state.
// `state` is in `[sz, 2*sz)`: flush `nb = (state + delta_nb) >> 16` low bits
// of `state`, then `state = st[(state >> nb) + delta_fs]`.
inline void fse_encode_sym(std::uint32_t& state, std::uint8_t sym, const std::vector<FseEe>& ee,
                            const std::vector<std::uint16_t>& st, RevBitWriter& bw) {
    const FseEe& e = ee[sym];
    const std::uint8_t nb = static_cast<std::uint8_t>((state + e.delta_nb) >> 16);
    bw.add_bits(state, nb);
    const std::int64_t slot_i = static_cast<std::int64_t>(state >> nb) + e.delta_fs;
    const std::size_t slot = static_cast<std::size_t>(slot_i < 0 ? 0 : slot_i);
    state = st[slot];
}

// Initialises an FSE encoder state directly from a symbol, WITHOUT flushing
// any bits — the reverse-encode-loop analogue of real zstd's
// `FSE_initCState2`. RFC 8878's decoder never performs a state-update read
// after the LAST sequence (there is no "next" sequence's peek to prepare a
// state for); symmetrically, the ENCODER's first-processed symbol (which
// corresponds to that same last sequence, since the loop runs backwards)
// cannot derive its state via a normal `fse_encode_sym` flush — there is no
// bit-consuming decode-side read to produce it. See the module banner.
inline std::uint32_t fse_init_state(std::uint8_t sym, const std::vector<FseEe>& ee,
                                     const std::vector<std::uint16_t>& st) {
    const FseEe& e = ee[sym];
    const std::uint64_t delta_nb = e.delta_nb;
    const std::uint64_t nb_bits_out = (delta_nb + (std::uint64_t(1) << 15)) >> 16;
    const std::uint64_t value = (nb_bits_out << 16) - delta_nb;
    const std::int64_t slot_i = static_cast<std::int64_t>(value >> nb_bits_out) + e.delta_fs;
    const std::size_t slot = static_cast<std::size_t>(slot_i < 0 ? 0 : slot_i);
    return st[slot];
}

// Peeks the symbol encoded at the current FSE decode state WITHOUT consuming
// any bits — the state itself IS the decode-table index, so this is a bare
// table lookup. RFC 8878 requires all three symbols (LL, ML, OF) to be
// peeked from their CURRENT states before any extra bits or state updates
// are read. See `decompress_block`.
inline FseDe fse_peek(std::uint16_t state, const std::vector<FseDe>& de) {
    return de[state];
}

// Consumes `entry.nb` bits and computes the next FSE decode state from a
// previously peeked table entry: `new_state = entry.base + read(entry.nb)`.
//
// Per the reference decoder, this update is SKIPPED for the LAST sequence in
// a block — see `decompress_block`'s `if (i + 1 != n_seqs)` guard. Calling
// this unconditionally would consume bits the encoder never wrote,
// corrupting every read that follows. See the module banner.
inline std::uint16_t fse_update_state(const FseDe& entry, RevBitReader& br) {
    return static_cast<std::uint16_t>(entry.base + br.read_bits(entry.nb));
}

// ─── LL/ML code-number lookup ───────────────────────────────────────────────

// Maps a literal-length value to its LL code (0..35). Codes are stored in
// increasing baseline order, so the last code whose baseline <= ll wins.
inline std::size_t ll_to_code(std::uint32_t ll) {
    std::size_t code = 0;
    for (std::size_t i = 0; i < LL_CODES.size(); ++i) {
        if (LL_CODES[i].baseline <= ll) {
            code = i;
        } else {
            break;
        }
    }
    return code;
}

// Maps a match-length value to its ML code (0..52).
inline std::size_t ml_to_code(std::uint32_t ml) {
    std::size_t code = 0;
    for (std::size_t i = 0; i < ML_CODES.size(); ++i) {
        if (ML_CODES[i].baseline <= ml) {
            code = i;
        } else {
            break;
        }
    }
    return code;
}

// ─── Sequence struct ─────────────────────────────────────────────────────────
//
// One ZStd sequence: emit `ll` literal bytes from the literals section, then
// copy `ml` bytes starting `off` positions back in the output buffer. After
// all sequences, any trailing literals (with no following match) are
// appended verbatim.
struct Seq {
    std::uint32_t ll;
    std::uint32_t ml;
    std::uint32_t off;
};

// Converts LZSS tokens (from `ca::lzss::encode`) into a flat literals buffer
// plus a list of ZStd sequences: consecutive literals before each match
// become one sequence's `ll`; any literals after the last match have no
// corresponding sequence and stay in the trailing part of `lits`.
inline std::pair<Bytes, std::vector<Seq>> tokens_to_seqs(const std::vector<ca::lzss::Token>& tokens) {
    Bytes lits;
    std::vector<Seq> seqs;
    std::uint32_t lit_run = 0;
    for (const auto& tok : tokens) {
        if (!tok.is_match) {
            lits.push_back(tok.literal);
            lit_run += 1;
        } else {
            seqs.push_back(Seq{lit_run, static_cast<std::uint32_t>(tok.length),
                                static_cast<std::uint32_t>(tok.offset)});
            lit_run = 0;
        }
    }
    return {lits, seqs};
}

// ─── Literals section (Raw_Literals only — RFC 8878 §3.1.1.2.1) ────────────
//
// Header format depends on literal count (bottom 2 bits of byte 0 are always
// the Literals_Block_Type = 00 for Raw):
//   <= 31 bytes:   1-byte header = (n << 3) | 0b000
//   <= 4095 bytes: 2-byte header = (n << 4) | 0b0100
//   else:          3-byte header = (n << 4) | 0b1100

inline Bytes encode_literals_section(const Bytes& lits) {
    const std::size_t n = lits.size();
    Bytes out;
    out.reserve(n + 3);

    if (n <= 31) {
        out.push_back(static_cast<std::uint8_t>((static_cast<std::uint32_t>(n) << 3)));
    } else if (n <= 4095) {
        const std::uint32_t hdr = (static_cast<std::uint32_t>(n) << 4) | 0b0100u;
        out.push_back(static_cast<std::uint8_t>(hdr & 0xFF));
        out.push_back(static_cast<std::uint8_t>((hdr >> 8) & 0xFF));
    } else {
        const std::uint32_t hdr = (static_cast<std::uint32_t>(n) << 4) | 0b1100u;
        out.push_back(static_cast<std::uint8_t>(hdr & 0xFF));
        out.push_back(static_cast<std::uint8_t>((hdr >> 8) & 0xFF));
        out.push_back(static_cast<std::uint8_t>((hdr >> 16) & 0xFF));
    }

    out.insert(out.end(), lits.begin(), lits.end());
    return out;
}

// Decodes a literals section, returning (literals, bytes_consumed). Throws
// on truncation or an unsupported literals type (this port only emits/reads
// Raw_Literals — Huffman-coded literals (types 2, 3) are rejected).
inline std::pair<Bytes, std::size_t> decode_literals_section(const std::uint8_t* data, std::size_t len) {
    if (len == 0) {
        throw ZstdError("empty literals section");
    }

    const std::uint8_t b0 = data[0];
    const std::uint8_t ltype = b0 & 0b11;
    if (ltype != 0) {
        throw ZstdError("unsupported literals type " + std::to_string(ltype) +
                         " (only Raw=0 supported)");
    }

    const std::uint8_t size_format = (b0 >> 2) & 0b11;
    std::size_t n = 0;
    std::size_t header_bytes = 0;
    switch (size_format) {
        case 0:
        case 2:
            n = static_cast<std::size_t>(b0 >> 3);
            header_bytes = 1;
            break;
        case 1:
            if (len < 2) {
                throw ZstdError("truncated literals header (2-byte)");
            }
            n = (static_cast<std::size_t>(b0 >> 4)) | (static_cast<std::size_t>(data[1]) << 4);
            header_bytes = 2;
            break;
        case 3:
            if (len < 3) {
                throw ZstdError("truncated literals header (3-byte)");
            }
            n = (static_cast<std::size_t>(b0 >> 4)) | (static_cast<std::size_t>(data[1]) << 4) |
                (static_cast<std::size_t>(data[2]) << 12);
            header_bytes = 3;
            break;
        default:
            break;  // unreachable: size_format is 2 bits, all cases covered
    }

    const std::size_t start = header_bytes;
    const std::size_t end = start + n;
    if (end > len) {
        throw ZstdError("literals data truncated: need " + std::to_string(end) + ", have " +
                         std::to_string(len));
    }

    return {Bytes(data + start, data + end), end};
}

// ─── Sequences section ──────────────────────────────────────────────────────
//
// Layout: [sequence_count: 1-3 bytes] [symbol_compression_modes: 1 byte]
// [FSE bitstream]. The modes byte is always 0x00 here (all three fields
// Predefined) — see the module banner for why.

// Encodes Number_of_Sequences (RFC 8878 §3.1.1.3.1). Verified against the
// real `zstd` CLI and the reference C source
// (`lib/compress/zstd_compress_sequences.c`):
//   0             : 1 byte  = 0x00
//   1..127        : 1 byte  = count
//   128..32511    : 2 bytes; byte0 = (count>>8)|0x80, byte1 = count & 0xFF
//   32512..       : 3 bytes; byte0 = 0xFF, then (count-0x7F00) as LE u16
//
// The marker/high byte MUST come first on the wire regardless of host
// endianness — a plain little-endian `count | 0x8000` pair round-trips
// against itself but is not the real wire format (any 128+-sequence block
// would misparse under a real RFC 8878 decoder). See lessons.md.
inline Bytes encode_seq_count(std::size_t count) {
    Bytes out;
    if (count < 128) {
        out.push_back(static_cast<std::uint8_t>(count));
    } else if (count < 0x7F00) {
        const std::uint8_t hi = static_cast<std::uint8_t>((count >> 8) | 0x80);
        const std::uint8_t lo = static_cast<std::uint8_t>(count & 0xFF);
        out.push_back(hi);
        out.push_back(lo);
    } else {
        const std::size_t r = count - 0x7F00;
        out.push_back(0xFF);
        out.push_back(static_cast<std::uint8_t>(r & 0xFF));
        out.push_back(static_cast<std::uint8_t>((r >> 8) & 0xFF));
    }
    return out;
}

// Decodes Number_of_Sequences, mirroring `encode_seq_count`. Returns
// (count, bytes_consumed).
inline std::pair<std::size_t, std::size_t> decode_seq_count(const std::uint8_t* data, std::size_t len) {
    if (len == 0) {
        throw ZstdError("empty sequence count");
    }
    const std::uint8_t b0 = data[0];
    if (b0 < 128) {
        return {static_cast<std::size_t>(b0), 1};
    }
    if (b0 < 0xFF) {
        if (len < 2) {
            throw ZstdError("truncated sequence count");
        }
        const std::size_t count = (static_cast<std::size_t>(b0 & 0x7F) << 8) | data[1];
        return {count, 2};
    }
    if (len < 3) {
        throw ZstdError("truncated sequence count (3-byte)");
    }
    const std::size_t count = 0x7F00 + data[1] + (static_cast<std::size_t>(data[2]) << 8);
    return {count, 3};
}

// Encodes the sequences section using predefined FSE tables. `seqs` must be
// non-empty (`compress_block` never calls this otherwise).
//
// See the module banner ("THE FSE BUG CLASS") for the exact field order this
// mirrors. Summary of what a FORWARD-READING decoder consumes (this encoder
// writes the exact reverse, since the bitstream is backward, and processes
// sequences in reverse order — last real sequence first):
//   1. Initial states, read order LL, OF, ML (asymmetric vs. (3) below).
//   2. Per sequence: peek all 3 symbols (free) -> read extras, order OF, ML,
//      LL -> update states, order LL, ML, OF (skipped for the last sequence
//      — no "next" sequence needs a prepared state).
inline Bytes encode_sequences_section(const std::vector<Seq>& seqs) {
    auto [ee_ll, st_ll] = build_encode_sym(LL_NORM.data(), LL_NORM.size(), LL_ACC_LOG);
    auto [ee_ml, st_ml] = build_encode_sym(ML_NORM.data(), ML_NORM.size(), ML_ACC_LOG);
    auto [ee_of, st_of] = build_encode_sym(OF_NORM.data(), OF_NORM.size(), OF_ACC_LOG);

    const std::uint32_t sz_ll = std::uint32_t(1) << LL_ACC_LOG;
    const std::uint32_t sz_ml = std::uint32_t(1) << ML_ACC_LOG;
    const std::uint32_t sz_of = std::uint32_t(1) << OF_ACC_LOG;

    // Always overwritten by fse_init_state on the first loop iteration
    // (guaranteed by the non-empty-`seqs` precondition) before ever read.
    std::uint32_t state_ll = sz_ll;
    std::uint32_t state_ml = sz_ml;
    std::uint32_t state_of = sz_of;

    RevBitWriter bw;
    bool first = true;

    for (auto it = seqs.rbegin(); it != seqs.rend(); ++it) {
        const Seq& seq = *it;
        const std::size_t ll_code = ll_to_code(seq.ll);
        const std::size_t ml_code = ml_to_code(seq.ml);

        // Offset encoding: raw = offset + 3; code = floor(log2(raw)); extra
        // = raw - (1 << code) (RFC 8878 §3.1.1.3.2.1).
        const std::uint32_t raw_off = seq.off + 3;
        std::uint8_t of_code = 0;
        if (raw_off > 1) {
            for (std::uint32_t v = raw_off; v > 1; v >>= 1) {
                ++of_code;
            }
        }
        const std::uint32_t of_extra = raw_off - (std::uint32_t(1) << of_code);
        const std::uint32_t ml_extra = seq.ml - ML_CODES[ml_code].baseline;
        const std::uint32_t ll_extra = seq.ll - LL_CODES[ll_code].baseline;

        if (!first) {
            // Transition FROM the state used to peek the PREVIOUSLY
            // processed sequence TO the state used to peek THIS one — write
            // order OF, ML, LL (a forward decoder consumes this as update
            // order LL, ML, OF right after decoding the previous sequence).
            fse_encode_sym(state_of, of_code, ee_of, st_of, bw);
            fse_encode_sym(state_ml, static_cast<std::uint8_t>(ml_code), ee_ml, st_ml, bw);
            fse_encode_sym(state_ll, static_cast<std::uint8_t>(ll_code), ee_ll, st_ll, bw);
        } else {
            // Last real sequence: no incoming transition — init directly.
            state_of = fse_init_state(of_code, ee_of, st_of);
            state_ml = fse_init_state(static_cast<std::uint8_t>(ml_code), ee_ml, st_ml);
            state_ll = fse_init_state(static_cast<std::uint8_t>(ll_code), ee_ll, st_ll);
            first = false;
        }

        // Extra bits, write order LL, ML, OF (a forward decoder reads these
        // as OF, ML, LL immediately after peeking symbols).
        bw.add_bits(ll_extra, LL_CODES[ll_code].extra_bits);
        bw.add_bits(ml_extra, ML_CODES[ml_code].extra_bits);
        bw.add_bits(of_extra, of_code);
    }

    // Flush the initial states (used to peek the FIRST real sequence). A
    // forward decoder reads these FIRST, in order LL, OF, ML; since these
    // are the LAST bits written overall, we write the reverse: ML, OF, LL.
    bw.add_bits(state_ml - sz_ml, ML_ACC_LOG);
    bw.add_bits(state_of - sz_of, OF_ACC_LOG);
    bw.add_bits(state_ll - sz_ll, LL_ACC_LOG);
    bw.flush();

    return std::move(bw).finish();
}

// ─── Block-level compress ────────────────────────────────────────────────────

// Compresses one block (already capped to <= MAX_BLOCK_SIZE by the caller).
// Returns nullopt if the compressed form isn't smaller than `block` (caller
// falls back to a Raw block) or if LZ77 found no matches at all (a
// Compressed block with zero sequences still has section overhead, so a Raw
// block is always at least as good).
inline std::optional<Bytes> compress_block(const Bytes& block) {
    // Window = 32 KB (bigger than LZSS's own 4 KB default, for better
    // ratio); max match = 255, min match = 3 — bounded to fit `ca::lzss::Token`'s
    // uint16_t offset / uint8_t length fields.
    const std::vector<ca::lzss::Token> tokens = ca::lzss::encode(block, 32768, 255, 3);
    auto [lits, seqs] = tokens_to_seqs(tokens);

    if (seqs.empty()) {
        return std::nullopt;
    }

    Bytes out;
    const Bytes lit_sec = encode_literals_section(lits);
    out.insert(out.end(), lit_sec.begin(), lit_sec.end());

    const Bytes sc = encode_seq_count(seqs.size());
    out.insert(out.end(), sc.begin(), sc.end());
    out.push_back(0x00);  // Symbol_Compression_Modes = all Predefined

    const Bytes bitstream = encode_sequences_section(seqs);
    out.insert(out.end(), bitstream.begin(), bitstream.end());

    if (out.size() >= block.size()) {
        return std::nullopt;
    }
    return out;
}

// Decompresses one ZStd Compressed block into `out` (appended). `data` is
// the block's payload (post block-header). Throws ZstdError on any
// malformed input — see the module banner for the exact per-sequence order
// this enforces, and code/specs/CMP07-zstd.md's Security Considerations for
// the validation this performs (offset bounds, output-budget checks inside
// the per-sequence loop, sequence-count cap, symbol-index bounds).
inline void decompress_block(const Bytes& data, Bytes& out) {
    // ── Literals section ────────────────────────────────────────────────
    auto [lits, lit_consumed] = decode_literals_section(data.data(), data.size());
    std::size_t pos = lit_consumed;

    // ── Sequences count ─────────────────────────────────────────────────
    if (pos >= data.size()) {
        check_output_budget(out.size(), lits.size());
        out.insert(out.end(), lits.begin(), lits.end());
        return;
    }

    auto [n_seqs, sc_bytes] = decode_seq_count(data.data() + pos, data.size() - pos);
    pos += sc_bytes;

    if (n_seqs == 0) {
        check_output_budget(out.size(), lits.size());
        out.insert(out.end(), lits.begin(), lits.end());
        return;
    }
    if (n_seqs > MAX_SEQ_COUNT) {
        throw ZstdError("sequence count " + std::to_string(n_seqs) + " exceeds 24-bit field limit");
    }

    // ── Symbol compression modes ────────────────────────────────────────
    if (pos >= data.size()) {
        throw ZstdError("missing symbol compression modes byte");
    }
    const std::uint8_t modes_byte = data[pos];
    pos += 1;

    const std::uint8_t ll_mode = (modes_byte >> 6) & 3;
    const std::uint8_t of_mode = (modes_byte >> 4) & 3;
    const std::uint8_t ml_mode = (modes_byte >> 2) & 3;
    if (ll_mode != 0 || of_mode != 0 || ml_mode != 0) {
        throw ZstdError("unsupported FSE modes: LL=" + std::to_string(ll_mode) +
                         " OF=" + std::to_string(of_mode) + " ML=" + std::to_string(ml_mode) +
                         " (only Predefined=0 supported)");
    }

    // ── FSE bitstream ───────────────────────────────────────────────────
    RevBitReader br(data.data() + pos, data.size() - pos);

    const std::vector<FseDe> dt_ll = build_decode_table(LL_NORM.data(), LL_NORM.size(), LL_ACC_LOG);
    const std::vector<FseDe> dt_ml = build_decode_table(ML_NORM.data(), ML_NORM.size(), ML_ACC_LOG);
    const std::vector<FseDe> dt_of = build_decode_table(OF_NORM.data(), OF_NORM.size(), OF_ACC_LOG);

    // Initial states, read order LL, OF, ML — DIFFERENT from the
    // per-sequence extras order (OF, ML, LL) and update order (LL, ML, OF)
    // below. RFC 8878 is genuinely asymmetric here; see the module banner.
    std::uint16_t state_ll = static_cast<std::uint16_t>(br.read_bits(LL_ACC_LOG));
    std::uint16_t state_of = static_cast<std::uint16_t>(br.read_bits(OF_ACC_LOG));
    std::uint16_t state_ml = static_cast<std::uint16_t>(br.read_bits(ML_ACC_LOG));

    std::size_t lit_pos = 0;

    for (std::size_t i = 0; i < n_seqs; ++i) {
        // Step 1 — PEEK all three symbols from current states. Bare table
        // lookups; consumes NO bits.
        const FseDe ll_entry = fse_peek(state_ll, dt_ll);
        const FseDe ml_entry = fse_peek(state_ml, dt_ml);
        const FseDe of_entry = fse_peek(state_of, dt_of);
        const std::uint8_t ll_code = ll_entry.sym;
        const std::uint8_t ml_code = ml_entry.sym;
        const std::uint8_t of_code = of_entry.sym;

        if (ll_code >= LL_CODES.size()) {
            throw ZstdError("invalid LL code " + std::to_string(ll_code));
        }
        if (ml_code >= ML_CODES.size()) {
            throw ZstdError("invalid ML code " + std::to_string(ml_code));
        }
        const CodeInfo& ll_info = LL_CODES[ll_code];
        const CodeInfo& ml_info = ML_CODES[ml_code];

        // Step 2 — read extra bits, order OF, ML, LL (RFC 8878
        // §3.1.1.3.2.1.2).
        const std::uint32_t of_raw =
            (std::uint32_t(1) << of_code) | static_cast<std::uint32_t>(br.read_bits(of_code));
        const std::uint32_t ml = ml_info.baseline + static_cast<std::uint32_t>(br.read_bits(ml_info.extra_bits));
        const std::uint32_t ll = ll_info.baseline + static_cast<std::uint32_t>(br.read_bits(ll_info.extra_bits));
        if (of_raw < 3) {
            throw ZstdError("decoded offset underflow: of_raw=" + std::to_string(of_raw));
        }
        const std::uint32_t offset = of_raw - 3;

        // Step 3 — update states, order LL, ML, OF, preparing the states
        // the NEXT sequence's peek will use. SKIPPED for the last sequence
        // — see the module banner.
        if (i + 1 != n_seqs) {
            state_ll = fse_update_state(ll_entry, br);
            state_ml = fse_update_state(ml_entry, br);
            state_of = fse_update_state(of_entry, br);
        }

        // Emit `ll` literal bytes from the literals buffer.
        const std::size_t lit_end = lit_pos + ll;
        if (lit_end > lits.size()) {
            throw ZstdError("literal run " + std::to_string(ll) + " overflows literals buffer (pos=" +
                             std::to_string(lit_pos) + " len=" + std::to_string(lits.size()) + ")");
        }
        check_output_budget(out.size(), ll);
        out.insert(out.end(), lits.begin() + static_cast<std::ptrdiff_t>(lit_pos),
                   lits.begin() + static_cast<std::ptrdiff_t>(lit_end));
        lit_pos = lit_end;

        // Copy `ml` bytes from `offset` positions back in the output.
        // offset == 0 has no valid meaning here (minimum valid offset is 1:
        // "the last byte written"); offset beyond what's been produced so
        // far is also invalid.
        if (offset == 0 || static_cast<std::size_t>(offset) > out.size()) {
            throw ZstdError("bad match offset " + std::to_string(offset) + " (output len " +
                             std::to_string(out.size()) + ")");
        }
        check_output_budget(out.size(), ml);
        const std::size_t copy_start = out.size() - offset;
        for (std::size_t j = 0; j < ml; ++j) {
            out.push_back(out[copy_start + j]);
        }
    }

    // Any remaining literals after the last sequence.
    check_output_budget(out.size(), lits.size() - lit_pos);
    out.insert(out.end(), lits.begin() + static_cast<std::ptrdiff_t>(lit_pos), lits.end());
}

}  // namespace detail

// ─── Public API ───────────────────────────────────────────────────────────

// Compresses `data` to a valid ZStd frame (RFC 8878 subset — see the module
// banner). The output can be decompressed by the real `zstd` CLI or this
// package's `decompress()`. Never throws: every input has a valid encoding
// (a Raw block copy, in the worst case).
inline Bytes compress(const Bytes& data) {
    Bytes out;

    // ── Frame header ────────────────────────────────────────────────────
    out.push_back(static_cast<std::uint8_t>(detail::MAGIC & 0xFF));
    out.push_back(static_cast<std::uint8_t>((detail::MAGIC >> 8) & 0xFF));
    out.push_back(static_cast<std::uint8_t>((detail::MAGIC >> 16) & 0xFF));
    out.push_back(static_cast<std::uint8_t>((detail::MAGIC >> 24) & 0xFF));

    // Frame Header Descriptor (FHD):
    //   bits [7:6] = FCS_Field_Size = 11 -> 8-byte FCS
    //   bit  [5]   = Single_Segment_Flag = 1 (no Window_Descriptor follows)
    //   bit  [4]   = Unused_bit = 0
    //   bit  [3]   = Reserved_bit = 0
    //   bit  [2]   = Content_Checksum_Flag = 0 (we don't append a checksum —
    //                see Lesson 95: this is bit 2, NOT bit 4)
    //   bits [1:0] = Dict_ID_Flag = 0
    // = 0b1110_0000 = 0xE0
    out.push_back(0xE0);

    // Frame_Content_Size (8 bytes LE): the uncompressed size. A decoder may
    // use this as a size HINT only — never to pre-allocate output (see
    // Security Considerations); this implementation's own decoder ignores
    // it entirely and grows output incrementally.
    const std::uint64_t content_size = data.size();
    for (int i = 0; i < 8; ++i) {
        out.push_back(static_cast<std::uint8_t>((content_size >> (8 * i)) & 0xFF));
    }

    // ── Blocks ──────────────────────────────────────────────────────────
    if (data.empty()) {
        // Empty input still needs one (empty) block: Last=1, Type=Raw(00),
        // Size=0 -> header bytes = [0x01, 0x00, 0x00].
        out.push_back(0x01);
        out.push_back(0x00);
        out.push_back(0x00);
        return out;
    }

    std::size_t offset = 0;
    while (offset < data.size()) {
        const std::size_t end = std::min(offset + detail::MAX_BLOCK_SIZE, data.size());
        const bool last = (end == data.size());
        const Bytes block(data.begin() + static_cast<std::ptrdiff_t>(offset),
                           data.begin() + static_cast<std::ptrdiff_t>(end));

        const bool all_same =
            std::all_of(block.begin(), block.end(), [&](std::uint8_t b) { return b == block[0]; });

        if (all_same) {
            // RLE block: 1 byte total, repeated Block_Size times.
            const std::uint32_t hdr = (static_cast<std::uint32_t>(block.size()) << 3) | (0b01u << 1) |
                                       (last ? 1u : 0u);
            out.push_back(static_cast<std::uint8_t>(hdr & 0xFF));
            out.push_back(static_cast<std::uint8_t>((hdr >> 8) & 0xFF));
            out.push_back(static_cast<std::uint8_t>((hdr >> 16) & 0xFF));
            out.push_back(block[0]);
        } else if (auto compressed = detail::compress_block(block)) {
            const std::uint32_t hdr = (static_cast<std::uint32_t>(compressed->size()) << 3) |
                                       (0b10u << 1) | (last ? 1u : 0u);
            out.push_back(static_cast<std::uint8_t>(hdr & 0xFF));
            out.push_back(static_cast<std::uint8_t>((hdr >> 8) & 0xFF));
            out.push_back(static_cast<std::uint8_t>((hdr >> 16) & 0xFF));
            out.insert(out.end(), compressed->begin(), compressed->end());
        } else {
            // Raw block fallback.
            const std::uint32_t hdr = (static_cast<std::uint32_t>(block.size()) << 3) | (last ? 1u : 0u);
            out.push_back(static_cast<std::uint8_t>(hdr & 0xFF));
            out.push_back(static_cast<std::uint8_t>((hdr >> 8) & 0xFF));
            out.push_back(static_cast<std::uint8_t>((hdr >> 16) & 0xFF));
            out.insert(out.end(), block.begin(), block.end());
        }

        offset = end;
    }

    return out;
}

// Decompresses a ZStd frame, returning the original data.
//
// Accepts Single_Segment or multi-segment layouts, Raw/RLE/Compressed
// blocks, and Predefined-mode FSE sequences. Throws ZstdError on truncation,
// a bad magic number, an unsupported feature (custom FSE tables, Huffman
// literals, the Reserved block type), or trailing bytes after the frame end
// (see Lesson 94 — a strict decoder must reject concatenated/corrupted
// input, not silently ignore what follows a valid-looking frame).
inline Bytes decompress(const Bytes& data) {
    if (data.size() < 5) {
        throw ZstdError("frame too short");
    }

    // ── Magic ───────────────────────────────────────────────────────────
    const std::uint32_t magic = static_cast<std::uint32_t>(data[0]) |
                                 (static_cast<std::uint32_t>(data[1]) << 8) |
                                 (static_cast<std::uint32_t>(data[2]) << 16) |
                                 (static_cast<std::uint32_t>(data[3]) << 24);
    if (magic != detail::MAGIC) {
        throw ZstdError("bad magic number");
    }

    std::size_t pos = 4;

    // ── Frame Header Descriptor ─────────────────────────────────────────
    const std::uint8_t fhd = data[pos];
    pos += 1;

    const std::uint8_t fcs_flag = (fhd >> 6) & 3;
    const std::uint8_t single_seg = (fhd >> 5) & 1;
    // Content_Checksum_Flag is bit 2 (NOT bit 4 — Lesson 95). Verified
    // against RFC 8878 §3.1.1.1 and empirically against the real `zstd`
    // CLI: `zstd -c file` (checksum on by default) emits FHD 0x64;
    // `zstd -c --no-check file` emits FHD 0x60 — the differing bit is
    // bit 2.
    const std::uint8_t checksum_flag = (fhd >> 2) & 1;
    const std::uint8_t dict_flag = fhd & 3;

    // ── Window Descriptor (present only if Single_Segment_Flag == 0) ────
    if (single_seg == 0) {
        if (pos >= data.size()) {
            throw ZstdError("truncated window descriptor");
        }
        pos += 1;
    }

    // ── Dictionary ID ───────────────────────────────────────────────────
    static constexpr std::array<std::size_t, 4> DICT_ID_BYTES = {0, 1, 2, 4};
    const std::size_t dict_id_bytes = DICT_ID_BYTES[dict_flag];
    if (pos + dict_id_bytes > data.size()) {
        throw ZstdError("truncated dictionary id");
    }
    pos += dict_id_bytes;  // dictionaries are not supported; skip only

    // ── Frame Content Size ──────────────────────────────────────────────
    std::size_t fcs_bytes = 0;
    switch (fcs_flag) {
        case 0:
            fcs_bytes = (single_seg == 1) ? 1 : 0;
            break;
        case 1:
            fcs_bytes = 2;
            break;
        case 2:
            fcs_bytes = 4;
            break;
        case 3:
            fcs_bytes = 8;
            break;
        default:
            break;  // unreachable: fcs_flag is 2 bits
    }
    if (pos + fcs_bytes > data.size()) {
        throw ZstdError("truncated frame content size");
    }
    pos += fcs_bytes;  // FCS is an untrusted hint; never used to pre-allocate

    // ── Blocks ──────────────────────────────────────────────────────────
    Bytes out;
    for (;;) {
        if (pos + 3 > data.size()) {
            throw ZstdError("truncated block header");
        }

        const std::uint32_t hdr = static_cast<std::uint32_t>(data[pos]) |
                                   (static_cast<std::uint32_t>(data[pos + 1]) << 8) |
                                   (static_cast<std::uint32_t>(data[pos + 2]) << 16);
        pos += 3;

        const bool last = (hdr & 1) != 0;
        const std::uint32_t btype = (hdr >> 1) & 3;
        const std::size_t bsize = static_cast<std::size_t>(hdr >> 3);

        // Block size cap: real zstd bounds the REGENERATED size of every
        // block (Raw, RLE, and Compressed alike) to <= 128 KB (see
        // detail::MAX_BLOCK_SIZE's doc comment). A block claiming more is
        // malformed.
        if (bsize > detail::MAX_BLOCK_SIZE) {
            throw ZstdError("block size " + std::to_string(bsize) + " exceeds 128 KB cap");
        }

        switch (btype) {
            case 0: {  // Raw
                if (pos + bsize > data.size()) {
                    throw ZstdError("raw block truncated: need " + std::to_string(bsize) +
                                     " bytes at pos " + std::to_string(pos));
                }
                detail::check_output_budget(out.size(), bsize);
                out.insert(out.end(), data.begin() + static_cast<std::ptrdiff_t>(pos),
                           data.begin() + static_cast<std::ptrdiff_t>(pos + bsize));
                pos += bsize;
                break;
            }
            case 1: {  // RLE
                if (pos >= data.size()) {
                    throw ZstdError("RLE block missing byte");
                }
                detail::check_output_budget(out.size(), bsize);
                const std::uint8_t byte = data[pos];
                pos += 1;
                out.insert(out.end(), bsize, byte);
                break;
            }
            case 2: {  // Compressed
                if (pos + bsize > data.size()) {
                    throw ZstdError("compressed block truncated: need " + std::to_string(bsize) +
                                     " bytes");
                }
                const Bytes block_data(data.begin() + static_cast<std::ptrdiff_t>(pos),
                                        data.begin() + static_cast<std::ptrdiff_t>(pos + bsize));
                pos += bsize;
                detail::decompress_block(block_data, out);
                break;
            }
            case 3:
                throw ZstdError("reserved block type 3");
            default:
                break;  // unreachable: btype is 2 bits
        }

        if (last) {
            break;
        }
    }

    // ── Content checksum ────────────────────────────────────────────────
    // We don't compute/verify xxHash64 (no such implementation in this
    // package), but must still skip the 4 checksum bytes when present so
    // the trailing-bytes check below doesn't misfire on real-world frames
    // (real `zstd` writes a checksum by default).
    if (checksum_flag == 1) {
        if (pos + 4 > data.size()) {
            throw ZstdError("truncated content checksum");
        }
        pos += 4;
    }

    // Reject trailing bytes after the frame end (Lesson 94): a fuzzed or
    // concatenated input must fail loudly rather than silently returning
    // partial output.
    if (pos != data.size()) {
        throw ZstdError("trailing bytes after end of frame (" + std::to_string(data.size() - pos) +
                         " byte(s))");
    }

    return out;
}

}  // namespace zstd
}  // namespace ca

#endif  // CA_ZSTD_HPP
