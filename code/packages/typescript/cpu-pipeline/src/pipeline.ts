/**
 * Pipeline -- the main pipeline simulator.
 *
 * The pipeline is a configurable N-stage instruction pipeline. It manages
 * the FLOW of instructions through stages, handling:
 *
 * - Normal advancement: tokens move one stage per clock cycle
 * - Stalls: freeze earlier stages and insert a "bubble" (NOP)
 * - Flushes: replace speculative instructions with bubbles
 * - Statistics: track IPC, stall cycles, flush cycles
 *
 * The actual work of each stage (fetching, decoding, executing, etc.)
 * is performed by callback functions injected from the CPU core.
 */

import type { PipelineSnapshot } from "./snapshot.js";
import { PipelineStats } from "./snapshot.js";
import {
  type DecodeFunc,
  type ExecuteFunc,
  type FetchFunc,
  HazardAction,
  type HazardFunc,
  type HazardResponse,
  type MemoryFunc,
  type PipelineConfig,
  type PipelineToken,
  type PredictFunc,
  StageCategory,
  type WritebackFunc,
  cloneToken,
  newBubble,
  newToken,
  noHazard,
  validateConfig,
} from "./token.js";

// =========================================================================
// Pipeline class
// =========================================================================

/**
 * Pipeline is a configurable N-stage instruction pipeline.
 *
 * # How it Works
 *
 * The pipeline is a slice of "slots", one per stage. Each slot holds a
 * PipelineToken (or null if the stage is empty). On each clock cycle
 * (call to step()):
 *
 *   1. Check for hazards (via HazardFunc callback)
 *   2. If stalled: freeze stages before the stall point, insert bubble
 *   3. If flushing: replace speculative stages with bubbles
 *   4. Otherwise: shift all tokens one stage forward
 *   5. Execute stage callbacks (fetch, decode, execute, memory, writeback)
 *   6. Record a snapshot for tracing
 *
 * # Example: 5-cycle execution of ADD instruction
 *
 *     Cycle 1: IF  -- fetch instruction at PC, ask branch predictor for next PC
 *     Cycle 2: ID  -- decode: extract opcode=ADD, Rd=1, Rs1=2, Rs2=3
 *     Cycle 3: EX  -- execute: ALUResult = Reg[2] + Reg[3]
 *     Cycle 4: MEM -- memory: pass through (ADD doesn't access memory)
 *     Cycle 5: WB  -- writeback: Reg[1] = ALUResult
 */
export class Pipeline {
  /** Pipeline configuration (stages, width). */
  private _config: PipelineConfig;

  /**
   * Current token in each pipeline stage.
   * stages[0] is the first stage (IF), stages[N-1] is the last (WB).
   * A null entry means the stage is empty.
   */
  private _stages: (PipelineToken | null)[];

  /** Current program counter (address of next instruction to fetch). */
  private _pc: number = 0;

  /** Current clock cycle number (starts at 0, incremented by step). */
  private _cycle: number = 0;

  /** True if a halt instruction has reached the last stage. */
  private _halted: boolean = false;

  /** Execution statistics. */
  private _stats: PipelineStats = new PipelineStats();

  /** History of snapshots, one per cycle. */
  private _history: PipelineSnapshot[] = [];

  // --- Callbacks ---
  private _fetchFn: FetchFunc;
  private _decodeFn: DecodeFunc;
  private _executeFn: ExecuteFunc;
  private _memoryFn: MemoryFunc;
  private _writebackFn: WritebackFunc;
  private _hazardFn: HazardFunc | null = null;
  private _predictFn: PredictFunc | null = null;

  /**
   * Private constructor -- use Pipeline.create() instead.
   */
  private constructor(
    config: PipelineConfig,
    fetch: FetchFunc,
    decode: DecodeFunc,
    execute: ExecuteFunc,
    memory: MemoryFunc,
    writeback: WritebackFunc,
  ) {
    this._config = config;
    this._stages = new Array(config.stages.length).fill(null);
    this._fetchFn = fetch;
    this._decodeFn = decode;
    this._executeFn = execute;
    this._memoryFn = memory;
    this._writebackFn = writeback;
  }

  /**
   * Creates a new pipeline with the given configuration and callbacks.
   *
   * Returns the pipeline or throws an error if the configuration is invalid.
   */
  static create(
    config: PipelineConfig,
    fetch: FetchFunc,
    decode: DecodeFunc,
    execute: ExecuteFunc,
    memory: MemoryFunc,
    writeback: WritebackFunc,
  ): Pipeline {
    const error = validateConfig(config);
    if (error !== null) {
      throw new Error(error);
    }
    return new Pipeline(config, fetch, decode, execute, memory, writeback);
  }

  /**
   * Sets the optional hazard detection callback.
   */
  setHazardFunc(fn: HazardFunc): void {
    this._hazardFn = fn;
  }

  /**
   * Sets the optional branch prediction callback.
   */
  setPredictFunc(fn: PredictFunc): void {
    this._predictFn = fn;
  }

  /** Sets the program counter. */
  setPC(pc: number): void {
    this._pc = pc;
  }

  /** Returns the current program counter. */
  pc(): number {
    return this._pc;
  }

  /**
   * Advances the pipeline by one clock cycle.
   *
   * This is the heart of the pipeline simulator. Each call corresponds
   * to one rising clock edge in hardware.
   *
   * Returns the pipeline snapshot for this cycle.
   */
  step(): PipelineSnapshot {
    if (this._halted) {
      return this.takeSnapshot();
    }

    this._cycle++;
    this._stats.totalCycles++;
    const stageCount = this._config.stages.length;

    // --- Phase 1: Check for hazards ---
    let hazard: HazardResponse = noHazard();
    if (this._hazardFn !== null) {
      const stagesCopy = [...this._stages];
      hazard = this._hazardFn(stagesCopy);
    }

    // --- Phase 2: Compute next state ---
    const nextStages: (PipelineToken | null)[] = new Array(stageCount).fill(null);
    let stalled = false;
    let flushing = false;

    switch (hazard.action) {

      case HazardAction.Flush: {
        // FLUSH: Replace speculative stages with bubbles and redirect PC.
        flushing = true;
        this._stats.flushCycles++;

        // Determine how many stages to flush (from the front).
        let flushCount = hazard.flushCount;
        if (flushCount <= 0) {
          for (let i = 0; i < this._config.stages.length; i++) {
            if (this._config.stages[i].category === StageCategory.Execute) {
              flushCount = i;
              break;
            }
          }
          if (flushCount <= 0) flushCount = 1;
        }
        if (flushCount > stageCount) flushCount = stageCount;

        // Shift non-flushed stages forward.
        for (let i = stageCount - 1; i >= flushCount; i--) {
          if (i > 0 && i - 1 >= flushCount) {
            nextStages[i] = this._stages[i - 1];
          } else if (i > 0) {
            const bubble = newBubble();
            bubble.stageEntered[this._config.stages[i].name] = this._cycle;
            nextStages[i] = bubble;
          } else {
            nextStages[i] = this._stages[i];
          }
        }

        // Replace flushed stages with bubbles.
        for (let i = 0; i < flushCount; i++) {
          const bubble = newBubble();
          bubble.stageEntered[this._config.stages[i].name] = this._cycle;
          nextStages[i] = bubble;
        }

        // Redirect PC and fetch from the correct target.
        this._pc = hazard.redirectPC;
        const tok = this.fetchNewInstruction();
        nextStages[0] = tok;
        break;
      }

      case HazardAction.Stall: {
        // STALL: Freeze earlier stages and insert a bubble.
        stalled = true;
        this._stats.stallCycles++;

        // Find the stall insertion point.
        let stallPoint = hazard.stallStages;
        if (stallPoint <= 0) {
          for (let i = 0; i < this._config.stages.length; i++) {
            if (this._config.stages[i].category === StageCategory.Execute) {
              stallPoint = i;
              break;
            }
          }
          if (stallPoint <= 0) stallPoint = 1;
        }
        if (stallPoint >= stageCount) stallPoint = stageCount - 1;

        // Stages AFTER the stall point advance normally.
        for (let i = stageCount - 1; i > stallPoint; i--) {
          nextStages[i] = this._stages[i - 1];
        }

        // Insert bubble at the stall point.
        const bubble = newBubble();
        bubble.stageEntered[this._config.stages[stallPoint].name] = this._cycle;
        nextStages[stallPoint] = bubble;

        // Stages BEFORE the stall point are frozen.
        for (let i = 0; i < stallPoint; i++) {
          nextStages[i] = this._stages[i];
        }

        // PC does NOT advance during a stall.
        break;
      }

      default: {
        // NONE or FORWARD: Normal advancement.

        // Handle forwarding if needed.
        if (hazard.action === HazardAction.ForwardFromEX || hazard.action === HazardAction.ForwardFromMEM) {
          for (let i = 0; i < this._config.stages.length; i++) {
            const s = this._config.stages[i];
            const tok = this._stages[i];
            if (s.category === StageCategory.Decode && tok !== null && !tok.isBubble) {
              tok.aluResult = hazard.forwardValue;
              tok.forwardedFrom = hazard.forwardSource;
              break;
            }
          }
        }

        // Shift tokens forward (from back to front).
        for (let i = stageCount - 1; i > 0; i--) {
          nextStages[i] = this._stages[i - 1];
        }

        // Fetch new instruction into IF stage.
        const tok = this.fetchNewInstruction();
        nextStages[0] = tok;
        break;
      }
    }

    // --- Phase 3: Commit the new state ---
    this._stages = nextStages;

    // --- Phase 4: Execute stage callbacks ---
    for (let i = stageCount - 1; i >= 0; i--) {
      const tok = this._stages[i];
      if (tok === null || tok.isBubble) continue;

      const stage = this._config.stages[i];

      // Record when this token entered this stage.
      if (!(stage.name in tok.stageEntered)) {
        tok.stageEntered[stage.name] = this._cycle;
      }

      switch (stage.category) {
        case StageCategory.Fetch:
          // Already handled by fetchNewInstruction().
          break;

        case StageCategory.Decode:
          if (tok.opcode === "") {
            this._stages[i] = this._decodeFn(tok.rawInstruction, tok);
          }
          break;

        case StageCategory.Execute:
          if (tok.stageEntered[stage.name] === this._cycle) {
            this._stages[i] = this._executeFn(tok);
          }
          break;

        case StageCategory.Memory:
          if (tok.stageEntered[stage.name] === this._cycle) {
            this._stages[i] = this._memoryFn(tok);
          }
          break;

        case StageCategory.Writeback:
          // Writeback is handled in Phase 5 (retirement).
          break;
      }
    }

    // --- Phase 5: Retire the instruction in the last stage ---
    const lastTok = this._stages[stageCount - 1];
    if (lastTok !== null && !lastTok.isBubble) {
      this._writebackFn(lastTok);
      this._stats.instructionsCompleted++;
      if (lastTok.isHalt) {
        this._halted = true;
      }
    }

    // Count bubbles across all stages.
    for (const tok of this._stages) {
      if (tok !== null && tok.isBubble) {
        this._stats.bubbleCycles++;
      }
    }

    // --- Phase 6: Take snapshot ---
    const snap: PipelineSnapshot = {
      cycle: this._cycle,
      stages: {},
      stalled,
      flushing,
      pc: this._pc,
    };
    for (let i = 0; i < this._config.stages.length; i++) {
      const tok = this._stages[i];
      if (tok !== null) {
        const cloned = cloneToken(tok);
        if (cloned !== null) {
          snap.stages[this._config.stages[i].name] = cloned;
        }
      }
    }
    this._history.push(snap);

    return snap;
  }

  /**
   * Creates a new token by calling the fetch callback.
   */
  private fetchNewInstruction(): PipelineToken {
    const tok = newToken();
    tok.pc = this._pc;
    tok.rawInstruction = this._fetchFn(this._pc);
    tok.stageEntered[this._config.stages[0].name] = this._cycle;

    // Advance PC: use branch predictor if available, otherwise PC+4.
    if (this._predictFn !== null) {
      this._pc = this._predictFn(this._pc);
    } else {
      this._pc += 4;
    }

    return tok;
  }

  /**
   * Runs the pipeline until a halt instruction is encountered or
   * the maximum cycle count is reached.
   *
   * Returns the final execution statistics.
   */
  run(maxCycles: number): PipelineStats {
    while (this._cycle < maxCycles && !this._halted) {
      this.step();
    }
    return this._stats;
  }

  /**
   * Returns the current pipeline state without advancing the clock.
   */
  snapshot(): PipelineSnapshot {
    return this.takeSnapshot();
  }

  /** Returns a copy of the current execution statistics. */
  stats(): PipelineStats {
    // Return the same stats object -- callers can read but should not modify.
    return this._stats;
  }

  /** Returns true if a halt instruction has reached the last stage. */
  isHalted(): boolean {
    return this._halted;
  }

  /** Returns the current cycle number. */
  cycle(): number {
    return this._cycle;
  }

  /** Returns the complete history of pipeline snapshots. */
  trace(): PipelineSnapshot[] {
    return [...this._history];
  }

  /**
   * Returns the token currently occupying the given stage.
   * Returns null if the stage is empty or the name is invalid.
   */
  stageContents(stageName: string): PipelineToken | null {
    for (let i = 0; i < this._config.stages.length; i++) {
      if (this._config.stages[i].name === stageName) {
        return this._stages[i];
      }
    }
    return null;
  }

  /** Returns the pipeline configuration. */
  config(): PipelineConfig {
    return this._config;
  }

  /**
   * Creates a snapshot of the current pipeline state.
   */
  private takeSnapshot(): PipelineSnapshot {
    const snap: PipelineSnapshot = {
      cycle: this._cycle,
      stages: {},
      stalled: false,
      flushing: false,
      pc: this._pc,
    };
    for (let i = 0; i < this._config.stages.length; i++) {
      const tok = this._stages[i];
      if (tok !== null) {
        const cloned = cloneToken(tok);
        if (cloned !== null) {
          snap.stages[this._config.stages[i].name] = cloned;
        }
      }
    }
    return snap;
  }
}
