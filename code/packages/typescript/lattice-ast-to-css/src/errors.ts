/**
 * Lattice Error Types — Structured errors for the AST-to-CSS compiler.
 *
 * Every error in the Lattice compiler carries:
 *   - A human-readable message explaining what went wrong.
 *   - The line and column where the error occurred (from the originating token).
 *
 * The error hierarchy mirrors the compiler's three passes:
 *
 * Pass 1 (Module Resolution):
 *   LatticeModuleNotFoundError — @use references a file that doesn't exist.
 *
 * Pass 2 (Symbol Collection):
 *   ReturnOutsideFunctionError — @return appears outside a @function body.
 *
 * Pass 3 (Expansion):
 *   UndefinedVariableError   — $var referenced but never declared.
 *   UndefinedMixinError      — @include references an unknown mixin.
 *   UndefinedFunctionError   — function call references an unknown function.
 *   WrongArityError          — mixin/function called with wrong arg count.
 *   CircularReferenceError   — mixin or function calls itself (directly or
 *                              indirectly), forming a cycle.
 *   TypeErrorInExpression    — arithmetic on incompatible types (e.g., 10px + red).
 *   UnitMismatchError        — arithmetic on incompatible units (e.g., 10px + 5s).
 *   MissingReturnError       — function body has no @return statement.
 *
 * All errors inherit from LatticeError so callers can catch the whole family
 * with a single catch (LatticeError) clause.
 *
 * Example:
 *
 *     try {
 *       const css = transpileLattice(source);
 *     } catch (e) {
 *       if (e instanceof LatticeError) {
 *         console.error(`Error at line ${e.line}, col ${e.column}: ${e.message}`);
 *       }
 *     }
 */

// =============================================================================
// Base Error Class
// =============================================================================

/**
 * Base class for all Lattice compiler errors.
 *
 * Every subclass stores the line and column where the error occurred.
 * These come from the token that triggered the error — the Lattice lexer
 * embeds position info in every token.
 *
 * Why inherit from Error?
 * TypeScript's Error class properly captures stack traces and works with
 * instanceof checks. We extend it rather than creating a custom base class
 * to get all those benefits.
 */
export class LatticeError extends Error {
  /** Human-readable error description (without position info). */
  readonly latticeMessage: string;

  /** 1-based line number in the source file (0 if unknown). */
  readonly line: number;

  /** 1-based column number in the source file (0 if unknown). */
  readonly column: number;

  constructor(message: string, line: number = 0, column: number = 0) {
    // Append position info to the Error message string for human display.
    const location = line ? ` at line ${line}, column ${column}` : "";
    super(`${message}${location}`);

    this.latticeMessage = message;
    this.line = line;
    this.column = column;

    // Restore the prototype chain after calling super().
    // This is required in TypeScript when extending built-in classes.
    Object.setPrototypeOf(this, new.target.prototype);
    this.name = new.target.name;
  }
}

// =============================================================================
// Pass 1: Module Resolution Errors
// =============================================================================

/**
 * Raised when @use references a module that cannot be found.
 *
 * Example: @use "nonexistent";
 *
 * Note: In this implementation, @use is collected and silently skipped
 * (module resolution is not fully implemented). This error is defined for
 * completeness and future use.
 */
export class LatticeModuleNotFoundError extends LatticeError {
  readonly moduleName: string;

  constructor(moduleName: string, line: number = 0, column: number = 0) {
    super(`Module '${moduleName}' not found`, line, column);
    this.moduleName = moduleName;
  }
}

// =============================================================================
// Pass 2: Symbol Collection Errors
// =============================================================================

/**
 * Raised when @return appears outside a @function body.
 *
 * Example: @return 42;  (at top level or inside a mixin)
 *
 * @return is only valid inside a @function body. Using it at the top level
 * or inside a mixin body is a syntax/semantic error.
 */
export class ReturnOutsideFunctionError extends LatticeError {
  constructor(line: number = 0, column: number = 0) {
    super("@return outside @function", line, column);
  }
}

// =============================================================================
// Pass 3: Expansion Errors
// =============================================================================

/**
 * Raised when a $variable is referenced but never declared.
 *
 * Example: color: $nonexistent;
 *
 * This means the variable was never declared in any enclosing scope.
 * It's different from a variable that exists but is null.
 */
export class UndefinedVariableError extends LatticeError {
  readonly name: string;

  constructor(name: string, line: number = 0, column: number = 0) {
    super(`Undefined variable '${name}'`, line, column);
    this.name = name;
  }
}

/**
 * Raised when @include references a mixin that was never defined.
 *
 * Example: @include nonexistent;
 *
 * Mixins must be defined before or after use (Pass 1 collects all definitions
 * before expansion, so forward references are allowed). But if a mixin name
 * doesn't exist at all, this error is raised.
 */
export class UndefinedMixinError extends LatticeError {
  readonly name: string;

  constructor(name: string, line: number = 0, column: number = 0) {
    super(`Undefined mixin '${name}'`, line, column);
    this.name = name;
  }
}

/**
 * Raised when a function call references a function that was never defined.
 *
 * Note: this only applies to Lattice functions (@function), not CSS functions
 * like rgb(), calc(), var(), etc. CSS functions are passed through unchanged.
 *
 * Example: padding: spacing(2);  (if spacing was never defined)
 */
export class UndefinedFunctionError extends LatticeError {
  readonly name: string;

  constructor(name: string, line: number = 0, column: number = 0) {
    super(`Undefined function '${name}'`, line, column);
    this.name = name;
  }
}

/**
 * Raised when a mixin or function is called with the wrong number of args.
 *
 * The expected count accounts for parameters that have defaults — only
 * parameters without defaults are required.
 *
 * Example: @mixin button($bg, $fg) called as @include button(red, blue, green);
 */
export class WrongArityError extends LatticeError {
  readonly name: string;
  readonly expected: number;
  readonly got: number;

  constructor(
    kind: string,
    name: string,
    expected: number,
    got: number,
    line: number = 0,
    column: number = 0
  ) {
    super(`${kind} '${name}' expects ${expected} args, got ${got}`, line, column);
    this.name = name;
    this.expected = expected;
    this.got = got;
  }
}

/**
 * Raised when a mixin or function calls itself, forming a cycle.
 *
 * The chain shows the full call path: a → b → a.
 *
 * Example: @mixin a { @include b; }  @mixin b { @include a; }
 *
 * Cycle detection is implemented via a call stack maintained during
 * expansion. If a name appears twice in the stack, a cycle is detected.
 */
export class CircularReferenceError extends LatticeError {
  readonly chain: string[];

  constructor(kind: string, chain: string[], line: number = 0, column: number = 0) {
    const chainStr = chain.join(" → ");
    super(`Circular ${kind}: ${chainStr}`, line, column);
    this.chain = chain;
  }
}

/**
 * Raised when arithmetic is attempted on incompatible types.
 *
 * Example: 10px + red  (can't add a dimension and a color/ident)
 *
 * The op field describes the attempted operation: "add", "subtract",
 * "multiply", "negate".
 */
export class TypeErrorInExpression extends LatticeError {
  readonly op: string;
  readonly leftType: string;
  readonly rightType: string;

  constructor(
    op: string,
    left: string,
    right: string,
    line: number = 0,
    column: number = 0
  ) {
    super(`Cannot ${op} '${left}' and '${right}'`, line, column);
    this.op = op;
    this.leftType = left;
    this.rightType = right;
  }
}

/**
 * Raised when arithmetic combines dimensions with incompatible units.
 *
 * Compatible units can be added/subtracted directly: 10px + 5px → 15px.
 * Incompatible units that belong to the same CSS category could use calc()
 * in output, but some combinations are never valid: 10px + 5s (length + time).
 *
 * Example: 10px + 5s
 */
export class UnitMismatchError extends LatticeError {
  readonly leftUnit: string;
  readonly rightUnit: string;

  constructor(
    leftUnit: string,
    rightUnit: string,
    line: number = 0,
    column: number = 0
  ) {
    super(`Cannot add '${leftUnit}' and '${rightUnit}' units`, line, column);
    this.leftUnit = leftUnit;
    this.rightUnit = rightUnit;
  }
}

/**
 * Raised when a function body has no @return statement.
 *
 * Every @function must return a value via @return. A function body that
 * contains only variable declarations or control flow with no @return in
 * any reachable branch is an error.
 *
 * Example: @function noop($x) { $y: $x; }
 */
export class MissingReturnError extends LatticeError {
  readonly name: string;

  constructor(name: string, line: number = 0, column: number = 0) {
    super(`Function '${name}' has no @return`, line, column);
    this.name = name;
  }
}

// =============================================================================
// Lattice v2: New Error Types
// =============================================================================
//
// These errors support the new features introduced in Lattice v2:
// - @while loops (MaxIterationError)
// - @extend directive (ExtendTargetNotFoundError)
// - Built-in functions (RangeError, ZeroDivisionInExpressionError)
// =============================================================================

/**
 * Raised when a @while loop exceeds the maximum iteration count.
 *
 * The max-iteration guard prevents infinite loops. Lattice sets a
 * configurable limit (default: 1000 iterations). If a @while loop's
 * condition remains truthy after this many iterations, compilation
 * halts with this error.
 *
 * The most common cause is a missing or incorrect loop variable update:
 *
 *     $i: 1;
 *     @while $i <= 10 {
 *         // Oops -- forgot to increment $i!
 *         .item-#{$i} { display: block; }
 *     }
 *
 * Example: @while true { } (no mutation to break the loop)
 */
export class MaxIterationError extends LatticeError {
  readonly maxIterations: number;

  constructor(maxIterations: number = 1000, line: number = 0, column: number = 0) {
    super(
      `@while loop exceeded maximum iteration count (${maxIterations})`,
      line,
      column
    );
    this.maxIterations = maxIterations;
  }
}

/**
 * Raised when @extend references a selector not found in the stylesheet.
 *
 * @extend works by appending the current rule's selector to another rule's
 * selector list. If the target selector does not exist anywhere in the
 * stylesheet, it is an error -- the programmer likely made a typo or
 * forgot to define the base rule.
 *
 *     .success {
 *         @extend %message-shared;  // Error if %message-shared is never defined
 *     }
 *
 * Example: @extend .nonexistent; where .nonexistent has no matching rule
 */
export class ExtendTargetNotFoundError extends LatticeError {
  readonly target: string;

  constructor(target: string, line: number = 0, column: number = 0) {
    super(
      `@extend target '${target}' was not found in the stylesheet`,
      line,
      column
    );
    this.target = target;
  }
}

/**
 * Raised when a value is outside the valid range for an operation.
 *
 * Used by built-in functions that require bounded inputs:
 *
 * - nth($list, $n) -- index must be >= 1 and <= list length
 * - lighten($color, $amount) -- amount must be between 0% and 100%
 * - mix($c1, $c2, $weight) -- weight must be between 0% and 100%
 *
 * Example: nth((a, b, c), 5) -- index 5 out of bounds for list of length 3
 */
export class LatticeRangeError extends LatticeError {
  constructor(message: string, line: number = 0, column: number = 0) {
    super(message, line, column);
  }
}

/**
 * Raised when math.div() encounters a zero divisor.
 *
 * Division by zero is undefined. Unlike CSS calc() which defers
 * evaluation to the browser, Lattice evaluates math.div() at compile
 * time and must reject zero divisors.
 *
 * Example: math.div(100px, 0)
 */
export class ZeroDivisionInExpressionError extends LatticeError {
  constructor(line: number = 0, column: number = 0) {
    super("Division by zero", line, column);
  }
}
