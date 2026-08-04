// logic_core.hpp — terms, substitutions, and first-order unification.
// ====================================================================
//
// A faithful, header-only port of the Rust `logic-core` crate (namespace
// `ca::logic_core`) — the data layer of a logic programming engine (à la
// Prolog): the term universe, a substitution (variable → term map), and
// unification with the occurs-check.
//
//   atom(name)              a zero-arity symbolic constant
//   integer(i) / real(x)    a numeric term (int and float are distinct)
//   string(s)               a quoted string (distinct from an atom)
//   var(name)               a fresh logic variable (identity is its id)
//   compound(f, args)       a functor applied to argument terms
//   logic_list({a, b})      the Prolog cons-cell list .(a, .(b, []))
//
// A `Substitution` is persistent in spirit: `extend` returns a NEW substitution
// and never mutates the old one. `unify` returns `std::optional<Substitution>` —
// `nullopt` when the terms cannot be made syntactically equal. The occurs-check
// is on, so `X = f(X)` fails rather than looping.
//
// Faithful divergences: Rust's process-wide `AtomicU64` id counter becomes a
// plain `static` counter (only distinct ids are observable); the Rust builders
// `int` / `float` are spelled `integer` / `real` here (`int`/`float` are
// keywords); float display uses `%g` (matches Rust `{}` for `1.0` → `"1"`).
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no compiler extensions.
#ifndef LOGIC_CORE_HPP
#define LOGIC_CORE_HPP

#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <optional>
#include <string>
#include <unordered_map>
#include <variant>
#include <vector>

namespace ca::logic_core {

// ── Numbers ──────────────────────────────────────────────────────────────────
//
// Int and float are distinct alternatives, so `1` and `1.0` are NOT equal
// (Prolog tradition) and integer equality is exact.
using Number = std::variant<std::int64_t, double>;

inline std::string number_to_string(const Number& n) {
    if (const auto* i = std::get_if<std::int64_t>(&n))
        return std::to_string(*i);
    char buf[64];
    std::snprintf(buf, sizeof buf, "%g", std::get<double>(n));
    return buf;
}

// ── Logic variables ──────────────────────────────────────────────────────────

struct LogicVar {
    std::uint64_t id;
    std::optional<std::string> display_name;

    // Allocate a brand-new variable with a fresh, unique id.
    static LogicVar fresh(std::optional<std::string> display_name = std::nullopt) {
        return LogicVar{next_id_++, std::move(display_name)};
    }

    friend bool operator==(const LogicVar& a, const LogicVar& b) {
        return a.id == b.id;  // identity is the id; the name is cosmetic
    }
    friend bool operator!=(const LogicVar& a, const LogicVar& b) {
        return !(a == b);
    }

   private:
    // Single-threaded pure-ISO counter (Rust uses AtomicU64; only distinct ids
    // are observable).
    inline static std::uint64_t next_id_ = 0;
};

// ── Terms ────────────────────────────────────────────────────────────────────

class Term;

// `Atom` and `Str` are distinct tag types even though both wrap a string.
struct Atom {
    std::string name;
};
struct Str {
    std::string value;
};
struct Compound {
    std::string functor;
    std::vector<Term> args;
};

class Term {
   public:
    std::variant<Atom, Number, Str, LogicVar, Compound> node;
};

inline bool operator==(const Atom& a, const Atom& b) { return a.name == b.name; }
inline bool operator==(const Str& a, const Str& b) { return a.value == b.value; }
bool operator==(const Term& a, const Term& b);  // forward
inline bool operator==(const Compound& a, const Compound& b) {
    return a.functor == b.functor && a.args == b.args;
}
inline bool operator==(const Term& a, const Term& b) { return a.node == b.node; }
inline bool operator!=(const Term& a, const Term& b) { return !(a == b); }

// ── Constructors ─────────────────────────────────────────────────────────────

inline Term atom(std::string name) { return Term{Atom{std::move(name)}}; }
inline Term integer(std::int64_t value) { return Term{Number{value}}; }
inline Term real(double value) { return Term{Number{value}}; }
inline Term string(std::string value) { return Term{Str{std::move(value)}}; }
inline LogicVar var(const std::string& display_name) {
    return LogicVar::fresh(display_name);
}
inline Term var_term(const LogicVar& v) { return Term{v}; }
inline Term compound(std::string functor, std::vector<Term> args) {
    return Term{Compound{std::move(functor), std::move(args)}};
}
// The canonical Prolog `'.'/2` cons-cell list: .(a, .(b, … [])).
inline Term logic_list(std::vector<Term> items) {
    Term result = atom("[]");
    for (auto it = items.rbegin(); it != items.rend(); ++it)
        result = compound(".", {std::move(*it), std::move(result)});
    return result;
}

// ── Display ──────────────────────────────────────────────────────────────────

inline std::string to_string(const Term& t) {
    if (const auto* a = std::get_if<Atom>(&t.node)) return a->name;
    if (const auto* n = std::get_if<Number>(&t.node))
        return number_to_string(*n);
    if (const auto* s = std::get_if<Str>(&t.node)) {
        std::string out = "\"";
        for (char c : s->value) {
            if (c == '"' || c == '\\') out.push_back('\\');
            out.push_back(c);
        }
        out.push_back('"');
        return out;
    }
    if (const auto* v = std::get_if<LogicVar>(&t.node))
        return v->display_name ? *v->display_name : "_G" + std::to_string(v->id);
    const auto& c = std::get<Compound>(t.node);
    std::string out = c.functor + "(";
    for (std::size_t i = 0; i < c.args.size(); ++i) {
        if (i > 0) out += ", ";
        out += to_string(c.args[i]);
    }
    out.push_back(')');
    return out;
}

// ── Substitution & unification ───────────────────────────────────────────────

class Substitution {
   public:
    static Substitution empty() { return Substitution{}; }

    // Return a NEW substitution with `var_id` bound to `term`; original intact.
    Substitution extend(std::uint64_t var_id, Term term) const {
        Substitution s = *this;
        s.bindings_[var_id] = std::move(term);
        return s;
    }

    // Chase variable bindings to a non-variable or an unbound variable.
    Term walk(const Term& term) const {
        Term current = term;
        while (const auto* v = std::get_if<LogicVar>(&current.node)) {
            auto it = bindings_.find(v->id);
            if (it == bindings_.end()) break;
            current = it->second;
        }
        return current;
    }

    Term walk_var(const LogicVar& v) const { return walk(var_term(v)); }

    std::size_t size() const { return bindings_.size(); }

    // True if `var_id` occurs anywhere in `term` (after walking). Occurs-check.
    bool occurs(std::uint64_t var_id, const Term& term) const {
        Term w = walk(term);
        if (const auto* v = std::get_if<LogicVar>(&w.node))
            return v->id == var_id;
        if (const auto* c = std::get_if<Compound>(&w.node))
            for (const auto& a : c->args)
                if (occurs(var_id, a)) return true;
        return false;
    }

    friend bool operator==(const Substitution& a, const Substitution& b) {
        return a.bindings_ == b.bindings_;
    }
    friend bool operator!=(const Substitution& a, const Substitution& b) {
        return !(a == b);
    }

   private:
    std::unordered_map<std::uint64_t, Term> bindings_;
};

// First-order unification with occurs-check. Returns the extended substitution,
// or `std::nullopt` if the terms cannot be unified.
inline std::optional<Substitution> unify(const Term& a0, const Term& b0,
                                         const Substitution& s) {
    Term a = s.walk(a0);
    Term b = s.walk(b0);

    const auto* va = std::get_if<LogicVar>(&a.node);
    const auto* vb = std::get_if<LogicVar>(&b.node);

    if (va && vb && va->id == vb->id) return s;
    if (va) {
        if (s.occurs(va->id, b)) return std::nullopt;
        return s.extend(va->id, b);
    }
    if (vb) {
        if (s.occurs(vb->id, a)) return std::nullopt;
        return s.extend(vb->id, a);
    }
    if (const auto* aa = std::get_if<Atom>(&a.node)) {
        const auto* ba = std::get_if<Atom>(&b.node);
        return (ba && aa->name == ba->name) ? std::optional<Substitution>(s)
                                            : std::nullopt;
    }
    if (const auto* an = std::get_if<Number>(&a.node)) {
        const auto* bn = std::get_if<Number>(&b.node);
        return (bn && *an == *bn) ? std::optional<Substitution>(s)
                                  : std::nullopt;
    }
    if (const auto* as = std::get_if<Str>(&a.node)) {
        const auto* bs = std::get_if<Str>(&b.node);
        return (bs && as->value == bs->value) ? std::optional<Substitution>(s)
                                              : std::nullopt;
    }
    if (const auto* ac = std::get_if<Compound>(&a.node)) {
        const auto* bc = std::get_if<Compound>(&b.node);
        if (!bc || ac->functor != bc->functor ||
            ac->args.size() != bc->args.size())
            return std::nullopt;
        Substitution cur = s;
        for (std::size_t i = 0; i < ac->args.size(); ++i) {
            auto next = unify(ac->args[i], bc->args[i], cur);
            if (!next) return std::nullopt;
            cur = std::move(*next);
        }
        return cur;
    }
    return std::nullopt;
}

}  // namespace ca::logic_core

#endif  // LOGIC_CORE_HPP
