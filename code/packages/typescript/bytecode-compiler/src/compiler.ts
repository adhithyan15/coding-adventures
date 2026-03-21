/**
 * Bytecode Compiler — The bridge between parsing and execution.
 *
 * =================================================================
 * Chapter 4a: From Trees to Instructions
 * =================================================================
 *
 * In the previous layers, we built a lexer (Layer 2) that turns source code into
 * tokens, and a parser (Layer 3) that arranges those tokens into an Abstract
 * Syntax Tree (AST). Now we face the next question: how do we turn a *tree* into
 * something a machine can actually execute?
 *
 * The answer is **compilation** — walking the tree and emitting a flat sequence
 * of stack-machine instructions. This is exactly what real compilers do:
 *
 *     javac    : Java source  -->  JVM bytecode  (.class files)
 *     csc      : C# source    -->  CLR IL        (.dll files)
 *     cpython  : Python source -->  Python bytecode (.pyc files)
 *     Our compiler: AST       -->  CodeObject    (for our VM)
 *
 * The key insight is that a tree-structured program can always be "flattened"
 * into a sequence of stack operations. Consider the expression ``1 + 2 * 3``.
 * The AST looks like this:
 *
 *         +
 *        / \
 *       1   *
 *          / \
 *         2   3
 *
 * To evaluate this on a stack machine, we do a **post-order traversal** (visit
 * children before the parent):
 *
 *     1. Visit the left child of ``+``:  emit LOAD_CONST 1
 *     2. Visit the right child of ``+`` (which is ``*``):
 *        a. Visit left child of ``*``:   emit LOAD_CONST 2
 *        b. Visit right child of ``*``:  emit LOAD_CONST 3
 *        c. Visit ``*`` itself:          emit MUL
 *     3. Visit ``+`` itself:             emit ADD
 *
 * The result is: ``LOAD_CONST 1, LOAD_CONST 2, LOAD_CONST 3, MUL, ADD``
 *
 * This is called **Reverse Polish Notation** (RPN), and it's the natural output
 * format for a stack-machine compiler. The stack does all the bookkeeping that
 * parentheses and precedence rules handle in the source code.
 *
 * Terminology
 * -----------
 * - **Emit**: To append an instruction to the output list.
 * - **Constant pool**: A list of literal values (numbers, strings) that
 *   instructions reference by index, not by value.
 * - **Name pool**: A list of variable names, similarly referenced by index.
 * - **CodeObject**: The final compiled artifact — instructions + pools.
 */

import { tokenize } from "@coding-adventures/lexer";
import type { LexerConfig } from "@coding-adventures/lexer";
import { Parser } from "@coding-adventures/parser";
import type {
  Assignment,
  BinaryOp,
  Expression,
  Name,
  NumberLiteral,
  Program,
  Statement,
  StringLiteral,
} from "@coding-adventures/parser";
import type { CodeObject, Instruction } from "./vm-types.js";
import { OpCode } from "./vm-types.js";

// ---------------------------------------------------------------------------
// Operator-to-opcode mapping
// ---------------------------------------------------------------------------

/**
 * Maps the source-level operator symbols to their corresponding VM opcodes.
 *
 * Each arithmetic operator in the source language has a direct counterpart in the
 * VM instruction set. The compiler uses this table during expression compilation
 * to translate ``BinaryOp.op`` strings into the correct ``OpCode``.
 *
 * Why a dictionary and not a chain of if/else? Because:
 *   1. It's easier to extend — adding ``%`` just means one new entry.
 *   2. It separates data (the mapping) from logic (the compilation).
 *   3. It's faster for large operator sets (O(1) lookup vs. O(n) branches).
 */
const OPERATOR_MAP: Record<string, (typeof OpCode)[keyof typeof OpCode]> = {
  "+": OpCode.ADD,
  "-": OpCode.SUB,
  "*": OpCode.MUL,
  "/": OpCode.DIV,
};

// ---------------------------------------------------------------------------
// BytecodeCompiler
// ---------------------------------------------------------------------------

/**
 * Compiles an AST into a CodeObject for the virtual machine.
 *
 * This is the bridge between the parser (which understands language syntax)
 * and the VM (which executes instructions). The compiler's job is to
 * translate tree-structured code into a flat sequence of stack operations.
 *
 * This is analogous to:
 * - ``javac``:  compiles Java source  -> JVM bytecode (.class files)
 * - ``csc``:    compiles C# source    -> CLR IL bytecode (.dll files)
 * - Our compiler: compiles AST        -> CodeObject (for our VM)
 *
 * How it works
 * ------------
 * The compiler maintains three pieces of state as it walks the AST:
 *
 * 1. **instructions** — The growing list of bytecode instructions. Each call
 *    to ``compileExpression`` or ``compileStatement`` appends one or more
 *    instructions to this list.
 *
 * 2. **constants** — The constant pool. When the compiler encounters a literal
 *    value like ``42`` or ``"hello"``, it adds it here (if not already present)
 *    and emits a ``LOAD_CONST <index>`` instruction referencing its position.
 *
 * 3. **names** — The name pool. Variable names like ``x`` or ``total`` go here,
 *    and ``STORE_NAME <index>`` / ``LOAD_NAME <index>`` reference them.
 *
 * Example walkthrough
 * -------------------
 * Compiling ``x = 1 + 2``:
 *
 *     AST:
 *         Assignment(
 *             target=Name("x"),
 *             value=BinaryOp(NumberLiteral(1), "+", NumberLiteral(2))
 *         )
 *
 *     Step 1: compileAssignment is called.
 *     Step 2: It calls compileExpression on the BinaryOp.
 *     Step 3: compileExpression recurses:
 *         - Left:  NumberLiteral(1) -> adds 1 to constants[0], emits LOAD_CONST 0
 *         - Right: NumberLiteral(2) -> adds 2 to constants[1], emits LOAD_CONST 1
 *         - Op "+":                 -> emits ADD
 *     Step 4: Back in compileAssignment:
 *         - Adds "x" to names[0], emits STORE_NAME 0
 *
 *     Result:
 *         instructions = [LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0]
 *         constants    = [1, 2]
 *         names        = ["x"]
 */
export class BytecodeCompiler {
  /** The bytecode instructions emitted so far, in order. */
  instructions: Instruction[] = [];

  /** The constant pool — literal values referenced by LOAD_CONST. */
  constants: (number | string)[] = [];

  /** The name pool — variable names referenced by STORE_NAME / LOAD_NAME. */
  names: string[] = [];

  // -------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------

  /**
   * Compile a full program AST into a CodeObject.
   *
   * This is the main entry point. It iterates over every statement in the
   * program, compiles each one, then appends a final ``HALT`` instruction
   * to tell the VM that execution is complete.
   *
   * @param program - The root AST node, as produced by ``Parser.parse()``.
   * @returns A self-contained unit of bytecode ready for the VM to execute.
   *
   * Example:
   *
   *     import { Parser } from "@coding-adventures/parser";
   *     import { tokenize } from "@coding-adventures/lexer";
   *
   *     const tokens = tokenize("x = 42");
   *     const ast = new Parser(tokens).parse();
   *
   *     const compiler = new BytecodeCompiler();
   *     const code = compiler.compile(ast);
   *     // code.instructions = [LOAD_CONST 0, STORE_NAME 0, HALT]
   *     // code.constants = [42]
   *     // code.names = ["x"]
   */
  compile(program: Program): CodeObject {
    for (const statement of program.statements) {
      this.compileStatement(statement);
    }

    // Every program ends with HALT so the VM knows to stop.
    // Without this, the VM would try to read past the end of the
    // instruction array — just like a CPU needs a HLT instruction.
    this.instructions.push({ opcode: OpCode.HALT });

    return {
      instructions: this.instructions,
      constants: this.constants,
      names: this.names,
    };
  }

  // -------------------------------------------------------------------
  // Statement compilation
  // -------------------------------------------------------------------

  /**
   * Compile a single statement.
   *
   * There are two kinds of statements in our language:
   *
   * 1. **Assignment** (``x = expr``) — Evaluate the expression, then store
   *    the result in a named variable. The value stays bound to that name
   *    for the rest of the program.
   *
   * 2. **Expression statement** (just ``expr`` on its own) — Evaluate the
   *    expression for its side effects (there are none yet, but there will
   *    be when we add function calls and print). Since no one captures the
   *    result, we emit a ``POP`` to discard it and keep the stack clean.
   *
   * Why POP for expression statements?
   * ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
   * The stack machine's invariant is: after each statement completes, the
   * stack should be in the same state as before. An expression like ``1 + 2``
   * would leave the result ``3`` on the stack. If we didn't pop it, the
   * stack would grow by one element for every expression statement, and
   * subsequent operations would find unexpected values.
   */
  private compileStatement(stmt: Statement): void {
    if (stmt.kind === "Assignment") {
      this.compileAssignment(stmt);
    } else {
      // Expression statement — compile the expression, then throw away
      // the result. The expression's value is computed and pushed onto
      // the stack, but since nobody assigned it to a variable, we
      // discard it with POP.
      this.compileExpression(stmt);
      this.instructions.push({ opcode: OpCode.POP });
    }
  }

  /**
   * Compile a variable assignment: ``name = expression``.
   *
   * The compilation strategy is straightforward:
   * 1. Compile the right-hand side expression (pushes its value onto the stack).
   * 2. Emit ``STORE_NAME <index>`` to pop the value and bind it to the name.
   *
   * This mirrors how CPython compiles assignments: evaluate the value first,
   * then store it. The order matters — we need the value on the stack before
   * we can store it.
   *
   * Example: ``x = 42`` compiles to:
   *
   *     LOAD_CONST 0    // Push 42 onto the stack
   *     STORE_NAME 0    // Pop it and bind to "x"
   */
  private compileAssignment(node: Assignment): void {
    // First, evaluate the right-hand side. After this, the value sits
    // on top of the stack, waiting to be stored.
    this.compileExpression(node.value);

    // Now store the top-of-stack value into the named variable.
    // addName handles deduplication: if "x" was already used, we reuse
    // its index rather than adding a duplicate entry to the name pool.
    const nameIndex = this.addName(node.target.name);
    this.instructions.push({ opcode: OpCode.STORE_NAME, operand: nameIndex });
  }

  // -------------------------------------------------------------------
  // Expression compilation — the recursive heart
  // -------------------------------------------------------------------

  /**
   * Compile an expression — the recursive heart of the compiler.
   *
   * Every expression, no matter how complex, ultimately compiles down to a
   * sequence of instructions that leaves exactly one value on the stack.
   * This is the fundamental contract of expression compilation:
   *
   *     **Before**: stack has N items.
   *     **After**:  stack has N + 1 items (the expression's value on top).
   *
   * The compiler handles each expression type differently:
   *
   * - **NumberLiteral** / **StringLiteral**: Add the value to the constant
   *   pool, emit ``LOAD_CONST <index>`` to push it onto the stack.
   *
   * - **Name**: Add the variable name to the name pool, emit
   *   ``LOAD_NAME <index>`` to look up and push its current value.
   *
   * - **BinaryOp**: Recursively compile left and right operands (each pushes
   *   one value), then emit the appropriate arithmetic instruction (ADD, SUB,
   *   etc.) which pops both values and pushes the result.
   *
   * The recursion is what makes this work for arbitrarily nested expressions.
   * ``1 + 2 * 3`` has a BinaryOp at the top, whose right child is another
   * BinaryOp. The compiler just keeps recursing until it hits leaf nodes
   * (literals or names), then the instructions "unwind" in the correct order.
   *
   * @throws TypeError if ``node`` is an unrecognized expression type.
   */
  compileExpression(node: Expression): void {
    switch (node.kind) {
      case "NumberLiteral": {
        // A number literal like 42. We store the value in the constant pool
        // and emit an instruction to push it onto the stack at runtime.
        const constIndex = this.addConstant(node.value);
        this.instructions.push({
          opcode: OpCode.LOAD_CONST,
          operand: constIndex,
        });
        break;
      }

      case "StringLiteral": {
        // A string literal like "hello". Same strategy as numbers — store
        // in the constant pool, emit LOAD_CONST.
        const constIndex = this.addConstant(node.value);
        this.instructions.push({
          opcode: OpCode.LOAD_CONST,
          operand: constIndex,
        });
        break;
      }

      case "Name": {
        // A variable reference like x. We store the name string in the name
        // pool and emit LOAD_NAME, which tells the VM to look up the current
        // value of that variable and push it onto the stack.
        const nameIndex = this.addName(node.name);
        this.instructions.push({
          opcode: OpCode.LOAD_NAME,
          operand: nameIndex,
        });
        break;
      }

      case "BinaryOp": {
        // A binary operation like 1 + 2. The compilation order is critical:
        //
        //   1. Compile left operand  -> pushes left value onto stack
        //   2. Compile right operand -> pushes right value onto stack
        //   3. Emit the operator     -> pops both, pushes result
        //
        // This is a post-order traversal of the expression tree, which
        // naturally produces Reverse Polish Notation (RPN). The stack
        // handles all the intermediate storage that explicit temporary
        // variables would handle in a register machine.
        this.compileExpression(node.left);
        this.compileExpression(node.right);
        const opcode = OPERATOR_MAP[node.op];
        this.instructions.push({ opcode });
        break;
      }

      default: {
        // TypeScript's exhaustive check: if we get here, we missed a case.
        const exhaustive: never = node;
        throw new TypeError(
          `Unknown expression type: ${(exhaustive as { kind: string }).kind}. ` +
            `The compiler doesn't know how to handle this AST node.`,
        );
      }
    }
  }

  // -------------------------------------------------------------------
  // Pool management — constants and names
  // -------------------------------------------------------------------

  /**
   * Add a constant to the pool, returning its index. Deduplicates.
   *
   * The constant pool is a list of literal values that appear in the source
   * code. Instead of embedding the value directly in each instruction, we
   * store it once in the pool and reference it by index. This has two
   * benefits:
   *
   * 1. **Space efficiency**: If the program uses ``42`` in ten places, we
   *    store it once and emit ``LOAD_CONST 0`` ten times.
   *
   * 2. **Simplicity**: Instructions have a uniform format (opcode + integer
   *    operand), regardless of whether the constant is a small number or a
   *    long string.
   *
   * Deduplication means we check if the value is already in the pool before
   * adding it. If ``42`` is already at index 0, we just return 0.
   */
  private addConstant(value: number | string): number {
    const existing = this.constants.indexOf(value);
    if (existing !== -1) {
      return existing;
    }
    this.constants.push(value);
    return this.constants.length - 1;
  }

  /**
   * Add a variable name to the name pool, returning its index. Deduplicates.
   *
   * The name pool works exactly like the constant pool, but for variable
   * names instead of literal values. When the compiler sees ``x = 42``,
   * it adds ``"x"`` to the name pool and emits ``STORE_NAME <index>``.
   * Later, when it sees ``x`` used in an expression, it reuses the same
   * index and emits ``LOAD_NAME <index>``.
   *
   * Deduplication is especially important for names because the same variable
   * is typically used many times.
   */
  private addName(name: string): number {
    const existing = this.names.indexOf(name);
    if (existing !== -1) {
      return existing;
    }
    this.names.push(name);
    return this.names.length - 1;
  }
}

// ---------------------------------------------------------------------------
// Convenience function for end-to-end compilation
// ---------------------------------------------------------------------------

/**
 * Convenience: source code string -> CodeObject in one call.
 *
 * This chains the entire front-end pipeline:
 *
 *     Source code -> Lexer -> Tokens -> Parser -> AST -> Compiler -> CodeObject
 *
 * It's the simplest way to go from human-readable code to VM-executable
 * bytecode. Under the hood, it creates a fresh tokenizer, Parser, and
 * BytecodeCompiler for each call.
 *
 * @param source - The source code to compile (e.g., ``"x = 1 + 2"``).
 * @param keywords - Optional language-specific keywords for the lexer.
 * @returns A compiled bytecode object ready for the VM to execute.
 *
 * Example:
 *
 *     import { compileSource } from "@coding-adventures/bytecode-compiler";
 *
 *     const code = compileSource("x = 1 + 2");
 *     // Execute with a VirtualMachine...
 *
 * Note: For production use, you'd typically keep the lexer/parser/compiler
 * separate so you can inspect or transform the intermediate representations.
 * This function is for quick experiments and tests.
 */
export function compileSource(
  source: string,
  keywords?: string[],
): CodeObject {
  const config = keywords ? { keywords } : undefined;
  const tokens = tokenize(source, config);
  const ast = new Parser(tokens).parse();
  return new BytecodeCompiler().compile(ast);
}
