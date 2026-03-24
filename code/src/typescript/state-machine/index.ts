/**
 * State Machine Library — formal automata for parsing, protocols, and hardware.
 *
 * === What is this library? ===
 *
 * This library provides composable, traceable state machines — the foundational
 * abstraction behind parsers, network protocols, hardware controllers, and much
 * more. It covers the full spectrum of automata theory:
 *
 * - **DFA** (Deterministic Finite Automaton): exactly one transition per
 *   (state, input) pair. The simplest and most efficient machine.
 *
 * - **NFA** (Non-deterministic Finite Automaton): multiple transitions allowed,
 *   including epsilon (empty) transitions. Easier to construct for complex
 *   patterns, can be converted to an equivalent DFA.
 *
 * - **DFA Minimization**: Hopcroft's algorithm to find the smallest DFA that
 *   recognizes the same language.
 *
 * - **PDA** (Pushdown Automaton): a finite automaton with a stack, capable of
 *   recognizing context-free languages like balanced parentheses and nested HTML.
 *
 * - **Modal State Machine**: multiple sub-machines (modes) with transitions
 *   between them. The practical tool for context-sensitive tokenization (e.g.,
 *   an HTML tokenizer that switches modes for tags, scripts, and stylesheets).
 *
 * Every transition in every machine type is traced, making the library ideal
 * for learning, debugging, and the coding-adventures philosophy of tracing
 * computations all the way to logic gates.
 *
 * @module state-machine
 */

export { DFA } from "./dfa.js";
export { minimize } from "./minimize.js";
export { ModalStateMachine } from "./modal.js";
export type { ModeTransitionRecord } from "./modal.js";
export { EPSILON, NFA, stateSetName } from "./nfa.js";
export type { PDATraceEntry, PDATransition } from "./pda.js";
export { PushdownAutomaton } from "./pda.js";
export type { Action, Event, State, TransitionRecord } from "./types.js";
export { transitionKey } from "./types.js";
