---
category: Rust
---

# BEAM imported calls cannot carry array lengths in scratch X registers

PR #16126's first macOS CI run found an intermittent BEAM11 upper-bound
failure: an index equal to the declared extent returned normally. The
allocation path had placed that extent in a scratch X register before
`ets:new/2` and read it again afterward, even though imported calls can
clobber X registers. Windows and Ubuntu passed, and repeated local Erlang
tests did not reveal it. Reserve an initialized Y stack slot for values that
must cross a call inside one lowered IIR instruction, reload after the call,
and include that slot in the frame-size limit. A real Erlang bounds test and
a stress test that reads the resulting array length protect the contract.
