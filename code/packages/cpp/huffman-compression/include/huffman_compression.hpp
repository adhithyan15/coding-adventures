// huffman_compression.hpp — Huffman compression, in pure ISO C++17
// (header-only). A faithful port of the Rust `huffman-compression` crate (CMP04).
// ===========================================================================
//
// Canonical Huffman coding: codes are determined by per-symbol lengths, so the
// stream carries only a lengths table. Wire format (big-endian header):
//   [0..4] original length, [4..8] symbol count N,
//   [8..8+2N] lengths table (N × [symbol, length], sorted by (length, symbol)),
//   [8+2N..] bit stream (canonical codes, packed LSB-first).
//
// The Huffman tree is built in a std::vector (no recursion into owned pointers).
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef HUFFMAN_COMPRESSION_HPP
#define HUFFMAN_COMPRESSION_HPP

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <queue>
#include <stdexcept>
#include <vector>

namespace ca {
namespace huffman {

// Longest code length either side of the wire accepts.
//
// WHY 56 (and why the encoder never needs length-limiting):
//   A Huffman tree of depth d needs a total weight of at least Fibonacci(d + 2)
//   (the worst case is the "Fibonacci" frequency distribution 1,1,2,3,5,8,...).
//   The CMP04 header stores the input length in a u32, so the total weight is at
//   most 2^32 - 1, and Fibonacci(48) = 4,807,526,976 > 2^32 caps the depth at
//   46. 46 <= 56, so with size() <= UINT32_MAX every code the encoder builds fits
//   in the 64-bit code accumulator with 8+ bits to spare, and the decoder's
//   1..56 limit can never reject encoder output. (A depth of 33..46 DOES occur,
//   for ~15 MB of Fibonacci-distributed bytes, and overflowed the previous
//   32-bit code values.) No package-merge or zlib-style length cap is needed,
//   and because nothing is capped the canonical codes (hence the output bytes)
//   are unchanged for every input the old encoder handled correctly.
constexpr unsigned kMaxCodeLen = 56;

// Symbols are single bytes, so a valid table has at most 256 entries.
constexpr std::uint64_t kMaxSymbols = 256;

namespace detail {

struct hnode {
    std::uint32_t weight;
    int left;
    int right;
    int symbol; // 0..255 leaf, -1 internal
    int order;
};

struct sym_len {
    std::uint16_t symbol;
    std::uint8_t length;
};

// Compute each present symbol's code length (0 if absent).
inline std::array<std::uint8_t, 256>
compute_code_lengths(const std::array<std::uint32_t, 256> &freq) {
    std::array<std::uint8_t, 256> code_len{};
    std::vector<hnode> nodes;
    nodes.reserve(512);
    int order = 0;

    // Min-heap of node indices by (weight, order).
    auto cmp = [&nodes](int a, int b) {
        if (nodes[a].weight != nodes[b].weight) {
            return nodes[a].weight > nodes[b].weight; // priority_queue is max → invert
        }
        return nodes[a].order > nodes[b].order;
    };
    std::priority_queue<int, std::vector<int>, decltype(cmp)> heap(cmp);

    int distinct = 0, first_leaf = -1;
    for (int sym = 0; sym < 256; sym++) {
        if (freq[sym] > 0) {
            nodes.push_back(hnode{freq[sym], -1, -1, sym, order++});
            heap.push(static_cast<int>(nodes.size() - 1));
            if (first_leaf < 0) {
                first_leaf = static_cast<int>(nodes.size() - 1);
            }
            distinct++;
        }
    }
    if (distinct == 0) {
        return code_len;
    }
    if (distinct == 1) {
        code_len[static_cast<std::size_t>(nodes[first_leaf].symbol)] = 1;
        return code_len;
    }
    while (heap.size() > 1) {
        int a = heap.top();
        heap.pop();
        int b = heap.top();
        heap.pop();
        nodes.push_back(hnode{nodes[a].weight + nodes[b].weight, a, b, -1, order++});
        heap.push(static_cast<int>(nodes.size() - 1));
    }
    // DFS depths.
    std::vector<std::pair<int, int>> stack;
    stack.emplace_back(heap.top(), 0);
    while (!stack.empty()) {
        auto [node, depth] = stack.back();
        stack.pop_back();
        if (nodes[node].symbol >= 0) {
            code_len[static_cast<std::size_t>(nodes[node].symbol)] =
                static_cast<std::uint8_t>(depth == 0 ? 1 : depth);
        } else {
            stack.emplace_back(nodes[node].left, depth + 1);
            stack.emplace_back(nodes[node].right, depth + 1);
        }
    }
    return code_len;
}

// Canonical (DEFLATE-style) codes for a table sorted by (length, symbol).
//
// Codes are 64-bit: a code longer than 32 bits does not fit in a uint32_t, and
// shifting a 32-bit value by >= 32 is undefined behaviour. Every length is in
// 1..kMaxCodeLen, so the shift distance is at most 55 and codes stay below 2^56.
inline std::vector<std::uint64_t> canonical_codes(const std::vector<sym_len> &table) {
    std::vector<std::uint64_t> codes(table.size(), 0);
    std::uint64_t code = 0;
    for (std::size_t i = 1; i < table.size(); i++) {
        code = (code + 1) << (table[i].length - table[i - 1].length);
        codes[i] = code;
    }
    return codes;
}

} // namespace detail

// Throws std::length_error if the input exceeds UINT32_MAX bytes (the header's
// length field and the frequency counters are 32-bit; truncating would also void
// the depth bound documented at kMaxCodeLen).
inline std::vector<std::uint8_t> compress(const std::vector<std::uint8_t> &data) {
    using namespace detail;
    std::vector<std::uint8_t> out;
    if (static_cast<std::uint64_t>(data.size()) > 0xFFFFFFFFull) {
        throw std::length_error("huffman: input larger than 4 GiB");
    }
    std::uint32_t original_length = static_cast<std::uint32_t>(data.size());
    if (data.empty()) {
        out.assign(8, 0);
        return out;
    }
    std::array<std::uint32_t, 256> freq{};
    for (std::uint8_t b : data) {
        freq[b]++;
    }
    auto code_len = compute_code_lengths(freq);

    std::vector<sym_len> table;
    for (int i = 0; i < 256; i++) {
        // Defence in depth: unreachable for size() <= UINT32_MAX (depth <= 46),
        // but never emit a table the decoder would reject.
        if (code_len[static_cast<std::size_t>(i)] > kMaxCodeLen) {
            throw std::length_error("huffman: code length exceeds 56 bits");
        }
        if (code_len[i] > 0) {
            table.push_back(sym_len{static_cast<std::uint16_t>(i), code_len[i]});
        }
    }
    std::sort(table.begin(), table.end(), [](const sym_len &a, const sym_len &b) {
        return a.length != b.length ? a.length < b.length : a.symbol < b.symbol;
    });
    auto codes = canonical_codes(table);

    std::array<std::uint64_t, 256> sym_code{};
    std::array<std::uint8_t, 256> sym_len_by_byte{};
    for (std::size_t i = 0; i < table.size(); i++) {
        sym_code[table[i].symbol] = codes[i];
        sym_len_by_byte[table[i].symbol] = table[i].length;
    }

    std::vector<std::uint8_t> bits;
    unsigned bit_acc = 0, bit_cnt = 0;
    for (std::uint8_t b : data) {
        std::uint8_t clen = sym_len_by_byte[b];
        std::uint64_t ccode = sym_code[b];
        for (int j = clen - 1; j >= 0; j--) {
            unsigned bit = static_cast<unsigned>((ccode >> j) & 1u);
            bit_acc |= bit << bit_cnt;
            if (++bit_cnt == 8) {
                bits.push_back(static_cast<std::uint8_t>(bit_acc));
                bit_acc = 0;
                bit_cnt = 0;
            }
        }
    }
    if (bit_cnt > 0) {
        bits.push_back(static_cast<std::uint8_t>(bit_acc));
    }

    std::uint32_t symbol_count = static_cast<std::uint32_t>(table.size());
    out.reserve(8 + 2 * table.size() + bits.size());
    for (int s = 24; s >= 0; s -= 8) {
        out.push_back(static_cast<std::uint8_t>(original_length >> s));
    }
    for (int s = 24; s >= 0; s -= 8) {
        out.push_back(static_cast<std::uint8_t>(symbol_count >> s));
    }
    for (const sym_len &e : table) {
        out.push_back(static_cast<std::uint8_t>(e.symbol));
        out.push_back(e.length);
    }
    out.insert(out.end(), bits.begin(), bits.end());
    return out;
}

// Throws std::invalid_argument on a malformed or hostile stream.
inline std::vector<std::uint8_t> decompress(const std::vector<std::uint8_t> &data) {
    using namespace detail;
    if (data.size() < 8) {
        throw std::invalid_argument("huffman: input too short (header)");
    }
    const std::uint64_t original_length = (static_cast<std::uint64_t>(data[0]) << 24) |
                                          (static_cast<std::uint64_t>(data[1]) << 16) |
                                          (static_cast<std::uint64_t>(data[2]) << 8) | data[3];
    const std::uint64_t symbol_count = (static_cast<std::uint64_t>(data[4]) << 24) |
                                       (static_cast<std::uint64_t>(data[5]) << 16) |
                                       (static_cast<std::uint64_t>(data[6]) << 8) | data[7];
    if (original_length == 0) {
        return {};
    }
    // SECURITY: the header is attacker-controlled. Symbols are single bytes, so a
    // table can name at most 256 of them; checking first also bounds the
    // table-size arithmetic below.
    if (symbol_count == 0 || symbol_count > kMaxSymbols) {
        throw std::invalid_argument("huffman: bad symbol count");
    }
    const std::size_t count = static_cast<std::size_t>(symbol_count);
    const std::size_t table_end = 8 + 2 * count;
    if (data.size() < table_end) {
        throw std::invalid_argument("huffman: truncated table");
    }

    // Rebuild canonical codes (DEFLATE style):
    //     code = 0
    //     for each (symbol, length) in sorted order:
    //         code <<= (length - previous length)
    //         assign `code`; code += 1
    //
    // SECURITY: the table is hostile input, so each entry is checked before use:
    //   - a repeated symbol makes the table ambiguous;
    //   - a length of 0 matches nothing, and one above kMaxCodeLen would overflow
    //     the 64-bit shift;
    //   - a table not sorted by length breaks the "codes of a length are
    //     consecutive" property (the encoder always sorts);
    //   - an over-subscribed table (Kraft sum > 1) is detected by requiring the
    //     running code to still fit in `length` bits (code >> length == 0).
    //
    // Codes of one length are consecutive integers starting at first_code[L]; the
    // symbol for code c is syms[base[L] + (c - first_code[L])]. That makes each
    // probe O(1) instead of scanning every symbol per bit.
    std::array<std::uint8_t, 256> syms{};
    std::array<bool, 256> seen{};
    std::array<std::uint64_t, kMaxCodeLen + 1> first_code{};
    std::array<std::size_t, kMaxCodeLen + 1> counts{};
    std::array<std::size_t, kMaxCodeLen + 1> base{};
    std::uint64_t code = 0;
    unsigned prev_len = 0;
    unsigned max_len = 0;
    for (std::size_t i = 0; i < count; i++) {
        const std::uint8_t symbol = data[8 + 2 * i];
        const unsigned length = data[8 + 2 * i + 1];
        if (seen[symbol]) {
            throw std::invalid_argument("huffman: duplicate symbol in table");
        }
        seen[symbol] = true;
        if (length == 0 || length > kMaxCodeLen) {
            throw std::invalid_argument("huffman: bad code length");
        }
        if (i > 0 && length < prev_len) {
            throw std::invalid_argument("huffman: table not sorted by length");
        }
        if (i > 0) {
            code <<= (length - prev_len);
        }
        if ((code >> length) != 0) {
            throw std::invalid_argument("huffman: table over-subscribes the code space");
        }
        if (counts[length] == 0) {
            first_code[length] = code;
            base[length] = i;
        }
        counts[length]++;
        syms[i] = symbol;
        code++;
        prev_len = length;
        max_len = length;
    }

    // SECURITY: every code is at least one bit, so N payload bytes hold at most
    // 8*N symbols. Reject a larger declared length BEFORE allocating anything
    // sized from it. Written as ceil(original/8) > bytes to avoid overflow.
    const std::size_t payload_bytes = data.size() - table_end;
    if ((original_length + 7) / 8 > static_cast<std::uint64_t>(payload_bytes)) {
        throw std::invalid_argument("huffman: declared length exceeds the payload");
    }

    std::vector<std::uint8_t> output;
    output.reserve(static_cast<std::size_t>(original_length));
    std::uint64_t cur = 0;
    unsigned cur_len = 0;
    const std::size_t total_bits = payload_bytes * 8;
    std::size_t bit_pos = 0;
    // Read the LSB-first bit stream, accumulating a code MSB-first. If the
    // accumulated length reaches the longest code without a match, no later bit
    // can ever match (an incomplete table fed unmatched bits): fail immediately.
    while (output.size() < original_length) {
        if (bit_pos >= total_bits) {
            throw std::invalid_argument("huffman: truncated bit stream");
        }
        const unsigned bit = (static_cast<unsigned>(data[table_end + bit_pos / 8]) >> (bit_pos % 8)) & 1u;
        bit_pos++;
        cur = (cur << 1) | bit;
        cur_len++;
        if (counts[cur_len] != 0 && cur >= first_code[cur_len] &&
            cur - first_code[cur_len] < counts[cur_len]) {
            output.push_back(syms[base[cur_len] + static_cast<std::size_t>(cur - first_code[cur_len])]);
            cur = 0;
            cur_len = 0;
        } else if (cur_len >= max_len) {
            throw std::invalid_argument("huffman: malformed code stream");
        }
    }
    return output;
}

} // namespace huffman
} // namespace ca

#endif // HUFFMAN_COMPRESSION_HPP
