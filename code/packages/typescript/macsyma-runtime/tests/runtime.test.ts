import { describe, expect, it } from "vitest";
import {
  ADD,
  MUL,
  POW,
  app,
  int,
  numberNode,
  sym,
} from "@coding-adventures/symbolic-ir";
import { VM } from "@coding-adventures/symbolic-vm";
import {
  ALL_SYMBOL,
  DISPLAY,
  EV,
  EXPAND,
  History,
  KILL,
  MACSYMA_NAME_TABLE,
  MacsymaBackend,
  MacsymaSession,
  RAT_SIMPLIFY,
  SUPPRESS,
  extendCompilerNameTable,
  evalSourceJson,
} from "../src/index.js";

describe("macsyma-runtime", () => {
  it("evaluates arithmetic programs", () => {
    const session = new MacsymaSession();
    const [result] = session.evalSource("1 + 2 * 3;");
    expect(result.output).toEqual(int(7));
    expect(result.display).toBe(true);
  });

  it("preserves suppressed statement metadata", () => {
    const session = new MacsymaSession();
    const results = session.evalSource("x : 5$ x + 1;");
    expect(results).toHaveLength(2);
    expect(results[0].display).toBe(false);
    expect(results[1].display).toBe(true);
    expect(results[1].output).toEqual(int(6));
  });

  it("exports runtime-owned heads", () => {
    expect(DISPLAY).toEqual(sym("Display"));
    expect(SUPPRESS).toEqual(sym("Suppress"));
    expect(KILL).toEqual(sym("Kill"));
    expect(EV).toEqual(sym("Ev"));
  });

  it("exports MACSYMA name-table routes and extends maps idempotently", () => {
    expect(MACSYMA_NAME_TABLE.get("kill")).toEqual(KILL);
    expect(MACSYMA_NAME_TABLE.get("ev")).toEqual(EV);
    expect(MACSYMA_NAME_TABLE.get("expand")).toEqual(EXPAND);
    expect(MACSYMA_NAME_TABLE.get("ratsimp")).toEqual(RAT_SIMPLIFY);

    const target = new Map<string, typeof KILL>();
    extendCompilerNameTable(target);
    const snapshot = [...target.entries()];
    extendCompilerNameTable(target);
    expect([...target.entries()]).toEqual(snapshot);
  });

  it("extends object name tables idempotently", () => {
    const target: Record<string, typeof KILL | undefined> = {};
    extendCompilerNameTable(target);
    const snapshot = { ...target };
    extendCompilerNameTable(target);
    expect(target).toEqual(snapshot);
    expect(target.kill).toEqual(KILL);
  });

  it("canonicalizes runtime name-table calls after grammar compilation", () => {
    const session = new MacsymaSession();
    const [killResult] = session.evalSource("kill(x);");
    const [evResult] = session.evalSource("ev(1 + 2, numer);");

    expect(killResult.input).toEqual(app(KILL, [sym("x")]));
    expect(evResult.input).toEqual(app(EV, [app(ADD, [int(1), int(2)]), sym("numer")]));
    expect(evResult.output).toEqual(numberNode(3));
  });

  it("records and resolves history references", () => {
    const session = new MacsymaSession();
    const results = session.evalSource("2 + 3; % * 2; %i1; %o2;");
    expect(results.map((result) => result.output)).toEqual([
      int(5),
      int(10),
      int(5),
      int(10),
    ]);
    expect(session.history().getInput(1)).toEqual(app(ADD, [int(2), int(3)]));
    expect(session.history().getOutput(1)).toEqual(int(5));
    expect(session.history().nextInputIndex()).toBe(5);
  });

  it("exposes Python-parity history symbol resolution", () => {
    const history = new History();
    const input = app(ADD, [int(2), int(3)]);
    history.recordInput(input);
    history.recordOutput(int(5));
    history.recordOutput(int(10));

    expect(history.resolveHistorySymbol("%")).toEqual(int(10));
    expect(history.resolveHistorySymbol("%i1")).toEqual(input);
    expect(history.resolveHistorySymbol("%o1")).toEqual(int(5));
    expect(history.resolveHistorySymbol("%o2")).toEqual(int(10));
  });

  it("returns undefined for unknown or out-of-range history symbols", () => {
    const history = new History();
    history.recordInput(sym("x"));
    history.recordOutput(int(1));

    expect(history.resolveHistorySymbol("xyz")).toBeUndefined();
    expect(history.resolveHistorySymbol("%foo")).toBeUndefined();
    expect(history.resolveHistorySymbol("%i999")).toBeUndefined();
    expect(history.resolveHistorySymbol("%o999")).toBeUndefined();
    expect(history.resolveHistorySymbol("%i0")).toBeUndefined();
    expect(history.resolveHistorySymbol("%o0")).toBeUndefined();
  });

  it("returns undefined for history symbols before any history is recorded", () => {
    const history = new History();
    expect(history.resolveHistorySymbol("%")).toBeUndefined();
    expect(history.resolveHistorySymbol("%i1")).toBeUndefined();
    expect(history.resolveHistorySymbol("%o1")).toBeUndefined();
  });

  it("clears resolvable history symbols on reset", () => {
    const history = new History();
    history.recordInput(int(1));
    history.recordOutput(int(2));
    history.reset();

    expect(history.nextInputIndex()).toBe(1);
    expect(history.resolveHistorySymbol("%")).toBeUndefined();
    expect(history.resolveHistorySymbol("%i1")).toBeUndefined();
    expect(history.resolveHistorySymbol("%o1")).toBeUndefined();
  });

  it("keeps env bindings ahead of history fallback in backend lookup", () => {
    const history = new History();
    history.recordOutput(int(42));
    const backend = new MacsymaBackend(history);
    backend.bind("%o1", int(99));

    expect(backend.lookup("%o1")).toEqual(int(99));
    expect(backend.lookup("%")).toEqual(int(42));
    expect(backend.lookup("not_history")).toBeUndefined();
  });

  it("registers held Kill and Ev runtime handlers", () => {
    const backend = new MacsymaBackend(new History());
    expect(backend.handlers().has(KILL.name)).toBe(true);
    expect(backend.handlers().has(EV.name)).toBe(true);
    expect(backend.holdHeads().has(KILL.name)).toBe(true);
    expect(backend.holdHeads().has(EV.name)).toBe(true);
  });

  it("kills single and multiple bindings without evaluating names first", () => {
    const session = new MacsymaSession();
    const results = session.evalSource("x : 5; y : 6; kill(x, y); x; y;");

    expect(results[2].output).toEqual(sym("done"));
    expect(results[3].output).toEqual(sym("x"));
    expect(results[4].output).toEqual(sym("y"));
  });

  it("kill(all) clears bindings and history", () => {
    const history = new History();
    history.recordInput(sym("old"));
    history.recordOutput(int(42));
    const backend = new MacsymaBackend(history);
    backend.bind("x", int(5));
    const vm = new VM(backend);

    vm.eval(app(KILL, [ALL_SYMBOL]));

    expect(backend.lookup("x")).toBeUndefined();
    expect(backend.lookup("%pi")).toEqual(numberNode(Math.PI));
    expect(history.nextInputIndex()).toBe(1);
    expect(history.lastOutput()).toBeUndefined();
  });

  it("session kill(all) leaves the next input index reset", () => {
    const session = new MacsymaSession();
    session.evalSource("x : 5; 1; kill(all);");
    expect(session.history().nextInputIndex()).toBe(1);
    expect(session.evalSource("x;")[0].output).toEqual(sym("x"));
  });

  it("Ev numer and float coerce evaluated exact results to floats", () => {
    const session = new MacsymaSession();
    const results = session.evalSource("ev(1 + 2, numer); ev(42, float);");

    expect(results[0].output).toEqual(numberNode(3));
    expect(results[1].output).toEqual(numberNode(42));
  });

  it("Ev transform flags keep the evaluated expression when no handler is available", () => {
    const session = new MacsymaSession();
    const [result] = session.evalSource("ev(x + 0, expand);");

    expect(result.output).toEqual(sym("x"));
  });

  it("evaluates function definitions across statements", () => {
    const session = new MacsymaSession();
    const results = session.evalSource("f(x) := x^2; f(4);");
    expect(results[0].output).toEqual(sym("f"));
    expect(results[1].output).toEqual(int(16));
  });

  it("leaves symbolic results unevaluated when needed", () => {
    const session = new MacsymaSession();
    const [result] = session.evalSource("(x + 0) * (y^2);");
    expect(result.output).toEqual(app(MUL, [sym("x"), app(POW, [sym("y"), int(2)])]));
  });

  it("pre-binds MACSYMA constants", () => {
    const session = new MacsymaSession();
    const results = session.evalSource("%pi; %e; %i;");
    expect(results[0].output).toEqual({ kind: "float", value: Math.PI });
    expect(results[1].output).toEqual({ kind: "float", value: Math.E });
    expect(results[2].output).toEqual(sym("ImaginaryUnit"));
  });

  it("emits JSON-safe results for browser callers", () => {
    const payload = JSON.parse(evalSourceJson("1 + 2; x$"));
    expect(payload.ok).toBe(true);
    expect(payload.results[0].outputIr).toEqual({ kind: "integer", value: "3" });
    expect(payload.visibleOutputs).toEqual(["3"]);
    expect(payload.history.nextInputIndex).toBe(3);
  });

  it("reports errors as JSON without losing history", () => {
    const session = new MacsymaSession();
    session.evalSource("1;");
    const payload = JSON.parse(session.evalJson("1 + ;"));
    expect(payload.ok).toBe(false);
    expect(payload.error.kind).toBe("runtime");
    expect(payload.history.inputCount).toBe(1);
  });

  it("can reset history", () => {
    const session = new MacsymaSession();
    session.evalSource("1; 2;");
    session.resetHistory();
    expect(session.history().nextInputIndex()).toBe(1);
    expect(session.history().lastOutput()).toBeUndefined();
  });
});
