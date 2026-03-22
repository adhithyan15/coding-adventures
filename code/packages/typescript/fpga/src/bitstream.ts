/**
 * Bitstream -- FPGA configuration data.
 *
 * === What is a Bitstream? ===
 *
 * In a real FPGA, a bitstream is a binary blob that programs every
 * configurable element: LUT truth tables, flip-flop enables, carry chain
 * enables, routing switch states, I/O pad modes, and Block RAM contents.
 *
 * The bitstream is loaded at power-up (or during runtime for partial
 * reconfiguration) and writes to the SRAM cells that control the fabric.
 *
 * === Our JSON Configuration ===
 *
 * Instead of a binary format, we use JSON for readability and education.
 * The JSON configuration specifies:
 *
 * 1. **CLBs**: Which LUTs get which truth tables, FF enables, carry enables
 * 2. **Routing**: Which switch matrix ports are connected
 * 3. **I/O**: Pin names, modes, and mappings
 *
 * Example JSON:
 *
 *     {
 *         "clbs": {
 *             "clb_0_0": {
 *                 "slice0": {
 *                     "lutA": [0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0],
 *                     "lutB": [0,1,1,0,1,0,0,1,1,0,0,1,0,1,1,0],
 *                     "ffA": true,
 *                     "ffB": false,
 *                     "carry": false
 *                 },
 *                 "slice1": { ... }
 *             }
 *         },
 *         "routing": {
 *             "sw_0_0": [
 *                 {"src": "clb_out_a", "dst": "east"}
 *             ]
 *         },
 *         "io": {
 *             "pin_A0": {"mode": "input"},
 *             "pin_B0": {"mode": "output"}
 *         }
 *     }
 */

import { type Bit } from "@coding-adventures/logic-gates";

/**
 * Configuration for one slice.
 */
export interface SliceConfig {
  /** Truth table for LUT A (2^k entries). */
  readonly lutA: Bit[];
  /** Truth table for LUT B (2^k entries). */
  readonly lutB: Bit[];
  /** Route LUT A through flip-flop. */
  readonly ffAEnabled: boolean;
  /** Route LUT B through flip-flop. */
  readonly ffBEnabled: boolean;
  /** Enable carry chain. */
  readonly carryEnabled: boolean;
}

/**
 * Configuration for one CLB (2 slices).
 */
export interface CLBConfig {
  readonly slice0: SliceConfig;
  readonly slice1: SliceConfig;
}

/**
 * A single routing connection.
 */
export interface RouteConfig {
  readonly source: string;
  readonly destination: string;
}

/**
 * Configuration for one I/O block.
 */
export interface IOConfig {
  /** "input", "output", or "tristate". */
  readonly mode: string;
}

/**
 * FPGA configuration data -- the 'program' for the fabric.
 *
 * @example
 * const bs = Bitstream.fromObject({
 *   clbs: { "clb_0": { slice0: { lutA: [...], lutB: [...] } } },
 *   io: { "pinA": { mode: "input" } },
 * });
 */
export class Bitstream {
  /** CLB configurations keyed by name (e.g., "clb_0_0"). */
  readonly clbs: Record<string, CLBConfig>;
  /** Switch matrix connections keyed by matrix name. */
  readonly routing: Record<string, RouteConfig[]>;
  /** I/O block configurations keyed by pin name. */
  readonly io: Record<string, IOConfig>;
  /** Number of LUT inputs (default 4). */
  readonly lutK: number;

  constructor(
    clbs: Record<string, CLBConfig> = {},
    routing: Record<string, RouteConfig[]> = {},
    io: Record<string, IOConfig> = {},
    lutK: number = 4,
  ) {
    this.clbs = clbs;
    this.routing = routing;
    this.io = io;
    this.lutK = lutK;
  }

  /**
   * Load a bitstream from a JSON string.
   */
  static fromJSON(json: string): Bitstream {
    const data = JSON.parse(json);
    return Bitstream.fromObject(data);
  }

  /**
   * Create a Bitstream from a plain object (same structure as JSON).
   */
  static fromObject(data: Record<string, unknown>): Bitstream {
    const lutK = (data.lutK as number) ?? (data.lut_k as number) ?? 4;
    const defaultTt = Array(1 << lutK).fill(0) as Bit[];

    // Parse CLBs
    const clbs: Record<string, CLBConfig> = {};
    const clbsData = (data.clbs ?? {}) as Record<string, Record<string, unknown>>;
    for (const [name, clbData] of Object.entries(clbsData)) {
      const s0 = (clbData.slice0 ?? {}) as Record<string, unknown>;
      const s1 = (clbData.slice1 ?? {}) as Record<string, unknown>;
      clbs[name] = {
        slice0: {
          lutA: (s0.lutA ?? s0.lut_a ?? [...defaultTt]) as Bit[],
          lutB: (s0.lutB ?? s0.lut_b ?? [...defaultTt]) as Bit[],
          ffAEnabled: (s0.ffA ?? s0.ff_a ?? false) as boolean,
          ffBEnabled: (s0.ffB ?? s0.ff_b ?? false) as boolean,
          carryEnabled: (s0.carry ?? false) as boolean,
        },
        slice1: {
          lutA: (s1.lutA ?? s1.lut_a ?? [...defaultTt]) as Bit[],
          lutB: (s1.lutB ?? s1.lut_b ?? [...defaultTt]) as Bit[],
          ffAEnabled: (s1.ffA ?? s1.ff_a ?? false) as boolean,
          ffBEnabled: (s1.ffB ?? s1.ff_b ?? false) as boolean,
          carryEnabled: (s1.carry ?? false) as boolean,
        },
      };
    }

    // Parse routing
    const routing: Record<string, RouteConfig[]> = {};
    const routingData = (data.routing ?? {}) as Record<string, Array<Record<string, string>>>;
    for (const [swName, routes] of Object.entries(routingData)) {
      routing[swName] = routes.map((r) => ({
        source: r.src,
        destination: r.dst,
      }));
    }

    // Parse I/O
    const io: Record<string, IOConfig> = {};
    const ioData = (data.io ?? {}) as Record<string, Record<string, string>>;
    for (const [pinName, ioConf] of Object.entries(ioData)) {
      io[pinName] = { mode: ioConf.mode ?? "input" };
    }

    return new Bitstream(clbs, routing, io, lutK);
  }
}
