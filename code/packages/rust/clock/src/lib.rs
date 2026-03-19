//! # Clock — the heartbeat of every digital circuit.
//!
//! Every sequential circuit in a computer — flip-flops, registers, counters,
//! CPU pipeline stages, GPU cores — is driven by a clock signal. The clock
//! is a square wave that alternates between 0 and 1:
//!
//! ```text
//!     +--+  +--+  +--+  +--+
//!     |  |  |  |  |  |  |  |
//! ----+  +--+  +--+  +--+  +--
//! ```
//!
//! On each rising edge (0->1), flip-flops capture their inputs. This is
//! what makes synchronous digital logic work — everything happens in
//! lockstep, driven by the clock.
//!
//! # Real hardware clock speeds
//!
//! - CPU clock: 3-5 GHz (3-5 billion cycles per second)
//! - GPU clock: 1-2 GHz
//! - Memory clock: 4-8 GHz (DDR5)
//!
//! # Half-cycles and edges
//!
//! A single clock cycle has two halves:
//!
//! ```text
//! Tick 0: value goes 0 -> 1 (RISING EDGE)   <- most circuits trigger here
//! Tick 1: value goes 1 -> 0 (FALLING EDGE)   <- some DDR circuits use this too
//! ```
//!
//! "DDR" (Double Data Rate) memory uses BOTH edges, which is why DDR5-6400
//! actually runs at 3200 MHz but transfers data on both rising and falling
//! edges, achieving 6400 MT/s (megatransfers per second).
//!
//! # Rust ownership and closures
//!
//! The listener pattern in this module is a great teaching moment about Rust's
//! ownership model. In Python, you can freely pass functions around and call
//! them whenever you want. In Rust, closures that capture mutable state must
//! be carefully managed:
//!
//! - `Box<dyn FnMut(&ClockEdge)>` — a heap-allocated closure that can mutate
//!   its captured variables. We use `FnMut` (not `Fn`) because listeners
//!   typically need to update counters or state.
//! - Each listener is owned by the `Clock`. Only the `Clock` can call it.
//!   This is enforced by the borrow checker at compile time.
//!
//! In real hardware, this "ownership" is physical: a wire from the clock to
//! a component IS the listener. The component reacts to voltage changes on
//! that wire. Our `Box<dyn FnMut>` is the software analog of that wire.

// ---------------------------------------------------------------------------
// ClockEdge — a record of one transition
// ---------------------------------------------------------------------------

/// Record of a clock transition.
///
/// Every time the clock ticks, it produces an edge. An edge captures:
/// - Which cycle we are in (cycles count from 1)
/// - The current signal level (0 or 1)
/// - Whether this was a rising edge (0->1) or falling edge (1->0)
///
/// Think of it like a timestamp in a logic analyzer trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClockEdge {
    pub cycle: u64,
    pub value: u8,
    pub is_rising: bool,
    pub is_falling: bool,
}

// ---------------------------------------------------------------------------
// Clock — the main square-wave generator
// ---------------------------------------------------------------------------

/// System clock generator.
///
/// The clock maintains a cycle count and alternates between low (0) and
/// high (1) on each tick. Components connect to the clock and react to
/// edges (transitions).
///
/// A complete cycle is: low -> high -> low (two ticks).
///
/// # The observer pattern and Rust's borrow checker
///
/// In Python or Go, you'd store listeners as a simple list of callbacks.
/// In Rust, mutable closures require `Box<dyn FnMut>` because:
///
/// 1. `dyn FnMut` — the closure can mutate its captured state
/// 2. `Box<...>` — we need heap allocation because closures have unknown sizes
/// 3. The `Clock` OWNS each listener, ensuring no dangling references
///
/// This is more verbose than Python, but the compiler GUARANTEES there are
/// no data races or use-after-free bugs — something that Python cannot do.
///
/// # Example
///
/// ```
/// use clock::{Clock, ClockEdge};
///
/// let mut clk = Clock::new(1_000_000); // 1 MHz
/// let edge = clk.tick();               // rising edge, cycle 1
/// assert!(edge.is_rising);
/// let edge = clk.tick();               // falling edge, cycle 1
/// assert!(edge.is_falling);
/// ```
pub struct Clock {
    frequency_hz: u64,
    cycle: u64,
    value: u8,
    total_ticks: u64,
    listeners: Vec<Box<dyn FnMut(&ClockEdge)>>,
}

impl Clock {
    /// Create a new clock with the given frequency in Hz.
    ///
    /// The clock starts at value 0, cycle 0, with no ticks elapsed.
    ///
    /// # Example
    ///
    /// ```
    /// use clock::Clock;
    /// let clk = Clock::new(1_000_000_000); // 1 GHz
    /// assert_eq!(clk.frequency_hz(), 1_000_000_000);
    /// ```
    pub fn new(frequency_hz: u64) -> Self {
        assert!(frequency_hz > 0, "frequency_hz must be > 0");
        Self {
            frequency_hz,
            cycle: 0,
            value: 0,
            total_ticks: 0,
            listeners: Vec::new(),
        }
    }

    /// Advance one half-cycle. Returns the edge that occurred.
    ///
    /// The clock alternates like a toggle switch:
    /// - If currently 0, goes to 1 (rising edge, new cycle starts)
    /// - If currently 1, goes to 0 (falling edge, cycle ends)
    ///
    /// After toggling, all registered listeners are notified with the
    /// edge record. This is how connected components "see" the clock.
    pub fn tick(&mut self) -> ClockEdge {
        let old_value = self.value;
        self.value = 1 - self.value;
        self.total_ticks += 1;

        let is_rising = old_value == 0 && self.value == 1;
        let is_falling = old_value == 1 && self.value == 0;

        // Cycle count increments on each rising edge
        if is_rising {
            self.cycle += 1;
        }

        let edge = ClockEdge {
            cycle: self.cycle,
            value: self.value,
            is_rising,
            is_falling,
        };

        // Notify all listeners — this is the observer pattern.
        // In real hardware, this is just electrical connectivity.
        for listener in &mut self.listeners {
            listener(&edge);
        }

        edge
    }

    /// Execute one complete cycle (rising + falling edge).
    ///
    /// A full cycle is two ticks:
    /// 1. Rising edge (0 -> 1): the "active" half
    /// 2. Falling edge (1 -> 0): the "idle" half
    pub fn full_cycle(&mut self) -> (ClockEdge, ClockEdge) {
        let rising = self.tick();
        let falling = self.tick();
        (rising, falling)
    }

    /// Run for N complete cycles. Returns all edges.
    ///
    /// Since each cycle has two edges (rising + falling), running N cycles
    /// produces 2N edges total.
    pub fn run(&mut self, cycles: u64) -> Vec<ClockEdge> {
        let mut edges = Vec::with_capacity((cycles * 2) as usize);
        for _ in 0..cycles {
            let (r, f) = self.full_cycle();
            edges.push(r);
            edges.push(f);
        }
        edges
    }

    /// Register a function to be called on every clock edge.
    ///
    /// In real hardware, this is like connecting a wire from the clock
    /// to a component's clock input pin.
    ///
    /// # Rust ownership note
    ///
    /// We take `Box<dyn FnMut(&ClockEdge)>` rather than a plain closure
    /// because:
    /// - `dyn` — we need to erase the concrete closure type (different
    ///   closures have different anonymous types in Rust)
    /// - `FnMut` — the closure may mutate its captured state (e.g., a counter)
    /// - `Box` — heap-allocate since closure sizes are unknown at compile time
    ///
    /// In Python, you'd just pass a function. Rust's approach is more
    /// explicit but guarantees memory safety at compile time.
    pub fn register_listener(&mut self, f: Box<dyn FnMut(&ClockEdge)>) {
        self.listeners.push(f);
    }

    /// Reset the clock to its initial state.
    ///
    /// Sets the value back to 0, cycle count to 0, and tick count to 0.
    /// Listeners are preserved — only the timing state is reset.
    pub fn reset(&mut self) {
        self.cycle = 0;
        self.value = 0;
        self.total_ticks = 0;
    }

    /// Clock period in nanoseconds.
    ///
    /// The period is the time for one complete cycle (rising + falling).
    /// For a 1 GHz clock, the period is 1 ns. For 1 MHz, it is 1000 ns.
    ///
    /// Formula: `period_ns = 1e9 / frequency_hz`
    pub fn period_ns(&self) -> f64 {
        1e9 / self.frequency_hz as f64
    }

    /// Total half-cycles elapsed since creation or last reset.
    pub fn total_ticks(&self) -> u64 {
        self.total_ticks
    }

    /// The clock frequency in Hz.
    pub fn frequency_hz(&self) -> u64 {
        self.frequency_hz
    }

    /// Current cycle number (increments on each rising edge).
    pub fn cycle(&self) -> u64 {
        self.cycle
    }

    /// Current signal value (0 or 1).
    pub fn value(&self) -> u8 {
        self.value
    }
}

// ---------------------------------------------------------------------------
// ClockDivider — frequency division
// ---------------------------------------------------------------------------

/// Divides a clock frequency by an integer factor.
///
/// In hardware, clock dividers are used to generate slower clocks from
/// a fast master clock. For example, a 1 GHz CPU clock might be divided
/// by 4 to get a 250 MHz bus clock.
///
/// # How it works
///
/// Count rising edges from the source clock. Every `divisor` rising edges,
/// generate one full cycle on the output.
///
/// Output frequency = source frequency / divisor.
///
/// # Rust implementation note
///
/// In Python, the ClockDivider registers itself as a listener on the source
/// clock and stores a reference to the source. In Rust, we can't have the
/// divider both registered as a listener AND hold a mutable reference to
/// the source (that would violate Rust's borrowing rules).
///
/// Instead, we provide an `on_edge` method that the caller must invoke
/// manually. This is a common Rust pattern: make data flow explicit rather
/// than hiding it behind shared mutable state.
///
/// # Example
///
/// ```
/// use clock::ClockDivider;
///
/// let mut divider = ClockDivider::new(1_000_000_000, 4); // 1 GHz / 4 = 250 MHz
/// assert_eq!(divider.output().frequency_hz(), 250_000_000);
/// ```
pub struct ClockDivider {
    divisor: u64,
    counter: u64,
    output: Clock,
}

impl ClockDivider {
    /// Create a clock divider.
    ///
    /// # Panics
    ///
    /// Panics if `divisor` is less than 2.
    pub fn new(source_frequency_hz: u64, divisor: u64) -> Self {
        assert!(divisor >= 2, "Divisor must be >= 2, got {divisor}");
        Self {
            divisor,
            counter: 0,
            output: Clock::new(source_frequency_hz / divisor),
        }
    }

    /// Call this on every source clock edge.
    ///
    /// We only count rising edges. When we have counted `divisor` rising
    /// edges, we generate one complete output cycle (rising + falling).
    pub fn on_edge(&mut self, edge: &ClockEdge) {
        if edge.is_rising {
            self.counter += 1;
            if self.counter >= self.divisor {
                self.counter = 0;
                self.output.tick(); // rising
                self.output.tick(); // falling
            }
        }
    }

    /// Access the output clock (immutable).
    pub fn output(&self) -> &Clock {
        &self.output
    }

    /// Access the output clock (mutable, e.g., to register listeners on it).
    pub fn output_mut(&mut self) -> &mut Clock {
        &mut self.output
    }
}

// ---------------------------------------------------------------------------
// MultiPhaseClock — non-overlapping phase generation
// ---------------------------------------------------------------------------

/// Generates multiple clock phases from a single source.
///
/// Used in CPU pipelines where different stages need offset clocks.
/// A 4-phase clock generates 4 non-overlapping clock signals, each
/// active for 1/4 of the master cycle.
///
/// # Timing diagram for a 4-phase clock
///
/// ```text
/// Source:  _|^|_|^|_|^|_|^|_
/// Phase 0: _|^|___|___|___|_
/// Phase 1: _|___|^|___|___|_
/// Phase 2: _|___|___|^|___|_
/// Phase 3: _|___|___|___|^|_
/// ```
///
/// On each rising edge of the source, exactly ONE phase is active (1)
/// and all others are inactive (0). The active phase rotates.
///
/// # Example
///
/// ```
/// use clock::{ClockEdge, MultiPhaseClock};
///
/// let mut mpc = MultiPhaseClock::new(4);
/// assert_eq!(mpc.get_phase(0), 0); // No edges yet
/// ```
pub struct MultiPhaseClock {
    phases: usize,
    active_phase: usize,
    phase_values: Vec<u8>,
}

impl MultiPhaseClock {
    /// Create a multi-phase clock with the given number of phases.
    ///
    /// # Panics
    ///
    /// Panics if `phases` is less than 2.
    pub fn new(phases: usize) -> Self {
        assert!(phases >= 2, "Phases must be >= 2, got {phases}");
        Self {
            phases,
            active_phase: 0,
            phase_values: vec![0; phases],
        }
    }

    /// Call this on every source clock edge.
    ///
    /// On rising edges, we rotate the active phase. Only one phase
    /// is high at any time — this is the "non-overlapping" property
    /// that prevents pipeline hazards.
    pub fn on_edge(&mut self, edge: &ClockEdge) {
        if edge.is_rising {
            self.phase_values = vec![0; self.phases];
            self.phase_values[self.active_phase] = 1;
            self.active_phase = (self.active_phase + 1) % self.phases;
        }
    }

    /// Get current value of phase N.
    ///
    /// Returns 1 if phase is active, 0 if inactive.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range.
    pub fn get_phase(&self, index: usize) -> u8 {
        self.phase_values[index]
    }

    /// Number of phases.
    pub fn num_phases(&self) -> usize {
        self.phases
    }
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_new() {
        let clk = Clock::new(1_000_000);
        assert_eq!(clk.frequency_hz(), 1_000_000);
        assert_eq!(clk.cycle(), 0);
        assert_eq!(clk.value(), 0);
        assert_eq!(clk.total_ticks(), 0);
    }

    #[test]
    fn test_clock_tick_rising() {
        let mut clk = Clock::new(1_000_000);
        let edge = clk.tick();
        assert!(edge.is_rising);
        assert!(!edge.is_falling);
        assert_eq!(edge.value, 1);
        assert_eq!(edge.cycle, 1);
    }

    #[test]
    fn test_clock_tick_falling() {
        let mut clk = Clock::new(1_000_000);
        clk.tick(); // rising
        let edge = clk.tick(); // falling
        assert!(!edge.is_rising);
        assert!(edge.is_falling);
        assert_eq!(edge.value, 0);
        assert_eq!(edge.cycle, 1);
    }

    #[test]
    fn test_clock_full_cycle() {
        let mut clk = Clock::new(1_000_000);
        let (rising, falling) = clk.full_cycle();
        assert!(rising.is_rising);
        assert!(falling.is_falling);
        assert_eq!(clk.cycle(), 1);
        assert_eq!(clk.total_ticks(), 2);
    }

    #[test]
    fn test_clock_run() {
        let mut clk = Clock::new(1_000_000);
        let edges = clk.run(5);
        assert_eq!(edges.len(), 10);
        assert_eq!(clk.cycle(), 5);
        assert_eq!(clk.total_ticks(), 10);
    }

    #[test]
    fn test_clock_period_ns() {
        let clk = Clock::new(1_000_000_000);
        assert!((clk.period_ns() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_clock_reset() {
        let mut clk = Clock::new(1_000_000);
        clk.run(5);
        clk.reset();
        assert_eq!(clk.cycle(), 0);
        assert_eq!(clk.value(), 0);
        assert_eq!(clk.total_ticks(), 0);
    }

    #[test]
    fn test_clock_listener() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let count = Rc::new(RefCell::new(0u32));
        let count_clone = count.clone();

        let mut clk = Clock::new(1_000_000);
        clk.register_listener(Box::new(move |edge: &ClockEdge| {
            if edge.is_rising {
                *count_clone.borrow_mut() += 1;
            }
        }));

        clk.run(3);
        assert_eq!(*count.borrow(), 3);
    }

    #[test]
    #[should_panic(expected = "frequency_hz must be > 0")]
    fn test_clock_zero_frequency_panics() {
        Clock::new(0);
    }

    #[test]
    #[should_panic(expected = "Divisor must be >= 2")]
    fn test_divider_small_divisor_panics() {
        ClockDivider::new(1_000_000, 1);
    }

    #[test]
    #[should_panic(expected = "Phases must be >= 2")]
    fn test_multi_phase_small_phases_panics() {
        MultiPhaseClock::new(1);
    }
}
