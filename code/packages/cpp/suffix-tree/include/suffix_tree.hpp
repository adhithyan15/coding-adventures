// suffix_tree.hpp — a suffix index over a string, in pure ISO C++17
// (header-only). A faithful port of the Rust `suffix-tree` crate (DT15), in
// namespace `ca`.
// ===========================================================================
//
// The reference crate keeps a deliberately simple structure — a root whose
// children are one leaf per suffix start — and answers substring queries with
// direct scans over the stored text. This port mirrors that: `ca::suffix_tree`
// owns the text and the queries operate on it.
//
//   search / count_occurrences         — locate a pattern
//   longest_repeated_substring         — longest substring occurring twice
//   all_suffixes                       — the text's suffixes
//   node_count                         — 1 (root) + one leaf per character
//   ca::longest_common_substring(a, b) — LCS of two strings (a free function)
//
// The crate counts Unicode scalar values; this port (like std::string) works on
// bytes, so results match for ASCII / single-byte text. Offsets are byte
// offsets.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef SUFFIX_TREE_HPP
#define SUFFIX_TREE_HPP

#include <cstddef>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace ca {

class suffix_tree {
public:
    explicit suffix_tree(std::string text) : text_(std::move(text)) {}

    static suffix_tree build(std::string s) { return suffix_tree(std::move(s)); }
    // The crate's build_ukkonen is an alias for the simple build.
    static suffix_tree build_ukkonen(std::string s) {
        return build(std::move(s));
    }

    // Every start offset where `pattern` occurs. An empty pattern matches at
    // every position 0..=size(), mirroring the crate.
    std::vector<std::size_t> search(std::string_view pattern) const {
        std::vector<std::size_t> out;
        std::size_t n = text_.size();
        std::size_t m = pattern.size();
        if (m == 0) {
            for (std::size_t i = 0; i <= n; i++) {
                out.push_back(i);
            }
            return out;
        }
        if (m > n) {
            return out;
        }
        for (std::size_t start = 0; start <= n - m; start++) {
            if (std::string_view(text_).substr(start, m) == pattern) {
                out.push_back(start);
            }
        }
        return out;
    }

    std::size_t count_occurrences(std::string_view pattern) const {
        return search(pattern).size();
    }

    // Longest substring that occurs at least twice (earliest on ties).
    std::string longest_repeated_substring() const {
        std::size_t n = text_.size();
        std::size_t best_len = 0;
        std::size_t best_start = 0;
        for (std::size_t i = 0; i < n; i++) {
            for (std::size_t j = i + 1; j < n; j++) {
                std::size_t k = 0;
                while (j + k < n && text_[i + k] == text_[j + k]) {
                    k++;
                }
                if (k > best_len) {
                    best_len = k;
                    best_start = i;
                }
            }
        }
        return text_.substr(best_start, best_len);
    }

    std::vector<std::string> all_suffixes() const {
        std::vector<std::string> out;
        out.reserve(text_.size());
        for (std::size_t i = 0; i < text_.size(); i++) {
            out.push_back(text_.substr(i));
        }
        return out;
    }

    std::size_t node_count() const { return 1 + text_.size(); }
    std::size_t text_len() const { return text_.size(); }
    const std::string &text() const { return text_; }

private:
    std::string text_;
};

// Longest substring common to `a` and `b`, via rolling dynamic programming.
inline std::string longest_common_substring(std::string_view a,
                                            std::string_view b) {
    if (a.empty() || b.empty()) {
        return std::string();
    }
    std::vector<std::size_t> prev(b.size() + 1, 0);
    std::vector<std::size_t> cur(b.size() + 1, 0);
    std::size_t best_len = 0;
    std::size_t best_end = 0; // 1-based end index in `a`
    for (std::size_t i = 1; i <= a.size(); i++) {
        for (std::size_t j = 1; j <= b.size(); j++) {
            if (a[i - 1] == b[j - 1]) {
                cur[j] = prev[j - 1] + 1;
                if (cur[j] > best_len) {
                    best_len = cur[j];
                    best_end = i;
                }
            } else {
                cur[j] = 0;
            }
        }
        std::swap(prev, cur); // prev now holds row i; cur overwritten next row
    }
    return std::string(a.substr(best_end - best_len, best_len));
}

} // namespace ca

#endif // SUFFIX_TREE_HPP
