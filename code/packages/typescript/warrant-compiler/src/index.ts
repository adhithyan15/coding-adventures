export {
  // Parser and Lexer
  tokenizeWarrant,
  parseWarrant,
  WarrantParser,
  type TokenType,
  type Token,

  // Errors
  WarrantParserError,
  type ParserErrorKind,

  // Evaluator & Runner
  evaluateWarrantExpr,
  areWarrantValuesEqual,
  executeWarrantModule,
  type EvaluatorContext,
  type StepBudget,
  type WarrantTestSuiteOptions,
  type WarrantTestResult,

  // AST Definitions
  type WarrantType,
  type WarrantParam,
  type WarrantPattern,
  type WarrantMatchArm,
  type WarrantExpr,
  type WarrantModel,
  type WarrantOp,
  type WarrantExample,
  type WarrantClaim,
  type WarrantInputContract,
  type WarrantModule,

  // Verdict & Testing Utilities
  type Verdict,
  pass,
  fail,
  Gen,
  assertProperty,
} from "./warrant.js";
