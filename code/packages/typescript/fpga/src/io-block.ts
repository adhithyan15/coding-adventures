/**
 * I/O Block -- bidirectional pad connecting FPGA internals to the outside world.
 *
 * === What is an I/O Block? ===
 *
 * I/O blocks sit at the perimeter of the FPGA and provide the interface
 * between the internal logic fabric and the external pins of the chip.
 *
 * Each I/O block can be configured in three modes:
 * - **Input**: External signal enters the FPGA (pad -> internal)
 * - **Output**: Internal signal exits the FPGA (internal -> pad)
 * - **Tri-state**: Output is high-impedance (disconnected) when not enabled
 *
 * === I/O Block Architecture ===
 *
 *     External Pin (pad)
 *          |
 *          v
 *     +--------------------+
 *     |    I/O Block       |
 *     |                    |
 *     |  +---------------+ |
 *     |  | Input Reg     | | -- (optional) register the input
 *     |  +-------+-------+ |
 *     |          |          |
 *     |  +-------v-------+ |
 *     |  | Tri-State      | | -- output enable controls direction
 *     |  | Buffer         | |
 *     |  +-------+-------+ |
 *     |          |          |
 *     |  +-------v-------+ |
 *     |  | Output Reg    | | -- (optional) register the output
 *     |  +---------------+ |
 *     |                    |
 *     +--------------------+
 *          |
 *          v
 *     To/From Internal Fabric
 */

import { type Bit, triState } from "@coding-adventures/logic-gates";

/**
 * I/O block operating mode.
 *
 * - INPUT:    Pad drives internal signal (external -> fabric)
 * - OUTPUT:   Fabric drives pad (fabric -> external)
 * - TRISTATE: Output is high-impedance (pad is disconnected)
 */
export enum IOMode {
  INPUT = "input",
  OUTPUT = "output",
  TRISTATE = "tristate",
}

/**
 * Bidirectional I/O pad for the FPGA perimeter.
 *
 * Each I/O block connects one external pin to the internal fabric.
 * The mode determines the direction of data flow.
 *
 * @example
 * // Input pin
 * const io = new IOBlock("sensorIn", IOMode.INPUT);
 * io.drivePad(1);        // External signal arrives
 * io.readInternal()       // 1 -- fabric sees the signal
 *
 * // Output pin
 * const led = new IOBlock("led0", IOMode.OUTPUT);
 * led.driveInternal(1);   // Fabric sends signal
 * led.readPad()           // 1 -- external pin shows the signal
 *
 * // Tri-state (disconnected)
 * const bus = new IOBlock("bus0", IOMode.TRISTATE);
 * bus.driveInternal(1);
 * bus.readPad()           // null -- high impedance
 */
export class IOBlock {
  private readonly _name: string;
  private _mode: IOMode;
  private _padValue: Bit = 0;
  private _internalValue: Bit = 0;

  /**
   * @param name - Identifier for this I/O block (e.g., "pinA0", "led0")
   * @param mode - Initial operating mode (default: INPUT)
   */
  constructor(name: string, mode: IOMode = IOMode.INPUT) {
    if (typeof name !== "string" || name === "") {
      throw new TypeError("name must be a non-empty string");
    }
    this._name = name;
    this._mode = mode;
  }

  /**
   * Change the I/O block's operating mode.
   */
  configure(mode: IOMode): void {
    if (!Object.values(IOMode).includes(mode)) {
      throw new TypeError(`mode must be an IOMode, got ${String(mode)}`);
    }
    this._mode = mode;
  }

  /**
   * Drive the external pad with a signal (used in INPUT mode).
   *
   * @param value - Signal value (0 or 1)
   */
  drivePad(value: Bit): void {
    if (value !== 0 && value !== 1) {
      throw new RangeError(`value must be 0 or 1, got ${value}`);
    }
    this._padValue = value;
  }

  /**
   * Drive the internal (fabric) side with a signal (used in OUTPUT mode).
   *
   * @param value - Signal value (0 or 1)
   */
  driveInternal(value: Bit): void {
    if (value !== 0 && value !== 1) {
      throw new RangeError(`value must be 0 or 1, got ${value}`);
    }
    this._internalValue = value;
  }

  /**
   * Read the signal visible to the internal fabric.
   *
   * In INPUT mode, returns the pad value (external -> fabric).
   * In OUTPUT/TRISTATE mode, returns the internally driven value.
   */
  readInternal(): Bit {
    if (this._mode === IOMode.INPUT) {
      return this._padValue;
    }
    return this._internalValue;
  }

  /**
   * Read the signal visible on the external pad.
   *
   * In INPUT mode, returns the pad value.
   * In OUTPUT mode, returns the internally driven value.
   * In TRISTATE mode, returns null (high impedance).
   */
  readPad(): Bit | null {
    if (this._mode === IOMode.INPUT) {
      return this._padValue;
    }
    if (this._mode === IOMode.TRISTATE) {
      return triState(this._internalValue, 0);
    }
    // OUTPUT mode: tri-state with enable=1
    return triState(this._internalValue, 1);
  }

  /** I/O block identifier. */
  get name(): string {
    return this._name;
  }

  /** Current operating mode. */
  get mode(): IOMode {
    return this._mode;
  }
}
