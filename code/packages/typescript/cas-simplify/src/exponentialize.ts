import {
  ADD,
  COS,
  COSH,
  DIV,
  EXP,
  IRNode,
  MUL,
  NEG,
  SIN,
  SINH,
  SUB,
  TAN,
  TANH,
  app,
  equals,
  headName,
  int,
  sym,
} from "@coding-adventures/symbolic-ir";

export const IMAGINARY_UNIT = sym("ImaginaryUnit");

const TWO = int(2);

export function exponentialize(expr: IRNode): IRNode {
  if (expr.kind !== "apply") return expr;
  const node = app(expr.head, expr.args.map(exponentialize));
  return exponentializeNode(node);
}

export function demoivre(expr: IRNode): IRNode {
  if (expr.kind !== "apply") return expr;
  const node = app(expr.head, expr.args.map(demoivre));
  return demoivreNode(node);
}

function exponentializeNode(expr: Extract<IRNode, { kind: "apply" }>): IRNode {
  if (expr.args.length !== 1) return expr;
  const name = headName(expr.head);
  const x = expr.args[0];
  if (name === SIN.name) return sinExp(x);
  if (name === COS.name) return cosExp(x);
  if (name === TAN.name) return tanExp(x);
  if (name === SINH.name) return sinhExp(x);
  if (name === COSH.name) return coshExp(x);
  if (name === TANH.name) return tanhExp(x);
  return expr;
}

function ix(x: IRNode): IRNode {
  return app(MUL, [IMAGINARY_UNIT, x]);
}

function negIx(x: IRNode): IRNode {
  return app(MUL, [IMAGINARY_UNIT, app(NEG, [x])]);
}

function sinExp(x: IRNode): IRNode {
  const ePos = app(EXP, [ix(x)]);
  const eNeg = app(EXP, [negIx(x)]);
  return app(DIV, [app(SUB, [ePos, eNeg]), app(MUL, [TWO, IMAGINARY_UNIT])]);
}

function cosExp(x: IRNode): IRNode {
  const ePos = app(EXP, [ix(x)]);
  const eNeg = app(EXP, [negIx(x)]);
  return app(DIV, [app(ADD, [ePos, eNeg]), TWO]);
}

function tanExp(x: IRNode): IRNode {
  const ePos = app(EXP, [ix(x)]);
  const eNeg = app(EXP, [negIx(x)]);
  const numerator = app(MUL, [app(NEG, [IMAGINARY_UNIT]), app(SUB, [ePos, eNeg])]);
  return app(DIV, [numerator, app(ADD, [ePos, eNeg])]);
}

function sinhExp(x: IRNode): IRNode {
  const ePos = app(EXP, [x]);
  const eNeg = app(EXP, [app(NEG, [x])]);
  return app(DIV, [app(SUB, [ePos, eNeg]), TWO]);
}

function coshExp(x: IRNode): IRNode {
  const ePos = app(EXP, [x]);
  const eNeg = app(EXP, [app(NEG, [x])]);
  return app(DIV, [app(ADD, [ePos, eNeg]), TWO]);
}

function tanhExp(x: IRNode): IRNode {
  const ePos = app(EXP, [x]);
  const eNeg = app(EXP, [app(NEG, [x])]);
  return app(DIV, [app(SUB, [ePos, eNeg]), app(ADD, [ePos, eNeg])]);
}

function demoivreNode(expr: Extract<IRNode, { kind: "apply" }>): IRNode {
  if (headName(expr.head) !== EXP.name || expr.args.length !== 1) return expr;
  const [real, imag] = splitRealImag(expr.args[0]);
  if (imag === undefined) return expr;

  const trigSum = app(ADD, [app(COS, [imag]), app(MUL, [IMAGINARY_UNIT, app(SIN, [imag])])]);
  if (real === undefined) return trigSum;
  return app(MUL, [app(EXP, [real]), trigSum]);
}

function splitRealImag(arg: IRNode): readonly [IRNode | undefined, IRNode | undefined] {
  if (equals(arg, IMAGINARY_UNIT)) return [undefined, int(1)];

  if (isApplyHead(arg, MUL.name)) {
    const coeff = extractIFromMul(arg);
    if (coeff !== undefined) return [undefined, coeff];
  }

  if (isApplyHead(arg, ADD.name)) {
    const realTerms: IRNode[] = [];
    let imagCoeff: IRNode | undefined;
    for (const term of arg.args) {
      if (equals(term, IMAGINARY_UNIT)) {
        if (imagCoeff !== undefined) return [arg, undefined];
        imagCoeff = int(1);
        continue;
      }
      const iPart = extractIFromTerm(term);
      if (iPart !== undefined) {
        if (imagCoeff !== undefined) return [arg, undefined];
        imagCoeff = iPart;
      } else {
        realTerms.push(term);
      }
    }

    if (imagCoeff === undefined) return [arg, undefined];
    if (realTerms.length === 0) return [undefined, imagCoeff];
    return [realTerms.length === 1 ? realTerms[0] : app(ADD, realTerms), imagCoeff];
  }

  return [arg, undefined];
}

function extractIFromMul(mulNode: Extract<IRNode, { kind: "apply" }>): IRNode | undefined {
  const iPositions = mulNode.args.flatMap((arg, index) => (equals(arg, IMAGINARY_UNIT) ? [index] : []));
  if (iPositions.length !== 1) return undefined;
  const rest = mulNode.args.filter((_, index) => index !== iPositions[0]);
  if (rest.length === 0) return int(1);
  if (rest.length === 1) return rest[0];
  return app(MUL, rest);
}

function extractIFromTerm(term: IRNode): IRNode | undefined {
  if (equals(term, IMAGINARY_UNIT)) return int(1);
  if (isApplyHead(term, MUL.name)) return extractIFromMul(term);
  return undefined;
}

function isApplyHead(node: IRNode, name: string): node is Extract<IRNode, { kind: "apply" }> {
  return node.kind === "apply" && headName(node.head) === name;
}
