# An output proof must retain safe literal alternatives after call tracking is cleared

The first direct-output post-check passed six focused alias/value controls, but
the complete compiler/shared suite failed one unchanged test (927pass/1fail):
a user-defined abs call in a conditional cleared the statically known Boolean
selector. The alternative1.5 is safe for runtime formatting but has no dynamic
assignment-provenance tag because ordinary literal output takes a static path.
The rejected complete-suite receipt and original test remain unchanged.

Rechecking a formatter proof must account for both alternatives independently.
Use context-free finite literal arithmetic, without procedure calls or tracked
variable values, alongside runtime-proven branches. Keep the original initial
proof, avoid resurrecting lost selector facts and never use a nested conditional
inside multiplication/calls as a shortcut around the bounded-root proof.
Full-cone and real-process controls must establish compatibility; a focused
alias refusal alone cannot accept a new proof consumer.
