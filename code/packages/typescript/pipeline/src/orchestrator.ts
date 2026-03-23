/**
 * Pipeline Orchestrator — Wiring the Full Computing Stack.
 *
 * ==========================================================
 * Chapter 1: What Is a Pipeline?
 * ==========================================================
 *
 * Imagine a factory assembly line. Raw steel enters at one end and a finished
 * car rolls out at the other. Between those two points, a dozen stations each
 * do one specific job: stamping, welding, painting, assembly. No single station
 * builds the whole car — each one transforms its input and passes the result
 * downstream.
 *
 * A **compiler pipeline** works the same way. Raw source code enters at one
 * end, and executable results come out the other. Our pipeline has four
 * stations:
 *
 *     Source code  ->  Lexer  ->  Parser  ->  Compiler  ->  VM
 *
 * 1. **Lexer** (tokenizer): Reads raw characters and groups them into
 *    meaningful tokens — identifiers, numbers, operators, keywords.
 *    Input: `"x = 1 + 2"`  Output: `[NAME("x"), EQUALS, NUMBER(1), PLUS, NUMBER(2)]`
 *
 * 2. **Parser**: Takes the flat token stream and builds a tree structure
 *    (the Abstract Syntax Tree) that encodes precedence and grouping.
 *    Input: token list  Output: `Assignment(Name("x"), BinaryOp(1, "+", 2))`
 *
 * 3. **Compiler**: Walks the AST and emits flat bytecode instructions for
 *    a stack machine. This is where tree structure becomes linear execution.
 *    Input: AST  Output: `[LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0]`
 *
 * 4. **Virtual Machine**: Executes the bytecode instructions one by one,
 *    maintaining a stack, variables, and captured output.
 *    Input: bytecode  Output: `{x: 3}`
 *
 * ==========================================================
 * Chapter 2: Why Capture Traces?
 * ==========================================================
 *
 * The pipeline doesn't just *run* code — it **records** what happened at every
 * stage. This is critical for the HTML visualizer, which lets a learner see
 * exactly how `"x = 1 + 2"` transforms at each step:
 *
 * - The lexer stage shows which characters became which tokens.
 * - The parser stage shows the tree structure (why `*` binds tighter than `+`).
 * - The compiler stage shows the flat bytecode instructions.
 * - The VM stage shows the stack evolving as each instruction executes.
 *
 * Each stage captures its output into a dedicated interface (`LexerStage`,
 * `ParserStage`, `CompilerStage`, `VMStage`), and the complete result
 * is bundled into a `PipelineResult`. The visualizer can then iterate over
 * these stages and render each one.
 *
 * ==========================================================
 * Chapter 3: Implementation
 * ==========================================================
 */

import type { Token } from "@coding-adventures/lexer";
import { tokenize } from "@coding-adventures/lexer";
import type { LexerConfig } from "@coding-adventures/lexer";
import type {
  Assignment,
  BinaryOp,
  Name,
  NumberLiteral,
  Program,
  StringLiteral,
} from "@coding-adventures/parser";
import { Parser } from "@coding-adventures/parser";

import type { CodeObject, Instruction, VMTrace } from "./vm-types.js";
import { OpCode, OpCodeName } from "./vm-types.js";
import { BytecodeCompiler } from "./compiler.js";
import { VirtualMachine } from "./vm.js";

// ---------------------------------------------------------------------------
// Stage interfaces — one per pipeline stage
// ---------------------------------------------------------------------------
//
// Each interface captures the output of one stage in a format that is both
// programmatically useful (the raw objects) and visualization-friendly
// (JSON-serializable summaries, human-readable text).

/**
 * Captured output from the lexer stage.
 *
 * The lexer is the first station on the assembly line. It reads raw
 * source characters and produces a list of *tokens* — the smallest
 * meaningful units of the language.
 *
 * Properties:
 *   tokens      - The complete token stream, including the final EOF token.
 *   tokenCount  - How many tokens were produced (a quick summary stat).
 *   source      - The original source code that was tokenized.
 */
export interface LexerStage {
  readonly tokens: readonly Token[];
  readonly tokenCount: number;
  readonly source: string;
}

/**
 * Captured output from the parser stage.
 *
 * The parser takes the flat token list and builds a tree (the AST)
 * that captures the grammatical structure of the source code —
 * operator precedence, grouping, statement boundaries.
 *
 * Properties:
 *   ast     - The root of the Abstract Syntax Tree.
 *   astDict - A JSON-serializable representation of the AST, suitable for
 *             rendering in the HTML visualizer as an interactive tree diagram.
 */
export interface ParserStage {
  readonly ast: Program;
  readonly astDict: Record<string, unknown>;
}

/**
 * Captured output from the compiler stage.
 *
 * The compiler walks the AST and emits a flat list of bytecode
 * instructions for the stack-based VM. This is where tree structure
 * becomes linear execution order.
 *
 * Properties:
 *   code             - The compiled bytecode — instructions, constants, and names.
 *   instructionsText - Human-readable instruction listing (e.g., `"LOAD_CONST 0 (42)"`).
 *                      This is what the visualizer displays in the bytecode panel.
 *   constants        - The constant pool — literal values referenced by instructions.
 *   names            - The name pool — variable names referenced by instructions.
 */
export interface CompilerStage {
  readonly code: CodeObject;
  readonly instructionsText: readonly string[];
  readonly constants: readonly (number | string)[];
  readonly names: readonly string[];
}

/**
 * Captured output from the VM execution stage.
 *
 * The VM executes bytecode instructions one at a time, recording a
 * trace snapshot after each step. This gives the visualizer a complete
 * replay of the execution — stack states, variable changes, and output.
 *
 * Properties:
 *   traces         - One trace per executed instruction, showing the VM state
 *                    before and after each step.
 *   finalVariables - The variable bindings after execution completes (e.g., `{x: 3}`).
 *   output         - Any captured print output from the program.
 */
export interface VMStage {
  readonly traces: readonly VMTrace[];
  readonly finalVariables: Readonly<Record<string, unknown>>;
  readonly output: readonly string[];
}

/**
 * The complete result of running source code through all stages.
 *
 * This is the top-level container that bundles every stage's output
 * into a single object. The HTML visualizer receives one of these and
 * renders each stage in its own panel.
 *
 * Properties:
 *   source        - The original source code that entered the pipeline.
 *   lexerStage    - Tokens produced by the lexer.
 *   parserStage   - AST produced by the parser.
 *   compilerStage - Bytecode produced by the compiler.
 *   vmStage       - Execution traces and final state from the VM.
 */
export interface PipelineResult {
  readonly source: string;
  readonly lexerStage: LexerStage;
  readonly parserStage: ParserStage;
  readonly compilerStage: CompilerStage;
  readonly vmStage: VMStage;
}

// ---------------------------------------------------------------------------
// AST-to-dictionary conversion
// ---------------------------------------------------------------------------
//
// The HTML visualizer needs a JSON-serializable representation of the AST
// so it can render the tree as an interactive diagram. TypeScript interfaces
// aren't directly JSON-serializable (they may contain circular references or
// non-serializable types), so we convert each node type manually.
//
// This is a classic "visitor" pattern: we inspect the `kind` of each node and
// recursively convert its children. The result is a nested plain object that
// mirrors the tree structure.

/**
 * Convert an AST node to a JSON-serializable dictionary.
 *
 * This function walks the AST recursively, converting each node into a
 * plain object with a `"type"` key and type-specific fields. The
 * HTML visualizer uses these dictionaries to render the AST as an
 * interactive tree.
 *
 * @param node - An AST node (Program, Assignment, BinaryOp, NumberLiteral,
 *               StringLiteral, Name) or any other object.
 * @returns A JSON-serializable representation of the node.
 *
 * @example
 *     astToDict({ kind: "NumberLiteral", value: 42 })
 *     // => { type: "NumberLiteral", value: 42 }
 *
 *     astToDict({ kind: "BinaryOp", left: ..., op: "+", right: ... })
 *     // => { type: "BinaryOp", op: "+", left: {...}, right: {...} }
 */
export function astToDict(
  node: unknown,
): Record<string, unknown> | string | number {
  // We use duck-typing to check for AST node shapes. Each node has a
  // `kind` field that tells us what type it is — this is TypeScript's
  // discriminated union pattern.
  const n = node as { kind?: string };

  // --- Program: the root node containing all statements ---
  if (n.kind === "Program") {
    const prog = node as Program;
    return {
      type: "Program",
      statements: prog.statements.map((s) => astToDict(s)),
    };
  }

  // --- Assignment: `target = value` ---
  if (n.kind === "Assignment") {
    const assign = node as Assignment;
    return {
      type: "Assignment",
      target: astToDict(assign.target),
      value: astToDict(assign.value),
    };
  }

  // --- BinaryOp: `left op right` (e.g., `1 + 2`) ---
  if (n.kind === "BinaryOp") {
    const binOp = node as BinaryOp;
    return {
      type: "BinaryOp",
      op: binOp.op,
      left: astToDict(binOp.left),
      right: astToDict(binOp.right),
    };
  }

  // --- NumberLiteral: a numeric value like `42` ---
  if (n.kind === "NumberLiteral") {
    return { type: "NumberLiteral", value: (node as NumberLiteral).value };
  }

  // --- StringLiteral: a string value like `"hello"` ---
  if (n.kind === "StringLiteral") {
    return { type: "StringLiteral", value: (node as StringLiteral).value };
  }

  // --- Name: a variable reference like `x` ---
  if (n.kind === "Name") {
    return { type: "Name", name: (node as Name).name };
  }

  // --- Fallback for unknown node types ---
  // If the parser adds new node types in the future, they'll be caught
  // here with a reasonable representation rather than crashing.
  if (typeof node === "object" && node !== null) {
    const obj = node as Record<string, unknown>;
    return { type: String(obj.constructor?.name ?? "Unknown"), repr: String(node) };
  }

  return { type: typeof node, repr: String(node) };
}

// ---------------------------------------------------------------------------
// Instruction-to-text conversion
// ---------------------------------------------------------------------------
//
// The visualizer shows human-readable bytecode like:
//
//     LOAD_CONST 0 (42)
//     LOAD_CONST 1 (2)
//     ADD
//     STORE_NAME 0 ('x')
//
// This helper resolves operand indices to their actual values from the
// constant and name pools, making the output much more readable than
// raw indices.

/**
 * Convert a bytecode instruction to human-readable text.
 *
 * For instructions with operands, this resolves the operand index to the
 * actual value from the constant or name pool. For example:
 *
 * - `LOAD_CONST 0` becomes `LOAD_CONST 0 (42)` if constants[0] is 42.
 * - `STORE_NAME 0` becomes `STORE_NAME 0 ('x')` if names[0] is "x".
 * - `ADD` stays `ADD` (no operand to resolve).
 *
 * @param instr - An Instruction object with `opcode` and optional `operand`.
 * @param code  - The compiled code object (provides the constant and name pools).
 * @returns A human-readable string representation of the instruction.
 */
export function instructionToText(instr: Instruction, code: CodeObject): string {
  const opcodeName = OpCodeName[instr.opcode] ?? `UNKNOWN(0x${instr.opcode.toString(16)})`;

  if (instr.operand !== undefined) {
    // For LOAD_CONST instructions, show the actual constant value
    // alongside the index. This is like showing "MOV R1, #42" instead
    // of just "MOV R1, [const_pool+0]" — much easier to understand.
    if (
      instr.opcode === OpCode.LOAD_CONST &&
      typeof instr.operand === "number"
    ) {
      if (instr.operand >= 0 && instr.operand < code.constants.length) {
        const value = code.constants[instr.operand];
        return `${opcodeName} ${instr.operand} (${JSON.stringify(value)})`;
      }
    }

    // For STORE_NAME and LOAD_NAME, show the actual variable name.
    // This turns "STORE_NAME 0" into "STORE_NAME 0 ('x')" — you can
    // read the bytecode like pseudocode.
    if (
      (instr.opcode === OpCode.STORE_NAME ||
        instr.opcode === OpCode.LOAD_NAME) &&
      typeof instr.operand === "number"
    ) {
      if (instr.operand >= 0 && instr.operand < code.names.length) {
        const name = code.names[instr.operand];
        return `${opcodeName} ${instr.operand} ('${name}')`;
      }
    }

    // For any other instruction with an operand, just show the raw value.
    return `${opcodeName} ${instr.operand}`;
  }

  // Instructions without operands (ADD, SUB, MUL, DIV, POP, HALT, etc.)
  // are self-describing — just the opcode name is enough.
  return opcodeName;
}

// ---------------------------------------------------------------------------
// The Pipeline class — the assembly line itself
// ---------------------------------------------------------------------------

/**
 * The main pipeline orchestrator.
 *
 * Chains: Source -> Lexer -> Parser -> Compiler -> VM
 *
 * This class is the assembly line foreman. It doesn't do any of the
 * actual work (tokenizing, parsing, compiling, executing) — that's
 * handled by the specialized packages. Instead, it coordinates them:
 *
 * 1. Creates a lexer, feeds it the source, collects tokens.
 * 2. Creates a parser, feeds it the tokens, collects the AST.
 * 3. Creates a compiler, feeds it the AST, collects bytecode.
 * 4. Creates a VM, feeds it the bytecode, collects execution traces.
 *
 * At each step, it captures the output into a stage interface so the
 * HTML visualizer can show exactly what happened.
 *
 * Why a class and not a bare function?
 * ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
 * Right now, `Pipeline` has no instance state — `run()` could easily
 * be a module-level function. We use a class because:
 *
 * 1. Future configuration (e.g., choosing between hand-written and
 *    grammar-driven parsers) will be instance attributes.
 * 2. It's conventional for orchestrators to be classes (think of
 *    `unittest.TestRunner`, `concurrent.futures.Executor`).
 * 3. It makes the API consistent: `new Pipeline().run(source)`.
 */
export class Pipeline {
  /**
   * Run source code through the full pipeline.
   *
   * This is the main entry point. Given source code like `"x = 1 + 2"`,
   * it runs all four stages and returns a `PipelineResult` containing
   * captured data from every stage.
   *
   * @param source   - The source code to execute (e.g., `"x = 1 + 2"`).
   * @param keywords - Language-specific keywords to pass to the lexer. If
   *                   undefined, the lexer uses its default configuration
   *                   with no keywords.
   * @returns A complete record of what happened at every stage.
   *
   * @example
   *     const result = new Pipeline().run("x = 1 + 2");
   *     assert(result.vmStage.finalVariables.x === 3);
   *
   * @example
   *     // Multiple statements:
   *     const result = new Pipeline().run("a = 10\nb = 20\nc = a + b");
   *     assert(result.vmStage.finalVariables.c === 30);
   *
   * @example
   *     // With custom keywords:
   *     const result = new Pipeline().run("if x = 1", ["if", "else"]);
   */
  run(source: string, keywords?: string[]): PipelineResult {
    // ---------------------------------------------------------------
    // Stage 1: Lexing — characters to tokens
    // ---------------------------------------------------------------
    // The lexer reads the source string character by character and
    // groups characters into tokens. This is like the first station
    // on an assembly line that cuts raw material into standard pieces.

    const config: LexerConfig | undefined = keywords
      ? { keywords }
      : undefined;
    const tokens = tokenize(source, config);

    const lexerStage: LexerStage = {
      tokens,
      tokenCount: tokens.length,
      source,
    };

    // ---------------------------------------------------------------
    // Stage 2: Parsing — tokens to AST
    // ---------------------------------------------------------------
    // The parser takes the flat token stream and builds a tree that
    // represents the grammatical structure. This is where operator
    // precedence is encoded: `1 + 2 * 3` becomes a tree where
    // `*` is deeper than `+`, ensuring it's evaluated first.

    const parser = new Parser(tokens);
    const ast = parser.parse();

    const parserStage: ParserStage = {
      ast,
      astDict: astToDict(ast) as Record<string, unknown>,
    };

    // ---------------------------------------------------------------
    // Stage 3: Compilation — AST to bytecode
    // ---------------------------------------------------------------
    // The compiler walks the AST in post-order and emits a flat
    // sequence of stack-machine instructions. Tree structure becomes
    // linear execution order. This is the same transformation that
    // javac, csc, and cpython perform.

    const compiler = new BytecodeCompiler();
    const code = compiler.compile(ast);

    const compilerStage: CompilerStage = {
      code,
      instructionsText: code.instructions.map((instr) =>
        instructionToText(instr, code),
      ),
      constants: [...code.constants],
      names: [...code.names],
    };

    // ---------------------------------------------------------------
    // Stage 4: VM Execution — bytecode to results
    // ---------------------------------------------------------------
    // The VM interprets the bytecode instructions one by one,
    // maintaining a stack, variables, and captured output. It records
    // a trace after each instruction so the visualizer can replay
    // the entire execution step by step.

    const vm = new VirtualMachine();
    const traces = vm.execute(code);

    const vmStage: VMStage = {
      traces,
      finalVariables: { ...vm.variables },
      output: [...vm.output],
    };

    // ---------------------------------------------------------------
    // Bundle everything into a PipelineResult
    // ---------------------------------------------------------------

    return {
      source,
      lexerStage,
      parserStage,
      compilerStage,
      vmStage,
    };
  }
}
