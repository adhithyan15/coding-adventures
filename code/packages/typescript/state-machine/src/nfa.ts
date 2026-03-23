/**
 * Non-deterministic Finite Automaton (NFA) with epsilon transitions.
 *
 * === What is an NFA? ===
 *
 * An NFA relaxes the deterministic constraint of a DFA in two ways:
 *
 * 1. **Multiple transitions:** A single (state, input) pair can lead to
 *    multiple target states. The machine explores all possibilities
 *    simultaneously — like spawning parallel universes.
 *
 * 2. **Epsilon transitions:** The machine can jump to another state
 *    without consuming any input. These are "free" moves.
 *
 * === The "parallel universes" model ===
 *
 * Think of an NFA as a machine that clones itself at every non-deterministic
 * choice point. All clones run in parallel:
 *
 *     - A clone that reaches a dead end (no transition) simply vanishes.
 *     - A clone that reaches an accepting state means the whole NFA accepts.
 *     - If ALL clones die without reaching an accepting state, the NFA rejects.
 *
 * The NFA accepts if there EXISTS at least one path through the machine
 * that ends in an accepting state.
 *
 * === Why NFAs matter ===
 *
 * NFAs are much easier to construct for certain problems. For example, "does
 * this string contain the substring 'abc'?" is trivial as an NFA (just guess
 * where 'abc' starts) but requires careful tracking as a DFA.
 *
 * Every NFA can be converted to an equivalent DFA via subset construction.
 * This is how regex engines work: regex -> NFA (easy) -> DFA (mechanical) ->
 * efficient execution (O(1) per character).
 *
 * === Formal definition ===
 *
 *     NFA = (Q, Sigma, delta, q0, F)
 *
 *     Q  = finite set of states
 *     Sigma = finite alphabet (input symbols)
 *     delta = transition function: Q x (Sigma | {epsilon}) -> P(Q)
 *          maps (state, input_or_epsilon) to a SET of states
 *     q0 = initial state
 *     F  = accepting states
 *
 * @module nfa
 */

import { LabeledDirectedGraph } from "@coding-adventures/directed-graph";
import { DFA } from "./dfa.js";
import { transitionKey } from "./types.js";

// === Epsilon Sentinel ===
//
// We use the empty string "" as the epsilon symbol. This works because
// no real input alphabet should contain the empty string — input symbols
// are always at least one character long.

/**
 * Sentinel value for epsilon transitions (transitions that consume no input).
 *
 * Used as the event in transition keys to represent "free" moves that
 * don't consume any input character.
 */
export const EPSILON = "";

/**
 * Non-deterministic Finite Automaton with epsilon transitions.
 *
 * An NFA can be in multiple states simultaneously. Processing an input
 * event means: for each current state, find all transitions on that
 * event, take the union of target states, then compute the epsilon
 * closure of the result.
 *
 * The NFA accepts an input sequence if, after processing all inputs,
 * ANY of the current states is an accepting state.
 *
 * @example
 * ```typescript
 * // NFA that accepts strings containing "ab"
 * const nfa = new NFA(
 *   new Set(["q0", "q1", "q2"]),
 *   new Set(["a", "b"]),
 *   new Map([
 *     ["q0\0a", new Set(["q0", "q1"])],  // non-deterministic!
 *     ["q0\0b", new Set(["q0"])],
 *     ["q1\0b", new Set(["q2"])],
 *     ["q2\0a", new Set(["q2"])],
 *     ["q2\0b", new Set(["q2"])],
 *   ]),
 *   "q0",
 *   new Set(["q2"]),
 * );
 * nfa.accepts(["a", "b"]); // true
 * nfa.accepts(["b", "a"]); // false
 * ```
 */
export class NFA {
  private readonly _states: ReadonlySet<string>;
  private readonly _alphabet: ReadonlySet<string>;
  private readonly _transitions: Map<string, ReadonlySet<string>>;
  private readonly _initial: string;
  private readonly _accepting: ReadonlySet<string>;

  // --- Internal graph representation ---
  //
  // We maintain a LabeledDirectedGraph alongside the _transitions Map.
  // The Map is kept for O(1) lookups in process(), epsilonClosure(),
  // accepts(), and toDfa() — the performance-critical paths.
  // The graph captures the structure of the NFA for introspection and
  // future algorithmic queries.
  //
  // Epsilon transitions use the EPSILON constant ("") as the edge label,
  // preserving the distinction between input-consuming and free transitions.
  private readonly _graph: LabeledDirectedGraph;

  /** The NFA starts in the epsilon closure of the initial state. */
  private _current: ReadonlySet<string>;

  /**
   * Create a new NFA.
   *
   * @param states - The finite set of states. Must be non-empty.
   * @param alphabet - The finite set of input symbols. Must not contain
   *   the empty string (reserved for epsilon).
   * @param transitions - Map from encoded (state, event_or_epsilon) keys
   *   to a set of target states. Use EPSILON ("") for epsilon transitions.
   *   Keys are created with {@link transitionKey}.
   * @param initial - The starting state. Must be in `states`.
   * @param accepting - The set of accepting/final states.
   *
   * @throws Error if any validation check fails.
   */
  constructor(
    states: Set<string>,
    alphabet: Set<string>,
    transitions: Map<string, Set<string>>,
    initial: string,
    accepting: Set<string>,
  ) {
    if (states.size === 0) {
      throw new Error("States set must be non-empty");
    }
    if (alphabet.has(EPSILON)) {
      throw new Error(
        "Alphabet must not contain the empty string (reserved for epsilon)",
      );
    }
    if (!states.has(initial)) {
      throw new Error(
        `Initial state '${initial}' is not in the states set`,
      );
    }
    for (const s of accepting) {
      if (!states.has(s)) {
        throw new Error(
          `Accepting state '${s}' is not in the states set`,
        );
      }
    }

    // Validate transitions
    for (const [key, targets] of transitions) {
      const sep = key.indexOf("\0");
      const source = key.substring(0, sep);
      const event = key.substring(sep + 1);

      if (!states.has(source)) {
        throw new Error(
          `Transition source '${source}' is not in the states set`,
        );
      }
      if (event !== EPSILON && !alphabet.has(event)) {
        throw new Error(
          `Transition event '${event}' is not in the alphabet and is not epsilon`,
        );
      }
      for (const t of targets) {
        if (!states.has(t)) {
          throw new Error(
            `Transition target '${t}' (from (${source}, '${event}')) is not in the states set`,
          );
        }
      }
    }

    this._states = new Set(states);
    this._alphabet = new Set(alphabet);
    this._transitions = new Map(
      [...transitions.entries()].map(([k, v]) => [k, new Set(v)] as const),
    );
    this._initial = initial;
    this._accepting = new Set(accepting);

    // --- Build internal graph representation ---
    //
    // Each state becomes a node. Each transition (source, event) -> targets
    // becomes labeled edges from source to each target with the event as label.
    // Epsilon transitions use the EPSILON constant ("") as the label.
    this._graph = new LabeledDirectedGraph();
    for (const state of states) {
      this._graph.addNode(state);
    }
    for (const [key, targets] of transitions) {
      const sep = key.indexOf("\0");
      const source = key.substring(0, sep);
      const event = key.substring(sep + 1);
      const label = event !== EPSILON ? event : EPSILON;
      for (const target of targets) {
        this._graph.addEdge(source, target, label);
      }
    }

    // The NFA starts in the epsilon closure of the initial state
    this._current = this.epsilonClosure(new Set([initial]));
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

  /** The initial state. */
  get initial(): string {
    return this._initial;
  }

  /** The set of accepting/final states. */
  get accepting(): ReadonlySet<string> {
    return this._accepting;
  }

  /** The set of states the NFA is currently in. */
  get currentStates(): ReadonlySet<string> {
    return this._current;
  }

  // === Epsilon Closure ===

  /**
   * Compute the epsilon closure of a set of states.
   *
   * Starting from the given states, follow ALL epsilon transitions
   * recursively. Return the full set of states reachable via zero or
   * more epsilon transitions.
   *
   * This is the key operation that makes NFAs work: before and after
   * processing each input, we expand to include all states reachable
   * via "free" epsilon moves.
   *
   * The algorithm is a simple BFS over epsilon edges:
   *
   *     1. Start with the input set
   *     2. For each state, find epsilon transitions
   *     3. Add all targets to the set
   *     4. Repeat until no new states are found
   *
   * @param states - The starting set of states.
   * @returns A Set of all states reachable via epsilon transitions
   *   from any state in the input set.
   *
   * @example
   * ```typescript
   * // Given: q0 --epsilon--> q1 --epsilon--> q2
   * nfa.epsilonClosure(new Set(["q0"])); // Set(["q0", "q1", "q2"])
   * ```
   */
  epsilonClosure(states: ReadonlySet<string>): Set<string> {
    const closure = new Set(states);
    const worklist = [...states];

    while (worklist.length > 0) {
      const state = worklist.pop()!;
      // Find epsilon transitions from this state
      const targets =
        this._transitions.get(transitionKey(state, EPSILON)) ??
        new Set<string>();
      for (const target of targets) {
        if (!closure.has(target)) {
          closure.add(target);
          worklist.push(target);
        }
      }
    }

    return closure;
  }

  // === Processing ===

  /**
   * Process one input event and return the new set of states.
   *
   * For each current state, find all transitions on this event.
   * Take the union of all target states, then compute the epsilon
   * closure of the result.
   *
   * @param event - An input symbol from the alphabet.
   * @returns The new set of current states after processing.
   * @throws Error if the event is not in the alphabet.
   */
  process(event: string): ReadonlySet<string> {
    if (!this._alphabet.has(event)) {
      throw new Error(
        `Event '${event}' is not in the alphabet [${[...this._alphabet].sort().join(", ")}]`,
      );
    }

    // Collect all target states from all current states
    const nextStates = new Set<string>();
    for (const state of this._current) {
      const targets =
        this._transitions.get(transitionKey(state, event)) ??
        new Set<string>();
      for (const t of targets) {
        nextStates.add(t);
      }
    }

    // Expand via epsilon closure
    this._current = this.epsilonClosure(nextStates);
    return this._current;
  }

  /**
   * Process a sequence of inputs and return the trace.
   *
   * Each entry in the trace is: [states_before, event, states_after].
   *
   * @param events - A list of input symbols.
   * @returns A list of [before_states, event, after_states] tuples.
   */
  processSequence(
    events: string[],
  ): Array<[ReadonlySet<string>, string, ReadonlySet<string>]> {
    const trace: Array<[ReadonlySet<string>, string, ReadonlySet<string>]> = [];
    for (const event of events) {
      const before = this._current;
      this.process(event);
      trace.push([before, event, this._current]);
    }
    return trace;
  }

  /**
   * Check if the NFA accepts the input sequence.
   *
   * The NFA accepts if, after processing all inputs, ANY of the
   * current states is an accepting state.
   *
   * Does NOT modify the NFA's current state — runs on a copy.
   *
   * @param events - A list of input symbols.
   * @returns True if the NFA accepts, false otherwise.
   */
  accepts(events: string[]): boolean {
    // Simulate without modifying this NFA's state
    let current = this.epsilonClosure(new Set([this._initial]));

    for (const event of events) {
      if (!this._alphabet.has(event)) {
        throw new Error(
          `Event '${event}' is not in the alphabet [${[...this._alphabet].sort().join(", ")}]`,
        );
      }
      const nextStates = new Set<string>();
      for (const state of current) {
        const targets =
          this._transitions.get(transitionKey(state, event)) ??
          new Set<string>();
        for (const t of targets) {
          nextStates.add(t);
        }
      }
      current = this.epsilonClosure(nextStates);

      // If no states are active, the NFA is dead — reject early
      if (current.size === 0) {
        return false;
      }
    }

    for (const s of current) {
      if (this._accepting.has(s)) {
        return true;
      }
    }
    return false;
  }

  /**
   * Reset to the initial state (with epsilon closure).
   */
  reset(): void {
    this._current = this.epsilonClosure(new Set([this._initial]));
  }

  // === Conversion to DFA ===

  /**
   * Convert this NFA to an equivalent DFA using subset construction.
   *
   * === The Subset Construction Algorithm ===
   *
   * The key insight: if an NFA can be in states {q0, q1, q3}
   * simultaneously, we create a single DFA state representing that
   * entire set. The DFA's states are sets of NFA states.
   *
   * Algorithm:
   *     1. Start with d0 = epsilon-closure({q0})
   *     2. For each DFA state D and each input symbol a:
   *         - For each NFA state q in D, find delta(q, a)
   *         - Take the union of all targets
   *         - Compute epsilon-closure of the union
   *         - That is the new DFA state D'
   *     3. Repeat until no new DFA states are discovered
   *     4. A DFA state is accepting if it contains ANY NFA accepting state
   *
   * DFA state names are generated from sorted NFA state names:
   *     Set(["q0", "q1"]) -> "{q0,q1}"
   *
   * @returns A DFA that recognizes exactly the same language as this NFA.
   */
  toDfa(): DFA {
    // Step 1: initial DFA state = epsilon-closure of NFA initial state
    const startClosure = this.epsilonClosure(new Set([this._initial]));
    const dfaStart = stateSetName(startClosure);

    // Track DFA states and transitions as we discover them
    const dfaStates = new Set<string>([dfaStart]);
    const dfaTransitions = new Map<string, string>();
    const dfaAccepting = new Set<string>();

    // Map from DFA state name -> Set of NFA states
    const stateMap = new Map<string, ReadonlySet<string>>([
      [dfaStart, startClosure],
    ]);

    // Check if start state is accepting
    for (const s of startClosure) {
      if (this._accepting.has(s)) {
        dfaAccepting.add(dfaStart);
        break;
      }
    }

    // Step 2-3: BFS over DFA states
    const worklist: string[] = [dfaStart];

    while (worklist.length > 0) {
      const currentName = worklist.pop()!;
      const currentNfaStates = stateMap.get(currentName)!;

      for (const event of [...this._alphabet].sort()) {
        // Collect all NFA states reachable via this event
        const nextNfa = new Set<string>();
        for (const nfaState of currentNfaStates) {
          const targets =
            this._transitions.get(transitionKey(nfaState, event)) ??
            new Set<string>();
          for (const t of targets) {
            nextNfa.add(t);
          }
        }

        // Epsilon closure of the result
        const nextClosure = this.epsilonClosure(nextNfa);

        if (nextClosure.size === 0) {
          // Dead state — no transition (DFA will be incomplete)
          continue;
        }

        const nextName = stateSetName(nextClosure);

        // Record this DFA transition
        dfaTransitions.set(transitionKey(currentName, event), nextName);

        // If this is a new DFA state, add it to the worklist
        if (!dfaStates.has(nextName)) {
          dfaStates.add(nextName);
          stateMap.set(nextName, nextClosure);
          worklist.push(nextName);

          // Check if accepting
          for (const s of nextClosure) {
            if (this._accepting.has(s)) {
              dfaAccepting.add(nextName);
              break;
            }
          }
        }
      }
    }

    return new DFA(
      dfaStates,
      new Set(this._alphabet),
      dfaTransitions,
      dfaStart,
      dfaAccepting,
    );
  }

  // === Visualization ===

  /**
   * Return a Graphviz DOT representation of this NFA.
   *
   * Epsilon transitions are labeled "epsilon". Non-deterministic transitions
   * (multiple targets) produce multiple edges from the same source.
   *
   * @returns A string in DOT format.
   */
  toDot(): string {
    const lines: string[] = [];
    lines.push("digraph NFA {");
    lines.push("    rankdir=LR;");
    lines.push("");

    // Start arrow
    lines.push("    __start [shape=point, width=0.2];");
    lines.push(`    __start -> "${this._initial}";`);
    lines.push("");

    // State shapes
    for (const state of [...this._states].sort()) {
      const shape = this._accepting.has(state) ? "doublecircle" : "circle";
      lines.push(`    "${state}" [shape=${shape}];`);
    }
    lines.push("");

    // Transitions — group by (source, target) to combine labels
    const edgeLabels = new Map<string, string[]>();
    const sortedEntries = [...this._transitions.entries()].sort(
      ([a], [b]) => (a < b ? -1 : a > b ? 1 : 0),
    );

    for (const [key, targets] of sortedEntries) {
      const sep = key.indexOf("\0");
      const source = key.substring(0, sep);
      const event = key.substring(sep + 1);
      const label = event === EPSILON ? "\u03B5" : event;
      for (const target of [...targets].sort()) {
        const edgeKey = `${source}\0${target}`;
        if (!edgeLabels.has(edgeKey)) {
          edgeLabels.set(edgeKey, []);
        }
        edgeLabels.get(edgeKey)!.push(label);
      }
    }

    const sortedEdges = [...edgeLabels.entries()].sort(
      ([a], [b]) => (a < b ? -1 : a > b ? 1 : 0),
    );
    for (const [edgeKey, labels] of sortedEdges) {
      const sep = edgeKey.indexOf("\0");
      const source = edgeKey.substring(0, sep);
      const target = edgeKey.substring(sep + 1);
      const label = labels.join(", ");
      lines.push(`    "${source}" -> "${target}" [label="${label}"];`);
    }

    lines.push("}");
    return lines.join("\n");
  }
}

/**
 * Convert a Set of state names to a DFA state name.
 *
 * The name is deterministic: sorted state names joined with commas
 * and wrapped in braces.
 *
 * @param states - The set of NFA states
 * @returns A canonical name like "{q0,q1,q2}"
 *
 * @example
 * ```typescript
 * stateSetName(new Set(["q0", "q2", "q1"])); // "{q0,q1,q2}"
 * ```
 */
export function stateSetName(states: ReadonlySet<string>): string {
  return "{" + [...states].sort().join(",") + "}";
}
