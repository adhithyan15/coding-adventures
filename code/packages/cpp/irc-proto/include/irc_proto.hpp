// irc_proto.hpp — pure IRC message parsing and serialization (RFC 1459), in
// pure ISO C++17, header-only, in namespace ca::irc. A faithful port of the
// Rust `irc-proto` crate.
// ===========================================================================
//
// WHAT IT IS. The foundation of an IRC stack: it knows nothing about sockets,
// threads, or buffers — it only converts between the raw text lines of the IRC
// protocol and structured `Message` values.
//
//     message  = [ ":" prefix SPACE ] command [ params ] CRLF
//     params   = 0*14( SPACE middle ) [ SPACE ":" trailing ]
//
// A message carries an optional prefix, a command, and up to 15 parameters —
// the last of which may contain spaces when introduced by ':'.
//
// ERRORS. `parse` throws `ca::irc::ParseError` on an empty/whitespace-only line
// or a prefix with no command; `try_parse` returns `std::nullopt` instead.
// Value semantics throughout.
//
// DIVERGENCE. Command upper-casing is ASCII-only (Rust's `to_uppercase` is
// Unicode-aware) — IRC commands are ASCII, so the two agree byte-for-byte.
//
// PORTABILITY. Pure ISO C++17 — standard library only. Compiles clean under
// GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
// warnings-as-errors.
#ifndef CA_IRC_PROTO_HPP
#define CA_IRC_PROTO_HPP

#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace irc {

// A single parsed IRC protocol message.
struct Message {
    std::optional<std::string> prefix; // who sent it (nullopt if client-originated)
    std::string command;               // ASCII-uppercased
    std::vector<std::string> params;   // trailing param's leading ':' already stripped
};

// Thrown by parse() on a malformed line.
class ParseError : public std::runtime_error {
public:
    explicit ParseError(const std::string& what)
        : std::runtime_error("IRC parse error: " + what) {}
};

// RFC 1459 allows at most 15 parameters in a single message.
inline constexpr std::size_t MAX_PARAMS = 15;

namespace detail {

inline bool is_ws(char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' ||
           c == '\v';
}
inline bool is_all_ws(const std::string& s) {
    for (char c : s) {
        if (!is_ws(c)) return false;
    }
    return true;
}
// ASCII-uppercase a–z, copying every other byte verbatim.
inline std::string ascii_upper(std::string s) {
    for (char& c : s) {
        if (c >= 'a' && c <= 'z') c = static_cast<char>(c - 'a' + 'A');
    }
    return s;
}

}  // namespace detail

// Parse one IRC line (with its trailing CRLF already stripped) into a Message.
// Throws ParseError when the line is empty/whitespace-only, has a prefix but no
// command, or yields no command token.
inline Message parse(const std::string& line) {
    if (line.empty() || detail::is_all_ws(line)) {
        throw ParseError("empty or whitespace-only line");
    }

    std::size_t pos = 0;
    Message msg;

    // Stage 1: optional prefix — a leading ':' up to the first space.
    if (line[pos] == ':') {
        std::size_t sp = line.find(' ', pos);
        if (sp == std::string::npos) {
            throw ParseError("line has prefix but no command");
        }
        msg.prefix = line.substr(1, sp - 1);
        pos = sp + 1;
    }

    // Stage 2: command — the first space-delimited token, ASCII-uppercased.
    std::size_t csp = line.find(' ', pos);
    std::size_t cmd_end = csp == std::string::npos ? line.size() : csp;
    std::string command = detail::ascii_upper(line.substr(pos, cmd_end - pos));
    if (command.empty()) {
        throw ParseError("could not extract command");
    }
    msg.command = std::move(command);
    pos = csp == std::string::npos ? line.size() : csp + 1;

    // Stage 3: parameters (at most MAX_PARAMS). A token beginning with ':' is
    // the trailing parameter and absorbs the rest of the line.
    while (pos < line.size()) {
        if (line[pos] == ':') {
            msg.params.push_back(line.substr(pos + 1));
            break;
        }
        std::size_t psp = line.find(' ', pos);
        if (psp == std::string::npos) {
            msg.params.push_back(line.substr(pos));
            break;
        }
        msg.params.push_back(line.substr(pos, psp - pos));
        pos = psp + 1;
        if (msg.params.size() == MAX_PARAMS) break;
    }

    return msg;
}

// Non-throwing form: std::nullopt on any parse error.
inline std::optional<Message> try_parse(const std::string& line) {
    try {
        return parse(line);
    } catch (const ParseError&) {
        return std::nullopt;
    }
}

// Serialize a Message to IRC wire format (CRLF-terminated). The trailing param
// is introduced with ':' when it contains a space, is empty, or begins with ':'.
inline std::string serialize(const Message& msg) {
    std::string out;
    bool need_space = false;
    if (msg.prefix) {
        out += ':';
        out += *msg.prefix;
        need_space = true;
    }
    if (need_space) out += ' ';
    out += msg.command;

    std::size_t n = msg.params.size();
    for (std::size_t i = 0; i < n; i++) {
        out += ' ';
        const std::string& p = msg.params[i];
        bool is_last = (i + 1 == n);
        if (is_last &&
            (p.find(' ') != std::string::npos || p.empty() || p[0] == ':')) {
            out += ':';
        }
        out += p;
    }
    out += "\r\n";
    return out;
}

}  // namespace irc
}  // namespace ca

#endif  // CA_IRC_PROTO_HPP
