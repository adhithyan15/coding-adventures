/**
 * Tests for the `load("name")` runtime directive (Track M2).
 *
 * Acceptance contract mirrored from `macsyma-truly-finish-plan.md` §M1,
 * which Python implemented in commit `dc78e0931`.  Each test here maps
 * 1:1 to a Python test in `test_load_package.py` so a future audit can
 * walk the two files side-by-side.
 *
 *   - Without `load`, orthopoly heads round-trip unevaluated.
 *   - After `load("orthopoly")`, the closed-form polynomial fires.
 *   - Unknown names raise `MacsymaUserError` with a helpful message.
 *   - Re-loading is idempotent.
 *   - Loaded state is per-session (two backends stay independent).
 *   - Regression: non-orthopoly ops still work without a load.
 */

import { describe, expect, it } from "vitest";
import {
  app,
  int,
  stringNode,
  sym,
  type IRApply,
} from "@coding-adventures/symbolic-ir";
import { VM } from "@coding-adventures/symbolic-vm";

import {
  History,
  LOAD,
  MACSYMA_NAME_TABLE,
  MacsymaBackend,
  MacsymaUserError,
} from "../src/index.js";

// ---------------------------------------------------------------------
// Fixtures and helpers
// ---------------------------------------------------------------------

function freshSession(): { vm: VM; backend: MacsymaBackend } {
  const backend = new MacsymaBackend(new History());
  return { vm: new VM(backend), backend };
}

function loadPackage(vm: VM, name: string): unknown {
  return vm.eval(app(LOAD, [stringNode(name)]));
}

function legendreP(n: number, xName: string): IRApply {
  return app(sym("LegendreP"), [int(n), sym(xName)]) as IRApply;
}

// ---------------------------------------------------------------------
// 1. Without load, orthopoly heads stay unevaluated.
// ---------------------------------------------------------------------

describe("Track M2 — load directive, unloaded round-trip", () => {
  it("legendre_p(3, x) round-trips unevaluated before load", () => {
    const { vm } = freshSession();
    const result = vm.eval(legendreP(3, "x"));
    expect(result.kind).toBe("apply");
    if (result.kind !== "apply") throw new Error("unreachable");
    expect(result.head.kind === "symbol" && result.head.name).toBe("LegendreP");
    expect(result.args).toEqual([int(3), sym("x")]);
  });

  it("chebyshev_t(4, x) round-trips unevaluated before load", () => {
    const { vm } = freshSession();
    const result = vm.eval(app(sym("ChebyshevT"), [int(4), sym("x")]));
    expect(result.kind).toBe("apply");
    if (result.kind !== "apply") throw new Error("unreachable");
    expect(result.head.kind === "symbol" && result.head.name).toBe(
      "ChebyshevT",
    );
  });

  it("hermite_h(2, x) round-trips unevaluated before load", () => {
    const { vm } = freshSession();
    const result = vm.eval(app(sym("HermiteH"), [int(2), sym("x")]));
    expect(result.kind).toBe("apply");
    if (result.kind !== "apply") throw new Error("unreachable");
    expect(result.head.kind === "symbol" && result.head.name).toBe("HermiteH");
  });
});

// ---------------------------------------------------------------------
// 2. After load("orthopoly"), closed-form reductions kick in.
// ---------------------------------------------------------------------

describe("Track M2 — load directive, reductions after load", () => {
  it("legendre_p(3, 2) reduces to 17 via the Bonnet recurrence", () => {
    // P_3(x) = (5x^3 − 3x)/2; at x = 2 → (40 − 6)/2 = 17.  We pass the
    // concrete integer directly rather than calling `Subst` because the
    // TS Subst handler does structural substitution without re-eval —
    // a pre-existing implementation detail that's irrelevant to the
    // orthopoly contract we're testing here.
    const { vm } = freshSession();
    loadPackage(vm, "orthopoly");
    const result = vm.eval(app(sym("LegendreP"), [int(3), int(2)]));
    expect(result).toEqual(int(17));
  });

  it("legendre_p(0, x) = 1 and legendre_p(1, x) = x are seed values", () => {
    const { vm } = freshSession();
    loadPackage(vm, "orthopoly");
    expect(vm.eval(legendreP(0, "x"))).toEqual(int(1));
    expect(vm.eval(legendreP(1, "x"))).toEqual(sym("x"));
  });

  it("chebyshev_t(4, 1) = 1 (T_n(1) is always 1)", () => {
    const { vm } = freshSession();
    loadPackage(vm, "orthopoly");
    const result = vm.eval(app(sym("ChebyshevT"), [int(4), int(1)]));
    expect(result).toEqual(int(1));
  });

  it("chebyshev_u(3, 1) = 4 (U_n(1) = n + 1)", () => {
    const { vm } = freshSession();
    loadPackage(vm, "orthopoly");
    const result = vm.eval(app(sym("ChebyshevU"), [int(3), int(1)]));
    expect(result).toEqual(int(4));
  });

  it("hermite_h(3, 1) = −4 (physicists' convention: 8x^3 − 12x at 1)", () => {
    const { vm } = freshSession();
    loadPackage(vm, "orthopoly");
    const result = vm.eval(app(sym("HermiteH"), [int(3), int(1)]));
    expect(result).toEqual(int(-4));
  });
});

// ---------------------------------------------------------------------
// 3. Passthrough heads — symbols known after load, no reduction.
// ---------------------------------------------------------------------

describe("Track M2 — passthrough heads", () => {
  it("bessel_j(0, x) is unevaluated even after load (no closed form)", () => {
    const { vm } = freshSession();
    loadPackage(vm, "orthopoly");
    const result = vm.eval(app(sym("BesselJ"), [int(0), sym("x")]));
    expect(result.kind).toBe("apply");
    if (result.kind !== "apply") throw new Error("unreachable");
    expect(result.head.kind === "symbol" && result.head.name).toBe("BesselJ");
  });

  it("legendre_q(2, x) is unevaluated after load", () => {
    const { vm } = freshSession();
    loadPackage(vm, "orthopoly");
    const result = vm.eval(app(sym("LegendreQ"), [int(2), sym("x")]));
    expect(result.kind).toBe("apply");
    if (result.kind !== "apply") throw new Error("unreachable");
    expect(result.head.kind === "symbol" && result.head.name).toBe("LegendreQ");
  });
});

// ---------------------------------------------------------------------
// 4. Allowlist enforcement.
// ---------------------------------------------------------------------

describe("Track M2 — load allowlist enforcement", () => {
  it("load('nonexistent') raises MacsymaUserError naming the allowed packages", () => {
    const { vm } = freshSession();
    expect(() => loadPackage(vm, "nonexistent")).toThrow(MacsymaUserError);
    try {
      loadPackage(vm, "nonexistent");
    } catch (err) {
      const message = (err as Error).message;
      expect(message).toContain("unknown package");
      expect(message).toContain("'nonexistent'");
      expect(message).toContain("orthopoly");
    }
  });

  it("load(42) raises MacsymaUserError for non-string-non-symbol arg", () => {
    const { vm } = freshSession();
    const expr = app(LOAD, [int(42)]);
    expect(() => vm.eval(expr)).toThrowError(/string or symbol/);
  });

  it("load() with wrong arity raises MacsymaUserError", () => {
    const { vm } = freshSession();
    expect(() => vm.eval(app(LOAD, []))).toThrow(MacsymaUserError);
  });

  it("path-traversal-shaped strings are rejected as unknown names", () => {
    // The allowlist match is by string equality, so there is no path
    // resolution — these strings simply aren't on the list.  This test
    // nails down the absence of an importlib/require/eval code path.
    const hostile = ["../etc/passwd", "/tmp/orthopoly", "orthopoly.js", "os"];
    for (const name of hostile) {
      const { vm } = freshSession();
      expect(() => loadPackage(vm, name)).toThrow(MacsymaUserError);
    }
  });
});

// ---------------------------------------------------------------------
// 5. Idempotence — re-loading is a no-op.
// ---------------------------------------------------------------------

describe("Track M2 — load idempotence", () => {
  it("load('orthopoly') twice is safe and keeps the evaluator working", () => {
    const { vm, backend } = freshSession();
    loadPackage(vm, "orthopoly");
    const second = loadPackage(vm, "orthopoly");
    expect(second).toEqual(stringNode("orthopoly"));
    expect(backend.loadedPackages.has("orthopoly")).toBe(true);
    // P_2(x) = (3x^2 − 1)/2 → at x = 3: (27 − 1)/2 = 13.  Pass the
    // concrete value directly into the recurrence instead of routing
    // through `Subst` (see notes in the reductions-after-load block).
    const p2At3 = vm.eval(app(sym("LegendreP"), [int(2), int(3)]));
    expect(p2At3).toEqual(int(13));
  });
});

// ---------------------------------------------------------------------
// 6. Per-session state — two backends are independent.
// ---------------------------------------------------------------------

describe("Track M2 — per-session state", () => {
  it("two backends have independent loadedPackages sets", () => {
    const a = freshSession();
    const b = freshSession();
    loadPackage(a.vm, "orthopoly");
    expect(a.backend.loadedPackages.has("orthopoly")).toBe(true);
    expect(b.backend.loadedPackages.has("orthopoly")).toBe(false);

    // vm_a reduces, vm_b doesn't.
    expect(a.vm.eval(legendreP(0, "x"))).toEqual(int(1));
    const bResult = b.vm.eval(legendreP(0, "x"));
    expect(bResult.kind).toBe("apply");
    if (bResult.kind !== "apply") throw new Error("unreachable");
    expect(bResult.head.kind === "symbol" && bResult.head.name).toBe(
      "LegendreP",
    );
  });
});

// ---------------------------------------------------------------------
// 7. Regression — non-orthopoly ops still work without a load.
// ---------------------------------------------------------------------

describe("Track M2 — non-orthopoly ops unaffected", () => {
  it("basic arithmetic still folds without any load", () => {
    // The regression we care about: installing the M2 orthopoly
    // gates and the LOAD handler must not perturb the substrate
    // simplifier.  Run a trivial 1+2 through the VM and confirm we
    // get the canonical integer back — if anything down-stream of
    // the Add handler regressed, this test would fail loudly.
    const { vm } = freshSession();
    const result = vm.eval(app(sym("Add"), [int(1), int(2)]));
    expect(result).toEqual(int(3));
  });

  it("Simplify still folds (1+2) without any load", () => {
    // Regression check for the Simplify CAS handler — the runtime
    // mounts it independently of the M2 changes, so this should
    // continue to reduce to a canonical integer.
    const { vm } = freshSession();
    const result = vm.eval(app(sym("Simplify"), [
      app(sym("Add"), [int(1), int(2)]),
    ]));
    expect(result).toEqual(int(3));
  });
});

// ---------------------------------------------------------------------
// 8. Surface-name routing — `load` is wired through the name table.
// ---------------------------------------------------------------------

describe("Track M2 — surface-name routing", () => {
  it("MACSYMA_NAME_TABLE maps load and orthopoly surface names", () => {
    expect(MACSYMA_NAME_TABLE.get("load")?.name).toBe("Load");
    expect(MACSYMA_NAME_TABLE.get("legendre_p")?.name).toBe("LegendreP");
    expect(MACSYMA_NAME_TABLE.get("legendre_q")?.name).toBe("LegendreQ");
    expect(MACSYMA_NAME_TABLE.get("chebyshev_t")?.name).toBe("ChebyshevT");
    expect(MACSYMA_NAME_TABLE.get("chebyshev_u")?.name).toBe("ChebyshevU");
    expect(MACSYMA_NAME_TABLE.get("hermite")?.name).toBe("HermiteH");
    expect(MACSYMA_NAME_TABLE.get("bessel_j")?.name).toBe("BesselJ");
    expect(MACSYMA_NAME_TABLE.get("bessel_y")?.name).toBe("BesselY");
  });
});

// ---------------------------------------------------------------------
// 9. Symbol-form loading — `load(orthopoly)` also works.
// ---------------------------------------------------------------------

describe("Track M2 — symbol-form loading", () => {
  it("load(orthopoly) (bare symbol) is accepted, same as the string form", () => {
    const { vm, backend } = freshSession();
    const result = vm.eval(app(LOAD, [sym("orthopoly")]));
    expect(result).toEqual(stringNode("orthopoly"));
    expect(backend.loadedPackages.has("orthopoly")).toBe(true);
  });
});

// ---------------------------------------------------------------------
// 10. Non-integer first argument keeps the polynomial heads unevaluated.
// ---------------------------------------------------------------------

describe("Track M2 — non-integer / negative degree", () => {
  it("legendre_p(n, x) with a free symbol n is unevaluated even after load", () => {
    const { vm } = freshSession();
    loadPackage(vm, "orthopoly");
    const result = vm.eval(app(sym("LegendreP"), [sym("n"), sym("x")]));
    expect(result.kind).toBe("apply");
    if (result.kind !== "apply") throw new Error("unreachable");
    expect(result.head.kind === "symbol" && result.head.name).toBe("LegendreP");
  });

  it("legendre_p(-1, x) is unevaluated even after load", () => {
    const { vm } = freshSession();
    loadPackage(vm, "orthopoly");
    const result = vm.eval(app(sym("LegendreP"), [int(-1), sym("x")]));
    expect(result.kind).toBe("apply");
    if (result.kind !== "apply") throw new Error("unreachable");
    expect(result.head.kind === "symbol" && result.head.name).toBe("LegendreP");
  });
});
