/**
 * Clock -- the heartbeat of every digital circuit.
 *
 * Every sequential circuit in a computer -- flip-flops, registers, counters,
 * CPU pipeline stages, GPU cores -- is driven by a clock signal. The clock
 * is a square wave that alternates between 0 and 1:
 *
 *     +--+  +--+  +--+  +--+
 *     |  |  |  |  |  |  |  |
 * ----+  +--+  +--+  +--+  +--
 *
 * On each rising edge (0->1), flip-flops capture their inputs. This is
 * what makes synchronous digital logic work -- everything happens in
 * lockstep, driven by the clock.
 *
 * In real hardware:
 * - CPU clock: 3-5 GHz (3-5 billion cycles per second)
 * - GPU clock: 1-2 GHz
 * - Memory clock: 4-8 GHz (DDR5)
 * - The clock frequency is the single most important performance number
 *
 * Why does the clock matter?
 * =========================
 *
 * Without a clock, digital circuits would be chaotic. Imagine a chain of
 * logic gates where each gate has a slightly different propagation delay.
 * Without synchronization, signals would arrive at different times and
 * produce garbage. The clock solves this by saying: "Everyone, capture
 * your inputs NOW." This is called synchronous design.
 *
 * The clock period must be long enough for the slowest signal path to
 * settle. This slowest path is called the "critical path," and it
 * determines the maximum clock frequency. Faster clocks = more operations
 * per second = faster computers, but only up to the point where signals
 * can still settle between edges.
 *
 * Half-cycles and edges
 * =====================
 *
 * A single clock cycle has two halves:
 *
 *     Tick 0: value goes 0 -> 1 (RISING EDGE)   <- most circuits trigger here
 *     Tick 1: value goes 1 -> 0 (FALLING EDGE)   <- some DDR circuits use this too
 *
 * "DDR" (Double Data Rate) memory uses BOTH edges, which is why DDR5-6400
 * actually runs at 3200 MHz but transfers data on both rising and falling
 * edges, achieving 6400 MT/s (megatransfers per second).
 *
 * In our simulation, each call to tick() advances one half-cycle.
 *
 * @module
 */

// ---------------------------------------------------------------------------
// ClockEdge -- a record of one transition
// ---------------------------------------------------------------------------

/**
 * Record of a clock transition.
 *
 * Every time the clock ticks, it produces an edge. An edge captures:
 * - Which cycle we are in (cycles count from 1)
 * - The current signal level (0 or 1)
 * - Whether this was a rising edge (0->1) or falling edge (1->0)
 *
 * Think of it like a timestamp in a logic analyzer trace.
 */
export interface ClockEdge {
  /** Which cycle this edge belongs to (starts at 1). */
  readonly cycle: number;
  /** Current level after the transition (0 or 1). */
  readonly value: number;
  /** True if this was a 0->1 transition. */
  readonly isRising: boolean;
  /** True if this was a 1->0 transition. */
  readonly isFalling: boolean;
}

/**
 * A function that receives clock edges.
 *
 * Components register listeners to react to clock transitions.
 * In real hardware, this is just electrical connectivity -- the
 * clock wire is physically connected to every component.
 */
export type ClockListener = (edge: ClockEdge) => void;

// ---------------------------------------------------------------------------
// Clock -- the main square-wave generator
// ---------------------------------------------------------------------------

/**
 * System clock generator.
 *
 * The clock maintains a cycle count and alternates between low (0) and
 * high (1) on each tick. Components connect to the clock and react to
 * edges (transitions).
 *
 * A complete cycle is: low -> high -> low (two ticks).
 *
 * Example usage:
 *
 *     const clock = new Clock(1_000_000);  // 1 MHz
 *     let edge = clock.tick();             // rising edge, cycle 1
 *     edge = clock.tick();                 // falling edge, cycle 1
 *     edge = clock.tick();                 // rising edge, cycle 2
 *
 * The observer pattern (listeners) allows components to react to clock
 * edges without polling. This mirrors how real hardware works: components
 * are physically connected to the clock line and react to voltage changes.
 */
export class Clock {
  /** Clock frequency in Hz. */
  readonly frequencyHz: number;

  /** Current cycle count (starts at 0, increments on rising edges). */
  cycle: number = 0;

  /** Current signal level (0 or 1). */
  value: number = 0;

  /** Total half-cycles elapsed since creation or last reset. */
  private _tickCount: number = 0;

  /** Registered edge listeners (observer pattern). */
  private _listeners: ClockListener[] = [];

  /**
   * Create a new Clock with the given frequency in Hz.
   *
   * The clock starts at value 0 (low), cycle 0, with no ticks elapsed.
   * This is the state of a real oscillator before it starts oscillating.
   *
   * @param frequencyHz - Clock frequency in Hz (default: 1 MHz).
   */
  constructor(frequencyHz: number = 1_000_000) {
    this.frequencyHz = frequencyHz;
  }

  /**
   * Advance one half-cycle. Returns the edge that occurred.
   *
   * The clock alternates like a toggle switch:
   * - If currently 0, goes to 1 (rising edge, new cycle starts)
   * - If currently 1, goes to 0 (falling edge, cycle ends)
   *
   * After toggling, all registered listeners are notified with the
   * edge record. This is how connected components "see" the clock.
   *
   * @returns ClockEdge with the transition details.
   */
  tick(): ClockEdge {
    const oldValue = this.value;
    this.value = 1 - this.value;
    this._tickCount += 1;

    const isRising = oldValue === 0 && this.value === 1;
    const isFalling = oldValue === 1 && this.value === 0;

    // Cycle count increments on each rising edge.
    // Cycle 1 starts with the first rising edge, cycle 2 with the second, etc.
    if (isRising) {
      this.cycle += 1;
    }

    const edge: ClockEdge = {
      cycle: this.cycle,
      value: this.value,
      isRising,
      isFalling,
    };

    // Notify all listeners -- this is the observer pattern.
    // In real hardware, this is just electrical connectivity.
    for (const listener of this._listeners) {
      listener(edge);
    }

    return edge;
  }

  /**
   * Execute one complete cycle (rising + falling edge).
   *
   * A full cycle is two ticks:
   * 1. Rising edge (0 -> 1): the "active" half
   * 2. Falling edge (1 -> 0): the "idle" half
   *
   * @returns Tuple of [risingEdge, fallingEdge].
   */
  fullCycle(): [ClockEdge, ClockEdge] {
    const rising = this.tick();
    const falling = this.tick();
    return [rising, falling];
  }

  /**
   * Run for N complete cycles. Returns all edges.
   *
   * This is a convenience method for running a fixed number of cycles.
   * Since each cycle has two edges (rising + falling), running N cycles
   * produces 2N edges total.
   *
   * @param cycles - Number of complete cycles to execute.
   * @returns Array of all ClockEdge objects produced.
   */
  run(cycles: number): ClockEdge[] {
    const edges: ClockEdge[] = [];
    for (let i = 0; i < cycles; i++) {
      const [r, f] = this.fullCycle();
      edges.push(r, f);
    }
    return edges;
  }

  /**
   * Register a function to be called on every clock edge.
   *
   * In real hardware, this is like connecting a wire from the clock
   * to a component's clock input pin. The component will "see" every
   * transition.
   *
   * @param callback - Function that takes a ClockEdge argument.
   */
  registerListener(callback: ClockListener): void {
    this._listeners.push(callback);
  }

  /**
   * Remove a previously registered listener.
   *
   * @param callback - The same function reference that was registered.
   * @throws Error if the callback was not registered.
   */
  unregisterListener(callback: ClockListener): void {
    const index = this._listeners.indexOf(callback);
    if (index === -1) {
      throw new Error("Listener not found");
    }
    this._listeners.splice(index, 1);
  }

  /**
   * Reset the clock to its initial state.
   *
   * Sets the value back to 0, cycle count to 0, and tick count to 0.
   * Listeners are preserved -- only the timing state is reset.
   * This is like hitting the reset button on an oscillator.
   */
  reset(): void {
    this.cycle = 0;
    this.value = 0;
    this._tickCount = 0;
  }

  /**
   * Clock period in nanoseconds.
   *
   * The period is the time for one complete cycle (rising + falling).
   * For a 1 GHz clock, the period is 1 ns. For 1 MHz, it is 1000 ns.
   *
   * Formula: period = 1 / frequency
   * In nanoseconds: period_ns = 1e9 / frequency_hz
   */
  get periodNs(): number {
    return 1e9 / this.frequencyHz;
  }

  /**
   * Total half-cycles elapsed since creation or last reset.
   */
  get totalTicks(): number {
    return this._tickCount;
  }
}

// ---------------------------------------------------------------------------
// ClockDivider -- frequency division
// ---------------------------------------------------------------------------

/**
 * Divides a clock frequency by an integer factor.
 *
 * In hardware, clock dividers are used to generate slower clocks from
 * a fast master clock. For example, a 1 GHz CPU clock might be divided
 * by 4 to get a 250 MHz bus clock.
 *
 * How it works:
 * - Count rising edges from the source clock
 * - Every `divisor` rising edges, generate one full cycle on the output
 *
 * This means the output frequency = source frequency / divisor.
 *
 * Example:
 *
 *     const master = new Clock(1_000_000_000);  // 1 GHz
 *     const divider = new ClockDivider(master, 4);
 *     // divider.output runs at 250 MHz
 *
 *     master.run(8);  // 8 source cycles
 *     // divider.output has completed 2 cycles (8 / 4 = 2)
 *
 * Real-world uses:
 * - CPU-to-bus clock ratio (e.g., CPU at 4 GHz, bus at 1 GHz)
 * - USB clock derivation from system clock
 * - Audio sample rate generation from master clock
 */
export class ClockDivider {
  /** The faster source clock. */
  readonly source: Clock;

  /** Division factor. */
  readonly divisor: number;

  /** The slower output clock. */
  readonly output: Clock;

  /** Rising edge counter. */
  private _counter: number = 0;

  /**
   * Create a clock divider.
   *
   * @param source - The faster clock to divide.
   * @param divisor - Division factor (must be >= 2).
   * @throws Error if divisor is less than 2.
   */
  constructor(source: Clock, divisor: number) {
    if (divisor < 2) {
      throw new Error(`Divisor must be >= 2, got ${divisor}`);
    }
    this.source = source;
    this.divisor = divisor;
    this.output = new Clock(Math.floor(source.frequencyHz / divisor));
    source.registerListener(this._onEdge.bind(this));
  }

  /**
   * Called on every source clock edge.
   *
   * We only count rising edges. When we have counted `divisor` rising
   * edges, we generate one complete output cycle (rising + falling).
   */
  private _onEdge(edge: ClockEdge): void {
    if (edge.isRising) {
      this._counter += 1;
      if (this._counter >= this.divisor) {
        this._counter = 0;
        this.output.tick(); // rising
        this.output.tick(); // falling
      }
    }
  }
}

// ---------------------------------------------------------------------------
// MultiPhaseClock -- non-overlapping phase generation
// ---------------------------------------------------------------------------

/**
 * Generates multiple clock phases from a single source.
 *
 * Used in CPU pipelines where different stages need offset clocks.
 * A 4-phase clock generates 4 non-overlapping clock signals, each
 * active for 1/4 of the master cycle.
 *
 * Timing diagram for a 4-phase clock:
 *
 *     Source:  _|^|_|^|_|^|_|^|_
 *     Phase 0: _|^|___|___|___|_
 *     Phase 1: _|___|^|___|___|_
 *     Phase 2: _|___|___|^|___|_
 *     Phase 3: _|___|___|___|^|_
 *
 * On each rising edge of the source, exactly ONE phase is active (1)
 * and all others are inactive (0). The active phase rotates.
 *
 * Real-world uses:
 * - Classic RISC pipelines (fetch, decode, execute, writeback)
 * - DRAM refresh timing
 * - Multiplexed bus access
 */
export class MultiPhaseClock {
  /** The master clock. */
  readonly source: Clock;

  /** Number of phases. */
  readonly phases: number;

  /** Index of the currently active phase. */
  activePhase: number = 0;

  /** Current value of each phase (0 or 1). */
  private _phaseValues: number[];

  /**
   * Create a multi-phase clock.
   *
   * @param source - The master clock to derive phases from.
   * @param phases - Number of phases (must be >= 2).
   * @throws Error if phases is less than 2.
   */
  constructor(source: Clock, phases: number = 4) {
    if (phases < 2) {
      throw new Error(`Phases must be >= 2, got ${phases}`);
    }
    this.source = source;
    this.phases = phases;
    this._phaseValues = new Array(phases).fill(0);
    source.registerListener(this._onEdge.bind(this));
  }

  /**
   * Called on every source clock edge.
   *
   * On rising edges, we rotate the active phase. Only one phase
   * is high at any time -- this is the "non-overlapping" property
   * that prevents pipeline hazards.
   */
  private _onEdge(edge: ClockEdge): void {
    if (edge.isRising) {
      this._phaseValues = new Array(this.phases).fill(0);
      this._phaseValues[this.activePhase] = 1;
      this.activePhase = (this.activePhase + 1) % this.phases;
    }
  }

  /**
   * Get current value of phase N.
   *
   * @param index - Phase index (0 to phases-1).
   * @returns 1 if phase is active, 0 if inactive.
   */
  getPhase(index: number): number {
    return this._phaseValues[index];
  }
}
