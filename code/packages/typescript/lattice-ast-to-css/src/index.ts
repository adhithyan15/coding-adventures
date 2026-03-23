/**
 * @coding-adventures/lattice-ast-to-css
 *
 * Three-pass compiler: Lattice AST → Clean CSS AST → CSS text.
 *
 * This package is the core of the Lattice-to-CSS compilation pipeline.
 * It takes a Lattice AST (from lattice-parser) and:
 *
 * 1. Collects all variable/mixin/function definitions (Pass 1)
 * 2. Expands all Lattice constructs into CSS (Pass 2)
 * 3. Cleans up empty nodes (Pass 3)
 * 4. Emits the resulting CSS as a string (CSSEmitter)
 *
 * Architecture
 * ------------
 *
 * The package is organized into several modules:
 *
 *   errors.ts      — 10 error classes (LatticeError and subclasses)
 *   scope.ts       — ScopeChain for lexical variable scoping
 *   values.ts      — LatticeValue discriminated union and arithmetic
 *   evaluator.ts   — ExpressionEvaluator (compile-time expression evaluation)
 *   transformer.ts — LatticeTransformer (the 3-pass pipeline)
 *   emitter.ts     — CSSEmitter (AST → CSS text)
 *
 * Usage
 * -----
 *
 *     import { LatticeTransformer, CSSEmitter } from "@coding-adventures/lattice-ast-to-css";
 *     import { parseLattice } from "@coding-adventures/lattice-parser";
 *
 *     const ast = parseLattice("$color: red; h1 { color: $color; }");
 *     const transformer = new LatticeTransformer();
 *     const cssAst = transformer.transform(ast);
 *     const emitter = new CSSEmitter();
 *     const css = emitter.emit(cssAst);
 *     // "h1 {\n  color: red;\n}\n"
 */

// Errors
export {
  LatticeError,
  LatticeModuleNotFoundError,
  ReturnOutsideFunctionError,
  UndefinedVariableError,
  UndefinedMixinError,
  UndefinedFunctionError,
  WrongArityError,
  CircularReferenceError,
  TypeErrorInExpression,
  UnitMismatchError,
  MissingReturnError,
} from "./errors.js";

// Scope
export { ScopeChain } from "./scope.js";

// Values
export {
  LatticeNumber,
  LatticeDimension,
  LatticePercentage,
  LatticeString,
  LatticeIdent,
  LatticeColor,
  LatticeBool,
  LatticeNull,
  LatticeList,
  isTruthy,
  tokenToValue,
  valueToCss,
  extractValueFromAst,
  addValues,
  subtractValues,
  multiplyValues,
  negateValue,
  compareValues,
} from "./values.js";
export type { LatticeValue } from "./values.js";

// Evaluator
export { ExpressionEvaluator } from "./evaluator.js";

// Transformer
export { LatticeTransformer } from "./transformer.js";

// Emitter
export { CSSEmitter } from "./emitter.js";
