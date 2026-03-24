/**
 * Pushdown Automaton (PDA) — a finite automaton with a stack.
 *
 * === What is a PDA? ===
 *
 * A PDA is a state machine augmented with a **stack** — an unbounded LIFO
 * (last-in, first-out) data structure. The stack gives the PDA the ability
 * to "remember" things that a finite automaton cannot, like how many open
 * parentheses it has seen.
 *
 * This extra memory is exactly what is needed to recognize **context-free
 * languages** — the class of languages that includes balanced parentheses,
 * nested HTML tags, arithmetic expressions, and most programming language
 * syntax.
 *
 * === The Chomsky Hierarchy Connection ===
 *
 *     Regular languages    subset of  Context-free languages  subset of  Context-sensitive  subset of  RE
 *     (DFA/NFA)                       (PDA)                              (LBA)                         (TM)
 *
 * A DFA can recognize "does this string match the pattern a*b*?" but CANNOT
 * recognize "does this string have equal numbers of a's and b's?" — that
 * requires counting, and a DFA has no memory beyond its finite state.
 *
 * A PDA can recognize "a^n b^n" (n a's followed by n b's) because it can
 * push an 'a' for each 'a' it reads, then pop an 'a' for each 'b'. If the
 * stack is empty at the end, the counts match.
 *
 * === Formal Definition ===
 *
 *     PDA = (Q, Sigma, Gamma, delta, q0, Z0, F)
 *
 *     Q  = finite set of states
 *     Sigma = input alphabet
 *     Gamma = stack alphabet (may differ from Sigma)
 *     delta = transition function: Q x (Sigma | {epsilon}) x Gamma -> P(Q x Gamma*)
 *     q0 = initial state
 *     Z0 = initial stack symbol (bottom marker)
 *     F  = accepting states
 *
 * Our implementation is deterministic (DPDA): at most one transition
 * applies at any time. This is simpler to implement and trace, and
 * sufficient for most practical parsing tasks.
 *
 * @module pda
 */

/**
 * A single transition rule for a pushdown automaton.
 *
 * A PDA transition says: "If I am in state `source`, and I see input
 * `event` (or epsilon if null), and the top of my stack is `stackRead`,
 * then move to state `target` and replace the stack top with `stackPush`.
 *
 * Stack semantics:
 * - stackPush = []           -> pop the top (consume it)
 * - stackPush = [X]          -> replace top with X
 * - stackPush = [X, Y]       -> pop top, push X, then push Y (Y is new top)
 * - stackPush = [stackRead]  -> leave the stack unchanged
 *
 * @example
 * ```typescript
 * const t: PDATransition = {
 *   source: "q0", event: "(", stackRead: "$",
 *   target: "q0", stackPush: ["$", "("],
 * };
 * // "In q0, reading '(', with '$' on top: stay in q0, push '(' above '$'"
 * ```
 */
export interface PDATransition {
  readonly source: string;
  /** The input event, or null for epsilon transitions. */
  readonly event: string | null;
  /** What must be on top of the stack for this transition to fire. */
  readonly stackRead: string;
  readonly target: string;
  /** What to push onto the stack (replaces stackRead). */
  readonly stackPush: readonly string[];
}

/**
 * One step in a PDA's execution trace.
 *
 * Captures the full state of the PDA at each transition: which rule
 * fired, what the stack looked like after the transition.
 */
export interface PDATraceEntry {
  readonly source: string;
  readonly event: string | null;
  readonly stackRead: string;
  readonly target: string;
  readonly stackPush: readonly string[];
  /** Full stack contents after the transition (bottom to top). */
  readonly stackAfter: readonly string[];
}

/**
 * Deterministic Pushdown Automaton.
 *
 * A finite state machine with a stack, capable of recognizing
 * context-free languages (balanced parentheses, nested tags, a^n b^n).
 *
 * The PDA accepts by final state: it accepts if, after processing all
 * input, it is in an accepting state. (Some formulations accept by
 * empty stack instead; ours uses accepting states for consistency
 * with DFA/NFA.)
 *
 * @example
 * ```typescript
 * // PDA for balanced parentheses
 * const pda = new PushdownAutomaton(
 *   new Set(["q0", "accept"]),
 *   new Set(["(", ")"]),
 *   new Set(["(", "$"]),
 *   [
 *     { source: "q0", event: "(", stackRead: "$", target: "q0", stackPush: ["$", "("] },
 *     { source: "q0", event: "(", stackRead: "(", target: "q0", stackPush: ["(", "("] },
 *     { source: "q0", event: ")", stackRead: "(", target: "q0", stackPush: [] },
 *     { source: "q0", event: null, stackRead: "$", target: "accept", stackPush: [] },
 *   ],
 *   "q0",
 *   "$",
 *   new Set(["accept"]),
 * );
 * pda.accepts(["(", "(", ")", ")"]); // true
 * ```
 */
export class PushdownAutomaton {
  private readonly _states: ReadonlySet<string>;
  private readonly _inputAlphabet: ReadonlySet<string>;
  private readonly _stackAlphabet: ReadonlySet<string>;
  private readonly _transitions: readonly PDATransition[];
  private readonly _initial: string;
  private readonly _initialStackSymbol: string;
  private readonly _accepting: ReadonlySet<string>;

  /**
   * Index transitions for fast lookup: key is "state\0event\0stackTop"
   * where event is "null" for epsilon transitions.
   */
  private readonly _transitionIndex: Map<string, PDATransition>;

  /** Mutable execution state. */
  private _current: string;
  private _stack: string[];
  private _trace: PDATraceEntry[];

  /**
   * Create a new PDA.
   *
   * @param states - Finite set of states.
   * @param inputAlphabet - Finite set of input symbols.
   * @param stackAlphabet - Finite set of stack symbols.
   * @param transitions - List of transition rules.
   * @param initial - Starting state.
   * @param initialStackSymbol - Symbol placed on the stack initially
   *   (typically '$' as a bottom-of-stack marker).
   * @param accepting - Set of accepting/final states.
   *
   * @throws Error if validation fails.
   */
  constructor(
    states: Set<string>,
    inputAlphabet: Set<string>,
    stackAlphabet: Set<string>,
    transitions: PDATransition[],
    initial: string,
    initialStackSymbol: string,
    accepting: Set<string>,
  ) {
    if (states.size === 0) {
      throw new Error("States set must be non-empty");
    }
    if (!states.has(initial)) {
      throw new Error(
        `Initial state '${initial}' is not in the states set`,
      );
    }
    if (!stackAlphabet.has(initialStackSymbol)) {
      throw new Error(
        `Initial stack symbol '${initialStackSymbol}' is not in the stack alphabet`,
      );
    }
    for (const s of accepting) {
      if (!states.has(s)) {
        throw new Error(
          `Accepting state '${s}' is not in the states set`,
        );
      }
    }

    this._states = new Set(states);
    this._inputAlphabet = new Set(inputAlphabet);
    this._stackAlphabet = new Set(stackAlphabet);
    this._transitions = [...transitions];
    this._initial = initial;
    this._initialStackSymbol = initialStackSymbol;
    this._accepting = new Set(accepting);

    // Index transitions for fast lookup: (state, event_or_null, stack_top)
    this._transitionIndex = new Map();
    for (const t of transitions) {
      const key = pdaTransitionKey(t.source, t.event, t.stackRead);
      if (this._transitionIndex.has(key)) {
        throw new Error(
          `Duplicate transition for (${t.source}, ${t.event === null ? "null" : `'${t.event}'`}, '${t.stackRead}') — this PDA must be deterministic`,
        );
      }
      this._transitionIndex.set(key, t);
    }

    // Mutable execution state
    this._current = initial;
    this._stack = [initialStackSymbol];
    this._trace = [];
  }

  // === Properties ===

  /** The finite set of states. */
  get states(): ReadonlySet<string> {
    return this._states;
  }

  /** The current state. */
  get currentState(): string {
    return this._current;
  }

  /** Current stack contents (bottom to top) as a readonly array. */
  get stack(): readonly string[] {
    return [...this._stack];
  }

  /** The top of the stack, or null if empty. */
  get stackTop(): string | null {
    return this._stack.length > 0 ? this._stack[this._stack.length - 1] : null;
  }

  /** The execution trace. Returns a copy. */
  get trace(): PDATraceEntry[] {
    return [...this._trace];
  }

  // === Processing ===

  /**
   * Find a matching transition for the current state and stack top.
   *
   * Looks for a transition matching (current_state, event, stack_top).
   * Returns undefined if no transition exists.
   */
  private findTransition(event: string | null): PDATransition | undefined {
    if (this._stack.length === 0) {
      return undefined;
    }
    const top = this._stack[this._stack.length - 1];
    return this._transitionIndex.get(
      pdaTransitionKey(this._current, event, top),
    );
  }

  /**
   * Apply a transition: change state and modify the stack.
   */
  private applyTransition(transition: PDATransition): void {
    // Pop the stack top (it was "read" by the transition)
    this._stack.pop();

    // Push new symbols (in order: first element goes deepest)
    for (const symbol of transition.stackPush) {
      this._stack.push(symbol);
    }

    // Record the trace
    this._trace.push({
      source: transition.source,
      event: transition.event,
      stackRead: transition.stackRead,
      target: transition.target,
      stackPush: [...transition.stackPush],
      stackAfter: [...this._stack],
    });

    // Change state
    this._current = transition.target;
  }

  /**
   * Try to take an epsilon transition.
   * @returns True if one was taken.
   */
  private tryEpsilon(): boolean {
    const t = this.findTransition(null);
    if (t !== undefined) {
      this.applyTransition(t);
      return true;
    }
    return false;
  }

  /**
   * Process one input symbol.
   *
   * First checks for a transition on the given event. If none exists,
   * throws an error.
   *
   * @param event - An input symbol.
   * @returns The new current state.
   * @throws Error if no transition matches.
   */
  process(event: string): string {
    const t = this.findTransition(event);
    if (t === undefined) {
      throw new Error(
        `No transition for (state='${this._current}', event='${event}', stackTop='${this.stackTop}')`,
      );
    }
    this.applyTransition(t);
    return this._current;
  }

  /**
   * Process a sequence of inputs and return the trace.
   *
   * After processing all inputs, tries epsilon transitions until
   * none are available (this handles acceptance transitions that
   * fire at end-of-input).
   *
   * @param events - List of input symbols.
   * @returns The trace entries generated during processing.
   */
  processSequence(events: string[]): PDATraceEntry[] {
    const traceStart = this._trace.length;
    for (const event of events) {
      this.process(event);
    }
    // Try epsilon transitions at end of input
    while (this.tryEpsilon()) {
      // keep going
    }
    return this._trace.slice(traceStart);
  }

  /**
   * Check if the PDA accepts the input sequence.
   *
   * Processes all inputs, then tries epsilon transitions until none
   * are available. Returns true if the final state is accepting.
   *
   * Does NOT modify this PDA's state — runs on a copy.
   *
   * @param events - List of input symbols.
   * @returns True if the PDA accepts.
   */
  accepts(events: string[]): boolean {
    // Simulate on copies of the mutable state
    let state = this._initial;
    const stack = [this._initialStackSymbol];

    for (const event of events) {
      if (stack.length === 0) {
        return false;
      }
      const top = stack[stack.length - 1];
      const t = this._transitionIndex.get(
        pdaTransitionKey(state, event, top),
      );
      if (t === undefined) {
        return false;
      }
      stack.pop();
      for (const symbol of t.stackPush) {
        stack.push(symbol);
      }
      state = t.target;
    }

    // Try epsilon transitions at end of input
    const maxEpsilon = this._transitions.length + 1; // bound to prevent infinite loops
    for (let i = 0; i < maxEpsilon; i++) {
      if (stack.length === 0) {
        break;
      }
      const top = stack[stack.length - 1];
      const t = this._transitionIndex.get(
        pdaTransitionKey(state, null, top),
      );
      if (t === undefined) {
        break;
      }
      stack.pop();
      for (const symbol of t.stackPush) {
        stack.push(symbol);
      }
      state = t.target;
    }

    return this._accepting.has(state);
  }

  /**
   * Reset to initial state with initial stack.
   */
  reset(): void {
    this._current = this._initial;
    this._stack = [this._initialStackSymbol];
    this._trace = [];
  }
}

/**
 * Create a lookup key for the PDA transition index.
 *
 * Encodes (state, event, stackTop) as a single string using null byte
 * separators. The event is encoded as the string "null" for epsilon
 * transitions to distinguish from events that happen to be empty strings.
 *
 * @param state - The current state
 * @param event - The input event (null for epsilon)
 * @param stackTop - The top of the stack
 * @returns An encoded key string
 */
function pdaTransitionKey(
  state: string,
  event: string | null,
  stackTop: string,
): string {
  const eventStr = event === null ? "\0null" : event;
  return `${state}\0${eventStr}\0${stackTop}`;
}
