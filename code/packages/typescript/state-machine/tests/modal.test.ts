/**
 * Tests for the Modal State Machine implementation.
 */

import { describe, expect, it } from "vitest";
import { DFA } from "../src/dfa.js";
import { ModalStateMachine } from "../src/modal.js";
import { transitionKey } from "../src/types.js";

// ============================================================
// Helpers — reusable mode DFA definitions
// ============================================================

/** DFA for the DATA mode: reads chars, detects '<' for tag open. */
function makeDataMode(): DFA {
  return new DFA(
    new Set(["text", "tag_detected"]),
    new Set(["char", "open_angle"]),
    new Map([
      [transitionKey("text", "char"), "text"],
      [transitionKey("text", "open_angle"), "tag_detected"],
      [transitionKey("tag_detected", "char"), "text"],
      [transitionKey("tag_detected", "open_angle"), "tag_detected"],
    ]),
    "text",
    new Set(["text"]),
  );
}

/** DFA for the TAG mode: reads tag name chars, detects '>' for close. */
function makeTagMode(): DFA {
  return new DFA(
    new Set(["reading_name", "tag_done"]),
    new Set(["char", "close_angle"]),
    new Map([
      [transitionKey("reading_name", "char"), "reading_name"],
      [transitionKey("reading_name", "close_angle"), "tag_done"],
      [transitionKey("tag_done", "char"), "reading_name"],
      [transitionKey("tag_done", "close_angle"), "tag_done"],
    ]),
    "reading_name",
    new Set(["tag_done"]),
  );
}

/** DFA for SCRIPT mode: reads raw chars until end-script detected. */
function makeScriptMode(): DFA {
  return new DFA(
    new Set(["raw"]),
    new Set(["char", "end_marker"]),
    new Map([
      [transitionKey("raw", "char"), "raw"],
      [transitionKey("raw", "end_marker"), "raw"],
    ]),
    "raw",
    new Set(["raw"]),
  );
}

/** Simplified HTML tokenizer with 3 modes. */
function makeHtmlTokenizer(): ModalStateMachine {
  return new ModalStateMachine(
    new Map([
      ["data", makeDataMode()],
      ["tag", makeTagMode()],
      ["script", makeScriptMode()],
    ]),
    new Map([
      [`data\0enter_tag`, "tag"],
      [`tag\0exit_tag`, "data"],
      [`tag\0enter_script`, "script"],
      [`script\0exit_script`, "data"],
    ]),
    "data",
  );
}

// ============================================================
// Construction Tests
// ============================================================

describe("Modal Construction", () => {
  it("should construct a valid modal machine", () => {
    const html = makeHtmlTokenizer();
    expect(html.currentMode).toBe("data");
    expect(html.modes.size).toBe(3);
  });

  it("should reject no modes", () => {
    expect(
      () => new ModalStateMachine(new Map(), new Map(), "data"),
    ).toThrow(/one mode/);
  });

  it("should reject invalid initial mode", () => {
    expect(
      () =>
        new ModalStateMachine(
          new Map([["data", makeDataMode()]]),
          new Map(),
          "missing",
        ),
    ).toThrow(/Initial mode/);
  });

  it("should reject invalid transition source mode", () => {
    expect(
      () =>
        new ModalStateMachine(
          new Map([["data", makeDataMode()]]),
          new Map([[`missing\0trigger`, "data"]]),
          "data",
        ),
    ).toThrow(/source/);
  });

  it("should reject invalid transition target mode", () => {
    expect(
      () =>
        new ModalStateMachine(
          new Map([["data", makeDataMode()]]),
          new Map([[`data\0trigger`, "missing"]]),
          "data",
        ),
    ).toThrow(/target/);
  });
});

// ============================================================
// Mode Switching Tests
// ============================================================

describe("Mode Switching", () => {
  it("should switch mode", () => {
    const html = makeHtmlTokenizer();
    expect(html.currentMode).toBe("data");
    html.switchMode("enter_tag");
    expect(html.currentMode).toBe("tag");
  });

  it("should return new mode name from switchMode", () => {
    const html = makeHtmlTokenizer();
    const result = html.switchMode("enter_tag");
    expect(result).toBe("tag");
  });

  it("should reset target DFA on switch", () => {
    const html = makeHtmlTokenizer();
    html.switchMode("enter_tag");
    html.process("char");
    html.process("close_angle");
    expect(html.activeMachine.currentState).toBe("tag_done");

    // Switch away and back — should reset
    html.switchMode("exit_tag");
    html.switchMode("enter_tag");
    expect(html.activeMachine.currentState).toBe("reading_name");
  });

  it("should handle data -> tag -> data cycle", () => {
    const html = makeHtmlTokenizer();
    html.switchMode("enter_tag");
    expect(html.currentMode).toBe("tag");
    html.switchMode("exit_tag");
    expect(html.currentMode).toBe("data");
  });

  it("should handle data -> tag -> script -> data cycle", () => {
    const html = makeHtmlTokenizer();
    html.switchMode("enter_tag");
    html.switchMode("enter_script");
    expect(html.currentMode).toBe("script");
    html.switchMode("exit_script");
    expect(html.currentMode).toBe("data");
  });

  it("should throw on invalid trigger", () => {
    const html = makeHtmlTokenizer();
    expect(() => html.switchMode("nonexistent_trigger")).toThrow(
      /No mode transition/,
    );
  });

  it("should record mode switches in trace", () => {
    const html = makeHtmlTokenizer();
    html.switchMode("enter_tag");
    html.switchMode("exit_tag");

    const trace = html.modeTrace;
    expect(trace.length).toBe(2);
    expect(trace[0].fromMode).toBe("data");
    expect(trace[0].trigger).toBe("enter_tag");
    expect(trace[0].toMode).toBe("tag");
    expect(trace[1].fromMode).toBe("tag");
    expect(trace[1].trigger).toBe("exit_tag");
    expect(trace[1].toMode).toBe("data");
  });
});

// ============================================================
// Processing Within Modes Tests
// ============================================================

describe("Processing in Modes", () => {
  it("should process events in data mode", () => {
    const html = makeHtmlTokenizer();
    const result = html.process("char");
    expect(result).toBe("text");
  });

  it("should process events in tag mode after switching", () => {
    const html = makeHtmlTokenizer();
    html.switchMode("enter_tag");
    const result = html.process("char");
    expect(result).toBe("reading_name");
    const result2 = html.process("close_angle");
    expect(result2).toBe("tag_done");
  });

  it("should process events in script mode", () => {
    const html = makeHtmlTokenizer();
    html.switchMode("enter_tag");
    html.switchMode("enter_script");
    const result = html.process("char");
    expect(result).toBe("raw");
  });

  it("should throw on invalid event for current mode", () => {
    const html = makeHtmlTokenizer();
    // "close_angle" is not in data mode's alphabet
    expect(() => html.process("close_angle")).toThrow();
  });

  it("should return active machine DFA", () => {
    const html = makeHtmlTokenizer();
    const dataDfa = html.activeMachine;
    expect(dataDfa.currentState).toBe("text");

    html.switchMode("enter_tag");
    const tagDfa = html.activeMachine;
    expect(tagDfa.currentState).toBe("reading_name");
  });
});

// ============================================================
// Reset Tests
// ============================================================

describe("Modal Reset", () => {
  it("should return to initial mode", () => {
    const html = makeHtmlTokenizer();
    html.switchMode("enter_tag");
    html.reset();
    expect(html.currentMode).toBe("data");
  });

  it("should clear mode trace", () => {
    const html = makeHtmlTokenizer();
    html.switchMode("enter_tag");
    html.switchMode("exit_tag");
    expect(html.modeTrace.length).toBe(2);

    html.reset();
    expect(html.modeTrace).toEqual([]);
  });

  it("should reset all sub-machine DFAs", () => {
    const html = makeHtmlTokenizer();
    html.switchMode("enter_tag");
    html.process("char");
    html.process("close_angle");

    html.reset();
    // Tag mode's DFA should be back at initial state
    html.switchMode("enter_tag");
    expect(html.activeMachine.currentState).toBe("reading_name");
  });
});
