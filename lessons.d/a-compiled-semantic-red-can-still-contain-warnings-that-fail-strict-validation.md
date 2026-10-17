---
category: Rust
---

# A compiled semantic red can still contain warnings that fail strict validation

CLOC40's tests-first regression compiled and showed the intended stale-root
failures. Its two child-fixture branches also had unnecessary parentheses around
if expressions. The later full suites actually passed all 2,962 tests, but the
warnings-denied package check failed on those parentheses; the CLI strict check
was never reached. A successful Rust build and expected semantic red did not
establish strict acceptance.

Remove only the two unnecessary pairs of parentheses, retain every expression,
literal and assertion, and add no lint allowance. Preserve the failed receipt
and its genuine passing test counts with aggregate acceptance withheld. Run the
affected full suites and both strict checks on the actual repaired successor;
do not transfer the earlier test result to another source head. Parse compiler
diagnostic messages as well as test-failure text when inspecting a red run.
