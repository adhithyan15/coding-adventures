/**
 * FPGA Fabric -- the top-level FPGA model.
 *
 * === What is an FPGA? ===
 *
 * An FPGA (Field-Programmable Gate Array) is a chip containing:
 * - A grid of CLBs (Configurable Logic Blocks) for computation
 * - A routing fabric (switch matrices) for interconnection
 * - I/O blocks at the perimeter for external connections
 * - Block RAM tiles for on-chip memory
 *
 * The key property: **all of this is programmable**. By loading a
 * bitstream (configuration data), the same physical chip can become
 * any digital circuit -- a CPU, a signal processor, a network switch,
 * or anything else that fits within its resources.
 *
 * === Our FPGA Model ===
 *
 *     +-----------------------------------------------------+
 *     |                    FPGA Fabric                        |
 *     |                                                       |
 *     |  [IO] [IO] [IO] [IO] [IO] [IO] [IO] [IO]            |
 *     |                                                       |
 *     |  [IO] [CLB]--[SW]--[CLB]--[SW]--[CLB] [IO]          |
 *     |         |            |            |                    |
 *     |        [SW]         [SW]         [SW]                 |
 *     |         |            |            |                    |
 *     |  [IO] [CLB]--[SW]--[CLB]--[SW]--[CLB] [IO]          |
 *     |                                                       |
 *     |  [IO] [IO] [IO] [IO] [IO] [IO] [IO] [IO]            |
 *     |                                                       |
 *     |            [BRAM]        [BRAM]                       |
 *     +-----------------------------------------------------+
 */

import { type Bit } from "@coding-adventures/logic-gates";
import { Bitstream } from "./bitstream.js";
import { CLB, type CLBOutput } from "./clb.js";
import { IOBlock, IOMode } from "./io-block.js";
import { SwitchMatrix } from "./switch-matrix.js";

/**
 * Top-level FPGA fabric model.
 *
 * Creates and configures CLBs, switch matrices, and I/O blocks
 * from a Bitstream, then provides methods to evaluate the circuit.
 *
 * @example
 * const config = {
 *   clbs: { "clb_0": { slice0: { lutA: [...] } } },
 *   io: { "inA": { mode: "input" }, "out": { mode: "output" } },
 * };
 * const bs = Bitstream.fromObject(config);
 * const fpga = new FPGA(bs);
 */
export class FPGA {
  private readonly _bitstream: Bitstream;
  private readonly _clbs: Record<string, CLB> = {};
  private readonly _switches: Record<string, SwitchMatrix> = {};
  private readonly _ios: Record<string, IOBlock> = {};

  constructor(bitstream: Bitstream) {
    this._bitstream = bitstream;
    this._configure(bitstream);
  }

  private _configure(bs: Bitstream): void {
    // Create and configure CLBs
    for (const [name, clbCfg] of Object.entries(bs.clbs)) {
      const clb = new CLB(bs.lutK);

      clb.slice0.configure(
        clbCfg.slice0.lutA,
        clbCfg.slice0.lutB,
        clbCfg.slice0.ffAEnabled,
        clbCfg.slice0.ffBEnabled,
        clbCfg.slice0.carryEnabled,
      );
      clb.slice1.configure(
        clbCfg.slice1.lutA,
        clbCfg.slice1.lutB,
        clbCfg.slice1.ffAEnabled,
        clbCfg.slice1.ffBEnabled,
        clbCfg.slice1.carryEnabled,
      );

      this._clbs[name] = clb;
    }

    // Create and configure switch matrices
    for (const [swName, routes] of Object.entries(bs.routing)) {
      const ports = new Set<string>();
      for (const route of routes) {
        ports.add(route.source);
        ports.add(route.destination);
      }

      if (ports.size > 0) {
        const sm = new SwitchMatrix(ports);
        for (const route of routes) {
          sm.connect(route.source, route.destination);
        }
        this._switches[swName] = sm;
      }
    }

    // Create I/O blocks
    const modeMap: Record<string, IOMode> = {
      input: IOMode.INPUT,
      output: IOMode.OUTPUT,
      tristate: IOMode.TRISTATE,
    };
    for (const [pinName, ioCfg] of Object.entries(bs.io)) {
      const mode = modeMap[ioCfg.mode] ?? IOMode.INPUT;
      this._ios[pinName] = new IOBlock(pinName, mode);
    }
  }

  /**
   * Evaluate a specific CLB.
   *
   * @param clbName - Name of the CLB to evaluate.
   * @param slice0InputsA - Inputs for slice 0's LUT A.
   * @param slice0InputsB - Inputs for slice 0's LUT B.
   * @param slice1InputsA - Inputs for slice 1's LUT A.
   * @param slice1InputsB - Inputs for slice 1's LUT B.
   * @param clock - Clock signal (0 or 1).
   * @param carryIn - External carry input.
   * @returns CLBOutput from the evaluated CLB.
   * @throws Error if clbName not found.
   */
  evaluateCLB(
    clbName: string,
    slice0InputsA: Bit[],
    slice0InputsB: Bit[],
    slice1InputsA: Bit[],
    slice1InputsB: Bit[],
    clock: Bit,
    carryIn: Bit = 0,
  ): CLBOutput {
    if (!(clbName in this._clbs)) {
      throw new Error(`CLB ${JSON.stringify(clbName)} not found`);
    }

    return this._clbs[clbName].evaluate(
      slice0InputsA,
      slice0InputsB,
      slice1InputsA,
      slice1InputsB,
      clock,
      carryIn,
    );
  }

  /**
   * Route signals through a switch matrix.
   *
   * @param switchName - Name of the switch matrix.
   * @param signals - Input signals (portName -> value).
   * @returns Routed output signals.
   * @throws Error if switchName not found.
   */
  route(switchName: string, signals: Record<string, Bit>): Record<string, Bit> {
    if (!(switchName in this._switches)) {
      throw new Error(`Switch matrix ${JSON.stringify(switchName)} not found`);
    }

    return this._switches[switchName].route(signals);
  }

  /**
   * Drive an input pin.
   *
   * @param pinName - Name of the I/O pin.
   * @param value - Signal value (0 or 1).
   * @throws Error if pinName not found.
   */
  setInput(pinName: string, value: Bit): void {
    if (!(pinName in this._ios)) {
      throw new Error(`I/O pin ${JSON.stringify(pinName)} not found`);
    }
    this._ios[pinName].drivePad(value);
  }

  /**
   * Read an output pin.
   *
   * @param pinName - Name of the I/O pin.
   * @returns Signal value (0, 1, or null for tri-state).
   * @throws Error if pinName not found.
   */
  readOutput(pinName: string): Bit | null {
    if (!(pinName in this._ios)) {
      throw new Error(`I/O pin ${JSON.stringify(pinName)} not found`);
    }
    return this._ios[pinName].readPad();
  }

  /**
   * Drive the internal side of an output pin (fabric -> external).
   *
   * @param pinName - Name of the I/O pin.
   * @param value - Signal value (0 or 1).
   * @throws Error if pinName not found.
   */
  driveOutput(pinName: string, value: Bit): void {
    if (!(pinName in this._ios)) {
      throw new Error(`I/O pin ${JSON.stringify(pinName)} not found`);
    }
    this._ios[pinName].driveInternal(value);
  }

  /** All CLBs in the fabric. */
  get clbs(): Record<string, CLB> {
    return { ...this._clbs };
  }

  /** All switch matrices in the fabric. */
  get switches(): Record<string, SwitchMatrix> {
    return { ...this._switches };
  }

  /** All I/O blocks. */
  get ios(): Record<string, IOBlock> {
    return { ...this._ios };
  }

  /** The loaded bitstream configuration. */
  get bitstream(): Bitstream {
    return this._bitstream;
  }
}
