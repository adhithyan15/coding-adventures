# ASCII-only repeated classes scan their byte set directly

The deterministic regex matcher now scans repeated ASCII-only character
classes directly in its compiled byte set. It caps the scan by remaining input
and the finite repetition maximum before computing an endpoint, then preserves
the existing minimum-count rejection and following-step behavior. Classes
that may match non-ASCII runes keep the existing UTF-8-aware path.

The bounded matcher tests compare with the pre-change loop and anchored Go
regexp, cover strace grammar dispatch and high/invalid UTF-8 inputs, and include
a positive-offset large-bound overflow regression. These tests establish only
the finite input domains recorded in the G1 profile receipt. The fixed four-
host Linux B/C cohort passed all 16 official cells for both versions, with no
baseline miss/candidate pass rescue; its separate precision measurements are
not gate results. This bounded cohort does not establish general G1 host
robustness or repair the separate protected CI failure at 13.7 MB/s. No
Warrant/SAT source proof is claimed.
