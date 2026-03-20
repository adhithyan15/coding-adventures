# State Machine

Formal state machine library — DFA, NFA, PDA, and Modal state machines with full traceability.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) Formal Languages track (F-series).

## What's Inside

| Module | Description |
|--------|-------------|
| `dfa` | Deterministic Finite Automaton — one transition per (state, input) |
| `nfa` | Non-deterministic FA — multiple transitions + epsilon, with NFA→DFA conversion |
| `minimize` | Hopcroft's algorithm for DFA minimization |
| `pda` | Pushdown Automaton — FSM + stack for context-free languages |
| `modal` | Modal State Machine — multiple sub-machines with mode switching |

## Quick Start

```python
from state_machine import DFA

# A turnstile: insert coin to unlock, push to lock
turnstile = DFA(
    states={"locked", "unlocked"},
    alphabet={"coin", "push"},
    transitions={
        ("locked", "coin"): "unlocked",
        ("locked", "push"): "locked",
        ("unlocked", "coin"): "unlocked",
        ("unlocked", "push"): "locked",
    },
    initial="locked",
    accepting={"unlocked"},
)

# Process events
turnstile.process("coin")       # → "unlocked"
turnstile.accepts(["coin"])     # → True
turnstile.accepts(["push"])     # → False

# Full trace of execution
trace = turnstile.process_sequence(["coin", "push", "coin"])
for t in trace:
    print(f"{t.source} --{t.event}--> {t.target}")

# Visualize as Graphviz DOT
print(turnstile.to_dot())

# ASCII transition table
print(turnstile.to_ascii())
```

## How It Fits in the Stack

The state machine library is a cross-cutting foundation:

- **Lexer / Grammar Tools**: modal state machines enable context-sensitive tokenization (HTML, Markdown)
- **Branch Predictor (D02)**: the 2-bit saturating counter is a 4-state DFA
- **CPU Pipeline (D04)**: fetch-decode-execute is a linear state machine
- **Browser (future)**: HTML tokenizer, TCP states, HTTP parsing

See the [spec](https://github.com/adhithyan15/coding-adventures/tree/main/code/specs/F01-state-machine.md) for full details.
