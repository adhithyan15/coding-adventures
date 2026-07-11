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
inline std::vector<std::uint32_t> canonical_codes(const std::vector<sym_len> &table) {
    std::vector<std::uint32_t> codes(table.size(), 0);
    std::uint32_t code = 0;
    for (std::size_t i = 1; i < table.size(); i++) {
        code = (code + 1) << (table[i].length - table[i - 1].length);
        codes[i] = code;
    }
    return codes;
}

} // namespace detail

inline std::vector<std::uint8_t> compress(const std::vector<std::uint8_t> &data) {
    using namespace detail;
    std::vector<std::uint8_t> out;
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
        if (code_len[i] > 0) {
            table.push_back(sym_len{static_cast<std::uint16_t>(i), code_len[i]});
        }
    }
    std::sort(table.begin(), table.end(), [](const sym_len &a, const sym_len &b) {
        return a.length != b.length ? a.length < b.length : a.symbol < b.symbol;
    });
    auto codes = canonical_codes(table);

    std::array<std::uint32_t, 256> sym_code{};
    std::array<std::uint8_t, 256> sym_len_by_byte{};
    for (std::size_t i = 0; i < table.size(); i++) {
        sym_code[table[i].symbol] = codes[i];
        sym_len_by_byte[table[i].symbol] = table[i].length;
    }

    std::vector<std::uint8_t> bits;
    unsigned bit_acc = 0, bit_cnt = 0;
    for (std::uint8_t b : data) {
        std::uint8_t clen = sym_len_by_byte[b];
        std::uint32_t ccode = sym_code[b];
        for (int j = clen - 1; j >= 0; j--) {
            unsigned bit = (ccode >> j) & 1u;
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

// Throws std::invalid_argument on a malformed stream.
inline std::vector<std::uint8_t> decompress(const std::vector<std::uint8_t> &data) {
    using namespace detail;
    if (data.size() < 8) {
        throw std::invalid_argument("huffman: input too short (header)");
    }
    std::size_t original_length = (static_cast<std::size_t>(data[0]) << 24) |
                                  (static_cast<std::size_t>(data[1]) << 16) |
                                  (static_cast<std::size_t>(data[2]) << 8) | data[3];
    std::size_t symbol_count = (static_cast<std::size_t>(data[4]) << 24) |
                               (static_cast<std::size_t>(data[5]) << 16) |
                               (static_cast<std::size_t>(data[6]) << 8) | data[7];
    if (original_length == 0) {
        return {};
    }
    if (symbol_count == 0 || symbol_count > 256) {
        throw std::invalid_argument("huffman: bad symbol count");
    }
    std::size_t table_end = 8 + 2 * symbol_count;
    if (data.size() < table_end) {
        throw std::invalid_argument("huffman: truncated table");
    }
    std::vector<sym_len> table(symbol_count);
    for (std::size_t i = 0; i < symbol_count; i++) {
        table[i].symbol = data[8 + 2 * i];
        table[i].length = data[8 + 2 * i + 1];
        if (table[i].length == 0 || table[i].length > 32) {
            throw std::invalid_argument("huffman: bad code length");
        }
    }
    auto codes = canonical_codes(table);

    std::vector<std::uint8_t> output;
    std::uint32_t cur = 0;
    unsigned cur_len = 0;
    std::size_t produced = 0;
    std::size_t total_bits = (data.size() - table_end) * 8;
    std::size_t bit_pos = 0;
    while (produced < original_length && bit_pos < total_bits) {
        std::size_t byte_index = table_end + bit_pos / 8;
        unsigned bit = (data[byte_index] >> (bit_pos % 8)) & 1u;
        bit_pos++;
        cur = (cur << 1) | bit;
        cur_len++;
        for (std::size_t i = 0; i < symbol_count; i++) {
            if (table[i].length == cur_len && codes[i] == cur) {
                output.push_back(static_cast<std::uint8_t>(table[i].symbol));
                produced++;
                cur = 0;
                cur_len = 0;
                break;
            }
        }
        if (cur_len > 32) {
            throw std::invalid_argument("huffman: malformed code stream");
        }
    }
    if (produced != original_length) {
        throw std::invalid_argument("huffman: truncated bit stream");
    }
    return output;
}

} // namespace huffman
} // namespace ca

#endif // HUFFMAN_COMPRESSION_HPP
