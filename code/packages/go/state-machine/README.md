# state-machine

A Go package implementing finite automata and pushdown automata — the theoretical foundation of all computation. Covers DFA, NFA, DFA minimization, pushdown automata, and modal state machines.

## Where this fits in the stack

```
Layer 0: Logic Gates
Layer 1: Arithmetic
Layer 2: ALU
Layer 3: CPU / GPU control
Layer 4: State Machines      <-- you are here
Layer 5: Grammar Tools (lexer, parser)
```

State machines sit at the base of the Chomsky hierarchy. Every lexer, parser, protocol handler, and control system is built on these primitives. This package provides the automata layer that grammar-tools and other higher-level packages build upon.

## What's included

### Core Types (`types.go`)

| Type | Description |
|------|-------------|
| `State` | Type alias for `string` — a named state |
| `Event` | Type alias for `string` — an input symbol |
| `Action` | `func(source, event, target string)` callback |
| `TransitionRecord` | Trace entry capturing one transition step |

### DFA (`dfa.go`)

Deterministic Finite Automaton — exactly one transition per (state, input) pair.

| Method | Description |
|--------|-------------|
| `NewDFA(...)` | Create with validation |
| `Process(event)` | Process one input, return new state |
| `ProcessSequence(events)` | Process multiple inputs, return trace |
| `Accepts(events)` | Non-mutating acceptance check |
| `Reset()` | Return to initial state |
| `ReachableStates()` | BFS to find reachable states |
| `IsComplete()` | Check if all transitions defined |
| `Validate()` | Return warnings about issues |
| `ToDot()` | Graphviz DOT output |
| `ToAscii()` | ASCII transition table |
| `ToTable()` | Structured table data |

### NFA (`nfa.go`)

Non-deterministic Finite Automaton with epsilon transitions.

| Method | Description |
|--------|-------------|
| `NewNFA(...)` | Create with validation |
| `EpsilonClosure(states)` | Compute epsilon closure |
| `Process(event)` | Process one input |
| `Accepts(events)` | Non-mutating acceptance check |
| `Reset()` | Return to initial state |
| `ToDFA()` | Subset construction conversion |
| `ToDot()` | Graphviz DOT output |

### Minimization (`minimize.go`)

| Function | Description |
|----------|-------------|
| `Minimize(dfa)` | Hopcroft's algorithm — produce minimal equivalent DFA |

### PDA (`pda.go`)

Pushdown Automaton — DFA + stack for context-free languages.

| Method | Description |
|--------|-------------|
| `NewPushdownAutomaton(...)` | Create with validation |
| `Process(event)` | Process one input |
| `ProcessSequence(events)` | Process with epsilon transitions at end |
| `Accepts(events)` | Non-mutating acceptance check |
| `Reset()` | Return to initial state and stack |

### Modal State Machine (`modal.go`)

Multiple DFA sub-machines with mode switching — for context-sensitive tokenizing.

| Method | Description |
|--------|-------------|
| `NewModalStateMachine(...)` | Create with validation |
| `SwitchMode(trigger)` | Switch active DFA mode |
| `Process(event)` | Process in current mode's DFA |
| `Reset()` | Reset all modes |

## Usage

```go
import sm "github.com/adhithyan15/coding-adventures/code/packages/go/state-machine"

// Turnstile DFA
turnstile := sm.NewDFA(
    []string{"locked", "unlocked"},
    []string{"coin", "push"},
    map[[2]string]string{
        {"locked", "coin"}:   "unlocked",
        {"locked", "push"}:   "locked",
        {"unlocked", "coin"}: "unlocked",
        {"unlocked", "push"}: "locked",
    },
    "locked",
    []string{"unlocked"},
    nil,
)
turnstile.Process("coin")         // "unlocked"
turnstile.Accepts([]string{"coin"})  // true

// NFA -> DFA conversion
nfa := sm.NewNFA(...)
dfa := nfa.ToDFA()
minimized := sm.Minimize(dfa)

// Balanced parentheses PDA
open := "("
close := ")"
pda := sm.NewPushdownAutomaton(
    []string{"q0", "accept"},
    []string{"(", ")"},
    []string{"(", "$"},
    []sm.PDATransition{
        {Source: "q0", Event: &open, StackRead: "$", Target: "q0", StackPush: []string{"$", "("}},
        {Source: "q0", Event: &open, StackRead: "(", Target: "q0", StackPush: []string{"(", "("}},
        {Source: "q0", Event: &close, StackRead: "(", Target: "q0", StackPush: []string{}},
        {Source: "q0", Event: nil, StackRead: "$", Target: "accept", StackPush: []string{}},
    },
    "q0", "$", []string{"accept"},
)
pda.Accepts([]string{"(", "(", ")", ")"})  // true
```

## Input validation

All constructors validate inputs eagerly and panic on invalid configuration, matching the convention in the logic-gates package. Runtime errors (invalid events, missing transitions) also panic.

## Testing

```bash
go test ./... -v -cover
```

110 tests with 99% statement coverage. All tests are table-driven.

## Literate programming

All source files use Knuth-style literate programming with extensive comments explaining:
- Formal definitions (5-tuple notation)
- The Chomsky hierarchy and where each automaton fits
- Algorithm walkthroughs (subset construction, Hopcroft's partition refinement)
- Real-world applications (regex engines, HTML tokenizers, CPU branch predictors)
- ASCII diagrams and examples
