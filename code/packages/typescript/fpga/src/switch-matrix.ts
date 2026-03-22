/**
 * Switch Matrix -- programmable routing crossbar for the FPGA fabric.
 *
 * === What is a Switch Matrix? ===
 *
 * The routing fabric is what makes an FPGA truly programmable. LUTs and
 * CLBs compute boolean functions, but the switch matrix determines how
 * those functions connect to each other.
 *
 * A switch matrix sits at each intersection of the routing grid. It's a
 * crossbar that can connect any of its input wires to any of its output
 * wires, based on configuration bits stored in SRAM.
 *
 * === Connection Model ===
 *
 * We model the switch matrix as a set of named ports and a configurable
 * connection map. Each connection maps an input port to an output port.
 * When a signal arrives at an input port, the switch matrix routes it to
 * all connected output ports.
 *
 * This is equivalent to the real hardware: SRAM bits control pass
 * transistors that connect wire segments through the crossbar.
 */

import { type Bit } from "@coding-adventures/logic-gates";

/**
 * Programmable routing crossbar.
 *
 * Connects named signal ports via configurable routes. Each route
 * maps a source port to a destination port. Multiple routes can
 * share the same source (fan-out) but each destination can only
 * have one source (no bus contention).
 *
 * @example
 * const sm = new SwitchMatrix(new Set(["north", "south", "east", "west", "clbOut"]));
 * sm.connect("clbOut", "east");
 * sm.connect("north", "south");
 * sm.route({ clbOut: 1, north: 0 })  // { east: 1, south: 0 }
 */
export class SwitchMatrix {
  private readonly _ports: ReadonlySet<string>;
  /** Maps destination -> source. */
  private readonly _connections: Map<string, string> = new Map();

  /**
   * @param ports - Set of port names (strings). Must be non-empty.
   */
  constructor(ports: Set<string>) {
    if (!ports || ports.size === 0) {
      throw new RangeError("ports must be non-empty");
    }
    for (const p of ports) {
      if (typeof p !== "string" || p === "") {
        throw new TypeError(`port names must be non-empty strings, got ${JSON.stringify(p)}`);
      }
    }

    this._ports = new Set(ports);
  }

  /**
   * Create a route from source to destination.
   *
   * @param source - Name of the input port
   * @param destination - Name of the output port
   * @throws RangeError if ports are unknown or destination already connected.
   */
  connect(source: string, destination: string): void {
    if (!this._ports.has(source)) {
      throw new RangeError(`unknown source port: ${JSON.stringify(source)}`);
    }
    if (!this._ports.has(destination)) {
      throw new RangeError(`unknown destination port: ${JSON.stringify(destination)}`);
    }
    if (source === destination) {
      throw new RangeError(`cannot connect port ${JSON.stringify(source)} to itself`);
    }
    if (this._connections.has(destination)) {
      throw new RangeError(
        `destination ${JSON.stringify(destination)} already connected to ${JSON.stringify(this._connections.get(destination))}`,
      );
    }

    this._connections.set(destination, source);
  }

  /**
   * Remove the route to a destination port.
   *
   * @param destination - The port to disconnect.
   * @throws RangeError if port is unknown or not connected.
   */
  disconnect(destination: string): void {
    if (!this._ports.has(destination)) {
      throw new RangeError(`unknown port: ${JSON.stringify(destination)}`);
    }
    if (!this._connections.has(destination)) {
      throw new RangeError(`port ${JSON.stringify(destination)} is not connected`);
    }

    this._connections.delete(destination);
  }

  /** Remove all connections (reset the switch matrix). */
  clear(): void {
    this._connections.clear();
  }

  /**
   * Propagate signals through the switch matrix.
   *
   * @param inputs - Map of port name -> signal value (0 or 1) for
   *                 ports that have external signals driving them.
   * @returns Map of destination port -> routed signal value for all
   *          connected destinations whose source appears in inputs.
   */
  route(inputs: Record<string, Bit>): Record<string, Bit> {
    const outputs: Record<string, Bit> = {};
    for (const [dest, src] of this._connections.entries()) {
      if (src in inputs) {
        outputs[dest] = inputs[src];
      }
    }
    return outputs;
  }

  /** Set of all port names. */
  get ports(): ReadonlySet<string> {
    return this._ports;
  }

  /** Current connection map (destination -> source). Returns a copy. */
  get connections(): Record<string, string> {
    const result: Record<string, string> = {};
    for (const [dest, src] of this._connections.entries()) {
      result[dest] = src;
    }
    return result;
  }

  /** Number of active connections. */
  get connectionCount(): number {
    return this._connections.size;
  }
}
