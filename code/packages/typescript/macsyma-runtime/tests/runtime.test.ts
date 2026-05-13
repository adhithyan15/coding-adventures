import { describe, expect, it } from "vitest";
import {
  ADD,
  ASSUME,
  EQUAL,
  EXP,
  FALSE,
  FORGET,
  GREATER,
  GREATER_EQUAL,
  IS,
  LESS,
  LESS_EQUAL,
  LIST,
  LOG,
  MUL,
  POW,
  RULE,
  SIN,
  SOLVE,
  SUB,
  SUBST,
  TRUE,
  app,
  int,
  numberNode,
  rational,
  sym,
} from "@coding-adventures/symbolic-ir";
import { VM } from "@coding-adventures/symbolic-vm";
import {
  ALL_SYMBOL,
  APPEND,
  APPLY,
  DECLARE,
  DISPLAY,
  EV,
  EXPAND,
  FIRST,
  FLATTEN,
  History,
  JOIN,
  KILL,
  LAST,
  LENGTH,
  MACSYMA_NAME_TABLE,
  MacsymaBackend,
  MacsymaSession,
  MAP,
  PART,
  PROP_VARS,
  PROPERTIES,
  RAT_SIMPLIFY,
  RANGE,
  REST,
  REVERSE,
  SORT,
  SIMPLIFY,
  SUPPRESS,
  TRIG_EXPAND,
  TRIG_REDUCE,
  TRIG_SIMPLIFY,
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
    expect(DECLARE).toEqual(sym("Declare"));
    expect(PROPERTIES).toEqual(sym("Properties"));
    expect(PROP_VARS).toEqual(sym("PropVars"));
  });

  it("exports MACSYMA name-table routes and extends maps idempotently", () => {
    expect(MACSYMA_NAME_TABLE.get("kill")).toEqual(KILL);
    expect(MACSYMA_NAME_TABLE.get("ev")).toEqual(EV);
    expect(MACSYMA_NAME_TABLE.get("assume")).toEqual(ASSUME);
    expect(MACSYMA_NAME_TABLE.get("forget")).toEqual(FORGET);
    expect(MACSYMA_NAME_TABLE.get("is")).toEqual(IS);
    expect(MACSYMA_NAME_TABLE.get("declare")).toEqual(DECLARE);
    expect(MACSYMA_NAME_TABLE.get("properties")).toEqual(PROPERTIES);
    expect(MACSYMA_NAME_TABLE.get("propvars")).toEqual(PROP_VARS);
    expect(MACSYMA_NAME_TABLE.get("expand")).toEqual(EXPAND);
    expect(MACSYMA_NAME_TABLE.get("simplify")).toEqual(SIMPLIFY);
    expect(MACSYMA_NAME_TABLE.get("ratsimp")).toEqual(RAT_SIMPLIFY);
    expect(MACSYMA_NAME_TABLE.get("trigsimp")).toEqual(TRIG_SIMPLIFY);
    expect(MACSYMA_NAME_TABLE.get("trigexpand")).toEqual(TRIG_EXPAND);
    expect(MACSYMA_NAME_TABLE.get("trigreduce")).toEqual(TRIG_REDUCE);

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

  it("tracks the showtime option flag through normal assignments", () => {
    const session = new MacsymaSession();
    const [enabled, timed, disabled, untimed] = session.evalSource(
      "showtime:true$ 2 + 3; showtime:false$ 4 + 5;",
    );

    expect(enabled.output).toEqual(TRUE);
    expect(timed.output).toEqual(int(5));
    expect(timed.timingText).toMatch(/^Evaluation took \d+\.\d{6} seconds\.$/);
    expect(disabled.output).toEqual(FALSE);
    expect(disabled.timingText).toBeUndefined();
    expect(untimed.output).toEqual(int(9));
    expect(untimed.timingText).toBeUndefined();
  });

  it("reports showtime diagnostics for suppressed statements", () => {
    const session = new MacsymaSession();
    const [, suppressed] = session.evalSource("showtime:true$ 2 + 3$");

    expect(suppressed.display).toBe(false);
    expect(suppressed.output).toEqual(int(5));
    expect(suppressed.timingText).toMatch(/^Evaluation took \d+\.\d{6} seconds\.$/);
  });

  it("restores showtime to false when killed", () => {
    const backend = new MacsymaBackend(new History());
    backend.bind("showtime", TRUE);

    backend.unbind("showtime");

    expect(backend.showtime).toBe(false);
    expect(backend.lookup("showtime")).toEqual(FALSE);
  });

  it("includes showtime diagnostics in JSON visible outputs", () => {
    const payload = JSON.parse(evalSourceJson("showtime:true$ 2 + 3$"));
    const [, result] = payload.results;

    expect(result.display).toBe(false);
    expect(result.timingText).toMatch(/^Evaluation took \d+\.\d{6} seconds\.$/);
    expect(payload.visibleOutputs).toEqual([result.timingText]);
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

  it("registers held runtime handlers", () => {
    const backend = new MacsymaBackend(new History());
    expect(backend.handlers().has(KILL.name)).toBe(true);
    expect(backend.handlers().has(EV.name)).toBe(true);
    expect(backend.handlers().has(ASSUME.name)).toBe(true);
    expect(backend.handlers().has(FORGET.name)).toBe(true);
    expect(backend.handlers().has(IS.name)).toBe(true);
    expect(backend.handlers().has(DECLARE.name)).toBe(true);
    expect(backend.handlers().has(PROPERTIES.name)).toBe(true);
    expect(backend.handlers().has(PROP_VARS.name)).toBe(true);
    expect(backend.handlers().has(SOLVE.name)).toBe(true);
    expect(backend.handlers().has(SUBST.name)).toBe(true);
    expect(backend.handlers().has(SIMPLIFY.name)).toBe(true);
    expect(backend.handlers().has(RAT_SIMPLIFY.name)).toBe(true);
    expect(backend.handlers().has(TRIG_SIMPLIFY.name)).toBe(true);
    expect(backend.handlers().has(TRIG_EXPAND.name)).toBe(true);
    expect(backend.handlers().has(TRIG_REDUCE.name)).toBe(true);
    expect(backend.handlers().has(LENGTH.name)).toBe(true);
    expect(backend.holdHeads().has(KILL.name)).toBe(true);
    expect(backend.holdHeads().has(EV.name)).toBe(true);
    expect(backend.holdHeads().has(ASSUME.name)).toBe(true);
    expect(backend.holdHeads().has(FORGET.name)).toBe(true);
    expect(backend.holdHeads().has(IS.name)).toBe(true);
    expect(backend.holdHeads().has(DECLARE.name)).toBe(true);
    expect(backend.holdHeads().has(PROPERTIES.name)).toBe(true);
    expect(backend.holdHeads().has(PROP_VARS.name)).toBe(true);
    expect(backend.holdHeads().has(SOLVE.name)).toBe(true);
    expect(backend.holdHeads().has(SUBST.name)).toBe(true);
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

  it("assume, is, and forget share session assumptions", () => {
    const session = new MacsymaSession();
    const [assumeResult, trueResult, forgetResult, unknownResult] = session.evalSource(
      "assume(x > 0); is(x > 0); forget(); is(x > 0);",
    );

    expect(assumeResult.output).toEqual(sym("done"));
    expect(trueResult.output).toEqual(TRUE);
    expect(forgetResult.output).toEqual(sym("done"));
    expect(unknownResult.output).toEqual(sym("unknown"));
  });

  it("declare feeds properties into is queries", () => {
    const session = new MacsymaSession();
    const [declareResult, query] = session.evalSource("declare(x, positive); is(x > 0);");

    expect(declareResult.input).toEqual(app(DECLARE, [sym("x"), sym("positive")]));
    expect(declareResult.output).toEqual(sym("done"));
    expect(query.output).toEqual(TRUE);
  });

  it("properties lists declared properties deterministically", () => {
    const session = new MacsymaSession();
    const [, result] = session.evalSource("declare(n, integer, n, positive); properties(n);");

    expect(result.output).toEqual(app(LIST, [sym("integer"), sym("positive")]));
  });

  it("propvars lists symbols with declared properties deterministically", () => {
    const session = new MacsymaSession();
    const [, , result] = session.evalSource("declare(z, integer); declare(a, positive); propvars();");

    expect(result.output).toEqual(app(LIST, [sym("a"), sym("z")]));
  });

  it("properties queries raw symbols even when they are bound", () => {
    const session = new MacsymaSession();
    const [, , result] = session.evalSource("x : 10; declare(x, integer); properties(x);");

    expect(result.output).toEqual(app(LIST, [sym("integer")]));
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

  it("evaluates simplify and ratsimp through cas-simplify", () => {
    const x = sym("x");
    const expr = app(MUL, [app(ADD, [x, int(0)]), int(1)]);
    const session = new MacsymaSession();
    const [simplifyResult, ratsimpResult, badArity] = session.evalStatements([
      app(sym("simplify"), [expr]),
      app(RAT_SIMPLIFY, [expr]),
      app(SIMPLIFY, [expr, int(1)]),
    ]);

    expect(simplifyResult.input).toEqual(app(SIMPLIFY, [expr]));
    expect(simplifyResult.output).toEqual(x);
    expect(ratsimpResult.output).toEqual(x);
    expect(badArity.output).toEqual(app(SIMPLIFY, [x, int(1)]));
  });

  it("evaluates trigsimp and trigexpand through cas-trig", () => {
    const x = sym("x");
    const y = sym("y");
    const session = new MacsymaSession();
    const [simplifyResult, expandResult] = session.evalStatements([
      app(TRIG_SIMPLIFY, [app(ADD, [app(SIN, [int(0)]), app(sym("Cos"), [int(0)])])]),
      app(TRIG_EXPAND, [app(SIN, [app(ADD, [x, y])])]),
    ]);

    expect(simplifyResult.output).toEqual(int(1));
    expect(expandResult.output).toEqual(app(ADD, [
      app(MUL, [app(SIN, [x]), app(sym("Cos"), [y])]),
      app(MUL, [app(sym("Cos"), [x]), app(SIN, [y])]),
    ]));
  });

  it("routes Ev simplify flags through runtime handlers", () => {
    const session = new MacsymaSession();
    const [ratsimpResult, trigsimpResult] = session.evalSource(
      "ev((x + 0) * 1, ratsimp); ev(sin(0) + cos(0), trigsimp);",
    );

    expect(ratsimpResult.output).toEqual(sym("x"));
    expect(trigsimpResult.output).toEqual(int(1));
  });

  it("routes Ev display2d through the MACSYMA box pretty-printer", () => {
    const payload = JSON.parse(evalSourceJson("ev(1/(x + 1), display2d);"));
    const [result] = payload.results;

    expect(payload.visibleOutputs).toEqual([result.outputText]);
    expect(result.outputText).toContain("\n");
    expect(result.outputText).toContain("─");
    expect(result.outputText).toContain("x + 1");
  });

  it("evaluates trigreduce through cas-trig", () => {
    const x = sym("x");
    const session = new MacsymaSession();
    const [direct, viaEv] = session.evalStatements([
      app(TRIG_REDUCE, [app(POW, [app(SIN, [x]), int(2)])]),
      app(EV, [app(POW, [app(sym("Cos"), [x]), int(2)]), sym("trigreduce")]),
    ]);

    expect(direct.output).toEqual(app(MUL, [
      rational(1, 2),
      app(SUB, [int(1), app(sym("Cos"), [app(MUL, [int(2), x])])]),
    ]));
    expect(viaEv.output).toEqual(app(MUL, [
      rational(1, 2),
      app(ADD, [int(1), app(sym("Cos"), [app(MUL, [int(2), x])])]),
    ]));
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

  it("evaluates linsolve linear systems through the cas-solve handler", () => {
    const x = sym("x");
    const y = sym("y");
    const session = new MacsymaSession();
    const [result] = session.evalStatements([
      app(sym("linsolve"), [
        app(LIST, [
          app(EQUAL, [app(ADD, [x, y]), int(3)]),
          app(EQUAL, [app(SUB, [x, y]), int(1)]),
        ]),
        app(LIST, [x, y]),
      ]),
    ]);

    expect(result.input).toEqual(app(SOLVE, [
      app(LIST, [
        app(EQUAL, [app(ADD, [x, y]), int(3)]),
        app(EQUAL, [app(SUB, [x, y]), int(1)]),
      ]),
      app(LIST, [x, y]),
    ]));
    expect(result.output).toEqual(app(LIST, [
      app(RULE, [x, int(2)]),
      app(RULE, [y, int(1)]),
    ]));
  });

  it("keeps unsolved or non-linear Solve calls unevaluated", () => {
    const x = sym("x");
    const session = new MacsymaSession();
    const expr = app(SOLVE, [
      app(LIST, [app(EQUAL, [app(POW, [x, int(2)]), int(4)])]),
      app(LIST, [x]),
    ]);

    const [result] = session.evalStatements([expr]);
    expect(result.output).toEqual(expr);
  });

  it("solves polynomial inequalities through the cas-solve handler", () => {
    const x = sym("x");
    const session = new MacsymaSession();
    const [linear, quadratic, bounded, allReals] = session.evalStatements([
      app(SOLVE, [
        app(GREATER, [app(SUB, [x, int(1)]), int(0)]),
        x,
      ]),
      app(SOLVE, [
        app(GREATER, [app(SUB, [app(POW, [x, int(2)]), int(1)]), int(0)]),
        x,
      ]),
      app(SOLVE, [
        app(LESS_EQUAL, [app(SUB, [app(POW, [x, int(2)]), int(1)]), int(0)]),
        x,
      ]),
      app(SOLVE, [
        app(GREATER_EQUAL, [app(POW, [x, int(2)]), int(0)]),
        x,
      ]),
    ]);

    expect(linear.output).toEqual(app(LIST, [
      app(GREATER, [x, int(1)]),
    ]));
    expect(quadratic.output).toEqual(app(LIST, [
      app(LESS, [x, int(-1)]),
      app(GREATER, [x, int(1)]),
    ]));
    expect(bounded.output).toEqual(app(LIST, [
      app(sym("And"), [
        app(GREATER_EQUAL, [x, int(-1)]),
        app(LESS_EQUAL, [x, int(1)]),
      ]),
    ]));
    expect(allReals.output).toEqual(app(LIST, [
      app(GREATER_EQUAL, [int(0), int(0)]),
    ]));
  });

  it("keeps unsupported inequality Solve calls unevaluated", () => {
    const x = sym("x");
    const session = new MacsymaSession();
    const expr = app(SOLVE, [
      app(GREATER, [app(sym("Sin"), [x]), int(0)]),
      x,
    ]);

    const [result] = session.evalStatements([expr]);
    expect(result.output).toEqual(expr);
  });

  it("solves direct transcendental equations through the cas-solve handler", () => {
    const x = sym("x");
    const session = new MacsymaSession();
    const [expResult, sinResult] = session.evalStatements([
      app(SOLVE, [
        app(EQUAL, [app(EXP, [x]), int(2)]),
        x,
      ]),
      app(SOLVE, [
        app(EQUAL, [app(SIN, [x]), int(0)]),
        x,
      ]),
    ]);

    expect(expResult.output).toEqual(app(LIST, [
      app(LOG, [int(2)]),
    ]));
    expect(sinResult.output).toEqual(app(LIST, [
      app(sym("Add"), [
        app(sym("Asin"), [int(0)]),
        app(MUL, [int(2), app(MUL, [sym("%pi"), sym("FreeInteger")])]),
      ]),
      app(sym("Add"), [
        app(SUB, [sym("%pi"), app(sym("Asin"), [int(0)])]),
        app(MUL, [int(2), app(MUL, [sym("%pi"), sym("FreeInteger")])]),
      ]),
    ]));
  });

  it("keeps unsupported transcendental Solve calls unevaluated", () => {
    const x = sym("x");
    const session = new MacsymaSession();
    const expr = app(SOLVE, [
      app(EQUAL, [app(SIN, [app(SIN, [x])]), int(0)]),
      x,
    ]);

    const [result] = session.evalStatements([expr]);
    expect(result.output).toEqual(expr);
  });

  it("evaluates structural substitution through cas-substitution", () => {
    const x = sym("x");
    const session = new MacsymaSession();
    const [simple, compound] = session.evalStatements([
      app(SUBST, [int(3), x, app(POW, [x, int(2)])]),
      app(SUBST, [
        sym("z"),
        app(ADD, [x, int(1)]),
        app(MUL, [app(ADD, [x, int(1)]), app(ADD, [x, int(1)])]),
      ]),
    ]);

    expect(simple.output).toEqual(app(POW, [int(3), int(2)]));
    expect(compound.output).toEqual(app(MUL, [sym("z"), sym("z")]));
  });

  it("keeps substitution variables unevaluated", () => {
    const session = new MacsymaSession();
    const [bindResult, substResult] = session.evalSource("x : 5; subst(3, x, x^2);");

    expect(bindResult.output).toEqual(int(5));
    expect(substResult.output).toEqual(app(POW, [int(3), int(2)]));
  });

  it("keeps invalid substitution calls unevaluated", () => {
    const session = new MacsymaSession();
    const expr = app(SUBST, [int(3), sym("x")]);
    const [result] = session.evalStatements([expr]);

    expect(result.output).toEqual(expr);
  });

  it("evaluates deterministic list operations through cas-list-operations", () => {
    const xs = app(LIST, [int(1), int(2), int(3)]);
    const nested = app(LIST, [int(1), app(LIST, [int(2), app(LIST, [int(3)])])]);
    const session = new MacsymaSession();
    const [
      lengthResult,
      firstResult,
      restResult,
      lastResult,
      reverseResult,
      appendResult,
      joinResult,
      rangeResult,
      partResult,
      mapResult,
      applyResult,
      sortResult,
      flattenResult,
    ] = session.evalStatements([
      app(LENGTH, [xs]),
      app(FIRST, [xs]),
      app(REST, [xs]),
      app(LAST, [xs]),
      app(REVERSE, [xs]),
      app(APPEND, [app(LIST, [int(1)]), app(LIST, [int(2), int(3)])]),
      app(JOIN, [app(LIST, [int(1)]), app(LIST, [int(2)])]),
      app(RANGE, [int(1), int(5), int(2)]),
      app(PART, [xs, int(-1)]),
      app(MAP, [sym("f"), app(LIST, [sym("x"), sym("y")])]),
      app(APPLY, [ADD, app(LIST, [sym("x"), sym("y")])]),
      app(SORT, [app(LIST, [sym("b"), sym("a")])]),
      app(FLATTEN, [nested, int(-1)]),
    ]);

    expect(lengthResult.output).toEqual(int(3));
    expect(firstResult.output).toEqual(int(1));
    expect(restResult.output).toEqual(app(LIST, [int(2), int(3)]));
    expect(lastResult.output).toEqual(int(3));
    expect(reverseResult.output).toEqual(app(LIST, [int(3), int(2), int(1)]));
    expect(appendResult.output).toEqual(app(LIST, [int(1), int(2), int(3)]));
    expect(joinResult.output).toEqual(app(LIST, [int(1), int(2)]));
    expect(rangeResult.output).toEqual(app(LIST, [int(1), int(3), int(5)]));
    expect(partResult.output).toEqual(int(3));
    expect(mapResult.output).toEqual(app(LIST, [
      app(sym("f"), [sym("x")]),
      app(sym("f"), [sym("y")]),
    ]));
    expect(applyResult.output).toEqual(app(ADD, [sym("x"), sym("y")]));
    expect(sortResult.output).toEqual(app(LIST, [sym("a"), sym("b")]));
    expect(flattenResult.output).toEqual(app(LIST, [int(1), int(2), int(3)]));
  });

  it("keeps invalid list operation calls unevaluated", () => {
    const session = new MacsymaSession();
    const badPart = app(PART, [app(LIST, [int(1)]), int(0)]);
    const badLength = app(LENGTH, [sym("x")]);
    const [partResult, lengthResult] = session.evalStatements([badPart, badLength]);

    expect(partResult.output).toEqual(badPart);
    expect(lengthResult.output).toEqual(badLength);
  });

  it("returns rational linsolve results", () => {
    const x = sym("x");
    const y = sym("y");
    const session = new MacsymaSession();
    const [result] = session.evalStatements([
      app(SOLVE, [
        app(LIST, [
          app(EQUAL, [app(ADD, [app(MUL, [int(2), x]), app(MUL, [int(3), y])]), int(7)]),
          app(EQUAL, [app(SUB, [app(MUL, [int(4), x]), y]), int(1)]),
        ]),
        app(LIST, [x, y]),
      ]),
    ]);

    expect(result.output).toEqual(app(LIST, [
      app(RULE, [x, rational(5, 7)]),
      app(RULE, [y, rational(13, 7)]),
    ]));
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
