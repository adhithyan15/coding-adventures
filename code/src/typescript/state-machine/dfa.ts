/**
 * Deterministic Finite Automaton (DFA) — the workhorse of state machines.
 *
 * === What is a DFA? ===
 *
 * A DFA is the simplest kind of state machine. It has a fixed set of states,
 * reads input symbols one at a time, and follows exactly one transition for
 * each (state, input) pair. There is no ambiguity, no guessing, no backtracking.
 *
 * Formally, a DFA is a 5-tuple (Q, Sigma, delta, q0, F):
 *
 *     Q  = a finite set of states
 *     Sigma = a finite set of input symbols (the "alphabet")
 *     delta = a transition function: Q x Sigma -> Q
 *     q0 = the initial state (q0 in Q)
 *     F  = a set of accepting/final states (F is a subset of Q)
 *
 * === Why "deterministic"? ===
 *
 * "Deterministic" means there is exactly ONE next state for every (state, input)
 * combination. Given the same starting state and the same input sequence, a DFA
 * always follows the same path and reaches the same final state. This makes DFAs
 * predictable, efficient, and easy to implement in hardware — which is why they
 * appear everywhere from CPU branch predictors to network protocol handlers.
 *
 * === Example: a turnstile ===
 *
 * A turnstile at a subway station has two states: locked and unlocked.
 * Insert a coin -> it unlocks. Push the arm -> it locks.
 *
 *     States:      {locked, unlocked}
 *     Alphabet:    {coin, push}
 *     Transitions: (locked, coin) -> unlocked
 *                  (locked, push) -> locked
 *                  (unlocked, coin) -> unlocked
 *                  (unlocked, push) -> locked
 *     Initial:     locked
 *     Accepting:   {unlocked}
 *
 * This DFA answers the question: "after this sequence of coin/push events,
 * is the turnstile unlocked?"
 *
 * === Connection to existing code ===
 *
 * The 2-bit branch predictor in the branch-predictor package (D02) is a DFA:
 *
 *     States:      {SNT, WNT, WT, ST}  (strongly/weakly not-taken/taken)
 *     Alphabet:    {taken, not_taken}
 *     Transitions: defined by the saturating counter logic
 *     Initial:     WNT
 *     Accepting:   {WT, ST}  (states that predict "taken")
 *
 * @module dfa
 */

import { LabeledDirectedGraph } from "../directed-graph/index.js";
import type { Action, TransitionRecord } from "./types.js";
import { transitionKey } from "./types.js";

/**
 * Deterministic Finite Automaton.
 *
 * A DFA is always in exactly one state. Each input causes exactly one
 * transition. If no transition is defined for the current (state, input)
 * pair, processing that input throws an error.
 *
 * All transitions are traced via TransitionRecord objects, providing
 * complete execution history for debugging and visualization.
 *
 * @example
 * ```typescript
 * const turnstile = new DFA(
 *   new Set(["locked", "unlocked"]),
 *   new Set(["coin", "push"]),
 *   new Map([
 *     ["locked\0coin", "unlocked"],
 *     ["locked\0push", "locked"],
 *     ["unlocked\0coin", "unlocked"],
 *     ["unlocked\0push", "locked"],
 *   ]),
 *   "locked",
 *   new Set(["unlocked"]),
 * );
 * turnstile.process("coin"); // "unlocked"
 * turnstile.accepts(["coin", "push", "coin"]); // true
 * ```
 */
export class DFA {
  // === Internal State ===
  //
  // We store the 5-tuple as private readonly fields, plus mutable
  // execution state (_current and _trace). The readonly fields use
  // ReadonlySet to prevent mutation of the DFA's definition.

  private readonly _states: ReadonlySet<string>;
  private readonly _alphabet: ReadonlySet<string>;
  private readonly _transitions: Map<string, string>;
  private readonly _initial: string;
  private readonly _accepting: ReadonlySet<string>;
  private readonly _actions: Map<string, Action>;

  // --- Internal graph representation ---
  //
  // We maintain a LabeledDirectedGraph alongside the _transitions Map.
  // The Map provides O(1) lookups for process() (the hot path).
  // The graph provides structural queries like reachableStates() via
  // transitiveClosure, avoiding the need for hand-rolled BFS.
  //
  // Each state becomes a node. Each transition (source, event) -> target
  // becomes a labeled edge from source to target with the event as label.
  private readonly _graph: LabeledDirectedGraph;

  /** The state the machine is currently in. */
  private _current: string;
  /** The execution trace — a list of all transitions taken so far. */
  private _trace: TransitionRecord[];

  // === Construction ===
  //
  // We validate all inputs eagerly in the constructor so that errors are
  // caught at definition time, not at runtime when the machine processes
  // its first input. This is the "fail fast" principle.

  /**
   * Create a new DFA.
   *
   * @param states - The finite set of states. Must be non-empty.
   * @param alphabet - The finite set of input symbols. Must be non-empty.
   * @param transitions - Map from encoded (state, event) keys to target state.
   *   Use {@link transitionKey} to create keys: `transitionKey("locked", "coin")`.
   *   Every target must be in `states`. Not every (state, event) pair needs a
   *   transition -- missing transitions cause errors at processing time.
   * @param initial - The starting state. Must be in `states`.
   * @param accepting - The set of accepting/final states. Must be a subset
   *   of `states`. Can be empty (the machine never accepts).
   * @param actions - Optional map from encoded (state, event) keys to a callback
   *   function that fires when that transition occurs.
   *
   * @throws Error if any validation check fails.
   */
  constructor(
    states: Set<string>,
    alphabet: Set<string>,
    transitions: Map<string, string>,
    initial: string,
    accepting: Set<string>,
    actions?: Map<string, Action>,
  ) {
    // --- Validate states ---
    if (states.size === 0) {
      throw new Error("States set must be non-empty");
    }

    // --- Validate initial state ---
    if (!states.has(initial)) {
      throw new Error(
        `Initial state '${initial}' is not in the states set [${[...states].sort().join(", ")}]`,
      );
    }

    // --- Validate accepting states ---
    for (const s of accepting) {
      if (!states.has(s)) {
        throw new Error(
          `Accepting state '${s}' is not in the states set [${[...states].sort().join(", ")}]`,
        );
      }
    }

    // --- Validate transitions ---
    //
    // Every transition must go FROM a known state ON a known event
    // TO a known state. We check all three.
    for (const [key, target] of transitions) {
      const sep = key.indexOf("\0");
      const source = key.substring(0, sep);
      const event = key.substring(sep + 1);

      if (!states.has(source)) {
        throw new Error(
          `Transition source '${source}' is not in the states set`,
        );
      }
      if (!alphabet.has(event)) {
        throw new Error(
          `Transition event '${event}' is not in the alphabet [${[...alphabet].sort().join(", ")}]`,
        );
      }
      if (!states.has(target)) {
        throw new Error(
          `Transition target '${target}' (from (${source}, ${event})) is not in the states set`,
        );
      }
    }

    // --- Validate actions ---
    if (actions) {
      for (const key of actions.keys()) {
        if (!transitions.has(key)) {
          const sep = key.indexOf("\0");
          const source = key.substring(0, sep);
          const event = key.substring(sep + 1);
          throw new Error(
            `Action defined for (${source}, ${event}) but no transition exists for that pair`,
          );
        }
      }
    }

    // --- Store the 5-tuple + extras ---
    this._states = new Set(states);
    this._alphabet = new Set(alphabet);
    this._transitions = new Map(transitions);
    this._initial = initial;
    this._accepting = new Set(accepting);
    this._actions = new Map(actions ?? []);

    // --- Build internal graph representation ---
    //
    // We build a LabeledDirectedGraph from states and transitions so that
    // structural queries like reachableStates() can delegate to the graph's
    // transitiveClosure algorithm instead of hand-rolling BFS.
    this._graph = new LabeledDirectedGraph();
    for (const state of states) {
      this._graph.addNode(state);
    }
    for (const [key, target] of transitions) {
      const sep = key.indexOf("\0");
      const source = key.substring(0, sep);
      const event = key.substring(sep + 1);
      this._graph.addEdge(source, target, event);
    }

    // --- Mutable execution state ---
    this._current = initial;
    this._trace = [];
  }

  // === Properties ===

  /** The finite set of states. */
  get states(): ReadonlySet<string> {
    return this._states;
  }

  /** The finite set of input symbols. */
  get alphabet(): ReadonlySet<string> {
    return this._alphabet;
  }

  /**
   * The transition function as a new Map (copy).
   *
   * Returns a copy so callers cannot mutate the internal transitions.
   */
  get transitions(): Map<string, string> {
    return new Map(this._transitions);
  }

  /** The initial state. */
  get initial(): string {
    return this._initial;
  }

  /** The set of accepting/final states. */
  get accepting(): ReadonlySet<string> {
    return this._accepting;
  }

  /** The state the machine is currently in. */
  get currentState(): string {
    return this._current;
  }

  /**
   * The execution trace — a list of all transitions taken so far.
   *
   * Returns a copy so callers cannot mutate the internal trace.
   */
  get trace(): TransitionRecord[] {
    return [...this._trace];
  }

  // === Processing ===

  /**
   * Process a single input event and return the new state.
   *
   * Looks up the transition for (currentState, event), moves to the
   * target state, executes the action (if defined), logs a
   * TransitionRecord, and returns the new current state.
   *
   * @param event - An input symbol from the alphabet.
   * @returns The new current state after the transition.
   * @throws Error if the event is not in the alphabet, or if no
   *   transition is defined for (currentState, event).
   *
   * @example
   * ```typescript
   * const m = new DFA(
   *   new Set(["a","b"]), new Set(["x"]),
   *   new Map([["a\0x","b"], ["b\0x","a"]]),
   *   "a", new Set(["b"]),
   * );
   * m.process("x"); // "b"
   * m.currentState;  // "b"
   * ```
   */
  process(event: string): string {
    // Validate the event
    if (!this._alphabet.has(event)) {
      throw new Error(
        `Event '${event}' is not in the alphabet [${[...this._alphabet].sort().join(", ")}]`,
      );
    }

    // Look up the transition
    const key = transitionKey(this._current, event);
    const target = this._transitions.get(key);
    if (target === undefined) {
      throw new Error(
        `No transition defined for (state='${this._current}', event='${event}')`,
      );
    }

    // Execute the action if one exists
    let actionName: string | null = null;
    const action = this._actions.get(key);
    if (action) {
      action(this._current, event, target);
      actionName = action.name || String(action);
    }

    // Log the transition
    const record: TransitionRecord = {
      source: this._current,
      event,
      target,
      actionName,
    };
    this._trace.push(record);

    // Move to the new state
    this._current = target;
    return target;
  }

  /**
   * Process a sequence of inputs and return the trace.
   *
   * Each input is processed in order. The full trace of transitions
   * is returned. The machine's state is updated after each input.
   *
   * @param events - A list of input symbols.
   * @returns A list of TransitionRecord objects, one per input.
   *
   * @example
   * ```typescript
   * const trace = m.processSequence(["x", "x", "x"]);
   * trace.map(t => [t.source, t.target]);
   * // [["a", "b"], ["b", "a"], ["a", "b"]]
   * ```
   */
  processSequence(events: string[]): TransitionRecord[] {
    const traceStart = this._trace.length;
    for (const event of events) {
      this.process(event);
    }
    return this._trace.slice(traceStart);
  }

  /**
   * Check if the machine accepts the input sequence.
   *
   * Processes the entire sequence and returns true if the machine
   * ends in an accepting state.
   *
   * IMPORTANT: This method does NOT modify the machine's current state
   * or trace. It runs on a fresh simulation starting from the initial state.
   *
   * @param events - A list of input symbols.
   * @returns True if the machine ends in an accepting state after
   *   processing all inputs, false otherwise.
   *
   * @example
   * ```typescript
   * turnstile.accepts(["coin"]);            // true
   * turnstile.accepts(["coin", "push"]);    // false
   * turnstile.accepts([]);                  // false (initial is not accepting)
   * ```
   */
  accepts(events: string[]): boolean {
    // Run on a copy so we don't modify this machine's state
    let state = this._initial;
    for (const event of events) {
      if (!this._alphabet.has(event)) {
        throw new Error(
          `Event '${event}' is not in the alphabet [${[...this._alphabet].sort().join(", ")}]`,
        );
      }
      const key = transitionKey(state, event);
      const target = this._transitions.get(key);
      if (target === undefined) {
        return false;
      }
      state = target;
    }
    return this._accepting.has(state);
  }

  /**
   * Reset the machine to its initial state and clear the trace.
   *
   * After reset, the machine is in the same state as when it was
   * first constructed — as if no inputs had ever been processed.
   */
  reset(): void {
    this._current = this._initial;
    this._trace = [];
  }

  // === Introspection ===
  //
  // These methods analyze the structure of the DFA itself, not its
  // execution. They answer questions like "is the DFA well-formed?"
  // and "which states can actually be reached?"

  /**
   * Return the set of states reachable from the initial state.
   *
   * Delegates to the internal LabeledDirectedGraph's transitiveClosure,
   * which performs a BFS over the transition graph. A state is reachable
   * if there exists any sequence of inputs that leads from the initial
   * state to that state.
   *
   * States that are defined but not reachable are "dead weight" —
   * they can never be entered and can be safely removed during
   * minimization.
   *
   * @returns A Set of reachable state names.
   */
  reachableStates(): Set<string> {
    // transitiveClosure returns all nodes reachable FROM the initial
    // state (not including the initial state itself), so we union it
    // with {initial} to get the full set of reachable states.
    const reachable = this._graph.transitiveClosure(this._initial);
    reachable.add(this._initial);
    return reachable;
  }

  /**
   * Check if a transition is defined for every (state, input) pair.
   *
   * A complete DFA never gets "stuck" — every state handles every
   * input. Textbook DFAs are usually complete (missing transitions
   * go to an explicit "dead" or "trap" state). Practical DFAs often
   * omit transitions to save space, treating missing transitions as
   * errors.
   *
   * @returns True if every (state, event) pair has a defined transition.
   */
  isComplete(): boolean {
    for (const state of this._states) {
      for (const event of this._alphabet) {
        if (!this._transitions.has(transitionKey(state, event))) {
          return false;
        }
      }
    }
    return true;
  }

  /**
   * Check for common issues and return a list of warnings.
   *
   * Checks performed:
   * - Unreachable states (defined but never entered)
   * - Missing transitions (incomplete DFA)
   * - Accepting states that are unreachable
   *
   * @returns A list of warning messages. Empty if no issues found.
   */
  validate(): string[] {
    const warnings: string[] = [];

    // Check for unreachable states
    const reachable = this.reachableStates();
    const unreachable: string[] = [];
    for (const s of this._states) {
      if (!reachable.has(s)) {
        unreachable.push(s);
      }
    }
    if (unreachable.length > 0) {
      warnings.push(`Unreachable states: [${unreachable.sort().join(", ")}]`);
    }

    // Check for unreachable accepting states
    const unreachableAccepting: string[] = [];
    for (const s of this._accepting) {
      if (!reachable.has(s)) {
        unreachableAccepting.push(s);
      }
    }
    if (unreachableAccepting.length > 0) {
      warnings.push(
        `Unreachable accepting states: [${unreachableAccepting.sort().join(", ")}]`,
      );
    }

    // Check for missing transitions
    const missing: string[] = [];
    for (const state of [...this._states].sort()) {
      for (const event of [...this._alphabet].sort()) {
        if (!this._transitions.has(transitionKey(state, event))) {
          missing.push(`(${state}, ${event})`);
        }
      }
    }
    if (missing.length > 0) {
      warnings.push(`Missing transitions: ${missing.join(", ")}`);
    }

    return warnings;
  }

  // === Visualization ===

  /**
   * Return a Graphviz DOT representation of this DFA.
   *
   * Accepting states are drawn as double circles (doublecircle shape).
   * The initial state has an invisible node pointing to it (the
   * standard convention for marking the start state in automata diagrams).
   *
   * The output can be rendered with:
   *     dot -Tpng machine.dot -o machine.png
   *
   * @returns A string in DOT format.
   */
  toDot(): string {
    const lines: string[] = [];
    lines.push("digraph DFA {");
    lines.push("    rankdir=LR;");
    lines.push("");

    // Invisible start node pointing to initial state
    lines.push("    __start [shape=point, width=0.2];");
    lines.push(`    __start -> "${this._initial}";`);
    lines.push("");

    // Accepting states get double circles
    for (const state of [...this._states].sort()) {
      const shape = this._accepting.has(state) ? "doublecircle" : "circle";
      lines.push(`    "${state}" [shape=${shape}];`);
    }
    lines.push("");

    // Transitions as labeled edges
    // Group transitions with same source and target to combine labels
    const edgeLabels = new Map<string, string[]>();
    const sortedEntries = [...this._transitions.entries()].sort(
      ([a], [b]) => (a < b ? -1 : a > b ? 1 : 0),
    );

    for (const [key, target] of sortedEntries) {
      const sep = key.indexOf("\0");
      const source = key.substring(0, sep);
      const event = key.substring(sep + 1);
      const edgeKey = `${source}\0${target}`;
      if (!edgeLabels.has(edgeKey)) {
        edgeLabels.set(edgeKey, []);
      }
      edgeLabels.get(edgeKey)!.push(event);
    }

    const sortedEdges = [...edgeLabels.entries()].sort(
      ([a], [b]) => (a < b ? -1 : a > b ? 1 : 0),
    );
    for (const [edgeKey, labels] of sortedEdges) {
      const sep = edgeKey.indexOf("\0");
      const source = edgeKey.substring(0, sep);
      const target = edgeKey.substring(sep + 1);
      const label = labels.sort().join(", ");
      lines.push(`    "${source}" -> "${target}" [label="${label}"];`);
    }

    lines.push("}");
    return lines.join("\n");
  }

  /**
   * Return an ASCII transition table.
   *
   * Accepting states are marked with (*). The initial state is
   * marked with (>).
   *
   * @example
   * ```
   *           | coin     | push
   * ---------+----------+----------
   * locked   | unlocked | locked
   * unlocked | unlocked | locked
   * ```
   *
   * @returns A formatted ASCII table string.
   */
  toAscii(): string {
    const sortedEvents = [...this._alphabet].sort();
    const sortedStates = [...this._states].sort();

    // Calculate column widths
    const stateWidth = Math.max(
      ...sortedStates.map((s) => s.length + 4), // +4 for markers
    );
    const eventWidth = Math.max(
      5, // minimum column width
      ...sortedEvents.map((e) => e.length),
      ...sortedStates.flatMap((s) =>
        sortedEvents.map((e) => {
          const target = this._transitions.get(transitionKey(s, e));
          return (target ?? "\u2014").length;
        }),
      ),
    );

    // Header row
    let header = " ".repeat(stateWidth) + "\u2502";
    for (const event of sortedEvents) {
      header += ` ${event.padEnd(eventWidth)} \u2502`;
    }
    const lines = [header];

    // Separator
    let sep = "\u2500".repeat(stateWidth) + "\u253C";
    for (let i = 0; i < sortedEvents.length; i++) {
      sep += "\u2500".repeat(eventWidth + 2);
      if (i < sortedEvents.length - 1) {
        sep += "\u253C";
      }
    }
    lines.push(sep);

    // Data rows
    for (const state of sortedStates) {
      let markers = "";
      if (state === this._initial) {
        markers += ">";
      }
      if (this._accepting.has(state)) {
        markers += "*";
      }
      const label = markers ? `${markers} ${state}` : `  ${state}`;

      let row = `${label.padEnd(stateWidth)}\u2502`;
      for (const event of sortedEvents) {
        const target = this._transitions.get(transitionKey(state, event)) ?? "\u2014";
        row += ` ${target.padEnd(eventWidth)} \u2502`;
      }
      lines.push(row);
    }

    return lines.join("\n");
  }

  /**
   * Return the transition table as a list of rows.
   *
   * First row is the header: ["State", event1, event2, ...].
   * Subsequent rows: [state_name, target1, target2, ...].
   * Missing transitions are represented as "\u2014".
   *
   * @returns A list of string arrays, suitable for formatting or export.
   */
  toTable(): string[][] {
    const sortedEvents = [...this._alphabet].sort();
    const sortedStates = [...this._states].sort();

    const rows: string[][] = [];
    rows.push(["State", ...sortedEvents]);

    for (const state of sortedStates) {
      const row = [state];
      for (const event of sortedEvents) {
        const target = this._transitions.get(transitionKey(state, event)) ?? "\u2014";
        row.push(target);
      }
      rows.push(row);
    }

    return rows;
  }
}
