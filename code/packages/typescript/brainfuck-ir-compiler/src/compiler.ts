/**
 * Brainfuck IR Compiler — translates Brainfuck ASTs into general-purpose IR.
 *
 * =============================================================================
 * What this compiler does
 * =============================================================================
 *
 * This is the Brainfuck-specific **frontend** of the AOT compiler pipeline.
 * It knows Brainfuck semantics (tape, cells, pointer, loops, I/O) and
 * translates them into target-independent IR instructions. It does NOT know
 * about RISC-V, ARM, ELF, or any specific machine target.
 *
 * The compiler produces two outputs:
 *   1. An IrProgram containing the compiled IR instructions
 *   2. SourceToAst and AstToIr source map segments for debugging
 *
 * =============================================================================
 * Register allocation
 * =============================================================================
 *
 * Brainfuck needs very few registers:
 *
 *   v0 = tape base address (pointer to the start of the tape)
 *   v1 = tape pointer offset (current cell index, 0-based)
 *   v2 = temporary (cell value for loads/stores)
 *   v3 = temporary (for bounds checks)
 *   v4 = temporary (for syscall arguments)
 *   v5 = max pointer value (tape_size - 1, for bounds checks)
 *   v6 = zero constant (for bounds checks)
 *
 * This fixed allocation maps directly to a small set of physical registers
 * in any ISA. Future languages (BASIC) that need more registers will use a
 * register allocator in the backend.
 *
 * =============================================================================
 * Command → IR mapping
 * =============================================================================
 *
 * ┌────────────────┬──────────────────────────────────────────────────────────┐
 * │ Command        │ IR Output                                                │
 * ├────────────────┼──────────────────────────────────────────────────────────┤
 * │ > (RIGHT)      │ ADD_IMM v1, v1, 1                                       │
 * │ < (LEFT)       │ ADD_IMM v1, v1, -1                                      │
 * │ + (INC)        │ LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, 1;               │
 * │                │ AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1             │
 * │ - (DEC)        │ LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, -1;              │
 * │                │ AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1             │
 * │ . (OUTPUT)     │ LOAD_BYTE v2, v0, v1; ADD_IMM v4, v2, 0; SYSCALL 1    │
 * │ , (INPUT)      │ SYSCALL 2; STORE_BYTE v4, v0, v1                       │
 * └────────────────┴──────────────────────────────────────────────────────────┘
 */

import type { ASTNode } from "@coding-adventures/parser";
import { isASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";
import {
  IrProgram,
  IDGenerator,
  IrOp,
  reg,
  imm,
  lbl,
} from "@coding-adventures/compiler-ir";
import { SourceMapChain } from "@coding-adventures/compiler-source-map";
import type { SourcePosition } from "@coding-adventures/compiler-source-map";
import type { BuildConfig } from "./build_config.js";

// ──────────────────────────────────────────────────────────────────────────────
// Register constants — virtual register indices used by the compiler.
//
// These are fixed allocations: every Brainfuck program compiled by this
// frontend uses the same register numbers for the same purposes. The backend
// maps them to physical registers (e.g., a0, a1, t0, t1 in RISC-V ABI).
// ──────────────────────────────────────────────────────────────────────────────

const REG_TAPE_BASE = 0; // v0: base address of the tape
const REG_TAPE_PTR = 1;  // v1: current cell offset (0-based index)
const REG_TEMP = 2;      // v2: temporary for cell values
const REG_TEMP2 = 3;     // v3: temporary for bounds checks
const REG_SYS_ARG = 4;   // v4: syscall argument / return value
const REG_MAX_PTR = 5;   // v5: tape_size - 1 (for bounds checks)
const REG_ZERO = 6;      // v6: constant 0 (for bounds checks and output)

// ──────────────────────────────────────────────────────────────────────────────
// Syscall numbers — match the RISC-V simulator's ecall dispatch.
//
//   1 = write: output the byte in a0 to stdout
//   2 = read:  read one byte from stdin into a0
//  10 = exit:  halt the program; exit code in a0
// ──────────────────────────────────────────────────────────────────────────────

const SYSCALL_WRITE = 1;  // write byte in v4 to stdout
const SYSCALL_READ = 2;   // read byte from stdin into v4
const SYSCALL_EXIT = 10;  // halt with exit code in v4

// ──────────────────────────────────────────────────────────────────────────────
// CompileResult — the two outputs of compilation
// ──────────────────────────────────────────────────────────────────────────────

export interface CompileResult {
  /** The compiled IR program. Feed this to an optimizer or backend. */
  readonly program: IrProgram;
  /** The source map chain for debugging. Feed this to the same optimizer/backend. */
  readonly sourceMap: SourceMapChain;
}

// ──────────────────────────────────────────────────────────────────────────────
// compile — the public entry point
// ──────────────────────────────────────────────────────────────────────────────

/**
 * Compile a Brainfuck AST to IR.
 *
 * Takes the AST produced by `parseBrainfuck()` and a source filename, and
 * produces an IrProgram plus source map segments.
 *
 * The filename is used in source map entries to identify which file the
 * source positions refer to.
 *
 * @param ast - The Brainfuck AST (root node must have ruleName "program").
 * @param filename - The source file name (e.g., "hello.bf").
 * @param config - Compilation flags (use debugConfig() or releaseConfig()).
 * @returns CompileResult with the IR program and source map.
 * @throws {Error} If the AST root is not a "program" node or tapeSize <= 0.
 *
 * @example
 *   import { parseBrainfuck } from "@coding-adventures/brainfuck";
 *   import { compile, releaseConfig } from "@coding-adventures/brainfuck-ir-compiler";
 *
 *   const ast = parseBrainfuck("++[-].");
 *   const { program, sourceMap } = compile(ast, "hello.bf", releaseConfig());
 */
export function compile(
  ast: ASTNode,
  filename: string,
  config: BuildConfig
): CompileResult {
  // Validate inputs
  if (ast.ruleName !== "program") {
    throw new Error(`expected 'program' AST node, got "${ast.ruleName}"`);
  }
  if (config.tapeSize <= 0) {
    throw new Error(`invalid tapeSize ${config.tapeSize}: must be positive`);
  }

  const c = new Compiler(config, filename);
  return c.compileProgram(ast);
}

// ──────────────────────────────────────────────────────────────────────────────
// Internal Compiler class
//
// The Compiler class holds mutable state during compilation:
//   - idGen:     monotonic instruction ID counter
//   - nodeIdGen: monotonic AST node ID counter (for source mapping)
//   - loopCount: next loop number (for generating unique loop labels)
//   - program:   the IrProgram being built
//   - sourceMap: the SourceMapChain being built
// ──────────────────────────────────────────────────────────────────────────────

class Compiler {
  private readonly config: BuildConfig;
  private readonly filename: string;
  private readonly idGen: IDGenerator;
  private nodeIdCounter: number;
  private loopCount: number;
  private readonly program: IrProgram;
  private readonly sourceMap: SourceMapChain;

  constructor(config: BuildConfig, filename: string) {
    this.config = config;
    this.filename = filename;
    this.idGen = new IDGenerator();
    this.nodeIdCounter = 0;
    this.loopCount = 0;
    this.program = new IrProgram("_start");
    this.sourceMap = new SourceMapChain();
  }

  /** Allocate the next unique AST node ID. */
  private nextNodeId(): number {
    return this.nodeIdCounter++;
  }

  /**
   * Emit one instruction and return its ID.
   *
   * This is the central emission method. Every IR instruction passes through
   * here, ensuring every instruction gets a unique ID from idGen.
   *
   * Accepts any IrOperand values (IrRegister, IrImmediate, or IrLabel).
   * Use the factory functions reg(), imm(), lbl() to create operands.
   */
  private emit(opcode: IrOp, ...operands: import("@coding-adventures/compiler-ir").IrOperand[]): number {
    const id = this.idGen.next();
    this.program.addInstruction({ opcode, operands, id });
    return id;
  }

  /**
   * Emit a label instruction.
   *
   * Labels produce no machine code — they mark an address in the
   * instruction stream. Labels always get ID -1 (not from idGen).
   */
  private emitLabel(name: string): void {
    this.program.addInstruction({
      opcode: IrOp.LABEL,
      operands: [lbl(name)],
      id: -1,
    });
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Top-level compilation
  // ────────────────────────────────────────────────────────────────────────────

  compileProgram(ast: ASTNode): CompileResult {
    // Add tape data declaration
    this.program.addData({
      label: "tape",
      size: this.config.tapeSize,
      init: 0,
    });

    // Emit prologue
    this.emitPrologue();

    // Compile the program body (all top-level instructions)
    for (const child of ast.children) {
      if (isASTNode(child)) {
        this.compileNode(child);
      }
    }

    // Emit epilogue
    this.emitEpilogue();

    return { program: this.program, sourceMap: this.sourceMap };
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Prologue and Epilogue
  //
  // Prologue: sets up the execution environment
  //   - _start label
  //   - v0 = &tape   (base address)
  //   - v1 = 0       (tape pointer starts at cell 0)
  //   - v5 = tape_size - 1  (debug: max valid pointer index)
  //   - v6 = 0              (debug: lower bound for comparison)
  //
  // Epilogue: terminates the program
  //   - HALT
  //   - (debug) __trap_oob handler
  // ────────────────────────────────────────────────────────────────────────────

  private emitPrologue(): void {
    this.emitLabel("_start");

    // v0 = &tape (base address of the tape array)
    this.emit(IrOp.LOAD_ADDR, reg(REG_TAPE_BASE), lbl("tape"));

    // v1 = 0 (tape pointer starts at cell 0)
    this.emit(IrOp.LOAD_IMM, reg(REG_TAPE_PTR), imm(0));

    // Debug mode: set up bounds check registers
    if (this.config.insertBoundsChecks) {
      // v5 = tape_size - 1 (max valid pointer index)
      this.emit(IrOp.LOAD_IMM, reg(REG_MAX_PTR), imm(this.config.tapeSize - 1));
      // v6 = 0 (for lower bound check comparison)
      this.emit(IrOp.LOAD_IMM, reg(REG_ZERO), imm(0));
    }
  }

  private emitEpilogue(): void {
    this.emit(IrOp.HALT);

    // Out-of-bounds trap handler (debug builds only)
    if (this.config.insertBoundsChecks) {
      this.emitLabel("__trap_oob");
      // Load error exit code (1) into syscall argument register
      this.emit(IrOp.LOAD_IMM, reg(REG_SYS_ARG), imm(1));
      // Exit with error code 1
      this.emit(IrOp.SYSCALL, imm(SYSCALL_EXIT));
    }
  }

  // ────────────────────────────────────────────────────────────────────────────
  // AST walking
  //
  // The AST structure (from the Brainfuck grammar):
  //
  //   program     → { instruction }
  //   instruction → loop | command
  //   loop        → LOOP_START { instruction } LOOP_END
  //   command     → RIGHT | LEFT | INC | DEC | OUTPUT | INPUT
  // ────────────────────────────────────────────────────────────────────────────

  private compileNode(node: ASTNode): void {
    switch (node.ruleName) {
      case "instruction":
        // An instruction wraps either a loop or a command — descend into it
        for (const child of node.children) {
          if (isASTNode(child)) {
            this.compileNode(child);
          }
        }
        break;

      case "command":
        this.compileCommand(node);
        break;

      case "loop":
        this.compileLoop(node);
        break;

      default:
        throw new Error(`unexpected AST node type: "${node.ruleName}"`);
    }
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Command compilation
  // ────────────────────────────────────────────────────────────────────────────

  private compileCommand(node: ASTNode): void {
    const tok = extractToken(node);
    if (tok === null) {
      throw new Error("command node has no token");
    }

    // Create source map entries for this command
    const astNodeId = this.nextNodeId();
    const pos: SourcePosition = {
      file: this.filename,
      line: tok.line,
      column: tok.column,
      length: 1,
    };
    this.sourceMap.sourceToAst.add(pos, astNodeId);

    const irIds: number[] = [];

    switch (tok.value) {
      case ">": { // RIGHT: move tape pointer right
        if (this.config.insertBoundsChecks) {
          irIds.push(...this.emitBoundsCheckRight());
        }
        const id = this.emit(
          IrOp.ADD_IMM,
          reg(REG_TAPE_PTR),
          reg(REG_TAPE_PTR),
          imm(1)
        );
        irIds.push(id);
        break;
      }

      case "<": { // LEFT: move tape pointer left
        if (this.config.insertBoundsChecks) {
          irIds.push(...this.emitBoundsCheckLeft());
        }
        const id = this.emit(
          IrOp.ADD_IMM,
          reg(REG_TAPE_PTR),
          reg(REG_TAPE_PTR),
          imm(-1)
        );
        irIds.push(id);
        break;
      }

      case "+": // INC: increment current cell
        irIds.push(...this.emitCellMutation(1));
        break;

      case "-": // DEC: decrement current cell
        irIds.push(...this.emitCellMutation(-1));
        break;

      case ".": { // OUTPUT: write current cell to stdout
        // Load current cell value into temp register
        const id1 = this.emit(
          IrOp.LOAD_BYTE,
          reg(REG_TEMP),
          reg(REG_TAPE_BASE),
          reg(REG_TAPE_PTR)
        );
        irIds.push(id1);
        // Copy to the syscall argument register without depending on v6.
        const id2 = this.emit(
          IrOp.ADD_IMM,
          reg(REG_SYS_ARG),
          reg(REG_TEMP),
          imm(0)
        );
        irIds.push(id2);
        // Syscall 1 = write byte
        const id3 = this.emit(IrOp.SYSCALL, imm(SYSCALL_WRITE));
        irIds.push(id3);
        break;
      }

      case ",": { // INPUT: read byte from stdin into current cell
        // Syscall 2 = read byte (result goes into v4)
        const id1 = this.emit(IrOp.SYSCALL, imm(SYSCALL_READ));
        irIds.push(id1);
        // Store result (from syscall arg register v4) to current cell
        const id2 = this.emit(
          IrOp.STORE_BYTE,
          reg(REG_SYS_ARG),
          reg(REG_TAPE_BASE),
          reg(REG_TAPE_PTR)
        );
        irIds.push(id2);
        break;
      }

      default:
        throw new Error(`unknown command token: "${tok.value}"`);
    }

    // Record AST → IR mapping for source mapping
    this.sourceMap.astToIr.add(astNodeId, irIds);
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Cell mutation: INC (+) and DEC (-)
  //
  // The sequence is:
  //   LOAD_BYTE  v2, v0, v1        ← load current cell value
  //   ADD_IMM    v2, v2, delta      ← increment/decrement by delta
  //   AND_IMM    v2, v2, 255        ← mask to byte range (if enabled)
  //   STORE_BYTE v2, v0, v1        ← store back to current cell
  // ────────────────────────────────────────────────────────────────────────────

  private emitCellMutation(delta: number): number[] {
    const ids: number[] = [];

    // Load current cell value
    ids.push(this.emit(
      IrOp.LOAD_BYTE,
      reg(REG_TEMP),
      reg(REG_TAPE_BASE),
      reg(REG_TAPE_PTR)
    ));

    // Add/subtract delta
    ids.push(this.emit(
      IrOp.ADD_IMM,
      reg(REG_TEMP),
      reg(REG_TEMP),
      imm(delta)
    ));

    // Mask to byte range 0-255 (if enabled)
    if (this.config.maskByteArithmetic) {
      ids.push(this.emit(
        IrOp.AND_IMM,
        reg(REG_TEMP),
        reg(REG_TEMP),
        imm(255)
      ));
    }

    // Store back to current cell
    ids.push(this.emit(
      IrOp.STORE_BYTE,
      reg(REG_TEMP),
      reg(REG_TAPE_BASE),
      reg(REG_TAPE_PTR)
    ));

    return ids;
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Bounds checking
  //
  // RIGHT (>): check ptr < tape_size before incrementing
  //   CMP_GT  v3, v1, v5        ← is ptr >= tape_size? (i.e., ptr > tape_size-1)
  //   BRANCH_NZ v3, __trap_oob  ← if so, trap
  //
  // LEFT (<): check ptr > 0 before decrementing
  //   CMP_LT  v3, v1, v6        ← is ptr < 0?
  //   BRANCH_NZ v3, __trap_oob  ← if so, trap
  //
  // Note: bounds checks fire BEFORE the pointer move, not after.
  // ────────────────────────────────────────────────────────────────────────────

  private emitBoundsCheckRight(): number[] {
    const ids: number[] = [];
    ids.push(this.emit(
      IrOp.CMP_GT,
      reg(REG_TEMP2),
      reg(REG_TAPE_PTR),
      reg(REG_MAX_PTR)
    ));
    ids.push(this.emit(
      IrOp.BRANCH_NZ,
      reg(REG_TEMP2),
      lbl("__trap_oob")
    ));
    return ids;
  }

  private emitBoundsCheckLeft(): number[] {
    const ids: number[] = [];
    ids.push(this.emit(
      IrOp.CMP_LT,
      reg(REG_TAPE_PTR),
      reg(REG_TAPE_PTR),
      reg(REG_ZERO)
    ));
    ids.push(this.emit(
      IrOp.BRANCH_NZ,
      reg(REG_TAPE_PTR),
      lbl("__trap_oob")
    ));
    return ids;
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Loop compilation
  //
  // A Brainfuck loop [body] compiles to:
  //
  //   LABEL      loop_N_start
  //   LOAD_BYTE  v2, v0, v1          ← load current cell
  //   BRANCH_Z   v2, loop_N_end      ← skip body if cell == 0
  //   ...compile body...
  //   JUMP       loop_N_start        ← repeat
  //   LABEL      loop_N_end
  //
  // Each loop gets a unique number N (from loopCount) to ensure labels
  // are unique even for nested loops.
  // ────────────────────────────────────────────────────────────────────────────

  private compileLoop(node: ASTNode): void {
    const loopNum = this.loopCount++;
    const startLabel = `loop_${loopNum}_start`;
    const endLabel = `loop_${loopNum}_end`;

    // Source mapping for the loop construct itself
    const astNodeId = this.nextNodeId();
    if (node.startLine !== undefined && node.startLine > 0) {
      this.sourceMap.sourceToAst.add(
        {
          file: this.filename,
          line: node.startLine,
          column: node.startColumn ?? 1,
          length: 1,
        },
        astNodeId
      );
    }

    const irIds: number[] = [];

    // Emit loop start label
    this.emitLabel(startLabel);

    // Load current cell and branch to end if cell is zero
    irIds.push(this.emit(
      IrOp.LOAD_BYTE,
      reg(REG_TEMP),
      reg(REG_TAPE_BASE),
      reg(REG_TAPE_PTR)
    ));
    irIds.push(this.emit(
      IrOp.BRANCH_Z,
      reg(REG_TEMP),
      lbl(endLabel)
    ));

    // Compile loop body — all instruction children, skipping bracket tokens
    for (const child of node.children) {
      if (isASTNode(child)) {
        this.compileNode(child);
      }
      // Tokens (LOOP_START "[" and LOOP_END "]") are skipped
    }

    // Jump back to loop start
    irIds.push(this.emit(IrOp.JUMP, lbl(startLabel)));

    // Emit loop end label
    this.emitLabel(endLabel);

    // Record AST → IR mapping for the loop construct
    this.sourceMap.astToIr.add(astNodeId, irIds);
  }
}

// ──────────────────────────────────────────────────────────────────────────────
// Token extraction
//
// The AST structure is:
//   command → ASTNode (leaf, single-token children)
//
// We need to find the Token inside the command node. The grammar produces:
//   command → Token(INC, "+")
//
// In practice the token may be wrapped in another ASTNode layer depending on
// the grammar parser internals, so we do a depth-first search.
// ──────────────────────────────────────────────────────────────────────────────

function extractToken(node: ASTNode): Token | null {
  for (const child of node.children) {
    if (!isASTNode(child)) {
      // child is a Token
      return child as Token;
    } else {
      // Recurse into AST node children
      const tok = extractToken(child);
      if (tok !== null) return tok;
    }
  }
  return null;
}
