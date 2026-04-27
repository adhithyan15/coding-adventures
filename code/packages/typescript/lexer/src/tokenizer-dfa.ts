/**
 * Tokenizer DFA -- formal model of the hand-written lexer's dispatch logic.
 *
 * The hand-written tokenizer in tokenizer.ts has an *implicit* DFA in its
 * main loop: it looks at the current character, classifies it, and dispatches
 * to the appropriate sub-routine. This module makes that implicit DFA
 * *explicit* by defining it as a formal DFA using the state-machine library.
 *
 * ## States
 *
 * | State          | Description                                   |
 * |----------------|-----------------------------------------------|
 * | start          | Idle, examining the next character             |
 * | in_number      | Reading a sequence of digits                  |
 * | in_name        | Reading an identifier                         |
 * | in_string      | Reading a string literal                      |
 * | in_operator    | Emitting a single-character operator/delimiter |
 * | in_equals      | Handling = with lookahead for ==              |
 * | at_newline     | Emitting a NEWLINE token                      |
 * | at_whitespace  | Skipping whitespace                           |
 * | done           | End of input                                  |
 * | error          | Unexpected character                          |
 *
 * ## How the DFA is used
 *
 * The DFA does NOT replace the tokenizer's logic. The sub-routines like
 * readNumber() and readString() still do the actual work. What the DFA
 * provides is a formal, verifiable model of the dispatch decision.
 *
 * @module tokenizer-dfa
 */

import { DFA, transitionKey } from "@coding-adventures/state-machine";

// ---------------------------------------------------------------------------
// Character Classification
// ---------------------------------------------------------------------------

/**
 * Classify a character into one of the DFA's alphabet symbols.
 *
 * Maps every possible character to a named class. The DFA's transition
 * table uses these class names to decide what to do next.
 *
 * @param ch - A single character, or null if at end of input.
 * @returns A string naming the character class.
 */
export function classifyChar(ch: string | null): string {
  if (ch === null) return "eof";
  if (ch === " " || ch === "\t" || ch === "\r") return "whitespace";
  if (ch === "\n") return "newline";
  if (ch >= "0" && ch <= "9") return "digit";
  if ((ch >= "a" && ch <= "z") || (ch >= "A" && ch <= "Z")) return "alpha";
  if (ch === "_") return "underscore";
  if (ch === '"') return "quote";
  if (ch === "=") return "equals";
  if (ch === "+" || ch === "-" || ch === "*" || ch === "/") return "operator";
  if (ch === "(") return "open_paren";
  if (ch === ")") return "close_paren";
  if (ch === ",") return "comma";
  if (ch === ":") return "colon";
  if (ch === ";") return "semicolon";
  if (ch === "{") return "open_brace";
  if (ch === "}") return "close_brace";
  if (ch === "[") return "open_bracket";
  if (ch === "]") return "close_bracket";
  if (ch === ".") return "dot";
  if (ch === "!") return "bang";
  return "other";
}

// ---------------------------------------------------------------------------
// DFA Definition
// ---------------------------------------------------------------------------

const STATES = new Set([
  "start",
  "in_number",
  "in_name",
  "in_string",
  "in_operator",
  "in_equals",
  "at_newline",
  "at_whitespace",
  "done",
  "error",
]);

const ALPHABET = new Set([
  "digit",
  "alpha",
  "underscore",
  "quote",
  "newline",
  "whitespace",
  "operator",
  "equals",
  "open_paren",
  "close_paren",
  "comma",
  "colon",
  "semicolon",
  "open_brace",
  "close_brace",
  "open_bracket",
  "close_bracket",
  "dot",
  "bang",
  "eof",
  "other",
]);

/** Map from character class to target state from "start". */
const START_DISPATCH: ReadonlyMap<string, string> = new Map([
  ["digit", "in_number"],
  ["alpha", "in_name"],
  ["underscore", "in_name"],
  ["quote", "in_string"],
  ["newline", "at_newline"],
  ["whitespace", "at_whitespace"],
  ["operator", "in_operator"],
  ["equals", "in_equals"],
  ["open_paren", "in_operator"],
  ["close_paren", "in_operator"],
  ["comma", "in_operator"],
  ["colon", "in_operator"],
  ["semicolon", "in_operator"],
  ["open_brace", "in_operator"],
  ["close_brace", "in_operator"],
  ["open_bracket", "in_operator"],
  ["close_bracket", "in_operator"],
  ["dot", "in_operator"],
  ["bang", "in_operator"],
  ["eof", "done"],
  ["other", "error"],
]);

/** Build the full transition map. */
function buildTransitions(): Map<string, string> {
  const transitions = new Map<string, string>();
  const alphabetArr = [...ALPHABET];

  // From "start", dispatch based on character class.
  for (const [charClass, target] of START_DISPATCH) {
    transitions.set(transitionKey("start", charClass), target);
  }

  // All handler states return to "start" on every symbol.
  const handlers = [
    "in_number",
    "in_name",
    "in_string",
    "in_operator",
    "in_equals",
    "at_newline",
    "at_whitespace",
  ];
  for (const handler of handlers) {
    for (const symbol of alphabetArr) {
      transitions.set(transitionKey(handler, symbol), "start");
    }
  }

  // "done" and "error" loop on themselves for every symbol.
  for (const terminal of ["done", "error"]) {
    for (const symbol of alphabetArr) {
      transitions.set(transitionKey(terminal, symbol), terminal);
    }
  }

  return transitions;
}

/**
 * Create a new tokenizer dispatch DFA.
 *
 * Each call returns a fresh DFA so callers can process independently.
 * The DFA models the top-level character classification dispatch of the
 * hand-written tokenizer.
 *
 * @returns A new DFA instance.
 *
 * @example
 * ```typescript
 * const dfa = newTokenizerDFA();
 * const charClass = classifyChar('5');       // "digit"
 * const nextState = dfa.process(charClass);  // "in_number"
 * ```
 */
export function newTokenizerDFA(): DFA {
  return new DFA(
    new Set(STATES),
    new Set(ALPHABET),
    buildTransitions(),
    "start",
    new Set(["done"]),
  );
}
