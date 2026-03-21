/**
 * Types — The JSON data contract for pipeline reports.
 * =====================================================
 *
 * The HTML renderer is *language-agnostic*. It doesn't know whether the
 * pipeline was run by the Python, Ruby, or TypeScript implementation.
 * All it sees is JSON conforming to a well-defined contract.
 *
 * This is the same architectural pattern used by web APIs everywhere:
 * the front-end (our renderer) and back-end (the pipeline) communicate
 * through a shared data format. As long as both sides agree on the
 * shape of the data, they can evolve independently.
 *
 * ```
 * Python packages ----> PipelineReport (JSON) --+
 * Ruby packages -----> PipelineReport (JSON) ---+--> HTML Renderer --> report.html
 * TypeScript pkgs ---> PipelineReport (JSON) --+
 * ```
 *
 * The types below are TypeScript interfaces that describe the JSON
 * contract. They have no runtime cost — they exist purely for
 * type checking during development. At runtime, the data is just
 * plain JavaScript objects parsed from JSON.
 */

// ===========================================================================
// Top-Level Report
// ===========================================================================

/**
 * PipelineReport — The root object of every pipeline report.
 *
 * This is what you get when you JSON.parse() a pipeline report file.
 * It contains metadata about the compilation run plus an ordered list
 * of stages (lexer -> parser -> compiler -> VM, etc.).
 *
 * Example:
 * ```json
 * {
 *   "source": "x = 1 + 2",
 *   "language": "python",
 *   "target": "vm",
 *   "metadata": {
 *     "generated_at": "2026-03-18T12:00:00Z",
 *     "generator_version": "0.1.0",
 *     "packages": { "lexer": "0.1.0", "parser": "0.1.0" }
 *   },
 *   "stages": [ ... ]
 * }
 * ```
 */
export interface PipelineReport {
  /** The source code that was compiled (e.g., "x = 1 + 2"). */
  source: string;

  /** Which language implementation produced this report ("python", "ruby", "typescript"). */
  language: string;

  /** The execution target ("vm", "riscv", "arm"). */
  target: string;

  /** Timestamp, version info, and any extra context. */
  metadata: ReportMetadata;

  /** Ordered list of pipeline stages — one StageReport per compilation phase. */
  stages: StageReport[];
}

/**
 * ReportMetadata — Extra context about how the report was generated.
 *
 * Think of this like the EXIF data on a photo: it tells you *when*
 * the picture was taken, *what camera* was used, etc. Similarly,
 * metadata tells us when the compilation happened and which versions
 * of the packages were used.
 */
export interface ReportMetadata {
  /** ISO 8601 timestamp of when the report was generated. */
  generated_at: string;

  /** Version of the generator tool. */
  generator_version: string;

  /** Map of package name -> version for each package used. */
  packages: Record<string, string>;
}

// ===========================================================================
// Stage Reports
// ===========================================================================

/**
 * StageReport — One stage in the compilation pipeline.
 *
 * Each stage transforms its input into an output. For example:
 * - The lexer stage turns source code into tokens
 * - The parser stage turns tokens into an AST
 * - The compiler stage turns an AST into bytecode
 *
 * The `name` field is a machine-readable identifier that the renderer
 * uses to dispatch to the correct rendering function. The `display_name`
 * is a human-friendly label for the HTML section header.
 *
 * The `data` field contains stage-specific structured data. Its shape
 * depends on the stage type — see the individual data interfaces below.
 */
export interface StageReport {
  /** Machine identifier used for rendering dispatch (e.g., "lexer", "parser"). */
  name: string;

  /** Human-readable title for the HTML section (e.g., "Tokenization"). */
  display_name: string;

  /** Short description of what this stage received. */
  input_repr: string;

  /** Short description of what this stage produced. */
  output_repr: string;

  /** How long this stage took, in milliseconds. */
  duration_ms: number;

  /** Stage-specific structured data — see the individual data interfaces. */
  data: StageData;
}

/**
 * StageData — Union of all possible stage data shapes.
 *
 * Rather than using a single giant interface, we define separate
 * interfaces for each stage type. This gives us precise type checking
 * when we know which stage we're dealing with.
 *
 * In practice the renderer uses the stage `name` field to decide
 * which rendering function to call, and each function casts the data
 * to the appropriate type.
 */
export type StageData =
  | LexerData
  | ParserData
  | CompilerData
  | VMData
  | AssemblerData
  | HardwareExecutionData
  | ALUData
  | GateData
  | Record<string, unknown>; // fallback for unknown stages

// ===========================================================================
// Stage-Specific Data Interfaces
// ===========================================================================

/**
 * LexerData — Token stream produced by the lexer stage.
 *
 * Each token is a labeled piece of source text with its position.
 * The renderer displays these as colored badges:
 *
 *     [NAME: x] [EQUALS: =] [NUMBER: 1] [PLUS: +] [NUMBER: 2] [EOF]
 */
export interface LexerData {
  tokens: LexerToken[];
}

/**
 * A single token from the lexer.
 *
 * ```
 * ┌──────────────┐
 * │ type: "NAME" │  <-- what kind of token
 * │ value: "x"   │  <-- the actual text
 * │ line: 1      │  <-- where in the source
 * │ column: 1    │
 * └──────────────┘
 * ```
 */
export interface LexerToken {
  type: string;
  value: string;
  line: number;
  column: number;
}

/**
 * ParserData — Abstract Syntax Tree produced by the parser stage.
 *
 * The AST is a recursive tree of nodes. Each node has a type,
 * an optional value, and zero or more children. The renderer
 * draws this as an SVG tree diagram.
 *
 * Example for `x = 1 + 2`:
 * ```
 *        Assignment
 *        /        \
 *     Name       BinaryOp(+)
 *      |         /         \
 *      x     Number(1)   Number(2)
 * ```
 */
export interface ParserData {
  ast: ASTNode;
}

/**
 * A single node in the AST.
 *
 * The tree is recursive: each node can have children, and each
 * child is itself an ASTNode. Leaf nodes (like numbers and names)
 * have an empty children array.
 */
export interface ASTNode {
  type: string;
  value?: string;
  children: ASTNode[];
}

/**
 * CompilerData — Bytecode instructions produced by the compiler stage.
 *
 * The compiler transforms the AST into a flat list of instructions
 * that a virtual machine can execute. Each instruction has an opcode
 * (like "LOAD_CONST" or "ADD") and an optional argument.
 *
 * The `stack_effect` field shows what happens to the VM's stack
 * when this instruction executes — handy for understanding how
 * stack-based VMs work.
 */
export interface CompilerData {
  instructions: BytecodeInstruction[];
  constants: (number | string)[];
  names: string[];
}

export interface BytecodeInstruction {
  index: number;
  opcode: string;
  arg: string | null;
  stack_effect: string;
}

/**
 * VMData — Step-by-step execution trace from the virtual machine.
 *
 * Each step shows the VM executing one instruction and how the
 * stack and variables change. This is like watching a debugger
 * step through the code one instruction at a time.
 *
 * ```
 * Step 0: LOAD_CONST 1    stack: [] -> [1]        vars: {}
 * Step 1: LOAD_CONST 2    stack: [1] -> [1, 2]    vars: {}
 * Step 2: ADD             stack: [1, 2] -> [3]    vars: {}
 * Step 3: STORE_NAME x    stack: [3] -> []        vars: {x: 3}
 * ```
 */
export interface VMData {
  steps: VMStep[];
}

export interface VMStep {
  index: number;
  instruction: string;
  stack_before: (number | string)[];
  stack_after: (number | string)[];
  variables: Record<string, number | string>;
}

/**
 * AssemblerData — Assembly listing with binary encoding.
 *
 * Each line shows the human-readable assembly instruction alongside
 * its binary encoding. The `encoding` field breaks the binary down
 * into named bit fields so the renderer can color-code them.
 *
 * For example, an R-type RISC-V instruction has these fields:
 * ```
 * | funct7  | rs2   | rs1   | funct3 | rd    | opcode  |
 * | 7 bits  | 5 bits| 5 bits| 3 bits | 5 bits| 7 bits  |
 * ```
 */
export interface AssemblerData {
  lines: AssemblyLine[];
}

export interface AssemblyLine {
  address: number;
  assembly: string;
  binary: string;
  encoding: Record<string, string>;
}

/**
 * HardwareExecutionData — Register-level execution trace.
 *
 * Used for RISC-V and ARM execution stages. Each step shows
 * which instruction executed, which registers changed, and
 * the full register state after execution.
 */
export interface HardwareExecutionData {
  steps: HardwareStep[];
}

export interface HardwareStep {
  address: number;
  instruction: string;
  registers_changed: Record<string, number>;
  registers: Record<string, number>;
}

/**
 * ALUData — Arithmetic Logic Unit operation details.
 *
 * Shows the bit-level details of arithmetic operations. Each
 * operation includes the binary representation of inputs and
 * outputs, plus CPU flags (zero, carry, negative, overflow).
 */
export interface ALUData {
  operations: ALUOperation[];
}

export interface ALUOperation {
  op: string;
  a: number;
  b: number;
  result: number;
  bits_a: string;
  bits_b: string;
  bits_result: string;
  flags: {
    zero: boolean;
    carry: boolean;
    negative: boolean;
    overflow: boolean;
  };
}

/**
 * GateData — Gate-level circuit trace.
 *
 * The lowest level of the computing stack. Each operation
 * represents a group of logic gate evaluations (like a full
 * adder for one bit position). Individual gates show their
 * inputs, output, and a human-readable label.
 *
 * This is where it all bottoms out — every computation a
 * computer does ultimately reduces to AND, OR, XOR, and NOT
 * gates operating on individual bits.
 */
export interface GateData {
  operations: GateOperation[];
}

export interface GateOperation {
  description: string;
  gates: Gate[];
}

export interface Gate {
  gate: string;
  inputs: number[];
  output: number;
  label: string;
}
