import { describe, expect, it } from "vitest";
import {
  ADD,
  MUL,
  POW,
  app,
  int,
  sym,
} from "@coding-adventures/symbolic-ir";
import { MacsymaSession, evalSourceJson } from "../src/index.js";

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
