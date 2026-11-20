---
category: Rust
---

# A real document used as the example of a refused level also holds the refusals that level was hiding


Two tests of the warrant generator used `vos-sched-any.warrant` as "a document beyond what is evaluated,
refused naming `ir.data`". When S11b made `ir.data` lower, they failed with a `W0701`: ten claims of that
document call no operation or sit under an impure guard, and an example is not `op(..) == pure`. The level
refusal had been the first answer for every item, so the VER04r 4.3 and 4.4 rules never ran. The census pin
moved the same way (`ir.data` 15 became `ok` 5 and `W0701` 10, and the oracle's lowering census says exactly that).

Fix: when a level starts to lower, expect every committed document that was refused at it to show its next
refusal, and pin that (the census table, with why each count moved). Use a document whose only blocker is the
level for a "refused whole" test, or the guard that refuses the level on purpose (`generate` refusing `ir.data`
until S11c-2), and say in the test which one it is.
