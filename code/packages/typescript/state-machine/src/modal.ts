/**
 * Modal State Machine — multiple sub-machines with mode switching.
 *
 * === What is a Modal State Machine? ===
 *
 * A modal state machine is a collection of named sub-machines (modes), each
 * a DFA, with transitions that switch between them. When a mode switch
 * occurs, the active sub-machine changes.
 *
 * Think of it like a text editor with Normal, Insert, and Visual modes. Each
 * mode handles keystrokes differently, and certain keys switch between modes.
 *
 * === Why modal machines matter ===
 *
 * The most important use case is **context-sensitive tokenization**. Consider
 * HTML: the characters `p > .foo { color: red; }` mean completely different
 * things depending on whether they appear inside a `<style>` tag (CSS) or
 * in normal text. A single set of token rules cannot handle both contexts.
 *
 * A modal state machine solves this: the HTML tokenizer has modes like
 * DATA, TAG_OPEN, SCRIPT_DATA, and STYLE_DATA. Each mode has its own DFA
 * with its own token rules. Certain tokens (like seeing `<style>`) trigger
 * a mode switch.
 *
 * This is how real browser engines tokenize HTML, and it is the key
 * abstraction that the grammar-tools lexer needs to support HTML, Markdown,
 * and other context-sensitive languages.
 *
 * === Connection to the Chomsky Hierarchy ===
 *
 * A single DFA recognizes regular languages (Type 3). A modal state machine
 * is more powerful: it can track context (which mode am I in?) and switch
 * rules accordingly. This moves us toward context-sensitive languages
 * (Type 1), though a modal machine is still not as powerful as a full
 * linear-bounded automaton.
 *
 * In practice, modal machines + pushdown automata cover the vast majority
 * of real-world parsing needs.
 *
 * @module modal
 */

import { LabeledDirectedGraph } from "@coding-adventures/directed-graph";
import { DFA } from "./dfa.js";

/**
 * Record of a mode switch event.
 *
 * Captures which mode we switched from and to, and what triggered it.
 */
export interface ModeTransitionRecord {
  /** The mode we were in before the switch. */
  readonly fromMode: string;
  /** The trigger event that caused the switch. */
  readonly trigger: string;
  /** The mode we switched to. */
  readonly toMode: string;
}

/**
 * A collection of named DFA sub-machines with mode transitions.
 *
 * Each mode is a DFA that handles inputs within that context. Mode
 * transitions switch which DFA is active. When a mode switch occurs,
 * the new mode's DFA is reset to its initial state.
 *
 * @example
 * ```typescript
 * // Simplified HTML tokenizer with two modes
 * const html = new ModalStateMachine(
 *   new Map([["data", dataMode], ["tag", tagMode]]),
 *   new Map([
 *     ["data\0enter_tag", "tag"],
 *     ["tag\0exit_tag", "data"],
 *   ]),
 *   "data",
 * );
 * html.currentMode; // "data"
 * html.switchMode("enter_tag"); // "tag"
 * ```
 */
export class ModalStateMachine {
  private readonly _modes: Map<string, DFA>;
  private readonly _modeTransitions: Map<string, string>;
  private readonly _initialMode: string;

  // --- Internal graph of mode transitions ---
  //
  // The mode graph captures the structure of mode switching: each mode
  // is a node, and each mode transition (mode, trigger) -> target_mode
  // becomes a labeled edge with the trigger as the label. This makes
  // the mode transition topology available for structural queries
  // (e.g., "which modes are reachable from the initial mode?").
  private readonly _modeGraph: LabeledDirectedGraph;

  /** Mutable execution state. */
  private _currentMode: string;
  private _modeTrace: ModeTransitionRecord[];

  /**
   * Create a new Modal State Machine.
   *
   * @param modes - A Map from mode names to DFA sub-machines.
   * @param modeTransitions - Map from encoded (current_mode, trigger)
   *   keys (using {@link transitionKey}) to the name of the target mode.
   * @param initialMode - The name of the starting mode.
   *
   * @throws Error if validation fails.
   */
  constructor(
    modes: Map<string, DFA>,
    modeTransitions: Map<string, string>,
    initialMode: string,
  ) {
    if (modes.size === 0) {
      throw new Error("At least one mode must be provided");
    }
    if (!modes.has(initialMode)) {
      throw new Error(
        `Initial mode '${initialMode}' is not in the modes map`,
      );
    }

    // Validate mode transitions
    for (const [key, toMode] of modeTransitions) {
      const sep = key.indexOf("\0");
      const fromMode = key.substring(0, sep);
      if (!modes.has(fromMode)) {
        throw new Error(
          `Mode transition source '${fromMode}' is not a valid mode`,
        );
      }
      if (!modes.has(toMode)) {
        throw new Error(
          `Mode transition target '${toMode}' is not a valid mode`,
        );
      }
    }

    this._modes = new Map(modes);
    this._modeTransitions = new Map(modeTransitions);
    this._initialMode = initialMode;

    // --- Build internal graph of mode transitions ---
    //
    // Each mode becomes a node. Each mode transition (mode, trigger) -> target
    // becomes a labeled edge from mode to target with the trigger as the label.
    this._modeGraph = new LabeledDirectedGraph();
    for (const mode of modes.keys()) {
      this._modeGraph.addNode(mode);
    }
    for (const [key, toMode] of modeTransitions) {
      const sep = key.indexOf("\0");
      const fromMode = key.substring(0, sep);
      const trigger = key.substring(sep + 1);
      this._modeGraph.addEdge(fromMode, toMode, trigger);
    }

    this._currentMode = initialMode;
    this._modeTrace = [];
  }

  // === Properties ===

  /** The name of the currently active mode. */
  get currentMode(): string {
    return this._currentMode;
  }

  /** The DFA for the current mode. */
  get activeMachine(): DFA {
    return this._modes.get(this._currentMode)!;
  }

  /** All modes and their DFAs. Returns a copy. */
  get modes(): Map<string, DFA> {
    return new Map(this._modes);
  }

  /** The history of mode switches. Returns a copy. */
  get modeTrace(): ModeTransitionRecord[] {
    return [...this._modeTrace];
  }

  // === Processing ===

  /**
   * Process an input event in the current mode's DFA.
   *
   * Delegates to the active DFA's process() method.
   *
   * @param event - An input symbol for the current mode's DFA.
   * @returns The new state of the active DFA.
   * @throws Error if the event is invalid for the current mode.
   */
  process(event: string): string {
    return this._modes.get(this._currentMode)!.process(event);
  }

  /**
   * Switch to a different mode based on a trigger event.
   *
   * Looks up (current_mode, trigger) in the mode transitions.
   * If found, switches to the target mode and resets its DFA
   * to the initial state.
   *
   * @param trigger - The event that triggers the mode switch.
   * @returns The name of the new mode.
   * @throws Error if no mode transition exists for this trigger.
   */
  switchMode(trigger: string): string {
    const key = `${this._currentMode}\0${trigger}`;
    const newMode = this._modeTransitions.get(key);
    if (newMode === undefined) {
      throw new Error(
        `No mode transition for (mode='${this._currentMode}', trigger='${trigger}')`,
      );
    }

    const oldMode = this._currentMode;

    // Reset the target mode's DFA to its initial state
    this._modes.get(newMode)!.reset();

    // Record the switch
    this._modeTrace.push({
      fromMode: oldMode,
      trigger,
      toMode: newMode,
    });

    this._currentMode = newMode;
    return newMode;
  }

  /**
   * Reset to initial mode and reset all sub-machines.
   */
  reset(): void {
    this._currentMode = this._initialMode;
    this._modeTrace = [];
    for (const dfa of this._modes.values()) {
      dfa.reset();
    }
  }
}
