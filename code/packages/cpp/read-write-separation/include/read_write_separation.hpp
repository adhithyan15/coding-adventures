// read_write_separation.hpp — capability read/write-separation analysis,
// header-only in pure ISO C++17 (namespace ca::read_write_separation). A
// faithful port of the Rust `read-write-separation` crate.
// ===========================================================================
//
// An agent declares a manifest of capabilities — each a (category, action,
// target) triple. The read/write separation (RWS) principle forbids a single
// agent from holding both an UNTRUSTED INPUT and an EXTERNAL ACTUATION (or
// overlapping read/write access to one resource), since that lets untrusted
// data drive a side-effecting action. This library classifies capabilities and
// validates a manifest against that rule.
//
// Each capability has a Flavor (Ingestion / Actuation / Internal) and a Trust
// (Trusted / Untrusted); when unset they are inferred from the category/action
// (e.g. "net connect" defaults to an untrusted actuation; "fs read" of a
// "package:"-internal target is trusted).
//
// DIVERGENCE FROM RUST. Rust returns `Result<(), RwsViolation>`; this port's
// `validate_manifest` returns `std::optional<Violation>` (empty == valid).
// Capabilities use value semantics with a fluent builder.
//
// PORTABILITY. Pure ISO C++17 — standard library only, no compiler extensions.
#ifndef CA_READ_WRITE_SEPARATION_HPP
#define CA_READ_WRITE_SEPARATION_HPP

#include <cstddef>
#include <optional>
#include <string>
#include <vector>

namespace ca {
namespace read_write_separation {

enum class Flavor { Ingestion, Actuation, Internal };
enum class Trust { Trusted, Untrusted };

inline const char* to_string(Flavor f) {
    switch (f) {
        case Flavor::Ingestion: return "ingestion";
        case Flavor::Actuation: return "actuation";
        case Flavor::Internal: return "internal";
    }
    return "internal";
}
inline const char* to_string(Trust t) {
    switch (t) {
        case Trust::Trusted: return "trusted";
        case Trust::Untrusted: return "untrusted";
    }
    return "trusted";
}

// A declared capability with a fluent builder.
class Capability {
public:
    std::string category;
    std::string action;
    std::string target;
    std::optional<Flavor> flavor;
    std::optional<Trust> trust;
    std::optional<std::string> justification;

    Capability(std::string category_, std::string action_, std::string target_)
        : category(std::move(category_)),
          action(std::move(action_)),
          target(std::move(target_)) {}

    Capability with_flavor(Flavor f) const {
        Capability c = *this;
        c.flavor = f;
        return c;
    }
    Capability with_trust(Trust t) const {
        Capability c = *this;
        c.trust = t;
        return c;
    }
    Capability with_justification(std::string j) const {
        Capability c = *this;
        c.justification = std::move(j);
        return c;
    }

    std::string identifier() const {
        return category + ":" + action + ":" + target;
    }

    bool operator==(const Capability& o) const {
        return category == o.category && action == o.action &&
               target == o.target && flavor == o.flavor && trust == o.trust &&
               justification == o.justification;
    }
    bool operator!=(const Capability& o) const { return !(*this == o); }
};

// The resolved classification of a single capability.
struct Classification {
    Flavor flavor;
    Trust trust;
    bool is_input;
    bool is_untrusted_input;
    bool is_external_actuation;
};

namespace detail {

inline bool starts_with(const std::string& s, const std::string& prefix) {
    return s.size() >= prefix.size() &&
           s.compare(0, prefix.size(), prefix) == 0;
}

inline Flavor default_flavor(const Capability& c) {
    const std::string& cat = c.category;
    const std::string& act = c.action;
    if ((cat == "net" && act == "connect") ||
        (cat == "fs" &&
         (act == "write" || act == "create" || act == "delete")) ||
        (cat == "vault" && (act == "write" || act == "request_lease"))) {
        return Flavor::Actuation;
    }
    if (cat == "proc") return Flavor::Actuation;
    return Flavor::Internal;
}

inline bool is_loopback_target(const std::string& t) {
    return t == "localhost" || starts_with(t, "localhost:") ||
           t == "127.0.0.1" || starts_with(t, "127.0.0.1:") || t == "::1" ||
           starts_with(t, "[::1]:");
}

inline bool is_package_internal_target(const std::string& t) {
    return starts_with(t, "package:") || starts_with(t, "pkg:") ||
           starts_with(t, "./package/") || starts_with(t, "package/");
}

inline Trust default_trust(const Capability& c) {
    const std::string& cat = c.category;
    const std::string& act = c.action;
    if (cat == "net" && act == "connect") return Trust::Untrusted;
    if (cat == "net" && act == "listen")
        return is_loopback_target(c.target) ? Trust::Trusted : Trust::Untrusted;
    if (cat == "fs" && act == "read")
        return is_package_internal_target(c.target) ? Trust::Trusted
                                                    : Trust::Untrusted;
    return Trust::Trusted;
}

inline bool is_input_capability(const Capability& c, Flavor flavor) {
    const std::string& cat = c.category;
    const std::string& act = c.action;
    if (cat == "net" && act == "connect") return flavor == Flavor::Ingestion;
    if ((cat == "net" && act == "listen") || (cat == "fs" && act == "read") ||
        (cat == "channel" && act == "read"))
        return true;
    return flavor == Flavor::Ingestion;
}

inline bool is_read_side(const Capability& c) {
    const std::string& cat = c.category;
    const std::string& act = c.action;
    return (cat == "fs" && act == "read") || (cat == "vault" && act == "read") ||
           (cat == "channel" && act == "read");
}

inline bool is_write_side(const Capability& c) {
    const std::string& cat = c.category;
    const std::string& act = c.action;
    return (cat == "fs" &&
            (act == "write" || act == "create" || act == "delete")) ||
           (cat == "vault" && (act == "write" || act == "request_lease")) ||
           (cat == "channel" && act == "write");
}

inline bool glob_prefix_matches(const std::string& pattern,
                                const std::string& value) {
    if (pattern.empty() || pattern.back() != '*') return false;
    std::string prefix = pattern.substr(0, pattern.size() - 1);
    return starts_with(value, prefix);
}

inline bool resources_overlap(const std::string& left,
                              const std::string& right) {
    return left == right || glob_prefix_matches(left, right) ||
           glob_prefix_matches(right, left);
}

inline void push_unique(std::vector<Capability>& v, const Capability& c) {
    for (const Capability& e : v)
        if (e == c) return;
    v.push_back(c);
}

inline std::size_t count_overlap_pairs(
    const std::vector<Capability>& caps) {
    std::size_t count = 0;
    for (const Capability& read : caps) {
        if (!is_read_side(read)) continue;
        for (const Capability& write : caps) {
            if (!is_write_side(write) || read.category != write.category)
                continue;
            if (resources_overlap(read.target, write.target)) count++;
        }
    }
    return count;
}

inline bool collect_overlap_violations(const std::vector<Capability>& caps,
                                       std::vector<Capability>& reads,
                                       std::vector<Capability>& writes) {
    bool found = false;
    for (const Capability& read : caps) {
        if (!is_read_side(read)) continue;
        for (const Capability& write : caps) {
            if (!is_write_side(write) || read.category != write.category)
                continue;
            if (resources_overlap(read.target, write.target)) {
                push_unique(reads, read);
                push_unique(writes, write);
                found = true;
            }
        }
    }
    return found;
}

}  // namespace detail

inline Classification classify_capability(const Capability& c) {
    Flavor flavor = c.flavor.value_or(detail::default_flavor(c));
    Trust trust = c.trust.value_or(detail::default_trust(c));
    bool is_input = detail::is_input_capability(c, flavor);
    Classification cl;
    cl.flavor = flavor;
    cl.trust = trust;
    cl.is_input = is_input;
    cl.is_untrusted_input = is_input && trust == Trust::Untrusted;
    cl.is_external_actuation = flavor == Flavor::Actuation;
    return cl;
}

// Aggregate counts over a manifest.
struct Summary {
    std::size_t total_capabilities = 0;
    std::size_t ingestion_capabilities = 0;
    std::size_t actuation_capabilities = 0;
    std::size_t internal_capabilities = 0;
    std::size_t trusted_capabilities = 0;
    std::size_t untrusted_capabilities = 0;
    std::size_t input_capabilities = 0;
    std::size_t untrusted_inputs = 0;
    std::size_t external_actuations = 0;
    std::size_t read_side_capabilities = 0;
    std::size_t write_side_capabilities = 0;
    std::size_t overlapping_read_write_pairs = 0;
    std::size_t justified_capabilities = 0;

    bool is_empty() const { return total_capabilities == 0; }
    bool has_rws_risk() const {
        return untrusted_inputs > 0 && external_actuations > 0;
    }
    bool has_same_resource_overlap() const {
        return overlapping_read_write_pairs > 0;
    }
};

inline Summary summarize_manifest(const std::vector<Capability>& caps) {
    Summary s;
    s.overlapping_read_write_pairs = detail::count_overlap_pairs(caps);
    for (const Capability& c : caps) {
        Classification cl = classify_capability(c);
        s.total_capabilities++;
        switch (cl.flavor) {
            case Flavor::Ingestion: s.ingestion_capabilities++; break;
            case Flavor::Actuation: s.actuation_capabilities++; break;
            case Flavor::Internal: s.internal_capabilities++; break;
        }
        switch (cl.trust) {
            case Trust::Trusted: s.trusted_capabilities++; break;
            case Trust::Untrusted: s.untrusted_capabilities++; break;
        }
        if (cl.is_input) s.input_capabilities++;
        if (cl.is_untrusted_input) s.untrusted_inputs++;
        if (cl.is_external_actuation) s.external_actuations++;
        if (detail::is_read_side(c)) s.read_side_capabilities++;
        if (detail::is_write_side(c)) s.write_side_capabilities++;
        if (c.justification.has_value()) s.justified_capabilities++;
    }
    return s;
}

// A read/write-separation violation.
struct Violation {
    std::vector<Capability> untrusted_inputs;
    std::vector<Capability> actuations;
    std::string message;
};

// Validate a manifest. Returns std::nullopt when it satisfies RWS, or the
// Violation describing why it does not.
inline std::optional<Violation> validate_manifest(
    const std::vector<Capability>& caps) {
    std::vector<Capability> untrusted_inputs;
    std::vector<Capability> actuations;

    for (const Capability& c : caps) {
        Classification cl = classify_capability(c);
        if (cl.is_untrusted_input) detail::push_unique(untrusted_inputs, c);
        if (cl.is_external_actuation) detail::push_unique(actuations, c);
    }

    bool has_untrusted_and_actuation =
        !untrusted_inputs.empty() && !actuations.empty();
    bool has_overlap = detail::collect_overlap_violations(caps, untrusted_inputs,
                                                          actuations);

    if (has_untrusted_and_actuation || has_overlap) {
        std::string message =
            has_overlap
                ? "read/write separation violation: manifest contains "
                  "overlapping read/write capabilities"
                : "read/write separation violation: manifest contains "
                  "untrusted inputs and external actuations; split the agent "
                  "or insert a trusted channel boundary";
        return Violation{std::move(untrusted_inputs), std::move(actuations),
                         std::move(message)};
    }
    return std::nullopt;
}

}  // namespace read_write_separation
}  // namespace ca

#endif  // CA_READ_WRITE_SEPARATION_HPP
