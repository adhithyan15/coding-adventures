# 00060 - LGE-S3b: tighten the C++ memory extension

* **Spec (VER15a):**
  * New section 5.6.1 defines `cpp.no-address-surface`, `cpp.no-ub-reachable` and `cpp.integer-value-range`
    normatively: they run on the preprocessed and template-instantiated translation unit, scope is every
    externally visible entity including `detail::`, and the banned constructs are listed.
  * The extension's allow-list holds no pointer parameter (the rejection helpers take only `int`) and is pinned
    by AST shape and content hash, not by name; it is "std::string temporaries produced by one fixed expression
    plus one exception object".
  * Trust boundary stated: the standard library and unwinder are trusted, allocation failure is CWE-248/703,
    stack exhaustion is CWE-674; a closing row renders "excluded, assuming a correct standard library".
  * C++ premise paragraph (ODR, falling off a non-void function, `[[noreturn]]` returning, constexpr and lifetime
    UB, division); `requires_closed` also lists the other threats the no-ub rule names. `requires_closed` is
    satisfied only by `excluded` or live-evidence `discharged` (C and C++). A skippable sanitizer run is never
    `live` for an extension row (rule 7 amended). Rule 10 and `requires_closed` are stated as not yet
    machine-enforced. Table names `compiler-flag:strict-cxx17`, `sanitizer-run:asan+ubsan` and
    `analyzer-rule:cpp.no-global-or-static-state`; CWE-190 adds the unsigned/size_t and no-authored-arithmetic
    requirements.
* **cpp.json:** the six memory rows cite LGE-CPP-HARDEN (int-only helper change) as a prerequisite in
  `becomes_when`, add the new `requires_closed` entries and the static-state rule, and record that
  `logic_gates::detail::reject(ptr, v)` is callable today with a caller pointer (CWE-125/476 as shipped).
