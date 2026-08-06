// deflate.hpp — RFC 1951 DEFLATE lossless compression, in pure ISO C++17,
// header-only, in namespace ca::deflate. A faithful port of the Rust
// `deflate` crate (CMP05).
// ===========================================================================
//
// DEFLATE (Phil Katz, PKZIP 1989; specified as RFC 1951 by L. Peter Deutsch,
// 1996) is the compression layer inside ZIP, gzip, PNG, and zlib. It is a
// COMPOSITION of two earlier techniques in this series:
//
//   1. LZSS tokenization (CMP02, see the sibling `lzss` package) — replaces
//      repeated substrings with back-references into a sliding window. Here
//      the window is the full RFC 1951 32768-byte window, so matches are
//      `Literal(byte)` or `Match{offset: 1..=32768, length: 3..=255}`.
//
//   2. Huffman coding (CMP04) — entropy-codes the resulting token stream.
//      `compress` builds BOTH a fixed-table encoding (RFC 1951 §3.2.6, no
//      table transmitted) and a dynamic (per-block, data-adapted) encoding,
//      then emits whichever is smaller in exact bits. The dynamic tree is
//      length-limited to 15 bits via the package-merge algorithm.
//
// THE EXPANDED LITERAL/LENGTH (LL) ALPHABET. LZSS produces two token kinds;
// DEFLATE Huffman-codes them with ONE shared alphabet:
//
//   Symbols   0–255  literal byte values
//   Symbol      256  end-of-block marker (replaces an original-length count —
//                     a single Match token can expand to many output bytes,
//                     so token count and byte count don't correspond 1:1)
//   Symbols 257–285  length codes: each covers a range of match lengths via
//                     a base value plus "extra bits" (raw, not Huffman-coded)
//
// A separate, smaller alphabet (30 symbols) encodes back-reference distances
// the same way (base + extra bits).
//
// WIRE FORMAT — standard RFC 1951 raw DEFLATE (no envelope, no private
// header): the exact bytes a ZIP entry or gzip body carries.
//
//   [3 bits]  block header — BFINAL=1, BTYPE=01 (fixed) or 10 (dynamic), LSB-first
//   [ ... ]   (dynamic only) HLIT/HDIST/HCLEN + code-length trees, RLE'd
//   [ ... ]   token stream — literals / (length,distance) matches; Huffman
//             codes MSB-first, extra bits LSB-first
//   [ n bits] end-of-block — symbol 256
//
// `compress` always emits exactly ONE final block (BFINAL=1) and picks fixed
// vs. dynamic by comparing EXACT emitted-bit counts of the same token stream,
// so it never produces a larger stream than fixed-only encoding.
//
// `inflate` (aliased as `decompress`) reads ALL THREE RFC 1951 block types —
// stored (BTYPE=00), fixed Huffman (BTYPE=01), dynamic Huffman (BTYPE=10) —
// so it decodes this library's own output as well as real `zlib`/`gzip`/
// Microsoft Office streams. This asymmetry — encode conservatively (our own
// encoder never needs distance codes 24–29 or LL symbol 285, since it caps
// match length at 255), decode liberally (the full standard alphabet) — is
// why a decoder that only recognises its own subset fails to open real files,
// and why this decoder recognises the full range.
//
// ERROR HANDLING (this repo's convention for decoders of untrusted bytes,
// mirroring `canonical-cbor`'s `CborException`): `inflate`/`decompress` throw
// `DeflateException` (carrying a `DeflateError`) on any malformed input.
// `compress` never fails — it returns `std::vector<uint8_t>` directly.
//
// ROBUSTNESS. `inflate` enforces `MAX_INFLATE_OUTPUT` (256 MB) against
// decompression bombs, validates every back-reference distance against the
// bytes decoded so far, validates length/distance Huffman symbols against
// their tables, and guards every size computation against integer overflow
// before it reaches a `push_back`/`reserve` call.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only. No <span> (that
// is C++20); byte buffers are `std::vector<uint8_t>`, matching the sibling
// `lzss` package's convention.
//
// Dependency: the sibling `lzss` package (CMP02) supplies `ca::lzss::Token`
// and `ca::lzss::encode` for the LZSS tokenization pass — see `BUILD`.
#ifndef CA_DEFLATE_HPP
#define CA_DEFLATE_HPP

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <exception>
#include <unordered_map>
#include <utility>
#include <vector>

#include "lzss.hpp"

namespace ca {
namespace deflate {

using Bytes = std::vector<std::uint8_t>;

// ===========================================================================
// Errors
// ===========================================================================

// Decoder error kinds. `InternalEncoderInvariant` guards an encoder-side
// invariant (see `length_limited_huffman` below) that should be
// unreachable for our fixed alphabet sizes; it is kept as a hard, always-on
// check rather than a debug-only assert so a future regression aborts loudly
// instead of silently emitting a malformed stream.
enum class DeflateError {
    UnexpectedEof,
    InvalidHuffmanCode,
    InvalidLengthSymbol,
    InvalidDistanceSymbol,
    InvalidClSymbol,
    CodeLengthOverflow,
    StoredBlockLenMismatch,
    BackReferenceOutOfRange,
    OutputSizeExceeded,
    ReservedBlockType,
    InternalEncoderInvariant,
};

// Thrown by `inflate` / `decompress` on any violation of RFC 1951, and (in
// the InternalEncoderInvariant case only) by `compress` if a package-merge
// invariant is ever violated — which should be unreachable.
class DeflateException : public std::exception {
public:
    explicit DeflateException(DeflateError e) : err_(e) {}
    DeflateError error() const noexcept { return err_; }
    const char* what() const noexcept override { return "deflate error"; }

private:
    DeflateError err_;
};

namespace detail {

// ===========================================================================
// Length code table (LL symbols 257–285)
// ===========================================================================
//
// Each length symbol covers a range of match lengths (3–258). The exact
// length within the range is encoded as `extra_bits` raw bits after the
// Huffman code.
//
//   length=13 -> symbol 266 (base=13, extra=1, extra_value=0 -> bit "0")
//   length=14 -> symbol 266 (base=13, extra=1, extra_value=1 -> bit "1")

struct LengthEntry {
    std::uint16_t symbol;
    std::uint32_t base;
    std::uint32_t extra_bits;
};

inline constexpr std::array<LengthEntry, 29> LENGTH_TABLE = {{
    {257, 3, 0},   {258, 4, 0},   {259, 5, 0},   {260, 6, 0},
    {261, 7, 0},   {262, 8, 0},   {263, 9, 0},   {264, 10, 0},
    {265, 11, 1},  {266, 13, 1},  {267, 15, 1},  {268, 17, 1},
    {269, 19, 2},  {270, 23, 2},  {271, 27, 2},  {272, 31, 2},
    {273, 35, 3},  {274, 43, 3},  {275, 51, 3},  {276, 59, 3},
    {277, 67, 4},  {278, 83, 4},  {279, 99, 4},  {280, 115, 4},
    {281, 131, 5}, {282, 163, 5}, {283, 195, 5}, {284, 227, 5},
    // Symbol 285 is the special "maximum match" code: length 258 exactly,
    // with NO extra bits. Real-world producers with a full 32 KB window
    // (zlib, gzip, Office) emit it routinely; our own encoder never reaches
    // length 258 (it caps match length at 255), but the DECODER must
    // recognise it to read real-world streams.
    {285, 258, 0},
}};

// ===========================================================================
// Distance code table (codes 0–29)
// ===========================================================================
//
// Covers the full RFC 1951 32768-byte window (code 29 reaches 32768). Our
// own encoder uses this same 32768-byte window, so every code here is
// reachable by `compress`; the full table is retained (rather than the
// 24-code / 4096-byte subset some smaller ports use) precisely so `inflate`
// reads real-world 32 KB-window streams too.

struct DistEntry {
    std::uint16_t code;
    std::uint32_t base;
    std::uint32_t extra_bits;
};

inline constexpr std::array<DistEntry, 30> DIST_TABLE = {{
    {0, 1, 0},        {1, 2, 0},        {2, 3, 0},        {3, 4, 0},
    {4, 5, 1},        {5, 7, 1},        {6, 9, 2},        {7, 13, 2},
    {8, 17, 3},       {9, 25, 3},       {10, 33, 4},      {11, 49, 4},
    {12, 65, 5},      {13, 97, 5},      {14, 129, 6},     {15, 193, 6},
    {16, 257, 7},     {17, 385, 7},     {18, 513, 8},     {19, 769, 8},
    {20, 1025, 9},    {21, 1537, 9},    {22, 2049, 10},   {23, 3073, 10},
    {24, 4097, 11},   {25, 6145, 11},   {26, 8193, 12},   {27, 12289, 12},
    {28, 16385, 13},  {29, 24577, 13},
}};

// ===========================================================================
// Length / distance symbol lookup helpers
// ===========================================================================

inline std::uint16_t length_symbol(std::uint32_t length) {
    for (const auto& e : LENGTH_TABLE) {
        std::uint32_t max_len = e.base + (std::uint32_t(1) << e.extra_bits) - 1;
        if (length <= max_len) {
            return e.symbol;
        }
    }
    return 284;
}

inline std::uint16_t dist_code_for(std::uint32_t offset) {
    for (const auto& e : DIST_TABLE) {
        std::uint32_t max_dist = e.base + (std::uint32_t(1) << e.extra_bits) - 1;
        if (offset <= max_dist) {
            return e.code;
        }
    }
    return 23;
}

inline std::uint32_t length_base(std::uint16_t sym) {
    for (const auto& e : LENGTH_TABLE) {
        if (e.symbol == sym) {
            return e.base;
        }
    }
    return 0;
}

inline std::uint32_t length_extra(std::uint16_t sym) {
    for (const auto& e : LENGTH_TABLE) {
        if (e.symbol == sym) {
            return e.extra_bits;
        }
    }
    return 0;
}

inline std::uint32_t dist_base(std::uint16_t code) {
    for (const auto& e : DIST_TABLE) {
        if (e.code == code) {
            return e.base;
        }
    }
    return 0;
}

inline std::uint32_t dist_extra(std::uint16_t code) {
    for (const auto& e : DIST_TABLE) {
        if (e.code == code) {
            return e.extra_bits;
        }
    }
    return 0;
}

// ===========================================================================
// Bit I/O — encoder side
// ===========================================================================

// Accumulates bits into a byte buffer, LSB-first: the first bit written
// occupies bit 0 (the least-significant bit) of the first byte, and bits
// fill each byte low-to-high before moving to the next byte. This is the
// same convention used by the sibling CMP02/CMP03/CMP04 ports.
class BitWriter {
public:
    // Write a Huffman code of `nbits` bits.
    //
    // RFC 1951 assigns Huffman codes MOST-significant-bit first, but the bit
    // stream itself is packed LSB-first. So we bit-reverse the low `nbits` of
    // `code` and emit THAT LSB-first — the decoder, reading LSB-first and
    // re-accumulating MSB-first, recovers the original code. This is the
    // exact inverse of `decode_symbol` below.
    void write_huffman(std::uint32_t code, std::uint32_t nbits) {
        write_raw_bits_lsb(reverse_bits(code, nbits), nbits);
    }

    // Write `n` raw bits from `val`, LSB of `val` first.
    void write_raw_bits_lsb(std::uint32_t val, std::uint32_t n) {
        for (std::uint32_t i = 0; i < n; ++i) {
            if (((val >> i) & 1u) != 0u) {
                buf_ |= (std::uint64_t(1) << bit_pos_);
            }
            ++bit_pos_;
            if (bit_pos_ == 64) {
                for (int k = 0; k < 8; ++k) {
                    out_.push_back(static_cast<std::uint8_t>(buf_ & 0xFFu));
                    buf_ >>= 8;
                }
                bit_pos_ = 0;
            }
        }
    }

    // Flush any partial byte, zero-padding to the next byte boundary.
    void flush() {
        while (bit_pos_ > 0) {
            out_.push_back(static_cast<std::uint8_t>(buf_ & 0xFFu));
            buf_ >>= 8;
            bit_pos_ = (bit_pos_ >= 8) ? (bit_pos_ - 8) : 0;
        }
    }

    Bytes finish() {
        flush();
        return std::move(out_);
    }

private:
    static std::uint32_t reverse_bits(std::uint32_t code, std::uint32_t nbits) {
        std::uint32_t result = 0;
        for (std::uint32_t i = 0; i < nbits; ++i) {
            result = (result << 1) | ((code >> i) & 1u);
        }
        return result;
    }

    std::uint64_t buf_ = 0;
    std::uint32_t bit_pos_ = 0;
    Bytes out_;
};

// ===========================================================================
// Fixed literal/length codes (RFC 1951 §3.2.6) — encoder side
// ===========================================================================
//
// The fixed literal/length codes are pre-agreed, so `compress` transmits no
// table:
//
//   Symbols   0–143 -> 8-bit codes, starting at 0b0011_0000  (= 48)
//   Symbols 144–255 -> 9-bit codes, starting at 0b1_1001_0000 (= 400)
//   Symbols 256–279 -> 7-bit codes, starting at 0b000_0000    (= 0)
//   Symbols 280–287 -> 8-bit codes, starting at 0b1100_0000   (= 192)
//
// Distance symbols are all 5-bit codes equal to the symbol number (emitted
// directly via `write_huffman(code, 5)` at the call site — no table needed).

inline std::pair<std::uint32_t, std::uint32_t> fixed_ll_code(std::uint16_t sym) {
    std::uint32_t s = sym;
    if (s <= 143) {
        return {0x30u + s, 8u};
    }
    if (s <= 255) {
        return {0x190u + (s - 144u), 9u};
    }
    if (s <= 279) {
        return {s - 256u, 7u};
    }
    if (s <= 287) {
        return {0xC0u + (s - 280u), 8u};
    }
    // Unreachable: every LL symbol compress() ever asks for is 0..287. A
    // request outside that range would be an encoder bug, not malformed
    // input, so this mirrors the Rust reference's `panic!`.
    throw DeflateException(DeflateError::InternalEncoderInvariant);
}

// ===========================================================================
// Length-limited Huffman: the package-merge algorithm (Larmore–Hirschberg 1990)
// ===========================================================================
//
// RFC 1951 caps every Huffman code at 15 bits (literal/length and distance
// alphabets) and every code-length (CL) code at 7 bits. A plain (unlimited)
// Huffman tree, built greedily, CAN exceed those limits on skewed
// frequencies — a symbol appearing once among a million copies of another
// wants a code deeper than 15 bits. So we need the OPTIMAL *length-limited*
// code: minimum total bits subject to max-length <= L.
//
// Package-merge (Larmore & Hirschberg, JACM 1990) solves this as the
// "coin-collector's problem":
//
//   - Kraft's inequality: a code with lengths l_i is valid iff
//     sum(2^-l_i) <= 1. Equivalently, capping every code at L bits, the
//     total "width" sum(2^(L-l_i)) must be <= 2^L.
//   - Model each *bit of depth* a symbol occupies as a coin: symbol i at
//     depth d (1 <= d <= L) is a coin of weight = freq_i, and we need to
//     "buy" exactly 2n-2 of the cheapest coins (n = symbol count) across L
//     levels — the coin-collector's problem.
//
// Concrete procedure (see zlib's `zopfli`, Wikipedia's "Package-merge
// algorithm"):
//
//   1. The symbols with nonzero frequency are the *original coins*.
//   2. For each level from L down to 1, maintain a sorted-by-weight list of
//      *items* — either an original symbol or a *package* of two items from
//      the previous (deeper) level.
//   3. Start: level-L list = all symbols, sorted by weight ascending.
//      Level -> next-shallower level:
//        a. Package: pair up the current list's items greedily
//           (items[0]+items[1], items[2]+items[3], ...), dropping a leftover
//           odd item. Each pair becomes one package (summed weight, unioned
//           coverage).
//        b. Merge those packages with the ORIGINAL symbol list (both sorted)
//           into the next level's list.
//   4. After building level-1's list, select the 2n-2 lowest-weight items.
//      Symbol i's code length = the number of selected items that cover it.
//
// We track item "coverage" explicitly (a small vector of covered original-
// symbol indices per item) rather than a cleverer implicit encoding —
// alphabets here are tiny (<=286 symbols, <=15 levels), so the O(n) per-item
// vector cost is negligible and the code stays easy to verify against the
// algorithm description above.
//
// WHY THIS IS CORRECT:
//   - Package-merge provably yields the OPTIMAL code among all codes with
//     max-length <= L.
//   - It ALWAYS produces a valid code (Kraft sum <= 1) whenever a
//     length-limited code exists at all, i.e. whenever n <= 2^L. For us:
//     LL n <= 286 <= 2^15, dist n <= 30 <= 2^15, CL n <= 19 <= 2^7 = 128. So
//     it never fails for our alphabets and never emits a length > L. We
//     additionally check the Kraft sum and per-symbol length range as a hard
//     (release-time, not debug-only) invariant: a malformed tree from a
//     future regression must never reach the wire.

// Verify code lengths satisfy Kraft's inequality:
// sum(2^(max_len - len)) <= 2^max_len, and no length exceeds max_len. Uses
// integer arithmetic to avoid floating-point error.
inline bool kraft_sum_ok(const std::vector<std::uint8_t>& lengths, std::uint32_t max_len) {
    std::uint64_t total = 0;
    std::uint64_t limit = std::uint64_t(1) << max_len;
    for (std::uint8_t l : lengths) {
        if (l == 0) {
            continue;
        }
        if (static_cast<std::uint32_t>(l) > max_len) {
            return false;
        }
        total += std::uint64_t(1) << (max_len - static_cast<std::uint32_t>(l));
    }
    return total <= limit;
}

// Compute optimal Huffman code lengths for the given symbol frequencies,
// limited to at most `max_len` bits per code.
//
// `freqs[i]` is the frequency of symbol `i`. Symbols with frequency 0 are
// absent from the alphabet and receive length 0. Returns `lengths` where
// `lengths[i]` is the bit length (1..=max_len) for present symbols, 0 for
// absent ones.
inline std::vector<std::uint8_t> length_limited_huffman(
    const std::vector<std::uint32_t>& freqs, std::uint32_t max_len) {
    std::size_t n = freqs.size();
    std::vector<std::uint8_t> lengths(n, 0);

    std::vector<std::size_t> present;
    for (std::size_t i = 0; i < n; ++i) {
        if (freqs[i] > 0) {
            present.push_back(i);
        }
    }
    std::size_t m = present.size();

    // Degenerate cases the general algorithm doesn't need to handle:
    if (m == 0) {
        // No symbols at all -- empty code. The caller handles the "must
        // have >= 1 code" rule (e.g. dummy distance code) separately.
        return lengths;
    }
    if (m == 1) {
        // A single symbol needs a valid 1-bit code: a 0-bit code is not a
        // prefix code. RFC 1951 §3.2.7 and zlib both assign length 1 here.
        lengths[present[0]] = 1;
        return lengths;
    }

    // An `Item` is a coin (or package of coins): a weight plus the set of
    // original-symbol indices (into `present`) whose depth-count it
    // contributes to.
    struct Item {
        std::uint64_t weight;
        std::vector<std::uint32_t> covers;
    };

    // The "original coins": one per present symbol, sorted by weight
    // ascending (ties broken by index for determinism).
    std::vector<Item> originals;
    originals.reserve(m);
    for (std::size_t idx = 0; idx < m; ++idx) {
        Item it;
        it.weight = freqs[present[idx]];
        it.covers.push_back(static_cast<std::uint32_t>(idx));
        originals.push_back(std::move(it));
    }
    std::sort(originals.begin(), originals.end(), [](const Item& a, const Item& b) {
        if (a.weight != b.weight) {
            return a.weight < b.weight;
        }
        return a.covers[0] < b.covers[0];
    });

    // Level-L list starts as the originals.
    std::vector<Item> list = originals;

    // Walk from level L down to level 2 (max_len - 1 package+merge steps),
    // ending with the level-1 list.
    for (std::uint32_t level = 1; level < max_len; ++level) {
        // Package: pair adjacent items (list is sorted ascending), summing
        // weights and unioning covers. Drop a trailing odd item.
        std::vector<Item> packaged;
        packaged.reserve(list.size() / 2);
        std::size_t k = 0;
        while (k + 1 < list.size()) {
            Item pkg;
            pkg.weight = list[k].weight + list[k + 1].weight;
            pkg.covers = list[k].covers;
            pkg.covers.insert(pkg.covers.end(), list[k + 1].covers.begin(),
                               list[k + 1].covers.end());
            packaged.push_back(std::move(pkg));
            k += 2;
        }

        // Merge the packaged list with the ORIGINALS (both sorted ascending)
        // into the next shallower level's list.
        std::vector<Item> merged;
        merged.reserve(packaged.size() + originals.size());
        std::size_t i = 0;
        std::size_t j = 0;
        while (i < originals.size() && j < packaged.size()) {
            if (originals[i].weight <= packaged[j].weight) {
                merged.push_back(originals[i]);
                ++i;
            } else {
                merged.push_back(packaged[j]);
                ++j;
            }
        }
        while (i < originals.size()) {
            merged.push_back(originals[i]);
            ++i;
        }
        while (j < packaged.size()) {
            merged.push_back(packaged[j]);
            ++j;
        }
        list = std::move(merged);
    }

    // Select the 2m-2 lowest-weight items from the level-1 list (already
    // sorted ascending). Each selected item that covers a symbol contributes
    // 1 to that symbol's code length (its depth-count).
    std::size_t take = 2 * m - 2;
    std::vector<std::uint32_t> depth(m, 0);
    for (std::size_t idx = 0; idx < take && idx < list.size(); ++idx) {
        for (std::uint32_t c : list[idx].covers) {
            depth[c] += 1;
        }
    }

    // Every present symbol must end up with depth in [1, max_len].
    // Package-merge with m >= 2 guarantees this for our alphabet sizes
    // (n <= 2^max_len); this check is a hard invariant (always on, not a
    // debug-only assert) precisely because a violation would otherwise
    // truncate silently into an INVALID RFC 1951 code.
    for (std::size_t idx = 0; idx < m; ++idx) {
        std::uint32_t d = depth[idx];
        if (d < 1 || d > max_len) {
            throw DeflateException(DeflateError::InternalEncoderInvariant);
        }
        lengths[present[idx]] = static_cast<std::uint8_t>(d);
    }

    if (!kraft_sum_ok(lengths, max_len)) {
        throw DeflateException(DeflateError::InternalEncoderInvariant);
    }

    return lengths;
}

// ===========================================================================
// Canonical code assignment (RFC 1951 §3.2.2)
// ===========================================================================
//
// Given code lengths, RFC 1951 assigns canonical codes deterministically:
//
//   1. Count how many symbols have each length.
//   2. Compute the starting code for each length:
//        next_code[len] = (next_code[len-1] + count[len-1]) << 1
//   3. Assign codes in symbol order within each length.
//
// The encoder (`build_canonical_codes`, returning `(code, len)` pairs so
// `write_huffman` can emit MSB-first) and the decoder (`build_huffman_decoder`
// below, returning a lookup table) compute the SAME assignment from the
// SAME algorithm, so an encoder/decoder pair agree by construction.

inline std::vector<std::pair<std::uint32_t, std::uint32_t>> build_canonical_codes(
    const std::vector<std::uint8_t>& lengths) {
    std::size_t n = lengths.size();
    std::vector<std::pair<std::uint32_t, std::uint32_t>> codes(n, {0u, 0u});

    std::uint8_t max_len_u8 = 0;
    for (std::uint8_t l : lengths) {
        if (l > max_len_u8) {
            max_len_u8 = l;
        }
    }
    std::size_t max_len = max_len_u8;
    if (max_len == 0) {
        return codes;
    }

    std::vector<std::uint32_t> bl_count(max_len + 1, 0);
    for (std::uint8_t l : lengths) {
        if (l > 0) {
            bl_count[l] += 1;
        }
    }

    std::vector<std::uint32_t> next_code(max_len + 2, 0);
    std::uint32_t code = 0;
    bl_count[0] = 0;
    for (std::size_t bits = 1; bits <= max_len; ++bits) {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }

    for (std::size_t sym = 0; sym < n; ++sym) {
        std::uint8_t l = lengths[sym];
        if (l > 0) {
            std::size_t len = l;
            codes[sym] = {next_code[len], static_cast<std::uint32_t>(len)};
            next_code[len] += 1;
        }
    }
    return codes;
}

// ===========================================================================
// Dynamic-Huffman block encoder (BTYPE=10, RFC 1951 §3.2.7)
// ===========================================================================

// A single element of the RLE'd code-length stream:
//
//   sym 0-15 : a literal code length (no extra bits)
//   sym 16   : repeat previous length, extra = (count-3), count 3..6  (2 bits)
//   sym 17   : run of zeros,          extra = (count-3), count 3..10 (3 bits)
//   sym 18   : run of zeros,          extra = (count-11), count 11..138 (7 bits)
struct ClItem {
    std::uint16_t sym;
    std::uint32_t extra_bits;
    std::uint32_t extra_val;
};

// RLE-encode a sequence of code lengths into CL symbols per RFC 1951 §3.2.7.
//
//   - A literal length L (0..15) is emitted as symbol L.
//   - A run of the SAME nonzero length uses symbol 16 (repeat-previous) for
//     3..6 additional copies after at least one literal emission.
//   - A run of zeros uses symbol 18 (11..138), then symbol 17 (3..10), then
//     literal zeros (1..2) for whatever remains.
inline std::vector<ClItem> rle_code_lengths(const std::vector<std::uint8_t>& lengths) {
    std::vector<ClItem> out;
    std::size_t n = lengths.size();
    std::size_t i = 0;
    while (i < n) {
        std::uint8_t cur = lengths[i];
        std::size_t run = 1;
        while (i + run < n && lengths[i + run] == cur) {
            ++run;
        }

        if (cur == 0) {
            std::size_t remaining = run;
            while (remaining >= 11) {
                std::size_t count = std::min<std::size_t>(remaining, 138);
                out.push_back(ClItem{18, 7, static_cast<std::uint32_t>(count - 11)});
                remaining -= count;
            }
            while (remaining >= 3) {
                std::size_t count = std::min<std::size_t>(remaining, 10);
                out.push_back(ClItem{17, 3, static_cast<std::uint32_t>(count - 3)});
                remaining -= count;
            }
            for (std::size_t r = 0; r < remaining; ++r) {
                out.push_back(ClItem{0, 0, 0});
            }
        } else {
            out.push_back(ClItem{static_cast<std::uint16_t>(cur), 0, 0});
            std::size_t remaining = run - 1;
            while (remaining >= 3) {
                std::size_t count = std::min<std::size_t>(remaining, 6);
                out.push_back(ClItem{16, 2, static_cast<std::uint32_t>(count - 3)});
                remaining -= count;
            }
            for (std::size_t r = 0; r < remaining; ++r) {
                out.push_back(ClItem{static_cast<std::uint16_t>(cur), 0, 0});
            }
        }
        i += run;
    }
    return out;
}

// The wire order for transmitting CL code lengths: front-loads the most
// commonly-used lengths so HCLEN can often be small.
inline constexpr std::array<std::size_t, 19> CL_PERMUTATION = {
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
};

// Everything needed to emit a dynamic block: the code tables and the RLE'd
// header, plus the exact bit cost so `compress` can pick fixed vs. dynamic.
struct DynamicPlan {
    std::vector<std::uint8_t> ll_lengths;    // length HLIT (257..286)
    std::vector<std::uint8_t> dist_lengths;  // length HDIST (1..30)
    std::vector<std::pair<std::uint32_t, std::uint32_t>> ll_codes;    // full LL symbol -> code
    std::vector<std::pair<std::uint32_t, std::uint32_t>> dist_codes;  // full dist code -> code
    std::vector<std::uint8_t> cl_lengths;                              // 19 CL code lengths
    std::vector<std::pair<std::uint32_t, std::uint32_t>> cl_codes;    // canonical CL codes
    std::size_t cl_order_count = 0;  // HCLEN + 4 (# CL lengths transmitted)
    std::vector<ClItem> rle;         // RLE'd (LL ++ dist) code lengths
    std::uint64_t total_bits = 0;    // exact size of the whole block in bits
};

// Build a `DynamicPlan` for `tokens` (the LZSS token stream for one block).
inline DynamicPlan plan_dynamic(const std::vector<ca::lzss::Token>& tokens) {
    // ── 1. Frequencies ───────────────────────────────────────────────────
    // LL alphabet has 286 symbols (0..285); dist alphabet has 30 (0..29).
    std::vector<std::uint32_t> ll_freq(286, 0);
    std::vector<std::uint32_t> dist_freq(30, 0);
    ll_freq[256] = 1;  // end-of-block always appears exactly once
    for (const auto& tok : tokens) {
        if (!tok.is_match) {
            ll_freq[tok.literal] += 1;
        } else {
            ll_freq[length_symbol(tok.length)] += 1;
            dist_freq[dist_code_for(tok.offset)] += 1;
        }
    }

    // ── 2. Length-limited codes ──────────────────────────────────────────
    std::vector<std::uint8_t> ll_lengths_full = length_limited_huffman(ll_freq, 15);
    std::vector<std::uint8_t> dist_lengths_full = length_limited_huffman(dist_freq, 15);

    // RFC 1951 §3.2.7: HDIST is (#dist codes - 1), so there must be at least
    // one distance code even when the block has no matches. When no dist
    // code is present, emit a single dummy code of length 1 for symbol 0.
    bool any_dist = false;
    for (std::uint8_t l : dist_lengths_full) {
        if (l > 0) {
            any_dist = true;
            break;
        }
    }
    if (!any_dist) {
        dist_lengths_full[0] = 1;
    }

    // ── 3. Trim to HLIT / HDIST ──────────────────────────────────────────
    std::size_t hlit = 286;
    while (hlit > 257 && ll_lengths_full[hlit - 1] == 0) {
        --hlit;
    }
    std::size_t hdist = 30;
    while (hdist > 1 && dist_lengths_full[hdist - 1] == 0) {
        --hdist;
    }
    std::vector<std::uint8_t> ll_lengths(ll_lengths_full.begin(),
                                          ll_lengths_full.begin() + static_cast<std::ptrdiff_t>(hlit));
    std::vector<std::uint8_t> dist_lengths(
        dist_lengths_full.begin(), dist_lengths_full.begin() + static_cast<std::ptrdiff_t>(hdist));

    // ── 4. Canonical codes (over the FULL alphabet for easy indexing) ─────
    std::vector<std::pair<std::uint32_t, std::uint32_t>> ll_codes = build_canonical_codes(ll_lengths_full);
    std::vector<std::pair<std::uint32_t, std::uint32_t>> dist_codes =
        build_canonical_codes(dist_lengths_full);

    // ── 5. RLE the concatenated code-length sequence ────────────────────
    std::vector<std::uint8_t> combined = ll_lengths;
    combined.insert(combined.end(), dist_lengths.begin(), dist_lengths.end());
    std::vector<ClItem> rle = rle_code_lengths(combined);

    // ── 6. CL code (length-limited to 7 bits) ────────────────────────────
    std::vector<std::uint32_t> cl_freq(19, 0);
    for (const auto& it : rle) {
        cl_freq[it.sym] += 1;
    }
    std::vector<std::uint8_t> cl_lengths = length_limited_huffman(cl_freq, 7);
    std::vector<std::pair<std::uint32_t, std::uint32_t>> cl_codes = build_canonical_codes(cl_lengths);

    // HCLEN: number of CL lengths transmitted, in CL_PERMUTATION order. At
    // least 4 (the minimum HCLEN encodes); trimmed to the last nonzero
    // entry in permutation order.
    std::size_t cl_order_count = 19;
    while (cl_order_count > 4 && cl_lengths[CL_PERMUTATION[cl_order_count - 1]] == 0) {
        --cl_order_count;
    }

    // ── 7. Exact bit cost of the whole block ─────────────────────────────
    std::uint64_t total_bits = 3;      // BFINAL + BTYPE
    total_bits += 5 + 5 + 4;           // HLIT + HDIST + HCLEN fields
    total_bits += static_cast<std::uint64_t>(cl_order_count) * 3;  // CL lengths, 3 bits each
    for (const auto& it : rle) {
        total_bits += cl_lengths[it.sym];  // CL code
        total_bits += it.extra_bits;       // extra bits
    }
    for (const auto& tok : tokens) {
        if (!tok.is_match) {
            total_bits += ll_lengths_full[tok.literal];
        } else {
            std::uint16_t lsym = length_symbol(tok.length);
            total_bits += ll_lengths_full[lsym];
            total_bits += length_extra(lsym);
            std::uint16_t dc = dist_code_for(tok.offset);
            total_bits += dist_lengths_full[dc];
            total_bits += dist_extra(dc);
        }
    }
    total_bits += ll_lengths_full[256];  // EOB

    DynamicPlan plan;
    plan.ll_lengths = std::move(ll_lengths);
    plan.dist_lengths = std::move(dist_lengths);
    plan.ll_codes = std::move(ll_codes);
    plan.dist_codes = std::move(dist_codes);
    plan.cl_lengths = std::move(cl_lengths);
    plan.cl_codes = std::move(cl_codes);
    plan.cl_order_count = cl_order_count;
    plan.rle = std::move(rle);
    plan.total_bits = total_bits;
    return plan;
}

// Compute the exact bit cost of encoding `tokens` as a fixed-Huffman block.
inline std::uint64_t fixed_block_bits(const std::vector<ca::lzss::Token>& tokens) {
    std::uint64_t bits = 3;  // BFINAL + BTYPE
    for (const auto& tok : tokens) {
        if (!tok.is_match) {
            bits += fixed_ll_code(tok.literal).second;
        } else {
            std::uint16_t lsym = length_symbol(tok.length);
            bits += fixed_ll_code(lsym).second + length_extra(lsym);
            bits += 5 + dist_extra(dist_code_for(tok.offset));  // 5-bit dist code
        }
    }
    bits += fixed_ll_code(256).second;  // EOB
    return bits;
}

// Emit a single fixed-Huffman block (BFINAL=1) for `tokens` into `bw`.
inline void emit_fixed_block(BitWriter& bw, const std::vector<ca::lzss::Token>& tokens) {
    bw.write_raw_bits_lsb(1, 1);  // BFINAL = 1
    bw.write_raw_bits_lsb(1, 2);  // BTYPE  = 01 (fixed)
    for (const auto& tok : tokens) {
        if (!tok.is_match) {
            auto code = fixed_ll_code(tok.literal);
            bw.write_huffman(code.first, code.second);
        } else {
            std::uint16_t sym = length_symbol(tok.length);
            auto code = fixed_ll_code(sym);
            bw.write_huffman(code.first, code.second);
            bw.write_raw_bits_lsb(static_cast<std::uint32_t>(tok.length) - length_base(sym),
                                   length_extra(sym));
            std::uint16_t dc = dist_code_for(tok.offset);
            bw.write_huffman(dc, 5);
            bw.write_raw_bits_lsb(static_cast<std::uint32_t>(tok.offset) - dist_base(dc), dist_extra(dc));
        }
    }
    auto eob = fixed_ll_code(256);
    bw.write_huffman(eob.first, eob.second);
}

// Emit a single dynamic-Huffman block (BFINAL=1) for `tokens` into `bw`,
// using a pre-computed `DynamicPlan`.
inline void emit_dynamic_block(BitWriter& bw, const std::vector<ca::lzss::Token>& tokens,
                                const DynamicPlan& plan) {
    // ── Block header: BFINAL=1, BTYPE=10 ─────────────────────────────────
    bw.write_raw_bits_lsb(1, 1);  // BFINAL = 1
    bw.write_raw_bits_lsb(2, 2);  // BTYPE  = 10 (dynamic)

    // ── HLIT / HDIST / HCLEN (all LSB-first) ─────────────────────────────
    std::size_t hlit = plan.ll_lengths.size();     // 257..=286
    std::size_t hdist = plan.dist_lengths.size();  // 1..=30
    bw.write_raw_bits_lsb(static_cast<std::uint32_t>(hlit - 257), 5);
    bw.write_raw_bits_lsb(static_cast<std::uint32_t>(hdist - 1), 5);
    bw.write_raw_bits_lsb(static_cast<std::uint32_t>(plan.cl_order_count - 4), 4);

    // ── CL code lengths in permutation order, 3 bits each (LSB-first) ────
    for (std::size_t i = 0; i < plan.cl_order_count; ++i) {
        std::uint8_t l = plan.cl_lengths[CL_PERMUTATION[i]];
        bw.write_raw_bits_lsb(l, 3);
    }

    // ── RLE'd LL+dist code lengths: CL code MSB-first, extra LSB-first ────
    for (const auto& it : plan.rle) {
        auto code = plan.cl_codes[it.sym];
        bw.write_huffman(code.first, code.second);
        if (it.extra_bits > 0) {
            bw.write_raw_bits_lsb(it.extra_val, it.extra_bits);
        }
    }

    // ── Token stream: LL/dist codes MSB-first, extra bits LSB-first ──────
    for (const auto& tok : tokens) {
        if (!tok.is_match) {
            auto code = plan.ll_codes[tok.literal];
            bw.write_huffman(code.first, code.second);
        } else {
            std::uint16_t sym = length_symbol(tok.length);
            auto code = plan.ll_codes[sym];
            bw.write_huffman(code.first, code.second);
            bw.write_raw_bits_lsb(static_cast<std::uint32_t>(tok.length) - length_base(sym),
                                   length_extra(sym));
            std::uint16_t dc = dist_code_for(tok.offset);
            auto dcode = plan.dist_codes[dc];
            bw.write_huffman(dcode.first, dcode.second);
            bw.write_raw_bits_lsb(static_cast<std::uint32_t>(tok.offset) - dist_base(dc), dist_extra(dc));
        }
    }

    // ── End-of-block (symbol 256) ─────────────────────────────────────────
    auto eob = plan.ll_codes[256];
    bw.write_huffman(eob.first, eob.second);
}

// ===========================================================================
// Bit I/O — decoder side
// ===========================================================================
//
// DEFLATE packs bits into bytes LSB-first: the FIRST bit of a block header
// occupies bit 0 of the first byte. Huffman codes, however, are assigned
// MSB-first (canonical codes). So to decode a Huffman symbol we read bits
// one at a time from the stream and shift them into a code register from the
// left: `code = (code << 1) | next_stream_bit`. After k bits we have the
// same integer value the encoder wrote MSB-first.
//
// `BitReader` maintains a 64-bit lookahead buffer. `read_bits(n)` returns the
// next n bits LSB-first (bit 0 of the returned value = earliest bit in
// stream) — used for lengths, distances, and raw extra-bit fields. For
// Huffman decode we call `read_bits(1)` one bit at a time and accumulate
// MSB-first manually (see `decode_symbol`).
class BitReader {
public:
    explicit BitReader(const Bytes& data) : data_(data) {}

    // Refill `buf_` from `data_` so at least `n` bits are available. Throws
    // on truncated/malformed input rather than reading past the buffer.
    void refill(std::uint32_t n) {
        while (bits_in_buf_ < n) {
            if (byte_pos_ >= data_.size()) {
                throw DeflateException(DeflateError::UnexpectedEof);
            }
            buf_ |= (static_cast<std::uint64_t>(data_[byte_pos_]) << bits_in_buf_);
            ++byte_pos_;
            bits_in_buf_ += 8;
        }
    }

    // Read `n` bits LSB-first (bit 0 = earliest bit in stream). `n` is
    // always <= 16 at every call site (Huffman codes are read 1 bit at a
    // time; the widest extra-bits field is 13), so `buf_` never needs more
    // than 64 bits of headroom.
    std::uint32_t read_bits(std::uint32_t n) {
        if (n == 0) {
            return 0;
        }
        refill(n);
        std::uint32_t val = static_cast<std::uint32_t>(buf_ & ((std::uint64_t(1) << n) - 1));
        buf_ >>= n;
        bits_in_buf_ -= n;
        return val;
    }

    // Discard any partial bits remaining in the current byte so subsequent
    // reads are byte-aligned. Used after the 3-bit block header before a
    // stored block.
    void align_to_byte() {
        std::uint32_t leftover = bits_in_buf_ % 8;
        if (leftover != 0) {
            buf_ >>= leftover;
            bits_in_buf_ -= leftover;
        }
    }

    // Read one byte (must be byte-aligned; call `align_to_byte` first).
    std::uint8_t read_byte() {
        refill(8);
        std::uint8_t b = static_cast<std::uint8_t>(buf_ & 0xFFu);
        buf_ >>= 8;
        bits_in_buf_ -= 8;
        return b;
    }

    // Read a 16-bit little-endian value (two bytes).
    std::uint16_t read_u16_le() {
        std::uint16_t lo = read_byte();
        std::uint16_t hi = read_byte();
        return static_cast<std::uint16_t>(lo | static_cast<std::uint16_t>(hi << 8));
    }

private:
    const Bytes& data_;
    std::size_t byte_pos_ = 0;
    std::uint64_t buf_ = 0;
    std::uint32_t bits_in_buf_ = 0;
};

// ===========================================================================
// Canonical Huffman decoder
// ===========================================================================
//
// A canonical Huffman code is fully determined by the list of code lengths
// (one per symbol) — see "Canonical code assignment" above. The decode table
// maps (code, len) -> symbol. We pack (code, len) into one `uint32_t` key:
// `len` is at most 15 (4 bits) and `code` is at most 2^15-1 (15 bits), so
// `(len << 16) | code` is collision-free and fits comfortably.

using HuffTable = std::unordered_map<std::uint32_t, std::uint16_t>;

inline std::uint32_t huff_key(std::uint32_t code, std::uint32_t len) {
    return (len << 16) | code;
}

inline HuffTable build_huffman_decoder(const std::vector<std::uint8_t>& lengths) {
    HuffTable table;

    std::uint8_t max_len_u8 = 0;
    for (std::uint8_t l : lengths) {
        if (l > max_len_u8) {
            max_len_u8 = l;
        }
    }
    std::size_t max_len = max_len_u8;
    if (max_len == 0) {
        return table;
    }

    std::vector<std::uint32_t> bl_count(max_len + 1, 0);
    for (std::uint8_t l : lengths) {
        if (l > 0) {
            bl_count[l] += 1;
        }
    }

    std::vector<std::uint32_t> next_code(max_len + 2, 0);
    std::uint32_t code = 0;
    bl_count[0] = 0;
    for (std::size_t bits = 1; bits <= max_len; ++bits) {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }

    for (std::size_t sym = 0; sym < lengths.size(); ++sym) {
        std::uint8_t l = lengths[sym];
        if (l > 0) {
            std::size_t len = l;
            std::uint32_t c = next_code[len];
            table[huff_key(c, static_cast<std::uint32_t>(len))] = static_cast<std::uint16_t>(sym);
            next_code[len] += 1;
        }
    }
    return table;
}

// Decode one Huffman symbol using the given decode table. Throws
// `InvalidHuffmanCode` if no valid code is found within 15 bits (the RFC
// 1951 maximum).
inline std::uint16_t decode_symbol(const HuffTable& table, BitReader& reader) {
    std::uint32_t code = 0;
    for (std::uint32_t len = 1; len <= 15; ++len) {
        std::uint32_t bit = reader.read_bits(1);
        code = (code << 1) | bit;
        auto it = table.find(huff_key(code, len));
        if (it != table.end()) {
            return it->second;
        }
    }
    throw DeflateException(DeflateError::InvalidHuffmanCode);
}

// ===========================================================================
// Fixed Huffman code tables (RFC 1951 §3.2.6) — decoder side
// ===========================================================================

inline std::vector<std::uint8_t> fixed_ll_lengths() {
    std::vector<std::uint8_t> v(288, 0);
    for (std::size_t i = 0; i <= 143; ++i) {
        v[i] = 8;
    }
    for (std::size_t i = 144; i <= 255; ++i) {
        v[i] = 9;
    }
    for (std::size_t i = 256; i <= 279; ++i) {
        v[i] = 7;
    }
    for (std::size_t i = 280; i <= 287; ++i) {
        v[i] = 8;
    }
    return v;
}

inline std::vector<std::uint8_t> fixed_dist_lengths() {
    return std::vector<std::uint8_t>(32, 5);
}

// ===========================================================================
// Back-reference copy
// ===========================================================================

// Upper bound on decompressed output, guarding against "decompression
// bombs": tiny inputs that expand to enormous outputs (a highly compressible
// stream can reach ~1000:1, so a few KB of malicious `.xlsx`/`.gz` could
// otherwise exhaust memory). 256 MB comfortably exceeds any legitimate OOXML
// part while capping the blast radius of hostile input. Callers that
// legitimately need more should stream rather than inflate whole.
inline constexpr std::size_t MAX_INFLATE_OUTPUT = 256u * 1024u * 1024u;

// Copy `length` bytes from `dist` bytes behind the current end of `output`.
// Copies byte-by-byte (not a bulk memcpy) to correctly handle the
// "overlapping" case where dist < length, which encodes run-length
// sequences:
//
//   output = [A, B], dist=1, length=4
//     -> copies output[-1]=B, then the FRESH copy of output[-1]=B, etc.
//     -> result: [A, B, B, B, B, B]
//
// This is intentional in DEFLATE — it compresses runs cheaply.
//
// Validates `dist` against the bytes decoded so far (rejects 0 and
// out-of-range) and checks the growth against `MAX_INFLATE_OUTPUT` with an
// overflow-safe comparison BEFORE any allocation or copy.
inline void copy_back_ref(Bytes& output, std::size_t dist, std::size_t length) {
    std::size_t out_len = output.size();
    if (dist == 0 || dist > out_len) {
        throw DeflateException(DeflateError::BackReferenceOutOfRange);
    }
    if (out_len >= MAX_INFLATE_OUTPUT || length > MAX_INFLATE_OUTPUT - out_len) {
        throw DeflateException(DeflateError::OutputSizeExceeded);
    }
    std::size_t start = out_len - dist;
    output.reserve(out_len + length);
    for (std::size_t i = 0; i < length; ++i) {
        std::uint8_t b = output[start + i];
        output.push_back(b);
    }
}

// ===========================================================================
// Dynamic-Huffman block decoder (BTYPE=10) — code-length tree
// ===========================================================================
//
// Dynamic Huffman blocks transmit compressed code-length trees in the
// stream. The meta-tree (used to decode the LL and dist code lengths) is
// called the "code-length alphabet" (CL). Its wire order uses
// `CL_PERMUTATION` (defined above) so the most common lengths front-load.
//
// CL symbols:
//   0-15  -> literal code length
//   16    -> repeat the previous length x (3 + read_bits(2))
//   17    -> output zeros x (3 + read_bits(3))
//   18    -> output zeros x (11 + read_bits(7))
//
// `total` (= hlit + hdist) is bounded to at most 288 + 32 = 320 by the width
// of the HLIT/HDIST fields (5 bits each) regardless of adversarial input, so
// no separate size cap is needed here — but every push still checks against
// `total` to reject a code-length stream that overruns it.
inline std::vector<std::uint8_t> decode_code_lengths(const HuffTable& cl_table, BitReader& reader,
                                                       std::size_t total) {
    std::vector<std::uint8_t> lengths;
    lengths.reserve(total);
    std::uint8_t prev = 0;
    while (lengths.size() < total) {
        std::uint16_t sym = decode_symbol(cl_table, reader);
        if (sym <= 15) {
            prev = static_cast<std::uint8_t>(sym);
            lengths.push_back(prev);
        } else if (sym == 16) {
            std::uint32_t repeat = reader.read_bits(2) + 3;
            for (std::uint32_t r = 0; r < repeat; ++r) {
                if (lengths.size() >= total) {
                    throw DeflateException(DeflateError::CodeLengthOverflow);
                }
                lengths.push_back(prev);
            }
        } else if (sym == 17) {
            std::uint32_t repeat = reader.read_bits(3) + 3;
            for (std::uint32_t r = 0; r < repeat; ++r) {
                if (lengths.size() >= total) {
                    throw DeflateException(DeflateError::CodeLengthOverflow);
                }
                lengths.push_back(0);
            }
            prev = 0;
        } else if (sym == 18) {
            std::uint32_t repeat = reader.read_bits(7) + 11;
            for (std::uint32_t r = 0; r < repeat; ++r) {
                if (lengths.size() >= total) {
                    throw DeflateException(DeflateError::CodeLengthOverflow);
                }
                lengths.push_back(0);
            }
            prev = 0;
        } else {
            throw DeflateException(DeflateError::InvalidClSymbol);
        }
    }
    return lengths;
}

// ===========================================================================
// Shared decode loop (BTYPE=01 fixed and BTYPE=10 dynamic)
// ===========================================================================
//
// Once we have an LL decode table and a dist decode table (either the fixed
// tables or ones built from a dynamic header), the decode loop is identical:
//
//   loop:
//     sym <- decode LL symbol
//     if sym < 256  -> emit literal byte
//     if sym == 256 -> end-of-block
//     if sym >= 257 -> decode (length, dist) back-reference and copy
inline void decode_block(const HuffTable& ll_table, const HuffTable& dist_table, BitReader& reader,
                          Bytes& output) {
    for (;;) {
        std::uint16_t sym = decode_symbol(ll_table, reader);

        if (sym < 256) {
            if (output.size() >= MAX_INFLATE_OUTPUT) {
                throw DeflateException(DeflateError::OutputSizeExceeded);
            }
            output.push_back(static_cast<std::uint8_t>(sym));
        } else if (sym == 256) {
            return;
        } else {
            // Length/distance back-reference. `sym` is 257-285 (285 is the
            // "length 258, no extra bits" code some producers emit).
            std::size_t length_idx = static_cast<std::size_t>(sym - 257);
            if (length_idx >= LENGTH_TABLE.size()) {
                throw DeflateException(DeflateError::InvalidLengthSymbol);
            }
            const auto& entry = LENGTH_TABLE[length_idx];
            std::uint32_t extra_len = reader.read_bits(entry.extra_bits);
            std::size_t length = static_cast<std::size_t>(entry.base) + extra_len;

            std::uint16_t dist_sym = decode_symbol(dist_table, reader);
            if (dist_sym >= DIST_TABLE.size()) {
                throw DeflateException(DeflateError::InvalidDistanceSymbol);
            }
            const auto& dentry = DIST_TABLE[dist_sym];
            std::uint32_t extra_dist = reader.read_bits(dentry.extra_bits);
            std::size_t dist = static_cast<std::size_t>(dentry.base) + extra_dist;

            copy_back_ref(output, dist, length);
        }
    }
}

}  // namespace detail

// ===========================================================================
// Public API: compress
// ===========================================================================

// Compress `data` to a raw RFC 1951 DEFLATE bit-stream and return the bytes.
//
// Emits a SINGLE FINAL block, choosing per input between a fixed-Huffman
// block (BTYPE=01, pre-defined RFC 1951 §3.2.6 tables) and a dynamic-Huffman
// block (BTYPE=10, code lengths adapted to the data and transmitted inline)
// — whichever is smaller in exact emitted bits. Both are real, standard
// DEFLATE: the output is decodable by any conforming inflater — `inflate`
// here, and equally `zlib`, `gzip`, `unzip`, and web browsers.
//
// Dynamic Huffman usually wins on any input with skewed symbol frequencies
// (text, repetitive data): the fixed tables spend 8-9 bits on every literal,
// whereas a dynamic tree can give common bytes 2-4 bit codes. On tiny or
// near-incompressible inputs the dynamic HEADER (the transmitted code-length
// tree) costs more than it saves, so `compress` falls back to fixed. The
// choice is made by computing the exact bit length of each encoding and
// picking the minimum, so `compress` never produces a LARGER stream than
// fixed-only.
//
// Algorithm:
//   1. LZSS tokenization (window=32768, max_match=255, min_match=3) — the
//      full RFC 1951 window, so matches map into the length (3-255) and
//      distance (1-32768) tables.
//   2. Cost both a fixed and a dynamic encoding of the SAME token stream;
//      emit the cheaper as a single BFINAL=1 block, then the end-of-block
//      symbol (256).
//
// Never fails: returns a valid stream for every input, including `data`
// empty (-> the 2-byte fixed-Huffman block `03 00`).
inline Bytes compress(const Bytes& data) {
    std::vector<ca::lzss::Token> tokens = ca::lzss::encode(data, 32768, 255, 3);

    std::uint64_t fixed_bits = detail::fixed_block_bits(tokens);
    detail::DynamicPlan plan = detail::plan_dynamic(tokens);

    detail::BitWriter bw;
    if (plan.total_bits < fixed_bits) {
        detail::emit_dynamic_block(bw, tokens, plan);
    } else {
        detail::emit_fixed_block(bw, tokens);
    }
    return bw.finish();
}

// ===========================================================================
// Public API: inflate / decompress
// ===========================================================================

// Decompress a raw DEFLATE bit stream (RFC 1951) and return the original
// bytes. Supports all three block types:
//
//   BTYPE=00  stored (verbatim copy, no entropy coding)
//   BTYPE=01  fixed Huffman
//   BTYPE=10  dynamic Huffman
//
// Throws `DeflateException` on any malformed input — truncated streams,
// invalid Huffman codes, out-of-range length/distance symbols, out-of-range
// back-references, a corrupted stored-block LEN/NLEN pair, a reserved
// BTYPE(=11), or output exceeding `MAX_INFLATE_OUTPUT`.
//
// This is the decoder used to read this library's own `compress` output,
// but — because `compress` emits standard RFC 1951 — it equally reads
// `zlib`, `gzip`, and Microsoft Office (OOXML) DEFLATE streams.
inline Bytes inflate(const Bytes& data) {
    detail::BitReader reader(data);
    Bytes output;

    for (;;) {
        // ── Block header ─────────────────────────────────────────────────
        // Each block begins with 3 bits:
        //   bit 0    BFINAL — set if this is the last block
        //   bits 1-2 BTYPE  — 00=stored, 01=fixed Huffman, 10=dynamic Huffman
        std::uint32_t bfinal = reader.read_bits(1);
        std::uint32_t btype = reader.read_bits(2);

        switch (btype) {
            // ── BTYPE=00: Stored block ──────────────────────────────────
            //
            // The encoder writes the header bits into a partial byte, pads
            // to the next byte boundary, then:
            //   LEN  (2 bytes LE) — byte count of the literal data
            //   NLEN (2 bytes LE) — one's complement of LEN
            //   [LEN bytes of literal data]
            case 0b00: {
                reader.align_to_byte();
                std::uint16_t len = reader.read_u16_le();
                std::uint16_t nlen = reader.read_u16_le();
                if (static_cast<std::uint16_t>(~len) != nlen) {
                    throw DeflateException(DeflateError::StoredBlockLenMismatch);
                }
                if (output.size() >= detail::MAX_INFLATE_OUTPUT ||
                    static_cast<std::size_t>(len) > detail::MAX_INFLATE_OUTPUT - output.size()) {
                    throw DeflateException(DeflateError::OutputSizeExceeded);
                }
                output.reserve(output.size() + len);
                for (std::uint32_t i = 0; i < len; ++i) {
                    output.push_back(reader.read_byte());
                }
                break;
            }

            // ── BTYPE=01: Fixed Huffman ─────────────────────────────────
            //
            // Uses the pre-agreed code tables from RFC 1951 §3.2.6. No table
            // is transmitted; we reconstruct from the known lengths.
            case 0b01: {
                detail::HuffTable ll_table = detail::build_huffman_decoder(detail::fixed_ll_lengths());
                detail::HuffTable dist_table =
                    detail::build_huffman_decoder(detail::fixed_dist_lengths());
                detail::decode_block(ll_table, dist_table, reader, output);
                break;
            }

            // ── BTYPE=10: Dynamic Huffman ───────────────────────────────
            //
            // The block header encodes three counts, then the CL (meta)
            // tree, then the LL and dist code lengths encoded with the CL
            // tree.
            //
            //   hlit  = read_bits(5) + 257   -> number of LL lengths
            //   hdist = read_bits(5) + 1     -> number of dist lengths
            //   hclen = read_bits(4) + 4     -> number of CL lengths
            case 0b10: {
                std::size_t hlit = static_cast<std::size_t>(reader.read_bits(5)) + 257;
                std::size_t hdist = static_cast<std::size_t>(reader.read_bits(5)) + 1;
                std::size_t hclen = static_cast<std::size_t>(reader.read_bits(4)) + 4;

                std::vector<std::uint8_t> cl_lengths(19, 0);
                for (std::size_t i = 0; i < hclen; ++i) {
                    cl_lengths[detail::CL_PERMUTATION[i]] = static_cast<std::uint8_t>(reader.read_bits(3));
                }

                detail::HuffTable cl_table = detail::build_huffman_decoder(cl_lengths);

                std::vector<std::uint8_t> all_lengths =
                    detail::decode_code_lengths(cl_table, reader, hlit + hdist);
                std::vector<std::uint8_t> ll_lengths(
                    all_lengths.begin(), all_lengths.begin() + static_cast<std::ptrdiff_t>(hlit));
                std::vector<std::uint8_t> dist_lengths(
                    all_lengths.begin() + static_cast<std::ptrdiff_t>(hlit), all_lengths.end());

                detail::HuffTable ll_table = detail::build_huffman_decoder(ll_lengths);
                detail::HuffTable dist_table = detail::build_huffman_decoder(dist_lengths);
                detail::decode_block(ll_table, dist_table, reader, output);
                break;
            }

            // ── BTYPE=11: reserved — always malformed ───────────────────
            default: {
                throw DeflateException(DeflateError::ReservedBlockType);
            }
        }

        if (bfinal == 1) {
            break;
        }
    }

    return output;
}

// Alias for `inflate`, matching the sibling ports' `compress`/`decompress`
// naming symmetry: `decompress(compress(x)) == x` for all `x`.
inline Bytes decompress(const Bytes& data) { return inflate(data); }

}  // namespace deflate
}  // namespace ca

#endif  // CA_DEFLATE_HPP
