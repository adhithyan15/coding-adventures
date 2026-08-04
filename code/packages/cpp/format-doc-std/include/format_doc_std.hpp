// format_doc_std.hpp — reusable pretty-printing templates over format-doc.
// ========================================================================
//
// A faithful, header-only port of the Rust `format-doc-std` crate (namespace
// `ca::format_doc_std`) — the "80% layer" of the formatter stack. `format-doc`
// owns the primitive document algebra; this layer owns the common syntax shapes
// most languages reuse. Every template returns a `format_doc::Doc`; the layout
// later decides flat-vs-broken.
//
//   delimited_list  arrays / tuples / parameter & argument lists / fields
//   call_like       function & constructor calls (callee + delimited args)
//   block_like      braces / begin…end / indented block bodies
//   infix_chain     arithmetic / boolean / pipeline / type-operator chains
//
// `Doc` is cheaply copyable (structural sharing), so configs simply hold `Doc`
// values. Rust's `assert!`/panic on an infix arity mismatch becomes a thrown
// `std::invalid_argument`.
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no compiler extensions.
#ifndef FORMAT_DOC_STD_HPP
#define FORMAT_DOC_STD_HPP

#include <cstddef>
#include <stdexcept>
#include <utility>
#include <vector>

#include "format_doc.hpp"

namespace ca::format_doc_std {

using format_doc::Doc;

// Package version, mirrored in tests as a smoke check.
inline constexpr const char* kVersion = "0.1.0";

// Whether a delimited list emits a trailing separator.
enum class TrailingSeparator { Never, Always, IfBreak };

namespace detail {
inline Doc trailing_doc(const Doc& separator, TrailingSeparator t) {
    switch (t) {
        case TrailingSeparator::Always:
            return separator;
        case TrailingSeparator::IfBreak:
            return format_doc::if_break(separator, format_doc::nil());
        case TrailingSeparator::Never:
        default:
            return format_doc::nil();
    }
}
}  // namespace detail

// ── delimited_list ───────────────────────────────────────────────────────────

struct DelimitedListConfig {
    Doc separator = format_doc::text(",");
    TrailingSeparator trailing_separator = TrailingSeparator::Never;
    bool empty_spacing = false;
};

inline Doc delimited_list_with(Doc open, std::vector<Doc> items, Doc close,
                               const DelimitedListConfig& config) {
    using namespace format_doc;
    if (items.empty())
        return concat({std::move(open),
                       config.empty_spacing ? text(" ") : nil(),
                       std::move(close)});
    Doc body = join(concat({config.separator, line()}), std::move(items));
    Doc trailing = detail::trailing_doc(config.separator,
                                        config.trailing_separator);
    return group(concat({std::move(open),
                         indent(concat({softline(), std::move(body),
                                        std::move(trailing)}),
                                1),
                         softline(), std::move(close)}));
}

inline Doc delimited_list(Doc open, std::vector<Doc> items, Doc close) {
    return delimited_list_with(std::move(open), std::move(items),
                               std::move(close), DelimitedListConfig{});
}

// ── call_like ────────────────────────────────────────────────────────────────

struct CallLikeConfig {
    Doc open = format_doc::text("(");
    Doc close = format_doc::text(")");
    Doc separator = format_doc::text(",");
    TrailingSeparator trailing_separator = TrailingSeparator::Never;
};

inline Doc call_like(Doc callee, std::vector<Doc> args,
                     const CallLikeConfig& config) {
    DelimitedListConfig list_config;
    list_config.separator = config.separator;
    list_config.trailing_separator = config.trailing_separator;
    list_config.empty_spacing = false;
    return format_doc::concat(
        {std::move(callee),
         delimited_list_with(config.open, std::move(args), config.close,
                             list_config)});
}

// ── block_like ───────────────────────────────────────────────────────────────

struct BlockLikeConfig {
    bool empty_spacing = true;
};

inline Doc block_like_with(Doc open, Doc body, Doc close,
                           const BlockLikeConfig& config) {
    using namespace format_doc;
    if (is_nil(body))
        return concat({std::move(open),
                       config.empty_spacing ? text(" ") : nil(),
                       std::move(close)});
    return group(concat({std::move(open),
                         indent(concat({line(), std::move(body)}), 1), line(),
                         std::move(close)}));
}

inline Doc block_like(Doc open, Doc body, Doc close) {
    return block_like_with(std::move(open), std::move(body), std::move(close),
                           BlockLikeConfig{});
}

// ── infix_chain ──────────────────────────────────────────────────────────────

struct InfixChainConfig {
    bool break_before_operators = false;
};

inline Doc infix_chain(std::vector<Doc> operands, std::vector<Doc> operators,
                       const InfixChainConfig& config) {
    using namespace format_doc;
    if (operands.empty()) return nil();
    if (operators.size() != operands.size() - 1)
        throw std::invalid_argument(
            "infix_chain: operators.len() must equal operands.len() - 1");
    if (operands.size() == 1) return operands[0];

    std::vector<Doc> rest;
    rest.reserve(operators.size() * 4);
    for (std::size_t i = 0; i < operators.size(); ++i) {
        if (config.break_before_operators) {
            // <line><op><space><operand>
            rest.push_back(line());
            rest.push_back(operators[i]);
            rest.push_back(text(" "));
            rest.push_back(operands[i + 1]);
        } else {
            // <space><op><line><operand>
            rest.push_back(text(" "));
            rest.push_back(operators[i]);
            rest.push_back(line());
            rest.push_back(operands[i + 1]);
        }
    }
    return group(concat({operands[0], indent(concat(std::move(rest)), 1)}));
}

}  // namespace ca::format_doc_std

#endif  // FORMAT_DOC_STD_HPP
