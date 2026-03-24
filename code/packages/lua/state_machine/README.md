# state-machine

Finite automata (DFA, NFA, PDA, Modal, Minimize) built on directed graphs. A Lua 5.4 port of the Go `state-machine` package in the coding-adventures monorepo.

## What's included

| Class | Chomsky Level | Description |
|-------|---------------|-------------|
| `DFA` | Type 3 (Regular) | Deterministic Finite Automaton with actions, tracing, and visualization |
| `NFA` | Type 3 (Regular) | Non-deterministic FA with epsilon transitions and subset construction (`to_dfa`) |
| `PDA` | Type 2 (Context-Free) | Pushdown Automaton with stack for balanced parens, `a^n b^n`, etc. |
| `ModalStateMachine` | Practical extension | Multiple DFA sub-machines with mode switching (HTML tokenizer pattern) |
| `Minimize` | Algorithm | Hopcroft's DFA minimization via partition refinement |

## Dependencies

- `coding-adventures-directed-graph` (LabeledGraph for internal structure)

## Quick start

```lua
local sm = require("coding_adventures.state_machine")
local DFA = sm.DFA

-- Build a turnstile DFA
local turnstile = DFA.new(
    {"locked", "unlocked"},           -- states
    {"coin", "push"},                 -- alphabet
    {                                 -- transitions
        {"locked", "coin"}, "unlocked",
        {"locked", "push"}, "locked",
        {"unlocked", "coin"}, "unlocked",
        {"unlocked", "push"}, "locked",
    },
    "locked",                         -- initial state
    {"unlocked"},                     -- accepting states
    nil                               -- actions (optional)
)

-- Process events
turnstile:process("coin")     -- returns "unlocked"
turnstile:process("push")     -- returns "locked"

-- Check acceptance
turnstile:accepts({"coin"})          -- true
turnstile:accepts({"coin", "push"})  -- false

-- Introspection
turnstile:is_complete()        -- true
turnstile:reachable_states()   -- {locked=true, unlocked=true}
turnstile:validate()           -- {} (no warnings)

-- Visualization
print(turnstile:to_dot())     -- Graphviz DOT
print(turnstile:to_ascii())   -- ASCII transition table
```

### NFA with epsilon transitions

```lua
local NFA = sm.NFA

local nfa = NFA.new(
    {"q0", "q1", "q2"},
    {"a", "b"},
    {
        {"q0", "a"}, {"q0", "q1"},   -- non-deterministic: two targets
        {"q0", "b"}, {"q0"},
        {"q1", "b"}, {"q2"},
    },
    "q0",
    {"q2"}
)

nfa:accepts({"a", "b"})    -- true (contains "ab")
nfa:accepts({"b", "a"})    -- false

-- Convert to DFA
local dfa = nfa:to_dfa()
local min_dfa = sm.Minimize(dfa)
```

### PDA for balanced parentheses

```lua
local PDA = sm.PDA

local pda = PDA.new(
    {"q0", "accept"},
    {"(", ")"},
    {"(", "$"},
    {
        { source="q0", event="(", stack_read="$", target="q0", stack_push={"$","("} },
        { source="q0", event="(", stack_read="(", target="q0", stack_push={"(","("} },
        { source="q0", event=")", stack_read="(", target="q0", stack_push={} },
        { source="q0", event=nil, stack_read="$", target="accept", stack_push={} },
    },
    "q0", "$", {"accept"}
)

pda:accepts({"(", "(", ")", ")"})   -- true
pda:accepts({"(", ")"})             -- true
pda:accepts({"(", "(", ")"})        -- false
```

## Development

```bash
# Run tests
bash BUILD
```

## How it fits in the stack

This package sits at the automata theory layer of the computing stack:

```
  Applications (programs)
       |
  Parsing / Tokenization  <-- state-machine powers lexers
       |
  Formal Languages        <-- DFA/NFA = Regular, PDA = Context-Free
       |
  directed-graph           <-- structural queries (reachability, etc.)
       |
  Logic Gates / CPU        <-- hardware that executes the automata
```

The modal state machine is particularly important for context-sensitive tokenization (e.g., HTML with embedded CSS/JS), which is the bridge between the lexer and parser layers.
