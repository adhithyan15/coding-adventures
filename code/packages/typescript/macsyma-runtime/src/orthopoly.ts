/**
 * `orthopoly` — loadable orthogonal-polynomial evaluator pack (Track M2).
 *
 * A MACSYMA session gains numeric/symbolic closed-form expansion of the
 * classical orthogonal polynomials by calling::
 *
 *     load("orthopoly");
 *
 * Before that call, `legendre_p(3, x)` parses to `LegendreP(3, x)` and
 * round-trips unevaluated — the IR symbol exists (it's named in the spec
 * of the Legendre ODE) but the gated handler short-circuits.  After
 * `load`, the same expression collapses to its closed-form polynomial::
 *
 *     legendre_p(3, x);   →   (5*x^3 - 3*x) / 2
 *
 * What's covered
 * ==============
 *
 * Seven heads, all the classical orthogonal polynomial families that
 * `symbolic-ir` declares as named ODE solutions:
 *
 *  - `LegendreP(n, x)` — Bonnet recursion
 *    `(n+1) P_{n+1} = (2n+1) x P_n − n P_{n−1}`, `P_0 = 1`, `P_1 = x`.
 *  - `ChebyshevT(n, x)` — `T_{n+1} = 2x T_n − T_{n−1}`,
 *    `T_0 = 1`, `T_1 = x`.
 *  - `ChebyshevU(n, x)` — `U_{n+1} = 2x U_n − U_{n−1}`,
 *    `U_0 = 1`, `U_1 = 2x`.
 *  - `HermiteH(n, x)` — physicists' Hermite,
 *    `H_{n+1} = 2x H_n − 2n H_{n−1}`, `H_0 = 1`, `H_1 = 2x`.
 *  - `LegendreQ(n, x)`, `BesselJ(n, x)`, `BesselY(n, x)` — held as
 *    passthrough.  After `load("orthopoly")` the runtime "knows" these
 *    symbols but has no polynomial reduction; the expression survives
 *    for downstream rewrites (Taylor, integrate, …).
 *
 * Non-integer or negative `n` is left unevaluated even after `load`,
 * matching the Python implementation and Maxima's `orthopoly` package
 * contract.
 *
 * Design note — gated handlers
 * ============================
 *
 * The Python implementation literally mutates the backend's handler
 * table inside `register_handlers(backend)`.  The TS/Rust ports use an
 * equivalent but slightly different mechanism: the handlers are
 * registered *unconditionally* at backend construction, and each one
 * consults `backend.loadedPackages.has("orthopoly")` before doing any
 * work.  When the flag is unset they return the expression unchanged,
 * which is observationally identical to "no handler registered" for
 * the symbolic VM.  This avoids the need to mutate a `ReadonlyMap`
 * after construction and keeps the backend's public handler view
 * honest.  The user-facing contract is unchanged.
 */

import {
  DIV,
  MUL,
  SUB,
  app,
  int,
  type IRApply,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import type { VM } from "@coding-adventures/symbolic-vm";

import type { MacsymaBackend } from "./index.js";

/**
 * Package name the orthopoly evaluators consult.  Kept as a `const`
 * (not a parameter) so an audit reading the call site can trivially
 * verify the gate.
 */
const ORTHOPOLY_NAME = "orthopoly" as const;

/**
 * Bonnet recursion for `LegendreP(n, x)`.
 *
 * Stable for arbitrary `n ≥ 0` because every step is one multiply,
 * one subtract, one rational divide.  Each intermediate is run through
 * `vm.eval` so the polynomial stays in canonical form rather than
 * ballooning into deeply-nested unsimplified IR.
 */
export function legendreP(n: number, x: IRNode, vm: VM): IRNode {
  if (n === 0) return int(1);
  if (n === 1) return x;
  let pPrev: IRNode = int(1);
  let pCurr: IRNode = x;
  for (let k = 1; k < n; k += 1) {
    // (k+1) P_{k+1} = (2k+1) x P_k − k P_{k−1}
    const twoKPlusOne = int(2 * k + 1);
    const kNode = int(k);
    const kPlusOne = int(k + 1);
    const next = app(DIV, [
      app(SUB, [
        app(MUL, [twoKPlusOne, app(MUL, [x, pCurr])]),
        app(MUL, [kNode, pPrev]),
      ]),
      kPlusOne,
    ]);
    pPrev = pCurr;
    pCurr = vm.eval(next);
  }
  return pCurr;
}

/**
 * Chebyshev T recursion `T_{n+1} = 2x T_n − T_{n−1}`,
 * seeded with `T_0 = 1`, `T_1 = x`.
 */
export function chebyshevT(n: number, x: IRNode, vm: VM): IRNode {
  if (n === 0) return int(1);
  if (n === 1) return x;
  let tPrev: IRNode = int(1);
  let tCurr: IRNode = x;
  const twoX = app(MUL, [int(2), x]);
  for (let k = 1; k < n; k += 1) {
    const next = app(SUB, [app(MUL, [twoX, tCurr]), tPrev]);
    tPrev = tCurr;
    tCurr = vm.eval(next);
  }
  return tCurr;
}

/**
 * Chebyshev U recursion `U_{n+1} = 2x U_n − U_{n−1}`,
 * seeded with `U_0 = 1`, `U_1 = 2x` — note the different seed
 * from T.
 */
export function chebyshevU(n: number, x: IRNode, vm: VM): IRNode {
  if (n === 0) return int(1);
  const twoX = vm.eval(app(MUL, [int(2), x]));
  if (n === 1) return twoX;
  let uPrev: IRNode = int(1);
  let uCurr: IRNode = twoX;
  const twoXFactor = app(MUL, [int(2), x]);
  for (let k = 1; k < n; k += 1) {
    const next = app(SUB, [app(MUL, [twoXFactor, uCurr]), uPrev]);
    uPrev = uCurr;
    uCurr = vm.eval(next);
  }
  return uCurr;
}

/**
 * Physicists' Hermite recursion `H_{n+1} = 2x H_n − 2n H_{n−1}`,
 * seeded with `H_0 = 1`, `H_1 = 2x`.  This matches the convention used
 * by the `HermiteH` IR head's docstring and by Maxima's `hermite`.
 */
export function hermiteH(n: number, x: IRNode, vm: VM): IRNode {
  if (n === 0) return int(1);
  const twoX = vm.eval(app(MUL, [int(2), x]));
  if (n === 1) return twoX;
  let hPrev: IRNode = int(1);
  let hCurr: IRNode = twoX;
  const twoXFactor = app(MUL, [int(2), x]);
  for (let k = 1; k < n; k += 1) {
    const twoK = int(2 * k);
    const next = app(SUB, [
      app(MUL, [twoXFactor, hCurr]),
      app(MUL, [twoK, hPrev]),
    ]);
    hPrev = hCurr;
    hCurr = vm.eval(next);
  }
  return hCurr;
}

/**
 * Pull `(n, x)` out of an orthopoly-style two-arg call.
 *
 * Returns `undefined` when the expression shape isn't right (wrong
 * arity, non-integer `n`, negative `n`, oversized `n`).  The handlers
 * treat `undefined` as "leave the expression alone", matching the
 * Python implementation's "no surprise" rule.
 */
function checkNX(expr: IRApply): { n: number; x: IRNode } | undefined {
  if (expr.args.length !== 2) return undefined;
  const [nNode, xNode] = expr.args;
  if (nNode.kind !== "integer") return undefined;
  if (nNode.value < 0n) return undefined;
  // The recurrence runs O(n) times in a single JS call stack.  Capping
  // at JS's safe integer ceiling is a paranoia bound; in practice
  // anything beyond ~10^4 is already untenable from a memory-of-IR
  // standpoint.
  if (nNode.value > BigInt(Number.MAX_SAFE_INTEGER)) return undefined;
  return { n: Number(nNode.value), x: xNode };
}

/**
 * Build a handler that runs `recurrence(n, x, vm)` after the gate fires.
 *
 * Always-installed; runs the reduction only when `loadedPackages`
 * contains `"orthopoly"`.  Until then, returns the expression
 * unevaluated, so callers see exactly the same behaviour they would if
 * no handler were registered at all.
 */
export function makeOrthopolyRecurrenceHandler(
  backend: MacsymaBackend,
  recurrence: (n: number, x: IRNode, vm: VM) => IRNode,
) {
  return (vm: VM, expr: IRApply): IRNode => {
    if (!backend.loadedPackages.has(ORTHOPOLY_NAME)) {
      return expr;
    }
    const parsed = checkNX(expr);
    if (parsed === undefined) return expr;
    return vm.eval(recurrence(parsed.n, parsed.x, vm));
  };
}

/**
 * Build a handler that returns the expression unevaluated, but only
 * once the orthopoly gate has been opened.  Models the
 * `LegendreQ` / `BesselJ` / `BesselY` heads: after `load("orthopoly")`
 * the symbol is "known" but has no closed-form reduction.  Returning
 * the expression keeps it available for downstream rewrites and is
 * observationally identical to the no-handler path before `load`.
 */
export function makeOrthopolyPassthroughHandler(backend: MacsymaBackend) {
  return (_vm: VM, expr: IRApply): IRNode => {
    if (!backend.loadedPackages.has(ORTHOPOLY_NAME)) {
      return expr;
    }
    return expr;
  };
}
