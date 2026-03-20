# @coding-adventures/state-machine

Formal automata (DFA, NFA, PDA, Modal) for parsing, protocols, and hardware — with full execution tracing.

## What is this?

This library provides composable, traceable state machines covering the full spectrum of automata theory:

- **DFA** (Deterministic Finite Automaton) — exactly one transition per (state, input) pair
- **NFA** (Non-deterministic Finite Automaton) — multiple transitions, epsilon moves, subset construction to DFA
- **DFA Minimization** — Hopcroft's algorithm for the smallest equivalent DFA
- **PDA** (Pushdown Automaton) — finite automaton + stack for context-free languages
- **Modal State Machine** — multiple DFA sub-machines with mode switching

## Where it fits in the stack

This is the TypeScript port of the Python `state-machine` package. State machines are the formal foundation for:

- Lexers/tokenizers (each token rule is a DFA)
- Protocol handlers (TCP state machine, HTTP request parsing)
- Hardware controllers (CPU pipeline stages, branch predictors)
- UI state management (modal editors, form wizards)

## Usage

```typescript
import { DFA, NFA, PushdownAutomaton, ModalStateMachine, minimize, transitionKey } from "@coding-adventures/state-machine";

// Turnstile DFA
const turnstile = new DFA(
  new Set(["locked", "unlocked"]),
  new Set(["coin", "push"]),
  new Map([
    [transitionKey("locked", "coin"), "unlocked"],
    [transitionKey("locked", "push"), "locked"],
    [transitionKey("unlocked", "coin"), "unlocked"],
    [transitionKey("unlocked", "push"), "locked"],
  ]),
  "locked",
  new Set(["unlocked"]),
);

turnstile.process("coin");        // "unlocked"
turnstile.accepts(["coin"]);      // true
turnstile.accepts(["coin", "push"]); // false

// NFA -> DFA conversion
const nfa = new NFA(/* ... */);
const dfa = nfa.toDfa();
const minimal = minimize(dfa);
```

## Running tests

```bash
npm ci
npx vitest run --coverage
```
