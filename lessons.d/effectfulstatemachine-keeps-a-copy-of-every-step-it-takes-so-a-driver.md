---
category: Compiler / VM / language pipeline
---

# EffectfulStateMachine keeps a copy of every step it takes, so a driver that runs it over large input must drop the trace

**What went wrong.** `state_machine::transducer::EffectfulStateMachine::process`
pushes a clone of every step (source and target state names, the input,
the effect strings) onto `self.trace`, and only `reset()` clears it.
`ruby-lexer` drives one step per source character and never reads the
trace, so memory grew by a few hundred bytes per input byte: lexing 1 MiB
peaked at ~593 MB RSS.  It surfaced only when #18814 gave the lexer a
1 MiB budget and a test lexed exactly that much.

**Fix.** The lexer drops the trace every 4 096 steps without touching the
shared crate (which `spreadsheet-wasm` hashes): remember
`current_state()`, call `reset()`, then `set_current_state(saved)`.  The
engine holds no other state between steps.  A test lexes every input
twice, dropping the trace after every step and never, and requires
identical tokens and diagnostics.  Peak RSS for 1 MiB fell to ~146 MB.

**Do differently.**
- Before choosing a byte budget, measure peak memory at the budget, not
  just time; a per-character trace multiplies the input many times over.
- Any long-running driver of `EffectfulStateMachine` (or `process`-style
  engines with a `trace()` accessor) should clear the trace periodically,
  or the engine should grow a way to turn tracing off.
