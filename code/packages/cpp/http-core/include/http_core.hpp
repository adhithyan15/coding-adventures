// http_core.hpp — shared HTTP message types and helpers, in pure ISO C++17,
// header-only, in namespace ca::http. A faithful port of the Rust `http-core`
// crate.
// ===========================================================================
//
// Version-specific parsers disagree about wire syntax, but they should agree
// about the semantic shapes application code consumes. This crate provides those
// shared shapes — headers, versions, request/response heads, body-framing hints
// — plus the syntax-level helpers that read them: route-pattern matching,
// request-target splitting, query-pair iteration, and Content-* parsing.
//
// SCOPE. This is a syntax-level core: query values are NOT percent-decoded, so a
// caller can apply its own decoding policy.
//
// DIVERGENCE. `HttpVersion::parse` returns `std::optional` (std::nullopt on a
// malformed marker) where the Rust version returns `Result<_, String>` — the
// semantic outcome (a version or none) is identical; only the error text is
// dropped. Query pairs are returned as an owned `std::vector<std::pair<…>>`
// rather than a lazy iterator; the sequence is the same.
//
// PORTABILITY. Pure ISO C++17 — standard library only. Compiles clean under GCC,
// Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
#ifndef CA_HTTP_CORE_HPP
#define CA_HTTP_CORE_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace http {

namespace detail {

// Split a path or route pattern into slash-delimited, non-empty segments.
// "/" yields no segments; empty segments (leading slash, doubled slash) drop.
inline std::vector<std::string> split_path_segments(const std::string& path) {
    std::vector<std::string> out;
    if (path == "/") return out;
    std::size_t i = 0;
    while (i <= path.size()) {
        std::size_t slash = path.find('/', i);
        std::size_t end = slash == std::string::npos ? path.size() : slash;
        if (end > i) out.push_back(path.substr(i, end - i));
        if (slash == std::string::npos) break;
        i = slash + 1;
    }
    return out;
}

// ASCII case-insensitive equality.
inline bool eq_ignore_ascii_case(const std::string& a, const std::string& b) {
    if (a.size() != b.size()) return false;
    for (std::size_t i = 0; i < a.size(); i++) {
        char ca = a[i], cb = b[i];
        if (ca >= 'A' && ca <= 'Z') ca = static_cast<char>(ca - 'A' + 'a');
        if (cb >= 'A' && cb <= 'Z') cb = static_cast<char>(cb - 'A' + 'a');
        if (ca != cb) return false;
    }
    return true;
}

// Trim leading/trailing ASCII whitespace.
inline std::string trim(const std::string& s) {
    std::size_t b = 0, e = s.size();
    auto ws = [](char c) {
        return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' ||
               c == '\v';
    };
    while (b < e && ws(s[b])) b++;
    while (e > b && ws(s[e - 1])) e--;
    return s.substr(b, e - b);
}

// Trim leading/trailing occurrences of a single character.
inline std::string trim_char(const std::string& s, char ch) {
    std::size_t b = 0, e = s.size();
    while (b < e && s[b] == ch) b++;
    while (e > b && s[e - 1] == ch) e--;
    return s.substr(b, e - b);
}

// Parse an unsigned integer that must be all ASCII digits and fit in `Limit`.
template <typename T>
inline std::optional<T> parse_uint(const std::string& s, std::uint64_t limit) {
    if (s.empty()) return std::nullopt;
    std::uint64_t v = 0;
    for (char c : s) {
        if (c < '0' || c > '9') return std::nullopt;
        std::uint64_t d = static_cast<std::uint64_t>(c - '0');
        // Check BEFORE the multiply so the guard is meaningful even when
        // `limit == UINT64_MAX` (Content-Length on a 64-bit size_t) — a
        // post-multiply `v > limit` would silently accept a wrapped value.
        if (v > (limit - d) / 10) return std::nullopt;
        v = v * 10 + d;
    }
    return static_cast<T>(v);
}

}  // namespace detail

// ── Route patterns ──────────────────────────────────────────────────────────

enum class RouteSegmentKind { Literal, Param };

struct RouteSegment {
    RouteSegmentKind kind;
    std::string text; // literal text, or the parameter name
};

// A generic HTTP path pattern such as "/hello/:name".
struct RoutePattern {
    std::vector<RouteSegment> segments;

    static RoutePattern parse(const std::string& pattern) {
        RoutePattern rp;
        for (const std::string& seg : detail::split_path_segments(pattern)) {
            if (!seg.empty() && seg[0] == ':') {
                rp.segments.push_back({RouteSegmentKind::Param, seg.substr(1)});
            } else {
                rp.segments.push_back({RouteSegmentKind::Literal, seg});
            }
        }
        return rp;
    }

    // Match a path, capturing (name, value) for each Param segment. Returns
    // std::nullopt when the segment count differs or a Literal mismatches.
    std::optional<std::vector<std::pair<std::string, std::string>>> match_path(
        const std::string& path) const {
        std::vector<std::string> actual = detail::split_path_segments(path);
        if (actual.size() != segments.size()) return std::nullopt;
        std::vector<std::pair<std::string, std::string>> params;
        for (std::size_t i = 0; i < segments.size(); i++) {
            const RouteSegment& seg = segments[i];
            if (seg.kind == RouteSegmentKind::Literal) {
                if (seg.text != actual[i]) return std::nullopt;
            } else {
                params.emplace_back(seg.text, actual[i]);
            }
        }
        return params;
    }

    // Match against a full request target; only the path portion is used, so a
    // query string never makes a valid route miss.
    std::optional<std::vector<std::pair<std::string, std::string>>> match_target(
        const std::string& target) const;
};

// ── Headers ─────────────────────────────────────────────────────────────────

struct Header {
    std::string name;
    std::string value;
};

// First header value matching `name` (ASCII case-insensitive), or nullptr.
inline const std::string* find_header(const std::vector<Header>& headers,
                                      const std::string& name) {
    for (const Header& h : headers) {
        if (detail::eq_ignore_ascii_case(h.name, name)) return &h.value;
    }
    return nullptr;
}

// Content-Length when present and a valid non-negative integer.
inline std::optional<std::size_t> parse_content_length(
    const std::vector<Header>& headers) {
    const std::string* v = find_header(headers, "Content-Length");
    if (!v) return std::nullopt;
    return detail::parse_uint<std::size_t>(
        *v, static_cast<std::uint64_t>(static_cast<std::size_t>(-1)));
}

// Content-Type split into media type and optional charset.
inline std::optional<std::pair<std::string, std::optional<std::string>>>
parse_content_type(const std::vector<Header>& headers) {
    const std::string* v = find_header(headers, "Content-Type");
    if (!v) return std::nullopt;

    // Split on ';', trimming each piece; the first is the media type.
    std::vector<std::string> pieces;
    std::size_t i = 0;
    while (i <= v->size()) {
        std::size_t semi = v->find(';', i);
        std::size_t end = semi == std::string::npos ? v->size() : semi;
        pieces.push_back(detail::trim(v->substr(i, end - i)));
        if (semi == std::string::npos) break;
        i = semi + 1;
    }
    std::string media_type = pieces.empty() ? std::string() : pieces[0];
    if (media_type.empty()) return std::nullopt;

    std::optional<std::string> charset;
    for (std::size_t p = 1; p < pieces.size(); p++) {
        std::size_t eq = pieces[p].find('=');
        if (eq == std::string::npos) continue;
        std::string name = detail::trim(pieces[p].substr(0, eq));
        if (detail::eq_ignore_ascii_case(name, "charset")) {
            charset =
                detail::trim_char(detail::trim(pieces[p].substr(eq + 1)), '"');
            break;
        }
    }
    return std::make_pair(media_type, charset);
}

// ── Request target & query ──────────────────────────────────────────────────

// A view of an origin-form request target, owning its parts.
struct RequestTarget {
    std::string path;
    std::optional<std::string> query;
    std::optional<std::string> fragment;

    // Raw (name, value) pairs of the query string; values are NOT decoded.
    std::vector<std::pair<std::string, std::string>> query_pairs() const {
        std::vector<std::pair<std::string, std::string>> out;
        if (!query) return out;
        const std::string& q = *query;
        std::size_t i = 0;
        while (i <= q.size()) {
            std::size_t amp = q.find('&', i);
            std::size_t end = amp == std::string::npos ? q.size() : amp;
            std::string piece = q.substr(i, end - i);
            if (!piece.empty()) {
                std::size_t eq = piece.find('=');
                if (eq == std::string::npos) {
                    out.emplace_back(piece, std::string());
                } else {
                    out.emplace_back(piece.substr(0, eq), piece.substr(eq + 1));
                }
            }
            if (amp == std::string::npos) break;
            i = amp + 1;
        }
        return out;
    }

    std::optional<std::string> query_value(const std::string& name) const {
        for (auto& kv : query_pairs()) {
            if (kv.first == name) return kv.second;
        }
        return std::nullopt;
    }
};

// Split an origin-form target into path, query, and fragment.
inline RequestTarget parse_request_target(const std::string& target) {
    std::string before_fragment = target;
    std::optional<std::string> fragment;
    std::size_t hash = target.find('#');
    if (hash != std::string::npos) {
        before_fragment = target.substr(0, hash);
        fragment = target.substr(hash + 1);
    }
    std::string path = before_fragment;
    std::optional<std::string> query;
    std::size_t q = before_fragment.find('?');
    if (q != std::string::npos) {
        path = before_fragment.substr(0, q);
        query = before_fragment.substr(q + 1);
    }
    RequestTarget rt;
    rt.path = path.empty() ? "/" : path;
    rt.query = query;
    rt.fragment = fragment;
    return rt;
}

inline std::optional<std::vector<std::pair<std::string, std::string>>>
RoutePattern::match_target(const std::string& target) const {
    return match_path(parse_request_target(target).path);
}

// ── HTTP version ────────────────────────────────────────────────────────────

struct HttpVersion {
    std::uint16_t major;
    std::uint16_t minor;

    // Parse a textual "HTTP/x.y" marker; std::nullopt on any malformation.
    static std::optional<HttpVersion> parse(const std::string& text) {
        const std::string prefix = "HTTP/";
        if (text.size() < prefix.size() ||
            text.compare(0, prefix.size(), prefix) != 0) {
            return std::nullopt;
        }
        std::string rest = text.substr(prefix.size());
        std::size_t dot = rest.find('.');
        if (dot == std::string::npos) return std::nullopt;
        auto maj = detail::parse_uint<std::uint16_t>(rest.substr(0, dot), 0xFFFF);
        auto min = detail::parse_uint<std::uint16_t>(rest.substr(dot + 1), 0xFFFF);
        if (!maj || !min) return std::nullopt;
        return HttpVersion{*maj, *min};
    }

    std::string to_string() const {
        return "HTTP/" + std::to_string(major) + "." + std::to_string(minor);
    }
};

// ── Body framing, request/response heads ────────────────────────────────────

enum class BodyKind { None, ContentLength, UntilEof, Chunked };

struct RequestHead {
    std::string method;
    std::string target;
    HttpVersion version;
    std::vector<Header> headers;

    const std::string* header(const std::string& name) const {
        return find_header(headers, name);
    }
    RequestTarget target_parts() const { return parse_request_target(target); }
    std::string path() const { return target_parts().path; }
    std::optional<std::string> query_value(const std::string& name) const {
        return target_parts().query_value(name);
    }
    std::optional<std::size_t> content_length() const {
        return parse_content_length(headers);
    }
    std::optional<std::pair<std::string, std::optional<std::string>>>
    content_type() const {
        return parse_content_type(headers);
    }
};

struct ResponseHead {
    HttpVersion version;
    std::uint16_t status;
    std::string reason;
    std::vector<Header> headers;

    const std::string* header(const std::string& name) const {
        return find_header(headers, name);
    }
    std::optional<std::size_t> content_length() const {
        return parse_content_length(headers);
    }
    std::optional<std::pair<std::string, std::optional<std::string>>>
    content_type() const {
        return parse_content_type(headers);
    }
};

}  // namespace http
}  // namespace ca

#endif  // CA_HTTP_CORE_HPP
