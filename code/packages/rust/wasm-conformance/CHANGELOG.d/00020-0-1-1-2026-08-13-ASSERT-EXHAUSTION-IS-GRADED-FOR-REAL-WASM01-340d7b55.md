## 0.1.1 — 2026-08-13 — assert_exhaustion is graded for real (WASM01)

`wasm-execution` 0.6.2 added a real call-depth guard, closing the exact
gap that forced this crate to never execute `assert_exhaustion`
directives at all (an unbounded-recursion host-crash risk, not just a
coverage gap). Both vendored `assert_exhaustion` cases (`call.wast`) now
run for real and pass. Updated the module doc comment and `Executor`'s
own reasoning to match; the old `assert_exhaustion_is_never_executed`
test replaced with `assert_exhaustion_passes_on_real_unbounded_recursion`
and a matching `_fails_if_the_action_returns_normally` case. Baseline
regenerated: `assert_exhaustion 2/2 (100%)`, up from `0/2
(NotYetSupported)`.

A security review of `wasm-execution`'s guard found its first chosen
depth (200) wasn't actually safe on small thread stacks in a debug
build; the corrected, safe value (80) is deliberately conservative
enough that 2 previously-"passing" `assert_return` cases in `call.wast`
(`even(100)`/`odd(200)`, genuinely bounded mutual recursion needing more
than 80 levels) now correctly trap instead — see `wasm-execution`'s own
`0.6.2` changelog entry for the full trade-off reasoning and the tracked
follow-up. `assert_return` moved from 12032/12238 to 12030/12238 as a
direct, understood consequence, not a new bug.

