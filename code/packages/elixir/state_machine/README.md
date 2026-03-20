# CodingAdventures.StateMachine

A pure Elixir library implementing formal automata — the foundational abstraction
behind parsers, network protocols, hardware controllers, and much more.

This is the Elixir port of the Python `state-machine` package from the
coding-adventures monorepo.

## Modules

| Module | What it does |
|--------|-------------|
| `CodingAdventures.StateMachine.DFA` | Deterministic Finite Automaton — exactly one transition per (state, input) pair |
| `CodingAdventures.StateMachine.NFA` | Non-deterministic Finite Automaton with epsilon transitions |
| `CodingAdventures.StateMachine.Minimize` | Hopcroft's algorithm for DFA minimization |
| `CodingAdventures.StateMachine.PDA` | Pushdown Automaton — finite automaton with a stack for context-free languages |
| `CodingAdventures.StateMachine.Modal` | Modal State Machine — multiple DFA sub-machines with mode switching |
| `CodingAdventures.StateMachine.Types` | Core types (TransitionRecord) |

## How it fits in the stack

This library sits at Layer 5 (State Machines) of the coding-adventures computing
stack. It provides the automata abstractions used by higher layers like the lexer
(Layer 6) and parser (Layer 7). In the Chomsky hierarchy:

- DFA/NFA recognize **regular languages** (Type 3)
- PDA recognizes **context-free languages** (Type 2)
- Modal machines handle **context-sensitive tokenization**

## Usage Examples

### DFA: Turnstile

```elixir
alias CodingAdventures.StateMachine.DFA

{:ok, turnstile} = DFA.new(
  MapSet.new(["locked", "unlocked"]),
  MapSet.new(["coin", "push"]),
  %{
    {"locked", "coin"} => "unlocked",
    {"locked", "push"} => "locked",
    {"unlocked", "coin"} => "unlocked",
    {"unlocked", "push"} => "locked"
  },
  "locked",
  MapSet.new(["unlocked"])
)

DFA.accepts?(turnstile, ["coin"])          # true
DFA.accepts?(turnstile, ["coin", "push"])  # false

{:ok, turnstile} = DFA.process(turnstile, "coin")
turnstile.current  # "unlocked"
```

### NFA: Pattern matching with non-determinism

```elixir
alias CodingAdventures.StateMachine.NFA

{:ok, nfa} = NFA.new(
  MapSet.new(["q0", "q1", "q2"]),
  MapSet.new(["a", "b"]),
  %{
    {"q0", "a"} => MapSet.new(["q0", "q1"]),
    {"q0", "b"} => MapSet.new(["q0"]),
    {"q1", "b"} => MapSet.new(["q2"]),
    {"q2", "a"} => MapSet.new(["q2"]),
    {"q2", "b"} => MapSet.new(["q2"])
  },
  "q0",
  MapSet.new(["q2"])
)

NFA.accepts?(nfa, ["a", "b"])      # true (contains "ab")
NFA.accepts?(nfa, ["b", "a"])      # false

# Convert to DFA for efficient execution
{:ok, dfa} = NFA.to_dfa(nfa)
```

### PDA: Balanced parentheses

```elixir
alias CodingAdventures.StateMachine.PDA
alias CodingAdventures.StateMachine.PDA.Transition

{:ok, pda} = PDA.new(
  MapSet.new(["q0", "accept"]),
  MapSet.new(["(", ")"]),
  MapSet.new(["(", "$"]),
  [
    %Transition{source: "q0", event: "(", stack_read: "$",
                target: "q0", stack_push: ["$", "("]},
    %Transition{source: "q0", event: "(", stack_read: "(",
                target: "q0", stack_push: ["(", "("]},
    %Transition{source: "q0", event: ")", stack_read: "(",
                target: "q0", stack_push: []},
    %Transition{source: "q0", event: nil, stack_read: "$",
                target: "accept", stack_push: []}
  ],
  "q0", "$",
  MapSet.new(["accept"])
)

PDA.accepts?(pda, ["(", "(", ")", ")"])  # true
PDA.accepts?(pda, ["(", "(", ")"])       # false
```

## Design Notes

Since Elixir is functional and immutable, every operation returns a NEW struct.
No mutable state. This actually maps beautifully to automata theory, where each
step of a computation produces a new configuration.

All public functions that can fail return `{:ok, result}` or `{:error, reason}`
tuples, following Elixir conventions.

## Running Tests

```bash
mix test
```
