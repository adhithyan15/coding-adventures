---
category: Testing & coverage
---

# Direct entier output uses the integer builtin while a real destination uses the real formatter

The first eight-unit test expected one `__basic_print_real` in all three
contexts. Direct `output(entier(...))` legitimately emits `call_builtin print_i64`
instead, so the focused29P1F/library874P1F receipt stays failed even though all
nine owned runtime groups and the incoming case passed separately.

Correct the specification before correcting the new oracle. Assert both exact
formatter counts: real destination/relay1real0integer, direct0real1integer.
Keep an exact read-once producer call check in the direct context. Preserve all
positive forms, real destination and conditional-arm controls, exact runtime
normal-exit/order assertions and the complete four-line production boundary.
Do not widen production or erase the failed receipt to satisfy a wrong oracle.

The lessons creator truncates long filename slugs. Use the actual printed path;
the first helper assumed the full title's slug and stopped before any commit or
test edit. Its specification edit and generated lesson remained available.

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.
