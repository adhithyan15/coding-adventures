//! # sim-pdp8-ge225-io-bridge
//!
//! Era 0 functional and gate-level peripheral I/O bridge connecting DEC PDP-8
//! (`pdp8-simulator` and `pdp8-gatelevel`) and GE-225 (`ge225-simulator` and
//! `ge225-gatelevel`) computer systems to the ASR-33 Teletype (`sim-teletype-asr33`)
//! with end-to-end playable bindings for `hamurabi` and `lunar-lander` (GAME03b).
//!
//! ## Historical Lineage
//!
//! - **1965 — DEC PDP-8 Teletype Interface (KL8-E / W076):**
//!   The PDP-8 interacted with its primary console — the Teletype Model 33 ASR — via
//!   dedicated programmed I/O transfer (IOT) instructions operating on two reserved
//!   device codes:
//!   - Device `0o03` (Keyboard / Paper-Tape Reader):
//!     - `KSF` (`0o6031` / IOP1): Skip next instruction if keyboard flag is set.
//!     - `KCC` (`0o6032` / IOP2): Clear keyboard flag and clear Accumulator (AC = 0).
//!     - `KRS` (`0o6034` / IOP4): Bitwise OR keyboard buffer into AC (`AC |= buffer`).
//!     - `KRB` (`0o6036` / IOP2 | IOP4): Clear AC and flag, then read keyboard buffer into AC.
//!   - Device `0o04` (Teleprinter / Paper-Tape Punch):
//!     - `TSF` (`0o6041` / IOP1): Skip next instruction if teleprinter flag is set.
//!     - `TCF` (`0o6042` / IOP2): Clear teleprinter flag.
//!     - `TPC` (`0o6044` / IOP4): Load `AC & 0o377` into teleprinter transmitter latch.
//!     - `TLS` (`0o6046` / IOP2 | IOP4): Clear flag and start teleprinter transmission.
//!   - Interrupt request line:
//!     Asserts CPU interrupt request whenever `kb_flag || tp_flag` is high.
//!
//! - **1961 — GE-225 Console Typewriter & Paper-Tape System:**
//!   General Electric's 20-bit mainframe connected an electromechanical console
//!   typewriter (Model 33 / Flexowriter) using 6-bit Binary Coded Decimal (BCD)
//!   character codes `0o00..=0o76` via instructions `TON` (Typewriter On), `TYP`
//!   (Type Character from N register), and `HPT` (Halt Paper Tape), alongside
//!   paper-tape instructions `RON`, `RPT`, `PON`, `WPT`.
//!
//! ## Verification Architecture
//!
//! Conforming to the repository's verification standards:
//! - Strictly zero ambient system authority: `"capabilities": []` declared in `required_capabilities.json`.
//! - Pure zero-heap-allocation execution on hot IOT / BCD dispatch paths (`VER07`).
//! - Exhaustive gate-level differential equivalence: `Pdp8TeletypeBridge` (functional)
//!   and `Pdp8TeletypeGateLevelBridge` (gate-level) produce 100% identical machine-state
//!   transitions, cycle counts, serial bit traces, and paper-roll output.
//! - CWE weakness matrix closure (`VER15`).

#![forbid(unsafe_code)]
#![allow(clippy::identity_op, clippy::needless_range_loop)]

#[allow(dead_code)]
pub mod capabilities;

use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate};
use pdp8_simulator::{
    encode_memory_reference, IotEvent, Opcode, Pdp8Error, Pdp8Simulator, StepTrace, HLT,
};
use sim_teletype_asr33::{Asr33Functional, Asr33GateLevel};

// ============================================================================
// PDP-8 Hardware Device & Instruction Constants
// ============================================================================

/// DEC PDP-8 IOT Device Address for the Keyboard / Paper-Tape Reader.
pub const PDP8_DEV_KBD: u8 = 0o03;

/// DEC PDP-8 IOT Device Address for the Teleprinter / Paper-Tape Punch.
pub const PDP8_DEV_TPR: u8 = 0o04;

/// I/O Pulse 1 (bit 0): Skip Pulse.
pub const PDP8_IOP1: u8 = 0o1;

/// I/O Pulse 2 (bit 1): Clear Pulse.
pub const PDP8_IOP2: u8 = 0o2;

/// I/O Pulse 4 (bit 2): Data Read / Transfer Pulse.
pub const PDP8_IOP4: u8 = 0o4;

/// `KSF` (`0o6031`): Keyboard Skip if Flag set.
pub const IOT_KSF: u16 = 0o6031;

/// `KCC` (`0o6032`): Keyboard Clear flag & Clear Accumulator.
pub const IOT_KCC: u16 = 0o6032;

/// `KRS` (`0o6034`): Keyboard Read Static (`AC |= buffer`).
pub const IOT_KRS: u16 = 0o6034;

/// `KRB` (`0o6036`): Keyboard Read Buffer (`KCC | KRS`).
pub const IOT_KRB: u16 = 0o6036;

/// `TSF` (`0o6041`): Teleprinter Skip if Flag set.
pub const IOT_TSF: u16 = 0o6041;

/// `TCF` (`0o6042`): Teleprinter Clear Flag.
pub const IOT_TCF: u16 = 0o6042;

/// `TPC` (`0o6044`): Teleprinter Print Character (`tx_load(AC & 0o377)`).
pub const IOT_TPC: u16 = 0o6044;

/// `TLS` (`0o6046`): Teleprinter Load and Start (`TCF | TPC`).
pub const IOT_TLS: u16 = 0o6046;

/// 12-bit PDP-8 Word / Accumulator Mask.
pub const PDP8_WORD_MASK: u16 = 0o7777;

/// 12-bit PDP-8 Program Counter Mask.
pub const PDP8_PC_MASK: u16 = 0o7777;

/// Maximum allowable execution step budget for `run_sim_until_halt` and `run_gate_until_halt` (CWE-400 guard).
pub const MAX_BRIDGE_RUN_STEPS: usize = 1_000_000;

/// 8-bit Character Mask for Teletype I/O.
pub const PDP8_CHAR_MASK: u16 = 0o377;

/// 6-bit GE-225 BCD Character Mask.
pub const GE225_BCD_MASK: u8 = 0o77;

// ============================================================================
// PDP-8 IOT Functional Dispatch (Zero-Alloc VER07)
// ============================================================================

/// Result of a PDP-8 IOT dispatch cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pdp8IotDispatchResult {
    /// Whether the device address was recognized by this peripheral controller.
    pub handled: bool,
    /// Whether the CPU Program Counter should skip the next instruction (`PC = (PC + 1) & 0o7777`).
    pub skip: bool,
    /// New Accumulator value after pulse execution.
    pub new_ac: u16,
    /// Whether the keyboard flag should be cleared (`kb_flag = false`).
    pub clear_kb: bool,
    /// Whether the teleprinter flag should be cleared (`tp_flag = false`).
    pub clear_tp: bool,
    /// Whether a character transmission was strobed into the transmitter.
    pub tx_strobe: bool,
    /// Byte byte value to transmit if `tx_strobe` is true.
    pub tx_byte: u8,
    /// Resulting logical level of the external interrupt request line (`kb_flag || tp_flag`).
    pub irq: bool,
}

/// Evaluates a PDP-8 IOT instruction using pure functional logic.
///
/// Guaranteed zero heap allocation.
#[inline]
pub fn pdp8_dispatch_iot_functional(
    device: u8,
    pulses: u8,
    ac: u16,
    kb_flag: bool,
    tp_flag: bool,
    kb_buffer: u8,
) -> Pdp8IotDispatchResult {
    let masked_ac = ac & PDP8_WORD_MASK;

    if device == PDP8_DEV_KBD {
        // Device 0o03 (Keyboard)
        let skip = (pulses & PDP8_IOP1 != 0) && kb_flag;
        let clear_kb = pulses & PDP8_IOP2 != 0;

        let mut next_ac = if clear_kb { 0 } else { masked_ac };
        if pulses & PDP8_IOP4 != 0 {
            next_ac = (next_ac | u16::from(kb_buffer)) & PDP8_WORD_MASK;
        }

        let resulting_kb_flag = kb_flag && !clear_kb;
        let irq = resulting_kb_flag || tp_flag;

        Pdp8IotDispatchResult {
            handled: true,
            skip,
            new_ac: next_ac,
            clear_kb,
            clear_tp: false,
            tx_strobe: false,
            tx_byte: 0,
            irq,
        }
    } else if device == PDP8_DEV_TPR {
        // Device 0o04 (Teleprinter)
        let skip = (pulses & PDP8_IOP1 != 0) && tp_flag;
        let clear_tp = pulses & PDP8_IOP2 != 0;
        let tx_strobe = pulses & PDP8_IOP4 != 0;
        let tx_byte = if tx_strobe {
            (masked_ac & PDP8_CHAR_MASK) as u8
        } else {
            0
        };

        let resulting_tp_flag = if tx_strobe || clear_tp {
            false
        } else {
            tp_flag
        };
        let irq = kb_flag || resulting_tp_flag;

        Pdp8IotDispatchResult {
            handled: true,
            skip,
            new_ac: masked_ac,
            clear_kb: false,
            clear_tp,
            tx_strobe,
            tx_byte,
            irq,
        }
    } else {
        // Unhandled device
        let irq = kb_flag || tp_flag;
        Pdp8IotDispatchResult {
            handled: false,
            skip: false,
            new_ac: masked_ac,
            clear_kb: false,
            clear_tp: false,
            tx_strobe: false,
            tx_byte: 0,
            irq,
        }
    }
}

// ============================================================================
// Gate-Level PDP-8 IOT Decoder (`GateLevelIotDecoder`)
// ============================================================================

/// Output of the gate-level IOT netlist execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateLevelIotOutput {
    /// 6-bit device comparator match for `0o03`.
    pub dev_03: u8,
    /// 6-bit device comparator match for `0o04`.
    pub dev_04: u8,
    /// Gated pulse for `KSF`.
    pub ksf: u8,
    /// Gated pulse for `KCC`.
    pub kcc: u8,
    /// Gated pulse for `KRS`.
    pub krs: u8,
    /// Gated pulse for `TSF`.
    pub tsf: u8,
    /// Gated pulse for `TCF`.
    pub tcf: u8,
    /// Gated pulse for `TPC`.
    pub tpc: u8,
    /// Combinational skip flag line.
    pub skip: u8,
    /// 12-bit multiplexed Accumulator output.
    pub new_ac: u16,
    /// Keyboard flag clear pulse.
    pub clear_kb: u8,
    /// Teleprinter flag clear pulse.
    pub clear_tp: u8,
    /// Transmitter load strobe.
    pub tx_strobe: u8,
    /// Character byte presented to transmitter.
    pub tx_byte: u8,
    /// Post-cycle interrupt request line.
    pub irq: u8,
}

/// Cycle-accurate digital logic netlist implementation of the DEC PDP-8
/// KL8-E IOT decoder and bus controller.
///
/// Constructed from primitive logic gates in `logic-gates`:
/// - 6-bit address comparison netlists for `0o03` and `0o04`.
/// - 3-input pulse gating logic for IOP1, IOP2, and IOP4.
/// - 2-input skip flag OR netlist.
/// - 12-bit bus multiplexer using `mux2` and `or_gate` for AC updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GateLevelIotDecoder;

impl GateLevelIotDecoder {
    /// Executes the combinational logic netlist for a single IOT cycle.
    pub fn decode(
        device: u8,
        pulses: u8,
        ac: u16,
        kb_flag: bool,
        tp_flag: bool,
        kb_buffer: u8,
    ) -> GateLevelIotOutput {
        // 1. Device Address Decoder (6-bit Comparator Netlist)
        let d0 = (device >> 0) & 1;
        let d1 = (device >> 1) & 1;
        let d2 = (device >> 2) & 1;
        let d3 = (device >> 3) & 1;
        let d4 = (device >> 4) & 1;
        let d5 = (device >> 5) & 1;

        let not_d0 = not_gate(d0);
        let not_d1 = not_gate(d1);
        let not_d2 = not_gate(d2);
        let not_d3 = not_gate(d3);
        let not_d4 = not_gate(d4);
        let not_d5 = not_gate(d5);
        let dev_6bit_valid = and_gate(not_gate((device >> 6) & 1), not_gate((device >> 7) & 1));

        // Device 0o03: binary 000011 (d5=0, d4=0, d3=0, d2=0, d1=1, d0=1)
        let dev_03 = and_gate(
            dev_6bit_valid,
            and_gate(
                and_gate(and_gate(not_d5, not_d4), and_gate(not_d3, not_d2)),
                and_gate(d1, d0),
            ),
        );

        // Device 0o04: binary 000100 (d5=0, d4=0, d3=0, d2=1, d1=0, d0=0)
        let dev_04 = and_gate(
            dev_6bit_valid,
            and_gate(
                and_gate(and_gate(not_d5, not_d4), and_gate(not_d3, d2)),
                and_gate(not_d1, not_d0),
            ),
        );

        // 2. Pulse Demultiplexing & Gating
        let iop1 = (pulses >> 0) & 1;
        let iop2 = (pulses >> 1) & 1;
        let iop4 = (pulses >> 2) & 1;

        let ksf = and_gate(dev_03, iop1);
        let kcc = and_gate(dev_03, iop2);
        let krs = and_gate(dev_03, iop4);

        let tsf = and_gate(dev_04, iop1);
        let tcf = and_gate(dev_04, iop2);
        let tpc = and_gate(dev_04, iop4);

        // 3. Skip-Flag Combinational Logic
        let skip_kb = and_gate(ksf, kb_flag as u8);
        let skip_tp = and_gate(tsf, tp_flag as u8);
        let skip = or_gate(skip_kb, skip_tp);

        // 4. 12-Bit Accumulator Bus Multiplexer Netlist
        let mut new_ac = 0u16;
        for i in 0..12 {
            let ac_bit = ((ac >> i) & 1) as u8;
            let kb_bit = if i < 8 { (kb_buffer >> i) & 1 } else { 0 };

            // Stage A: If KCC is asserted, AC is zeroed.
            let ac_after_kcc = mux2(ac_bit, 0, kcc);

            // Stage B: If KRS is asserted, keyboard buffer is ORed onto AC bus.
            let ac_or_kb = or_gate(ac_after_kcc, kb_bit);
            let final_bit = mux2(ac_after_kcc, ac_or_kb, krs);

            new_ac |= (final_bit as u16) << i;
        }

        // 5. Clear Pulses & Transmission Strobe
        let clear_kb = kcc;
        let clear_tp = tcf;
        let tx_strobe = tpc;
        let mut tx_byte = 0u8;
        for b in 0..8 {
            let ac_b = ((ac >> b) & 1) as u8;
            let bit_val = and_gate(ac_b, tx_strobe);
            tx_byte |= bit_val << b;
        }

        // 6. Post-Cycle Interrupt Request Line
        let new_kb_flag = and_gate(kb_flag as u8, not_gate(clear_kb));
        let tp_busy_or_cleared = or_gate(clear_tp, tx_strobe);
        let new_tp_flag = and_gate(tp_flag as u8, not_gate(tp_busy_or_cleared));
        let irq = or_gate(new_kb_flag, new_tp_flag);

        GateLevelIotOutput {
            dev_03,
            dev_04,
            ksf,
            kcc,
            krs,
            tsf,
            tcf,
            tpc,
            skip,
            new_ac,
            clear_kb,
            clear_tp,
            tx_strobe,
            tx_byte,
            irq,
        }
    }
}

// ============================================================================
// Pdp8TeletypeBridge — Functional I/O Bridge
// ============================================================================

/// Functional peripheral bridge connecting a PDP-8 CPU (`pdp8_simulator::Pdp8Simulator`
/// or `pdp8_gatelevel::Pdp8GateLevel`) to an `Asr33Functional` Teletype simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pdp8TeletypeBridge {
    pub asr33: Asr33Functional,
}

impl Default for Pdp8TeletypeBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Pdp8TeletypeBridge {
    /// Constructs a fresh bridge with an idle ASR-33 Teletype.
    pub fn new() -> Self {
        Self {
            asr33: Asr33Functional::new(),
        }
    }

    /// Constructs a bridge wrapping an existing `Asr33Functional`.
    pub fn with_asr33(asr33: Asr33Functional) -> Self {
        Self { asr33 }
    }

    /// Read-only reference to the underlying Teletype simulator.
    #[inline]
    pub fn asr33(&self) -> &Asr33Functional {
        &self.asr33
    }

    /// Mutable reference to the underlying Teletype simulator.
    #[inline]
    pub fn asr33_mut(&mut self) -> &mut Asr33Functional {
        &mut self.asr33
    }

    /// Pushes a character byte from the keyboard into the KRB buffer.
    #[inline]
    pub fn kbd_push_byte(&mut self, byte: u8) {
        self.asr33.kbd_push_byte(byte);
    }

    /// Queries the teleprinter flag.
    #[inline]
    pub fn tp_flag(&self) -> bool {
        self.asr33.tp_flag()
    }

    /// Queries the keyboard flag.
    #[inline]
    pub fn kb_flag(&self) -> bool {
        self.asr33.kb_flag()
    }

    /// Processes an IOT event and updates internal Teletype state.
    pub fn handle_iot(&mut self, iot: IotEvent, ac: u16) -> Pdp8IotDispatchResult {
        let res = pdp8_dispatch_iot_functional(
            iot.device,
            iot.pulses,
            ac,
            self.asr33.kb_flag(),
            self.asr33.tp_flag(),
            self.asr33.state().kb_buffer,
        );

        if res.clear_kb {
            self.asr33.kbd_read_clear();
        }
        if res.clear_tp && !res.tx_strobe {
            self.asr33.tp_clear_flag();
        }
        if res.tx_strobe {
            self.asr33.tx_load(res.tx_byte);
        }

        res
    }

    /// Advances the CPU simulator by one instruction step and updates I/O state.
    pub fn step_sim(&mut self, sim: &mut Pdp8Simulator) -> Result<StepTrace, Pdp8Error> {
        let trace = sim.step()?;

        if let Some(iot) = trace.iot {
            let res = self.handle_iot(iot, sim.state().ac);
            if res.handled {
                sim.set_accumulator(res.new_ac)?;
                if res.skip {
                    let next_pc = (sim.state().pc.wrapping_add(1)) & PDP8_PC_MASK;
                    sim.set_program_counter(next_pc)?;
                }
            }
        }

        if self.asr33.state().tx_bits_remaining > 0 {
            self.asr33.step_serial_bit();
        }

        sim.set_interrupt_request(self.asr33.kb_flag() || self.asr33.tp_flag());
        Ok(trace)
    }

    /// Advances the gate-level CPU by one instruction step and updates I/O state.
    pub fn step_gate(
        &mut self,
        cpu: &mut pdp8_gatelevel::Pdp8GateLevel,
    ) -> Result<StepTrace, Pdp8Error> {
        let trace = cpu.step()?;

        if let Some(iot) = trace.iot {
            let res = self.handle_iot(iot, cpu.state().ac);
            if res.handled {
                cpu.set_accumulator(res.new_ac)?;
                if res.skip {
                    let next_pc = (cpu.state().pc.wrapping_add(1)) & PDP8_PC_MASK;
                    cpu.set_program_counter(next_pc)?;
                }
            }
        }

        if self.asr33.state().tx_bits_remaining > 0 {
            self.asr33.step_serial_bit();
        }

        cpu.set_interrupt_request(self.asr33.kb_flag() || self.asr33.tp_flag());
        Ok(trace)
    }

    /// Runs the CPU simulator until HLT or maximum steps.
    pub fn run_sim_until_halt(
        &mut self,
        sim: &mut Pdp8Simulator,
        max_steps: usize,
    ) -> Result<usize, Pdp8Error> {
        if max_steps > MAX_BRIDGE_RUN_STEPS {
            return Err(Pdp8Error::MaxStepsExceeded { max_steps });
        }
        for step in 1..=max_steps {
            self.step_sim(sim)?;
            if sim.state().halted {
                return Ok(step);
            }
        }
        Err(Pdp8Error::MaxStepsExceeded { max_steps })
    }

    /// Runs the gate-level CPU until HLT or maximum steps.
    pub fn run_gate_until_halt(
        &mut self,
        cpu: &mut pdp8_gatelevel::Pdp8GateLevel,
        max_steps: usize,
    ) -> Result<usize, Pdp8Error> {
        if max_steps > MAX_BRIDGE_RUN_STEPS {
            return Err(Pdp8Error::MaxStepsExceeded { max_steps });
        }
        for step in 1..=max_steps {
            self.step_gate(cpu)?;
            if cpu.state().halted {
                return Ok(step);
            }
        }
        Err(Pdp8Error::MaxStepsExceeded { max_steps })
    }

    /// Returns accumulated paper roll text.
    pub fn paper_roll_text(&self) -> String {
        self.asr33.paper_roll_text()
    }

    /// Renders paper roll to ASCII string via `paint-vm-ascii`.
    pub fn render_paper_roll_ascii(&self) -> String {
        self.asr33.render_paper_roll_ascii()
    }
}

// ============================================================================
// Pdp8TeletypeGateLevelBridge — Gate-Level I/O Bridge
// ============================================================================

/// Cycle-accurate gate-level peripheral bridge connecting a PDP-8 CPU
/// to an `Asr33GateLevel` Teletype simulator via `GateLevelIotDecoder`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pdp8TeletypeGateLevelBridge {
    pub asr33: Asr33GateLevel,
}

impl Default for Pdp8TeletypeGateLevelBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Pdp8TeletypeGateLevelBridge {
    /// Constructs a fresh gate-level bridge.
    pub fn new() -> Self {
        Self {
            asr33: Asr33GateLevel::new(),
        }
    }

    /// Constructs a bridge wrapping an existing `Asr33GateLevel`.
    pub fn with_asr33(asr33: Asr33GateLevel) -> Self {
        Self { asr33 }
    }

    /// Read-only reference to the underlying gate-level Teletype.
    #[inline]
    pub fn asr33(&self) -> &Asr33GateLevel {
        &self.asr33
    }

    /// Mutable reference to the underlying gate-level Teletype.
    #[inline]
    pub fn asr33_mut(&mut self) -> &mut Asr33GateLevel {
        &mut self.asr33
    }

    /// Pushes a character byte from the keyboard into the KRB buffer.
    #[inline]
    pub fn kbd_push_byte(&mut self, byte: u8) {
        self.asr33.kbd_push_byte(byte);
    }

    /// Queries the teleprinter flag.
    #[inline]
    pub fn tp_flag(&self) -> bool {
        self.asr33.tp_flag()
    }

    /// Queries the keyboard flag.
    #[inline]
    pub fn kb_flag(&self) -> bool {
        self.asr33.kb_flag()
    }

    /// Processes an IOT event using the gate-level decoder.
    pub fn handle_iot(&mut self, iot: IotEvent, ac: u16) -> GateLevelIotOutput {
        let out = GateLevelIotDecoder::decode(
            iot.device,
            iot.pulses,
            ac,
            self.asr33.kb_flag(),
            self.asr33.tp_flag(),
            self.asr33.krb_byte(),
        );

        if out.clear_kb == 1 {
            self.asr33.kbd_read_clear();
        }
        if out.clear_tp == 1 && out.tx_strobe == 0 {
            self.asr33.tp_clear_flag();
        }
        if out.tx_strobe == 1 {
            self.asr33.tx_load(out.tx_byte);
        }

        out
    }

    /// Advances the CPU simulator by one instruction step and updates I/O state.
    pub fn step_sim(&mut self, sim: &mut Pdp8Simulator) -> Result<StepTrace, Pdp8Error> {
        let trace = sim.step()?;

        if let Some(iot) = trace.iot {
            let out = self.handle_iot(iot, sim.state().ac);
            if out.dev_03 == 1 || out.dev_04 == 1 {
                sim.set_accumulator(out.new_ac)?;
                if out.skip == 1 {
                    let next_pc = (sim.state().pc.wrapping_add(1)) & PDP8_PC_MASK;
                    sim.set_program_counter(next_pc)?;
                }
            }
        }

        if self.asr33.tx_bits_remaining > 0 {
            self.asr33.step_serial_bit();
        }

        sim.set_interrupt_request(self.asr33.kb_flag() || self.asr33.tp_flag());
        Ok(trace)
    }

    /// Advances the gate-level CPU by one instruction step and updates I/O state.
    pub fn step_gate(
        &mut self,
        cpu: &mut pdp8_gatelevel::Pdp8GateLevel,
    ) -> Result<StepTrace, Pdp8Error> {
        let trace = cpu.step()?;

        if let Some(iot) = trace.iot {
            let out = self.handle_iot(iot, cpu.state().ac);
            if out.dev_03 == 1 || out.dev_04 == 1 {
                cpu.set_accumulator(out.new_ac)?;
                if out.skip == 1 {
                    let next_pc = (cpu.state().pc.wrapping_add(1)) & PDP8_PC_MASK;
                    cpu.set_program_counter(next_pc)?;
                }
            }
        }

        if self.asr33.tx_bits_remaining > 0 {
            self.asr33.step_serial_bit();
        }

        cpu.set_interrupt_request(self.asr33.kb_flag() || self.asr33.tp_flag());
        Ok(trace)
    }

    /// Runs the CPU simulator until HLT or maximum steps.
    pub fn run_sim_until_halt(
        &mut self,
        sim: &mut Pdp8Simulator,
        max_steps: usize,
    ) -> Result<usize, Pdp8Error> {
        if max_steps > MAX_BRIDGE_RUN_STEPS {
            return Err(Pdp8Error::MaxStepsExceeded { max_steps });
        }
        for step in 1..=max_steps {
            self.step_sim(sim)?;
            if sim.state().halted {
                return Ok(step);
            }
        }
        Err(Pdp8Error::MaxStepsExceeded { max_steps })
    }

    /// Runs the gate-level CPU until HLT or maximum steps.
    pub fn run_gate_until_halt(
        &mut self,
        cpu: &mut pdp8_gatelevel::Pdp8GateLevel,
        max_steps: usize,
    ) -> Result<usize, Pdp8Error> {
        if max_steps > MAX_BRIDGE_RUN_STEPS {
            return Err(Pdp8Error::MaxStepsExceeded { max_steps });
        }
        for step in 1..=max_steps {
            self.step_gate(cpu)?;
            if cpu.state().halted {
                return Ok(step);
            }
        }
        Err(Pdp8Error::MaxStepsExceeded { max_steps })
    }
}

// ============================================================================
// GE-225 BCD Code Translation & Console Typewriter Bridge
// ============================================================================

/// Converts a 6-bit GE-225 BCD character code (`0o00..=0o76`) to ASCII.
///
/// Implements the exact character table used by `ge225-simulator` and `ge225-gatelevel`:
/// - `0o00..=0o11`: `'0'..='9'`
/// - `0o13`: `'/'`
/// - `0o21..=0o31`: `'A'..='I'`
/// - `0o33`: `'-'`
/// - `0o37`: Carriage Return (`\r`)
/// - `0o40`: `'.'`
/// - `0o41..=0o51`: `'J'..='R'`
/// - `0o53`: `'$'`
/// - `0o60`: `' '` (Space)
/// - `0o62..=0o71`: `'S'..='Z'`
/// - `0o76`: Tab (`\t`)
#[inline]
pub const fn ge225_bcd_to_ascii(code: u8) -> Option<u8> {
    match code & GE225_BCD_MASK {
        0o00 => Some(b'0'),
        0o01 => Some(b'1'),
        0o02 => Some(b'2'),
        0o03 => Some(b'3'),
        0o04 => Some(b'4'),
        0o05 => Some(b'5'),
        0o06 => Some(b'6'),
        0o07 => Some(b'7'),
        0o10 => Some(b'8'),
        0o11 => Some(b'9'),
        0o13 => Some(b'/'),
        0o21 => Some(b'A'),
        0o22 => Some(b'B'),
        0o23 => Some(b'C'),
        0o24 => Some(b'D'),
        0o25 => Some(b'E'),
        0o26 => Some(b'F'),
        0o27 => Some(b'G'),
        0o30 => Some(b'H'),
        0o31 => Some(b'I'),
        0o33 => Some(b'-'),
        0o37 => Some(b'\r'),
        0o40 => Some(b'.'),
        0o41 => Some(b'J'),
        0o42 => Some(b'K'),
        0o43 => Some(b'L'),
        0o44 => Some(b'M'),
        0o45 => Some(b'N'),
        0o46 => Some(b'O'),
        0o47 => Some(b'P'),
        0o50 => Some(b'Q'),
        0o51 => Some(b'R'),
        0o53 => Some(b'$'),
        0o60 => Some(b' '),
        0o62 => Some(b'S'),
        0o63 => Some(b'T'),
        0o64 => Some(b'U'),
        0o65 => Some(b'V'),
        0o66 => Some(b'W'),
        0o67 => Some(b'X'),
        0o70 => Some(b'Y'),
        0o71 => Some(b'Z'),
        0o76 => Some(b'\t'),
        _ => None,
    }
}

/// Converts an ASCII character to 6-bit GE-225 BCD code.
///
/// Folds lowercase `'a'..='z'` to uppercase `'A'..='Z'`, and maps newline `\n` to CR (`0o37`).
#[inline]
pub const fn ascii_to_ge225_bcd(ascii: u8) -> Option<u8> {
    match ascii {
        b'0'..=b'9' => Some(ascii - b'0'),
        b'/' => Some(0o13),
        b'A'..=b'I' => Some(0o21 + (ascii - b'A')),
        b'a'..=b'i' => Some(0o21 + (ascii - b'a')),
        b'-' => Some(0o33),
        b'\r' | b'\n' => Some(0o37),
        b'.' => Some(0o40),
        b'J'..=b'R' => Some(0o41 + (ascii - b'J')),
        b'j'..=b'r' => Some(0o41 + (ascii - b'j')),
        b'$' => Some(0o53),
        b' ' => Some(0o60),
        b'S'..=b'Z' => Some(0o62 + (ascii - b'S')),
        b's'..=b'z' => Some(0o62 + (ascii - b's')),
        b'\t' => Some(0o76),
        _ => None,
    }
}

/// Converts a slice of GE-225 6-bit BCD codes into a UTF-8 string.
pub fn ge225_bcd_to_str(codes: &[u8]) -> String {
    let mut out = String::with_capacity(codes.len());
    for &code in codes {
        if let Some(ch) = ge225_bcd_to_ascii(code) {
            out.push(ch as char);
        }
    }
    out
}

/// Converts an ASCII string into GE-225 6-bit BCD codes.
pub fn str_to_ge225_bcd(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    for b in text.bytes() {
        if let Some(code) = ascii_to_ge225_bcd(b) {
            out.push(code);
        }
    }
    out
}

/// Peripheral bridge connecting the GE-225 Console Typewriter and Paper Tape
/// subsystems to the ASR-33 Teletype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ge225TeletypeBridge;

impl Ge225TeletypeBridge {
    /// Transfers output from `ge225_simulator::Simulator`'s console typewriter onto `Asr33Functional`.
    pub fn sync_sim_typewriter(sim: &ge225_simulator::Simulator, asr: &mut Asr33Functional) {
        let output = sim.get_typewriter_output();
        for b in output.bytes() {
            asr.print_char(b);
        }
    }

    /// Transfers output from `ge225_gatelevel::Ge225GateLevel`'s console typewriter onto `Asr33Functional`.
    pub fn sync_gate_typewriter(gate: &ge225_gatelevel::Ge225GateLevel, asr: &mut Asr33Functional) {
        let output = gate.get_typewriter_output();
        for b in output.bytes() {
            asr.print_char(b);
        }
    }

    /// Encodes ASCII text into BCD codes and queues them into `ge225_simulator::Simulator`'s typewriter input.
    pub fn feed_sim_typewriter(
        sim: &mut ge225_simulator::Simulator,
        text: &str,
    ) -> Result<(), String> {
        let bcd_codes = str_to_ge225_bcd(text);
        let i32_codes: Vec<i32> = bcd_codes.iter().map(|&c| c as i32).collect();
        sim.queue_typewriter_input(&i32_codes)
    }

    /// Encodes ASCII text into BCD codes and queues them into `ge225_gatelevel::Ge225GateLevel`'s typewriter input.
    pub fn feed_gate_typewriter(
        gate: &mut ge225_gatelevel::Ge225GateLevel,
        text: &str,
    ) -> Result<(), ge225_gatelevel::Ge225GateError> {
        let bcd_codes = str_to_ge225_bcd(text);
        let i32_codes: Vec<i32> = bcd_codes.iter().map(|&c| c as i32).collect();
        gate.queue_typewriter_input(&i32_codes)
    }
}

// ============================================================================
// End-to-End Machine Code Assembly Helpers
// ============================================================================

/// Assembles a canonical PDP-8 machine-code program that reads one keyboard
/// digit into `input_char_dest` and prints a null-terminated ASCII string at
/// `string_origin` to the teleprinter using polled I/O.
///
/// Origin of program is `0o0200`:
/// ```text
/// 0o0200: KSF          ; Skip if keyboard character ready
/// 0o0201: JMP 0o0200   ; Poll wait loop
/// 0o0202: KRB          ; Clear AC/flag and read character
/// 0o0203: DCA 0o0020   ; Store input digit at destination
/// 0o0204: CLA          ; Clear AC
/// ; Print loop:
/// 0o0205: TAD I 0o0010 ; Auto-index fetch next character (0o0010 auto-increments)
/// 0o0206: SNA          ; Skip if AC != 0
/// 0o0207: HLT          ; Halt if null terminator
/// 0o0210: TLS          ; Clear flag & transmit character
/// 0o0211: TSF          ; Skip if transmitter ready
/// 0o0212: JMP 0o0211   ; Poll wait loop
/// 0o0213: CLA          ; Clear AC
/// 0o0214: JMP 0o0205   ; Next character
/// ```
pub fn assemble_pdp8_teletype_printer_program(input_char_dest: u16) -> Vec<u16> {
    vec![
        IOT_KSF,                                                  // 0o0200: KSF
        encode_memory_reference(Opcode::Jmp, false, true, 0o000), // 0o0201: JMP 0o0200
        IOT_KRB,                                                  // 0o0202: KRB
        encode_memory_reference(Opcode::Dca, false, false, (input_char_dest & 0o177) as u8), // 0o0203: DCA dest
        0o7200,                                                   // 0o0204: CLA
        encode_memory_reference(Opcode::Tad, true, false, 0o010), // 0o0205: TAD I 0o0010
        0o7450,                                                   // 0o0206: SNA
        HLT,                                                      // 0o0207: HLT
        IOT_TLS,                                                  // 0o0210: TLS
        IOT_TSF,                                                  // 0o0211: TSF
        encode_memory_reference(Opcode::Jmp, false, true, 0o011), // 0o0212: JMP 0o0211
        0o7200,                                                   // 0o0213: CLA
        encode_memory_reference(Opcode::Jmp, false, true, 0o005), // 0o0214: JMP 0o0205
    ]
}

/// Encodes an ASCII string as null-terminated 12-bit PDP-8 words.
pub fn encode_ascii_for_pdp8(text: &str) -> Vec<u16> {
    let mut words: Vec<u16> = text
        .bytes()
        .map(|b| u16::from(b) & PDP8_CHAR_MASK)
        .collect();
    words.push(0); // Null terminator
    words
}

/// Maximum number of BCD words permitted in `assemble_ge225_typewriter_program`.
pub const MAX_GE225_TYPEWRITER_WORDS: usize = 4096;

/// Assembles a GE-225 program that turns on the typewriter (`TON`) and types
/// a sequence of BCD words stored at `bcd_data_addr`.
///
/// Returns `None` if `bcd_len` exceeds [`MAX_GE225_TYPEWRITER_WORDS`] or any
/// computed memory address is out of range for GE-225 instruction encoding.
pub fn assemble_ge225_typewriter_program(bcd_data_addr: i32, bcd_len: usize) -> Option<Vec<i32>> {
    if bcd_len > MAX_GE225_TYPEWRITER_WORDS {
        return None;
    }

    let mut words = Vec::with_capacity(bcd_len * 3 + 2);
    words.push(ge225_simulator::assemble_fixed("TON").ok()?);

    for i in 0..bcd_len {
        let offset = i32::try_from(i).ok()?;
        let addr = bcd_data_addr.checked_add(offset)?;
        // OP_LDA = 0o00
        words.push(ge225_simulator::encode_instruction(0o00, 0, addr).ok()?);
        words.push(ge225_simulator::assemble_shift("SAN", 6).ok()?);
        words.push(ge225_simulator::assemble_fixed("TYP").ok()?);
    }

    words.push(ge225_simulator::assemble_fixed("NOP").ok()?);
    Some(words)
}

// ============================================================================
// Public Standalone Constructors
// ============================================================================

/// Constructs a new functional PDP-8 Teletype bridge.
pub fn new_pdp8_teletype_bridge() -> Pdp8TeletypeBridge {
    Pdp8TeletypeBridge::new()
}

/// Constructs a new gate-level PDP-8 Teletype bridge.
pub fn new_pdp8_teletype_gate_bridge() -> Pdp8TeletypeGateLevelBridge {
    Pdp8TeletypeGateLevelBridge::new()
}
