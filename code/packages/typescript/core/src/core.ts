/**
 * Core -- a complete processor core composing all D-series sub-components.
 *
 * The Core wires together: Pipeline (D04), Branch Predictor (D02),
 * Hazard Unit (D03), Cache Hierarchy (D01), Register File, Clock,
 * and Memory Controller.
 *
 * The Core itself defines no new micro-architectural behavior. It wires
 * the parts together, like a motherboard connects CPU, RAM, and peripherals.
 */

import {
  type BranchPredictor,
  AlwaysTakenPredictor,
  AlwaysNotTakenPredictor,
  BackwardTakenForwardNotTaken,
  BranchTargetBuffer,
  OneBitPredictor,
  TwoBitPredictor,
  TwoBitState,
} from "@coding-adventures/branch-predictor";
import {
  Cache,
  CacheConfig,
  CacheHierarchy,
} from "@coding-adventures/cache";
import { Clock } from "@coding-adventures/clock";
import {
  type HazardResponse,
  type PipelineSnapshot,
  type PipelineToken,
  HazardAction as PipelineHazardAction,
  Pipeline,
  StageCategory,
  classic5Stage,
} from "@coding-adventures/cpu-pipeline";
import {
  HazardAction as HazDetAction,
  HazardUnit,
  PipelineSlot,
} from "@coding-adventures/hazard-detection";

import type { CoreConfig } from "./config.js";
import type { ISADecoder } from "./decoder.js";
import { MemoryController } from "./memory-controller.js";
import { RegisterFile } from "./register-file.js";
import { CoreStats } from "./stats.js";

// =========================================================================
// Branch predictor factory
// =========================================================================

function createBranchPredictor(typ: string, size: number): BranchPredictor {
  switch (typ) {
    case "static_always_taken":
      return new AlwaysTakenPredictor();
    case "static_always_not_taken":
      return new AlwaysNotTakenPredictor();
    case "static_btfnt":
      return new BackwardTakenForwardNotTaken();
    case "one_bit":
      return new OneBitPredictor(size);
    case "two_bit":
      return new TwoBitPredictor(size, TwoBitState.WeaklyNotTaken);
    default:
      return new AlwaysNotTakenPredictor();
  }
}

// =========================================================================
// Core class
// =========================================================================

export class Core {
  private _config: CoreConfig;
  private _decoder: ISADecoder;
  private _pipeline: Pipeline;
  private _predictor: BranchPredictor;
  private _btb: BranchTargetBuffer;
  private _hazardUnit: HazardUnit;
  private _cacheHierarchy: CacheHierarchy;
  private _regFile: RegisterFile;
  private _memCtrl: MemoryController;
  private _clk: Clock;
  private _halted: boolean = false;
  private _cycle: number = 0;
  private _instructionsCompleted: number = 0;
  private _forwardCount: number = 0;
  private _stallCount: number = 0;
  private _flushCount: number = 0;

  private constructor(
    config: CoreConfig,
    decoder: ISADecoder,
    pipeline: Pipeline,
    predictor: BranchPredictor,
    btb: BranchTargetBuffer,
    hazardUnit: HazardUnit,
    cacheHierarchy: CacheHierarchy,
    regFile: RegisterFile,
    memCtrl: MemoryController,
    clk: Clock,
  ) {
    this._config = config;
    this._decoder = decoder;
    this._pipeline = pipeline;
    this._predictor = predictor;
    this._btb = btb;
    this._hazardUnit = hazardUnit;
    this._cacheHierarchy = cacheHierarchy;
    this._regFile = regFile;
    this._memCtrl = memCtrl;
    this._clk = clk;
  }

  /**
   * Creates a fully-wired processor core from config and ISA decoder.
   */
  static create(config: CoreConfig, decoder: ISADecoder): Core {
    // 1. Register File
    const regFile = new RegisterFile(config.registerFile);

    // 2. Memory
    let memSize = config.memorySize;
    if (memSize <= 0) memSize = 65536;
    const memory = new Uint8Array(memSize);
    let memLatency = config.memoryLatency;
    if (memLatency <= 0) memLatency = 100;
    const memCtrl = new MemoryController(memory, memLatency);

    // 3. Cache Hierarchy
    const cacheHierarchy = buildCacheHierarchy(config, memLatency);

    // 4. Branch Predictor + BTB
    const predictor = createBranchPredictor(config.branchPredictorType, config.branchPredictorSize);
    let btbSize = config.btbSize;
    if (btbSize <= 0) btbSize = 64;
    const btb = new BranchTargetBuffer(btbSize);

    // 5. Hazard Unit
    const numFpUnits = config.fpUnit ? 1 : 0;
    const hazardUnit = new HazardUnit({ numAlus: 1, numFpUnits, splitCaches: true });

    // 6. Pipeline
    let pipelineConfig = config.pipeline;
    if (pipelineConfig.stages.length === 0) {
      pipelineConfig = classic5Stage();
    }

    // Create core first (partially), then wire callbacks.
    // We use a closure approach: create core, then set up the pipeline.
    const clk = new Clock(1000000000); // 1 GHz nominal

    // We need to create a core object to use in callbacks.
    // Create pipeline with callbacks that close over the core.
    let core: Core;

    const fetchCb = (pc: number): number => {
      core._cacheHierarchy.read(pc, true, core._cycle);
      return core._memCtrl.readWord(pc);
    };

    const decodeCb = (raw: number, token: PipelineToken): PipelineToken => {
      return core._decoder.decode(raw, token);
    };

    const executeCb = (token: PipelineToken): PipelineToken => {
      const result = core._decoder.execute(token, core._regFile);
      if (result.isBranch) {
        core._predictor.update(result.pc, result.branchTaken, result.branchTarget);
        if (result.branchTaken) {
          core._btb.update(result.pc, result.branchTarget, "conditional");
        }
      }
      return result;
    };

    const memoryCb = (token: PipelineToken): PipelineToken => {
      if (token.memRead) {
        core._cacheHierarchy.read(token.aluResult, false, core._cycle);
        token.memData = core._memCtrl.readWord(token.aluResult);
        token.writeData = token.memData;
      } else if (token.memWrite) {
        core._cacheHierarchy.write(token.aluResult, [token.writeData & 0xff], core._cycle);
        core._memCtrl.writeWord(token.aluResult, token.writeData);
      }
      return token;
    };

    const writebackCb = (token: PipelineToken): void => {
      if (token.regWrite && token.rd >= 0) {
        core._regFile.write(token.rd, token.writeData);
      }
    };

    const pipeline = Pipeline.create(
      pipelineConfig,
      fetchCb,
      decodeCb,
      executeCb,
      memoryCb,
      writebackCb,
    );

    // Wire optional callbacks.
    if (config.hazardDetection) {
      pipeline.setHazardFunc((stages: (PipelineToken | null)[]): HazardResponse => {
        return core.hazardCallback(stages);
      });
    }

    pipeline.setPredictFunc((pc: number): number => {
      return core.predictCallback(pc);
    });

    core = new Core(
      config, decoder, pipeline, predictor, btb, hazardUnit,
      cacheHierarchy, regFile, memCtrl, clk,
    );

    return core;
  }

  // =========================================================================
  // Pipeline Callbacks
  // =========================================================================

  private hazardCallback(stages: (PipelineToken | null)[]): HazardResponse {
    const numStages = stages.length;
    let pipelineCfg = this._config.pipeline;
    if (pipelineCfg.stages.length === 0) {
      pipelineCfg = classic5Stage();
    }

    // Find IF, ID, EX, MEM tokens by category.
    let ifTok: PipelineToken | null = null;
    let idTok: PipelineToken | null = null;
    let exTok: PipelineToken | null = null;
    let memTok: PipelineToken | null = null;

    for (let i = 0; i < pipelineCfg.stages.length && i < numStages; i++) {
      const tok = stages[i];
      switch (pipelineCfg.stages[i].category) {
        case StageCategory.Fetch:
          if (ifTok === null) ifTok = tok;
          break;
        case StageCategory.Decode:
          idTok = tok; // Use the LAST decode stage.
          break;
        case StageCategory.Execute:
          if (exTok === null) exTok = tok;
          break;
        case StageCategory.Memory:
          if (memTok === null) memTok = tok;
          break;
      }
    }

    const ifSlot = tokenToSlot(ifTok);
    const idSlot = tokenToSlot(idTok);
    const exSlot = tokenToSlot(exTok);
    const memSlot = tokenToSlot(memTok);

    const result = this._hazardUnit.check(ifSlot, idSlot, exSlot, memSlot);

    const response: HazardResponse = {
      action: PipelineHazardAction.None,
      forwardValue: 0,
      forwardSource: "",
      stallStages: 0,
      flushCount: 0,
      redirectPC: 0,
    };

    switch (result.action) {
      case HazDetAction.STALL:
        response.action = PipelineHazardAction.Stall;
        response.stallStages = result.stallCycles;
        this._stallCount++;
        break;

      case HazDetAction.FLUSH:
        response.action = PipelineHazardAction.Flush;
        response.flushCount = result.flushCount;
        if (exTok !== null && exTok.isBranch) {
          if (exTok.branchTaken) {
            response.redirectPC = exTok.branchTarget;
          } else {
            response.redirectPC = exTok.pc + this._decoder.instructionSize();
          }
        }
        this._flushCount++;
        break;

      case HazDetAction.FORWARD_FROM_EX:
        response.action = PipelineHazardAction.ForwardFromEX;
        if (result.forwardedValue !== null) {
          response.forwardValue = result.forwardedValue;
        }
        response.forwardSource = result.forwardedFrom;
        this._forwardCount++;
        break;

      case HazDetAction.FORWARD_FROM_MEM:
        response.action = PipelineHazardAction.ForwardFromMEM;
        if (result.forwardedValue !== null) {
          response.forwardValue = result.forwardedValue;
        }
        response.forwardSource = result.forwardedFrom;
        this._forwardCount++;
        break;
    }

    return response;
  }

  private predictCallback(pc: number): number {
    const prediction = this._predictor.predict(pc);
    const instrSize = this._decoder.instructionSize();

    if (prediction.taken) {
      const target = this._btb.lookup(pc);
      if (target !== null) {
        return target;
      }
    }

    return pc + instrSize;
  }

  // =========================================================================
  // Public API
  // =========================================================================

  /** Loads machine code into memory starting at the given address. */
  loadProgram(program: Uint8Array, startAddress: number): void {
    this._memCtrl.loadProgram(program, startAddress);
    this._pipeline.setPC(startAddress);
  }

  /** Executes one clock cycle. Returns the pipeline snapshot. */
  step(): PipelineSnapshot {
    if (this._halted) {
      return this._pipeline.snapshot();
    }

    this._cycle++;
    const snap = this._pipeline.step();

    if (this._pipeline.isHalted()) {
      this._halted = true;
    }

    this._instructionsCompleted = this._pipeline.stats().instructionsCompleted;

    return snap;
  }

  /** Runs until halt or maxCycles. Returns aggregate statistics. */
  run(maxCycles: number): CoreStats {
    while (this._cycle < maxCycles && !this._halted) {
      this.step();
    }
    return this.stats();
  }

  /** Returns aggregate statistics from all sub-components. */
  stats(): CoreStats {
    const pStats = this._pipeline.stats();
    const s = new CoreStats();
    s.instructionsCompleted = pStats.instructionsCompleted;
    s.totalCycles = pStats.totalCycles;
    s.pipelineStats = pStats;
    s.predictorStats = this._predictor.stats;
    s.cacheStats = {};

    if (this._cacheHierarchy.l1i) {
      s.cacheStats["L1I"] = this._cacheHierarchy.l1i.stats;
    }
    if (this._cacheHierarchy.l1d) {
      s.cacheStats["L1D"] = this._cacheHierarchy.l1d.stats;
    }
    if (this._cacheHierarchy.l2) {
      s.cacheStats["L2"] = this._cacheHierarchy.l2.stats;
    }

    s.forwardCount = this._forwardCount;
    s.stallCount = this._stallCount;
    s.flushCount = this._flushCount;

    return s;
  }

  isHalted(): boolean { return this._halted; }
  readRegister(index: number): number { return this._regFile.read(index); }
  writeRegister(index: number, value: number): void { this._regFile.write(index, value); }
  registerFile(): RegisterFile { return this._regFile; }
  memoryController(): MemoryController { return this._memCtrl; }
  cycle(): number { return this._cycle; }
  getConfig(): CoreConfig { return this._config; }
  pipeline(): Pipeline { return this._pipeline; }
  predictor(): BranchPredictor { return this._predictor; }
  cacheHierarchy(): CacheHierarchy { return this._cacheHierarchy; }

  /**
   * Allows replacing the memory controller (used by MultiCoreCPU to share memory).
   * @internal
   */
  _setMemCtrl(mc: MemoryController): void { this._memCtrl = mc; }
}

// =========================================================================
// Helpers
// =========================================================================

function buildCacheHierarchy(config: CoreConfig, memLatency: number): CacheHierarchy {
  let l1i: Cache | null = null;
  let l1d: Cache | null = null;
  let l2: Cache | null = null;

  if (config.l1iCache) {
    l1i = new Cache(config.l1iCache);
  } else {
    l1i = new Cache(new CacheConfig("L1I", 4096, 64, 1, 1));
  }

  if (config.l1dCache) {
    l1d = new Cache(config.l1dCache);
  } else {
    l1d = new Cache(new CacheConfig("L1D", 4096, 64, 1, 1));
  }

  if (config.l2Cache) {
    l2 = new Cache(config.l2Cache);
  }

  return new CacheHierarchy({ l1i, l1d, l2, mainMemoryLatency: memLatency });
}

function tokenToSlot(tok: PipelineToken | null): PipelineSlot {
  if (tok === null || tok.isBubble) {
    return new PipelineSlot({ valid: false });
  }

  const sourceRegs: number[] = [];
  if (tok.rs1 >= 0) sourceRegs.push(tok.rs1);
  if (tok.rs2 >= 0) sourceRegs.push(tok.rs2);

  let destReg: number | null = null;
  let destValue: number | null = null;
  if (tok.rd >= 0 && tok.regWrite) {
    destReg = tok.rd;
    if (tok.aluResult !== 0 || tok.writeData !== 0) {
      destValue = tok.writeData !== 0 ? tok.writeData : tok.aluResult;
    }
  }

  return new PipelineSlot({
    valid: true,
    pc: tok.pc,
    sourceRegs,
    destReg,
    destValue,
    isBranch: tok.isBranch,
    branchTaken: tok.branchTaken,
    branchPredictedTaken: false,
    memRead: tok.memRead,
    memWrite: tok.memWrite,
    usesAlu: true,
  });
}
