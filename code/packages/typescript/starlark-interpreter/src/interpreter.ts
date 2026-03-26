/**
 * Starlark Interpreter — The complete execution pipeline.
 *
 * ==========================================================================
 * Chapter 1: What Is an Interpreter?
 * ==========================================================================
 *
 * An interpreter takes source code and executes it. Unlike a compiler that
 * produces an executable file, an interpreter runs the program directly. Our
 * Starlark interpreter uses a **multi-stage pipeline** internally:
 *
 *     source code -> tokens -> AST -> bytecode -> execution
 *
 * Each stage is handled by a separate package:
 *
 *   1. **Lexer** (starlark-lexer): Breaks source text into tokens.
 *      ``"x = 1 + 2"`` -> ``[NAME("x"), EQUALS, INT("1"), PLUS, INT("2")]``
 *
 *   2. **Parser** (starlark-parser): Groups tokens into an Abstract Syntax
 *      Tree (AST). ``[NAME, EQUALS, INT, PLUS, INT]`` -> ``AssignStmt(x, Add(1, 2))``
 *
 *   3. **Compiler** (starlark-ast-to-bytecode-compiler): Translates the AST
 *      into bytecode instructions. ``AssignStmt(x, Add(1, 2))`` ->
 *      ``[LOAD_CONST 1, LOAD_CONST 2, ADD, STORE_NAME x]``
 *
 *   4. **VM** (starlark-vm): Executes bytecode on a virtual stack machine.
 *      Runs the instructions and produces the final result.
 *
 * This package chains them together and adds the critical ``load()`` function.
 *
 * ==========================================================================
 * Chapter 2: The load() Function
 * ==========================================================================
 *
 * ``load()`` is what makes BUILD files work. It is how a BUILD file imports
 * rule definitions from a shared library:
 *
 *     load("//rules/python.star", "py_library")
 *
 *     py_library(
 *         name = "mylib",
 *         deps = ["//other:lib"],
 *     )
 *
 * When the VM encounters a ``load()`` call:
 *
 *   1. **Resolve** the path -- ``//rules/python.star`` -> actual file contents
 *   2. **Execute** the file through the same interpreter pipeline
 *   3. **Extract** the requested symbols from the result
 *   4. **Inject** them into the current scope
 *
 * This means ``load()`` is **recursive** -- the loaded file is itself a
 * Starlark program that gets interpreted. Loaded files are cached so each
 * file is evaluated at most once, matching Bazel's semantics.
 *
 * ==========================================================================
 * Chapter 3: File Resolvers
 * ==========================================================================
 *
 * The interpreter does not know where files live on disk. Instead, it accepts
 * a **file resolver** -- a callable that maps label paths to file contents:
 *
 *     function myResolver(label: string): string {
 *       const path = label.replace("//", "/path/to/repo/");
 *       return fs.readFileSync(path, "utf-8");
 *     }
 *
 * The build tool provides a resolver that knows the repository layout.
 * For testing, you can provide a dict-based resolver:
 *
 *     const resolver = { "//rules/test.star": "def foo(): return 42" };
 *     const result = interpret(source, { fileResolver: resolver });
 *
 * ==========================================================================
 * Chapter 4: Architecture — Why Separate Packages?
 * ==========================================================================
 *
 * You might wonder: why not just put everything in one package? The answer
 * is **separation of concerns** and **testability**:
 *
 * - The **lexer** can be tested in isolation with just strings.
 * - The **parser** can be tested with token arrays (no VM needed).
 * - The **compiler** can be tested by inspecting emitted bytecode.
 * - The **VM** can be tested with hand-crafted bytecode.
 * - The **interpreter** tests the integration of all four.
 *
 * Each layer has a clean contract (tokens, ASTs, bytecode, results) making
 * bugs easy to isolate. This is the same architecture used by CPython,
 * the JVM, and V8 (Node.js).
 *
 * @module
 */

import { readFileSync } from "fs";

import {
  GenericVM,
  type CodeObject,
  type Instruction,
  type VMTrace,
} from "@coding-adventures/virtual-machine";

// =========================================================================
// Starlark Opcodes — Mirroring the Python starlark-ast-to-bytecode-compiler
// =========================================================================

/**
 * Starlark-specific opcodes.
 *
 * These numeric values MUST match the Python Op enum in
 * starlark_ast_to_bytecode_compiler/opcodes.py. When the TypeScript
 * starlark-ast-to-bytecode-compiler package is created, these will be
 * imported from there instead. For now, we define the subset we need
 * (LOAD_MODULE for the load() override, plus basics for testing).
 *
 * The full opcode set is organized by category using the high nibble:
 *
 *     0x0_ = Stack operations      (LOAD_CONST, POP, DUP, etc.)
 *     0x1_ = Variable operations   (STORE_NAME, LOAD_NAME, etc.)
 *     0x2_ = Arithmetic            (ADD, SUB, MUL, DIV, etc.)
 *     0x3_ = Comparison            (CMP_EQ, CMP_LT, etc.)
 *     0x4_ = Control flow          (JUMP, JUMP_IF_FALSE, etc.)
 *     0x5_ = Functions             (MAKE_FUNCTION, CALL_FUNCTION, RETURN)
 *     0x6_ = Collections           (BUILD_LIST, BUILD_DICT, etc.)
 *     0x7_ = Subscript & attribute (LOAD_SUBSCRIPT, LOAD_ATTR, etc.)
 *     0x8_ = Iteration             (GET_ITER, FOR_ITER, etc.)
 *     0x9_ = Module                (LOAD_MODULE, IMPORT_FROM)
 *     0xA_ = I/O                   (PRINT)
 *     0xF_ = VM control            (HALT)
 */
export const Op = {
  // Stack operations
  LOAD_CONST: 0x01,
  POP: 0x02,
  DUP: 0x03,
  LOAD_NONE: 0x04,
  LOAD_TRUE: 0x05,
  LOAD_FALSE: 0x06,

  // Variable operations
  STORE_NAME: 0x10,
  LOAD_NAME: 0x11,
  STORE_LOCAL: 0x12,
  LOAD_LOCAL: 0x13,

  // Arithmetic
  ADD: 0x20,
  SUB: 0x21,
  MUL: 0x22,
  DIV: 0x23,
  FLOOR_DIV: 0x24,
  MOD: 0x25,
  NEGATE: 0x27,

  // Comparison
  CMP_EQ: 0x30,
  CMP_NE: 0x31,
  CMP_LT: 0x32,
  CMP_GT: 0x33,
  CMP_LE: 0x34,
  CMP_GE: 0x35,
  NOT: 0x38,

  // Control flow
  JUMP: 0x40,
  JUMP_IF_FALSE: 0x41,
  JUMP_IF_TRUE: 0x42,

  // Functions
  MAKE_FUNCTION: 0x50,
  CALL_FUNCTION: 0x51,
  RETURN: 0x53,

  // Collections
  BUILD_LIST: 0x60,
  BUILD_DICT: 0x61,

  // Subscript & attribute
  LOAD_SUBSCRIPT: 0x70,
  STORE_SUBSCRIPT: 0x71,

  // Module operations (critical for load())
  LOAD_MODULE: 0x90,
  IMPORT_FROM: 0x91,

  // I/O
  PRINT: 0xa0,

  // VM control
  HALT: 0xff,
} as const;

// =========================================================================
// File Resolver Types
// =========================================================================

/**
 * A function that resolves a file label (like "//rules/python.star") to
 * its source code contents.
 *
 * In a real build system, this would read from disk using the repository
 * layout. In tests, it might look up a key in a Map or object.
 *
 * The function should throw an Error if the file cannot be found.
 */
export type FileResolverFn = (label: string) => string;

/**
 * A file resolver can be either:
 *
 * 1. A **function** — called with the label, returns source code.
 *    This is the production pattern, used by the build tool.
 *
 * 2. A **dictionary** (Record) — maps labels to source code strings.
 *    This is the testing pattern, convenient for unit tests.
 *
 * 3. **null/undefined** — no resolver configured. Any load() call
 *    will throw an error.
 *
 * This union type mirrors Python's ``Callable | dict | None`` pattern
 * from the reference implementation.
 */
export type FileResolver = FileResolverFn | Record<string, string> | null;

/**
 * Create a FileResolverFn from a dictionary of labels to source code.
 *
 * This is a convenience for testing. Instead of writing a function,
 * just pass a plain object:
 *
 *     const resolver = dictResolver({
 *       "//rules/math.star": "def double(n): return n * 2",
 *     });
 *
 * @param files - An object mapping file labels to their source code.
 * @returns A FileResolverFn that looks up labels in the dictionary.
 */
export function dictResolver(files: Record<string, string>): FileResolverFn {
  return (label: string): string => {
    if (label in files) {
      return files[label];
    }
    throw new FileNotFoundError(
      `load(): file not found in resolver: ${label}`,
    );
  };
}

// =========================================================================
// Error Classes
// =========================================================================

/**
 * Thrown when a load() call cannot resolve a file label.
 *
 * This mirrors Python's FileNotFoundError. It occurs when:
 *
 * - No file resolver is configured (fileResolver is null).
 * - The file resolver is a dict and the label is not a key.
 * - The file resolver function throws (e.g., file not on disk).
 */
export class FileNotFoundError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "FileNotFoundError";
  }
}

// =========================================================================
// Internal: Resolve a file label to source code
// =========================================================================

/**
 * Resolve a label to file contents using the configured resolver.
 *
 * This function normalizes the three resolver types (function, dict, null)
 * into a single code path. It is the internal workhorse that the
 * LOAD_MODULE handler calls.
 *
 * @param resolver - The file resolver (function, dict, or null).
 * @param label    - The file label to resolve (e.g., "//rules/python.star").
 * @returns The source code of the resolved file.
 * @throws {FileNotFoundError} If the label cannot be resolved.
 */
export function resolveFile(
  resolver: FileResolver,
  label: string,
): string {
  // Case 1: No resolver configured.
  if (resolver == null) {
    throw new FileNotFoundError(
      `load() called but no fileResolver configured. Cannot resolve: ${label}`,
    );
  }

  // Case 2: Dict resolver — look up the label as a key.
  if (typeof resolver === "object") {
    if (label in resolver) {
      return resolver[label];
    }
    throw new FileNotFoundError(
      `load(): file not found in resolver: ${label}`,
    );
  }

  // Case 3: Function resolver — delegate to the function.
  return resolver(label);
}

// =========================================================================
// Starlark Result
// =========================================================================

/**
 * The result of executing a Starlark program.
 *
 * Contains everything you need to inspect after execution:
 *
 * - **variables** — the final state of all named variables.
 *   After running ``x = 42``, variables will contain ``{ x: 42 }``.
 *
 * - **output** — captured print output, one entry per print() call.
 *   After running ``print("hello")``, output will be ``["hello"]``.
 *
 * - **traces** — step-by-step execution trace (for debugging).
 *   Each trace records one VM instruction: what was on the stack before,
 *   what instruction ran, and what the stack looked like after.
 */
export interface StarlarkResult {
  /** Final variable state after execution. */
  readonly variables: Record<string, unknown>;

  /** Captured print output, one entry per print() call. */
  readonly output: string[];

  /** Step-by-step execution trace. */
  readonly traces: VMTrace[];
}

// =========================================================================
// Compile Function Type
// =========================================================================

/**
 * A function that compiles Starlark source code to bytecode.
 *
 * When the starlark-ast-to-bytecode-compiler TypeScript package is
 * created, this will be imported from there. For now, users must provide
 * a compile function, or use the built-in mini-compiler for testing.
 *
 * The compile function chains together:
 *   1. Lexing (source -> tokens)
 *   2. Parsing (tokens -> AST)
 *   3. Compilation (AST -> CodeObject)
 *
 * @param source - Starlark source code (should end with newline).
 * @returns A CodeObject ready for VM execution.
 */
export type CompileFn = (source: string) => CodeObject;

/**
 * A function that creates a configured Starlark VM.
 *
 * When the starlark-vm TypeScript package is created, this will be
 * imported from there. For now, users must provide a VM factory function,
 * or use the built-in mini-VM for testing.
 *
 * @param options - Configuration options for the VM.
 * @returns A GenericVM configured with Starlark opcode handlers and builtins.
 */
export type CreateVMFn = (options?: { maxRecursionDepth?: number }) => GenericVM;

// =========================================================================
// Mini Starlark VM — For testing without the full starlark-vm package
// =========================================================================

/**
 * Register basic Starlark opcode handlers on a GenericVM.
 *
 * This is a **minimal** set of handlers — just enough to test the
 * interpreter's load() mechanism and basic execution. When the full
 * starlark-vm TypeScript package is available, use ``createStarlarkVM``
 * from that package instead.
 *
 * The handlers implemented here cover:
 *
 * - **LOAD_CONST** — Push a constant from the pool.
 * - **POP** — Discard top of stack.
 * - **DUP** — Duplicate top of stack.
 * - **STORE_NAME** — Pop and store in a named variable.
 * - **LOAD_NAME** — Push a named variable's value.
 * - **ADD/SUB/MUL/DIV** — Basic arithmetic.
 * - **CMP_EQ/CMP_LT/CMP_GT** — Basic comparisons.
 * - **JUMP/JUMP_IF_FALSE/JUMP_IF_TRUE** — Control flow.
 * - **PRINT** — Pop and capture to output.
 * - **HALT** — Stop execution.
 * - **LOAD_MODULE** — Stub (the interpreter overrides this).
 * - **IMPORT_FROM** — Extract symbol from module dict.
 * - **LOAD_NONE/LOAD_TRUE/LOAD_FALSE** — Push boolean/null literals.
 * - **MAKE_FUNCTION/CALL_FUNCTION/RETURN** — Function support.
 * - **BUILD_LIST/BUILD_DICT** — Collection construction.
 * - **NEGATE/NOT** — Unary operations.
 * - **FLOOR_DIV/MOD** — Additional arithmetic.
 * - **CMP_NE/CMP_LE/CMP_GE** — Additional comparisons.
 * - **STORE_LOCAL/LOAD_LOCAL** — Local variable slots.
 * - **LOAD_SUBSCRIPT/STORE_SUBSCRIPT** — Indexing operations.
 *
 * Each handler follows the same pattern:
 *   1. Read operands from instruction and/or stack.
 *   2. Perform the operation.
 *   3. Push results (if any).
 *   4. Advance the program counter.
 *   5. Return a description string for the trace.
 */
export function registerMiniStarlarkHandlers(vm: GenericVM): void {
  // -- LOAD_CONST: Push a constant from the pool onto the stack ----------
  vm.registerOpcode(
    Op.LOAD_CONST,
    (vm, instr, code) => {
      const index = instr.operand as number;
      const value = code.constants[index];
      vm.push(value);
      vm.advancePc();
      return `Loaded constant ${String(value)}`;
    },
  );

  // -- POP: Discard the top value on the stack ---------------------------
  vm.registerOpcode(Op.POP, (vm) => {
    vm.pop();
    vm.advancePc();
    return "Popped top of stack";
  });

  // -- DUP: Duplicate the top value on the stack -------------------------
  vm.registerOpcode(Op.DUP, (vm) => {
    const value = vm.peek();
    vm.push(value);
    vm.advancePc();
    return `Duplicated ${String(value)}`;
  });

  // -- LOAD_NONE: Push null (Starlark None) onto the stack ---------------
  vm.registerOpcode(Op.LOAD_NONE, (vm) => {
    vm.push(null);
    vm.advancePc();
    return "Loaded None";
  });

  // -- LOAD_TRUE: Push true onto the stack --------------------------------
  vm.registerOpcode(Op.LOAD_TRUE, (vm) => {
    vm.push(1);  // In our VM, true is represented as 1
    vm.advancePc();
    return "Loaded True";
  });

  // -- LOAD_FALSE: Push false onto the stack ------------------------------
  vm.registerOpcode(Op.LOAD_FALSE, (vm) => {
    vm.push(0);  // In our VM, false is represented as 0
    vm.advancePc();
    return "Loaded False";
  });

  // -- STORE_NAME: Pop value and store in a named variable ----------------
  vm.registerOpcode(
    Op.STORE_NAME,
    (vm, instr, code) => {
      const nameIndex = instr.operand as number;
      const name = code.names[nameIndex];
      const value = vm.pop();
      vm.variables[name] = value;
      vm.advancePc();
      return `Stored ${String(value)} in ${name}`;
    },
  );

  // -- LOAD_NAME: Push the value of a named variable ---------------------
  vm.registerOpcode(
    Op.LOAD_NAME,
    (vm, instr, code) => {
      const nameIndex = instr.operand as number;
      const name = code.names[nameIndex];
      const value = vm.variables[name];
      if (value === undefined) {
        // Check builtins before throwing
        const builtin = vm.getBuiltin(name);
        if (builtin) {
          vm.push(name);  // Push function name as reference
          vm.advancePc();
          return `Loaded builtin ${name}`;
        }
        throw new Error(`Undefined variable: ${name}`);
      }
      vm.push(value);
      vm.advancePc();
      return `Loaded ${name} = ${String(value)}`;
    },
  );

  // -- STORE_LOCAL: Pop and store in a local variable slot ----------------
  vm.registerOpcode(
    Op.STORE_LOCAL,
    (vm, instr) => {
      const index = instr.operand as number;
      const value = vm.pop();
      while (vm.locals.length <= index) vm.locals.push(null);
      vm.locals[index] = value;
      vm.advancePc();
      return `Stored local[${index}] = ${String(value)}`;
    },
  );

  // -- LOAD_LOCAL: Push a local variable slot's value ---------------------
  vm.registerOpcode(
    Op.LOAD_LOCAL,
    (vm, instr) => {
      const index = instr.operand as number;
      const value = vm.locals[index];
      vm.push(value);
      vm.advancePc();
      return `Loaded local[${index}] = ${String(value)}`;
    },
  );

  // -- ADD: Pop two values, push their sum --------------------------------
  vm.registerOpcode(Op.ADD, (vm) => {
    const b = vm.pop();
    const a = vm.pop();
    if (typeof a === "string" || typeof b === "string") {
      vm.push(String(a) + String(b));
    } else {
      vm.push((a as number) + (b as number));
    }
    vm.advancePc();
    return `Added ${String(a)} + ${String(b)}`;
  });

  // -- SUB: Pop two values, push their difference -------------------------
  vm.registerOpcode(Op.SUB, (vm) => {
    const b = vm.pop() as number;
    const a = vm.pop() as number;
    vm.push(a - b);
    vm.advancePc();
    return `Subtracted ${a} - ${b}`;
  });

  // -- MUL: Pop two values, push their product ----------------------------
  vm.registerOpcode(Op.MUL, (vm) => {
    const b = vm.pop();
    const a = vm.pop();
    if (typeof a === "string" && typeof b === "number") {
      vm.push(a.repeat(b));
    } else if (typeof a === "number" && typeof b === "string") {
      vm.push(b.repeat(a));
    } else {
      vm.push((a as number) * (b as number));
    }
    vm.advancePc();
    return `Multiplied ${String(a)} * ${String(b)}`;
  });

  // -- DIV: Pop two values, push their quotient (float division) ----------
  vm.registerOpcode(Op.DIV, (vm) => {
    const b = vm.pop() as number;
    const a = vm.pop() as number;
    if (b === 0) throw new Error("Division by zero");
    vm.push(a / b);
    vm.advancePc();
    return `Divided ${a} / ${b}`;
  });

  // -- FLOOR_DIV: Pop two values, push integer division -------------------
  vm.registerOpcode(Op.FLOOR_DIV, (vm) => {
    const b = vm.pop() as number;
    const a = vm.pop() as number;
    if (b === 0) throw new Error("Division by zero");
    vm.push(Math.floor(a / b));
    vm.advancePc();
    return `Floor divided ${a} // ${b}`;
  });

  // -- MOD: Pop two values, push remainder --------------------------------
  vm.registerOpcode(Op.MOD, (vm) => {
    const b = vm.pop() as number;
    const a = vm.pop() as number;
    if (b === 0) throw new Error("Division by zero");
    vm.push(((a % b) + b) % b);  // Python-style modulo
    vm.advancePc();
    return `Modulo ${a} % ${b}`;
  });

  // -- NEGATE: Pop one value, push its negation ---------------------------
  vm.registerOpcode(Op.NEGATE, (vm) => {
    const a = vm.pop() as number;
    vm.push(-a);
    vm.advancePc();
    return `Negated ${a}`;
  });

  // -- NOT: Pop one value, push logical not -------------------------------
  vm.registerOpcode(Op.NOT, (vm) => {
    const a = vm.pop();
    const truthy = a !== 0 && a !== null && a !== "" && a !== false;
    vm.push(truthy ? 0 : 1);
    vm.advancePc();
    return `Not ${String(a)}`;
  });

  // -- CMP_EQ: Pop two values, push 1 if equal, 0 otherwise ---------------
  vm.registerOpcode(Op.CMP_EQ, (vm) => {
    const b = vm.pop();
    const a = vm.pop();
    vm.push(a === b ? 1 : 0);
    vm.advancePc();
    return `Compared ${String(a)} == ${String(b)}`;
  });

  // -- CMP_NE: Pop two values, push 1 if not equal, 0 otherwise -----------
  vm.registerOpcode(Op.CMP_NE, (vm) => {
    const b = vm.pop();
    const a = vm.pop();
    vm.push(a !== b ? 1 : 0);
    vm.advancePc();
    return `Compared ${String(a)} != ${String(b)}`;
  });

  // -- CMP_LT: Pop two values, push 1 if a < b, 0 otherwise ---------------
  vm.registerOpcode(Op.CMP_LT, (vm) => {
    const b = vm.pop() as number;
    const a = vm.pop() as number;
    vm.push(a < b ? 1 : 0);
    vm.advancePc();
    return `Compared ${a} < ${b}`;
  });

  // -- CMP_GT: Pop two values, push 1 if a > b, 0 otherwise ---------------
  vm.registerOpcode(Op.CMP_GT, (vm) => {
    const b = vm.pop() as number;
    const a = vm.pop() as number;
    vm.push(a > b ? 1 : 0);
    vm.advancePc();
    return `Compared ${a} > ${b}`;
  });

  // -- CMP_LE: Pop two values, push 1 if a <= b, 0 otherwise --------------
  vm.registerOpcode(Op.CMP_LE, (vm) => {
    const b = vm.pop() as number;
    const a = vm.pop() as number;
    vm.push(a <= b ? 1 : 0);
    vm.advancePc();
    return `Compared ${a} <= ${b}`;
  });

  // -- CMP_GE: Pop two values, push 1 if a >= b, 0 otherwise --------------
  vm.registerOpcode(Op.CMP_GE, (vm) => {
    const b = vm.pop() as number;
    const a = vm.pop() as number;
    vm.push(a >= b ? 1 : 0);
    vm.advancePc();
    return `Compared ${a} >= ${b}`;
  });

  // -- JUMP: Unconditional jump to target ---------------------------------
  vm.registerOpcode(
    Op.JUMP,
    (vm, instr) => {
      const target = instr.operand as number;
      vm.jumpTo(target);
      return `Jumped to ${target}`;
    },
  );

  // -- JUMP_IF_FALSE: Pop value, jump if falsy ----------------------------
  vm.registerOpcode(
    Op.JUMP_IF_FALSE,
    (vm, instr) => {
      const target = instr.operand as number;
      const value = vm.pop();
      const falsy = value === 0 || value === null || value === "" || value === false;
      if (falsy) {
        vm.jumpTo(target);
        return `Jumped to ${target} (value was falsy)`;
      }
      vm.advancePc();
      return `Did not jump (value was truthy)`;
    },
  );

  // -- JUMP_IF_TRUE: Pop value, jump if truthy ----------------------------
  vm.registerOpcode(
    Op.JUMP_IF_TRUE,
    (vm, instr) => {
      const target = instr.operand as number;
      const value = vm.pop();
      const truthy = value !== 0 && value !== null && value !== "" && value !== false;
      if (truthy) {
        vm.jumpTo(target);
        return `Jumped to ${target} (value was truthy)`;
      }
      vm.advancePc();
      return `Did not jump (value was falsy)`;
    },
  );

  // -- MAKE_FUNCTION: Create a function object ----------------------------
  // In our mini VM, a function is represented as a CodeObject on the stack.
  // The MAKE_FUNCTION opcode pops the code object and optionally default
  // argument values, then pushes a "function wrapper" record.
  vm.registerOpcode(
    Op.MAKE_FUNCTION,
    (vm, instr, code) => {
      const flags = instr.operand as number;
      const codeObj = vm.pop() as CodeObject;
      let defaults: unknown[] = [];
      if (flags > 0) {
        defaults = vm.pop() as unknown[];
      }
      // Store as a callable record that CALL_FUNCTION can recognize
      const func = {
        __type__: "function" as const,
        code: codeObj,
        defaults,
      };
      vm.push(func as unknown as CodeObject);
      vm.advancePc();
      return "Made function";
    },
  );

  // -- CALL_FUNCTION: Call a function with N positional args ---------------
  vm.registerOpcode(
    Op.CALL_FUNCTION,
    (vm, instr) => {
      const argCount = instr.operand as number;
      const args: unknown[] = [];
      for (let i = 0; i < argCount; i++) {
        args.unshift(vm.pop());
      }
      const func = vm.pop() as unknown;

      // Check if it's a builtin function reference (string name)
      if (typeof func === "string") {
        const builtin = vm.getBuiltin(func);
        if (builtin) {
          const result = builtin.implementation(...(args as never[]));
          vm.push(result);
          vm.advancePc();
          return `Called builtin ${func}`;
        }
        throw new Error(`Unknown function: ${func}`);
      }

      // Check if it's a function record
      const funcObj = func as { __type__?: string; code?: CodeObject; defaults?: unknown[] };
      if (funcObj && funcObj.__type__ === "function" && funcObj.code) {
        // Save current state on call stack
        vm.pushFrame({
          returnPc: vm.pc + 1,
          savedVariables: { ...vm.variables },
          savedLocals: [...vm.locals],
        });

        // Set up locals from arguments and defaults
        const allArgs = [...args];
        const funcCode = funcObj.code;

        // Set up local slots from args
        vm.locals = [];
        for (let i = 0; i < allArgs.length; i++) {
          vm.locals.push(allArgs[i] as never);
        }

        // Execute the function's code object inline
        // We do this by running the function code on this VM
        const savedPc = vm.pc;
        vm.pc = 0;
        vm.halted = false;

        const subTraces = vm.execute(funcCode);

        // The RETURN handler should have restored state.
        // If we get here without a RETURN, push null.
        if (vm.callStack.length > 0) {
          const frame = vm.popFrame() as {
            returnPc: number;
            savedVariables: Record<string, unknown>;
            savedLocals: unknown[];
          };
          vm.pc = frame.returnPc;
          vm.variables = frame.savedVariables as Record<string, never>;
          vm.locals = frame.savedLocals as never[];
          vm.halted = false;
          vm.push(null);
        }

        return `Called function with ${argCount} args`;
      }

      throw new Error(`Cannot call non-function: ${String(func)}`);
    },
  );

  // -- RETURN: Return from a function -------------------------------------
  vm.registerOpcode(Op.RETURN, (vm) => {
    const returnValue = vm.pop();
    if (vm.callStack.length > 0) {
      const frame = vm.popFrame() as {
        returnPc: number;
        savedVariables: Record<string, unknown>;
        savedLocals: unknown[];
      };
      vm.pc = frame.returnPc;
      vm.variables = frame.savedVariables as Record<string, never>;
      vm.locals = frame.savedLocals as never[];
      vm.push(returnValue);
      vm.halted = false;
    } else {
      // Top-level return — halt execution
      vm.push(returnValue);
      vm.halted = true;
    }
    return `Returned ${String(returnValue)}`;
  });

  // -- BUILD_LIST: Create a list from N stack items -----------------------
  vm.registerOpcode(
    Op.BUILD_LIST,
    (vm, instr) => {
      const count = instr.operand as number;
      const items: unknown[] = [];
      for (let i = 0; i < count; i++) {
        items.unshift(vm.pop());
      }
      vm.push(items as unknown as never);
      vm.advancePc();
      return `Built list with ${count} items`;
    },
  );

  // -- BUILD_DICT: Create a dict from N key-value pairs -------------------
  vm.registerOpcode(
    Op.BUILD_DICT,
    (vm, instr) => {
      const pairCount = instr.operand as number;
      const dict: Record<string, unknown> = {};
      const pairs: [unknown, unknown][] = [];
      for (let i = 0; i < pairCount; i++) {
        const value = vm.pop();
        const key = vm.pop();
        pairs.unshift([key, value]);
      }
      for (const [key, value] of pairs) {
        dict[String(key)] = value;
      }
      vm.push(dict as unknown as never);
      vm.advancePc();
      return `Built dict with ${pairCount} pairs`;
    },
  );

  // -- LOAD_SUBSCRIPT: obj[key] -------------------------------------------
  vm.registerOpcode(Op.LOAD_SUBSCRIPT, (vm) => {
    const key = vm.pop();
    const obj = vm.pop() as unknown;
    if (Array.isArray(obj)) {
      vm.push((obj as unknown[])[key as number] as never);
    } else if (typeof obj === "object" && obj !== null) {
      vm.push((obj as Record<string, unknown>)[String(key)] as never);
    } else {
      throw new Error(`Cannot subscript ${typeof obj}`);
    }
    vm.advancePc();
    return `Loaded subscript [${String(key)}]`;
  });

  // -- STORE_SUBSCRIPT: obj[key] = value ----------------------------------
  vm.registerOpcode(Op.STORE_SUBSCRIPT, (vm) => {
    const value = vm.pop();
    const key = vm.pop();
    const obj = vm.pop() as unknown;
    if (Array.isArray(obj)) {
      (obj as unknown[])[key as number] = value;
    } else if (typeof obj === "object" && obj !== null) {
      (obj as Record<string, unknown>)[String(key)] = value;
    } else {
      throw new Error(`Cannot subscript ${typeof obj}`);
    }
    vm.advancePc();
    return `Stored subscript [${String(key)}]`;
  });

  // -- PRINT: Pop the top value and capture it to output -------------------
  vm.registerOpcode(Op.PRINT, (vm) => {
    const value = vm.pop();
    vm.output.push(String(value));
    vm.advancePc();
    return `Printed ${String(value)}`;
  });

  // -- HALT: Stop execution -----------------------------------------------
  vm.registerOpcode(Op.HALT, (vm) => {
    vm.halted = true;
    return "Halted";
  });

  // -- LOAD_MODULE: Stub (overridden by the interpreter) ------------------
  // This default handler simply pushes an empty dict. The interpreter's
  // _registerLoadHandlers method overrides this with a version that
  // actually resolves and executes files.
  vm.registerOpcode(
    Op.LOAD_MODULE,
    (vm, instr, code) => {
      const index = instr.operand as number;
      const label = code.names[index];
      vm.push({} as unknown as never);
      vm.advancePc();
      return `Loaded module stub for ${label}`;
    },
  );

  // -- IMPORT_FROM: Extract a symbol from a module dict --------------------
  vm.registerOpcode(
    Op.IMPORT_FROM,
    (vm, instr, code) => {
      const nameIndex = instr.operand as number;
      const name = code.names[nameIndex];
      const moduleDict = vm.peek() as unknown as Record<string, unknown>;
      if (typeof moduleDict !== "object" || moduleDict === null) {
        throw new Error(`IMPORT_FROM: expected module dict, got ${typeof moduleDict}`);
      }
      const value = moduleDict[name];
      if (value === undefined) {
        throw new Error(`IMPORT_FROM: symbol '${name}' not found in module`);
      }
      vm.push(value as never);
      vm.advancePc();
      return `Imported ${name} from module`;
    },
  );

  // -- Register print builtin so CALL_FUNCTION can find it ----------------
  vm.registerBuiltin("print", (...args: unknown[]) => {
    const output = args.map(String).join(" ");
    vm.output.push(output);
    return null;
  });

  // -- Register len builtin -----------------------------------------------
  vm.registerBuiltin("len", (...args: unknown[]) => {
    const value = args[0];
    if (typeof value === "string") return value.length;
    if (Array.isArray(value)) return value.length;
    if (typeof value === "object" && value !== null) {
      return Object.keys(value).length;
    }
    throw new Error(`len() requires a string, list, or dict`);
  });

  // -- Register str builtin -----------------------------------------------
  vm.registerBuiltin("str", (...args: unknown[]) => {
    return String(args[0]);
  });

  // -- Register int builtin -----------------------------------------------
  vm.registerBuiltin("int", (...args: unknown[]) => {
    return Number(args[0]);
  });

  // -- Register type builtin -----------------------------------------------
  vm.registerBuiltin("type", (...args: unknown[]) => {
    const value = args[0];
    if (value === null) return "NoneType";
    if (typeof value === "number") return "int";
    if (typeof value === "string") return "string";
    if (Array.isArray(value)) return "list";
    if (typeof value === "object") return "dict";
    return typeof value;
  });

  // -- Register bool builtin -----------------------------------------------
  vm.registerBuiltin("bool", (...args: unknown[]) => {
    const value = args[0];
    if (value === 0 || value === null || value === "" || value === false) return 0;
    if (Array.isArray(value) && value.length === 0) return 0;
    return 1;
  });

  // -- Register range builtin -----------------------------------------------
  vm.registerBuiltin("range", (...args: unknown[]) => {
    let start = 0;
    let stop: number;
    let step = 1;
    if (args.length === 1) {
      stop = args[0] as number;
    } else if (args.length === 2) {
      start = args[0] as number;
      stop = args[1] as number;
    } else {
      start = args[0] as number;
      stop = args[1] as number;
      step = args[2] as number;
    }
    const result: number[] = [];
    if (step > 0) {
      for (let i = start; i < stop; i += step) result.push(i);
    } else if (step < 0) {
      for (let i = start; i > stop; i += step) result.push(i);
    }
    return result;
  });
}

/**
 * Create a GenericVM configured with the mini Starlark handlers.
 *
 * This is the default VM factory used by {@link StarlarkInterpreter}
 * when no custom createVM function is provided.
 *
 * @param options - Optional configuration.
 * @returns A GenericVM ready to execute basic Starlark bytecode.
 */
export function createMiniStarlarkVM(
  options?: { maxRecursionDepth?: number },
): GenericVM {
  const vm = new GenericVM();
  registerMiniStarlarkHandlers(vm);
  if (options?.maxRecursionDepth != null) {
    vm.setMaxRecursionDepth(options.maxRecursionDepth);
  }
  return vm;
}

// =========================================================================
// The Interpreter
// =========================================================================

/**
 * A configurable Starlark interpreter.
 *
 * Wraps the full lexer -> parser -> compiler -> VM pipeline with:
 *
 * - ``load()`` support via a file resolver
 * - File caching (each loaded file is evaluated at most once)
 * - Configurable recursion limits
 * - Pluggable compiler and VM factory
 *
 * For most use cases, the module-level ``interpret()`` function is
 * simpler. Use this class when you need to share a cache across
 * multiple interpret calls or configure advanced options.
 *
 * **Usage:**
 *
 * ```typescript
 * // With a custom compiler and VM:
 * const interp = new StarlarkInterpreter({
 *   compileFn: compileStarlark,
 *   createVMFn: createStarlarkVM,
 *   fileResolver: { "//rules/math.star": "x = 42\n" },
 * });
 * const result = interp.interpret("load('//rules/math.star', 'x')\n");
 *
 * // With the built-in mini compiler (for testing):
 * const interp2 = new StarlarkInterpreter();
 * // Note: without a real compiler, you must provide pre-compiled bytecode.
 * ```
 */
export class StarlarkInterpreter {
  /** How to resolve ``load()`` paths to file contents. */
  readonly fileResolver: FileResolver;

  /** Maximum call stack depth for function calls. */
  readonly maxRecursionDepth: number;

  /** Function that compiles Starlark source to bytecode. */
  readonly compileFn: CompileFn | null;

  /** Function that creates a configured Starlark VM. */
  readonly createVMFn: CreateVMFn;

  /**
   * Pre-seeded variables injected into every VM instance.
   *
   * These are available in all Starlark scopes, including loaded files.
   * Use this for build context like ``_ctx``. Since ``interpret()`` is
   * called recursively for ``load()`` statements, globals are automatically
   * injected into every loaded file's VM instance.
   */
  readonly globals: Record<string, VMValue> | null;

  /**
   * Cache of already-loaded files: label -> exported variables.
   *
   * Each file is evaluated at most once. Subsequent ``load()`` calls
   * for the same file return cached symbols. This matches Bazel
   * semantics where loaded files are frozen after first evaluation.
   */
  private _loadCache: Map<string, Record<string, unknown>> = new Map();

  /**
   * Create a new StarlarkInterpreter.
   *
   * @param options - Configuration options.
   */
  constructor(options?: {
    fileResolver?: FileResolver;
    maxRecursionDepth?: number;
    compileFn?: CompileFn;
    createVMFn?: CreateVMFn;
    globals?: Record<string, VMValue>;
  }) {
    this.fileResolver = options?.fileResolver ?? null;
    this.maxRecursionDepth = options?.maxRecursionDepth ?? 200;
    this.compileFn = options?.compileFn ?? null;
    this.createVMFn = options?.createVMFn ?? createMiniStarlarkVM;
    this.globals = options?.globals ?? null;
  }

  /**
   * Execute Starlark source code and return the result.
   *
   * This is the main entry point. It:
   *   1. Compiles the source to bytecode (using the configured compiler).
   *   2. Creates a fresh VM with ``load()`` registered as a builtin.
   *   3. Executes the bytecode.
   *   4. Returns the result (variables, output, traces).
   *
   * @param source - Starlark source code. Should end with a newline.
   * @returns The execution result with variables, output, and traces.
   * @throws {Error} If no compile function is configured.
   */
  interpret(source: string): StarlarkResult {
    if (!this.compileFn) {
      throw new Error(
        "No compile function configured. Provide a compileFn in the constructor, " +
        "or use interpretBytecode() to execute pre-compiled bytecode.",
      );
    }

    // Compile source to bytecode
    const code = this.compileFn(source);

    return this.interpretBytecode(code);
  }

  /**
   * Execute pre-compiled bytecode and return the result.
   *
   * Use this when you have already compiled the source code (e.g.,
   * in tests where you construct CodeObjects by hand).
   *
   * @param code - A compiled CodeObject.
   * @returns The execution result with variables, output, and traces.
   */
  interpretBytecode(code: CodeObject): StarlarkResult {
    // Create a VM with load() support
    const vm = this.createVMFn({ maxRecursionDepth: this.maxRecursionDepth });
    if (this.globals) {
      vm.injectGlobals(this.globals);
    }
    this._registerLoadHandlers(vm);

    // Execute
    const traces = vm.execute(code);

    return {
      variables: { ...vm.variables },
      output: [...vm.output],
      traces,
    };
  }

  /**
   * Execute a Starlark file by reading it from the filesystem.
   *
   * @param path - Path to the Starlark file.
   * @returns The execution result.
   * @throws {Error} If no compile function is configured.
   */
  interpretFile(path: string): StarlarkResult {
    let source = readFileSync(path, "utf-8");
    // Ensure source ends with newline (parser requirement)
    if (!source.endsWith("\n")) {
      source += "\n";
    }
    return this.interpret(source);
  }

  /**
   * Clear the load cache.
   *
   * Normally each file is evaluated at most once. Calling this method
   * forces all files to be re-evaluated on the next load() call.
   * Useful in tests or when the underlying files have changed.
   */
  clearCache(): void {
    this._loadCache.clear();
  }

  /**
   * Get the current load cache (for inspection/debugging).
   *
   * @returns A read-only view of the cache.
   */
  getCache(): ReadonlyMap<string, Record<string, unknown>> {
    return this._loadCache;
  }

  /**
   * Override the VM's LOAD_MODULE handler to actually resolve and
   * execute files.
   *
   * The compiler compiles ``load("file.star", "symbol")`` into:
   *
   * - ``LOAD_MODULE`` -- resolve and execute the file, push a module dict
   * - ``IMPORT_FROM`` -- extract a symbol from the module dict
   *
   * The default VM handlers are stubs. We override ``LOAD_MODULE``
   * with a closure that uses the interpreter's file resolver and cache
   * to actually load files.
   *
   * @param vm - The GenericVM to configure.
   */
  private _registerLoadHandlers(vm: GenericVM): void {
    /**
     * We capture ``this`` (the interpreter) in a local variable so the
     * closure can access it. This is the TypeScript equivalent of
     * Python's ``interpreter = self`` pattern.
     */
    const interpreter = this;

    /**
     * LOAD_MODULE handler -- Resolve a file label, execute it, push
     * its variables.
     *
     * When the compiler encounters ``load("//rules/python.star", "sym")``,
     * it emits:
     *
     *     LOAD_MODULE 0    // names[0] = "//rules/python.star"
     *     DUP              // Keep module on stack for multiple imports
     *     IMPORT_FROM 1    // names[1] = "sym" -- extract from module dict
     *     STORE_NAME 1     // Store as "sym" in current scope
     *
     * This handler:
     *   1. Reads the module label from the names pool.
     *   2. Checks the interpreter's cache (each file evaluated once).
     *   3. If not cached, resolves the file and executes it.
     *   4. Pushes the module's variables as a dict onto the stack.
     *
     * IMPORT_FROM then pops symbols from this dict.
     */
    vm.registerOpcode(
      Op.LOAD_MODULE,
      (vmInst: GenericVM, instr: Instruction, code: CodeObject) => {
        const index = instr.operand as number;
        const moduleLabel = code.names[index];

        // Check cache -- each file is evaluated at most once.
        if (!interpreter._loadCache.has(moduleLabel)) {
          // Resolve and execute the file
          const contents = resolveFile(
            interpreter.fileResolver,
            moduleLabel,
          );
          // Ensure source ends with newline
          const source = contents.endsWith("\n") ? contents : contents + "\n";

          // Recursively interpret the loaded file.
          // This uses the SAME interpreter instance (same cache, same
          // resolver), so transitive loads are also cached.
          let result: StarlarkResult;
          if (interpreter.compileFn) {
            result = interpreter.interpret(source);
          } else {
            // Without a compiler, we cannot interpret source code.
            throw new Error(
              `Cannot load module "${moduleLabel}": no compile function configured.`,
            );
          }
          interpreter._loadCache.set(moduleLabel, { ...result.variables });
        }

        // Push the module's exported variables as a dict
        const cached = interpreter._loadCache.get(moduleLabel)!;
        vmInst.push({ ...cached } as unknown as never);
        vmInst.advancePc();
        return `Loaded module ${moduleLabel}`;
      },
    );
  }
}

// =========================================================================
// Module-level Convenience Functions
// =========================================================================

/**
 * Execute Starlark source code and return the result.
 *
 * This is the simplest API -- one function call does everything.
 *
 * **Note:** Requires a compile function. When the starlark-ast-to-
 * bytecode-compiler TypeScript package is available, you can pass
 * ``compileStarlark`` as the compileFn option.
 *
 * @param source       - Starlark source code. Should end with a newline.
 * @param options      - Configuration options.
 * @returns The execution result with variables, output, and traces.
 *
 * @example
 * ```typescript
 * import { interpret } from "@coding-adventures/starlark-interpreter";
 * import { compileStarlark } from "@coding-adventures/starlark-ast-to-bytecode-compiler";
 *
 * const result = interpret("x = 1 + 2\nprint(x)\n", {
 *   compileFn: compileStarlark,
 * });
 * console.log(result.variables["x"]); // 3
 * console.log(result.output);         // ["3"]
 * ```
 */
export function interpret(
  source: string,
  options?: {
    fileResolver?: FileResolver;
    maxRecursionDepth?: number;
    compileFn?: CompileFn;
    createVMFn?: CreateVMFn;
  },
): StarlarkResult {
  const interp = new StarlarkInterpreter(options);
  return interp.interpret(source);
}

/**
 * Execute pre-compiled Starlark bytecode and return the result.
 *
 * Use this when you have already compiled the source, or when testing
 * with hand-crafted CodeObjects. No compile function is needed.
 *
 * @param code    - A compiled CodeObject.
 * @param options - Configuration options.
 * @returns The execution result with variables, output, and traces.
 *
 * @example
 * ```typescript
 * import { interpretBytecode, Op } from "@coding-adventures/starlark-interpreter";
 *
 * const code = {
 *   instructions: [
 *     { opcode: Op.LOAD_CONST, operand: 0 },
 *     { opcode: Op.STORE_NAME, operand: 0 },
 *     { opcode: Op.HALT },
 *   ],
 *   constants: [42],
 *   names: ["x"],
 * };
 * const result = interpretBytecode(code);
 * console.log(result.variables["x"]); // 42
 * ```
 */
export function interpretBytecode(
  code: CodeObject,
  options?: {
    fileResolver?: FileResolver;
    maxRecursionDepth?: number;
    createVMFn?: CreateVMFn;
  },
): StarlarkResult {
  const interp = new StarlarkInterpreter(options);
  return interp.interpretBytecode(code);
}

/**
 * Execute a Starlark file by path.
 *
 * @param path    - Path to the Starlark file.
 * @param options - Configuration options.
 * @returns The execution result.
 */
export function interpretFile(
  path: string,
  options?: {
    fileResolver?: FileResolver;
    maxRecursionDepth?: number;
    compileFn?: CompileFn;
    createVMFn?: CreateVMFn;
  },
): StarlarkResult {
  const interp = new StarlarkInterpreter(options);
  return interp.interpretFile(path);
}
