# coding_adventures_state_machine

Formal automata theory in Ruby -- deterministic and non-deterministic finite automata, pushdown automata, Hopcroft minimization, and modal state machines.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) computing stack.

## What's Inside

| Module | What it does | Chomsky level |
|--------|-------------|---------------|
| `DFA` | Deterministic Finite Automaton (5-tuple) | Regular (Type 3) |
| `NFA` | Non-deterministic FA with epsilon transitions | Regular (Type 3) |
| `Minimize` | Hopcroft's DFA minimization algorithm | -- |
| `PDA` | Deterministic Pushdown Automaton | Context-free (Type 2) |
| `ModalStateMachine` | Multiple DFA modes with mode switching | Between Type 3 and Type 1 |

## Installation

```ruby
gem "coding_adventures_state_machine"
```

## Usage

### DFA -- Deterministic Finite Automaton

```ruby
require "coding_adventures_state_machine"
require "set"

# A turnstile: insert coin to unlock, push to lock
turnstile = CodingAdventures::StateMachine::DFA.new(
  states: Set["locked", "unlocked"],
  alphabet: Set["coin", "push"],
  transitions: {
    ["locked", "coin"] => "unlocked",
    ["locked", "push"] => "locked",
    ["unlocked", "coin"] => "unlocked",
    ["unlocked", "push"] => "locked"
  },
  initial: "locked",
  accepting: Set["unlocked"]
)

turnstile.process("coin")        # => "unlocked"
turnstile.accepts(%w[coin push]) # => false
turnstile.accepts(%w[coin])      # => true
turnstile.to_dot                 # => Graphviz DOT string
```

### NFA -- Non-deterministic Finite Automaton

```ruby
# NFA that accepts strings containing "ab"
nfa = CodingAdventures::StateMachine::NFA.new(
  states: Set["q0", "q1", "q2"],
  alphabet: Set["a", "b"],
  transitions: {
    ["q0", "a"] => Set["q0", "q1"],  # non-deterministic!
    ["q0", "b"] => Set["q0"],
    ["q1", "b"] => Set["q2"],
    ["q2", "a"] => Set["q2"],
    ["q2", "b"] => Set["q2"]
  },
  initial: "q0",
  accepting: Set["q2"]
)

nfa.accepts(%w[a b])     # => true
nfa.accepts(%w[b a])     # => false
dfa = nfa.to_dfa         # subset construction
dfa.accepts(%w[a b])     # => true (same language)
```

### Minimization

```ruby
minimized = CodingAdventures::StateMachine.minimize(dfa)
# minimized recognizes the same language with fewer states
```

### PDA -- Pushdown Automaton

```ruby
# PDA for balanced parentheses
pda = CodingAdventures::StateMachine::PushdownAutomaton.new(
  states: Set["q0", "accept"],
  input_alphabet: Set["(", ")"],
  stack_alphabet: Set["(", "$"],
  transitions: [
    CodingAdventures::StateMachine::PDATransition.new("q0", "(", "$", "q0", ["$", "("]),
    CodingAdventures::StateMachine::PDATransition.new("q0", "(", "(", "q0", ["(", "("]),
    CodingAdventures::StateMachine::PDATransition.new("q0", ")", "(", "q0", []),
    CodingAdventures::StateMachine::PDATransition.new("q0", nil, "$", "accept", [])
  ],
  initial: "q0",
  initial_stack_symbol: "$",
  accepting: Set["accept"]
)

pda.accepts(["(", "(", ")", ")"])  # => true
pda.accepts(["(", ")"])            # => true
pda.accepts(["(", "(", ")"])       # => false
```

### Modal State Machine

```ruby
# Simplified HTML tokenizer with DATA and TAG modes
html = CodingAdventures::StateMachine::ModalStateMachine.new(
  modes: { "data" => data_dfa, "tag" => tag_dfa },
  mode_transitions: {
    ["data", "enter_tag"] => "tag",
    ["tag", "exit_tag"] => "data"
  },
  initial_mode: "data"
)

html.process("char")           # processed by data mode's DFA
html.switch_mode("enter_tag")  # switch to tag mode
html.process("char")           # processed by tag mode's DFA
```

## How It Fits in the Stack

This package provides the formal automata theory layer. It connects to:

- **Below**: Logic gates and sequential circuits (the hardware that implements state machines)
- **Above**: Lexers and parsers (which use DFAs for tokenization and PDAs for parsing)
- **Sideways**: The branch predictor package (which is a DFA), the CPU pipeline (which is a linear DFA)

## Development

```bash
bundle install
bundle exec rake test        # run tests
bundle exec standardrb       # lint
```

## License

MIT
