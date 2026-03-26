"""State Machine Library — formal automata for parsing, protocols, and hardware.

=== What is this library? ===

This library provides composable, traceable state machines — the foundational
abstraction behind parsers, network protocols, hardware controllers, and much
more. It covers the full spectrum of automata theory:

- **DFA** (Deterministic Finite Automaton): exactly one transition per
  (state, input) pair. The simplest and most efficient machine.

- **NFA** (Non-deterministic Finite Automaton): multiple transitions allowed,
  including epsilon (empty) transitions. Easier to construct for complex
  patterns, can be converted to an equivalent DFA.

- **DFA Minimization**: Hopcroft's algorithm to find the smallest DFA that
  recognizes the same language.

- **PDA** (Pushdown Automaton): a finite automaton with a stack, capable of
  recognizing context-free languages like balanced parentheses and nested HTML.

- **Modal State Machine**: multiple sub-machines (modes) with transitions
  between them. The practical tool for context-sensitive tokenization (e.g.,
  an HTML tokenizer that switches modes for tags, scripts, and stylesheets).

Every transition in every machine type is traced, making the library ideal
for learning, debugging, and the coding-adventures philosophy of tracing
computations all the way to logic gates.
"""

from state_machine.dfa import DFA
from state_machine.minimize import minimize
from state_machine.modal import ModalStateMachine, ModeTransitionRecord
from state_machine.nfa import EPSILON, NFA
from state_machine.pda import PDATraceEntry, PDATransition, PushdownAutomaton
from state_machine.types import Action, Event, State, TransitionRecord

__all__ = [
    "Action",
    "DFA",
    "EPSILON",
    "Event",
    "ModalStateMachine",
    "ModeTransitionRecord",
    "NFA",
    "PDATraceEntry",
    "PDATransition",
    "PushdownAutomaton",
    "State",
    "TransitionRecord",
    "minimize",
]
