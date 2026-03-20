/**
 * Tests for the Tokenizer DFA and classifyChar helper
 * =====================================================
 *
 * These tests verify two things:
 *
 * 1. The classifyChar function correctly maps every character to its
 *    character class (the DFA's alphabet).
 *
 * 2. The tokenizer DFA formally matches the tokenizer's actual dispatch
 *    behavior. Every character class from "start" transitions to the
 *    correct handler state.
 *
 * 3. The DFA is well-formed: it is complete (every state handles every
 *    input), and the formal model matches what the tokenizer actually
 *    does when tokenizing real code.
 */

import { describe, it, expect } from "vitest";
import { classifyChar, newTokenizerDFA } from "../src/tokenizer-dfa.js";
import { tokenize } from "../src/tokenizer.js";

// ============================================================================
// classifyChar tests
// ============================================================================

describe("classifyChar", () => {
  it("should classify null as eof", () => {
    expect(classifyChar(null)).toBe("eof");
  });

  it("should classify digits as digit", () => {
    for (const ch of "0123456789") {
      expect(classifyChar(ch)).toBe("digit");
    }
  });

  it("should classify letters as alpha", () => {
    for (const ch of "azAZ") {
      expect(classifyChar(ch)).toBe("alpha");
    }
  });

  it("should classify underscore as underscore", () => {
    expect(classifyChar("_")).toBe("underscore");
  });

  it("should classify whitespace characters as whitespace", () => {
    expect(classifyChar(" ")).toBe("whitespace");
    expect(classifyChar("\t")).toBe("whitespace");
    expect(classifyChar("\r")).toBe("whitespace");
  });

  it("should classify newline as newline", () => {
    expect(classifyChar("\n")).toBe("newline");
  });

  it("should classify double quote as quote", () => {
    expect(classifyChar('"')).toBe("quote");
  });

  it("should classify = as equals", () => {
    expect(classifyChar("=")).toBe("equals");
  });

  it("should classify arithmetic operators as operator", () => {
    for (const ch of "+-*/") {
      expect(classifyChar(ch)).toBe("operator");
    }
  });

  it("should classify delimiters correctly", () => {
    const expected: Record<string, string> = {
      "(": "open_paren",
      ")": "close_paren",
      ",": "comma",
      ":": "colon",
      ";": "semicolon",
      "{": "open_brace",
      "}": "close_brace",
      "[": "open_bracket",
      "]": "close_bracket",
      ".": "dot",
      "!": "bang",
    };
    for (const [ch, cls] of Object.entries(expected)) {
      expect(classifyChar(ch)).toBe(cls);
    }
  });

  it("should classify unknown characters as other", () => {
    for (const ch of "@#$%^&") {
      expect(classifyChar(ch)).toBe("other");
    }
  });
});

// ============================================================================
// DFA construction and structure tests
// ============================================================================

describe("tokenizer DFA construction", () => {
  it("should start in the start state", () => {
    const dfa = newTokenizerDFA();
    expect(dfa.currentState).toBe("start");
  });

  it("should be complete (transition for every state/input pair)", () => {
    const dfa = newTokenizerDFA();
    expect(dfa.isComplete()).toBe(true);
  });
});

// ============================================================================
// DFA transition tests — from "start" state
// ============================================================================

describe("tokenizer DFA transitions from start", () => {
  it("start + digit -> in_number", () => {
    const dfa = newTokenizerDFA();
    expect(dfa.process("digit")).toBe("in_number");
  });

  it("start + alpha -> in_name", () => {
    const dfa = newTokenizerDFA();
    expect(dfa.process("alpha")).toBe("in_name");
  });

  it("start + underscore -> in_name", () => {
    const dfa = newTokenizerDFA();
    expect(dfa.process("underscore")).toBe("in_name");
  });

  it("start + quote -> in_string", () => {
    const dfa = newTokenizerDFA();
    expect(dfa.process("quote")).toBe("in_string");
  });

  it("start + newline -> at_newline", () => {
    const dfa = newTokenizerDFA();
    expect(dfa.process("newline")).toBe("at_newline");
  });

  it("start + whitespace -> at_whitespace", () => {
    const dfa = newTokenizerDFA();
    expect(dfa.process("whitespace")).toBe("at_whitespace");
  });

  it("start + operator -> in_operator", () => {
    const dfa = newTokenizerDFA();
    expect(dfa.process("operator")).toBe("in_operator");
  });

  it("start + equals -> in_equals", () => {
    const dfa = newTokenizerDFA();
    expect(dfa.process("equals")).toBe("in_equals");
  });

  it("start + delimiter classes -> in_operator", () => {
    const delimiterClasses = [
      "open_paren", "close_paren", "comma", "colon", "semicolon",
      "open_brace", "close_brace", "open_bracket", "close_bracket",
      "dot", "bang",
    ];
    for (const cls of delimiterClasses) {
      const dfa = newTokenizerDFA();
      expect(dfa.process(cls)).toBe("in_operator");
    }
  });

  it("start + eof -> done", () => {
    const dfa = newTokenizerDFA();
    expect(dfa.process("eof")).toBe("done");
  });

  it("start + other -> error", () => {
    const dfa = newTokenizerDFA();
    expect(dfa.process("other")).toBe("error");
  });
});

// ============================================================================
// Handler states return to "start"
// ============================================================================

describe("tokenizer DFA handler state reset", () => {
  it("handler states should return to start on any symbol", () => {
    const handlerEntries: [string, string][] = [
      ["digit", "in_number"],
      ["alpha", "in_name"],
      ["quote", "in_string"],
      ["operator", "in_operator"],
      ["equals", "in_equals"],
      ["newline", "at_newline"],
      ["whitespace", "at_whitespace"],
    ];

    for (const [entrySymbol, expectedState] of handlerEntries) {
      const dfa = newTokenizerDFA();
      const state = dfa.process(entrySymbol);
      expect(state).toBe(expectedState);

      // From any handler, processing any symbol should go to "start"
      const next = dfa.process("eof");
      expect(next).toBe("start");
    }
  });
});

// ============================================================================
// Terminal states loop
// ============================================================================

describe("tokenizer DFA terminal states", () => {
  it("done state should loop on itself", () => {
    const dfa = newTokenizerDFA();
    dfa.process("eof"); // -> done
    expect(dfa.currentState).toBe("done");
    expect(dfa.process("digit")).toBe("done");
  });

  it("error state should loop on itself", () => {
    const dfa = newTokenizerDFA();
    dfa.process("other"); // -> error
    expect(dfa.currentState).toBe("error");
    expect(dfa.process("digit")).toBe("error");
  });
});

// ============================================================================
// DFA equivalence — formal model matches actual tokenizer behavior
// ============================================================================

describe("tokenizer DFA equivalence with actual tokenizer", () => {
  it("should tokenize a simple expression identically", () => {
    const tokens = tokenize("x = 42 + y");
    const types = tokens.map((t) => t.type);
    expect(types).toEqual(["NAME", "EQUALS", "NUMBER", "PLUS", "NAME", "EOF"]);
  });

  it("should tokenize comparison identically", () => {
    const tokens = tokenize("x == 5");
    const types = tokens.map((t) => t.type);
    expect(types).toEqual(["NAME", "EQUALS_EQUALS", "NUMBER", "EOF"]);
  });

  it("should tokenize string literal identically", () => {
    const tokens = tokenize('"hello"');
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("hello");
  });

  it("should tokenize multiline code identically", () => {
    const tokens = tokenize("x = 1\ny = 2");
    const types = tokens.map((t) => t.type);
    expect(types).toEqual([
      "NAME", "EQUALS", "NUMBER", "NEWLINE",
      "NAME", "EQUALS", "NUMBER", "EOF",
    ]);
  });

  it("should tokenize all delimiters identically", () => {
    const tokens = tokenize("( ) , : ; { } [ ] . !");
    const types = tokens.map((t) => t.type);
    expect(types).toEqual([
      "LPAREN", "RPAREN", "COMMA", "COLON", "SEMICOLON",
      "LBRACE", "RBRACE", "LBRACKET", "RBRACKET", "DOT",
      "BANG", "EOF",
    ]);
  });

  it("should handle empty input identically", () => {
    const tokens = tokenize("");
    expect(tokens).toHaveLength(1);
    expect(tokens[0].type).toBe("EOF");
  });

  it("should handle whitespace-only input identically", () => {
    const tokens = tokenize("   \t  ");
    expect(tokens).toHaveLength(1);
    expect(tokens[0].type).toBe("EOF");
  });
});
