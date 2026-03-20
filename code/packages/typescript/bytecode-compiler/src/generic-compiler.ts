/**
 * GenericCompiler — A Pluggable AST-to-Bytecode Compiler Framework.
 *
 * ==========================================================================
 * Chapter 1: Why a Generic Compiler?
 * ==========================================================================
 *
 * In the previous modules, we built a ``BytecodeCompiler`` that knows how to
 * compile a specific AST format (the one produced by our parser, with node
 * kinds like ``"NumberLiteral"``, ``"BinaryOp"``, ``"Assignment"``). That
 * compiler is tightly coupled to one language's syntax.
 *
 * But what if we want to compile *different* languages — Python, Ruby, Lua —
 * all to the same bytecode? Each language has its own AST structure, its own
 * node types, and its own semantic rules. We'd need a different compiler for
 * each one.
 *
 * The **GenericCompiler** solves this with the **plugin pattern**:
 *
 *     1. Define a universal AST shape (ruleName + children).
 *     2. Let each language register *handlers* for its specific rule names.
 *     3. The framework handles the plumbing (instruction emission, constant
 *        pools, scope management, jump patching).
 *
 * This is exactly how real compiler frameworks work:
 *
 * - **LLVM** has a generic IR (Intermediate Representation) that many
 *   language front-ends compile to. Each front-end is a "plugin."
 * - **GraalVM's Truffle** framework lets languages register AST interpreters.
 * - **.NET's Roslyn** has a common compilation pipeline with language-specific
 *   syntax analyzers plugged in.
 *
 * Our GenericCompiler is a simplified version of the same idea. A language
 * author writes a set of ``CompileHandler`` functions — one per AST rule —
 * and registers them. The framework does the rest.
 *
 * ==========================================================================
 * Chapter 2: The AST Contract
 * ==========================================================================
 *
 * For the generic compiler to walk *any* language's AST, we need a common
 * tree shape. We use two node types:
 *
 * **ASTNode** — A non-terminal (interior) node. It has:
 *   - ``ruleName``: identifies what grammar rule produced this node
 *     (e.g., ``"if_statement"``, ``"binary_expression"``, ``"function_def"``).
 *   - ``children``: an ordered list of child nodes (either more ASTNodes
 *     or leaf TokenNodes).
 *
 * **TokenNode** — A terminal (leaf) node. It has:
 *   - ``type``: the token category (e.g., ``"NUMBER"``, ``"IDENTIFIER"``).
 *   - ``value``: the actual text from the source code.
 *
 * This two-level structure is a standard representation called a **Concrete
 * Syntax Tree** (CST) or **Parse Tree**. It maps directly to how grammars
 * work: each grammar rule produces an ASTNode, and each terminal symbol
 * produces a TokenNode.
 *
 * Example: the expression ``1 + 2`` might parse into:
 *
 *     ASTNode("addition", [
 *       TokenNode("NUMBER", "1"),
 *       TokenNode("PLUS", "+"),
 *       TokenNode("NUMBER", "2"),
 *     ])
 *
 * ==========================================================================
 * Chapter 3: The Dispatch Mechanism
 * ==========================================================================
 *
 * When the compiler encounters an ASTNode, it needs to decide what to do.
 * The decision is based on the node's ``ruleName``:
 *
 *     1. Look up ``ruleName`` in the handler registry.
 *     2. If a handler exists, call it. The handler receives both the
 *        compiler (for emitting instructions) and the node (for reading
 *        children).
 *     3. If no handler exists but the node has exactly one child, "pass
 *        through" to that child. This handles wrapper rules like
 *        ``expression -> addition`` where there's nothing to compile.
 *     4. If no handler exists and there are multiple children, raise an
 *        ``UnhandledRuleError`` — the language plugin is incomplete.
 *
 * This pass-through behavior is important. In a real grammar, many rules
 * exist purely for precedence or grouping:
 *
 *     expression     -> comparison
 *     comparison     -> addition
 *     addition       -> multiplication
 *     multiplication -> unary
 *     unary          -> primary
 *
 * When parsing ``42``, *all* of these rules fire, each producing a
 * single-child node wrapping the next level down. The pass-through rule
 * means we don't need handlers for these "wrapper" rules.
 *
 * ==========================================================================
 * Chapter 4: Scope Management
 * ==========================================================================
 *
 * Languages with functions, closures, or block scoping need to track which
 * variables are "local" to each scope. The GenericCompiler provides a scope
 * stack:
 *
 *     enterScope(params?)  — Push a new scope (optionally pre-loaded with
 *                            parameter names).
 *     exitScope()          — Pop the current scope and return it.
 *
 * Each ``CompilerScope`` maintains a ``locals`` map from variable names to
 * slot indices. When compiling a function body, the language plugin calls
 * ``enterScope(["x", "y"])`` to create a scope with parameters pre-assigned
 * to slots 0 and 1, then uses ``scope.addLocal("temp")`` for any additional
 * local variables.
 *
 * Scopes form a linked list via the ``parent`` pointer, enabling lexical
 * scoping lookups (though the GenericCompiler doesn't implement closures —
 * that's left to language plugins).
 */

import type { Instruction, CodeObject } from "./vm-types.js";
import { OpCode } from "./vm-types.js";

// =========================================================================
// Types — The contracts that language plugins implement
// =========================================================================

/**
 * A compile handler is a function that knows how to compile one specific
 * kind of AST node into bytecode instructions.
 *
 * The handler receives two arguments:
 *   - ``compiler`` — the GenericCompiler instance, used to emit instructions,
 *     add constants, manage scopes, and recursively compile child nodes.
 *   - ``node`` — the ASTNode being compiled, so the handler can inspect
 *     its children and extract information.
 *
 * Handlers are the "language-specific" part. A Python plugin registers
 * handlers for ``"if_statement"``, ``"for_loop"``, etc. A Ruby plugin
 * registers handlers for ``"method_definition"``, ``"block"``, etc.
 *
 * Example handler for a number literal rule:
 *
 *     compiler.registerRule("number_literal", (c, node) => {
 *       const token = node.children[0] as TokenNode;
 *       const value = Number(token.value);
 *       const index = c.addConstant(value);
 *       c.emit(OpCode.LOAD_CONST, index);
 *     });
 */
export type CompileHandler = (compiler: GenericCompiler, node: ASTNode) => void;

/**
 * An AST node — a non-terminal in the parse tree.
 *
 * Every interior node in the tree has a ``ruleName`` (which grammar rule
 * produced it) and a list of ``children`` (the sub-expressions and tokens
 * that make up this construct).
 *
 * The ``ruleName`` is the key used for dispatch: the compiler looks up
 * the registered handler for this rule name and calls it.
 */
export interface ASTNode {
  /** The grammar rule that produced this node (e.g., "if_statement"). */
  ruleName: string;

  /** The child nodes — a mix of ASTNodes (sub-rules) and TokenNodes (leaves). */
  children: (ASTNode | TokenNode)[];
}

/**
 * A token node — a terminal (leaf) in the parse tree.
 *
 * Token nodes represent the actual characters from the source code. They
 * have a ``type`` (the token category, like ``"NUMBER"`` or ``"IDENTIFIER"``)
 * and a ``value`` (the raw text, like ``"42"`` or ``"myVar"``).
 *
 * Token nodes do NOT have a ``ruleName`` property — this is how the compiler
 * distinguishes them from ASTNodes.
 */
export interface TokenNode {
  /** The token category (e.g., "NUMBER", "STRING", "IDENTIFIER", "OPERATOR"). */
  type: string;

  /** The raw text from the source code (e.g., "42", "hello", "+"). */
  value: string;
}

// =========================================================================
// Error Types — Clear diagnostics for plugin authors
// =========================================================================

/**
 * Base class for all compiler errors.
 *
 * Using a hierarchy of error classes (rather than plain ``Error``) lets
 * callers catch specific error types:
 *
 *     try {
 *       compiler.compile(ast);
 *     } catch (e) {
 *       if (e instanceof UnhandledRuleError) {
 *         console.log("Missing handler for:", e.message);
 *       }
 *     }
 */
export class CompilerError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CompilerError";
  }
}

/**
 * Thrown when the compiler encounters an ASTNode whose ``ruleName`` has no
 * registered handler AND the node has more than one child (so pass-through
 * is not possible).
 *
 * This typically means the language plugin is incomplete — it forgot to
 * register a handler for this grammar rule.
 */
export class UnhandledRuleError extends CompilerError {
  constructor(ruleName: string) {
    super(
      `No handler registered for rule "${ruleName}" and node has multiple children. ` +
        `Register a handler with compiler.registerRule("${ruleName}", handler).`,
    );
    this.name = "UnhandledRuleError";
  }
}

// =========================================================================
// CompilerScope — Local variable tracking for nested scopes
// =========================================================================

/**
 * The interface that all compiler scopes must satisfy.
 *
 * A scope tracks the local variables visible within a particular region of
 * code (a function body, a block, a module). Each local variable is assigned
 * a numeric "slot index" — this is what ``STORE_LOCAL`` and ``LOAD_LOCAL``
 * instructions reference.
 *
 * Scopes form a linked list: each scope has a ``parent`` pointer to the
 * enclosing scope. This enables lexical scoping — if a variable isn't found
 * in the current scope, you can walk up the chain to look in enclosing scopes.
 *
 * Real VMs do this too:
 * - The JVM uses a "local variable array" per stack frame, indexed by slot.
 * - CPython uses a ``co_varnames`` tuple, indexed by slot.
 * - Our scope's ``locals`` map serves the same purpose.
 */
export interface CompilerScope {
  /** Map from variable name to slot index. */
  locals: Map<string, number>;

  /** The enclosing scope, or null if this is the outermost scope. */
  parent: CompilerScope | null;

  /**
   * Register a new local variable and return its slot index.
   * If the name already exists, returns the existing slot index (deduplication).
   */
  addLocal(name: string): number;

  /**
   * Look up a variable's slot index by name.
   * Returns ``undefined`` if the variable is not in this scope.
   * (Does NOT search parent scopes — that's the language plugin's job.)
   */
  getLocal(name: string): number | undefined;

  /** The total number of local variables registered in this scope. */
  numLocals: number;
}

// =========================================================================
// DefaultCompilerScope — The built-in scope implementation
// =========================================================================

/**
 * Default implementation of ``CompilerScope``.
 *
 * This is a simple scope that assigns consecutive slot indices to local
 * variables as they are registered. It supports optional "pre-loading" of
 * parameter names — when entering a function scope, the parameter names
 * are assigned to slots 0, 1, 2, etc. before any other locals.
 *
 * Example:
 *
 *     // A function def(x, y): ...
 *     const scope = new DefaultCompilerScope(null, ["x", "y"]);
 *     scope.getLocal("x");  // => 0
 *     scope.getLocal("y");  // => 1
 *     scope.addLocal("temp"); // => 2
 *     scope.numLocals;       // => 3
 */
export class DefaultCompilerScope implements CompilerScope {
  /** Map from variable name to slot index. */
  locals: Map<string, number>;

  /** The enclosing scope, or null for the outermost scope. */
  parent: CompilerScope | null;

  /**
   * Create a new scope.
   *
   * @param parent - The enclosing scope (null for the outermost/global scope).
   * @param params - Optional parameter names to pre-assign to slots 0, 1, ...
   *
   * Pre-assigning parameters ensures they get the lowest slot indices, which
   * is a convention shared by the JVM (parameters occupy the first local
   * variable slots) and CPython (parameters are the first entries in
   * ``co_varnames``).
   */
  constructor(parent: CompilerScope | null, params?: string[]) {
    this.parent = parent;
    this.locals = new Map();

    // Pre-assign parameter slots: slot 0 for the first param, slot 1 for
    // the second, etc. This mirrors how the JVM's local variable table
    // works — method parameters always come first.
    if (params) {
      for (const name of params) {
        this.locals.set(name, this.locals.size);
      }
    }
  }

  /**
   * Register a local variable and return its slot index.
   *
   * If the variable was already registered (e.g., it's a parameter that was
   * pre-assigned), we return the existing slot rather than creating a
   * duplicate. This deduplication prevents bugs where ``addLocal("x")``
   * called twice would give different slot indices for the same variable.
   *
   * @param name - The variable name to register.
   * @returns The slot index assigned to this variable.
   */
  addLocal(name: string): number {
    const existing = this.locals.get(name);
    if (existing !== undefined) {
      return existing;
    }
    const slot = this.locals.size;
    this.locals.set(name, slot);
    return slot;
  }

  /**
   * Look up a variable's slot index.
   *
   * Returns ``undefined`` if the variable is not in this scope. This method
   * does NOT walk up the parent chain — that's intentional. Different
   * languages handle scope lookup differently (some have closures, some
   * don't), so we leave parent-scope resolution to the language plugin.
   *
   * @param name - The variable name to look up.
   * @returns The slot index, or undefined if not found.
   */
  getLocal(name: string): number | undefined {
    return this.locals.get(name);
  }

  /**
   * The total number of local variables in this scope.
   *
   * This is needed when generating the function's metadata — the VM needs
   * to know how many local slots to allocate when entering a function call.
   */
  get numLocals(): number {
    return this.locals.size;
  }
}

// =========================================================================
// Type guard — distinguishing ASTNode from TokenNode
// =========================================================================

/**
 * Type guard: is this node a TokenNode (leaf) rather than an ASTNode?
 *
 * The distinguishing feature is that TokenNodes have a ``type`` property
 * but no ``ruleName`` property. ASTNodes have ``ruleName`` but no ``type``.
 *
 * This is a standard TypeScript "type narrowing" pattern — after calling
 * ``isTokenNode(node)`` in an ``if`` block, TypeScript knows the node is
 * a ``TokenNode`` inside the true branch.
 */
export function isTokenNode(node: ASTNode | TokenNode): node is TokenNode {
  return "type" in node && !("ruleName" in node);
}

// =========================================================================
// GenericCompiler — The pluggable compilation framework
// =========================================================================

/**
 * A pluggable AST-to-bytecode compiler framework.
 *
 * The GenericCompiler provides the *infrastructure* for compilation:
 * instruction emission, constant/name pool management, scope tracking,
 * jump patching, and nested code object compilation. Language-specific
 * *behavior* is provided by registering ``CompileHandler`` functions for
 * each AST rule name.
 *
 * Think of it like a kitchen (GenericCompiler) with cooking equipment
 * (emit, addConstant, enterScope, etc.) — and the chef (language plugin)
 * decides what dish to make by registering recipes (handlers).
 *
 * =======================================================================
 * Usage Pattern
 * =======================================================================
 *
 *     // 1. Create the compiler
 *     const compiler = new GenericCompiler();
 *
 *     // 2. Register language-specific handlers
 *     compiler.registerRule("number", (c, node) => {
 *       const tok = node.children[0] as TokenNode;
 *       c.emit(OpCode.LOAD_CONST, c.addConstant(Number(tok.value)));
 *     });
 *
 *     compiler.registerRule("addition", (c, node) => {
 *       c.compileNode(node.children[0]);  // left operand
 *       c.compileNode(node.children[2]);  // right operand (skip operator token)
 *       c.emit(OpCode.ADD);
 *     });
 *
 *     // 3. Compile an AST
 *     const code = compiler.compile(ast);
 *
 *     // 4. Execute with the VM
 *     const vm = new VirtualMachine();
 *     vm.execute(code);
 *
 * =======================================================================
 * State Management During Compilation
 * =======================================================================
 *
 * The compiler maintains mutable state as it walks the AST:
 *
 * - ``instructions`` — The growing list of bytecode instructions.
 * - ``constants`` — The constant pool (deduplicated literals).
 * - ``names`` — The name pool (deduplicated identifiers).
 * - ``scope`` — The current local variable scope (or null).
 *
 * During ``compileNested()``, this state is saved, a fresh set is created
 * for the nested code (e.g., a function body), and then the original state
 * is restored. This is how the JVM compiler handles inner classes and
 * lambdas — each gets its own code unit.
 */
export class GenericCompiler {
  /** The bytecode instructions emitted so far, in order. */
  instructions: Instruction[] = [];

  /**
   * The constant pool — literal values referenced by ``LOAD_CONST``.
   *
   * We allow ``null`` in addition to numbers and strings because some
   * languages have a null/nil/None literal that needs to be representable
   * as a constant.
   */
  constants: (number | string | null)[] = [];

  /** The name pool — variable/function names referenced by index. */
  names: string[] = [];

  /** The current local variable scope, or null if not inside a scope. */
  scope: CompilerScope | null = null;

  /**
   * The handler registry — maps rule names to compile handlers.
   *
   * This is the core of the plugin architecture. Each entry says:
   * "When you see a node with ruleName X, call handler Y."
   */
  private dispatch: Map<string, CompileHandler> = new Map();

  /**
   * Accumulated code objects from ``compileNested()`` calls.
   *
   * When compiling a function definition, the function body is compiled
   * as a separate ``CodeObject`` and stored here. The outer code can then
   * reference it by index.
   */
  private codeObjects: CodeObject[] = [];

  // -------------------------------------------------------------------
  // Plugin registration
  // -------------------------------------------------------------------

  /**
   * Register a compile handler for a specific AST rule name.
   *
   * This is how language plugins teach the compiler about their syntax.
   * Each rule name (e.g., ``"if_statement"``, ``"for_loop"``) gets a
   * handler function that knows how to compile that construct.
   *
   * If a handler was already registered for the same rule name, it is
   * silently replaced. This allows plugins to override default behavior.
   *
   * @param ruleName - The AST rule name to handle (e.g., "binary_expression").
   * @param handler - The function that compiles nodes with this rule name.
   *
   * Example:
   *
   *     compiler.registerRule("print_statement", (c, node) => {
   *       c.compileNode(node.children[1]); // compile the expression to print
   *       c.emit(OpCode.PRINT);
   *     });
   */
  registerRule(ruleName: string, handler: CompileHandler): void {
    this.dispatch.set(ruleName, handler);
  }

  // -------------------------------------------------------------------
  // Instruction emission
  // -------------------------------------------------------------------

  /**
   * Emit a single bytecode instruction and return its index.
   *
   * This is the fundamental building block of compilation. Every handler
   * ultimately calls ``emit()`` one or more times to produce bytecode.
   *
   * The returned index is useful for jump patching — you might emit a
   * ``JUMP_IF_FALSE`` now and patch its target later when you know where
   * the else-branch starts.
   *
   * @param opcode - The operation code (from the OpCode enum).
   * @param operand - Optional data for the instruction (constant index,
   *                  name index, jump target, etc.).
   * @returns The index of the emitted instruction in the instructions array.
   *
   * Example:
   *
   *     // Emit "push the constant at pool index 0 onto the stack"
   *     const idx = compiler.emit(OpCode.LOAD_CONST, 0);
   *     // idx is the position of this instruction (e.g., 0, 1, 2, ...)
   */
  emit(opcode: number, operand?: number | string | null): number {
    const instruction: Instruction =
      operand !== undefined
        ? { opcode, operand }
        : { opcode };
    this.instructions.push(instruction);
    return this.instructions.length - 1;
  }

  /**
   * Emit a jump instruction with a placeholder operand (0).
   *
   * Jump instructions (``JUMP``, ``JUMP_IF_FALSE``, ``JUMP_IF_TRUE``) need
   * a target address, but at the time we emit the jump, we often don't know
   * the target yet (because we haven't compiled the code after the jump).
   *
   * The solution is a two-step process:
   *
   *     1. ``emitJump(opcode)`` — Emit the jump with operand=0 (placeholder).
   *     2. ``patchJump(index)`` — Later, fill in the real target.
   *
   * This is called **backpatching** and is used by every real compiler:
   * - The JVM's ``javac`` uses it for if/else, loops, and try/catch.
   * - GCC's code generator uses it for branch instructions.
   * - LLVM uses it for conditional branches in its IR.
   *
   * @param opcode - The jump opcode (JUMP, JUMP_IF_FALSE, etc.).
   * @returns The index of the emitted instruction (needed for ``patchJump``).
   *
   * Example:
   *
   *     // Compiling "if (cond) { ... } else { ... }"
   *     compileCondition(cond);
   *     const jumpToElse = compiler.emitJump(OpCode.JUMP_IF_FALSE);
   *     compileThenBranch();
   *     const jumpOverElse = compiler.emitJump(OpCode.JUMP);
   *     compiler.patchJump(jumpToElse);  // else starts here
   *     compileElseBranch();
   *     compiler.patchJump(jumpOverElse); // after else
   */
  emitJump(opcode: number): number {
    return this.emit(opcode, 0);
  }

  /**
   * Patch a previously emitted jump instruction with the real target.
   *
   * If ``target`` is provided, the jump goes to that specific instruction
   * index. If omitted, the jump targets ``currentOffset`` — the next
   * instruction that will be emitted. This default is the most common
   * case: "jump to whatever comes next."
   *
   * @param index - The index of the jump instruction to patch (as returned
   *                by ``emitJump``).
   * @param target - The target instruction index. Defaults to ``currentOffset``.
   *
   * @throws CompilerError if the index is out of bounds.
   */
  patchJump(index: number, target?: number): void {
    const instruction = this.instructions[index];
    if (!instruction) {
      throw new CompilerError(
        `Cannot patch jump at index ${index}: instruction does not exist.`,
      );
    }
    // Preserve the opcode, replace only the operand.
    // We create a new object because Instruction has readonly fields.
    this.instructions[index] = {
      opcode: instruction.opcode,
      operand: target !== undefined ? target : this.currentOffset,
    };
  }

  /**
   * The current instruction offset — the index where the *next* emitted
   * instruction will be placed.
   *
   * This is used for jump target calculations. If ``currentOffset`` is 5,
   * the next ``emit()`` call will place an instruction at index 5.
   */
  get currentOffset(): number {
    return this.instructions.length;
  }

  // -------------------------------------------------------------------
  // Pool management — constants and names
  // -------------------------------------------------------------------

  /**
   * Add a value to the constant pool and return its index.
   *
   * Constants are **deduplicated**: if the value already exists in the pool,
   * the existing index is returned instead of adding a duplicate. This saves
   * space and is how real VMs work — the JVM's constant pool deduplicates
   * strings, and CPython's compiler deduplicates constants.
   *
   * We use strict equality (``===``) for deduplication, which means:
   * - ``42`` and ``42`` are the same (reuses the slot).
   * - ``"hello"`` and ``"hello"`` are the same.
   * - ``null`` and ``null`` are the same.
   * - ``0`` and ``"0"`` are different (number vs. string).
   *
   * @param value - The constant to add (number, string, or null).
   * @returns The index of the constant in the pool.
   *
   * Example:
   *
   *     const i1 = compiler.addConstant(42);   // => 0 (new entry)
   *     const i2 = compiler.addConstant("hi"); // => 1 (new entry)
   *     const i3 = compiler.addConstant(42);   // => 0 (deduplicated!)
   */
  addConstant(value: number | string | null): number {
    // Search for an existing entry with the same value.
    // We use a simple linear scan. Real compilers use a hash map for O(1)
    // lookup, but for our educational purposes, linear scan is clear and
    // correct. The constant pool is typically small (dozens of entries).
    const existing = this.constants.indexOf(value);
    if (existing !== -1) {
      return existing;
    }
    this.constants.push(value);
    return this.constants.length - 1;
  }

  /**
   * Add a name to the name pool and return its index.
   *
   * Like ``addConstant``, names are deduplicated. The same variable name
   * used in multiple places gets the same index.
   *
   * @param name - The identifier name to add.
   * @returns The index of the name in the pool.
   *
   * Example:
   *
   *     const i1 = compiler.addName("x");     // => 0
   *     const i2 = compiler.addName("y");     // => 1
   *     const i3 = compiler.addName("x");     // => 0 (deduplicated!)
   */
  addName(name: string): number {
    const existing = this.names.indexOf(name);
    if (existing !== -1) {
      return existing;
    }
    this.names.push(name);
    return this.names.length - 1;
  }

  // -------------------------------------------------------------------
  // Scope management
  // -------------------------------------------------------------------

  /**
   * Enter a new local variable scope.
   *
   * This pushes a new ``CompilerScope`` onto the scope stack. If ``params``
   * are provided, they are pre-assigned to local slots (slot 0 for the first
   * param, slot 1 for the second, etc.).
   *
   * Scopes are linked: the new scope's ``parent`` points to the previous
   * scope (or null if there was none). This enables lexical scoping.
   *
   * @param params - Optional parameter names to pre-assign to local slots.
   * @returns The newly created scope.
   *
   * Example:
   *
   *     // Entering a function scope with two parameters
   *     const scope = compiler.enterScope(["x", "y"]);
   *     // scope.getLocal("x") => 0
   *     // scope.getLocal("y") => 1
   */
  enterScope(params?: string[]): CompilerScope {
    const newScope = new DefaultCompilerScope(this.scope, params);
    this.scope = newScope;
    return newScope;
  }

  /**
   * Exit the current scope, restoring the parent scope.
   *
   * Returns the scope that was just exited, so the caller can inspect its
   * ``numLocals`` (needed for function metadata) or other properties.
   *
   * @returns The scope that was just exited.
   * @throws CompilerError if not currently inside a scope.
   *
   * Example:
   *
   *     compiler.enterScope(["x"]);
   *     // ... compile function body ...
   *     const funcScope = compiler.exitScope();
   *     console.log(funcScope.numLocals); // How many locals the function uses
   */
  exitScope(): CompilerScope {
    if (this.scope === null) {
      throw new CompilerError(
        "Cannot exit scope: not currently inside a scope. " +
          "Did you call exitScope() without a matching enterScope()?",
      );
    }
    const exited = this.scope;
    this.scope = exited.parent;
    return exited;
  }

  // -------------------------------------------------------------------
  // Node compilation — the recursive dispatch engine
  // -------------------------------------------------------------------

  /**
   * Compile a nested code object (e.g., a function body).
   *
   * This saves the compiler's current state (instructions, constants, names),
   * compiles the given AST node into a fresh code unit, then restores the
   * original state. The nested code object is returned and also stored in
   * the ``codeObjects`` list.
   *
   * This is how real compilers handle functions-within-functions:
   * - CPython compiles each function body as a separate ``code`` object.
   * - The JVM compiles inner classes and lambdas as separate ``.class`` files.
   * - Our ``compileNested`` does the same thing.
   *
   * @param node - The AST node to compile as a nested code object.
   * @returns The compiled CodeObject for the nested code.
   *
   * Example:
   *
   *     compiler.registerRule("function_def", (c, node) => {
   *       const nameToken = node.children[1] as TokenNode;
   *       const body = node.children[3] as ASTNode;
   *       const funcCode = c.compileNested(body);
   *       // funcCode is a self-contained CodeObject for the function body
   *     });
   */
  compileNested(node: ASTNode): CodeObject {
    // Save current state — we'll restore it after compiling the nested code.
    const savedInstructions = this.instructions;
    const savedConstants = this.constants;
    const savedNames = this.names;

    // Start fresh for the nested code unit.
    this.instructions = [];
    this.constants = [];
    this.names = [];

    // Compile the nested AST into the fresh state.
    this.compileNode(node);

    // Package the result as a CodeObject.
    const codeObject: CodeObject = {
      instructions: this.instructions,
      constants: this.constants,
      names: this.names,
    };

    // Store the code object for later reference.
    this.codeObjects.push(codeObject);

    // Restore the outer compilation state.
    this.instructions = savedInstructions;
    this.constants = savedConstants;
    this.names = savedNames;

    return codeObject;
  }

  /**
   * Compile a single AST node or token node.
   *
   * This is the main dispatch method — the recursive heart of the compiler.
   * It decides what to do based on the node type:
   *
   *     1. **TokenNode** (leaf): Call ``compileToken()``, which is a no-op
   *        by default. Language plugins can override this if they need to
   *        handle specific tokens (e.g., emitting LOAD_CONST for number
   *        tokens).
   *
   *     2. **ASTNode with a registered handler**: Call the handler. This is
   *        the normal case — the language plugin knows how to compile this
   *        rule.
   *
   *     3. **ASTNode with one child and no handler**: Pass through to the
   *        child. This handles "wrapper" grammar rules that exist for
   *        precedence but don't need compilation logic.
   *
   *     4. **ASTNode with multiple children and no handler**: Throw
   *        ``UnhandledRuleError``. The language plugin is missing a handler
   *        for a rule that requires one.
   *
   * @param node - The node to compile (ASTNode or TokenNode).
   * @throws UnhandledRuleError if no handler exists for a multi-child ASTNode.
   */
  compileNode(node: ASTNode | TokenNode): void {
    // Case 1: Token nodes (leaves) are handled by compileToken.
    if (isTokenNode(node)) {
      this.compileToken(node);
      return;
    }

    // Case 2: Look for a registered handler for this rule name.
    const handler = this.dispatch.get(node.ruleName);
    if (handler) {
      handler(this, node);
      return;
    }

    // Case 3: No handler, but single child — pass through.
    // This is the "wrapper rule" case: the node exists only for grammar
    // structure, not for any compilation logic.
    if (node.children.length === 1) {
      this.compileNode(node.children[0]);
      return;
    }

    // Case 4: No handler and multiple children — error.
    // We can't guess what to do with multiple children, so we require
    // an explicit handler.
    throw new UnhandledRuleError(node.ruleName);
  }

  /**
   * Compile a token node.
   *
   * By default, this is a **no-op** — tokens are typically handled by their
   * parent ASTNode's handler, which knows the context (is this number a
   * literal? is this identifier a variable reference? a function name?).
   *
   * Language plugins can override this method (or the handlers that call
   * ``compileNode`` on tokens) if they need token-level compilation.
   *
   * @param _token - The token node (unused in the default implementation).
   */
  compileToken(_token: TokenNode): void {
    // No-op by default. The underscore prefix signals "intentionally unused."
    // Tokens are typically consumed by their parent node's handler.
  }

  // -------------------------------------------------------------------
  // Top-level compilation
  // -------------------------------------------------------------------

  /**
   * Compile an entire AST into a CodeObject.
   *
   * This is the main entry point for compilation. It:
   *   1. Compiles the root AST node (which recursively compiles all children).
   *   2. Appends a ``HALT`` instruction (or a custom halt opcode) to ensure
   *      the VM stops after executing the program.
   *   3. Returns a self-contained ``CodeObject`` with instructions, constants,
   *      and names — ready for the VM to execute.
   *
   * @param ast - The root AST node to compile.
   * @param haltOpcode - The opcode to use for the final halt instruction.
   *                     Defaults to ``OpCode.HALT`` (0xFF). Some VMs or
   *                     backends may use a different halt instruction.
   * @returns A compiled CodeObject ready for execution.
   *
   * Example:
   *
   *     const compiler = new GenericCompiler();
   *     // ... register handlers ...
   *     const code = compiler.compile(ast);
   *     // code.instructions ends with HALT
   *     // code.constants has all literals
   *     // code.names has all identifiers
   */
  compile(ast: ASTNode, haltOpcode?: number): CodeObject {
    // Compile the entire tree, emitting instructions as we go.
    this.compileNode(ast);

    // Append the halt instruction. Every program must end with HALT,
    // just like every real CPU program must eventually stop. Without this,
    // the VM would try to read past the end of the instruction array.
    this.emit(haltOpcode !== undefined ? haltOpcode : OpCode.HALT);

    // Package everything into a self-contained CodeObject.
    return {
      instructions: this.instructions,
      constants: this.constants,
      names: this.names,
    };
  }
}
