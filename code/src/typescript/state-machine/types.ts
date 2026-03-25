/**
 * Core types shared by all state machine implementations.
 *
 * === The Building Blocks ===
 *
 * Every state machine — whether it is a simple traffic light controller or
 * a complex HTML tokenizer — is built from the same fundamental concepts:
 *
 * - **State**: where the machine is right now (e.g., "locked", "red", "q0")
 * - **Event**: what input the machine just received (e.g., "coin", "timer", "a")
 * - **Transition**: the rule that says "in state X, on event Y, go to state Z"
 * - **Action**: an optional side effect that fires when a transition occurs
 * - **TransitionRecord**: a logged entry capturing one step of execution
 *
 * These types are deliberately simple — strings and plain objects. This makes
 * state machines easy to define, serialize, and visualize. There are no
 * abstract base classes or complex hierarchies here.
 *
 * @module types
 */

// === Type Aliases ===
//
// States and events are just strings. We use type aliases for clarity
// in function signatures — when you see `State` in a type hint, you
// know it is a state name, not just any arbitrary string.
//
// Why strings and not enums? Strings are simpler to construct, serialize,
// and display. You can define a state machine in one line without first
// declaring an enum. For the same reason, the grammar-tools package
// uses strings for token names and grammar rule names.

/**
 * A named state in a state machine.
 *
 * @example "locked", "q0", "SNT"
 */
export type State = string;

/**
 * An input symbol that triggers a transition.
 *
 * @example "coin", "a", "taken"
 */
export type Event = string;

/**
 * A callback executed when a transition fires.
 *
 * The three arguments are: (source_state, event, target_state).
 *
 * Actions are optional side effects — logging, incrementing counters,
 * emitting tokens, etc. The state machine itself does not depend on
 * action return values; actions are fire-and-forget.
 *
 * @example
 * ```typescript
 * const logTransition: Action = (source, event, target) => {
 *   console.log(`${source} --${event}--> ${target}`);
 * };
 * ```
 */
export type Action = (source: string, event: string, target: string) => void;

/**
 * One step in a state machine's execution trace.
 *
 * Every time a machine processes an input and transitions from one state
 * to another, a TransitionRecord is created. This gives complete
 * visibility into the machine's execution history.
 *
 * === Why trace everything? ===
 *
 * In the coding-adventures philosophy, we want to be able to trace any
 * computation all the way down to the logic gates that implement it.
 * TransitionRecords are the state machine layer's contribution to that
 * trace: they record exactly what happened, when, and why.
 *
 * You can replay an execution by walking through its list of
 * TransitionRecords. You can verify correctness by checking that the
 * source of each record matches the target of the previous one. You
 * can visualize the execution path on a state diagram by highlighting
 * the edges that were traversed.
 *
 * === Fields ===
 *
 * - source: the state before the transition
 * - event: the input that triggered it (null for epsilon transitions)
 * - target: the state after the transition
 * - actionName: the name of the action that fired, if any
 *
 * @example
 * ```typescript
 * { source: "locked", event: "coin", target: "unlocked", actionName: null }
 * // "The machine was in 'locked', received 'coin', moved to 'unlocked'"
 *
 * { source: "q0", event: null, target: "q1", actionName: null }
 * // "Epsilon transition from q0 to q1 (no input consumed)"
 * ```
 */
export interface TransitionRecord {
  /** The state before the transition. */
  readonly source: State;
  /** The input that triggered it (null for epsilon transitions). */
  readonly event: Event | null;
  /** The state after the transition. */
  readonly target: State;
  /** The name of the action that fired, if any. */
  readonly actionName: string | null;
}

// === Transition Key Helper ===
//
// In TypeScript, Maps cannot use tuples as keys (object identity, not
// value equality). We encode (state, event) pairs as a single string
// using a null byte separator. The null byte (\0) is chosen because it
// cannot appear in normal state or event names, avoiding ambiguity.
//
// For example, the Python key ("locked", "coin") becomes "locked\0coin"
// in TypeScript. This is a common pattern when porting from languages
// with tuple-keyed dicts.

/**
 * Encode a (state, event) pair as a single string key for Map lookup.
 *
 * Uses null byte (\0) as separator to avoid ambiguity — no valid state
 * or event name should contain a null byte.
 *
 * @param state - The state part of the key
 * @param event - The event part of the key
 * @returns A string key like "locked\0coin"
 *
 * @example
 * ```typescript
 * transitionKey("locked", "coin")  // "locked\0coin"
 * transitionKey("q0", "a")         // "q0\0a"
 * ```
 */
export function transitionKey(state: string, event: string): string {
  return `${state}\0${event}`;
}
