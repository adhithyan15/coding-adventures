---
category: Testing & coverage
---

# A structural test that emitted code sits in a save/restore bracket cannot catch an empty liveness set

`iir-to-beam` requires that every op emitting a `call_ext` be listed in one
`matches!` in `lower.rs`, because a call clobbers the x-registers and the
listed ops are the ones whose live variables get Y-spilled across it. Adding
`array_len` (BEAM10), I wrote the lowering correctly — operands staged above
`live`, `save_live_across_imported_call!` before the calls,
`restore_live_across_imported_call!` after — and forgot the list.

Every end-to-end program died with `{badarith,[{erlang,'-',[1,[]]}]}`.

The mechanism is worth knowing, because "I forgot to save registers" undersells
it. `restore_live_across_imported_call!` ends by calling
`sanitize_normal_x_after_call!`, which nils every normal x-register it cannot
prove is live or is the instruction's result. With an empty `live_across` entry
it can prove nothing about anything, so it nils them **all**. The omission does
not degrade register saving; it actively destroys the frame.

The part that matters for testing: **the test I had planned would have passed.**
I had written, in the spec, "every new `call_ext` sits inside a save/restore
bracket, asserted structurally against the emitted sequence". That assertion is
true of the broken code. The bracket was present, correctly placed, and
correctly paired. What was empty was the *set of variables the bracket had to
save* — a data-flow fact computed elsewhere, which the shape of the emitted
sequence does not reveal.

This is the fourth time this backend has been bitten by the missing-op-list
class (VM-D029, VM-D035, issue #15332, and this one), and the first time it
happened despite a spec section written specifically to warn about it. Writing
the warning down was not protection.

**What to do differently:** assert the fact the behaviour depends on, not a
shape that correlates with it. Here that means a test reading the op list
itself and asserting membership — plus a control assertion that the extracted
slice really is the list, so a mis-delimited slice fails loudly instead of
silently matching nothing. And for any codegen change, run the emitted code:
five real-`erl` executions found in seconds what the structural assertion was
designed to miss.

The general form: when a test asserts on the *output* of a stage, ask which
inputs to that stage it cannot see. If a silent-wrong-answer bug lives in one
of those inputs, the test is decorative for that bug no matter how precise it
looks.

Related: [[adding-a-rust-test-target-does-not-protect-it]],
[[assert-structure-not-substrings]].
