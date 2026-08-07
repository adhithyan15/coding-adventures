// symbolic_ir.hpp — the universal symbolic-expression IR, header-only in pure
// ISO C++17 (namespace ca::symbolic_ir). A faithful port of the Rust
// `symbolic-ir` crate.
// ===========================================================================
//
// One shared tree every CAS frontend compiles to and every backend consumes.
// A `Node` is one of six variants:
//
//   Symbol(name)     named atom: variable, constant, or operation head
//   Integer(i64)     64-bit integer literal
//   Rational(n, d)   exact fraction, ALWAYS reduced with d > 0
//   Float(f64)       double-precision literal
//   Str(text)        string literal
//   Apply(head,args) compound: head(arg0, arg1, ...)
//
// Nodes have value semantics; the recursive `Apply` payload is shared via a
// std::shared_ptr (immutable, so sharing is safe and copies are cheap).
//
// EQUALITY. Structural, matching the Rust PartialEq: floats compare by raw bit
// pattern (identical-bit NaNs are equal), Apply compares recursively. `hash()`
// is consistent with `operator==`.
//
// DIVERGENCE FROM RUST. Rust's `rational` panics on a zero denominator; this
// port throws std::invalid_argument. Float `to_string` uses the shortest
// `%g`-style round-tripping decimal (always with a decimal point), matching
// Rust's `{:?}` for the common cases. `ln` etc. are not needed here.
//
// PORTABILITY. Pure ISO C++17 — standard library only, no compiler extensions.
#ifndef CA_SYMBOLIC_IR_HPP
#define CA_SYMBOLIC_IR_HPP

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <stdexcept>
#include <string>
#include <utility>
#include <variant>
#include <vector>

namespace ca {
namespace symbolic_ir {

inline constexpr const char* VERSION = "0.2.0";

enum class Kind { Symbol, Integer, Rational, Float, Str, Apply };

struct ApplyData;  // defined below (needs a complete Node)

namespace detail {

// Two's-complement magnitude as u64 — correct for INT64_MIN, no UB.
inline std::uint64_t uabs64(std::int64_t v) {
    return v < 0 ? ~static_cast<std::uint64_t>(v) + 1u
                 : static_cast<std::uint64_t>(v);
}
inline std::int64_t i64_from_mag(std::uint64_t mag, bool neg) {
    return neg ? static_cast<std::int64_t>(~mag + 1u)
               : static_cast<std::int64_t>(mag);
}
inline std::uint64_t gcd_u64(std::uint64_t a, std::uint64_t b) {
    while (b != 0) {
        std::uint64_t t = b;
        b = a % b;
        a = t;
    }
    return a;
}

// FNV-1a mixing.
inline std::uint64_t fnv_mix(std::uint64_t h, const void* data, std::size_t n) {
    const unsigned char* p = static_cast<const unsigned char*>(data);
    for (std::size_t i = 0; i < n; i++) {
        h ^= p[i];
        h *= 1099511628211ull;
    }
    return h;
}

// Shortest round-tripping decimal, always with a decimal point / exponent.
inline std::string format_float(double v) {
    char tmp[64];
    for (int prec = 1; prec <= 17; prec++) {
        std::snprintf(tmp, sizeof tmp, "%.*g", prec, v);
        if (std::strtod(tmp, nullptr) == v) break;
    }
    std::string s(tmp);
    if (s.find_first_of(".eEni") == std::string::npos) s += ".0";
    return s;
}

}  // namespace detail

// The six-variant IR node (value semantics).
class Node {
public:
    // ── factories ────────────────────────────────────────────────────────
    static Node symbol(std::string name) {
        return Node(Kind::Symbol, std::move(name));
    }
    static Node integer(std::int64_t n) { return Node(Kind::Integer, n); }
    static Node floating(double v) { return Node(Kind::Float, v); }
    static Node str(std::string s) {
        return Node(Kind::Str, std::move(s));
    }

    // Reduced rational (sign in numerator, denom > 0); collapses to Integer
    // when the denominator reduces to 1. Throws std::invalid_argument if
    // denom == 0 (the Rust panic).
    static Node rational(std::int64_t numer, std::int64_t denom) {
        if (denom == 0)
            throw std::invalid_argument(
                "rational: denominator cannot be zero");
        bool neg = (numer < 0) != (denom < 0);
        std::uint64_t un = detail::uabs64(numer);
        std::uint64_t ud = detail::uabs64(denom);
        std::uint64_t g = detail::gcd_u64(un, ud);
        if (g == 0) g = 1;
        un /= g;
        ud /= g;
        if (ud == 1) return integer(detail::i64_from_mag(un, neg));
        Node n(Kind::Rational,
               std::pair<std::int64_t, std::int64_t>(
                   detail::i64_from_mag(un, neg),
                   static_cast<std::int64_t>(ud)));
        return n;
    }

    // Defined out-of-line below (needs a complete ApplyData).
    static Node apply(Node head, std::vector<Node> args);

    // ── accessors ────────────────────────────────────────────────────────
    Kind kind() const { return kind_; }
    const std::string& symbol_name() const { return std::get<std::string>(data_); }
    std::int64_t integer_value() const { return std::get<std::int64_t>(data_); }
    std::pair<std::int64_t, std::int64_t> rational_parts() const {
        return std::get<std::pair<std::int64_t, std::int64_t>>(data_);
    }
    double float_value() const { return std::get<double>(data_); }
    const std::string& str_value() const { return std::get<std::string>(data_); }
    const Node& apply_head() const;                 // out-of-line
    const std::vector<Node>& apply_args() const;    // out-of-line

    // ── operations ───────────────────────────────────────────────────────
    bool operator==(const Node& other) const;       // out-of-line
    bool operator!=(const Node& other) const { return !(*this == other); }
    std::size_t hash() const;                        // out-of-line
    std::string to_string() const;                   // out-of-line

private:
    using Payload =
        std::variant<std::string, std::int64_t,
                     std::pair<std::int64_t, std::int64_t>, double,
                     std::shared_ptr<const ApplyData>>;
    Kind kind_;
    Payload data_;

    Node(Kind k, Payload p) : kind_(k), data_(std::move(p)) {}
    const std::shared_ptr<const ApplyData>& apply_ptr() const {
        return std::get<std::shared_ptr<const ApplyData>>(data_);
    }
    friend struct ApplyData;
};

// The compound payload — a complete Node is available now.
struct ApplyData {
    Node head;
    std::vector<Node> args;
};

inline Node Node::apply(Node head, std::vector<Node> args) {
    auto data = std::make_shared<ApplyData>(
        ApplyData{std::move(head), std::move(args)});
    return Node(Kind::Apply, std::move(data));
}

inline const Node& Node::apply_head() const { return apply_ptr()->head; }
inline const std::vector<Node>& Node::apply_args() const {
    return apply_ptr()->args;
}

inline bool Node::operator==(const Node& other) const {
    if (kind_ != other.kind_) return false;
    switch (kind_) {
        case Kind::Symbol:
        case Kind::Str:
            return std::get<std::string>(data_) ==
                   std::get<std::string>(other.data_);
        case Kind::Integer:
            return std::get<std::int64_t>(data_) ==
                   std::get<std::int64_t>(other.data_);
        case Kind::Rational:
            return rational_parts() == other.rational_parts();
        case Kind::Float: {
            double a = float_value(), b = other.float_value();
            std::uint64_t ab, bb;
            std::memcpy(&ab, &a, sizeof ab);
            std::memcpy(&bb, &b, sizeof bb);
            return ab == bb;  // bit-pattern equality
        }
        case Kind::Apply: {
            const ApplyData& x = *apply_ptr();
            const ApplyData& y = *other.apply_ptr();
            if (x.args.size() != y.args.size()) return false;
            if (!(x.head == y.head)) return false;
            for (std::size_t i = 0; i < x.args.size(); i++)
                if (!(x.args[i] == y.args[i])) return false;
            return true;
        }
    }
    return false;
}

inline std::size_t Node::hash() const {
    std::uint64_t h = 14695981039346656037ull;  // FNV offset basis
    unsigned char tag = static_cast<unsigned char>(kind_);
    h = detail::fnv_mix(h, &tag, 1);
    switch (kind_) {
        case Kind::Symbol:
        case Kind::Str: {
            const std::string& s = std::get<std::string>(data_);
            h = detail::fnv_mix(h, s.data(), s.size());
            break;
        }
        case Kind::Integer: {
            std::int64_t n = std::get<std::int64_t>(data_);
            h = detail::fnv_mix(h, &n, sizeof n);
            break;
        }
        case Kind::Rational: {
            auto pr = rational_parts();
            h = detail::fnv_mix(h, &pr.first, sizeof pr.first);
            h = detail::fnv_mix(h, &pr.second, sizeof pr.second);
            break;
        }
        case Kind::Float: {
            double v = float_value();
            std::uint64_t bits;
            std::memcpy(&bits, &v, sizeof bits);
            h = detail::fnv_mix(h, &bits, sizeof bits);
            break;
        }
        case Kind::Apply: {
            const ApplyData& a = *apply_ptr();
            std::uint64_t hh = a.head.hash();
            h = detail::fnv_mix(h, &hh, sizeof hh);
            for (const Node& arg : a.args) {
                std::uint64_t ah = arg.hash();
                h = detail::fnv_mix(h, &ah, sizeof ah);
            }
            break;
        }
    }
    return static_cast<std::size_t>(h);
}

inline std::string Node::to_string() const {
    switch (kind_) {
        case Kind::Symbol:
            return symbol_name();
        case Kind::Integer:
            return std::to_string(integer_value());
        case Kind::Rational: {
            auto pr = rational_parts();
            return std::to_string(pr.first) + "/" + std::to_string(pr.second);
        }
        case Kind::Float:
            return detail::format_float(float_value());
        case Kind::Str:
            return "\"" + str_value() + "\"";
        case Kind::Apply: {
            const ApplyData& a = *apply_ptr();
            std::string out = a.head.to_string() + "(";
            for (std::size_t i = 0; i < a.args.size(); i++) {
                if (i > 0) out += ", ";
                out += a.args[i].to_string();
            }
            out += ")";
            return out;
        }
    }
    return std::string();
}

// ── Convenience free-function constructors (mirror the Rust helpers) ─────────
inline Node sym(std::string name) { return Node::symbol(std::move(name)); }
inline Node integer(std::int64_t n) { return Node::integer(n); }
inline Node rat(std::int64_t numer, std::int64_t denom) {
    return Node::rational(numer, denom);
}
inline Node flt(double v) { return Node::floating(v); }
inline Node str_node(std::string s) { return Node::str(std::move(s)); }
inline Node apply(Node head, std::vector<Node> args) {
    return Node::apply(std::move(head), std::move(args));
}

// ── Standard head-name constants ─────────────────────────────────────────────
inline constexpr const char* ADD = "Add";
inline constexpr const char* SUB = "Sub";
inline constexpr const char* MUL = "Mul";
inline constexpr const char* DIV = "Div";
inline constexpr const char* POW = "Pow";
inline constexpr const char* NEG = "Neg";
inline constexpr const char* INV = "Inv";
inline constexpr const char* EXP = "Exp";
inline constexpr const char* LOG = "Log";
inline constexpr const char* SIN = "Sin";
inline constexpr const char* COS = "Cos";
inline constexpr const char* TAN = "Tan";
inline constexpr const char* SQRT = "Sqrt";
inline constexpr const char* ATAN = "Atan";
inline constexpr const char* ASIN = "Asin";
inline constexpr const char* ACOS = "Acos";
inline constexpr const char* SINH = "Sinh";
inline constexpr const char* COSH = "Cosh";
inline constexpr const char* TANH = "Tanh";
inline constexpr const char* ASINH = "Asinh";
inline constexpr const char* ACOSH = "Acosh";
inline constexpr const char* ATANH = "Atanh";
inline constexpr const char* COTH = "Coth";
inline constexpr const char* SECH = "Sech";
inline constexpr const char* CSCH = "Csch";
inline constexpr const char* D = "D";
inline constexpr const char* INTEGRATE = "Integrate";
inline constexpr const char* EQUAL = "Equal";
inline constexpr const char* NOT_EQUAL = "NotEqual";
inline constexpr const char* LESS = "Less";
inline constexpr const char* GREATER = "Greater";
inline constexpr const char* LESS_EQUAL = "LessEqual";
inline constexpr const char* GREATER_EQUAL = "GreaterEqual";
inline constexpr const char* AND = "And";
inline constexpr const char* OR = "Or";
inline constexpr const char* NOT = "Not";
inline constexpr const char* IF = "If";
inline constexpr const char* LIST = "List";
inline constexpr const char* ASSIGN = "Assign";
inline constexpr const char* DEFINE = "Define";
inline constexpr const char* RULE = "Rule";

}  // namespace symbolic_ir
}  // namespace ca

#endif  // CA_SYMBOLIC_IR_HPP
