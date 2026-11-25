# 00050 - LGE-S3b: cpp lane list review fixes and the C++ memory extension

* **Spec amended (VER15a section 5.5, rule 10, section 12, row 8 of section 3):** added the lane extension
  `cpp_address_surface_allowlist` (CWE-416, 787, 125, 415, 476, 190). It needs `analyzer-rule:cpp.no-address-surface`
  and `analyzer-rule:cpp.no-ub-reachable` (CWE-190 also `analyzer-rule:cpp.integer-value-range`) and a
  `requires_closed` of `cpp.signed-overflow-ub`, `cpp.bitwise-shift`, `cpp.uninitialized-variable`. The allow-list
  is exactly `detail::reject` (one `std::string` temporary, one exception object), because a C++ package cannot
  claim "no allocation" while its rejection path throws a heap-allocated exception. The C extension is unchanged
  and `c.json` stays valid.
* **cpp.json (54 threats, 54 rows, all open):**
  * New threat `cpp.braced-init-list-bypass`: `and_gate({true}, 1)` returns 1 because a braced argument skips the
    deleted templates (also `{b}`, `{c}`; `{'a'}` throws "got 97"). Cwe-843, cwe-704, implicit-integer-conversion
    and caller-side-narrowing now say the guard covers plain-expression calls only.
  * `cpp.deleted-overload-bypass-via-indirection`: `std::function` and `std::bind` removed (they do not compile
    against the overload set); the limit is an explicit function-pointer type, a cast or a lambda.
  * The six memory rows now have an exclude route with the new subset (open, `becomes_status: excluded`).
  * `cpp.macro-collision-include-order` is open with a planned discharge (hostile-macro include test).
  * `reached: none` for cwe-400, optimizer-removes-check, exception-model-disabled, uninitialized-variable,
    bitwise-shift. CWE-400 is reworded as a registry-route mismatch.
  * The extended type matrix is carried by LGE-CPP-MATRIX, the fixes by LGE-CPP-HARDEN.
  * Low items: ABI skew (`_GLIBCXX_USE_CXX11_ABI`), `sizeof(int)`, noexcept caller termination, CWE-676 and
    CWE-480..482 coverage, AssertMacros qualification, 96,454 attributed to the backlog row.
* **Backlog:** added LGE-CPP-HARDEN and LGE-CPP-MATRIX, rewrote LGE-CPP-EXT, marked LGE-CPP1 merged (#19928).
