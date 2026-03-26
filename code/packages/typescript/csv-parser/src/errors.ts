/**
 * errors.ts — error classes for the CSV parser.
 *
 * We use a custom error class (rather than throwing a plain Error) so that
 * callers can use `instanceof UnclosedQuoteError` to distinguish CSV parse
 * errors from other unexpected errors.
 */

/**
 * Thrown when the CSV input ends while still inside a quoted field.
 *
 * An opening `"` that is never matched by a closing `"` before EOF is
 * unambiguously malformed. There is no reasonable way to recover from this,
 * so we throw rather than silently producing garbage output.
 *
 * Example of input that triggers this error:
 * ```
 * name,value
 * 1,"unclosed
 * ```
 * The second field of row 1 starts with `"` but the input ends before the
 * closing `"` is found.
 */
export class UnclosedQuoteError extends Error {
  constructor() {
    super("Unclosed quoted field: EOF reached inside a quoted field");
    // Setting `name` explicitly ensures that `err.name` shows "UnclosedQuoteError"
    // rather than the generic "Error" in stack traces and log output.
    this.name = "UnclosedQuoteError";

    // Restore the prototype chain. This is necessary in TypeScript when extending
    // built-in classes like Error, because the TypeScript compiler's `class`
    // desugaring can break `instanceof` checks in some environments.
    Object.setPrototypeOf(this, UnclosedQuoteError.prototype);
  }
}
