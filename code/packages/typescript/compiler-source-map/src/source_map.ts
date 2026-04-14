/**
 * Source Map Chain — pipeline sidecar for the AOT compiler.
 *
 * =============================================================================
 * Why a "chain" instead of a flat table?
 * =============================================================================
 *
 * A flat table (machine-code offset → source position) works for the final
 * consumer — a debugger, profiler, or error reporter. But it doesn't help
 * when you're debugging the *compiler itself*:
 *
 *   - "Why did the optimiser delete instruction #42?"
 *     → Look at the IrToIr segment for that pass.
 *
 *   - "Which AST node produced this IR instruction?"
 *     → Look at AstToIr.
 *
 *   - "The machine code for this instruction seems wrong — what IR produced it?"
 *     → Look at IrToMachineCode in reverse.
 *
 * The chain makes the compiler pipeline **transparent and debuggable at every
 * stage**. The flat composite mapping is just the composition of all segments.
 *
 * =============================================================================
 * Segment overview
 * =============================================================================
 *
 *   Segment 1: SourceToAst       — source text position  → AST node ID
 *   Segment 2: AstToIr           — AST node ID           → IR instruction IDs
 *   Segment 3: IrToIr            — IR instruction ID     → optimised IR instruction IDs
 *                                  (one segment per optimiser pass)
 *   Segment 4: IrToMachineCode   — IR instruction ID     → machine code byte offset + length
 *
 *   Composite: source position → machine code offset  (forward)
 *              machine code offset → source position   (reverse)
 */

// ──────────────────────────────────────────────────────────────────────────────
// SourcePosition — a span of characters in a source file
//
// Think of this as a "highlighter pen" marking a region of source code.
// The (line, column) pair marks the start; length tells you how many
// characters are highlighted. For Brainfuck, every command is exactly
// one character (length = 1). For BASIC, a keyword like "PRINT" would
// have length = 5.
// ──────────────────────────────────────────────────────────────────────────────

export interface SourcePosition {
  /** Source file path (e.g., "hello.bf"). */
  readonly file: string;
  /** 1-based line number. */
  readonly line: number;
  /** 1-based column number. */
  readonly column: number;
  /** Character span in source. */
  readonly length: number;
}

/**
 * Returns a human-readable representation like "hello.bf:1:3 (len=1)".
 */
export function sourcePositionToString(sp: SourcePosition): string {
  return `${sp.file}:${sp.line}:${sp.column} (len=${sp.length})`;
}

// ──────────────────────────────────────────────────────────────────────────────
// SourceToAstEntry — one mapping from a source position to an AST node
//
// Example: The "+" character at line 1, column 3 of "hello.bf" maps
// to AST node #42 (which is a command(INC) node in the parse tree).
// ──────────────────────────────────────────────────────────────────────────────

export interface SourceToAstEntry {
  readonly pos: SourcePosition;
  readonly astNodeId: number;
}

// ──────────────────────────────────────────────────────────────────────────────
// SourceToAst — Segment 1: source text positions → AST node IDs
//
// This segment is produced by the parser or by the language-specific
// frontend (e.g., brainfuck-ir-compiler). It maps every meaningful
// source position to the AST node that represents it.
// ──────────────────────────────────────────────────────────────────────────────

export class SourceToAst {
  public entries: SourceToAstEntry[] = [];

  /**
   * Record a mapping from a source position to an AST node ID.
   */
  add(pos: SourcePosition, astNodeId: number): void {
    this.entries.push({ pos, astNodeId });
  }

  /**
   * Return the source position for the given AST node ID, or null if not found.
   * Linear scan — suitable for small to medium programs.
   */
  lookupByNodeId(astNodeId: number): SourcePosition | null {
    for (const entry of this.entries) {
      if (entry.astNodeId === astNodeId) {
        return entry.pos;
      }
    }
    return null;
  }
}

// ──────────────────────────────────────────────────────────────────────────────
// AstToIrEntry — one mapping from an AST node to the IR instructions it produced
//
// A single AST node often produces multiple IR instructions. For example,
// a Brainfuck "+" command produces four instructions:
//   LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE
// So the mapping is one-to-many: ast_node_42 → [ir_7, ir_8, ir_9, ir_10].
// ──────────────────────────────────────────────────────────────────────────────

export interface AstToIrEntry {
  readonly astNodeId: number;
  readonly irIds: readonly number[]; // the IR instruction IDs this AST node produced
}

// ──────────────────────────────────────────────────────────────────────────────
// AstToIr — Segment 2: AST node IDs → IR instruction IDs
// ──────────────────────────────────────────────────────────────────────────────

export class AstToIr {
  public entries: AstToIrEntry[] = [];

  /**
   * Record that the given AST node produced the given IR instruction IDs.
   */
  add(astNodeId: number, irIds: number[]): void {
    this.entries.push({ astNodeId, irIds });
  }

  /**
   * Return the IR instruction IDs for the given AST node, or null if not found.
   */
  lookupByAstNodeId(astNodeId: number): readonly number[] | null {
    for (const entry of this.entries) {
      if (entry.astNodeId === astNodeId) {
        return entry.irIds;
      }
    }
    return null;
  }

  /**
   * Return the AST node ID that produced the given IR instruction, or -1 if
   * not found. Used for reverse lookups (IR → source).
   *
   * When multiple AST nodes map to the same IR ID (unusual), returns the
   * first match found.
   */
  lookupByIrId(irId: number): number {
    for (const entry of this.entries) {
      for (const id of entry.irIds) {
        if (id === irId) {
          return entry.astNodeId;
        }
      }
    }
    return -1;
  }
}

// ──────────────────────────────────────────────────────────────────────────────
// IrToIrEntry — one mapping from an original IR instruction to its
// replacement(s) after an optimiser pass
//
// Three cases:
//   1. Preserved:  original_id → [same_id]        (instruction unchanged)
//   2. Replaced:   original_id → [new_id_1, ...]  (instruction split/transformed)
//   3. Deleted:    original_id is in deleted set   (instruction optimised away)
//
// Example: A contraction pass folds three ADD_IMM 1 instructions
// (IDs 7, 8, 9) into one ADD_IMM 3 (ID 100):
//   7 → [100], 8 → [100], 9 → [100]
// ──────────────────────────────────────────────────────────────────────────────

export interface IrToIrEntry {
  readonly originalId: number;
  readonly newIds: readonly number[];
}

// ──────────────────────────────────────────────────────────────────────────────
// IrToIr — Segment 3: IR instruction IDs → optimised IR instruction IDs
//
// One segment is produced per optimiser pass. The passName field identifies
// which pass produced this mapping (e.g., "identity", "contraction",
// "clear_loop", "dead_store").
// ──────────────────────────────────────────────────────────────────────────────

export class IrToIr {
  public entries: IrToIrEntry[] = [];
  /** Set of original IDs that were optimised away (no replacement). */
  public deleted: Set<number> = new Set();
  /** Which optimiser pass produced this segment. */
  public readonly passName: string;

  constructor(passName: string) {
    this.passName = passName;
  }

  /**
   * Record that the original instruction was replaced by the new ones.
   *
   * @param originalId - The instruction ID before this pass.
   * @param newIds - The instruction IDs after this pass.
   */
  addMapping(originalId: number, newIds: number[]): void {
    this.entries.push({ originalId, newIds });
  }

  /**
   * Record that the original instruction was deleted (no replacement).
   * This happens when the optimiser removes dead code, no-ops, etc.
   */
  addDeletion(originalId: number): void {
    this.deleted.add(originalId);
    this.entries.push({ originalId, newIds: [] });
  }

  /**
   * Return the new IDs for the given original ID, or null if deleted or not found.
   */
  lookupByOriginalId(originalId: number): readonly number[] | null {
    if (this.deleted.has(originalId)) {
      return null;
    }
    for (const entry of this.entries) {
      if (entry.originalId === originalId) {
        return entry.newIds;
      }
    }
    return null;
  }

  /**
   * Return the original ID that produced the given new ID, or -1 if not found.
   * When multiple originals map to the same new ID (e.g., contraction),
   * returns the first match found.
   */
  lookupByNewId(newId: number): number {
    for (const entry of this.entries) {
      for (const id of entry.newIds) {
        if (id === newId) {
          return entry.originalId;
        }
      }
    }
    return -1;
  }
}

// ──────────────────────────────────────────────────────────────────────────────
// IrToMachineCodeEntry — one mapping from an IR instruction to the machine
// code bytes it produced
//
// Each entry is a triple: (ir_instruction_id, mc_byte_offset, mc_byte_length).
// For example, a LOAD_BYTE IR instruction might produce 8 bytes of RISC-V
// machine code starting at offset 0x14 in the .text section.
// ──────────────────────────────────────────────────────────────────────────────

export interface IrToMachineCodeEntry {
  /** IR instruction ID. */
  readonly irId: number;
  /** Byte offset in the .text section. */
  readonly mcOffset: number;
  /** Number of bytes of machine code. */
  readonly mcLength: number;
}

// ──────────────────────────────────────────────────────────────────────────────
// IrToMachineCode — Segment 4: IR instruction IDs → machine code byte offsets
// ──────────────────────────────────────────────────────────────────────────────

export class IrToMachineCode {
  public entries: IrToMachineCodeEntry[] = [];

  /**
   * Record that the given IR instruction produced machine code at the
   * given offset with the given length.
   */
  add(irId: number, mcOffset: number, mcLength: number): void {
    this.entries.push({ irId, mcOffset, mcLength });
  }

  /**
   * Return { offset, length } for the given IR instruction ID.
   * Returns { offset: -1, length: 0 } if not found.
   */
  lookupByIrId(irId: number): { offset: number; length: number } {
    for (const entry of this.entries) {
      if (entry.irId === irId) {
        return { offset: entry.mcOffset, length: entry.mcLength };
      }
    }
    return { offset: -1, length: 0 };
  }

  /**
   * Return the IR instruction ID whose machine code contains the given byte
   * offset, or -1 if not found.
   *
   * A machine code offset "contains" an IR instruction if:
   *   entry.mcOffset <= offset < entry.mcOffset + entry.mcLength
   *
   * This is used for reverse lookup: given a crash address, find the source line.
   */
  lookupByMCOffset(offset: number): number {
    for (const entry of this.entries) {
      if (offset >= entry.mcOffset && offset < entry.mcOffset + entry.mcLength) {
        return entry.irId;
      }
    }
    return -1;
  }
}

// ──────────────────────────────────────────────────────────────────────────────
// SourceMapChain — the full pipeline sidecar
//
// This is the central data structure that flows through every stage of the
// compiler pipeline. Each stage reads the existing segments and appends its own:
//
//   1. Frontend (brainfuck-ir-compiler) → fills sourceToAst + astToIr
//   2. Optimiser (compiler-ir-optimizer) → appends irToIr segments
//   3. Backend (codegen-riscv) → fills irToMachineCode
//
// ──────────────────────────────────────────────────────────────────────────────

export class SourceMapChain {
  public sourceToAst: SourceToAst;
  public astToIr: AstToIr;
  /** One IrToIr segment per optimiser pass. */
  public irToIr: IrToIr[] = [];
  /** Filled by the backend; null until then. */
  public irToMachineCode: IrToMachineCode | null = null;

  constructor() {
    this.sourceToAst = new SourceToAst();
    this.astToIr = new AstToIr();
  }

  /**
   * Append an IrToIr segment from an optimiser pass.
   */
  addOptimizerPass(segment: IrToIr): void {
    this.irToIr.push(segment);
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Composite queries — compose all segments for end-to-end lookups
  // ────────────────────────────────────────────────────────────────────────────

  /**
   * Compose all segments to look up the machine code offset(s) for a given
   * source position. Returns null if the chain is incomplete (no backend yet)
   * or no mapping exists.
   *
   * Algorithm:
   *   1. SourceToAst: source position → AST node ID
   *   2. AstToIr: AST node ID → IR instruction IDs
   *   3. IrToIr (each pass): follow IR IDs through each optimiser pass
   *   4. IrToMachineCode: final IR IDs → machine code offsets
   */
  sourceToMC(pos: SourcePosition): IrToMachineCodeEntry[] | null {
    if (this.irToMachineCode === null) {
      return null;
    }

    // Step 1: source → AST node
    let astNodeId = -1;
    for (const entry of this.sourceToAst.entries) {
      if (
        entry.pos.file === pos.file &&
        entry.pos.line === pos.line &&
        entry.pos.column === pos.column
      ) {
        astNodeId = entry.astNodeId;
        break;
      }
    }
    if (astNodeId === -1) return null;

    // Step 2: AST node → IR IDs
    const irIds = this.astToIr.lookupByAstNodeId(astNodeId);
    if (irIds === null) return null;

    // Step 3: follow through optimiser passes
    let currentIds: number[] = [...irIds];
    for (const pass of this.irToIr) {
      const nextIds: number[] = [];
      for (const id of currentIds) {
        if (pass.deleted.has(id)) continue; // optimised away
        const newIds = pass.lookupByOriginalId(id);
        if (newIds !== null) {
          nextIds.push(...newIds);
        }
      }
      currentIds = nextIds;
    }

    if (currentIds.length === 0) return null;

    // Step 4: IR IDs → machine code
    const results: IrToMachineCodeEntry[] = [];
    for (const id of currentIds) {
      const { offset, length } = this.irToMachineCode.lookupByIrId(id);
      if (offset >= 0) {
        results.push({ irId: id, mcOffset: offset, mcLength: length });
      }
    }
    return results;
  }

  /**
   * Compose all segments in reverse to look up the source position for a
   * given machine code offset. Returns null if the chain is incomplete or
   * no mapping exists.
   *
   * Algorithm (reverse of sourceToMC):
   *   1. IrToMachineCode: MC offset → IR instruction ID
   *   2. IrToIr (each pass, in reverse): follow IR ID back through passes
   *   3. AstToIr: IR ID → AST node ID
   *   4. SourceToAst: AST node ID → source position
   */
  mcToSource(mcOffset: number): SourcePosition | null {
    if (this.irToMachineCode === null) {
      return null;
    }

    // Step 1: MC offset → IR ID
    const irId = this.irToMachineCode.lookupByMCOffset(mcOffset);
    if (irId === -1) return null;

    // Step 2: follow back through optimiser passes (in reverse order)
    let currentId = irId;
    for (let i = this.irToIr.length - 1; i >= 0; i--) {
      const pass = this.irToIr[i];
      const originalId = pass.lookupByNewId(currentId);
      if (originalId === -1) return null;
      currentId = originalId;
    }

    // Step 3: IR ID → AST node ID
    const astNodeId = this.astToIr.lookupByIrId(currentId);
    if (astNodeId === -1) return null;

    // Step 4: AST node ID → source position
    return this.sourceToAst.lookupByNodeId(astNodeId);
  }
}
