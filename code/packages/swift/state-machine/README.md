# StateMachine (Swift)

Deterministic and Non-deterministic Finite Automata for the
coding-adventures project. This is a Swift port of the TypeScript
`state-machine` package, providing the same API and semantics.

## What is this?

This package implements two fundamental automata from the theory of
computation:

- **DFA** (Deterministic Finite Automaton) — always in exactly one state,
  follows exactly one transition per input. Predictable, efficient, and
  easy to implement in hardware.
- **NFA** (Non-deterministic Finite Automaton) — can be in multiple states
  simultaneously, supports epsilon (free) transitions. Easier to construct
  for pattern-matching problems, convertible to an equivalent DFA via
  subset construction.

## Usage

### DFA Example: Turnstile

```swift
import StateMachine

let turnstile = try DFA(
    states: ["locked", "unlocked"],
    alphabet: ["coin", "push"],
    transitions: [
        DFA.TransitionRule(from: "locked", on: "coin", to: "unlocked"),
        DFA.TransitionRule(from: "locked", on: "push", to: "locked"),
        DFA.TransitionRule(from: "unlocked", on: "coin", to: "unlocked"),
        DFA.TransitionRule(from: "unlocked", on: "push", to: "locked"),
    ],
    initial: "locked",
    accepting: ["unlocked"]
)

try turnstile.process("coin")        // returns "unlocked"
turnstile.accepts(["coin", "push"])   // false
turnstile.accepts(["coin"])           // true
```

### NFA Example: Contains "ab"

```swift
import StateMachine

let nfa = try NFA(
    states: ["q0", "q1", "q2"],
    alphabet: ["a", "b"],
    transitions: [
        NFATransitionRule(from: "q0", on: "a", to: ["q0", "q1"]),
        NFATransitionRule(from: "q0", on: "b", to: ["q0"]),
        NFATransitionRule(from: "q1", on: "b", to: ["q2"]),
        NFATransitionRule(from: "q2", on: "a", to: ["q2"]),
        NFATransitionRule(from: "q2", on: "b", to: ["q2"]),
    ],
    initial: "q0",
    accepting: ["q2"]
)

nfa.accepts(["a", "b"])       // true
nfa.accepts(["b", "a"])       // false

// Convert to DFA for efficient repeated matching
let dfa = try nfa.toDfa()
dfa.accepts(["a", "b"])       // true (same language)
```

### NFA to DFA Conversion

Every NFA can be converted to an equivalent DFA via subset construction:

```swift
let dfa = try nfa.toDfa()
// dfa accepts exactly the same language as nfa
```

## API

### DFA

| Method / Property       | Description                                      |
|------------------------|--------------------------------------------------|
| `init(states:alphabet:transitions:initial:accepting:actions:)` | Construct with eager validation |
| `process(_:)`          | Process one event, return new state               |
| `processSequence(_:)`  | Process multiple events, return trace              |
| `accepts(_:)`          | Check acceptance without modifying state           |
| `reset()`              | Return to initial state, clear trace               |
| `reachableStates()`    | BFS to find all reachable states                   |
| `isComplete()`         | True if every (state, event) has a transition      |
| `validate()`           | List of warnings (unreachable states, etc.)        |
| `currentState`         | The current state                                  |
| `trace`                | Execution history as `[TransitionRecord]`          |

### NFA

| Method / Property       | Description                                      |
|------------------------|--------------------------------------------------|
| `init(states:alphabet:transitions:initial:accepting:)` | Construct with eager validation |
| `process(_:)`          | Process one event, return new state set            |
| `processSequence(_:)`  | Process multiple events, return trace              |
| `accepts(_:)`          | Check acceptance without modifying state           |
| `reset()`              | Return to epsilon closure of initial state         |
| `epsilonClosure(_:)`   | Compute epsilon closure of a state set             |
| `toDfa()`              | Convert to equivalent DFA via subset construction  |
| `currentStates`        | The current set of active states                   |

## How it fits in the stack

This package sits alongside other state-machine implementations across
languages in coding-adventures. The DFA is the same computational model
used by the branch predictor (D02), and the NFA-to-DFA conversion is
the algorithm behind regex compilation.

## Dependencies

None. This is a pure-computation library using only the Swift standard library.
