import { blank, named, rule } from "@coding-adventures/cas-pattern-matching";
import {
  ADD,
  COS,
  DIV,
  EXP,
  LOG,
  MUL,
  POW,
  SIN,
  SUB,
  app,
  int,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

/**
 * Build and return the algebraic identity rules consumed by the pattern rewriter.
 *
 * Pattern variables use `named("x", blank())`, not `sym("x")`, so RHS
 * substitution can replace captures with the matched expression.
 */
export function buildIdentityRules(): IRNode[] {
  const x = () => named("x", blank());
  const zero = int(0);
  const one = int(1);

  return [
    rule(app(ADD, [x(), zero]), x()),
    rule(app(ADD, [zero, x()]), x()),

    rule(app(MUL, [x(), one]), x()),
    rule(app(MUL, [one, x()]), x()),
    rule(app(MUL, [x(), zero]), zero),
    rule(app(MUL, [zero, x()]), zero),

    rule(app(POW, [x(), zero]), one),
    rule(app(POW, [x(), one]), x()),
    rule(app(POW, [one, x()]), one),

    rule(app(SUB, [x(), x()]), zero),
    rule(app(DIV, [x(), x()]), one),

    rule(app(LOG, [app(EXP, [x()])]), x()),
    rule(app(EXP, [app(LOG, [x()])]), x()),

    rule(app(SIN, [zero]), zero),
    rule(app(COS, [zero]), one),
  ];
}

export const IDENTITY_RULES: readonly IRNode[] = Object.freeze(buildIdentityRules());
