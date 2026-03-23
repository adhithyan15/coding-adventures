/**
 * Scope Chain — Lexical scoping for Lattice variables, mixins, and functions.
 *
 * Why Lexical Scoping?
 * --------------------
 *
 * CSS has no concept of scope — everything is global. But Lattice adds
 * variables, mixins, and functions, which need scoping rules to prevent
 * name collisions and enable local reasoning.
 *
 * Lattice uses **lexical (static) scoping**, meaning a variable's scope is
 * determined by where it appears in the source text, not by runtime call order.
 * This is the same model used by JavaScript, Python, and most modern languages.
 *
 * How It Works
 * ------------
 *
 * Each { } block in the source creates a new child scope. Variables declared
 * inside a block are local to that scope and its descendants. Looking up a
 * variable walks up the parent chain until the name is found:
 *
 *     $color: red;              ← global scope (depth 0)
 *     .parent {                 ← child scope (depth 1)
 *         $color: blue;         ← shadows the global $color
 *         color: $color;        → blue (found at depth 1)
 *         .child {              ← grandchild scope (depth 2)
 *             color: $color;    → blue (inherited from depth 1)
 *         }
 *     }
 *     .sibling {                ← another child scope (depth 1)
 *         color: $color;        → red (global scope, not affected by .parent)
 *     }
 *
 * This is implemented as a linked list of scope nodes. Each node has a parent
 * reference and a bindings Map. Looking up a name walks the chain upward.
 *
 * Special Scoping Rules
 * ----------------------
 *
 * - Mixin expansion: creates a child scope of the caller's scope. This lets
 *   mixins see the caller's variables (like closures in JavaScript).
 *
 * - Function evaluation: creates an isolated scope whose parent is the
 *   definition-site global scope, NOT the caller's scope. This prevents
 *   functions from accidentally depending on where they're called from —
 *   they only see their own parameters and global definitions.
 */

// =============================================================================
// ScopeChain
// =============================================================================

/**
 * A lexical scope node in the scope chain.
 *
 * Each scope has:
 *   - bindings: A Map from names to values (ASTNodes, tokens, or
 *     LatticeValues — the scope doesn't care about the type).
 *   - parent: The enclosing scope, or null for the global scope.
 *
 * Operations:
 *   - get(name)  — look up a name, walking up the parent chain.
 *   - set(name, value) — bind a name in THIS scope (not parent's).
 *   - has(name)  — check if a name exists anywhere in the chain.
 *   - hasLocal(name) — check if a name exists in THIS scope only.
 *   - child()    — create a new child scope with self as parent.
 *   - depth      — how many levels deep this scope is (0 = global).
 *
 * Why a class and not just a Map?
 * Because nested scopes need parent-chain lookups, and we need to
 * distinguish between "not found anywhere" (error) and "not found
 * locally but found in parent" (normal lexical lookup).
 *
 * Example:
 *
 *     const global = new ScopeChain();
 *     global.set("$color", "red");
 *
 *     const block = global.child();
 *     block.set("$color", "blue");
 *
 *     block.get("$color");    // → "blue" (local)
 *     global.get("$color");   // → "red"  (unchanged)
 *
 *     const nested = block.child();
 *     nested.get("$color");   // → "blue" (inherited from parent)
 */
export class ScopeChain {
  /** Local bindings for this scope level. */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private readonly bindings: Map<string, any> = new Map();

  /** The enclosing scope, or null for the global scope. */
  readonly parent: ScopeChain | null;

  constructor(parent: ScopeChain | null = null) {
    this.parent = parent;
  }

  /**
   * Look up a name in this scope or any ancestor scope.
   *
   * Walks up the parent chain until the name is found. If the name
   * isn't found anywhere, returns undefined.
   *
   * This is the core of lexical scoping — a variable declared in an
   * outer scope is visible in all inner scopes unless shadowed.
   *
   * @param name - The variable/mixin/function name to look up.
   * @returns The bound value, or undefined if not found anywhere.
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  get(name: string): any {
    if (this.bindings.has(name)) {
      return this.bindings.get(name);
    }
    if (this.parent !== null) {
      return this.parent.get(name);
    }
    return undefined;
  }

  /**
   * Bind a name to a value in this scope.
   *
   * Always binds in the CURRENT scope, never in a parent scope.
   * This means a child scope can shadow a parent's binding without
   * modifying the parent.
   *
   * @param name - The variable/mixin/function name to bind.
   * @param value - The value to associate with the name.
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  set(name: string, value: any): void {
    this.bindings.set(name, value);
  }

  /**
   * Check whether a name exists in this scope or any ancestor.
   *
   * Like get(), walks up the parent chain. Returns true if the
   * name is bound anywhere, false otherwise.
   *
   * @param name - The name to check.
   * @returns true if the name is bound, false otherwise.
   */
  has(name: string): boolean {
    if (this.bindings.has(name)) return true;
    if (this.parent !== null) return this.parent.has(name);
    return false;
  }

  /**
   * Check whether a name exists in THIS scope only (not parents).
   *
   * Useful for detecting re-declarations and shadowing.
   *
   * @param name - The name to check.
   * @returns true if the name is bound locally, false otherwise.
   */
  hasLocal(name: string): boolean {
    return this.bindings.has(name);
  }

  /**
   * Create a new child scope with self as parent.
   *
   * The child inherits all bindings from the parent chain via get(),
   * but any set() calls on the child only affect the child.
   *
   * @returns A new ScopeChain whose parent is this scope.
   */
  child(): ScopeChain {
    return new ScopeChain(this);
  }

  /**
   * How many levels deep this scope is.
   *
   * The global scope has depth 0. Each child() call adds 1.
   * This is useful for debugging — deeper scopes are more nested.
   *
   * @returns The depth of this scope in the chain.
   */
  get depth(): number {
    if (this.parent === null) return 0;
    return 1 + this.parent.depth;
  }

  toString(): string {
    const names = Array.from(this.bindings.keys());
    return `ScopeChain(depth=${this.depth}, bindings=[${names.join(", ")}])`;
  }
}
