// url_parser.hpp — a URL parser with relative resolution and percent-coding, in
// pure ISO C++17, header-only, in namespace ca::url. A faithful port of the Rust
// `url-parser` crate.
// ===========================================================================
//
// Splits an absolute URL into its components:
//
//   http://alice:secret@www.example.com:8080/docs/page.html?q=hello#sec2
//   scheme  userinfo         host        port     path       query  fragment
//
// Also implements RFC 1808 relative resolution (Url::resolve) and
// percent-encoding / decoding. The scheme and host are lower-cased; the path
// starts with '/' for authority-based URLs; query/fragment exclude their
// leading '?'/'#'. IPv6 hosts keep their brackets.
//
// Parsing / resolving / decoding throw ca::url::ParseError (carrying an
// Error kind) on failure.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_URL_PARSER_HPP
#define CA_URL_PARSER_HPP

#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace ca {
namespace url {

enum class Error {
    MissingScheme,
    InvalidScheme,
    InvalidPort,
    InvalidPercentEncoding,
    EmptyHost,
    RelativeWithoutBase
};

class ParseError : public std::runtime_error {
public:
    explicit ParseError(Error k)
        : std::runtime_error(describe(k)), kind(k) {}
    Error kind;

private:
    static const char* describe(Error k) {
        switch (k) {
            case Error::MissingScheme: return "missing scheme (expected '://')";
            case Error::InvalidScheme: return "invalid scheme";
            case Error::InvalidPort: return "invalid port (must be 0-65535)";
            case Error::InvalidPercentEncoding: return "malformed percent-encoding";
            case Error::EmptyHost: return "empty host in authority-based URL";
            case Error::RelativeWithoutBase: return "relative URL requires a base";
        }
        return "url error";
    }
};

namespace detail {

inline bool is_space(unsigned char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' ||
           c == '\v';
}
inline bool is_lower(unsigned char c) { return c >= 'a' && c <= 'z'; }
inline bool is_digit(unsigned char c) { return c >= '0' && c <= '9'; }
inline bool is_alpha(unsigned char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}
inline bool is_alnum(unsigned char c) { return is_alpha(c) || is_digit(c); }

inline std::string_view trim(std::string_view s) {
    std::size_t start = 0, end = s.size();
    while (start < end && is_space(static_cast<unsigned char>(s[start]))) {
        ++start;
    }
    while (end > start && is_space(static_cast<unsigned char>(s[end - 1]))) {
        --end;
    }
    return s.substr(start, end - start);
}

inline std::string to_lower(std::string_view s) {
    std::string out;
    out.reserve(s.size());
    for (char c : s) {
        unsigned char u = static_cast<unsigned char>(c);
        out.push_back((u >= 'A' && u <= 'Z') ? static_cast<char>(u - 'A' + 'a')
                                             : static_cast<char>(u));
    }
    return out;
}

inline bool scheme_valid(const std::string& s) {
    if (s.empty() || !is_lower(static_cast<unsigned char>(s[0]))) {
        return false;
    }
    for (std::size_t i = 1; i < s.size(); ++i) {
        unsigned char c = static_cast<unsigned char>(s[i]);
        if (!is_lower(c) && !is_digit(c) && c != '+' && c != '-' && c != '.') {
            return false;
        }
    }
    return true;
}

inline bool scheme_like(std::string_view s) {
    if (s.empty() || !is_alpha(static_cast<unsigned char>(s[0]))) {
        return false;
    }
    for (char c : s) {
        unsigned char u = static_cast<unsigned char>(c);
        if (!is_alnum(u) && u != '+' && u != '-' && u != '.') {
            return false;
        }
    }
    return true;
}

inline bool all_digits(std::string_view s) {
    if (s.empty()) {
        return false;
    }
    for (char c : s) {
        if (!is_digit(static_cast<unsigned char>(c))) {
            return false;
        }
    }
    return true;
}

inline std::uint16_t parse_port(std::string_view s) {
    if (s.empty()) {
        throw ParseError(Error::InvalidPort);
    }
    unsigned long acc = 0;
    for (char c : s) {
        if (!is_digit(static_cast<unsigned char>(c))) {
            throw ParseError(Error::InvalidPort);
        }
        acc = acc * 10 + static_cast<unsigned long>(c - '0');
        if (acc > 65535ul) {
            throw ParseError(Error::InvalidPort);
        }
    }
    return static_cast<std::uint16_t>(acc);
}

inline bool utf8_valid(const std::string& s) {
    const unsigned char* p = reinterpret_cast<const unsigned char*>(s.data());
    std::size_t n = s.size(), i = 0;
    while (i < n) {
        unsigned char c = p[i];
        std::size_t extra, k;
        unsigned long min_cp, cp;
        if (c < 0x80) {
            ++i;
            continue;
        } else if ((c & 0xE0) == 0xC0) {
            extra = 1;
            min_cp = 0x80;
            cp = c & 0x1Fu;
        } else if ((c & 0xF0) == 0xE0) {
            extra = 2;
            min_cp = 0x800;
            cp = c & 0x0Fu;
        } else if ((c & 0xF8) == 0xF0) {
            extra = 3;
            min_cp = 0x10000;
            cp = c & 0x07u;
        } else {
            return false;
        }
        if (extra >= n - i) {
            return false;
        }
        for (k = 1; k <= extra; ++k) {
            unsigned char cc = p[i + k];
            if ((cc & 0xC0) != 0x80) {
                return false;
            }
            cp = (cp << 6) | (cc & 0x3Fu);
        }
        if (cp < min_cp || cp > 0x10FFFFuL || (cp >= 0xD800uL && cp <= 0xDFFFuL)) {
            return false;
        }
        i += extra + 1;
    }
    return true;
}

inline std::string remove_dot_segments(std::string_view path) {
    std::vector<std::string_view> out;
    bool leading_slash = !path.empty() && path[0] == '/';
    std::size_t start = 0;
    for (std::size_t i = 0; i <= path.size(); ++i) {
        if (i == path.size() || path[i] == '/') {
            std::string_view seg = path.substr(start, i - start);
            if (seg == ".") {
                // skip
            } else if (seg == "..") {
                if (!out.empty()) {
                    out.pop_back();
                }
            } else {
                out.push_back(seg);
            }
            start = i + 1;
        }
    }
    std::string result;
    for (std::size_t i = 0; i < out.size(); ++i) {
        if (i > 0) {
            result.push_back('/');
        }
        result.append(out[i]);
    }
    if (leading_slash && (result.empty() || result[0] != '/')) {
        result.insert(result.begin(), '/');
    }
    return result;
}

inline std::string merge_paths(std::string_view base, std::string_view rel) {
    std::size_t pos = base.rfind('/');
    if (pos != std::string_view::npos) {
        std::string r(base.substr(0, pos + 1));
        r.append(rel);
        return r;
    }
    std::string r = "/";
    r.append(rel);
    return r;
}

}  // namespace detail

struct Url {
    std::string scheme;
    std::optional<std::string> userinfo;
    std::optional<std::string> host;
    std::optional<std::uint16_t> port;
    std::string path;
    std::optional<std::string> query;
    std::optional<std::string> fragment;

    static Url parse(const std::string& input);

    Url resolve(const std::string& relative) const;

    std::optional<std::uint16_t> effective_port() const {
        if (port) {
            return port;
        }
        if (scheme == "http") return std::uint16_t(80);
        if (scheme == "https") return std::uint16_t(443);
        if (scheme == "ftp") return std::uint16_t(21);
        return std::nullopt;
    }

    std::string authority() const {
        std::string a;
        if (userinfo) {
            a += *userinfo;
            a.push_back('@');
        }
        if (host) {
            a += *host;
        }
        if (port) {
            a.push_back(':');
            a += std::to_string(*port);
        }
        return a;
    }

    std::string to_url_string() const {
        std::string s = scheme;
        if (host) {
            s += "://";
            s += authority();
        } else {
            s.push_back(':');
        }
        s += path;
        if (query) {
            s.push_back('?');
            s += *query;
        }
        if (fragment) {
            s.push_back('#');
            s += *fragment;
        }
        return s;
    }
};

inline Url Url::parse(const std::string& input) {
    using namespace detail;
    std::string_view s = trim(input);

    std::size_t sep = s.find("://");
    if (sep != std::string_view::npos) {
        Url u;
        u.scheme = to_lower(s.substr(0, sep));
        if (!scheme_valid(u.scheme)) {
            throw ParseError(Error::InvalidScheme);
        }
        s = s.substr(sep + 3);

        std::size_t hash = s.find('#');
        if (hash != std::string_view::npos) {
            u.fragment = std::string(s.substr(hash + 1));
            s = s.substr(0, hash);
        }
        std::size_t q = s.find('?');
        if (q != std::string_view::npos) {
            u.query = std::string(s.substr(q + 1));
            s = s.substr(0, q);
        }
        std::string_view authority_sv;
        std::string_view path_sv;
        std::size_t slash = s.find('/');
        if (slash != std::string_view::npos) {
            authority_sv = s.substr(0, slash);
            path_sv = s.substr(slash);
        } else {
            authority_sv = s;
            path_sv = "/";
        }
        u.path = std::string(path_sv);

        std::string_view host_port = authority_sv;
        std::size_t at = authority_sv.rfind('@');
        if (at != std::string_view::npos) {
            u.userinfo = std::string(authority_sv.substr(0, at));
            host_port = authority_sv.substr(at + 1);
        }

        std::string_view host_sv = host_port;
        if (!host_port.empty() && host_port[0] == '[') {
            std::size_t bracket = host_port.find(']');
            if (bracket != std::string_view::npos) {
                host_sv = host_port.substr(0, bracket + 1);
                std::string_view after = host_port.substr(bracket + 1);
                if (!after.empty() && after[0] == ':') {
                    u.port = parse_port(after.substr(1));
                }
            }
        } else {
            std::size_t colon = host_port.rfind(':');
            if (colon != std::string_view::npos) {
                std::string_view maybe = host_port.substr(colon + 1);
                if (all_digits(maybe)) {
                    host_sv = host_port.substr(0, colon);
                    u.port = parse_port(maybe);
                }
            }
        }
        if (!host_sv.empty()) {
            u.host = to_lower(host_sv);
        }
        return u;
    }

    // "scheme:path" form (no authority), e.g. mailto:.
    std::size_t colon = s.find(':');
    if (colon != std::string_view::npos && colon > 0 &&
        s.substr(0, colon).find('/') == std::string_view::npos) {
        Url u;
        u.scheme = to_lower(s.substr(0, colon));
        if (!scheme_valid(u.scheme)) {
            throw ParseError(Error::InvalidScheme);
        }
        std::string_view p = s.substr(colon + 1);
        std::size_t hash = p.find('#');
        if (hash != std::string_view::npos) {
            u.fragment = std::string(p.substr(hash + 1));
            p = p.substr(0, hash);
        }
        std::size_t q = p.find('?');
        if (q != std::string_view::npos) {
            u.query = std::string(p.substr(q + 1));
            p = p.substr(0, q);
        }
        u.path = std::string(p);
        return u;
    }

    throw ParseError(Error::MissingScheme);
}

inline Url Url::resolve(const std::string& relative) const {
    using namespace detail;
    std::string_view r = trim(relative);

    if (r.empty()) {
        Url result = *this;
        result.fragment = std::nullopt;
        return result;
    }
    if (r[0] == '#') {
        Url result = *this;
        result.fragment = std::string(r.substr(1));
        return result;
    }

    // Already absolute.
    std::size_t colon = r.find(':');
    bool has_cc_slash = r.find("://") != std::string_view::npos;
    if (has_cc_slash || (colon != std::string_view::npos && r[0] != '/')) {
        if (colon != std::string_view::npos && colon > 0 &&
            scheme_like(r.substr(0, colon))) {
            return Url::parse(std::string(r));
        }
    }

    // Scheme-relative "//host/path".
    if (r.size() >= 2 && r[0] == '/' && r[1] == '/') {
        return Url::parse(scheme + ":" + std::string(r));
    }

    auto split = [](std::string_view in, std::optional<std::string>& frag,
                    std::optional<std::string>& qry) -> std::string_view {
        std::size_t hash = in.find('#');
        if (hash != std::string_view::npos) {
            frag = std::string(in.substr(hash + 1));
            in = in.substr(0, hash);
        }
        std::size_t q = in.find('?');
        if (q != std::string_view::npos) {
            qry = std::string(in.substr(q + 1));
            in = in.substr(0, q);
        }
        return in;
    };

    // Absolute path "/path".
    if (r[0] == '/') {
        std::optional<std::string> frag, qry;
        std::string_view p = split(r, frag, qry);
        Url result = *this;
        result.path = remove_dot_segments(p);
        result.query = qry;
        result.fragment = frag;
        return result;
    }

    // Relative path — merge with the base path.
    {
        std::optional<std::string> frag, qry;
        std::string_view p = split(r, frag, qry);
        std::string merged = merge_paths(path, p);
        Url result = *this;
        result.path = remove_dot_segments(merged);
        result.query = qry;
        result.fragment = frag;
        return result;
    }
}

// ---- percent coding ---------------------------------------------------

inline std::string percent_encode(const std::string& input) {
    static const char HEX[] = "0123456789ABCDEF";
    std::string out;
    out.reserve(input.size());
    for (char ch : input) {
        unsigned char c = static_cast<unsigned char>(ch);
        bool unreserved = detail::is_alnum(c) || c == '-' || c == '_' ||
                          c == '.' || c == '~' || c == '/';
        if (unreserved) {
            out.push_back(static_cast<char>(c));
        } else {
            out.push_back('%');
            out.push_back(HEX[c >> 4]);
            out.push_back(HEX[c & 0x0F]);
        }
    }
    return out;
}

inline std::string percent_decode(const std::string& input) {
    auto hex = [](unsigned char c, int& out) -> bool {
        if (c >= '0' && c <= '9') {
            out = c - '0';
        } else if (c >= 'a' && c <= 'f') {
            out = c - 'a' + 10;
        } else if (c >= 'A' && c <= 'F') {
            out = c - 'A' + 10;
        } else {
            return false;
        }
        return true;
    };
    std::string bytes;
    std::size_t i = 0, n = input.size();
    while (i < n) {
        if (input[i] == '%') {
            if (i + 2 >= n) {
                throw ParseError(Error::InvalidPercentEncoding);
            }
            int hi, lo;
            if (!hex(static_cast<unsigned char>(input[i + 1]), hi) ||
                !hex(static_cast<unsigned char>(input[i + 2]), lo)) {
                throw ParseError(Error::InvalidPercentEncoding);
            }
            bytes.push_back(static_cast<char>((hi << 4) | lo));
            i += 3;
        } else {
            bytes.push_back(input[i]);
            ++i;
        }
    }
    if (!detail::utf8_valid(bytes)) {
        throw ParseError(Error::InvalidPercentEncoding);
    }
    return bytes;
}

}  // namespace url
}  // namespace ca

#endif  // CA_URL_PARSER_HPP
