/**
 * Pipelined floating-point arithmetic -- the bridge to GPU architecture.
 *
 * === Why Pipelining? ===
 *
 * Imagine a car factory with a single worker who does everything: welds the
 * frame, installs the engine, paints the body, mounts the wheels, inspects
 * the result. One car takes 5 hours. Want 100 cars? That's 500 hours.
 *
 * Now imagine a factory with 5 stations, each doing one step. The first car
 * still takes 5 hours to pass through all 5 stations. But while it moves to
 * station 2, a NEW car enters station 1. After the initial 5-hour fill-up
 * time, a finished car rolls off the line every HOUR -- 5x throughput!
 *
 * This is pipelining, and it's exactly how GPUs achieve massive throughput.
 *
 * === Latency vs Throughput ===
 *
 * These two concepts are often confused, but they're fundamentally different:
 *
 *     Latency:     Time for ONE operation to complete start-to-finish.
 *     Throughput:  How many operations complete per unit time.
 *
 * For a 5-stage pipeline:
 *
 *     Latency = 5 clock cycles (one operation still takes 5 cycles)
 *     Throughput = 1 result per clock cycle (after pipeline fills up)
 *
 *     Without pipeline:   Latency=5, Throughput=1/5
 *     With pipeline:      Latency=5, Throughput=1/1   <-- 5x better!
 *
 * This is the key insight: pipelining does NOT make individual operations
 * faster (same latency), but it makes the system process MORE operations
 * per second (higher throughput).
 *
 * === Pipeline Timing Diagram ===
 *
 * Here's what happens when we submit 4 additions (A, B, C, D) to a
 * 5-stage pipelined adder:
 *
 *     Clock:  1    2    3    4    5    6    7    8
 *     ----------------------------------------
 *     Stage1: [A1] [B1] [C1] [D1]  -    -    -    -
 *     Stage2:  -   [A2] [B2] [C2] [D2]  -    -    -
 *     Stage3:  -    -   [A3] [B3] [C3] [D3]  -    -
 *     Stage4:  -    -    -   [A4] [B4] [C4] [D4]  -
 *     Stage5:  -    -    -    -   [A5] [B5] [C5] [D5]
 *                                  ^    ^    ^    ^
 *                               Result Result Result Result
 *                               for A  for B  for C  for D
 *
 *     - A enters stage 1 at clock 1, exits stage 5 at clock 5 (latency = 5)
 *     - After clock 5, results come out every cycle (throughput = 1/cycle)
 *     - All 4 results done by clock 8 instead of clock 20 (without pipeline)
 *
 * === How This Connects to GPUs ===
 *
 * A modern GPU has thousands of "CUDA cores" (NVIDIA) or "shader processors"
 * (AMD), and each one contains pipelined FP units. A typical GPU core has:
 *
 *     - Pipelined FP32 adder (4-6 stages)
 *     - Pipelined FP32 multiplier (3-5 stages)
 *     - Pipelined FMA unit (6-8 stages)
 *
 * With 5000 cores each running pipelined FP, the GPU can sustain:
 *     5000 cores x 1 result/cycle x 1.5 GHz = 7.5 TFLOPS
 *
 * This is why GPUs dominate machine learning: the dot products in matrix
 * multiplication map perfectly to pipelined FMA units.
 *
 * === Clock Simulation ===
 *
 * Since we don't have the Python clock package in TypeScript, our pipeline
 * uses a simple tick-based simulation. Each call to tick() advances the
 * pipeline by one stage, equivalent to one rising clock edge. This mirrors
 * how hardware pipeline registers capture data on clock edges.
 */

import {
  type FloatBits,
  type FloatFormat,
  FP32,
} from "./formats.js";
import { fpAdd } from "./fp-adder.js";
import { fpMul } from "./fp-multiplier.js";
import { fpFma } from "./fma.js";

// ---------------------------------------------------------------------------
// PipelinedFPAdder -- 5-stage pipelined floating-point adder
// ---------------------------------------------------------------------------

/**
 * A 5-stage pipelined floating-point adder.
 *
 * In real GPU hardware, the FP adder is pipelined so that while one
 * addition is being normalized (stage 4), a newer addition is being
 * aligned (stage 2), and an even newer one is being unpacked (stage 1).
 *
 * === Pipeline Stages ===
 *
 *     Stage 1: UNPACK
 *         Extract sign, exponent, and mantissa from both operands.
 *         Handle special cases (NaN, Inf, zero).
 *
 *     Stage 2: ALIGN
 *         Compare exponents and shift the smaller mantissa right.
 *
 *     Stage 3: ADD/SUB
 *         Perform mantissa addition or subtraction.
 *
 *     Stage 4: NORMALIZE
 *         Shift the result so the leading 1 is correct.
 *
 *     Stage 5: ROUND & PACK
 *         Apply IEEE 754 round-to-nearest-even and pack result.
 *
 * In this educational implementation, we simplify by computing the full
 * fpAdd result at submission time and flowing it through pipeline stages.
 * The key educational point is the pipeline TIMING, not the per-stage logic.
 *
 * === Usage ===
 *
 *     const adder = new PipelinedFPAdder();
 *     adder.submit(floatToBits(1.5), floatToBits(2.5));
 *     // Tick 5 full cycles for the result to emerge
 *     for (let i = 0; i < 5; i++) adder.tick();
 *     assert(adder.results.length === 1);
 *     bitsToFloat(adder.results[0])  // 4.0
 */
export class PipelinedFPAdder {
  /** Number of pipeline stages -- this is the latency in clock cycles. */
  static readonly NUM_STAGES = 5;

  readonly fmt: FloatFormat;

  /** Pipeline stage registers: each slot holds intermediate data or null if idle. */
  private _stages: (FloatBits | null)[];

  /** Input queue: operand pairs waiting to enter the pipeline. */
  private _inputsPending: [FloatBits, FloatBits][] = [];

  /** Completed results that have exited the pipeline. */
  results: FloatBits[] = [];

  /** How many rising clock edges we've seen. */
  cycleCount: number = 0;

  constructor(fmt: FloatFormat = FP32) {
    this.fmt = fmt;
    this._stages = new Array(PipelinedFPAdder.NUM_STAGES).fill(null);
  }

  /**
   * Submit a new addition to the pipeline.
   *
   * The operands are queued and will enter stage 1 on the next tick.
   */
  submit(a: FloatBits, b: FloatBits): void {
    this._inputsPending.push([a, b]);
  }

  /**
   * Advance the pipeline by one clock cycle (rising edge).
   *
   * On every tick:
   *   1. Collect output from the last stage (if any)
   *   2. Shift all stages forward
   *   3. Load new input into stage 1 (if any is pending)
   */
  tick(): void {
    this.cycleCount += 1;

    // Shift pipeline forward (from end to start to avoid overwrites)
    for (let i = PipelinedFPAdder.NUM_STAGES - 1; i > 0; i--) {
      this._stages[i] = this._stages[i - 1];
    }

    // Load new input into stage 0
    if (this._inputsPending.length > 0) {
      const [a, b] = this._inputsPending.shift()!;
      // Compute the result eagerly and flow it through the pipeline.
      // In real hardware, each stage would do partial work.
      this._stages[0] = fpAdd(a, b);
    } else {
      this._stages[0] = null;
    }

    // Collect output from the last stage
    const lastStage = PipelinedFPAdder.NUM_STAGES - 1;
    if (this._stages[lastStage] !== null) {
      this.results.push(this._stages[lastStage]!);
      this._stages[lastStage] = null;
    }
  }
}

// ---------------------------------------------------------------------------
// PipelinedFPMultiplier -- 4-stage pipelined floating-point multiplier
// ---------------------------------------------------------------------------

/**
 * A 4-stage pipelined floating-point multiplier.
 *
 * Multiplication is simpler than addition because there's no alignment
 * step -- the exponents simply add and the mantissas multiply. This means
 * the multiplier pipeline has fewer stages (4 vs 5 for the adder).
 *
 * === Pipeline Stages ===
 *
 *     Stage 1: UNPACK + SIGN + EXPONENT
 *         Extract fields, XOR signs, add exponents, subtract bias.
 *
 *     Stage 2: MULTIPLY MANTISSAS
 *         24x24 bit multiplier producing a 48-bit product.
 *
 *     Stage 3: NORMALIZE
 *         At most a 1-bit right shift.
 *
 *     Stage 4: ROUND & PACK
 *         Apply round-to-nearest-even and pack.
 */
export class PipelinedFPMultiplier {
  static readonly NUM_STAGES = 4;

  readonly fmt: FloatFormat;
  private _stages: (FloatBits | null)[];
  private _inputsPending: [FloatBits, FloatBits][] = [];
  results: FloatBits[] = [];
  cycleCount: number = 0;

  constructor(fmt: FloatFormat = FP32) {
    this.fmt = fmt;
    this._stages = new Array(PipelinedFPMultiplier.NUM_STAGES).fill(null);
  }

  submit(a: FloatBits, b: FloatBits): void {
    this._inputsPending.push([a, b]);
  }

  tick(): void {
    this.cycleCount += 1;

    for (let i = PipelinedFPMultiplier.NUM_STAGES - 1; i > 0; i--) {
      this._stages[i] = this._stages[i - 1];
    }

    if (this._inputsPending.length > 0) {
      const [a, b] = this._inputsPending.shift()!;
      this._stages[0] = fpMul(a, b);
    } else {
      this._stages[0] = null;
    }

    const lastStage = PipelinedFPMultiplier.NUM_STAGES - 1;
    if (this._stages[lastStage] !== null) {
      this.results.push(this._stages[lastStage]!);
      this._stages[lastStage] = null;
    }
  }
}

// ---------------------------------------------------------------------------
// PipelinedFMA -- 6-stage pipelined fused multiply-add
// ---------------------------------------------------------------------------

/**
 * A 6-stage pipelined fused multiply-add (FMA) unit.
 *
 * FMA computes a * b + c with a single rounding step. It's the most
 * important operation in machine learning because the dot product
 * (the core of matrix multiplication) is just a chain of FMAs:
 *
 *     dot(a, w) = a[0]*w[0] + a[1]*w[1] + ... + a[N]*w[N]
 *               = FMA(a[0], w[0], FMA(a[1], w[1], FMA(...)))
 *
 * === Pipeline Stages ===
 *
 *     Stage 1: UNPACK all three operands (a, b, c)
 *     Stage 2: MULTIPLY a * b mantissas (full precision, no rounding!)
 *     Stage 3: ALIGN product with c's mantissa
 *     Stage 4: ADD product + c
 *     Stage 5: NORMALIZE the sum
 *     Stage 6: ROUND & PACK (single rounding step!)
 */
export class PipelinedFMA {
  static readonly NUM_STAGES = 6;

  readonly fmt: FloatFormat;
  private _stages: (FloatBits | null)[];
  private _inputsPending: [FloatBits, FloatBits, FloatBits][] = [];
  results: FloatBits[] = [];
  cycleCount: number = 0;

  constructor(fmt: FloatFormat = FP32) {
    this.fmt = fmt;
    this._stages = new Array(PipelinedFMA.NUM_STAGES).fill(null);
  }

  submit(a: FloatBits, b: FloatBits, c: FloatBits): void {
    this._inputsPending.push([a, b, c]);
  }

  tick(): void {
    this.cycleCount += 1;

    for (let i = PipelinedFMA.NUM_STAGES - 1; i > 0; i--) {
      this._stages[i] = this._stages[i - 1];
    }

    if (this._inputsPending.length > 0) {
      const [a, b, c] = this._inputsPending.shift()!;
      this._stages[0] = fpFma(a, b, c);
    } else {
      this._stages[0] = null;
    }

    const lastStage = PipelinedFMA.NUM_STAGES - 1;
    if (this._stages[lastStage] !== null) {
      this.results.push(this._stages[lastStage]!);
      this._stages[lastStage] = null;
    }
  }
}

// ---------------------------------------------------------------------------
// FPUnit -- a complete floating-point unit with all three pipelines
// ---------------------------------------------------------------------------

/**
 * A complete floating-point unit with pipelined adder, multiplier, and FMA.
 *
 * This is what sits inside every GPU core (CUDA core / shader processor /
 * execution unit). A single FP unit contains:
 *
 *     +---------------------------------------------+
 *     |                    FP Unit                   |
 *     |                                              |
 *     |   +-------------------------------+          |
 *     |   |  Pipelined FP Adder (5 stages)|          |
 *     |   +-------------------------------+          |
 *     |                                              |
 *     |   +-------------------------------+          |
 *     |   |  Pipelined FP Multiplier (4)  |          |
 *     |   +-------------------------------+          |
 *     |                                              |
 *     |   +-------------------------------+          |
 *     |   |  Pipelined FMA Unit (6 stages)|          |
 *     |   +-------------------------------+          |
 *     |                                              |
 *     |   All three share the same clock signal      |
 *     +---------------------------------------------+
 *
 * A modern GPU like the NVIDIA RTX 4090 has 16,384 CUDA cores, each
 * containing an FP unit like this. Running at ~2.5 GHz, that's:
 *
 *     16,384 cores x 2 FLOPs/cycle (FMA) x 2.52 GHz = 82.6 TFLOPS
 */
export class FPUnit {
  readonly fmt: FloatFormat;
  readonly adder: PipelinedFPAdder;
  readonly multiplier: PipelinedFPMultiplier;
  readonly fma: PipelinedFMA;

  constructor(fmt: FloatFormat = FP32) {
    this.fmt = fmt;
    this.adder = new PipelinedFPAdder(fmt);
    this.multiplier = new PipelinedFPMultiplier(fmt);
    this.fma = new PipelinedFMA(fmt);
  }

  /**
   * Run the clock for n complete cycles.
   *
   * Each tick advances all three pipelines by one stage simultaneously.
   *
   * @param n - Number of complete clock cycles to execute.
   */
  tick(n: number = 1): void {
    for (let i = 0; i < n; i++) {
      this.adder.tick();
      this.multiplier.tick();
      this.fma.tick();
    }
  }
}
