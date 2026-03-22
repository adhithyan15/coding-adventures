/**
 * Starlark Compiler -- Compiles Starlark ASTs to bytecode.
 *
 * ==========================================================================
 * Chapter 1: The Starlark Compilation Pipeline
 * ==========================================================================
 *
 * The full pipeline from source code to execution is:
 *
 *     Starlark source code
 *         | (starlark_lexer)
 *     Token stream
 *         | (starlark_parser)
 *     AST (ASTNode tree)
 *         | (THIS MODULE)
 *     CodeObject (bytecode)
 *         | (starlark_vm)
 *     Execution result
 *
 * This module handles the AST -> CodeObject step. It registers handlers for
 * all Starlark grammar rules with the ``GenericCompiler`` framework, then
 * provides a ``compileStarlark()`` convenience function that does the
 * full source -> bytecode path.
 *
 * ==========================================================================
 * Chapter 2: How Rule Handlers Work
 * ==========================================================================
 *
 * Each Starlark grammar rule (``file``, ``assign_stmt``, ``if_stmt``, etc.)
 * gets a corresponding handler function. The handler receives the compiler
 * and the AST node, then:
 *
 * 1. Inspects the node's children to understand the source construct.
 * 2. Calls ``compiler.compileNode(child)`` to recursively compile sub-expressions.
 * 3. Calls ``compiler.emit(opcode)`` to emit bytecode instructions.
 *
 * For example, the ``assign_stmt`` handler for ``x = 1 + 2``:
 *
 * 1. Compiles the RHS expression (``1 + 2``) -> emits LOAD_CONST, LOAD_CONST, ADD
 * 2. Emits STORE_NAME for the LHS (``x``)
 *
 * ==========================================================================
 * Chapter 3: Grammar Rules Reference
 * ==========================================================================
 *
 * The Starlark grammar defines ~55 rules. Not all need dedicated handlers --
 * many are pass-through rules (single child, no semantics). Here's the
 * breakdown:
 *
 * **Rules with handlers** (do real work):
 *     file, simple_stmt, assign_stmt, return_stmt, break_stmt, continue_stmt,
 *     pass_stmt, load_stmt, if_stmt, for_stmt, def_stmt, suite,
 *     expression, expression_list, or_expr, and_expr, not_expr, comparison,
 *     arith, term, factor, power, primary, atom, list_expr, dict_expr,
 *     paren_expr, lambda_expr, arguments, argument
 *
 * **Pass-through rules** (single child, handled automatically):
 *     statement, compound_stmt, small_stmt, bitwise_or, bitwise_xor,
 *     bitwise_and, shift
 */

import {
  GenericCompiler,
  type ASTNode,
  type TokenNode,
} from "@coding-adventures/bytecode-compiler";

/**
 * Type guard: is this node a TokenNode (leaf) rather than an ASTNode?
 *
 * The distinguishing feature is that TokenNodes have a ``type`` property
 * but no ``ruleName`` property. ASTNodes have ``ruleName`` but no ``type``.
 *
 * This is the same logic used by the GenericCompiler's internal isTokenNode,
 * but we define our own since it's not exported from the bytecode-compiler
 * package's public API.
 */
function isTokenNode(node: ASTNode | TokenNode): node is TokenNode {
  return "type" in node && !("ruleName" in node);
}
import { parseStarlark } from "@coding-adventures/starlark-parser";

import { Op, BINARY_OP_MAP, COMPARE_OP_MAP, AUGMENTED_ASSIGN_MAP } from "./opcodes.js";

// =========================================================================
// Helper: extracting children by type
// =========================================================================

/**
 * Extract all TokenNode children from an ASTNode.
 *
 * Many handlers need to inspect token children (to find operators, keywords,
 * or literal values) separately from ASTNode children (which represent
 * sub-expressions). This helper filters for just the tokens.
 */
function tokens(node: ASTNode): TokenNode[] {
  return node.children.filter((c): c is TokenNode => isTokenNode(c));
}

/**
 * Extract all ASTNode children from an ASTNode.
 *
 * The complement of ``tokens()`` -- gives us just the sub-expression nodes,
 * which are the parts that need recursive compilation.
 */
function nodes(node: ASTNode): ASTNode[] {
  return node.children.filter((c): c is ASTNode => !isTokenNode(c));
}

/**
 * Check if any child token has the given value.
 *
 * Useful for detecting operators, keywords, and punctuation. For example,
 * ``hasToken(node, "=")`` checks if there's an equals sign among the
 * children, which distinguishes assignment from expression statements.
 */
function hasToken(node: ASTNode, value: string): boolean {
  return node.children.some(
    (c) => isTokenNode(c) && c.value === value
  );
}

/**
 * Get the type name from a token.
 *
 * Token types can come as plain strings. This helper normalizes them.
 */
function typeName(token: TokenNode): string {
  return token.type;
}

// =========================================================================
// Rule Handlers -- Top-Level Structure
// =========================================================================

/**
 * Compile a Starlark file -- a sequence of statements.
 *
 * Grammar: file = { NEWLINE | statement } ;
 *
 * The file rule is the root of every Starlark AST. Its children are
 * a mix of NEWLINE tokens (which we skip) and statement nodes (which
 * we compile). This is the entry point for compilation -- every Starlark
 * program starts here.
 */
function compileFile(compiler: GenericCompiler, node: ASTNode): void {
  for (const child of node.children) {
    if (!isTokenNode(child)) {
      compiler.compileNode(child);
    }
    // Skip NEWLINE tokens -- they're structural, not semantic.
  }
}

/**
 * Compile a simple statement line.
 *
 * Grammar: simple_stmt = small_stmt { SEMICOLON small_stmt } NEWLINE ;
 *
 * A simple statement line can contain multiple small statements separated
 * by semicolons (e.g., ``x = 1; y = 2``). We compile each small_stmt child
 * and skip the SEMICOLON and NEWLINE tokens.
 */
function compileSimpleStmt(compiler: GenericCompiler, node: ASTNode): void {
  for (const child of node.children) {
    if (!isTokenNode(child)) {
      compiler.compileNode(child);
    }
    // Skip SEMICOLON and NEWLINE tokens
  }
}

/**
 * Compile an assignment or expression statement.
 *
 * Grammar: assign_stmt = expression_list
 *                         [ ( assign_op | augmented_assign_op ) expression_list ] ;
 *
 * This handler covers four cases:
 *
 * 1. **Expression statement**: ``f(x)`` -- compile expr, emit POP.
 *    The POP discards the unused return value. This is what happens when
 *    you write a bare function call or expression as a statement.
 *
 * 2. **Simple assignment**: ``x = expr`` -- compile RHS, emit STORE_NAME.
 *    The RHS value ends up on the stack, then STORE_NAME pops it and
 *    binds it to the variable name.
 *
 * 3. **Augmented assignment**: ``x += expr`` -- load x, compile RHS, op, store x.
 *    This is syntactic sugar: ``x += 1`` is equivalent to ``x = x + 1``,
 *    but we compile it more efficiently by avoiding a second name lookup.
 *
 * 4. **Tuple unpacking**: ``a, b = 1, 2`` -- compile RHS, UNPACK_SEQUENCE.
 *    The UNPACK_SEQUENCE instruction splits a tuple/list into individual
 *    values and pushes them onto the stack in reverse order.
 */
function compileAssignStmt(compiler: GenericCompiler, node: ASTNode): void {
  const subNodes = nodes(node);

  if (subNodes.length === 1) {
    // Case 1: Expression statement (no assignment operator).
    // Compile the expression and discard the result with POP.
    compiler.compileNode(subNodes[0]);
    compiler.emit(Op.POP);
    return;
  }

  // Cases 2-4: assignment
  // subNodes[0] = LHS (expression_list), subNodes[-1] = RHS (expression_list)
  const lhs = subNodes[0];
  const rhs = subNodes[subNodes.length - 1];

  // Find the operator node (assign_op or augmented_assign_op)
  let opNode: ASTNode | null = null;
  for (const child of node.children) {
    if (
      !isTokenNode(child) &&
      (child.ruleName === "assign_op" || child.ruleName === "augmented_assign_op")
    ) {
      opNode = child;
      break;
    }
  }

  if (opNode !== null && opNode.ruleName === "augmented_assign_op") {
    // Augmented assignment: x += expr -> LOAD x, compile expr, ADD, STORE x
    const opToken = opNode.children[0];
    if (isTokenNode(opToken)) {
      const arithOp = AUGMENTED_ASSIGN_MAP[opToken.value];

      // Load the current value of the target
      compileLoadTarget(compiler, lhs);
      // Compile the RHS
      compiler.compileNode(rhs);
      // Emit the arithmetic operation
      if (arithOp !== undefined) {
        compiler.emit(arithOp);
      }
      // Store back to the target
      compileStoreTarget(compiler, lhs);
    }
  } else {
    // Simple assignment: x = expr or a, b = expr
    // Compile RHS first -- this pushes the value onto the stack
    compiler.compileNode(rhs);

    // Check for tuple unpacking (multiple names on LHS)
    const lhsExprs = nodes(lhs);
    if (lhsExprs.length > 1) {
      // Tuple unpacking: a, b = ...
      compiler.emit(Op.UNPACK_SEQUENCE, lhsExprs.length);
      for (const expr of lhsExprs) {
        compileStoreTarget(compiler, expr);
      }
    } else {
      // Single assignment: x = ...
      compileStoreTarget(compiler, lhs);
    }
  }
}

/**
 * Emit load instructions for an assignment target (for augmented assign).
 *
 * When compiling ``x += 1``, we need to first load the current value of x.
 * This helper handles the LOAD_NAME/LOAD_LOCAL distinction based on whether
 * we're inside a function scope.
 */
function compileLoadTarget(compiler: GenericCompiler, target: ASTNode): void {
  const name = extractSimpleName(target);
  if (name !== null) {
    const idx = compiler.addName(name);
    if (compiler.scope && compiler.scope.getLocal(name) !== undefined) {
      compiler.emit(Op.LOAD_LOCAL, compiler.scope.getLocal(name)!);
    } else {
      compiler.emit(Op.LOAD_NAME, idx);
    }
    return;
  }
  // For subscript/attribute targets, compile the object and key
  compiler.compileNode(target);
}

/**
 * Emit store instructions for an assignment target.
 *
 * Handles three kinds of targets:
 * - Simple name: ``x`` -> STORE_NAME (or STORE_LOCAL inside functions)
 * - Subscript: ``obj[key]`` -> STORE_SUBSCRIPT
 * - Attribute: ``obj.attr`` -> STORE_ATTR
 *
 * The local vs. global distinction is important for performance:
 * STORE_LOCAL uses a numeric slot index (fast array access), while
 * STORE_NAME uses a string key (dictionary lookup). Real VMs like
 * CPython and the JVM make the same distinction.
 */
function compileStoreTarget(compiler: GenericCompiler, target: ASTNode): void {
  const name = extractSimpleName(target);
  if (name !== null) {
    if (compiler.scope && compiler.scope.getLocal(name) !== undefined) {
      const slot = compiler.scope.getLocal(name)!;
      compiler.emit(Op.STORE_LOCAL, slot);
    } else {
      const idx = compiler.addName(name);
      compiler.emit(Op.STORE_NAME, idx);
    }
    return;
  }

  // Not a simple name -- complex assignment targets not yet supported
  throw new Error(
    `Complex assignment targets not yet supported: ${target.ruleName}`
  );
}

/**
 * Try to extract a simple variable name from an expression node.
 *
 * Starlark's grammar wraps identifiers in multiple layers of nodes:
 *
 *     expression_list -> expression -> or_expr -> ... -> atom -> NAME
 *
 * This function "unwraps" single-child nodes until it finds a NAME token,
 * or returns null if the expression is not a simple variable reference
 * (e.g., it's a subscript like ``obj[key]`` or an attribute like ``obj.attr``).
 */
function extractSimpleName(node: ASTNode | TokenNode): string | null {
  let current: ASTNode | TokenNode = node;
  // Unwrap single-child wrappers: each level of the grammar precedence
  // chain (expression -> or_expr -> and_expr -> ... -> atom) produces
  // a single-child node when the expression is just a simple name.
  while (!isTokenNode(current) && current.children.length === 1) {
    current = current.children[0];
  }

  if (isTokenNode(current) && typeName(current) === "NAME") {
    return current.value;
  }
  return null;
}

// =========================================================================
// Rule Handlers -- Simple Statements
// =========================================================================

/**
 * Compile a return statement.
 *
 * Grammar: return_stmt = "return" [ expression ] ;
 *
 * If there's no expression, push None as the return value. Every function
 * must return *something* -- even ``return`` with no value returns None.
 * This mirrors Python's behavior: ``def f(): return`` is equivalent to
 * ``def f(): return None``.
 */
function compileReturnStmt(compiler: GenericCompiler, node: ASTNode): void {
  const subNodes = nodes(node);
  if (subNodes.length > 0) {
    compiler.compileNode(subNodes[0]);
  } else {
    compiler.emit(Op.LOAD_NONE);
  }
  compiler.emit(Op.RETURN);
}

/**
 * Compile a break statement.
 *
 * Grammar: break_stmt = "break" ;
 *
 * Break is compiled as a JUMP to the end of the enclosing for-loop.
 * We emit a placeholder jump that gets patched when the for-loop
 * compilation completes.
 *
 * We use a convention: the compiler stores pending break jumps in a
 * list on the compiler instance (set up by compileForStmt). This is
 * the same approach CPython uses -- the compiler maintains a stack of
 * "break targets" for nested loops.
 */
function compileBreakStmt(compiler: GenericCompiler, _node: ASTNode): void {
  const c = compiler as GenericCompiler & { _breakJumps?: number[][] };
  if (c._breakJumps && c._breakJumps.length > 0) {
    const jumpIdx = compiler.emitJump(Op.JUMP);
    c._breakJumps[c._breakJumps.length - 1].push(jumpIdx);
  } else {
    throw new SyntaxError("'break' outside of a for loop");
  }
}

/**
 * Compile a continue statement.
 *
 * Grammar: continue_stmt = "continue" ;
 *
 * Continue jumps to the top of the for-loop (back to FOR_ITER).
 * The target address is stored by the enclosing for-loop handler.
 */
function compileContinueStmt(compiler: GenericCompiler, _node: ASTNode): void {
  const c = compiler as GenericCompiler & { _continueTargets?: number[] };
  if (c._continueTargets && c._continueTargets.length > 0) {
    compiler.emit(Op.JUMP, c._continueTargets[c._continueTargets.length - 1]);
  } else {
    throw new SyntaxError("'continue' outside of a for loop");
  }
}

/**
 * Compile a pass statement.
 *
 * Grammar: pass_stmt = "pass" ;
 *
 * Pass is a no-op -- we emit nothing. This is exactly what CPython does.
 * The ``pass`` keyword exists purely for syntactic reasons: Python/Starlark
 * requires at least one statement in a block, so ``pass`` serves as a
 * placeholder when you want an empty block.
 */
function compilePassStmt(_compiler: GenericCompiler, _node: ASTNode): void {
  // No-op: pass does nothing, and we emit nothing.
}

/**
 * Compile a load statement.
 *
 * Grammar: load_stmt = "load" LPAREN STRING { COMMA load_arg } [ COMMA ] RPAREN ;
 *
 * The load statement is Starlark's import mechanism. It loads symbols from
 * another Starlark module:
 *
 *     load("module.star", "symbol1", alias = "symbol2")
 *
 * Compilation:
 * 1. Emit LOAD_MODULE to load the module object
 * 2. For each symbol: DUP the module, IMPORT_FROM to extract, STORE_NAME to bind
 * 3. POP the module object (we've extracted what we need)
 */
function compileLoadStmt(compiler: GenericCompiler, node: ASTNode): void {
  // Extract the module path (first STRING token)
  const toks = tokens(node);
  let modulePath: string | null = null;
  for (const t of toks) {
    if (typeName(t) === "STRING") {
      modulePath = parseStringLiteral(t.value);
      break;
    }
  }

  if (modulePath === null) {
    throw new SyntaxError("load() requires a module path string");
  }

  // Emit LOAD_MODULE
  const moduleIdx = compiler.addName(modulePath);
  compiler.emit(Op.LOAD_MODULE, moduleIdx);

  // Process load_arg children
  const loadArgs = node.children.filter(
    (c): c is ASTNode => !isTokenNode(c) && c.ruleName === "load_arg"
  );
  for (const arg of loadArgs) {
    compileLoadArg(compiler, arg);
  }

  // Pop the module object (we've extracted what we need)
  compiler.emit(Op.POP);
}

/**
 * Compile a single load argument.
 *
 * Grammar: load_arg = NAME EQUALS STRING | STRING ;
 *
 * Two forms:
 * - ``"symbol"`` -> imports symbol with its original name
 * - ``alias = "symbol"`` -> imports symbol with a local alias
 *
 * In both cases, we DUP the module (so it stays on the stack for the
 * next argument), extract the symbol with IMPORT_FROM, and bind it
 * with STORE_NAME.
 */
function compileLoadArg(compiler: GenericCompiler, node: ASTNode): void {
  const toks = tokens(node);

  if (hasToken(node, "=")) {
    // alias = "symbol"
    let alias: string | null = null;
    let symbol: string | null = null;
    for (const t of toks) {
      if (typeName(t) === "NAME") {
        alias = t.value;
      } else if (typeName(t) === "STRING") {
        symbol = parseStringLiteral(t.value);
      }
    }
    if (alias && symbol) {
      const symIdx = compiler.addName(symbol);
      compiler.emit(Op.DUP); // keep module on stack
      compiler.emit(Op.IMPORT_FROM, symIdx);
      const aliasIdx = compiler.addName(alias);
      compiler.emit(Op.STORE_NAME, aliasIdx);
    }
  } else {
    // "symbol" -> import with original name
    for (const t of toks) {
      if (typeName(t) === "STRING") {
        const symbol = parseStringLiteral(t.value);
        const symIdx = compiler.addName(symbol);
        compiler.emit(Op.DUP); // keep module on stack
        compiler.emit(Op.IMPORT_FROM, symIdx);
        compiler.emit(Op.STORE_NAME, symIdx);
        break;
      }
    }
  }
}

// =========================================================================
// Rule Handlers -- Compound Statements
// =========================================================================

/**
 * Compile an if/elif/else statement.
 *
 * Grammar: if_stmt = "if" expression COLON suite
 *                     { "elif" expression COLON suite }
 *                     [ "else" COLON suite ] ;
 *
 * Compilation pattern (classic "jump threading"):
 *
 *     compile condition
 *     JUMP_IF_FALSE -> elif_or_else
 *     compile if-body
 *     JUMP -> end
 *     elif_or_else:
 *     compile elif-condition (if any)
 *     JUMP_IF_FALSE -> next_elif_or_else
 *     compile elif-body
 *     JUMP -> end
 *     ...
 *     else:
 *     compile else-body (if any)
 *     end:
 *
 * This is the standard pattern used by CPython, the JVM, and every real
 * compiler. The key insight is that we don't know where "end" is when we
 * emit the JUMP instructions, so we use placeholder targets (0) and patch
 * them later with ``patchJump()``. This is called **backpatching**.
 */
function compileIfStmt(compiler: GenericCompiler, node: ASTNode): void {
  // Collect sections: (condition_node, suite_node) pairs, plus optional else_suite
  const sections: Array<{ cond: ASTNode | null; suite: ASTNode }> = [];
  let i = 0;
  const children = node.children;

  while (i < children.length) {
    const child = children[i];
    if (isTokenNode(child) && (child.value === "if" || child.value === "elif")) {
      // Next child is the condition, then COLON, then suite
      const cond = children[i + 1] as ASTNode;
      // Find the suite after the COLON
      let suite: ASTNode | null = null;
      for (let j = i + 2; j < children.length; j++) {
        const c = children[j];
        if (!isTokenNode(c) && c.ruleName === "suite") {
          suite = c;
          i = j + 1;
          break;
        }
      }
      if (suite) {
        sections.push({ cond, suite });
      }
    } else if (isTokenNode(child) && child.value === "else") {
      // Find the suite after the COLON
      for (let j = i + 1; j < children.length; j++) {
        const c = children[j];
        if (!isTokenNode(c) && c.ruleName === "suite") {
          sections.push({ cond: null, suite: c });
          i = j + 1;
          break;
        }
      }
      break;
    } else {
      i += 1;
    }
  }

  // Now compile: condition -> JUMP_IF_FALSE -> body -> JUMP -> end
  const endJumps: number[] = [];

  for (const { cond, suite } of sections) {
    if (cond !== null) {
      // Conditional branch
      compiler.compileNode(cond);
      const falseJump = compiler.emitJump(Op.JUMP_IF_FALSE);
      compileSuite(compiler, suite);
      const endJump = compiler.emitJump(Op.JUMP);
      endJumps.push(endJump);
      compiler.patchJump(falseJump);
    } else {
      // Else branch -- no condition
      compileSuite(compiler, suite);
    }
  }

  // Patch all end-jumps to point here (the instruction after the whole if/elif/else)
  for (const j of endJumps) {
    compiler.patchJump(j);
  }
}

/**
 * Compile a for loop.
 *
 * Grammar: for_stmt = "for" loop_vars "in" expression COLON suite ;
 *
 * Compilation pattern (same as CPython):
 *
 *     compile iterable
 *     GET_ITER
 *     loop_top:
 *     FOR_ITER -> loop_end    (jump past body when iterator exhausted)
 *     store loop variable(s)
 *     compile body
 *     JUMP -> loop_top
 *     loop_end:
 *
 * The FOR_ITER instruction is special: it tries to get the next value from
 * the iterator. If successful, it pushes the value and continues. If the
 * iterator is exhausted, it pops the iterator and jumps to the target.
 * This is exactly how CPython's FOR_ITER works.
 */
function compileForStmt(compiler: GenericCompiler, node: ASTNode): void {
  // Extract children: loop_vars, iterable (expression after "in"), suite
  let loopVarsNode: ASTNode | null = null;
  let iterableNode: ASTNode | null = null;
  let suiteNode: ASTNode | null = null;

  let foundIn = false;
  for (const child of node.children) {
    if (isTokenNode(child) && child.value === "in") {
      foundIn = true;
      continue;
    }
    if (!isTokenNode(child)) {
      if (child.ruleName === "loop_vars" && !foundIn) {
        loopVarsNode = child;
      } else if (child.ruleName === "suite") {
        suiteNode = child;
      } else if (foundIn && iterableNode === null) {
        iterableNode = child;
      }
    }
  }

  if (!loopVarsNode || !iterableNode || !suiteNode) {
    throw new SyntaxError("Malformed for statement");
  }

  // Initialize break/continue tracking on the compiler instance.
  // We use a stack (array of arrays) to support nested loops.
  const c = compiler as GenericCompiler & {
    _breakJumps?: number[][];
    _continueTargets?: number[];
  };
  if (!c._breakJumps) c._breakJumps = [];
  if (!c._continueTargets) c._continueTargets = [];

  // Compile the iterable and get its iterator
  compiler.compileNode(iterableNode);
  compiler.emit(Op.GET_ITER);

  // loop_top: FOR_ITER -> loop_end
  const loopTop = compiler.currentOffset;
  c._continueTargets.push(loopTop);
  c._breakJumps.push([]);

  const forIterJump = compiler.emitJump(Op.FOR_ITER);

  // Store loop variable(s)
  compileLoopVarsStore(compiler, loopVarsNode);

  // Compile body
  compileSuite(compiler, suiteNode);

  // Jump back to top
  compiler.emit(Op.JUMP, loopTop);

  // loop_end:
  compiler.patchJump(forIterJump);

  // Patch break jumps
  const breakJumps = c._breakJumps.pop()!;
  for (const breakJump of breakJumps) {
    compiler.patchJump(breakJump);
  }
  c._continueTargets.pop();
}

/**
 * Store loop variables after FOR_ITER pushes the next value.
 *
 * Grammar: loop_vars = NAME { COMMA NAME } ;
 *
 * Single variable: ``for x in ...`` -> just STORE_NAME x
 * Multiple: ``for x, y in ...`` -> UNPACK_SEQUENCE 2, STORE_NAME x, STORE_NAME y
 *
 * When there are multiple loop variables, the iterable must yield sequences
 * of the right length. The UNPACK_SEQUENCE instruction splits the sequence.
 */
function compileLoopVarsStore(compiler: GenericCompiler, node: ASTNode): void {
  const names = node.children.filter(
    (c): c is TokenNode => isTokenNode(c) && typeName(c) === "NAME"
  );

  if (names.length > 1) {
    compiler.emit(Op.UNPACK_SEQUENCE, names.length);
  }

  for (const nameToken of names) {
    const idx = compiler.addName(nameToken.value);
    if (compiler.scope && compiler.scope.getLocal(nameToken.value) !== undefined) {
      compiler.emit(Op.STORE_LOCAL, compiler.scope.getLocal(nameToken.value)!);
    } else {
      compiler.emit(Op.STORE_NAME, idx);
    }
  }
}

/**
 * Compile a function definition.
 *
 * Grammar: def_stmt = "def" NAME LPAREN [ parameters ] RPAREN COLON suite ;
 *
 * Compilation pattern:
 * 1. Compile default parameter values (pushed onto stack)
 * 2. Compile the function body as a nested CodeObject
 * 3. Emit MAKE_FUNCTION to create the function object
 * 4. Emit STORE_NAME to bind the function to its name
 *
 * The function body is compiled as a *separate* CodeObject -- it has its
 * own instruction list, constant pool, and name pool. This is exactly how
 * CPython handles function definitions: each ``def`` produces a new
 * ``code`` object embedded in the outer code's constant pool.
 */
function compileDefStmt(compiler: GenericCompiler, node: ASTNode): void {
  // Extract name, parameters, and suite
  let funcName: string | null = null;
  let paramsNode: ASTNode | null = null;
  let suiteNode: ASTNode | null = null;

  for (const child of node.children) {
    if (isTokenNode(child) && typeName(child) === "NAME" && funcName === null) {
      funcName = child.value;
    } else if (!isTokenNode(child) && child.ruleName === "parameters") {
      paramsNode = child;
    } else if (!isTokenNode(child) && child.ruleName === "suite") {
      suiteNode = child;
    }
  }

  if (!funcName || !suiteNode) {
    throw new SyntaxError("Malformed function definition");
  }

  // Parse parameters
  const paramNames: string[] = [];
  let defaultCount = 0;
  let hasVarargs = false;
  let hasKwargs = false;

  if (paramsNode !== null) {
    for (const paramChild of paramsNode.children) {
      if (!isTokenNode(paramChild) && paramChild.ruleName === "parameter") {
        const paramInfo = parseParameter(paramChild);
        if (paramInfo.kind === "varargs") {
          hasVarargs = true;
          paramNames.push("*" + paramInfo.name);
        } else if (paramInfo.kind === "kwargs") {
          hasKwargs = true;
          paramNames.push("**" + paramInfo.name);
        } else {
          paramNames.push(paramInfo.name);
          if (paramInfo.defaultNode !== null) {
            compiler.compileNode(paramInfo.defaultNode);
            defaultCount++;
          }
        }
      }
    }
  }

  // Compile the function body as a nested CodeObject.
  // Enter a new scope for the function -- parameter names get slots 0, 1, 2, ...
  const cleanParamNames = paramNames.map((n) => n.replace(/^\*+/, ""));
  compiler.enterScope(cleanParamNames);

  const bodyCode = compiler.compileNested(suiteNode);

  compiler.exitScope();

  // Push the parameter names tuple as a constant (needed for keyword arg dispatch)
  const paramNamesIdx = compiler.addConstant(cleanParamNames.join(","));
  compiler.emit(Op.LOAD_CONST, paramNamesIdx);

  // Push the body CodeObject as a constant
  const codeIdx = compiler.addConstant(bodyCode as unknown as string);
  compiler.emit(Op.LOAD_CONST, codeIdx);

  // Emit MAKE_FUNCTION with flags encoding parameter properties
  // Flags: bit 0 = has defaults, bit 1 = has varargs,
  //        bit 2 = has kwargs, bit 3 = has param names tuple
  let flags = 0x08; // Always include param names (bit 3)
  if (defaultCount > 0) flags |= 0x01;
  if (hasVarargs) flags |= 0x02;
  if (hasKwargs) flags |= 0x04;
  compiler.emit(Op.MAKE_FUNCTION, flags);

  // Store as a named function
  const nameIdx = compiler.addName(funcName);
  compiler.emit(Op.STORE_NAME, nameIdx);
}

/**
 * Parse a parameter node into its components.
 *
 * Grammar: parameter = DOUBLE_STAR NAME | STAR NAME | NAME EQUALS expression | NAME ;
 *
 * Returns an object describing the parameter's name, kind, and optional default.
 * The "kind" field is one of:
 * - "positional" -- a plain parameter (e.g., ``x``)
 * - "default" -- a parameter with a default value (e.g., ``x=1``)
 * - "varargs" -- a *args parameter (e.g., ``*args``)
 * - "kwargs" -- a **kwargs parameter (e.g., ``**kwargs``)
 */
function parseParameter(
  node: ASTNode
): { name: string; kind: string; defaultNode: ASTNode | null } {
  const toks = tokens(node);
  const subNodes = nodes(node);

  if (hasToken(node, "**")) {
    const name = toks.find((t) => typeName(t) === "NAME")!.value;
    return { name, kind: "kwargs", defaultNode: null };
  } else if (hasToken(node, "*")) {
    const name = toks.find((t) => typeName(t) === "NAME")!.value;
    return { name, kind: "varargs", defaultNode: null };
  } else if (hasToken(node, "=")) {
    const name = toks.find((t) => typeName(t) === "NAME")!.value;
    const defaultNode = subNodes.length > 0 ? subNodes[0] : null;
    return { name, kind: "default", defaultNode };
  } else {
    const name = toks[0].value;
    return { name, kind: "positional", defaultNode: null };
  }
}

/**
 * Compile a suite (function/if/for body).
 *
 * Grammar: suite = simple_stmt | NEWLINE INDENT { statement } DEDENT ;
 *
 * A suite is either:
 * 1. A single simple_stmt on the same line: ``if True: pass``
 * 2. An indented block: ``if True:\n    x = 1\n    y = 2``
 *
 * In both cases, we just compile all ASTNode children and skip tokens
 * (NEWLINE, INDENT, DEDENT are structural markers, not semantic).
 */
function compileSuite(compiler: GenericCompiler, node: ASTNode): void {
  for (const child of node.children) {
    if (!isTokenNode(child)) {
      compiler.compileNode(child);
    }
    // Skip NEWLINE, INDENT, DEDENT tokens
  }
}

// =========================================================================
// Rule Handlers -- Expressions
// =========================================================================

/**
 * Compile an expression (possibly with ternary if/else).
 *
 * Grammar: expression = lambda_expr | or_expr [ "if" or_expr "else" expression ] ;
 *
 * The ternary form ``value_if_true if condition else value_if_false`` is
 * Starlark's conditional expression. Note the unusual order compared to
 * C's ``condition ? true_val : false_val`` -- in Starlark, the "true" value
 * comes first, then the condition, then the "false" value.
 *
 * Compilation pattern:
 *     compile condition
 *     JUMP_IF_FALSE -> else_branch
 *     compile value_if_true
 *     JUMP -> end
 *     else_branch:
 *     compile value_if_false
 *     end:
 */
function compileExpression(compiler: GenericCompiler, node: ASTNode): void {
  const subNodes = nodes(node);

  if (subNodes.length === 1) {
    // No ternary -- pass through
    compiler.compileNode(subNodes[0]);
    return;
  }

  // Check for ternary: value "if" condition "else" value
  if (hasToken(node, "if") && hasToken(node, "else")) {
    // Children order: or_expr "if" or_expr "else" expression
    // subNodes[0] = value_if_true (first or_expr)
    // subNodes[1] = condition (second or_expr)
    // subNodes[2] = value_if_false (expression after "else")
    const valueTrue = subNodes[0];
    const condition = subNodes[1];
    const valueFalse = subNodes[2];

    compiler.compileNode(condition);
    const falseJump = compiler.emitJump(Op.JUMP_IF_FALSE);
    compiler.compileNode(valueTrue);
    const endJump = compiler.emitJump(Op.JUMP);
    compiler.patchJump(falseJump);
    compiler.compileNode(valueFalse);
    compiler.patchJump(endJump);
  } else {
    // Shouldn't happen for well-formed Starlark
    compiler.compileNode(subNodes[0]);
  }
}

/**
 * Compile an expression list (possible tuple creation).
 *
 * Grammar: expression_list = expression { COMMA expression } [ COMMA ] ;
 *
 * If there's just one expression (no commas), compile it directly.
 * If there are multiple, build a tuple. A trailing comma after a single
 * expression also creates a single-element tuple: ``(x,)``.
 */
function compileExpressionList(compiler: GenericCompiler, node: ASTNode): void {
  const exprs = nodes(node);

  if (exprs.length === 1) {
    // Check for trailing comma -> single-element tuple
    const hasTrailingComma = node.children.some(
      (c) => isTokenNode(c) && c.value === ","
    );
    if (hasTrailingComma) {
      compiler.compileNode(exprs[0]);
      compiler.emit(Op.BUILD_TUPLE, 1);
    } else {
      compiler.compileNode(exprs[0]);
    }
  } else {
    for (const expr of exprs) {
      compiler.compileNode(expr);
    }
    compiler.emit(Op.BUILD_TUPLE, exprs.length);
  }
}

/**
 * Compile a boolean OR expression with short-circuit evaluation.
 *
 * Grammar: or_expr = and_expr { "or" and_expr } ;
 *
 * Short-circuit: ``a or b`` -> if a is truthy, result is a (don't eval b).
 *
 * Compilation pattern:
 *     compile a
 *     JUMP_IF_TRUE_OR_POP -> end   (if truthy, keep a on stack)
 *     compile b
 *     end:
 *
 * JUMP_IF_TRUE_OR_POP is special: if the top of stack is truthy, it
 * leaves the value on the stack and jumps. Otherwise, it pops the value
 * and falls through to evaluate the next operand. This is how Python
 * implements ``or`` -- the result is the first truthy operand, not a boolean.
 */
function compileOrExpr(compiler: GenericCompiler, node: ASTNode): void {
  const subNodes = nodes(node);

  if (subNodes.length === 1) {
    compiler.compileNode(subNodes[0]);
    return;
  }

  // Compile first operand
  compiler.compileNode(subNodes[0]);

  // For each subsequent operand: short-circuit if truthy
  const endJumps: number[] = [];
  for (let i = 1; i < subNodes.length; i++) {
    const jump = compiler.emitJump(Op.JUMP_IF_TRUE_OR_POP);
    endJumps.push(jump);
    compiler.compileNode(subNodes[i]);
  }

  for (const j of endJumps) {
    compiler.patchJump(j);
  }
}

/**
 * Compile a boolean AND expression with short-circuit evaluation.
 *
 * Grammar: and_expr = not_expr { "and" not_expr } ;
 *
 * Short-circuit: ``a and b`` -> if a is falsy, result is a (don't eval b).
 *
 * Compilation pattern:
 *     compile a
 *     JUMP_IF_FALSE_OR_POP -> end   (if falsy, keep a on stack)
 *     compile b
 *     end:
 *
 * This is the mirror image of OR: instead of keeping truthy values, we
 * keep falsy values. The result of ``a and b`` is the first falsy operand,
 * or the last operand if all are truthy.
 */
function compileAndExpr(compiler: GenericCompiler, node: ASTNode): void {
  const subNodes = nodes(node);

  if (subNodes.length === 1) {
    compiler.compileNode(subNodes[0]);
    return;
  }

  compiler.compileNode(subNodes[0]);

  const endJumps: number[] = [];
  for (let i = 1; i < subNodes.length; i++) {
    const jump = compiler.emitJump(Op.JUMP_IF_FALSE_OR_POP);
    endJumps.push(jump);
    compiler.compileNode(subNodes[i]);
  }

  for (const j of endJumps) {
    compiler.patchJump(j);
  }
}

/**
 * Compile a boolean NOT expression.
 *
 * Grammar: not_expr = "not" not_expr | comparison ;
 *
 * ``not x`` -> compile x, emit NOT
 *
 * The NOT opcode pops one value and pushes its logical negation.
 * Note that ``not`` in Starlark is a logical operator, not a bitwise
 * one -- ``not True`` is ``False``, not ``-2`` (which is what ``~True``
 * would give in Python).
 */
function compileNotExpr(compiler: GenericCompiler, node: ASTNode): void {
  if (hasToken(node, "not")) {
    const subNodes = nodes(node);
    compiler.compileNode(subNodes[0]);
    compiler.emit(Op.NOT);
  } else {
    const subNodes = nodes(node);
    compiler.compileNode(subNodes[0]);
  }
}

/**
 * Compile a comparison expression.
 *
 * Grammar: comparison = bitwise_or { comp_op bitwise_or } ;
 *
 * Examples:
 *     ``a == b`` -> compile a, compile b, CMP_EQ
 *     ``a in b`` -> compile a, compile b, CMP_IN
 *     ``a not in b`` -> compile a, compile b, CMP_NOT_IN
 *
 * Starlark supports chained comparisons (``a < b < c``), though they're
 * less common than in Python. For now we compile each pair independently.
 */
function compileComparison(compiler: GenericCompiler, node: ASTNode): void {
  const subNodes = nodes(node);

  if (subNodes.length === 1) {
    compiler.compileNode(subNodes[0]);
    return;
  }

  // Separate operands and operators
  const operands: ASTNode[] = [];
  const operators: ASTNode[] = [];
  for (const sn of subNodes) {
    if (sn.ruleName === "comp_op") {
      operators.push(sn);
    } else {
      operands.push(sn);
    }
  }

  // Compile first operand
  compiler.compileNode(operands[0]);

  for (let i = 0; i < operators.length; i++) {
    compiler.compileNode(operands[i + 1]);
    const opStr = extractCompOp(operators[i]);
    const opcode = COMPARE_OP_MAP[opStr];
    if (opcode !== undefined) {
      compiler.emit(opcode);
    } else {
      throw new SyntaxError(`Unknown comparison operator: ${opStr}`);
    }
  }
}

/**
 * Extract the comparison operator string from a comp_op node.
 *
 * Grammar: comp_op = EQUALS_EQUALS | NOT_EQUALS | ... | "in" | "not" "in" ;
 *
 * The tricky case is ``not in`` -- the parser produces two separate tokens
 * ("not" and "in"), which we need to concatenate into the string "not in"
 * for the COMPARE_OP_MAP lookup.
 */
function extractCompOp(node: ASTNode): string {
  const toks = tokens(node);
  if (toks.length === 2 && toks[0].value === "not" && toks[1].value === "in") {
    return "not in";
  }
  if (toks.length > 0) {
    return toks[0].value;
  }
  return "";
}

/**
 * Compile a binary operation (arith, term, shift, bitwise_*).
 *
 * Grammar patterns:
 *     arith = term { ( PLUS | MINUS ) term } ;
 *     term  = factor { ( STAR | SLASH | FLOOR_DIV | PERCENT ) factor } ;
 *     shift = arith { ( LEFT_SHIFT | RIGHT_SHIFT ) arith } ;
 *     bitwise_or  = bitwise_xor { PIPE bitwise_xor } ;
 *     bitwise_xor = bitwise_and { CARET bitwise_and } ;
 *     bitwise_and = shift { AMP shift } ;
 *
 * All these follow the same pattern: left-associative binary operations.
 * Compile left operand, then for each (operator, right operand) pair:
 * compile right, emit the operation.
 *
 * This single handler covers 6 grammar rules -- they all have the same
 * compilation logic, just with different operators. The BINARY_OP_MAP
 * maps each operator token to its bytecode opcode.
 */
function compileBinaryOp(compiler: GenericCompiler, node: ASTNode): void {
  const children = node.children;

  // First child is always an operand
  compiler.compileNode(children[0]);

  // Process pairs: (operator_token, operand)
  let i = 1;
  while (i < children.length) {
    const child = children[i];
    if (isTokenNode(child)) {
      const opValue = child.value;
      // Next child should be the right operand
      if (i + 1 < children.length) {
        compiler.compileNode(children[i + 1]);
        const opcode = BINARY_OP_MAP[opValue];
        if (opcode !== undefined) {
          compiler.emit(opcode);
        } else {
          throw new SyntaxError(`Unknown binary operator: ${opValue}`);
        }
        i += 2;
      } else {
        i += 1;
      }
    } else {
      // Shouldn't happen in well-formed AST
      compiler.compileNode(child);
      i += 1;
    }
  }
}

/**
 * Compile a unary factor expression.
 *
 * Grammar: factor = ( PLUS | MINUS | TILDE ) factor | power ;
 *
 * ``-x`` -> compile x, emit NEGATE
 * ``~x`` -> compile x, emit BIT_NOT
 * ``+x`` -> compile x (no-op, but validates it's numeric)
 *
 * Unary ``+`` is intentionally a no-op -- it exists for symmetry with ``-``
 * and for documentation purposes (e.g., ``+42`` to emphasize that a number
 * is positive). We still evaluate the operand for type-checking side effects.
 */
function compileFactor(compiler: GenericCompiler, node: ASTNode): void {
  const children = node.children;

  if (children.length === 2 && isTokenNode(children[0])) {
    const op = (children[0] as TokenNode).value;
    compiler.compileNode(children[1]);
    if (op === "-") {
      compiler.emit(Op.NEGATE);
    } else if (op === "~") {
      compiler.emit(Op.BIT_NOT);
    }
    // unary + is a no-op
  } else if (children.length === 1) {
    compiler.compileNode(children[0]);
  } else {
    compiler.compileNode(children[0]);
  }
}

/**
 * Compile an exponentiation expression.
 *
 * Grammar: power = primary [ DOUBLE_STAR factor ] ;
 *
 * ``a ** b`` -> compile a, compile b, emit POWER
 *
 * Note: unlike other binary operations, exponentiation is right-associative
 * (``2 ** 3 ** 4`` = ``2 ** 81``), but the grammar handles this by having
 * the RHS be a ``factor`` (which includes further exponentiation) rather
 * than a ``primary``.
 */
function compilePower(compiler: GenericCompiler, node: ASTNode): void {
  const subNodes = nodes(node);

  if (subNodes.length === 1) {
    compiler.compileNode(subNodes[0]);
    return;
  }

  // a ** b
  compiler.compileNode(subNodes[0]);
  compiler.compileNode(subNodes[1]);
  compiler.emit(Op.POWER);
}

/**
 * Compile a primary expression (atom with suffixes).
 *
 * Grammar: primary = atom { suffix } ;
 *
 * A primary expression is an atom followed by zero or more suffixes:
 * - ``.attr`` -> LOAD_ATTR
 * - ``[key]`` -> LOAD_SUBSCRIPT
 * - ``(args)`` -> CALL_FUNCTION
 *
 * These suffixes "chain" onto the atom: ``obj.method(arg)[index]`` is
 * an atom ``obj`` with three suffixes: dot, call, subscript.
 */
function compilePrimary(compiler: GenericCompiler, node: ASTNode): void {
  const children = node.children;

  // Compile the atom (first child)
  compiler.compileNode(children[0]);

  // Apply each suffix
  for (let i = 1; i < children.length; i++) {
    const child = children[i];
    if (!isTokenNode(child) && child.ruleName === "suffix") {
      compileSuffix(compiler, child);
    }
  }
}

/**
 * Compile a single suffix (attribute, subscript, or call).
 *
 * Grammar: suffix = DOT NAME
 *                  | LBRACKET subscript RBRACKET
 *                  | LPAREN [ arguments ] RPAREN ;
 *
 * This is where function calls, attribute access, and indexing are compiled.
 * The object being accessed is already on the stack (compiled by the atom
 * or a previous suffix).
 */
function compileSuffix(compiler: GenericCompiler, node: ASTNode): void {
  const children = node.children;

  if (hasToken(node, ".")) {
    // Attribute access: obj.attr
    for (const child of children) {
      if (isTokenNode(child) && typeName(child) === "NAME") {
        const attrIdx = compiler.addName(child.value);
        compiler.emit(Op.LOAD_ATTR, attrIdx);
        break;
      }
    }
  } else if (hasToken(node, "[")) {
    // Subscript/slice: obj[key] or obj[start:stop:step]
    const subscriptNodes = children.filter(
      (c): c is ASTNode => !isTokenNode(c) && c.ruleName === "subscript"
    );
    if (subscriptNodes.length > 0) {
      compileSubscript(compiler, subscriptNodes[0]);
    } else {
      // Simple subscript with expression
      for (const c of children) {
        if (!isTokenNode(c)) {
          compiler.compileNode(c);
          compiler.emit(Op.LOAD_SUBSCRIPT);
          break;
        }
      }
    }
  } else if (hasToken(node, "(")) {
    // Function call: f(args)
    const argNodes = children.filter(
      (c): c is ASTNode => !isTokenNode(c) && c.ruleName === "arguments"
    );
    if (argNodes.length > 0) {
      const [argc, hasKw] = compileArguments(compiler, argNodes[0]);
      if (hasKw) {
        compiler.emit(Op.CALL_FUNCTION_KW, argc);
      } else {
        compiler.emit(Op.CALL_FUNCTION, argc);
      }
    } else {
      // No arguments: f()
      compiler.emit(Op.CALL_FUNCTION, 0);
    }
  }
}

/**
 * Compile a subscript expression.
 *
 * Grammar: subscript = expression
 *                     | [ expression ] COLON [ expression ] [ COLON [ expression ] ] ;
 *
 * Two cases:
 * 1. Simple index: ``obj[key]`` -> compile key, LOAD_SUBSCRIPT
 * 2. Slice: ``obj[start:stop:step]`` -> compile parts, LOAD_SLICE with flags
 *
 * The LOAD_SLICE operand is a flags byte indicating which parts are present:
 *   bit 0 = start present
 *   bit 1 = stop present
 *   bit 2 = step present
 *
 * Missing parts get LOAD_NONE pushed instead.
 */
function compileSubscript(compiler: GenericCompiler, node: ASTNode): void {
  if (hasToken(node, ":")) {
    // Slice: [start:stop:step]
    const parts: Array<ASTNode | null> = [];
    let currentExprs: ASTNode[] = [];
    let colonCount = 0;

    for (const child of node.children) {
      if (isTokenNode(child) && child.value === ":") {
        parts.push(currentExprs.length > 0 ? currentExprs[0] : null);
        currentExprs = [];
        colonCount++;
      } else if (!isTokenNode(child)) {
        currentExprs.push(child);
      }
    }
    // Last part
    parts.push(currentExprs.length > 0 ? currentExprs[0] : null);

    // Pad to 3 elements: [start, stop, step]
    while (parts.length < 3) {
      parts.push(null);
    }

    // Compile each part (push None for missing parts)
    let flags = 0;
    for (let i = 0; i < 3; i++) {
      if (parts[i] !== null) {
        compiler.compileNode(parts[i]!);
        flags |= 1 << i;
      } else {
        compiler.emit(Op.LOAD_NONE);
      }
    }

    compiler.emit(Op.LOAD_SLICE, flags);
  } else {
    // Simple index: [expr]
    const subNodes = nodes(node);
    if (subNodes.length > 0) {
      compiler.compileNode(subNodes[0]);
      compiler.emit(Op.LOAD_SUBSCRIPT);
    }
  }
}

/**
 * Compile function call arguments.
 *
 * Grammar: arguments = argument { COMMA argument } [ COMMA ] ;
 *
 * Returns [arg_count, has_keyword_args].
 *
 * ==========================================================================
 * Stack Layout Convention (CPython-style)
 * ==========================================================================
 *
 * For CALL_FUNCTION (no keyword args), the stack looks like:
 *     [func, arg1_value, arg2_value, ...]
 *
 * For CALL_FUNCTION_KW (has keyword args), the stack looks like:
 *     [func, pos_val1, ..., kw_val1, kw_val2, ..., kw_names_tuple]
 *
 * The keyword names tuple sits on top of the stack. Below it are all the
 * argument *values* in order: positional values first, then keyword values.
 */
function compileArguments(
  compiler: GenericCompiler,
  node: ASTNode
): [number, boolean] {
  const argNodes = node.children.filter(
    (c): c is ASTNode => !isTokenNode(c) && c.ruleName === "argument"
  );
  let argc = 0;
  let hasKw = false;
  const kwNames: string[] = [];

  for (const arg of argNodes) {
    const kwName = compileArgument(compiler, arg);
    if (kwName !== null) {
      hasKw = true;
      kwNames.push(kwName);
    }
    argc++;
  }

  // If there are keyword args, push a tuple of their names on top of the stack
  if (hasKw) {
    const kwTupleIdx = compiler.addConstant(kwNames.join(","));
    compiler.emit(Op.LOAD_CONST, kwTupleIdx);
  }

  return [argc, hasKw];
}

/**
 * Compile a single function call argument.
 *
 * Grammar: argument = DOUBLE_STAR expression | STAR expression
 *                    | NAME EQUALS expression | expression ;
 *
 * Returns the keyword name (string) if this is a keyword argument,
 * or null if positional. This lets ``compileArguments`` collect keyword
 * names to build the names tuple.
 *
 * For keyword arguments (``name=value``), we push only the *value* onto
 * the stack -- the name goes into the keyword names tuple that is pushed
 * separately after all arguments are compiled.
 */
function compileArgument(
  compiler: GenericCompiler,
  node: ASTNode
): string | null {
  if (hasToken(node, "**")) {
    // **kwargs unpacking
    const subNodes = nodes(node);
    if (subNodes.length > 0) {
      compiler.compileNode(subNodes[0]);
    }
    return null;
  } else if (hasToken(node, "*")) {
    // *args unpacking
    const subNodes = nodes(node);
    if (subNodes.length > 0) {
      compiler.compileNode(subNodes[0]);
    }
    return null;
  } else if (hasToken(node, "=")) {
    // Keyword argument: name=value
    const toks = tokens(node);
    const subNodes = nodes(node);
    let name: string | null = null;
    for (const t of toks) {
      if (typeName(t) === "NAME") {
        name = t.value;
        break;
      }
    }
    if (name && subNodes.length > 0) {
      compiler.compileNode(subNodes[0]);
    }
    return name;
  } else {
    // Positional argument
    const subNodes = nodes(node);
    if (subNodes.length > 0) {
      compiler.compileNode(subNodes[0]);
    } else {
      // Token-based expression
      for (const c of node.children) {
        if (isTokenNode(c)) {
          compiler.compileNode(c);
          break;
        }
      }
    }
    return null;
  }
}

// =========================================================================
// Rule Handlers -- Atoms
// =========================================================================

/**
 * Compile an atom -- the leaf-level expression.
 *
 * Grammar: atom = INT | FLOAT | STRING { STRING } | NAME
 *                | "True" | "False" | "None"
 *                | list_expr | dict_expr | paren_expr ;
 *
 * This is where literal values and variable references are compiled.
 * The atom handler is the most common entry point during compilation --
 * almost every expression eventually bottoms out at an atom.
 *
 * String concatenation: adjacent string literals (``"hello" "world"``) are
 * concatenated at compile time, producing a single LOAD_CONST instruction.
 * This is the same optimization CPython performs.
 */
function compileAtom(compiler: GenericCompiler, node: ASTNode): void {
  const children = node.children;

  if (children.length === 1) {
    const child = children[0];

    if (isTokenNode(child)) {
      const ttype = typeName(child);

      if (ttype === "INT") {
        // Integer literal: parse and add to constant pool
        const value = parseInt(child.value, 10);
        const idx = compiler.addConstant(value);
        compiler.emit(Op.LOAD_CONST, idx);
      } else if (ttype === "FLOAT") {
        // Float literal: parse and add to constant pool
        const value = parseFloat(child.value);
        const idx = compiler.addConstant(value);
        compiler.emit(Op.LOAD_CONST, idx);
      } else if (ttype === "STRING") {
        // String literal: strip quotes and handle escape sequences
        const value = parseStringLiteral(child.value);
        const idx = compiler.addConstant(value);
        compiler.emit(Op.LOAD_CONST, idx);
      } else if (ttype === "NAME") {
        // Variable reference or keyword literal
        if (child.value === "True") {
          compiler.emit(Op.LOAD_TRUE);
        } else if (child.value === "False") {
          compiler.emit(Op.LOAD_FALSE);
        } else if (child.value === "None") {
          compiler.emit(Op.LOAD_NONE);
        } else {
          // Variable reference -- use local slot if in scope, else name lookup
          if (
            compiler.scope &&
            compiler.scope.getLocal(child.value) !== undefined
          ) {
            const slot = compiler.scope.getLocal(child.value)!;
            compiler.emit(Op.LOAD_LOCAL, slot);
          } else {
            const idx = compiler.addName(child.value);
            compiler.emit(Op.LOAD_NAME, idx);
          }
        }
      } else {
        // Check for keyword literals by value (some parsers use KEYWORD type)
        if (child.value === "True") {
          compiler.emit(Op.LOAD_TRUE);
        } else if (child.value === "False") {
          compiler.emit(Op.LOAD_FALSE);
        } else if (child.value === "None") {
          compiler.emit(Op.LOAD_NONE);
        } else {
          throw new SyntaxError(`Unexpected token in atom: ${child.value}`);
        }
      }
    } else {
      // list_expr, dict_expr, or paren_expr
      compiler.compileNode(child);
    }
  } else if (children.length >= 2) {
    // Adjacent string concatenation: "hello" "world"
    const allStrings = children.every(
      (c) => isTokenNode(c) && typeName(c) === "STRING"
    );
    if (allStrings) {
      // Concatenate at compile time (just like Python)
      const concatenated = children
        .map((c) => parseStringLiteral((c as TokenNode).value))
        .join("");
      const idx = compiler.addConstant(concatenated);
      compiler.emit(Op.LOAD_CONST, idx);
    } else {
      // Shouldn't happen in well-formed Starlark
      for (const c of children) {
        if (!isTokenNode(c)) {
          compiler.compileNode(c);
        }
      }
    }
  }
}

/**
 * Parse a string literal, stripping quotes and handling escape sequences.
 *
 * Starlark supports both single and double quotes, and triple-quoted strings
 * for multi-line content. This function handles all quote styles and common
 * escape sequences: \\n, \\t, \\\\, \\", \\', \\r, \\0.
 */
function parseStringLiteral(s: string): string {
  // Strip outer quotes (single, double, or triple-quoted)
  if (s.startsWith('"""') || s.startsWith("'''")) {
    s = s.slice(3, -3);
  } else if (s.startsWith('"') || s.startsWith("'")) {
    s = s.slice(1, -1);
  }

  // Handle basic escape sequences
  const result: string[] = [];
  let i = 0;
  while (i < s.length) {
    if (s[i] === "\\" && i + 1 < s.length) {
      const c = s[i + 1];
      if (c === "n") {
        result.push("\n");
      } else if (c === "t") {
        result.push("\t");
      } else if (c === "\\") {
        result.push("\\");
      } else if (c === '"') {
        result.push('"');
      } else if (c === "'") {
        result.push("'");
      } else if (c === "r") {
        result.push("\r");
      } else if (c === "0") {
        result.push("\0");
      } else {
        result.push("\\");
        result.push(c);
      }
      i += 2;
    } else {
      result.push(s[i]);
      i += 1;
    }
  }

  return result.join("");
}

// =========================================================================
// Rule Handlers -- Collection Literals
// =========================================================================

/**
 * Compile a list literal or list comprehension.
 *
 * Grammar: list_expr = LBRACKET [ list_body ] RBRACKET ;
 *
 * Empty list: ``[]`` -> BUILD_LIST 0
 * Literal: ``[1, 2, 3]`` -> LOAD_CONST 1, LOAD_CONST 2, LOAD_CONST 3, BUILD_LIST 3
 * Comprehension: ``[x for x in lst]`` -> see compileListComprehension
 */
function compileListExpr(compiler: GenericCompiler, node: ASTNode): void {
  const bodyNodes = node.children.filter(
    (c): c is ASTNode => !isTokenNode(c) && c.ruleName === "list_body"
  );

  if (bodyNodes.length === 0) {
    // Empty list: []
    compiler.emit(Op.BUILD_LIST, 0);
    return;
  }

  compileListBody(compiler, bodyNodes[0]);
}

/**
 * Compile list body -- either literal elements or comprehension.
 *
 * Grammar: list_body = expression comp_clause
 *                     | expression { COMMA expression } [ COMMA ] ;
 */
function compileListBody(compiler: GenericCompiler, node: ASTNode): void {
  const subNodes = nodes(node);

  // Check for comprehension
  const hasComp = subNodes.some((sn) => sn.ruleName === "comp_clause");

  if (hasComp) {
    // List comprehension: [expr for x in iterable if cond]
    compileListComprehension(compiler, node);
  } else {
    // List literal: [expr, expr, ...]
    const exprs = subNodes.filter((sn) => sn.ruleName !== "comp_clause");
    for (const expr of exprs) {
      compiler.compileNode(expr);
    }
    compiler.emit(Op.BUILD_LIST, exprs.length);
  }
}

/**
 * Compile a list comprehension.
 *
 * [expr for x in iterable if cond]
 *
 * Compilation pattern:
 *     BUILD_LIST 0           # empty accumulator list
 *     compile iterable
 *     GET_ITER
 *     loop:
 *     FOR_ITER -> end
 *     store x
 *     compile condition (if any)
 *     JUMP_IF_FALSE -> loop_continue
 *     compile expr
 *     LIST_APPEND
 *     loop_continue:
 *     JUMP -> loop
 *     end:
 *
 * This is the standard way to compile comprehensions. CPython does the
 * same thing, except it wraps the comprehension in an implicit function.
 */
function compileListComprehension(
  compiler: GenericCompiler,
  node: ASTNode
): void {
  const subNodes = nodes(node);
  const exprNode = subNodes[0];
  const compClause = subNodes.find((sn) => sn.ruleName === "comp_clause")!;

  // Create empty list
  compiler.emit(Op.BUILD_LIST, 0);

  // Compile the comprehension clause(s)
  compileCompClause(compiler, compClause, exprNode, true);
}

/**
 * Compile comprehension for/if clauses.
 *
 * Grammar: comp_clause = comp_for { comp_for | comp_if } ;
 *
 * Comprehension clauses can be nested: ``[x for x in a for y in b if cond]``
 * has two comp_for clauses and one comp_if clause. We compile them recursively.
 */
function compileCompClause(
  compiler: GenericCompiler,
  node: ASTNode,
  exprNode: ASTNode,
  isList: boolean
): void {
  const subNodes = nodes(node);

  // First must be comp_for
  if (subNodes.length === 0 || subNodes[0].ruleName !== "comp_for") {
    return;
  }

  // Compile the first for clause, nesting subsequent clauses
  compileCompFor(compiler, subNodes, 0, exprNode, isList);
}

/**
 * Compile a single for clause in a comprehension, with nested clauses.
 *
 * This function is recursive: after compiling the current for clause's
 * loop setup, it calls itself for the next clause (or compiles the
 * expression and appends to the accumulator if there are no more clauses).
 */
function compileCompFor(
  compiler: GenericCompiler,
  clauses: ASTNode[],
  clauseIdx: number,
  exprNode: ASTNode,
  isList: boolean
): void {
  if (clauseIdx >= clauses.length) {
    // Base case: compile the expression and append
    compiler.compileNode(exprNode);
    if (isList) {
      compiler.emit(Op.LIST_APPEND);
    } else {
      compiler.emit(Op.DICT_SET);
    }
    return;
  }

  const clause = clauses[clauseIdx];

  if (clause.ruleName === "comp_for") {
    // for loop_vars in iterable
    let loopVarsNode: ASTNode | null = null;
    let iterableNode: ASTNode | null = null;
    let foundIn = false;

    for (const child of clause.children) {
      if (isTokenNode(child) && child.value === "in") {
        foundIn = true;
        continue;
      }
      if (!isTokenNode(child)) {
        if (!foundIn) {
          loopVarsNode = child;
        } else {
          iterableNode = child;
        }
      }
    }

    if (!loopVarsNode || !iterableNode) return;

    compiler.compileNode(iterableNode);
    compiler.emit(Op.GET_ITER);
    const loopTop = compiler.currentOffset;
    const forIterJump = compiler.emitJump(Op.FOR_ITER);
    compileLoopVarsStore(compiler, loopVarsNode);

    // Compile remaining clauses recursively
    compileCompFor(compiler, clauses, clauseIdx + 1, exprNode, isList);

    compiler.emit(Op.JUMP, loopTop);
    compiler.patchJump(forIterJump);
  } else if (clause.ruleName === "comp_if") {
    // if condition -- filter
    const subNodes = nodes(clause);
    if (subNodes.length > 0) {
      compiler.compileNode(subNodes[0]);
      const skipJump = compiler.emitJump(Op.JUMP_IF_FALSE);
      // Compile remaining clauses
      compileCompFor(compiler, clauses, clauseIdx + 1, exprNode, isList);
      compiler.patchJump(skipJump);
    } else {
      compileCompFor(compiler, clauses, clauseIdx + 1, exprNode, isList);
    }
  }
}

/**
 * Compile a dict literal or dict comprehension.
 *
 * Grammar: dict_expr = LBRACE [ dict_body ] RBRACE ;
 *
 * Empty dict: ``{}`` -> BUILD_DICT 0
 */
function compileDictExpr(compiler: GenericCompiler, node: ASTNode): void {
  const bodyNodes = node.children.filter(
    (c): c is ASTNode => !isTokenNode(c) && c.ruleName === "dict_body"
  );

  if (bodyNodes.length === 0) {
    // Empty dict: {}
    compiler.emit(Op.BUILD_DICT, 0);
    return;
  }

  compileDictBody(compiler, bodyNodes[0]);
}

/**
 * Compile dict body -- either literal entries or comprehension.
 *
 * Grammar: dict_body = dict_entry comp_clause
 *                     | dict_entry { COMMA dict_entry } [ COMMA ] ;
 */
function compileDictBody(compiler: GenericCompiler, node: ASTNode): void {
  const subNodes = nodes(node);

  const hasComp = subNodes.some((sn) => sn.ruleName === "comp_clause");

  if (hasComp) {
    // Dict comprehension
    compileDictComprehension(compiler, node);
  } else {
    // Dict literal: {key: val, ...}
    const entries = subNodes.filter((sn) => sn.ruleName === "dict_entry");
    for (const entry of entries) {
      compileDictEntry(compiler, entry);
    }
    compiler.emit(Op.BUILD_DICT, entries.length);
  }
}

/**
 * Compile a single dict entry (key: value).
 *
 * Grammar: dict_entry = expression COLON expression ;
 *
 * Pushes the key first, then the value. BUILD_DICT expects pairs on the
 * stack in [key1, val1, key2, val2, ...] order.
 */
function compileDictEntry(compiler: GenericCompiler, node: ASTNode): void {
  const subNodes = nodes(node);
  // First expression is key, second is value
  compiler.compileNode(subNodes[0]);
  compiler.compileNode(subNodes[1]);
}

/**
 * Compile a dict comprehension.
 *
 * {key: value for x in iterable if cond}
 *
 * Similar to list comprehension, but uses BUILD_DICT 0 for the accumulator
 * and DICT_SET instead of LIST_APPEND.
 */
function compileDictComprehension(
  compiler: GenericCompiler,
  node: ASTNode
): void {
  const subNodes = nodes(node);
  const entryNode = subNodes.find((sn) => sn.ruleName === "dict_entry")!;
  const compClause = subNodes.find((sn) => sn.ruleName === "comp_clause")!;

  // Create empty dict
  compiler.emit(Op.BUILD_DICT, 0);

  // The entry_node has key and value expressions
  compileCompClause(compiler, compClause, entryNode, false);
}

/**
 * Compile a parenthesized expression or tuple.
 *
 * Grammar: paren_expr = LPAREN [ paren_body ] RPAREN ;
 *
 * - ``()`` -> empty tuple
 * - ``(x)`` -> just x (parenthesized, not a tuple)
 * - ``(x,)`` -> single-element tuple
 * - ``(x, y)`` -> two-element tuple
 */
function compileParenExpr(compiler: GenericCompiler, node: ASTNode): void {
  const bodyNodes = node.children.filter(
    (c): c is ASTNode => !isTokenNode(c) && c.ruleName === "paren_body"
  );

  if (bodyNodes.length === 0) {
    // Empty tuple: ()
    compiler.emit(Op.BUILD_TUPLE, 0);
    return;
  }

  compileParenBody(compiler, bodyNodes[0]);
}

/**
 * Compile parenthesized expression body.
 *
 * Grammar: paren_body = expression comp_clause
 *                      | expression COMMA [ expression { COMMA expression } [ COMMA ] ]
 *                      | expression ;
 */
function compileParenBody(compiler: GenericCompiler, node: ASTNode): void {
  const subNodes = nodes(node);

  // Check for comprehension (generator expression)
  const hasComp = subNodes.some((sn) => sn.ruleName === "comp_clause");
  if (hasComp) {
    // Generator expression -- compile as list for now
    compileListComprehension(compiler, node);
    return;
  }

  // Check for commas (tuple)
  const hasComma = node.children.some(
    (c) => isTokenNode(c) && c.value === ","
  );

  if (hasComma) {
    // Tuple
    const exprs = subNodes.filter((sn) => sn.ruleName !== "comp_clause");
    for (const expr of exprs) {
      compiler.compileNode(expr);
    }
    compiler.emit(Op.BUILD_TUPLE, exprs.length);
  } else {
    // Parenthesized expression -- just compile the inner expression
    compiler.compileNode(subNodes[0]);
  }
}

/**
 * Compile a lambda expression.
 *
 * Grammar: lambda_expr = "lambda" [ lambda_params ] COLON expression ;
 *
 * Lambda is compiled just like a function definition, but anonymous.
 * The body is a single expression (not a suite), and there's no name
 * to bind it to -- the function object is left on the stack.
 */
function compileLambdaExpr(compiler: GenericCompiler, node: ASTNode): void {
  // Extract params and body expression
  let paramsNode: ASTNode | null = null;
  let bodyNode: ASTNode | null = null;

  for (const child of node.children) {
    if (!isTokenNode(child) && child.ruleName === "lambda_params") {
      paramsNode = child;
    } else if (!isTokenNode(child) && bodyNode === null && child.ruleName !== "lambda_params") {
      bodyNode = child;
    }
  }

  if (!bodyNode) {
    throw new SyntaxError("Malformed lambda expression");
  }

  // Parse parameters
  const paramNames: string[] = [];
  let defaultCount = 0;

  if (paramsNode !== null) {
    for (const paramChild of paramsNode.children) {
      if (!isTokenNode(paramChild) && paramChild.ruleName === "lambda_param") {
        const info = parseLambdaParam(paramChild);
        paramNames.push(info.name);
        if (info.defaultNode !== null) {
          compiler.compileNode(info.defaultNode);
          defaultCount++;
        }
      }
    }
  }

  // Compile body as nested CodeObject
  compiler.enterScope(paramNames);
  const bodyCode = compiler.compileNested(bodyNode);
  compiler.exitScope();

  const codeIdx = compiler.addConstant(bodyCode as unknown as string);
  compiler.emit(Op.LOAD_CONST, codeIdx);

  let flags = 0;
  if (defaultCount > 0) flags |= 0x01;
  compiler.emit(Op.MAKE_FUNCTION, flags);
}

/**
 * Parse a lambda parameter.
 *
 * Grammar: lambda_param = NAME [ EQUALS expression ] | STAR NAME | DOUBLE_STAR NAME ;
 */
function parseLambdaParam(
  node: ASTNode
): { name: string; kind: string; defaultNode: ASTNode | null } {
  const toks = tokens(node);
  const subNodes = nodes(node);

  if (hasToken(node, "*")) {
    const name = toks.find((t) => typeName(t) === "NAME")!.value;
    return { name, kind: "varargs", defaultNode: null };
  } else if (hasToken(node, "**")) {
    const name = toks.find((t) => typeName(t) === "NAME")!.value;
    return { name, kind: "kwargs", defaultNode: null };
  } else if (hasToken(node, "=")) {
    const name = toks.find((t) => typeName(t) === "NAME")!.value;
    return {
      name,
      kind: "default",
      defaultNode: subNodes.length > 0 ? subNodes[0] : null,
    };
  } else {
    return { name: toks[0].value, kind: "positional", defaultNode: null };
  }
}

// =========================================================================
// Registration -- creates a configured GenericCompiler
// =========================================================================

/**
 * Create a ``GenericCompiler`` configured with all Starlark rule handlers.
 *
 * This is the main factory function. It creates a fresh GenericCompiler
 * and registers handlers for all Starlark grammar rules that require
 * compilation logic. Pass-through rules (single child, no semantics)
 * are handled automatically by the GenericCompiler framework.
 *
 * Usage:
 *
 *     const compiler = createStarlarkCompiler();
 *     const ast = parseStarlark("x = 1 + 2\n");
 *     const code = compiler.compile(ast, Op.HALT);
 *
 * @returns A GenericCompiler ready to compile Starlark ASTs.
 */
export function createStarlarkCompiler(): GenericCompiler {
  const compiler = new GenericCompiler();

  // -- Top-level structure --
  compiler.registerRule("file", compileFile);
  compiler.registerRule("simple_stmt", compileSimpleStmt);

  // -- Simple statements --
  compiler.registerRule("assign_stmt", compileAssignStmt);
  compiler.registerRule("return_stmt", compileReturnStmt);
  compiler.registerRule("break_stmt", compileBreakStmt);
  compiler.registerRule("continue_stmt", compileContinueStmt);
  compiler.registerRule("pass_stmt", compilePassStmt);
  compiler.registerRule("load_stmt", compileLoadStmt);

  // -- Compound statements --
  compiler.registerRule("if_stmt", compileIfStmt);
  compiler.registerRule("for_stmt", compileForStmt);
  compiler.registerRule("def_stmt", compileDefStmt);
  compiler.registerRule("suite", compileSuite);

  // -- Expressions --
  compiler.registerRule("expression", compileExpression);
  compiler.registerRule("expression_list", compileExpressionList);
  compiler.registerRule("or_expr", compileOrExpr);
  compiler.registerRule("and_expr", compileAndExpr);
  compiler.registerRule("not_expr", compileNotExpr);
  compiler.registerRule("comparison", compileComparison);

  // Binary operations -- all follow the same pattern
  compiler.registerRule("arith", compileBinaryOp);
  compiler.registerRule("term", compileBinaryOp);
  compiler.registerRule("shift", compileBinaryOp);
  compiler.registerRule("bitwise_or", compileBinaryOp);
  compiler.registerRule("bitwise_xor", compileBinaryOp);
  compiler.registerRule("bitwise_and", compileBinaryOp);

  // Unary and power
  compiler.registerRule("factor", compileFactor);
  compiler.registerRule("power", compilePower);

  // Primary expressions (atom + suffixes)
  compiler.registerRule("primary", compilePrimary);

  // Atoms
  compiler.registerRule("atom", compileAtom);

  // Collection literals
  compiler.registerRule("list_expr", compileListExpr);
  compiler.registerRule("dict_expr", compileDictExpr);
  compiler.registerRule("paren_expr", compileParenExpr);

  // Dict entry (needed for dict comprehensions where dict_entry is compiled via compileNode)
  compiler.registerRule("dict_entry", compileDictEntry);

  // Lambda
  compiler.registerRule("lambda_expr", compileLambdaExpr);

  // -- Pass-through rules (handled by GenericCompiler automatically) --
  // statement, compound_stmt, small_stmt -- all have single child

  return compiler;
}

/**
 * Compile Starlark source code to bytecode in one step.
 *
 * This convenience function runs the full pipeline:
 *   1. Parse the source into an AST (using the starlark-parser)
 *   2. Create a configured Starlark compiler
 *   3. Compile the AST to a CodeObject
 *
 * @param source - The Starlark source code to compile.
 * @returns A CodeObject ready for execution by the VM.
 *
 * @example
 *     const code = compileStarlark("x = 1 + 2\n");
 *     // code.instructions contains: LOAD_CONST 1, LOAD_CONST 2, ADD, STORE_NAME x, HALT
 */
export function compileStarlark(source: string) {
  const ast = parseStarlark(source);
  const compiler = createStarlarkCompiler();
  return compiler.compile(ast, Op.HALT);
}

// Export parseStringLiteral for testing
export { parseStringLiteral };
