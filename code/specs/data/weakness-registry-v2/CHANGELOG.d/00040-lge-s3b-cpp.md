# 00040 - LGE-S3b: cpp lane threat list

* **Added** `lanes/cpp.json`: 53 threats (29 registry classes plus 24 cpp threats) and 53 rows, all `open`
  (no lane evidence is `live`, VER15a 5.2). Seeded from MITRE CWE-659 (97 members fetched 2026-11-24; 21 are
  threats, 76 are in `considered_not_included`), the C lane list, and an audit of `cpp/logic-gates`
  (merged as PR #19928, commit 2a26142613723ee2381a89347f99abbfcf3c4ded).
* **Findings recorded in the file:**
  * The header's "no pointers, arrays, allocation" claim (`logic_gates.hpp:62`) is false on the rejection
    path (`:77-80`): the C reading `c_no_pointers_arrays_allocation` cannot be copied, and CWE-400's
    `pure_zero_allocation` route does not hold (row is `cannot_close_in_package`).
  * No C++ memory-safety exclusion route exists until LGE-CPP-EXT; CWE-416, 787, 125, 415, 476 and 190 are
    `cannot_close_in_package` with `no_planned_closure`.
  * The deleted overloads guard direct calls only: `int (*f)(int,int) = and_gate; f(1.5, 1)` returns 1
    (`cpp.deleted-overload-bypass-via-indirection`).
  * Rejection allocates, so `std::bad_alloc` can replace `std::invalid_argument`; the test catches by
    base-class reference (`cpp.exception-identity`).
  * The lane never builds optimised (`tools/run.sh:21,31-32`), and the sanitizer run is skippable
    (`tools/run.sh:34-36`).
* **Unresolved planned ids:** none. `LGE-L-cpp` is used as the nearest carrier for the extended type matrix
  and the exact-identity binding; the report recommends dedicated backlog rows.
