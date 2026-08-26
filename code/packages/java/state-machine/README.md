# State Machine (Java)

Finite-state, nondeterministic, pushdown, minimization, and modal-machine tools for
[`F01-state-machine.md`](../../../specs/F01-state-machine.md).

- `Dfa` provides actions, isolated acceptance queries, traces, validation, tables, ASCII, and DOT.
- `Nfa` implements epsilon closure and collision-free, bounded subset conversion to an
  equivalent DFA.
- `DfaMinimizer` removes unreachable states and refines equivalent partitions.
- `PushdownAutomaton` supports deterministic stack transitions, bounded stacks/traces, and
  end-of-input epsilon closure with complete transition traces.
- `ModalStateMachine` connects resettable DFAs with named mode triggers and exposes the active
  machine.

Definitions and exposed traces/stacks are defensive snapshots. Invalid definitions fail at
construction, while processing distinguishes unknown alphabet symbols from missing transitions.
Trace, subset, and stack ceilings bound hostile or accidentally explosive workloads.
