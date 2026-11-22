//! # Discrete Arcade Logic Simulator (`sim-discrete-arcade-logic`)
//!
//! Cycle-accurate netlist and functional simulator for 7400-series TTL ICs,
//! arcade raster sync generators (15.75 kHz HSync / 60 Hz VSync), and
//! paddle potentiometer ADC ramp comparators (GAME15a).
//!
//! Pure `#![forbid(unsafe_code)]`, zero-heap-allocation, and deterministic execution.

#![forbid(unsafe_code)]

pub mod capabilities;
pub use capabilities::PureCapabilities;

// ---------------------------------------------------------------------------
// Constants: Arcade Timing & Physical Baselines
// ---------------------------------------------------------------------------

/// Historical arcade crystal master clock frequency: 14.31818 MHz (4x NTSC subcarrier).
pub const MASTER_CLOCK_HZ: u64 = 14_318_180;

/// Pixel dot clock frequency: 7.15909 MHz (master clock divided by 2).
pub const PIXEL_CLOCK_HZ: u64 = 7_159_090;

/// Clocks per scanline: 455 pixel clocks (approx. 15.734 kHz horizontal line rate).
pub const DOTS_PER_LINE: u16 = 455;

/// Scanlines per vertical frame: 262 scanlines (approx. 60.05 Hz vertical frame rate).
pub const SCANLINES_PER_FRAME: u16 = 262;

/// Visible active horizontal dot width: 384 pixels.
pub const ACTIVE_DOTS_WIDTH: u16 = 384;

/// Visible active vertical scanline height: 240 lines.
pub const ACTIVE_LINES_HEIGHT: u16 = 240;

/// Horizontal sync start dot coordinate.
pub const HSYNC_START_DOT: u16 = 400;

/// Horizontal sync end dot coordinate (32 pixel clocks duration).
pub const HSYNC_END_DOT: u16 = 432;

/// Vertical sync start scanline coordinate.
pub const VSYNC_START_LINE: u16 = 244;

/// Vertical sync end scanline coordinate (4 scanlines duration).
pub const VSYNC_END_LINE: u16 = 248;

/// Standard 555-timer threshold ratio in Q16 fixed point: 2/3 * 65536 = 43690.
pub const TIMER_555_THRESHOLD_Q16: u32 = 43_690;

/// Fixed-point ln(3) approximation in Q16: floor(1.098612288668 * 65536) = 71998.
pub const LN_3_Q16: u64 = 71_998;

// ---------------------------------------------------------------------------
// Error Types
// ---------------------------------------------------------------------------

/// Errors encountered in discrete logic simulation and parameter validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicError {
    /// Digital logic input was neither 0 nor 1 (CWE-20).
    InvalidLogicLevel { pin: &'static str, value: u8 },
    /// Selected channel / gate index was out of bounds (CWE-20).
    InvalidChannel { channel: usize, max: usize },
    /// Parameter validation failure (CWE-20).
    InvalidParameter {
        name: &'static str,
        reason: &'static str,
    },
    /// Division by zero prevented (CWE-369).
    DivisionByZero { component: &'static str },
    /// Counter or accumulator reached an invalid state (CWE-190).
    CounterOverflow { component: &'static str },
}

impl core::fmt::Display for LogicError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidLogicLevel { pin, value } => {
                write!(
                    f,
                    "invalid logic level on pin '{pin}': {value} (must be 0 or 1)"
                )
            }
            Self::InvalidChannel { channel, max } => {
                write!(f, "invalid channel index {channel} (max {max})")
            }
            Self::InvalidParameter { name, reason } => {
                write!(f, "invalid parameter '{name}': {reason}")
            }
            Self::DivisionByZero { component } => {
                write!(f, "division by zero on component '{component}'")
            }
            Self::CounterOverflow { component } => {
                write!(f, "counter overflow on component '{component}'")
            }
        }
    }
}

impl std::error::Error for LogicError {}

// ---------------------------------------------------------------------------
// Composite Signal Structs (Avoiding [T; N] with semicolons in signatures)
// ---------------------------------------------------------------------------

/// Four logic levels corresponding to a quad IC output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QuadLogicLevels {
    pub y0: u8,
    pub y1: u8,
    pub y2: u8,
    pub y3: u8,
}

/// Dual logic levels corresponding to a dual IC output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DualLogicLevels {
    pub y1: u8,
    pub y2: u8,
}

/// Flip-flop complementary output pair (Q and Q_bar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlipFlopOutput {
    pub q: u8,
    pub q_bar: u8,
}

/// Outputs of a 7490 BCD decade counter (QA, QB, QC, QD).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecadeOutputs {
    pub qa: u8,
    pub qb: u8,
    pub qc: u8,
    pub qd: u8,
}

impl DecadeOutputs {
    /// Returns the numerical BCD value (0..=9).
    #[inline]
    pub const fn bcd_value(&self) -> u8 {
        (self.qa & 1) | ((self.qb & 1) << 1) | ((self.qc & 1) << 2) | ((self.qd & 1) << 3)
    }
}

/// Outputs of a 7493 4-bit binary counter (QA, QB, QC, QD).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BinaryOutputs {
    pub qa: u8,
    pub qb: u8,
    pub qc: u8,
    pub qd: u8,
}

impl BinaryOutputs {
    /// Returns the numerical 4-bit binary value (0..=15).
    #[inline]
    pub const fn binary_value(&self) -> u8 {
        (self.qa & 1) | ((self.qb & 1) << 1) | ((self.qc & 1) << 2) | ((self.qd & 1) << 3)
    }
}

/// Instantaneous CRT raster beam position and sync signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RasterBeamPosition {
    pub dot_x: u16,
    pub scanline_y: u16,
    pub master_cycle: u64,
    pub hsync: bool,
    pub vsync: bool,
    pub hblank: bool,
    pub vblank: bool,
    pub active_video: bool,
}

/// State of the paddle potentiometer R-C ADC ramp comparator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaddleAdcState {
    pub cycle_count: u64,
    pub triggered: bool,
    pub trigger_pulse: bool,
    pub capacitor_voltage_q16: u32,
}

// ===========================================================================
// 1. TTL 7400 Quad 2-Input NAND Gate
// ===========================================================================

/// TTL 7400 Quad 2-Input NAND Gate simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ttl7400QuadNand;

impl Ttl7400QuadNand {
    /// Creates a new 7400 quad NAND gate simulator.
    pub const fn new_7400() -> Self {
        Self
    }

    /// Evaluates a single NAND channel functionally.
    pub fn nand_eval_gate(&self, channel: usize, a: u8, b: u8) -> Result<u8, LogicError> {
        if channel >= 4 {
            return Err(LogicError::InvalidChannel { channel, max: 4 });
        }
        if a > 1 {
            return Err(LogicError::InvalidLogicLevel { pin: "a", value: a });
        }
        if b > 1 {
            return Err(LogicError::InvalidLogicLevel { pin: "b", value: b });
        }
        Ok(if a == 1 && b == 1 { 0 } else { 1 })
    }

    /// Evaluates a single NAND channel via gate-level `logic-gates` parity.
    pub fn nand_eval_gate_level(&self, channel: usize, a: u8, b: u8) -> Result<u8, LogicError> {
        if channel >= 4 {
            return Err(LogicError::InvalidChannel { channel, max: 4 });
        }
        if a > 1 {
            return Err(LogicError::InvalidLogicLevel { pin: "a", value: a });
        }
        if b > 1 {
            return Err(LogicError::InvalidLogicLevel { pin: "b", value: b });
        }
        Ok(logic_gates::gates::nand_gate(a, b))
    }

    /// Evaluates all 4 NAND channels functionally.
    #[allow(clippy::too_many_arguments)]
    pub fn nand_eval_quad(
        &self,
        a0: u8,
        b0: u8,
        a1: u8,
        b1: u8,
        a2: u8,
        b2: u8,
        a3: u8,
        b3: u8,
    ) -> Result<QuadLogicLevels, LogicError> {
        Ok(QuadLogicLevels {
            y0: self.nand_eval_gate(0, a0, b0)?,
            y1: self.nand_eval_gate(1, a1, b1)?,
            y2: self.nand_eval_gate(2, a2, b2)?,
            y3: self.nand_eval_gate(3, a3, b3)?,
        })
    }

    /// Evaluates all 4 NAND channels via gate-level parity.
    #[allow(clippy::too_many_arguments)]
    pub fn nand_eval_quad_gate_level(
        &self,
        a0: u8,
        b0: u8,
        a1: u8,
        b1: u8,
        a2: u8,
        b2: u8,
        a3: u8,
        b3: u8,
    ) -> Result<QuadLogicLevels, LogicError> {
        Ok(QuadLogicLevels {
            y0: self.nand_eval_gate_level(0, a0, b0)?,
            y1: self.nand_eval_gate_level(1, a1, b1)?,
            y2: self.nand_eval_gate_level(2, a2, b2)?,
            y3: self.nand_eval_gate_level(3, a3, b3)?,
        })
    }
}

// ===========================================================================
// 2. TTL 7402 Quad 2-Input NOR Gate
// ===========================================================================

/// TTL 7402 Quad 2-Input NOR Gate simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ttl7402QuadNor;

impl Ttl7402QuadNor {
    /// Creates a new 7402 quad NOR gate simulator.
    pub const fn new_7402() -> Self {
        Self
    }

    /// Evaluates a single NOR channel functionally.
    pub fn nor_eval_gate(&self, channel: usize, a: u8, b: u8) -> Result<u8, LogicError> {
        if channel >= 4 {
            return Err(LogicError::InvalidChannel { channel, max: 4 });
        }
        if a > 1 {
            return Err(LogicError::InvalidLogicLevel { pin: "a", value: a });
        }
        if b > 1 {
            return Err(LogicError::InvalidLogicLevel { pin: "b", value: b });
        }
        Ok(if a == 0 && b == 0 { 1 } else { 0 })
    }

    /// Evaluates a single NOR channel via gate-level `logic-gates` parity.
    pub fn nor_eval_gate_level(&self, channel: usize, a: u8, b: u8) -> Result<u8, LogicError> {
        if channel >= 4 {
            return Err(LogicError::InvalidChannel { channel, max: 4 });
        }
        if a > 1 {
            return Err(LogicError::InvalidLogicLevel { pin: "a", value: a });
        }
        if b > 1 {
            return Err(LogicError::InvalidLogicLevel { pin: "b", value: b });
        }
        Ok(logic_gates::gates::nor_gate(a, b))
    }

    /// Evaluates all 4 NOR channels functionally.
    #[allow(clippy::too_many_arguments)]
    pub fn nor_eval_quad(
        &self,
        a0: u8,
        b0: u8,
        a1: u8,
        b1: u8,
        a2: u8,
        b2: u8,
        a3: u8,
        b3: u8,
    ) -> Result<QuadLogicLevels, LogicError> {
        Ok(QuadLogicLevels {
            y0: self.nor_eval_gate(0, a0, b0)?,
            y1: self.nor_eval_gate(1, a1, b1)?,
            y2: self.nor_eval_gate(2, a2, b2)?,
            y3: self.nor_eval_gate(3, a3, b3)?,
        })
    }

    /// Evaluates all 4 NOR channels via gate-level parity.
    #[allow(clippy::too_many_arguments)]
    pub fn nor_eval_quad_gate_level(
        &self,
        a0: u8,
        b0: u8,
        a1: u8,
        b1: u8,
        a2: u8,
        b2: u8,
        a3: u8,
        b3: u8,
    ) -> Result<QuadLogicLevels, LogicError> {
        Ok(QuadLogicLevels {
            y0: self.nor_eval_gate_level(0, a0, b0)?,
            y1: self.nor_eval_gate_level(1, a1, b1)?,
            y2: self.nor_eval_gate_level(2, a2, b2)?,
            y3: self.nor_eval_gate_level(3, a3, b3)?,
        })
    }
}

// ===========================================================================
// 3. TTL 7474 Dual D-Type Edge-Triggered Flip-Flop
// ===========================================================================

/// Internal channel state of one D flip-flop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ttl7474ChannelState {
    pub q: u8,
    pub q_bar: u8,
    pub last_clk: u8,
}

impl Default for Ttl7474ChannelState {
    fn default() -> Self {
        Self {
            q: 0,
            q_bar: 1,
            last_clk: 0,
        }
    }
}

/// TTL 7474 Dual D-Type Positive-Edge-Triggered Flip-Flop with async Preset and Clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ttl7474DualDFlipFlop {
    pub ch0: Ttl7474ChannelState,
    pub ch1: Ttl7474ChannelState,
}

impl Ttl7474DualDFlipFlop {
    /// Creates a new 7474 dual D flip-flop simulator in default reset state.
    pub const fn new_7474() -> Self {
        Self {
            ch0: Ttl7474ChannelState {
                q: 0,
                q_bar: 1,
                last_clk: 0,
            },
            ch1: Ttl7474ChannelState {
                q: 0,
                q_bar: 1,
                last_clk: 0,
            },
        }
    }

    fn channel_ref_mut(&mut self, channel: usize) -> Result<&mut Ttl7474ChannelState, LogicError> {
        match channel {
            0 => Ok(&mut self.ch0),
            1 => Ok(&mut self.ch1),
            _ => Err(LogicError::InvalidChannel { channel, max: 2 }),
        }
    }

    fn channel_ref(&self, channel: usize) -> Result<&Ttl7474ChannelState, LogicError> {
        match channel {
            0 => Ok(&self.ch0),
            1 => Ok(&self.ch1),
            _ => Err(LogicError::InvalidChannel { channel, max: 2 }),
        }
    }

    /// Steps a channel functionally with clock, data, and active-low async preset/clear.
    pub fn dff_step_channel(
        &mut self,
        channel: usize,
        clk: u8,
        d: u8,
        pre_b: u8,
        clr_b: u8,
    ) -> Result<FlipFlopOutput, LogicError> {
        if clk > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "clk",
                value: clk,
            });
        }
        if d > 1 {
            return Err(LogicError::InvalidLogicLevel { pin: "d", value: d });
        }
        if pre_b > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "pre_b",
                value: pre_b,
            });
        }
        if clr_b > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "clr_b",
                value: clr_b,
            });
        }

        let st = self.channel_ref_mut(channel)?;
        if pre_b == 0 && clr_b == 0 {
            // Unstable state per TI datasheet: both outputs forced high
            st.q = 1;
            st.q_bar = 1;
        } else if pre_b == 0 {
            st.q = 1;
            st.q_bar = 0;
        } else if clr_b == 0 {
            st.q = 0;
            st.q_bar = 1;
        } else if st.last_clk == 0 && clk == 1 {
            // Rising edge trigger
            st.q = d;
            st.q_bar = if d == 1 { 0 } else { 1 };
        }
        st.last_clk = clk;

        Ok(FlipFlopOutput {
            q: st.q,
            q_bar: st.q_bar,
        })
    }

    /// Steps a channel via 6-NAND gate network parity using `logic-gates`.
    pub fn dff_step_channel_gate_level(
        &mut self,
        channel: usize,
        clk: u8,
        d: u8,
        pre_b: u8,
        clr_b: u8,
    ) -> Result<FlipFlopOutput, LogicError> {
        if clk > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "clk",
                value: clk,
            });
        }
        if d > 1 {
            return Err(LogicError::InvalidLogicLevel { pin: "d", value: d });
        }
        if pre_b > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "pre_b",
                value: pre_b,
            });
        }
        if clr_b > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "clr_b",
                value: clr_b,
            });
        }

        let st = self.channel_ref_mut(channel)?;

        if pre_b == 0 && clr_b == 0 {
            st.q = 1;
            st.q_bar = 1;
        } else if pre_b == 0 {
            st.q = 1;
            st.q_bar = 0;
        } else if clr_b == 0 {
            st.q = 0;
            st.q_bar = 1;
        } else {
            let rising_edge = st.last_clk == 0 && clk == 1;
            if rising_edge {
                // Classical 6-NAND gate D-FF latch model:
                // G4 = NAND(G3, PRE_b, D)
                // G1 = NAND(G4, PRE_b, G2)
                // On rising edge:
                // If D=1: G4=0 -> G1=1 -> G2=0 -> Q=1, Q_bar=0
                // If D=0: G4=1 -> G1=0 -> G2=1, G3=0 -> Q_bar=1, Q=0
                let not_d = logic_gates::gates::not_gate(d);
                let q_new =
                    logic_gates::gates::nand_gate(pre_b, logic_gates::gates::nand_gate(d, clk));
                let q_bar_new =
                    logic_gates::gates::nand_gate(clr_b, logic_gates::gates::nand_gate(not_d, clk));
                st.q = if d == 1 { q_new } else { 0 };
                st.q_bar = if d == 1 { 0 } else { q_bar_new };
            }
        }
        st.last_clk = clk;

        Ok(FlipFlopOutput {
            q: st.q,
            q_bar: st.q_bar,
        })
    }

    /// Reads the current Q output of a channel.
    pub fn dff_q_channel(&self, channel: usize) -> Result<u8, LogicError> {
        let st = self.channel_ref(channel)?;
        Ok(st.q)
    }

    /// Reads the current Q_bar output of a channel.
    pub fn dff_q_bar_channel(&self, channel: usize) -> Result<u8, LogicError> {
        let st = self.channel_ref(channel)?;
        Ok(st.q_bar)
    }

    /// Resets a channel to Q=0, Q_bar=1.
    pub fn dff_reset_channel(&mut self, channel: usize) -> Result<(), LogicError> {
        let st = self.channel_ref_mut(channel)?;
        st.q = 0;
        st.q_bar = 1;
        st.last_clk = 0;
        Ok(())
    }
}

// ===========================================================================
// 4. TTL 7490 Decade Counter (BCD Divide-by-2 + Divide-by-5)
// ===========================================================================

/// TTL 7490 Decade Counter simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ttl7490DecadeCounter {
    pub qa: u8,
    pub qb: u8,
    pub qc: u8,
    pub qd: u8,
    pub last_clk_a: u8,
    pub last_clk_b: u8,
}

impl Ttl7490DecadeCounter {
    /// Creates a new 7490 decade counter initialized to 0000.
    pub const fn new_7490() -> Self {
        Self {
            qa: 0,
            qb: 0,
            qc: 0,
            qd: 0,
            last_clk_a: 1,
            last_clk_b: 1,
        }
    }

    /// Clocks Section A (divide-by-2) on negative (falling) edge.
    pub fn decade_clock_a(&mut self, clk_a: u8) -> Result<u8, LogicError> {
        if clk_a > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "clk_a",
                value: clk_a,
            });
        }
        if self.last_clk_a == 1 && clk_a == 0 {
            // Falling edge toggles QA
            self.qa = if self.qa == 0 { 1 } else { 0 };
        }
        self.last_clk_a = clk_a;
        Ok(self.qa)
    }

    /// Clocks Section B (divide-by-5) on negative (falling) edge.
    pub fn decade_clock_b(&mut self, clk_b: u8) -> Result<DecadeOutputs, LogicError> {
        if clk_b > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "clk_b",
                value: clk_b,
            });
        }
        if self.last_clk_b == 1 && clk_b == 0 {
            // Divide-by-5 state sequence: 000 -> 001 -> 010 -> 011 -> 100 -> 000
            let state = (self.qd << 2) | (self.qc << 1) | self.qb;
            let next_state = match state {
                0 => 1,
                1 => 2,
                2 => 3,
                3 => 4,
                _ => 0,
            };
            self.qb = next_state & 1;
            self.qc = (next_state >> 1) & 1;
            self.qd = (next_state >> 2) & 1;
        }
        self.last_clk_b = clk_b;
        Ok(self.decade_outputs())
    }

    /// Clocks Section A via gate-level parity.
    pub fn decade_clock_a_gate_level(&mut self, clk_a: u8) -> Result<u8, LogicError> {
        if clk_a > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "clk_a",
                value: clk_a,
            });
        }
        if self.last_clk_a == 1 && clk_a == 0 {
            self.qa = logic_gates::gates::xor_gate(self.qa, 1);
        }
        self.last_clk_a = clk_a;
        Ok(self.qa)
    }

    /// Clocks Section B via gate-level JK flip-flop excitation equations.
    pub fn decade_clock_b_gate_level(&mut self, clk_b: u8) -> Result<DecadeOutputs, LogicError> {
        if clk_b > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "clk_b",
                value: clk_b,
            });
        }
        if self.last_clk_b == 1 && clk_b == 0 {
            let not_qd = logic_gates::gates::not_gate(self.qd);
            let j_b = not_qd;
            let k_b = 1u8;
            let j_c = self.qb;
            let k_c = self.qb;
            let j_d = logic_gates::gates::and_gate(self.qb, self.qc);
            let k_d = 1u8;

            let next_qb = logic_gates::gates::or_gate(
                logic_gates::gates::and_gate(j_b, logic_gates::gates::not_gate(self.qb)),
                logic_gates::gates::and_gate(logic_gates::gates::not_gate(k_b), self.qb),
            );
            let next_qc = logic_gates::gates::or_gate(
                logic_gates::gates::and_gate(j_c, logic_gates::gates::not_gate(self.qc)),
                logic_gates::gates::and_gate(logic_gates::gates::not_gate(k_c), self.qc),
            );
            let next_qd = logic_gates::gates::or_gate(
                logic_gates::gates::and_gate(j_d, logic_gates::gates::not_gate(self.qd)),
                logic_gates::gates::and_gate(logic_gates::gates::not_gate(k_d), self.qd),
            );

            self.qb = next_qb;
            self.qc = next_qc;
            self.qd = next_qd;
        }
        self.last_clk_b = clk_b;
        Ok(self.decade_outputs())
    }

    /// Applies asynchronous reset (R0) or preset to 9 (R9).
    pub fn decade_reset(
        &mut self,
        r0_1: u8,
        r0_2: u8,
        r9_1: u8,
        r9_2: u8,
    ) -> Result<(), LogicError> {
        if r0_1 > 1 || r0_2 > 1 || r9_1 > 1 || r9_2 > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "reset",
                value: 2,
            });
        }
        let set9 = logic_gates::gates::and_gate(r9_1, r9_2);
        let reset0 = logic_gates::gates::and_gate(r0_1, r0_2);

        if set9 == 1 {
            self.qa = 1;
            self.qb = 0;
            self.qc = 0;
            self.qd = 1;
        } else if reset0 == 1 {
            self.qa = 0;
            self.qb = 0;
            self.qc = 0;
            self.qd = 0;
        }
        Ok(())
    }

    /// Steps a complete BCD decade counter (QA wired to CLK_B, input on CLK_A).
    pub fn decade_step_bcd(&mut self, clk: u8) -> Result<DecadeOutputs, LogicError> {
        let old_qa = self.qa;
        self.decade_clock_a(clk)?;
        if old_qa == 1 && self.qa == 0 {
            // QA falling edge clocks section B
            self.decade_clock_b(0)?;
            self.decade_clock_b(1)?;
        }
        Ok(self.decade_outputs())
    }

    /// Steps a complete BCD decade counter via gate-level parity.
    pub fn decade_step_bcd_gate_level(&mut self, clk: u8) -> Result<DecadeOutputs, LogicError> {
        let old_qa = self.qa;
        self.decade_clock_a_gate_level(clk)?;
        if old_qa == 1 && self.qa == 0 {
            self.decade_clock_b_gate_level(0)?;
            self.decade_clock_b_gate_level(1)?;
        }
        Ok(self.decade_outputs())
    }

    /// Reads the current numerical BCD value (0..=9).
    #[inline]
    pub const fn decade_read_bcd(&self) -> u8 {
        (self.qa & 1) | ((self.qb & 1) << 1) | ((self.qc & 1) << 2) | ((self.qd & 1) << 3)
    }

    /// Reads all 4 decade counter outputs.
    #[inline]
    pub const fn decade_outputs(&self) -> DecadeOutputs {
        DecadeOutputs {
            qa: self.qa,
            qb: self.qb,
            qc: self.qc,
            qd: self.qd,
        }
    }
}

// ===========================================================================
// 5. TTL 7493 4-Bit Binary Ripple Counter
// ===========================================================================

/// TTL 7493 4-Bit Binary Ripple Counter simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ttl7493BinaryCounter {
    pub qa: u8,
    pub qb: u8,
    pub qc: u8,
    pub qd: u8,
    pub last_clk_a: u8,
    pub last_clk_b: u8,
}

impl Ttl7493BinaryCounter {
    /// Creates a new 7493 4-bit binary counter initialized to 0.
    pub const fn new_7493() -> Self {
        Self {
            qa: 0,
            qb: 0,
            qc: 0,
            qd: 0,
            last_clk_a: 1,
            last_clk_b: 1,
        }
    }

    /// Clocks Section A (divide-by-2) on negative (falling) edge.
    pub fn binary_clock_a(&mut self, clk_a: u8) -> Result<u8, LogicError> {
        if clk_a > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "clk_a",
                value: clk_a,
            });
        }
        if self.last_clk_a == 1 && clk_a == 0 {
            self.qa = if self.qa == 0 { 1 } else { 0 };
        }
        self.last_clk_a = clk_a;
        Ok(self.qa)
    }

    /// Clocks Section B (divide-by-8 ripple) on negative (falling) edge.
    pub fn binary_clock_b(&mut self, clk_b: u8) -> Result<BinaryOutputs, LogicError> {
        if clk_b > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "clk_b",
                value: clk_b,
            });
        }
        if self.last_clk_b == 1 && clk_b == 0 {
            // Ripple counter: QB toggles on CLK_B falling edge
            let old_qb = self.qb;
            self.qb = if self.qb == 0 { 1 } else { 0 };
            if old_qb == 1 && self.qb == 0 {
                // QC toggles on QB falling edge
                let old_qc = self.qc;
                self.qc = if self.qc == 0 { 1 } else { 0 };
                if old_qc == 1 && self.qc == 0 {
                    // QD toggles on QC falling edge
                    self.qd = if self.qd == 0 { 1 } else { 0 };
                }
            }
        }
        self.last_clk_b = clk_b;
        Ok(self.binary_outputs())
    }

    /// Clocks Section A via gate-level XOR toggle.
    pub fn binary_clock_a_gate_level(&mut self, clk_a: u8) -> Result<u8, LogicError> {
        if clk_a > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "clk_a",
                value: clk_a,
            });
        }
        if self.last_clk_a == 1 && clk_a == 0 {
            self.qa = logic_gates::gates::xor_gate(self.qa, 1);
        }
        self.last_clk_a = clk_a;
        Ok(self.qa)
    }

    /// Clocks Section B via gate-level ripple evaluation.
    pub fn binary_clock_b_gate_level(&mut self, clk_b: u8) -> Result<BinaryOutputs, LogicError> {
        if clk_b > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "clk_b",
                value: clk_b,
            });
        }
        if self.last_clk_b == 1 && clk_b == 0 {
            let old_qb = self.qb;
            self.qb = logic_gates::gates::xor_gate(self.qb, 1);
            if old_qb == 1 && self.qb == 0 {
                let old_qc = self.qc;
                self.qc = logic_gates::gates::xor_gate(self.qc, 1);
                if old_qc == 1 && self.qc == 0 {
                    self.qd = logic_gates::gates::xor_gate(self.qd, 1);
                }
            }
        }
        self.last_clk_b = clk_b;
        Ok(self.binary_outputs())
    }

    /// Resets all 4 bits to 0 when both R0_1 and R0_2 are high.
    pub fn binary_reset(&mut self, r0_1: u8, r0_2: u8) -> Result<(), LogicError> {
        if r0_1 > 1 || r0_2 > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "reset",
                value: 2,
            });
        }
        if r0_1 == 1 && r0_2 == 1 {
            self.qa = 0;
            self.qb = 0;
            self.qc = 0;
            self.qd = 0;
        }
        Ok(())
    }

    /// Steps a complete 4-bit ripple counter (QA wired to CLK_B, input on CLK_A).
    pub fn binary_step_4bit(&mut self, clk: u8) -> Result<BinaryOutputs, LogicError> {
        let old_qa = self.qa;
        self.binary_clock_a(clk)?;
        if old_qa == 1 && self.qa == 0 {
            self.binary_clock_b(0)?;
            self.binary_clock_b(1)?;
        }
        Ok(self.binary_outputs())
    }

    /// Steps a complete 4-bit ripple counter via gate-level parity.
    pub fn binary_step_4bit_gate_level(&mut self, clk: u8) -> Result<BinaryOutputs, LogicError> {
        let old_qa = self.qa;
        self.binary_clock_a_gate_level(clk)?;
        if old_qa == 1 && self.qa == 0 {
            self.binary_clock_b_gate_level(0)?;
            self.binary_clock_b_gate_level(1)?;
        }
        Ok(self.binary_outputs())
    }

    /// Reads the current numerical 4-bit binary value (0..=15).
    #[inline]
    pub const fn binary_read_u8(&self) -> u8 {
        (self.qa & 1) | ((self.qb & 1) << 1) | ((self.qc & 1) << 2) | ((self.qd & 1) << 3)
    }

    /// Reads all 4 binary counter outputs.
    #[inline]
    pub const fn binary_outputs(&self) -> BinaryOutputs {
        BinaryOutputs {
            qa: self.qa,
            qb: self.qb,
            qc: self.qc,
            qd: self.qd,
        }
    }
}

// ===========================================================================
// 6. TTL 74153 Dual 4-to-1 Multiplexer with Strobe
// ===========================================================================

/// TTL 74153 Dual 4-Line to 1-Line Data Selector / Multiplexer simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ttl74153DualMux4to1;

impl Ttl74153DualMux4to1 {
    /// Creates a new 74153 dual 4-to-1 multiplexer simulator.
    pub const fn new_74153() -> Self {
        Self
    }

    /// Evaluates a single multiplexer section functionally.
    #[allow(clippy::too_many_arguments)]
    pub fn mux_eval_section(
        &self,
        section: usize,
        s0: u8,
        s1: u8,
        strobe_b: u8,
        c0: u8,
        c1: u8,
        c2: u8,
        c3: u8,
    ) -> Result<u8, LogicError> {
        if section >= 2 {
            return Err(LogicError::InvalidChannel {
                channel: section,
                max: 2,
            });
        }
        if s0 > 1 || s1 > 1 || strobe_b > 1 || c0 > 1 || c1 > 1 || c2 > 1 || c3 > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "mux_input",
                value: 2,
            });
        }
        if strobe_b != 0 {
            return Ok(0);
        }
        let sel = (s1 << 1) | s0;
        Ok(match sel {
            0 => c0,
            1 => c1,
            2 => c2,
            _ => c3,
        })
    }

    /// Evaluates a single multiplexer section via gate-level parity (`logic-gates`).
    #[allow(clippy::too_many_arguments)]
    pub fn mux_eval_section_gate_level(
        &self,
        section: usize,
        s0: u8,
        s1: u8,
        strobe_b: u8,
        c0: u8,
        c1: u8,
        c2: u8,
        c3: u8,
    ) -> Result<u8, LogicError> {
        if section >= 2 {
            return Err(LogicError::InvalidChannel {
                channel: section,
                max: 2,
            });
        }
        if s0 > 1 || s1 > 1 || strobe_b > 1 || c0 > 1 || c1 > 1 || c2 > 1 || c3 > 1 {
            return Err(LogicError::InvalidLogicLevel {
                pin: "mux_input",
                value: 2,
            });
        }

        let not_s0 = logic_gates::gates::not_gate(s0);
        let not_s1 = logic_gates::gates::not_gate(s1);
        let not_strobe = logic_gates::gates::not_gate(strobe_b);

        let m0 = logic_gates::gates::and_gate(
            logic_gates::gates::and_gate(c0, not_strobe),
            logic_gates::gates::and_gate(not_s1, not_s0),
        );
        let m1 = logic_gates::gates::and_gate(
            logic_gates::gates::and_gate(c1, not_strobe),
            logic_gates::gates::and_gate(not_s1, s0),
        );
        let m2 = logic_gates::gates::and_gate(
            logic_gates::gates::and_gate(c2, not_strobe),
            logic_gates::gates::and_gate(s1, not_s0),
        );
        let m3 = logic_gates::gates::and_gate(
            logic_gates::gates::and_gate(c3, not_strobe),
            logic_gates::gates::and_gate(s1, s0),
        );

        let out = logic_gates::gates::or_gate(
            logic_gates::gates::or_gate(m0, m1),
            logic_gates::gates::or_gate(m2, m3),
        );
        Ok(out)
    }

    /// Evaluates both multiplexer sections functionally.
    pub fn mux_eval_dual(
        &self,
        s0: u8,
        s1: u8,
        str1: u8,
        in1: QuadLogicLevels,
        str2: u8,
        in2: QuadLogicLevels,
    ) -> Result<DualLogicLevels, LogicError> {
        Ok(DualLogicLevels {
            y1: self.mux_eval_section(0, s0, s1, str1, in1.y0, in1.y1, in1.y2, in1.y3)?,
            y2: self.mux_eval_section(1, s0, s1, str2, in2.y0, in2.y1, in2.y2, in2.y3)?,
        })
    }

    /// Evaluates both multiplexer sections via gate-level parity.
    pub fn mux_eval_dual_gate_level(
        &self,
        s0: u8,
        s1: u8,
        str1: u8,
        in1: QuadLogicLevels,
        str2: u8,
        in2: QuadLogicLevels,
    ) -> Result<DualLogicLevels, LogicError> {
        Ok(DualLogicLevels {
            y1: self
                .mux_eval_section_gate_level(0, s0, s1, str1, in1.y0, in1.y1, in1.y2, in1.y3)?,
            y2: self
                .mux_eval_section_gate_level(1, s0, s1, str2, in2.y0, in2.y1, in2.y2, in2.y3)?,
        })
    }
}

// ===========================================================================
// 7. ArcadeSyncGenerator (Master Clock Divider & Raster Timing Chain)
// ===========================================================================

/// Arcade Raster Sync Generator simulating the discrete clock divider netlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArcadeSyncGenerator {
    pub master_clock_hz: u64,
    pub dot_x: u16,
    pub scanline_y: u16,
    pub master_phase: u8,
    pub master_cycles: u64,
    // Gate-level divider netlist components:
    pub pixel_dff: Ttl7474ChannelState,
    pub h_counter_lo: Ttl7493BinaryCounter,
    pub h_counter_mid: Ttl7493BinaryCounter,
    pub h_counter_hi: Ttl7493BinaryCounter,
    pub v_counter_lo: Ttl7493BinaryCounter,
    pub v_counter_mid: Ttl7493BinaryCounter,
    pub v_counter_hi: Ttl7493BinaryCounter,
}

impl ArcadeSyncGenerator {
    /// Constructs a new sync generator with master clock frequency validation (CWE-369).
    pub fn new_arcade_sync(master_clock_hz: u64) -> Result<Self, LogicError> {
        if master_clock_hz == 0 {
            return Err(LogicError::DivisionByZero {
                component: "master_clock_hz",
            });
        }
        Ok(Self {
            master_clock_hz,
            dot_x: 0,
            scanline_y: 0,
            master_phase: 0,
            master_cycles: 0,
            pixel_dff: Ttl7474ChannelState::default(),
            h_counter_lo: Ttl7493BinaryCounter::new_7493(),
            h_counter_mid: Ttl7493BinaryCounter::new_7493(),
            h_counter_hi: Ttl7493BinaryCounter::new_7493(),
            v_counter_lo: Ttl7493BinaryCounter::new_7493(),
            v_counter_mid: Ttl7493BinaryCounter::new_7493(),
            v_counter_hi: Ttl7493BinaryCounter::new_7493(),
        })
    }

    /// Advances the sync generator by 1 master crystal clock cycle functionally.
    pub fn sync_step_master_clock(&mut self) -> RasterBeamPosition {
        self.master_cycles = self.master_cycles.saturating_add(1);
        let old_phase = self.master_phase;
        self.master_phase ^= 1;

        // Every 2 master crystal cycles = 1 pixel clock (14.31818 MHz / 2 = 7.15909 MHz)
        if old_phase == 1 && self.master_phase == 0 {
            self.dot_x += 1;
            if self.dot_x >= DOTS_PER_LINE {
                self.dot_x = 0;
                self.scanline_y += 1;
                if self.scanline_y >= SCANLINES_PER_FRAME {
                    self.scanline_y = 0;
                }
            }
        }

        self.sync_raster_state()
    }

    /// Advances the sync generator by 1 master clock cycle via gate-level divider chain.
    pub fn sync_step_master_clock_gate_level(&mut self) -> RasterBeamPosition {
        self.master_cycles = self.master_cycles.saturating_add(1);

        // Step master /2 pixel D-FF divider on every master clock cycle
        let old_q = self.pixel_dff.q;
        let next_q = logic_gates::gates::not_gate(self.pixel_dff.q);
        self.pixel_dff.q = next_q;
        self.pixel_dff.q_bar = logic_gates::gates::not_gate(next_q);
        self.pixel_dff.last_clk = (self.master_cycles & 1) as u8;

        // On falling edge of pixel D-FF output (every 2 master cycles), step 7493 H/V divider chain
        if old_q == 1 && self.pixel_dff.q == 0 {
            let prev_h_lo_qd = self.h_counter_lo.qd;
            let _ = self.h_counter_lo.binary_step_4bit_gate_level(1);
            let _ = self.h_counter_lo.binary_step_4bit_gate_level(0);
            if prev_h_lo_qd == 1 && self.h_counter_lo.qd == 0 {
                let prev_h_mid_qd = self.h_counter_mid.qd;
                let _ = self.h_counter_mid.binary_step_4bit_gate_level(1);
                let _ = self.h_counter_mid.binary_step_4bit_gate_level(0);
                if prev_h_mid_qd == 1 && self.h_counter_mid.qd == 0 {
                    let _ = self.h_counter_hi.binary_step_4bit_gate_level(1);
                    let _ = self.h_counter_hi.binary_step_4bit_gate_level(0);
                }
            }
            let nibble = |b: BinaryOutputs| -> u16 {
                (b.qa as u16) | ((b.qb as u16) << 1) | ((b.qc as u16) << 2) | ((b.qd as u16) << 3)
            };
            let h_val = nibble(self.h_counter_lo.binary_outputs())
                | (nibble(self.h_counter_mid.binary_outputs()) << 4)
                | (nibble(self.h_counter_hi.binary_outputs()) << 8);

            if h_val >= DOTS_PER_LINE {
                let _ = self.h_counter_lo.binary_reset(1, 1);
                let _ = self.h_counter_mid.binary_reset(1, 1);
                let _ = self.h_counter_hi.binary_reset(1, 1);
                self.dot_x = 0;

                let prev_v_lo_qd = self.v_counter_lo.qd;
                let _ = self.v_counter_lo.binary_step_4bit_gate_level(1);
                let _ = self.v_counter_lo.binary_step_4bit_gate_level(0);
                if prev_v_lo_qd == 1 && self.v_counter_lo.qd == 0 {
                    let prev_v_mid_qd = self.v_counter_mid.qd;
                    let _ = self.v_counter_mid.binary_step_4bit_gate_level(1);
                    let _ = self.v_counter_mid.binary_step_4bit_gate_level(0);
                    if prev_v_mid_qd == 1 && self.v_counter_mid.qd == 0 {
                        let _ = self.v_counter_hi.binary_step_4bit_gate_level(1);
                        let _ = self.v_counter_hi.binary_step_4bit_gate_level(0);
                    }
                }
                let v_val = nibble(self.v_counter_lo.binary_outputs())
                    | (nibble(self.v_counter_mid.binary_outputs()) << 4)
                    | (nibble(self.v_counter_hi.binary_outputs()) << 8);

                if v_val >= SCANLINES_PER_FRAME {
                    let _ = self.v_counter_lo.binary_reset(1, 1);
                    let _ = self.v_counter_mid.binary_reset(1, 1);
                    let _ = self.v_counter_hi.binary_reset(1, 1);
                    self.scanline_y = 0;
                } else {
                    self.scanline_y = v_val;
                }
            } else {
                self.dot_x = h_val;
            }
        }
        self.master_phase = self.pixel_dff.q;

        self.sync_raster_state()
    }

    /// Advances the raster beam by an integer count of pixel clocks.
    pub fn sync_step_pixel_clocks(&mut self, count: u32) -> RasterBeamPosition {
        for _ in 0..count {
            self.sync_step_master_clock();
            self.sync_step_master_clock();
        }
        self.sync_raster_state()
    }

    /// Reads the instantaneous raster beam position and discrete sync signals.
    #[inline]
    pub const fn sync_raster_state(&self) -> RasterBeamPosition {
        let hsync = self.dot_x >= HSYNC_START_DOT && self.dot_x < HSYNC_END_DOT;
        let vsync = self.scanline_y >= VSYNC_START_LINE && self.scanline_y < VSYNC_END_LINE;
        let hblank = self.dot_x >= ACTIVE_DOTS_WIDTH;
        let vblank = self.scanline_y >= ACTIVE_LINES_HEIGHT;
        let active_video = !hblank && !vblank;

        RasterBeamPosition {
            dot_x: self.dot_x,
            scanline_y: self.scanline_y,
            master_cycle: self.master_cycles,
            hsync,
            vsync,
            hblank,
            vblank,
            active_video,
        }
    }

    /// Returns true if horizontal sync pulse is currently asserted.
    #[inline]
    pub const fn sync_hsync(&self) -> bool {
        self.dot_x >= HSYNC_START_DOT && self.dot_x < HSYNC_END_DOT
    }

    /// Returns true if vertical sync pulse is currently asserted.
    #[inline]
    pub const fn sync_vsync(&self) -> bool {
        self.scanline_y >= VSYNC_START_LINE && self.scanline_y < VSYNC_END_LINE
    }

    /// Returns true if horizontal blanking interval is active.
    #[inline]
    pub const fn sync_hblank(&self) -> bool {
        self.dot_x >= ACTIVE_DOTS_WIDTH
    }

    /// Returns true if vertical blanking interval is active.
    #[inline]
    pub const fn sync_vblank(&self) -> bool {
        self.scanline_y >= ACTIVE_LINES_HEIGHT
    }

    /// Returns true if the beam is inside the visible active raster area.
    #[inline]
    pub const fn sync_is_active_video(&self) -> bool {
        self.dot_x < ACTIVE_DOTS_WIDTH && self.scanline_y < ACTIVE_LINES_HEIGHT
    }

    /// Resets the sync generator to the start of frame (dot 0, scanline 0).
    pub fn sync_reset(&mut self) {
        self.dot_x = 0;
        self.scanline_y = 0;
        self.master_phase = 0;
        self.pixel_dff = Ttl7474ChannelState::default();
        let _ = self.h_counter_lo.binary_reset(1, 1);
        let _ = self.h_counter_mid.binary_reset(1, 1);
        let _ = self.h_counter_hi.binary_reset(1, 1);
        let _ = self.v_counter_lo.binary_reset(1, 1);
        let _ = self.v_counter_mid.binary_reset(1, 1);
        let _ = self.v_counter_hi.binary_reset(1, 1);
    }
}

// ===========================================================================
// 8. PaddlePotAdc (555-Timer R-C Ramp Comparator)
// ===========================================================================

/// Paddle Potentiometer Analog-to-Digital Converter simulator.
///
/// Models 555-timer / R-C exponential ramp charging behavior converting
/// normalized paddle potentiometer resistance `[0, 65535]` into deterministic
/// clock-counted scanline trigger pulses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaddlePotAdc {
    pub clock_hz: u64,
    pub capacitance_pf: u64,
    pub max_pot_resistance_ohms: u64,
    pub min_resistance_ohms: u64,
    pub cycle_count: u64,
    pub triggered: bool,
    pub last_triggered: bool,
}

impl PaddlePotAdc {
    /// Constructs a new paddle ADC model with clock, capacitance, and resistance checks (CWE-369).
    pub fn new_paddle_pot_adc(
        clock_hz: u64,
        capacitance_pf: u64,
        max_pot_resistance_ohms: u64,
    ) -> Result<Self, LogicError> {
        if clock_hz == 0 {
            return Err(LogicError::DivisionByZero {
                component: "clock_hz",
            });
        }
        if capacitance_pf == 0 {
            return Err(LogicError::DivisionByZero {
                component: "capacitance_pf",
            });
        }
        if max_pot_resistance_ohms == 0 {
            return Err(LogicError::DivisionByZero {
                component: "max_pot_resistance_ohms",
            });
        }
        Ok(Self {
            clock_hz,
            capacitance_pf,
            max_pot_resistance_ohms,
            min_resistance_ohms: 1_000, // 1k ohm minimum fixed series resistor
            cycle_count: 0,
            triggered: false,
            last_triggered: false,
        })
    }

    /// Triggers capacitor discharge (typically driven at VBlank / frame start).
    pub fn paddle_trigger_discharge(&mut self) {
        self.cycle_count = 0;
        self.triggered = false;
        self.last_triggered = false;
    }

    /// Computes the exact clock duration required for the R-C ramp to reach 2/3 Vcc.
    pub fn paddle_charge_duration_clocks(
        &self,
        pot_normalized_u16: u16,
    ) -> Result<u64, LogicError> {
        // R_total = R_min + (pot_u16 * R_max) / 65535
        let var_r = ((pot_normalized_u16 as u128)
            .saturating_mul(self.max_pot_resistance_ohms as u128))
            / 65535;
        let r_total = (self.min_resistance_ohms as u128).saturating_add(var_r);

        // Clocks = (R * C * ln(3) * f_clk)
        // C is in pF (10^-12 F), ln(3) in Q16 is 71998, denom = 10^12 * 65536
        let num = r_total
            .saturating_mul(self.capacitance_pf as u128)
            .saturating_mul(self.clock_hz as u128)
            .saturating_mul(LN_3_Q16 as u128);
        let den = 65_536_000_000_000_000u128; // 10^12 * 65536
        let duration = (num / den).min(u64::MAX as u128) as u64;
        Ok(duration)
    }

    /// Steps the ADC ramp comparator by 1 clock cycle for a given paddle knob position.
    pub fn paddle_step_clock(
        &mut self,
        pot_normalized_u16: u16,
    ) -> Result<PaddleAdcState, LogicError> {
        let target_clocks = self.paddle_charge_duration_clocks(pot_normalized_u16)?;
        self.cycle_count = self.cycle_count.saturating_add(1);

        let is_trig = self.cycle_count >= target_clocks;
        let pulse = is_trig && !self.last_triggered;
        self.triggered = is_trig;
        self.last_triggered = is_trig;

        let v_q16 = if is_trig {
            TIMER_555_THRESHOLD_Q16
        } else if target_clocks > 0 {
            (((TIMER_555_THRESHOLD_Q16 as u128) * (self.cycle_count as u128))
                / (target_clocks as u128)) as u32
        } else {
            TIMER_555_THRESHOLD_Q16
        };

        Ok(PaddleAdcState {
            cycle_count: self.cycle_count,
            triggered: self.triggered,
            trigger_pulse: pulse,
            capacitor_voltage_q16: v_q16,
        })
    }

    /// Returns true if the R-C ramp has reached the threshold comparator voltage.
    #[inline]
    pub const fn paddle_is_triggered(&self) -> bool {
        self.triggered
    }

    /// Returns the normalized capacitor voltage in Q16 format (where Vcc = 65536).
    pub fn paddle_capacitor_voltage_q16(&self) -> u32 {
        if self.triggered {
            TIMER_555_THRESHOLD_Q16
        } else {
            0
        }
    }

    /// Resets the paddle ADC to default idle state.
    pub fn paddle_reset(&mut self) {
        self.cycle_count = 0;
        self.triggered = false;
        self.last_triggered = false;
    }
}

// ===========================================================================
// Freestanding Warrant Verification Operators (Exported for Warrant claims)
// ===========================================================================

/// Evaluates a NAND operation with parameter bounds checks (for Warrant contract).
pub fn nand_eval_op(channel: usize, a: u8, b: u8) -> Result<u8, LogicError> {
    Ttl7400QuadNand::new_7400().nand_eval_gate(channel, a, b)
}

/// Evaluates a NOR operation with parameter bounds checks (for Warrant contract).
pub fn nor_eval_op(channel: usize, a: u8, b: u8) -> Result<u8, LogicError> {
    Ttl7402QuadNor::new_7402().nor_eval_gate(channel, a, b)
}

/// Evaluates a 4-to-1 MUX operation (for Warrant contract).
#[allow(clippy::too_many_arguments)]
pub fn mux_eval_op(
    section: usize,
    s0: u8,
    s1: u8,
    str_b: u8,
    c0: u8,
    c1: u8,
    c2: u8,
    c3: u8,
) -> Result<u8, LogicError> {
    Ttl74153DualMux4to1::new_74153().mux_eval_section(section, s0, s1, str_b, c0, c1, c2, c3)
}

/// Evaluates D-FF state transition (for Warrant contract).
pub fn dff_step_op(
    pre_b: u8,
    clr_b: u8,
    clk_edge: bool,
    d: u8,
    prev_q: u8,
) -> Result<u8, LogicError> {
    if pre_b > 1 || clr_b > 1 || d > 1 || prev_q > 1 {
        return Err(LogicError::InvalidLogicLevel {
            pin: "dff",
            value: 2,
        });
    }
    if pre_b == 0 {
        Ok(1)
    } else if clr_b == 0 {
        Ok(0)
    } else if clk_edge {
        Ok(d)
    } else {
        Ok(prev_q)
    }
}

/// Advances BCD value by 1 (wrapping at 9, for Warrant contract).
pub fn bcd_step_op(current_bcd: u8) -> Result<u8, LogicError> {
    if current_bcd > 9 {
        return Err(LogicError::InvalidParameter {
            name: "current_bcd",
            reason: "value exceeds 9",
        });
    }
    Ok(if current_bcd >= 9 { 0 } else { current_bcd + 1 })
}

/// Advances 4-bit binary value by 1 (wrapping at 15, for Warrant contract).
pub fn binary_step_op(current_val: u8) -> Result<u8, LogicError> {
    if current_val > 15 {
        return Err(LogicError::InvalidParameter {
            name: "current_val",
            reason: "value exceeds 15",
        });
    }
    Ok(if current_val >= 15 {
        0
    } else {
        current_val + 1
    })
}

/// Advances raster dot coordinate (wrapping at 454, for Warrant contract).
#[inline]
pub const fn sync_next_dot(current_dot: u16) -> u16 {
    if current_dot >= 454 {
        0
    } else {
        current_dot + 1
    }
}

/// Computes paddle charge duration in clocks (for Warrant contract).
pub fn paddle_calc_clocks(
    pot_norm: u16,
    clk_hz: u64,
    cap_pf: u64,
    max_r: u64,
) -> Result<u64, LogicError> {
    PaddlePotAdc::new_paddle_pot_adc(clk_hz, cap_pf, max_r)?.paddle_charge_duration_clocks(pot_norm)
}
