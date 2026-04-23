# F01 — State Machine

## Overview

A state machine is one of the most fundamental abstractions in all of
computing. Every digital circuit is a state machine. Every network protocol is
a state machine. Every parser is a state machine. Every vending machine, traffic
light, and elevator controller is a state machine.

The idea is deceptively simple: you have a set of **states**, a set of
**inputs**, and a set of **rules** that say "if you're in state X and you see
input Y, move to state Z." That's it. Yet this simple abstraction is powerful
enough to model everything from a light switch (2 states) to the HTML
tokenizer in your web browser (80+ states).

This library provides formal, composable, traceable state machines. DFA, NFA,
PDA, and modal machines are important specializations, but they are not the
only primitives. The shared foundation also includes effectful transducers:
ordered state machines whose transitions can emit portable effects while they
move between states. That wider primitive is what tokenizer-style systems such
as HTML need.

## Layer Position

The state machine library is a **cross-cutting foundation** — it has no
dependencies on other packages in the repo, but is used by many layers:

```
                    Formal Languages Track (F-series)
                    ┌─────────────────────────┐
                    │  F01: State Machine      │ ← YOU ARE HERE
                    └─────────┬───────────────┘
                              │
        ┌─────────────────────┼──────────────────────┐
        │                     │                      │
   Lexer (02)           Branch Predictor (D02)   Future: HTML tokenizer
   Grammar Tools        CPU Pipeline (D04)       Future: TCP/HTTP parser
   Parser (03)          Sequential Logic (10)    Future: Markdown parser
```

**Depends on:** `directed-graph` (uses `LabeledDirectedGraph` internally for
graph structure, reachability, and visualization).
**Used by:** `lexer`, `grammar-tools`, `branch-predictor` (retroactive demo),
future browser/networking packages.

## Key Concepts

### What Is a State Machine?

Imagine a traffic light. At any moment, it is in one of three states: **Red**,
**Yellow**, or **Green**. When a timer fires, it transitions to the next state.
It never skips a state, and it never goes backwards (well, except from Red to
Green, completing the cycle).

```
         timer          timer          timer
  ┌───┐ ─────→ ┌──────┐ ─────→ ┌────┐ ─────→ (back to Green)
  │Red│         │Green │         │Yel.│
  └───┘ ←───── └──────┘         └────┘
     ↑                                │
     └────────────────────────────────┘
```

That's a state machine: a fixed set of states, a set of inputs (here just
"timer"), and rules for transitioning between states. The machine is always in
exactly one state, and each input causes exactly one transition.

### The Formal Definition (5-Tuple)

Mathematicians define a **Deterministic Finite Automaton (DFA)** as a 5-tuple:

```
M = (Q, Σ, δ, q₀, F)

where:
  Q  = a finite set of states
  Σ  = a finite set of input symbols (the "alphabet")
  δ  = a transition function: Q × Σ → Q
       "given a state and an input, return the next state"
  q₀ = the initial state (q₀ ∈ Q)
  F  = a set of accepting/final states (F ⊆ Q)
```

The traffic light would be:

```
Q  = {Red, Green, Yellow}
Σ  = {timer}
δ  = {(Red, timer) → Green, (Green, timer) → Yellow, (Yellow, timer) → Red}
q₀ = Red
F  = {} (traffic lights don't "accept" — they run forever)
```

The accepting states F are used when the machine is recognizing a language —
for example, a DFA that checks whether a binary number is divisible by 3 would
have an accepting state for "remainder is 0."

### Why "Deterministic"?

A DFA is **deterministic** because for every (state, input) pair, there is
exactly **one** next state. No ambiguity, no choices. Given the same starting
state and the same sequence of inputs, a DFA always follows the same path and
reaches the same final state.

This is in contrast to a **Non-deterministic Finite Automaton (NFA)**, where a
single (state, input) pair can lead to **multiple** possible next states. We'll
get to NFAs shortly — they are more expressive for defining machines, but DFAs
are more efficient for executing them. The magic is that every NFA can be
converted to an equivalent DFA.

### Effectful Transducers

Recognition-only automata answer yes/no questions about input languages. Many
real systems need one more dimension: taking a transition should be able to
produce outputs or update declared registers.

An HTML tokenizer is the canonical browser example. In the `data` state, seeing
`<` does not merely move to `tag_open`; it also flushes buffered text. In a tag
name state, seeing `>` emits the current tag token. At EOF, the tokenizer emits
an EOF token without consuming a real character. These are still state-machine
transitions, but they are **effectful** transitions.

The generic primitive is:

```text
state + ordered matcher -> next state + consume flag + effects
```

DFA transitions are the zero-effect, always-consuming subset of this model.
NFA and PDA keep their own execution rules, but they share the same typed
definition layer so generated source, serializers, visualizers, and validators
can talk about state machines without assuming every machine is a DFA.

### A More Interesting Example: Binary Divisibility by 3

Let's build a DFA that reads a binary number one bit at a time (most
significant bit first) and accepts if the number is divisible by 3.

The key insight: as we read each bit, we're computing `number = number * 2 + bit`.
The remainder after dividing by 3 can only be 0, 1, or 2. So we need 3 states:

```
States: {r0, r1, r2}  (remainder is 0, 1, or 2)
Alphabet: {0, 1}
Initial state: r0  (we start with number = 0, which has remainder 0)
Accepting states: {r0}  (remainder 0 means divisible by 3)

Transition logic:
  If current remainder is r and we read bit b:
    new remainder = (r * 2 + b) mod 3

Transition table:
  ┌───────┬────────────┬────────────┐
  │ State │ Input: 0   │ Input: 1   │
  ├───────┼────────────┼────────────┤
  │  r0   │ r0         │ r1         │  (0*2+0=0, 0*2+1=1)
  │  r1   │ r2         │ r0         │  (1*2+0=2, 1*2+1=3→0)
  │  r2   │ r1         │ r2         │  (2*2+0=4→1, 2*2+1=5→2)
  └───────┴────────────┴────────────┘

State diagram:
              0                0
         ┌────────┐       ┌────────┐
         │        ▼       │        ▼
        ┌──┐     ┌──┐    ┌──┐
    ───►│r0│     │r1│    │r2│
        └──┘     └──┘    └──┘
         ▲  │      ▲  │    ▲  │
         │  │  1   │  │  1 │  │
         │  └──────┘  │    │  │
         │            │    │  │
         │     1      │    │  │
         └────────────┘    │  │
                      0    │  │
                      └────┘  │
                              │ 1
                         ┌────┘
                         └────┐
                              (self-loop)

Let's verify: input "110" (= 6 in decimal)
  Start in r0
  Read '1': r0 → r1  (remainder = 1)
  Read '1': r1 → r0  (remainder = (1*2+1) mod 3 = 0)
  Read '0': r0 → r0  (remainder = (0*2+0) mod 3 = 0)
  End in r0 → ACCEPT ✓  (6 is divisible by 3)

Input "101" (= 5 in decimal)
  Start in r0
  Read '1': r0 → r1
  Read '0': r1 → r2
  Read '1': r2 → r2
  End in r2 → REJECT ✓  (5 is not divisible by 3)
```

This example shows the power of DFAs: three states and a simple table can
check divisibility by 3 for arbitrarily long binary numbers, processing one
bit at a time with no memory beyond the current state.

### Non-Deterministic Finite Automata (NFA)

An NFA relaxes the deterministic constraint in two ways:

1. **Multiple transitions:** A single (state, input) pair can lead to multiple
   next states. The machine "guesses" which one to follow — or equivalently,
   follows all of them simultaneously.

2. **Epsilon (ε) transitions:** The machine can transition to another state
   without consuming any input. These are "free" jumps.

```
NFA = (Q, Σ, δ, q₀, F)

Same as DFA except:
  δ: Q × (Σ ∪ {ε}) → P(Q)
     "given a state and an input (or epsilon), return a SET of possible next states"
     P(Q) is the power set of Q — the set of all subsets of Q.
```

**Why are NFAs useful?** They are much easier to construct for certain problems.
For example, "does this string contain the substring `abc`?" is trivial as an
NFA but requires careful state management as a DFA.

**NFA for "contains abc":**

```
          any    a      b      c
   ┌────┐ ───→ ┌──┐ ──→ ┌──┐ ──→ ┌──┐
   │ q0 │      │q1│     │q2│     │q3│ (accepting)
   └────┘      └──┘     └──┘     └──┘
      │  ▲
      └──┘  any (self-loop: stay in q0 while looking for 'a')

The NFA non-deterministically guesses when the substring starts:
  - In q0, on input 'a', it goes to BOTH q0 AND q1 (two transitions!)
  - If the guess is wrong (q1 sees something other than 'b'), that path dies
  - If the guess is right, it reaches q3 and accepts
```

The "parallel universes" analogy: imagine the NFA spawns a clone of itself at
each non-deterministic choice. All clones run in parallel. If ANY clone reaches
an accepting state, the whole NFA accepts. Clones in dead-end states simply
vanish.

### Subset Construction: NFA → DFA

Here's the remarkable theorem: **every NFA can be converted to an equivalent
DFA.** The algorithm is called **subset construction** (or the powerset
construction).

The key insight: if an NFA can be in states {q0, q1, q3} simultaneously, we
create a single DFA state that represents that entire set. The DFA states are
sets of NFA states.

```
Algorithm: Subset Construction

Input:  NFA = (Q_N, Σ, δ_N, q₀, F_N)
Output: DFA = (Q_D, Σ, δ_D, d₀, F_D)

1. d₀ = ε-closure({q₀})
   (Start with the initial state and all states reachable via epsilon)

2. Q_D = {d₀}
   worklist = [d₀]

3. While worklist is not empty:
     D = worklist.pop()
     For each input symbol a ∈ Σ:
       T = {}
       For each NFA state q ∈ D:
         T = T ∪ δ_N(q, a)
       D' = ε-closure(T)
       δ_D(D, a) = D'
       If D' ∉ Q_D:
         Q_D = Q_D ∪ {D'}
         worklist.append(D')

4. F_D = {D ∈ Q_D | D ∩ F_N ≠ ∅}
   (A DFA state is accepting if it contains any NFA accepting state)

ε-closure(S): starting from set S, follow all epsilon transitions
              and return the full set of reachable states.
```

**Why this matters:** Regular expressions compile to NFAs (easy to construct).
NFAs convert to DFAs (this algorithm). DFAs execute in O(1) per input symbol.
This is how regex engines work.

**Trade-off:** The DFA can have up to 2^n states for an NFA with n states (one
DFA state per subset of NFA states). In practice, the blowup is usually modest,
and DFA minimization can reduce the state count further.

### DFA Minimization (Hopcroft's Algorithm)

Two DFA states are **equivalent** if, for every possible input sequence, they
either both accept or both reject. Equivalent states can be merged.

```
Algorithm: Hopcroft's DFA Minimization

1. Start with two groups: {accepting states} and {non-accepting states}
2. Repeat:
     For each group G and each input symbol a:
       Split G into subgroups based on where states in G go on input a.
       States that transition to different groups on the same input are
       NOT equivalent and must be in different subgroups.
   Until no group can be split further.
3. Each remaining group becomes a single state in the minimized DFA.
```

**Educational value:** This algorithm teaches the concept of **state
equivalence** — the idea that different-looking states can be fundamentally the
same because they behave identically. This concept appears throughout CS: alpha
equivalence in lambda calculus, bisimulation in process algebras, observational
equivalence in programming language theory.

### Pushdown Automata: Adding Memory

A DFA has no memory beyond its current state. This is exactly why it cannot
match balanced parentheses: it would need to remember how many open-parens it
has seen, and that count is unbounded.

A **Pushdown Automaton (PDA)** adds a **stack** to the finite automaton. The
stack gives the machine unbounded memory, but with a restriction: it can only
look at (and modify) the top of the stack.

```
PDA = (Q, Σ, Γ, δ, q₀, Z₀, F)

where:
  Q  = finite set of states
  Σ  = input alphabet
  Γ  = stack alphabet (may differ from Σ)
  δ  = transition function: Q × (Σ ∪ {ε}) × Γ → P(Q × Γ*)
       "given state, input (or ε), and stack top: return new state
        and what to push onto the stack"
  q₀ = initial state
  Z₀ = initial stack symbol (bottom-of-stack marker)
  F  = accepting states
```

**PDA for balanced parentheses:**

```
States: {q0, q_accept}
Input alphabet: { (, ) }
Stack alphabet: { (, $ }   ($ is the bottom-of-stack marker)

Transitions:
  (q0, '(', $) → (q0, push '(' then '$')    ; open paren on empty: push
  (q0, '(', '(') → (q0, push '(' then '(')  ; open paren on open: push
  (q0, ')', '(') → (q0, pop)                 ; close paren: pop matching open
  (q0, ε, $) → (q_accept, pop)               ; empty input, empty stack: accept

Trace for "(())" :
  State  Input  Stack (top→)   Action
  q0     (      $              push '(' → stack: ( $
  q0     (      ( $            push '(' → stack: ( ( $
  q0     )      ( ( $          pop '('  → stack: ( $
  q0     )      ( $            pop '('  → stack: $
  q0     ε      $              accept!  → q_accept

Trace for "(()" — unbalanced:
  q0     (      $              push '(' → stack: ( $
  q0     (      ( $            push '(' → stack: ( ( $
  q0     )      ( ( $          pop '('  → stack: ( $
  q0     ε      ( $            stuck! stack not empty, no matching transition
                                → REJECT ✓ (unbalanced)
```

**Why PDAs matter:** A DFA recognizes **regular languages** (the simplest class
in the Chomsky hierarchy). A PDA recognizes **context-free languages** — a
strictly more powerful class that includes nested structures like parentheses,
HTML tags, and programming language syntax. PDAs are the theoretical foundation
of parsers.

### Modal State Machines: Switching Between Sub-Machines

Sometimes a system's behavior changes modes: a car shifts between Drive,
Reverse, and Park; a text editor switches between Normal, Insert, and Visual
mode; an HTML tokenizer switches between Data, Tag, and Script modes.

A **Modal State Machine** is a collection of sub-machines (each a DFA) with
transitions between modes. When a mode-switch event occurs, the current
sub-machine pauses and a different sub-machine takes over.

```
ModalStateMachine = (M, μ, m₀)

where:
  M  = a set of named sub-machines (each a DFA)
  μ  = mode transition function: ModeName × Event → ModeName
       "given the current mode and a trigger event, switch to a new mode"
  m₀ = the initial mode
```

**HTML Tokenizer as a Modal State Machine:**

```
Modes:
  ┌──────────┐          ┌───────────┐          ┌──────────────┐
  │   DATA   │ ──────→  │ TAG_OPEN  │ ──────→  │ SCRIPT_DATA  │
  │          │ see '<'  │           │ see      │              │
  │ Normal   │          │ Inside    │ 'script' │ Raw text     │
  │ text and │ ◄──────  │ < ... >   │          │ until        │
  │ entities │ see '>'  │           │          │ </script>    │
  └──────────┘          └───────────┘          └──────────────┘
       ▲                      │                       │
       │                      │ see 'style'           │
       │                      ▼                       │
       │                ┌──────────────┐              │
       │                │  STYLE_DATA  │              │
       │                │              │              │
       │                │ Raw text     │              │
       └────────────────│ until        │◄─────────────┘
          see end tag   │ </style>     │  see end tag
                        └──────────────┘

Each mode has its own DFA that defines how tokens are recognized within
that context. The DATA mode tokenizes normal HTML text and entities. The
TAG_OPEN mode tokenizes tag names, attribute names, and attribute values.
The SCRIPT_DATA mode reads raw characters until it sees </script>.

This is why HTML cannot be tokenized with a single set of token rules:
the same characters mean different things in different modes. The '>'
character is a tag closer in TAG_OPEN mode but literal text in DATA mode
(after a comparison operator, for example).
```

**Why this matters for the browser project:** The existing `grammar-tools`
package defines token rules in `.tokens` files. A single `.tokens` file assumes
one set of rules applies everywhere. Modal state machines allow the lexer to
have **multiple `.tokens` rule sets** and switch between them based on context.
This is what unlocks HTML, Markdown, and other context-sensitive tokenization.

### The Chomsky Hierarchy

State machines fit into a beautiful hierarchy of computational power:

```
Type 3: Regular Languages
  Recognized by: DFA / NFA
  Example: email address pattern, the traffic light, binary div-by-3
  Power: cannot count (can't match balanced parens)
  Defined by: regular expressions, .tokens files

Type 2: Context-Free Languages
  Recognized by: PDA (pushdown automaton)
  Example: balanced parentheses, arithmetic expressions, CSS
  Power: can count with a stack (but only one stack)
  Defined by: context-free grammars, .grammar files

Type 1: Context-Sensitive Languages
  Recognized by: Linear-bounded automaton
  Example: HTML (implicit tag closing), Markdown (flanking delimiters)
  Power: can have multiple "stacks" or bounded tape
  Defined by: modal machines, parser actions

Type 0: Recursively Enumerable Languages
  Recognized by: Turing machine
  Example: LaTeX (macro expansion makes grammar change during parsing)
  Power: unlimited computation
  Defined by: arbitrary programs
```

This library provides tools for Type 3 (DFA, NFA), Type 2 (PDA), and a
practical approach to Type 1 (Modal State Machine). Each level strictly
includes the ones below it.

### Connection to Existing Code

State machines are already implicit throughout this codebase:

**Branch Predictor (D02):** The 2-bit saturating counter is a DFA with 4
states. It is currently implemented as a `TwoBitState` enum with transition
methods. With this library, it can be expressed declaratively:

```python
two_bit = DFA(
    states={"SNT", "WNT", "WT", "ST"},
    alphabet={"taken", "not_taken"},
    transitions={
        ("SNT", "taken"): "WNT", ("SNT", "not_taken"): "SNT",
        ("WNT", "taken"): "WT",  ("WNT", "not_taken"): "SNT",
        ("WT", "taken"): "ST",   ("WT", "not_taken"): "WNT",
        ("ST", "taken"): "ST",   ("ST", "not_taken"): "WT",
    },
    initial="WNT",
    accepting={"WT", "ST"},  # states that predict "taken"
)
```

**Sequential Logic (10):** SR latches, D flip-flops — these are hardware state
machines where the inputs are electrical signals and the states are voltage
levels. The convergence loops in the current implementation simulate the
physical feedback that keeps a latch in its current state.

**CPU Pipeline (D04):** The fetch-decode-execute cycle is a linear state
machine: FETCH → DECODE → EXECUTE → (repeat). Pipeline hazards and stalls add
transitions back to earlier states.

**Lexer (02):** The tokenizer's dispatch logic (`_read_number`, `_read_name`,
`_read_string`) is an implicit DFA where the current character class determines
the transition. A grammar-driven lexer generator compiles `.tokens` rules into
explicit DFAs.

## Public API

### Core Types

```python
from dataclasses import dataclass
from typing import Callable

# Type aliases for clarity
State = str
Event = str
Action = Callable[[str, str, str], None]  # (from_state, event, to_state) → side effect

@dataclass(frozen=True)
class TransitionRecord:
    """One step in a machine's execution trace.

    Every transition is logged as a TransitionRecord, giving complete
    visibility into the machine's execution history. This is the foundation
    of the library's traceability: you can replay any execution step by step.
    """
    source: State        # State before the transition
    event: Event | None  # Input that triggered it (None for epsilon transitions)
    target: State        # State after the transition
    action_name: str | None = None  # Name of the action executed, if any
```

### DFA

```python
class DFA:
    """
    Deterministic Finite Automaton — the workhorse of state machines.

    A DFA is defined by:
    - A finite set of states
    - A finite alphabet of input symbols
    - A transition function mapping (state, input) → next state
    - An initial state
    - A set of accepting (final) states
    - Optional actions executed on transitions

    The DFA is always in exactly one state. Each input causes exactly one
    transition. If no transition is defined for the current (state, input)
    pair, the DFA raises an error (it does not silently ignore inputs).
    """

    def __init__(
        self,
        states: set[str],
        alphabet: set[str],
        transitions: dict[tuple[str, str], str],
        initial: str,
        accepting: set[str],
        actions: dict[tuple[str, str], Action] | None = None,
    ) -> None: ...

    @property
    def current_state(self) -> str:
        """The state the machine is currently in."""
        ...

    def process(self, event: str) -> str:
        """
        Process a single input event.

        Looks up the transition for (current_state, event), moves to the
        target state, executes the action if one is defined, and returns
        the new current state.

        Raises ValueError if no transition exists for (current_state, event).
        """
        ...

    def process_sequence(self, events: list[str]) -> list[TransitionRecord]:
        """
        Process a sequence of inputs and return the full trace.

        Each transition is recorded as a TransitionRecord. The machine's
        state is updated after each input.
        """
        ...

    def accepts(self, events: list[str]) -> bool:
        """
        Process the input sequence and return True if the machine ends in
        an accepting state.

        Does NOT modify the machine's current state — runs on a copy.
        """
        ...

    def reset(self) -> None:
        """Reset the machine to its initial state."""
        ...

    # --- Introspection ---

    def reachable_states(self) -> set[str]:
        """Return the set of states reachable from the initial state."""
        ...

    def is_complete(self) -> bool:
        """
        Return True if a transition is defined for every (state, input) pair.

        A complete DFA never gets "stuck." Many textbook DFAs are complete;
        practical DFAs often omit transitions to an implicit "dead" state.
        """
        ...

    def validate(self) -> list[str]:
        """
        Check for common issues: unreachable states, missing transitions,
        initial state not in states set, accepting states not in states set.
        Returns a list of warning messages (empty if no issues).
        """
        ...

    # --- Visualization ---

    def to_dot(self) -> str:
        """
        Return a Graphviz DOT representation of the state machine.

        Accepting states are drawn as double circles. The initial state
        has an arrow pointing to it from nowhere. Transitions are labeled
        edges.
        """
        ...

    def to_ascii(self) -> str:
        """
        Return an ASCII transition table.

        Example:
                  │ coin     │ push
        ─────────┼──────────┼──────────
        locked   │ unlocked │ locked
        unlocked │ unlocked │ locked
        """
        ...

    def to_table(self) -> list[list[str]]:
        """
        Return the transition table as a list of rows.
        First row is the header: ["State", input1, input2, ...].
        Subsequent rows: [state_name, target1, target2, ...].
        """
        ...
```

### NFA

```python
EPSILON: str = ""  # Sentinel value for epsilon transitions

class NFA:
    """
    Non-deterministic Finite Automaton.

    Like a DFA, but with two additional capabilities:
    1. Multiple transitions: (state, input) can map to a SET of target states
    2. Epsilon transitions: transitions that occur without consuming input

    An NFA accepts an input sequence if there EXISTS at least one path
    through the machine that ends in an accepting state. Conceptually,
    the NFA explores all possible paths simultaneously.
    """

    def __init__(
        self,
        states: set[str],
        alphabet: set[str],
        transitions: dict[tuple[str, str], set[str]],
        initial: str,
        accepting: set[str],
    ) -> None: ...

    @property
    def current_states(self) -> frozenset[str]:
        """The set of states the NFA is currently in (after epsilon closure)."""
        ...

    def epsilon_closure(self, states: set[str]) -> frozenset[str]:
        """
        Compute the epsilon closure of a set of states.

        Starting from the given states, follow all epsilon transitions
        and return the full set of reachable states. This is a BFS/DFS
        over epsilon edges.
        """
        ...

    def process(self, event: str) -> frozenset[str]:
        """
        Process one input event.

        For each current state, find all transitions on this event,
        then compute the epsilon closure of the resulting states.
        Returns the new set of current states.
        """
        ...

    def process_sequence(self, events: list[str]) -> list[tuple[frozenset[str], str | None, frozenset[str]]]:
        """
        Process a sequence of inputs and return the trace.

        Each entry: (states_before, event, states_after).
        """
        ...

    def accepts(self, events: list[str]) -> bool:
        """
        Return True if the NFA accepts the input sequence.

        The NFA accepts if ANY of the current states after processing
        is an accepting state.
        """
        ...

    def reset(self) -> None:
        """Reset to initial state (with epsilon closure)."""
        ...

    # --- Conversion ---

    def to_dfa(self) -> DFA:
        """
        Convert this NFA to an equivalent DFA using subset construction.

        Each DFA state is a frozenset of NFA states. The resulting DFA
        recognizes exactly the same language as this NFA.

        The DFA state names are generated from sorted NFA state names,
        e.g., frozenset({"q0", "q1"}) becomes "{q0,q1}".
        """
        ...

    # --- Visualization ---

    def to_dot(self) -> str:
        """Graphviz DOT representation. Epsilon transitions labeled 'ε'."""
        ...
```

### DFA Minimization

```python
def minimize(dfa: DFA) -> DFA:
    """
    Minimize a DFA using Hopcroft's algorithm.

    Returns a new DFA with the minimum number of states that recognizes
    the same language. Unreachable states are removed. Equivalent states
    are merged.

    The minimized DFA is unique (up to state naming) — there is exactly
    one minimal DFA for any regular language.
    """
    ...
```

### Pushdown Automaton

```python
@dataclass(frozen=True)
class PDATransition:
    """
    A PDA transition rule.

    Reads: (source state, input symbol, top of stack)
    Writes: (target state, symbols to push onto stack)

    If event is None, this is an epsilon transition (no input consumed).
    If stack_push is empty, the stack top is popped (consumed).
    If stack_push is [X], the stack top is replaced with X.
    If stack_push is [X, Y], X is pushed first, then Y (Y ends on top).
    """
    source: str
    event: str | None       # Input symbol, or None for epsilon
    stack_read: str          # What must be on top of the stack
    target: str
    stack_push: list[str]    # What to push (empty = pop, [same] = keep)

@dataclass(frozen=True)
class PDATraceEntry:
    """One step in a PDA's execution trace."""
    source: str
    event: str | None
    stack_read: str
    target: str
    stack_push: list[str]
    stack_after: list[str]   # Full stack contents after the transition


class PushdownAutomaton:
    """
    Pushdown Automaton — a finite automaton augmented with a stack.

    The stack gives the PDA the ability to recognize context-free languages
    (like balanced parentheses), which finite automata cannot.
    """

    def __init__(
        self,
        states: set[str],
        input_alphabet: set[str],
        stack_alphabet: set[str],
        transitions: list[PDATransition],
        initial: str,
        initial_stack_symbol: str,
        accepting: set[str],
    ) -> None: ...

    @property
    def current_state(self) -> str: ...

    @property
    def stack(self) -> list[str]:
        """Current stack contents (top of stack is last element)."""
        ...

    def process(self, event: str) -> str:
        """Process one input symbol. Returns new state."""
        ...

    def accepts(self, events: list[str]) -> bool:
        """Return True if the PDA accepts the input sequence."""
        ...

    def process_sequence(self, events: list[str]) -> list[PDATraceEntry]:
        """Process inputs and return full trace including stack state."""
        ...

    def reset(self) -> None:
        """Reset to initial state with initial stack."""
        ...
```

### Modal State Machine

```python
class ModalStateMachine:
    """
    A collection of named sub-machines (modes) with transitions between them.

    Each mode is a DFA that handles inputs within that context. Mode
    transitions switch which DFA is active. When a mode switch occurs,
    the new mode's DFA starts from its initial state.

    This is the practical tool for context-sensitive tokenization:
    each mode has its own set of token rules, and certain tokens
    trigger mode switches.
    """

    def __init__(
        self,
        modes: dict[str, DFA],
        mode_transitions: dict[tuple[str, str], str],
        initial_mode: str,
    ) -> None: ...

    @property
    def current_mode(self) -> str:
        """The name of the currently active mode."""
        ...

    @property
    def active_machine(self) -> DFA:
        """The DFA for the current mode."""
        ...

    def process(self, event: str) -> str:
        """
        Process an input event in the current mode's DFA.

        If the event triggers a mode transition, switch modes first.
        Returns the current state of the active DFA after processing.
        """
        ...

    def switch_mode(self, trigger: str) -> str:
        """
        Explicitly switch modes based on a trigger event.

        Looks up (current_mode, trigger) in mode_transitions.
        Resets the new mode's DFA to its initial state.
        Returns the name of the new mode.
        """
        ...

    def reset(self) -> None:
        """Reset to initial mode, reset all sub-machines."""
        ...
```

## The `.states` File Format

Following the pattern of `.tokens` and `.grammar` files, state machines can be
defined declaratively in `.states` files. The `grammar-tools` package will be
extended with a `.states` parser.

**Canonical format note:** `F07-state-machine-markup-language.md` now defines
the implementation target for serialization and deserialization:
TOML-compatible `state-machine/v1` documents written as `.states.toml` or
`.states`. The examples below are the original educational sketch syntax and
should be treated as conceptual examples until they are migrated to F07.

### DFA Format

```
# Filename: turnstile.states
# A simple turnstile: insert coin to unlock, push to lock.

type: dfa

states:
    locked      [initial]
    unlocked    [accepting]

alphabet:
    coin
    push

transitions:
    locked,   coin  -> unlocked
    locked,   push  -> locked
    unlocked, coin  -> unlocked
    unlocked, push  -> locked
```

### NFA Format

```
# Filename: contains-abc.states
# Accepts strings containing "abc" as a substring.

type: nfa

states:
    q0  [initial]
    q1
    q2
    q3  [accepting]

alphabet:
    a
    b
    c

transitions:
    q0, a   -> q0       # stay in q0 (keep scanning)
    q0, b   -> q0
    q0, c   -> q0
    q0, a   -> q1       # non-deterministic: also try starting match
    q1, b   -> q2
    q2, c   -> q3
    q3, a   -> q3       # stay accepting (match found)
    q3, b   -> q3
    q3, c   -> q3
```

### PDA Format

```
# Filename: balanced-parens.states
# Accepts strings of balanced parentheses.

type: pda

states:
    q0      [initial]
    accept  [accepting]

input_alphabet:
    (
    )

stack_alphabet:
    (
    $

initial_stack: $

transitions:
    q0, (, $ -> q0, ( $     # open paren, empty stack: push
    q0, (, ( -> q0, ( (     # open paren, open on stack: push
    q0, ), ( -> q0,         # close paren: pop matching open
    q0, _, $ -> accept,     # end of input (_), stack empty: accept
```

### Modal Format

```
# Filename: html-tokenizer.states
# Simplified HTML tokenizer with mode switching.

type: modal

modes:
    data        [initial]   html-data.states
    tag_open                html-tag.states
    script_data             html-script.states
    style_data              html-style.states

mode_transitions:
    data,        open_tag     -> tag_open
    tag_open,    close_angle  -> data
    tag_open,    script_tag   -> script_data
    tag_open,    style_tag    -> style_data
    script_data, end_script   -> data
    style_data,  end_style    -> data
```

## Data Structures

### Internal Representation

```python
# DFA internal state
@dataclass
class DFAState:
    states: set[str]
    alphabet: set[str]
    transitions: dict[tuple[str, str], str]   # (state, event) → target
    initial: str
    accepting: set[str]
    current: str                               # mutable: tracks current state
    actions: dict[tuple[str, str], Action]
    trace: list[TransitionRecord]              # execution history

# NFA internal state
@dataclass
class NFAState:
    states: set[str]
    alphabet: set[str]
    transitions: dict[tuple[str, str], set[str]]  # (state, event) → {targets}
    initial: str
    accepting: set[str]
    current: frozenset[str]   # mutable: set of currently active states

# PDA internal state
@dataclass
class PDAInternalState:
    states: set[str]
    input_alphabet: set[str]
    stack_alphabet: set[str]
    transitions: list[PDATransition]
    initial: str
    initial_stack_symbol: str
    accepting: set[str]
    current: str              # mutable: current state
    stack: list[str]          # mutable: the stack (top = last element)
```

## Test Strategy

### DFA Tests

- **Construction:** valid DFA construction, reject invalid (initial not in
  states, accepting not subset of states, transition targets not in states)
- **Processing:** single event, event sequence, trace correctness
- **Acceptance:** accept/reject on various inputs, empty input (accept if
  initial is accepting)
- **Completeness:** `is_complete()` on complete and incomplete DFAs
- **Reachability:** `reachable_states()` finds all reachable, ignores unreachable
- **Validation:** `validate()` reports unreachable states, missing transitions
- **Reset:** `reset()` returns to initial state, clears trace
- **Actions:** actions fire on correct transitions with correct arguments
- **Visualization:** `to_dot()` produces valid DOT, `to_ascii()` readable table
- **Classic examples:**
  - Turnstile (2 states, 2 inputs)
  - Binary divisibility by 3 (3 states, 2 inputs)
  - 2-bit branch predictor (4 states, 2 inputs — verify equivalence with
    existing `branch_predictor.TwoBitState`)
- **Error cases:** undefined transition raises ValueError, invalid event raises
  ValueError

### NFA Tests

- **Epsilon closure:** single state, multiple states, chained epsilons, cycles
- **Processing:** deterministic subcase (NFA acting like DFA), non-deterministic
  branching, paths that die, paths that survive
- **Acceptance:** accept if any path accepts, reject if all paths reject
- **Subset construction:**
  - NFA that is already deterministic → equivalent DFA
  - NFA with epsilon transitions → DFA with no epsilons
  - "Contains abc" NFA → DFA that recognizes same language (test on many inputs)
  - Textbook examples with known DFA sizes
- **Visualization:** DOT output with epsilon edges labeled "ε"

### Minimization Tests

- **Already minimal:** minimize a minimal DFA → same number of states
- **Mergeable states:** DFA with equivalent states → fewer states
- **Unreachable removal:** DFA with unreachable states → removed in output
- **Language preservation:** original and minimized DFA accept same inputs
  (test on large random input sets)
- **NFA → DFA → minimize pipeline:** NFA → subset construction → minimize →
  verify language equivalence

### PDA Tests

- **Balanced parentheses:** `()`, `(())`, `((()))` accepted; `)`, `(()`, `)(` rejected
- **a^n b^n:** `ab`, `aabb`, `aaabbb` accepted; `aab`, `abb`, `ba` rejected
- **Stack inspection:** verify stack contents at each step via trace
- **Epsilon transitions:** PDA with epsilon moves
- **Error cases:** no matching transition (reject, don't crash)

### Modal State Machine Tests

- **Mode switching:** verify current mode changes on trigger events
- **Processing within modes:** events processed by correct mode's DFA
- **Simplified HTML modes:** DATA → TAG_OPEN → DATA cycle, DATA → TAG_OPEN →
  SCRIPT_DATA → DATA cycle
- **Reset:** all modes reset, initial mode restored
- **Invalid trigger:** trigger with no mode transition defined → error

## Future Extensions

- **`.states` file parser in grammar-tools:** Parse the canonical
  TOML-compatible `state-machine/v1` format from
  `F07-state-machine-markup-language.md` into DFA/NFA/PDA/Modal objects.
- **Lexer mode integration:** Extend the lexer to use `ModalStateMachine` for
  context-sensitive tokenization. Each mode maps to a `.tokens` file.
- **Transducer:** A state machine that produces output on transitions (Mealy
  machine) or in states (Moore machine). Useful for transliteration and encoding.
- **Regex → NFA compiler:** Compile regular expression syntax into NFA objects.
  Combined with NFA→DFA conversion, this gives a complete regex engine.
- **State machine composition:** Product construction (intersection), union,
  complement, concatenation of automata.
- **Infinite-state systems:** Timed automata, counter machines — extensions
  for modeling real-time systems and protocols.
- **Hardware synthesis:** Compile a DFA into logic gate representations using the
  existing `logic-gates` package — state encoded in flip-flops, transitions in
  combinational logic.
