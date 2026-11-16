//! # sim-teletype-asr33
//!
//! Cycle-accurate Teletype Model 33 ASR (Automatic Send-Receive) simulator
//! featuring both a pure zero-heap-allocation functional model (`Asr33Functional`
//! / `Asr33FrameState`) and a gate-level digital logic model (`Asr33GateLevel`)
//! constructed from primitive logic gates, with 72-column platen paper-roll
//! rendering via `paint-instructions` and `paint-vm-ascii` (GAME03a).
//!
//! ## Historical Lineage
//!
//! - **1963 — Teletype Model 33:** Introduced by the Teletype Corporation (a
//!   subsidiary of Western Electric / AT&T), the ASR-33 became the iconic user
//!   interface of the minicomputer revolution. Operating at 110 baud (10 characters
//!   per second) using an 11-unit asynchronous serial code, it connected to early
//!   machines like the DEC PDP-8, HP 2100, and GE-225.
//! - **The 72-Column Platen & Upper-Case Drum:** The Model 33 used a cylindrical
//!   typewheel containing a 64-character subset of ASCII (0x20..=0x5F). Lowercase
//!   characters `'a'..='z'` were physically folded to uppercase `'A'..='Z'` upon
//!   printing. The carriage advanced across 72 columns before auto-wrapping.
//!
//! ## Architectural Guarantees
//!
//! In accordance with the repository's verification architecture (`VER00`,
//! `VER04m`, `VER07`, `VER08a`, `VER08b`, `VER15`, `VER27`):
//! - Strictly zero ambient system authority: `"capabilities": []` declared in
//!   `required_capabilities.json`.
//! - Pure zero-allocation execution: `Asr33FrameState` is a `Copy` struct, and all
//!   core device operations execute with 0 heap allocation bytes (`VER07`).
//! - Exhaustive gate-level differential equivalence: `Asr33Functional` and
//!   `Asr33GateLevel` produce 100% identical serial wire waveforms, flags, and
//!   reconstructed characters across all 256 byte values and 11 bit times.
//! - Declarative 2D paper-roll telemetry rendered through `paint-vm-ascii` (`P2D02`).
//! - Full 26-CWE weakness closure (`VER15`).

#![forbid(unsafe_code)]
#![allow(clippy::identity_op, clippy::needless_range_loop)]

#[allow(dead_code)]
pub mod capabilities;

use logic_gates::gates::xor_gate;
use logic_gates::sequential::{d_flip_flop, shift_register, FlipFlopState};
use paint_instructions::{
    PaintBase, PaintInstruction, PaintRect, PaintScene, PaintText, TextAlign,
};
use paint_vm_ascii::{render, AsciiOptions};

// ============================================================================
// Hardware Constants
// ============================================================================

/// Standard serial transmission rate for ASR-33 (110 baud = 10 characters/sec).
pub const BAUD_RATE: u32 = 110;

/// Number of serial bit times in one 110-baud asynchronous character frame:
/// 1 Start bit (0) + 7 ASCII bits + 1 Parity bit + 2 Stop bits (1, 1) = 11 bits.
pub const BITS_PER_FRAME: usize = 11;

/// Standard line width of the ASR-33 platen in character columns.
pub const PLATEN_WIDTH: usize = 72;

/// Maximum number of lines retained in the fixed paper roll ring buffer.
pub const MAX_PAPER_LINES: usize = 64;

/// ASCII Control Code: Bell (0x07). Rings the physical electromechanical bell.
pub const ASCII_BEL: u8 = 0x07;

/// ASCII Control Code: Carriage Return (0x0D). Returns print carriage to column 0.
pub const ASCII_CR: u8 = 0x0D;

/// ASCII Control Code: Line Feed (0x0A). Advances the paper roll platen by 1 line.
pub const ASCII_LF: u8 = 0x0A;

/// Serial line idle state: Mark (high / 1).
pub const SERIAL_MARK: u8 = 1;

/// Serial line active start state: Space (low / 0).
pub const SERIAL_SPACE: u8 = 0;

// ============================================================================
// Parity Generator Tree (Gate-Level & Combinational)
// ============================================================================

/// 7-XOR even-parity generator tree.
///
/// Computes the even-parity bit for 7-bit ASCII data `b0..b6` such that the total
/// number of 1-bits among `(b0..b6, parity)` is even.
///
/// Built as a balanced 3-stage binary tree of 7 XOR gates from
/// `logic_gates::gates::xor_gate`, with the 8th terminal grounded to 0.
pub fn parity_generator_7xor(b0: u8, b1: u8, b2: u8, b3: u8, b4: u8, b5: u8, b6: u8) -> u8 {
    // Stage 1: 4 XOR gates (pairwise over 8 inputs with input 7 = 0)
    let g1 = xor_gate(b0, b1);
    let g2 = xor_gate(b2, b3);
    let g3 = xor_gate(b4, b5);
    let g4 = xor_gate(b6, 0);

    // Stage 2: 2 XOR gates
    let g5 = xor_gate(g1, g2);
    let g6 = xor_gate(g3, g4);

    // Stage 3: 1 XOR gate
    xor_gate(g5, g6)
}

/// Computes the even parity bit (0 or 1) for a given byte's low 7 bits.
#[inline]
pub fn compute_even_parity(b: u8) -> u8 {
    let ascii = b & 0x7F;
    let b0 = (ascii >> 0) & 1;
    let b1 = (ascii >> 1) & 1;
    let b2 = (ascii >> 2) & 1;
    let b3 = (ascii >> 3) & 1;
    let b4 = (ascii >> 4) & 1;
    let b5 = (ascii >> 5) & 1;
    let b6 = (ascii >> 6) & 1;
    parity_generator_7xor(b0, b1, b2, b3, b4, b5, b6)
}

// ============================================================================
// Asr33FrameState — Pure Zero-Alloc Functional State (VER07)
// ============================================================================

/// Cycle-accurate register and serial line state for an ASR-33 Teletype.
///
/// Guaranteed to be a `Copy` struct with zero dynamic memory allocation on any
/// state transition method (`step_serial_bit`, `tx_load`, `rx_feed_bit`,
/// `kbd_push_byte`, `kbd_read_clear`, `tp_clear_flag`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Asr33FrameState {
    /// 11-bit serial transmit shift register (shifts LSB first).
    pub tx_shift_reg: u16,
    /// Number of serial bit clocks remaining in current transmission (0..=11).
    pub tx_bits_remaining: u8,
    /// Current logical level on the serial TX wire (1 = Mark, 0 = Space).
    pub tx_line: u8,
    /// 11-bit serial receive shift register.
    pub rx_shift_reg: u16,
    /// Number of serial bits received in current incoming frame (0..=11).
    pub rx_bits_received: u8,
    /// Current logical level on the serial RX wire (1 = Mark, 0 = Space).
    pub rx_line: u8,
    /// Whether the receiver has detected a valid Start bit and is sampling.
    pub rx_active: bool,
    /// 8-bit Keyboard Reader Buffer (KRB / input latch).
    pub kb_buffer: u8,
    /// Keyboard flag (true when character is ready to read in KRB).
    pub kb_flag: bool,
    /// 8-bit Teleprinter Output Buffer (TTO / transmitter latch).
    pub tp_buffer: u8,
    /// Teleprinter flag (true when transmitter is idle and ready for next character).
    pub tp_flag: bool,
    /// Platen carriage column index (`0..=71`).
    pub platen_col: u8,
    /// Platen vertical line / row counter.
    pub platen_row: u32,
    /// Electromechanical bell ring counter (ASCII 0x07 `BEL`).
    pub bel_count: u32,
    /// Last printed uppercase character, if any.
    pub last_printed_char: Option<u8>,
}

impl Default for Asr33FrameState {
    fn default() -> Self {
        Self::new()
    }
}

impl Asr33FrameState {
    /// Creates a fresh ASR-33 state in idle condition.
    pub const fn new() -> Self {
        Self {
            tx_shift_reg: 0x07FF, // 11 bits of Mark (1)
            tx_bits_remaining: 0,
            tx_line: SERIAL_MARK,
            rx_shift_reg: 0,
            rx_bits_received: 0,
            rx_line: SERIAL_MARK,
            rx_active: false,
            kb_buffer: 0,
            kb_flag: false,
            tp_buffer: 0,
            tp_flag: true, // Transmitter is ready initially
            platen_col: 0,
            platen_row: 0,
            bel_count: 0,
            last_printed_char: None,
        }
    }

    /// Loads a byte into the teleprinter transmitter buffer, clearing `tp_flag`
    /// and assembling the 11-bit asynchronous serial frame.
    pub fn tx_load(&mut self, byte: u8) {
        self.tp_buffer = byte;
        self.tp_flag = false; // Teleprinter is busy transmitting

        let ascii = byte & 0x7F;
        let b0 = (ascii >> 0) & 1;
        let b1 = (ascii >> 1) & 1;
        let b2 = (ascii >> 2) & 1;
        let b3 = (ascii >> 3) & 1;
        let b4 = (ascii >> 4) & 1;
        let b5 = (ascii >> 5) & 1;
        let b6 = (ascii >> 6) & 1;
        let p = parity_generator_7xor(b0, b1, b2, b3, b4, b5, b6);

        // 11-bit serial frame:
        // bit 0: Start (0)
        // bits 1..=7: data bits b0..b6 (LSB first)
        // bit 8: parity bit (even parity)
        // bit 9: Stop 1 (1)
        // bit 10: Stop 2 (1)
        let frame: u16 = (0u16)
            | ((b0 as u16) << 1)
            | ((b1 as u16) << 2)
            | ((b2 as u16) << 3)
            | ((b3 as u16) << 4)
            | ((b4 as u16) << 5)
            | ((b5 as u16) << 6)
            | ((b6 as u16) << 7)
            | ((p as u16) << 8)
            | (1u16 << 9)
            | (1u16 << 10);

        self.tx_shift_reg = frame;
        self.tx_bits_remaining = 11;
        self.tx_line = SERIAL_SPACE; // Start bit is active immediately
    }

    /// Advances the serial transmitter by one bit clock.
    ///
    /// Returns the logical bit level output on the serial TX wire (`0` or `1`).
    pub fn step_serial_bit(&mut self) -> u8 {
        if self.tx_bits_remaining > 0 {
            let out_bit = (self.tx_shift_reg & 1) as u8;
            self.tx_line = out_bit;

            // Shift right, inserting Mark (1) at the MSB
            self.tx_shift_reg = (self.tx_shift_reg >> 1) | (1 << 10);
            self.tx_bits_remaining -= 1;

            if self.tx_bits_remaining == 0 {
                self.tp_flag = true;
                self.tx_line = SERIAL_MARK;
                self.print_byte(self.tp_buffer);
            }

            out_bit
        } else {
            self.tx_line = SERIAL_MARK;
            self.tp_flag = true;
            SERIAL_MARK
        }
    }

    /// Feeds one serial bit from the external line into the serial receiver.
    ///
    /// Returns `Some(byte)` if a complete 11-bit character frame was assembled,
    /// setting `kb_flag = true`. Returns `None` while receiving or idle.
    pub fn rx_feed_bit(&mut self, bit: u8) -> Option<u8> {
        let bit = bit & 1;
        self.rx_line = bit;

        if !self.rx_active {
            if bit == SERIAL_SPACE {
                // Start bit detected
                self.rx_active = true;
                self.rx_bits_received = 0;
                self.rx_shift_reg = 0;
            }
            None
        } else {
            self.rx_bits_received += 1;
            if self.rx_bits_received <= 8 {
                // Bits 1..=8: data bits b0..b6 (bits 1..7) + parity bit (bit 8)
                let shift_pos = self.rx_bits_received - 1;
                self.rx_shift_reg |= (bit as u16) << shift_pos;
                None
            } else if self.rx_bits_received == 9 {
                // Stop bit 1
                None
            } else {
                // Stop bit 2 (bit 10) completes the frame
                self.rx_active = false;
                let assembled = (self.rx_shift_reg & 0xFF) as u8;
                self.kb_buffer = assembled;
                self.kb_flag = true;
                Some(assembled)
            }
        }
    }

    /// Pushes a character byte from the keyboard into the KRB buffer with parity,
    /// raising `kb_flag = true`.
    pub fn kbd_push_byte(&mut self, byte: u8) {
        let ascii = byte & 0x7F;
        let b0 = (ascii >> 0) & 1;
        let b1 = (ascii >> 1) & 1;
        let b2 = (ascii >> 2) & 1;
        let b3 = (ascii >> 3) & 1;
        let b4 = (ascii >> 4) & 1;
        let b5 = (ascii >> 5) & 1;
        let b6 = (ascii >> 6) & 1;
        let p = parity_generator_7xor(b0, b1, b2, b3, b4, b5, b6);
        let full_byte = ascii | (p << 7);
        self.kb_buffer = full_byte;
        self.kb_flag = true;
    }

    /// Reads the keyboard buffer `kb_buffer` and clears `kb_flag = false`.
    pub fn kbd_read_clear(&mut self) -> u8 {
        self.kb_flag = false;
        self.kb_buffer
    }

    /// Clears the teleprinter flag `tp_flag = false`.
    pub fn tp_clear_flag(&mut self) {
        self.tp_flag = false;
    }

    /// Processes an ASCII byte on the platen mechanics: handles auto-wrap at
    /// column 72, CR, LF, BEL ring counter, and uppercase folding.
    pub fn print_byte(&mut self, byte: u8) {
        let ascii = byte & 0x7F;
        match ascii {
            ASCII_BEL => {
                self.bel_count = self.bel_count.saturating_add(1);
            }
            ASCII_CR => {
                self.platen_col = 0;
            }
            ASCII_LF => {
                self.platen_row = self.platen_row.saturating_add(1);
            }
            0x20..=0x7E => {
                // Auto-wrap at column 72: wrap to col 0 on next line before printing
                if self.platen_col >= PLATEN_WIDTH as u8 {
                    self.platen_row = self.platen_row.saturating_add(1);
                    self.platen_col = 0;
                }

                // ASR-33 uppercase drum folding: 'a'..='z' -> 'A'..='Z'
                let printed = if (0x61..=0x7A).contains(&ascii) {
                    ascii - 0x20
                } else {
                    ascii
                };

                self.last_printed_char = Some(printed);
                self.platen_col = self.platen_col.saturating_add(1);
            }
            _ => {}
        }
    }
}

// ============================================================================
// Asr33GateLevel — Gate-Level Digital Logic Model
// ============================================================================

/// Cycle-accurate digital logic netlist implementation of the ASR-33 Teletype.
///
/// Built exclusively using primitive sequential and combinational building blocks
/// from `logic_gates`:
/// - 11-stage D flip-flop shift register for serial transmission (`shift_register`)
/// - 8-stage D flip-flop registers for KRB and TTO storage (`d_flip_flop`)
/// - Individual D flip-flops for `kb_flag` and `tp_flag`
/// - 7-XOR balanced even-parity generator tree
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asr33GateLevel {
    /// 11-stage transmit shift register flip-flops.
    pub tx_shift_ff: [FlipFlopState; BITS_PER_FRAME],
    /// 8-stage keyboard receive buffer (KRB) flip-flops.
    pub krb_ff: [FlipFlopState; 8],
    /// 8-stage teleprinter output buffer (TTO) flip-flops.
    pub tto_ff: [FlipFlopState; 8],
    /// Keyboard flag flip-flop.
    pub kb_flag_ff: FlipFlopState,
    /// Teleprinter flag flip-flop.
    pub tp_flag_ff: FlipFlopState,
    /// Serial TX wire output level.
    pub tx_line: u8,
    /// Transmit bits remaining.
    pub tx_bits_remaining: u8,
    /// 8-stage receive shift register flip-flops.
    pub rx_shift_ff: [FlipFlopState; 8],
    /// Serial RX wire input level.
    pub rx_line: u8,
    /// Bits received in current frame.
    pub rx_bits_received: u8,
    /// Receiver active flag.
    pub rx_active: bool,
}

impl Default for Asr33GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl Asr33GateLevel {
    /// Constructs a new gate-level ASR-33 simulator in idle state.
    pub fn new() -> Self {
        // Transmitter shift register initialized with Mark (all 1s)
        let tx_shift_ff = [
            FlipFlopState { master_q: 1, master_q_bar: 0, slave_q: 1, slave_q_bar: 0 },
            FlipFlopState { master_q: 1, master_q_bar: 0, slave_q: 1, slave_q_bar: 0 },
            FlipFlopState { master_q: 1, master_q_bar: 0, slave_q: 1, slave_q_bar: 0 },
            FlipFlopState { master_q: 1, master_q_bar: 0, slave_q: 1, slave_q_bar: 0 },
            FlipFlopState { master_q: 1, master_q_bar: 0, slave_q: 1, slave_q_bar: 0 },
            FlipFlopState { master_q: 1, master_q_bar: 0, slave_q: 1, slave_q_bar: 0 },
            FlipFlopState { master_q: 1, master_q_bar: 0, slave_q: 1, slave_q_bar: 0 },
            FlipFlopState { master_q: 1, master_q_bar: 0, slave_q: 1, slave_q_bar: 0 },
            FlipFlopState { master_q: 1, master_q_bar: 0, slave_q: 1, slave_q_bar: 0 },
            FlipFlopState { master_q: 1, master_q_bar: 0, slave_q: 1, slave_q_bar: 0 },
            FlipFlopState { master_q: 1, master_q_bar: 0, slave_q: 1, slave_q_bar: 0 },
        ];

        let krb_ff = std::array::from_fn(|_| FlipFlopState::default());
        let tto_ff = std::array::from_fn(|_| FlipFlopState::default());
        let rx_shift_ff = std::array::from_fn(|_| FlipFlopState::default());

        // tp_flag initialized to 1 (ready)
        let tp_flag_ff = FlipFlopState {
            master_q: 1,
            master_q_bar: 0,
            slave_q: 1,
            slave_q_bar: 0,
        };

        // kb_flag initialized to 0
        let kb_flag_ff = FlipFlopState::default();

        Self {
            tx_shift_ff,
            krb_ff,
            tto_ff,
            kb_flag_ff,
            tp_flag_ff,
            tx_line: SERIAL_MARK,
            tx_bits_remaining: 0,
            rx_shift_ff,
            rx_line: SERIAL_MARK,
            rx_bits_received: 0,
            rx_active: false,
        }
    }

    /// Loads a byte into the TTO register and 11-stage TX shift register.
    pub fn tx_load(&mut self, byte: u8) {
        let ascii = byte & 0x7F;
        let b0 = (ascii >> 0) & 1;
        let b1 = (ascii >> 1) & 1;
        let b2 = (ascii >> 2) & 1;
        let b3 = (ascii >> 3) & 1;
        let b4 = (ascii >> 4) & 1;
        let b5 = (ascii >> 5) & 1;
        let b6 = (ascii >> 6) & 1;
        let p = parity_generator_7xor(b0, b1, b2, b3, b4, b5, b6);

        // Latch 8 bits into TTO flip-flops via clock edges
        let tto_bits = [b0, b1, b2, b3, b4, b5, b6, p];
        for i in 0..8 {
            d_flip_flop(tto_bits[i], 0, &mut self.tto_ff[i]);
            d_flip_flop(tto_bits[i], 1, &mut self.tto_ff[i]);
        }

        // Clear tp_flag_ff (teleprinter busy)
        d_flip_flop(0, 0, &mut self.tp_flag_ff);
        d_flip_flop(0, 1, &mut self.tp_flag_ff);

        // Load 11-bit serial frame into tx_shift_ff:
        // [Start(0), b0, b1, b2, b3, b4, b5, b6, p, Stop1(1), Stop2(1)]
        let frame_bits = [0, b0, b1, b2, b3, b4, b5, b6, p, 1, 1];
        for i in 0..BITS_PER_FRAME {
            d_flip_flop(frame_bits[i], 0, &mut self.tx_shift_ff[i]);
            d_flip_flop(frame_bits[i], 1, &mut self.tx_shift_ff[i]);
        }

        self.tx_bits_remaining = 11;
        self.tx_line = SERIAL_SPACE; // Start bit active
    }

    /// Advances the TX shift register by one bit clock using `shift_register`.
    pub fn step_serial_bit(&mut self) -> u8 {
        if self.tx_bits_remaining > 0 {
            // Read bit 0 from stage 0
            let out_bit = self.tx_shift_ff[0].slave_q;
            self.tx_line = out_bit;

            // Shift left using logic-gates shift_register (shifting in Mark=1 from the right)
            shift_register(SERIAL_MARK, 0, &mut self.tx_shift_ff, "left");
            shift_register(SERIAL_MARK, 1, &mut self.tx_shift_ff, "left");

            self.tx_bits_remaining -= 1;

            if self.tx_bits_remaining == 0 {
                // Raise tp_flag_ff
                d_flip_flop(1, 0, &mut self.tp_flag_ff);
                d_flip_flop(1, 1, &mut self.tp_flag_ff);
                self.tx_line = SERIAL_MARK;
            }

            out_bit
        } else {
            self.tx_line = SERIAL_MARK;
            d_flip_flop(1, 0, &mut self.tp_flag_ff);
            d_flip_flop(1, 1, &mut self.tp_flag_ff);
            SERIAL_MARK
        }
    }

    /// Feeds one serial bit to the receiver shift register.
    pub fn rx_feed_bit(&mut self, bit: u8) -> Option<u8> {
        let bit = bit & 1;
        self.rx_line = bit;

        if !self.rx_active {
            if bit == SERIAL_SPACE {
                self.rx_active = true;
                self.rx_bits_received = 0;
                for ff in &mut self.rx_shift_ff {
                    d_flip_flop(0, 0, ff);
                    d_flip_flop(0, 1, ff);
                }
            }
            None
        } else {
            self.rx_bits_received += 1;
            if self.rx_bits_received <= 8 {
                let idx = (self.rx_bits_received - 1) as usize;
                d_flip_flop(bit, 0, &mut self.rx_shift_ff[idx]);
                d_flip_flop(bit, 1, &mut self.rx_shift_ff[idx]);
                None
            } else if self.rx_bits_received == 9 {
                None
            } else {
                self.rx_active = false;
                let mut assembled = 0u8;
                for i in 0..8 {
                    let b = self.rx_shift_ff[i].slave_q;
                    assembled |= (b & 1) << i;
                    d_flip_flop(b, 0, &mut self.krb_ff[i]);
                    d_flip_flop(b, 1, &mut self.krb_ff[i]);
                }
                d_flip_flop(1, 0, &mut self.kb_flag_ff);
                d_flip_flop(1, 1, &mut self.kb_flag_ff);
                Some(assembled)
            }
        }
    }

    /// Pushes a byte into KRB flip-flops and raises `kb_flag_ff`.
    pub fn kbd_push_byte(&mut self, byte: u8) {
        let ascii = byte & 0x7F;
        let b0 = (ascii >> 0) & 1;
        let b1 = (ascii >> 1) & 1;
        let b2 = (ascii >> 2) & 1;
        let b3 = (ascii >> 3) & 1;
        let b4 = (ascii >> 4) & 1;
        let b5 = (ascii >> 5) & 1;
        let b6 = (ascii >> 6) & 1;
        let p = parity_generator_7xor(b0, b1, b2, b3, b4, b5, b6);

        let krb_bits = [b0, b1, b2, b3, b4, b5, b6, p];
        for i in 0..8 {
            d_flip_flop(krb_bits[i], 0, &mut self.krb_ff[i]);
            d_flip_flop(krb_bits[i], 1, &mut self.krb_ff[i]);
        }

        d_flip_flop(1, 0, &mut self.kb_flag_ff);
        d_flip_flop(1, 1, &mut self.kb_flag_ff);
    }

    /// Reads KRB byte and clears `kb_flag_ff`.
    pub fn kbd_read_clear(&mut self) -> u8 {
        let mut val = 0u8;
        for i in 0..8 {
            val |= (self.krb_ff[i].slave_q & 1) << i;
        }
        d_flip_flop(0, 0, &mut self.kb_flag_ff);
        d_flip_flop(0, 1, &mut self.kb_flag_ff);
        val
    }

    /// Clears the teleprinter flag `tp_flag_ff`.
    pub fn tp_clear_flag(&mut self) {
        d_flip_flop(0, 0, &mut self.tp_flag_ff);
        d_flip_flop(0, 1, &mut self.tp_flag_ff);
    }

    /// Queries the teleprinter flag status.
    pub fn tp_flag(&self) -> bool {
        self.tp_flag_ff.slave_q == 1
    }

    /// Queries the keyboard flag status.
    pub fn kb_flag(&self) -> bool {
        self.kb_flag_ff.slave_q == 1
    }

    /// Reads the byte currently stored in KRB flip-flops.
    pub fn krb_byte(&self) -> u8 {
        let mut val = 0u8;
        for i in 0..8 {
            val |= (self.krb_ff[i].slave_q & 1) << i;
        }
        val
    }

    /// Reads the byte currently stored in TTO flip-flops.
    pub fn tto_byte(&self) -> u8 {
        let mut val = 0u8;
        for i in 0..8 {
            val |= (self.tto_ff[i].slave_q & 1) << i;
        }
        val
    }
}

// ============================================================================
// Asr33Functional — Complete Teletype Functional Model with Paper Roll
// ============================================================================

/// Full functional ASR-33 Teletype simulator with circular paper-roll history.
///
/// Contains `Asr33FrameState` and an in-memory 72-column paper roll buffer.
/// Implements zero-allocation operation during simulation steps, with declarative
/// `paint-instructions` scene generation (`to_paint_scene`) and ASCII rendering
/// (`render_paper_roll_ascii`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Asr33Functional {
    /// Internal register and serial frame state.
    pub state: Asr33FrameState,
    /// Circular buffer of 64 lines, each 72 columns wide.
    pub paper: [[u8; PLATEN_WIDTH]; MAX_PAPER_LINES],
    /// Tracked length (non-blank width) of each line in the circular buffer.
    pub line_lengths: [u8; MAX_PAPER_LINES],
}

impl Default for Asr33Functional {
    fn default() -> Self {
        Self::new()
    }
}

impl Asr33Functional {
    /// Creates a fresh ASR-33 simulator with a blank paper roll.
    pub fn new() -> Self {
        Self {
            state: Asr33FrameState::new(),
            paper: [[b' '; PLATEN_WIDTH]; MAX_PAPER_LINES],
            line_lengths: [0; MAX_PAPER_LINES],
        }
    }

    /// Constructs an instance wrapping a pre-configured `Asr33FrameState`.
    pub fn with_state(state: Asr33FrameState) -> Self {
        Self {
            state,
            paper: [[b' '; PLATEN_WIDTH]; MAX_PAPER_LINES],
            line_lengths: [0; MAX_PAPER_LINES],
        }
    }

    /// Read-only reference to the underlying `Asr33FrameState`.
    #[inline]
    pub fn state(&self) -> &Asr33FrameState {
        &self.state
    }

    /// Mutable reference to the underlying `Asr33FrameState`.
    #[inline]
    pub fn state_mut(&mut self) -> &mut Asr33FrameState {
        &mut self.state
    }

    /// Loads a byte into the transmitter, assembling the 11-bit serial frame.
    #[inline]
    pub fn tx_load(&mut self, byte: u8) {
        self.state.tx_load(byte);
    }

    /// Steps one bit clock: transmits a bit, and upon character completion,
    /// records the printed character onto the paper roll.
    pub fn step_serial_bit(&mut self) -> u8 {
        let prev_remaining = self.state.tx_bits_remaining;
        let prev_row = self.state.platen_row;
        let bit = self.state.step_serial_bit();

        if prev_remaining == 1 && self.state.tx_bits_remaining == 0 {
            self.sync_paper_after_print(prev_row, self.state.tp_buffer);
        }

        bit
    }

    /// Feeds one serial bit to the receiver.
    #[inline]
    pub fn rx_feed_bit(&mut self, bit: u8) -> Option<u8> {
        self.state.rx_feed_bit(bit)
    }

    /// Pushes a byte into the keyboard reader buffer.
    #[inline]
    pub fn kbd_push_byte(&mut self, byte: u8) {
        self.state.kbd_push_byte(byte);
    }

    /// Reads the keyboard reader buffer and clears the keyboard flag.
    #[inline]
    pub fn kbd_read_clear(&mut self) -> u8 {
        self.state.kbd_read_clear()
    }

    /// Clears the teleprinter flag.
    #[inline]
    pub fn tp_clear_flag(&mut self) {
        self.state.tp_clear_flag();
    }

    /// Queries the teleprinter flag.
    #[inline]
    pub fn tp_flag(&self) -> bool {
        self.state.tp_flag
    }

    /// Queries the keyboard flag.
    #[inline]
    pub fn kb_flag(&self) -> bool {
        self.state.kb_flag
    }

    /// Queries current TX wire level.
    #[inline]
    pub fn tx_line(&self) -> u8 {
        self.state.tx_line
    }

    /// Returns current platen column index (`0..=71`).
    #[inline]
    pub fn platen_col(&self) -> u8 {
        self.state.platen_col
    }

    /// Returns current platen row index.
    #[inline]
    pub fn platen_row(&self) -> u32 {
        self.state.platen_row
    }

    /// Returns total count of bell rings.
    #[inline]
    pub fn bel_count(&self) -> u32 {
        self.state.bel_count
    }

    /// Prints a character directly onto the paper roll platen, updating columns,
    /// rows, wrapping at col 72, and folding `'a'..='z'` to `'A'..='Z'`.
    pub fn print_char(&mut self, ch: u8) {
        let prev_row = self.state.platen_row;
        self.state.print_byte(ch);
        self.sync_paper_after_print(prev_row, ch);
    }

    fn sync_paper_after_print(&mut self, prev_row: u32, ch: u8) {
        if self.state.platen_row != prev_row {
            let row_idx = (self.state.platen_row as usize) % MAX_PAPER_LINES;
            self.paper[row_idx] = [b' '; PLATEN_WIDTH];
            self.line_lengths[row_idx] = 0;
        }

        let ascii = ch & 0x7F;
        if (0x20..=0x7E).contains(&ascii) {
            if let Some(printed) = self.state.last_printed_char {
                let row_idx = (self.state.platen_row as usize) % MAX_PAPER_LINES;
                let col = (self.state.platen_col.saturating_sub(1) as usize).min(PLATEN_WIDTH - 1);
                self.paper[row_idx][col] = printed;
                let new_len = self.state.platen_col.max(self.line_lengths[row_idx]).min(PLATEN_WIDTH as u8);
                self.line_lengths[row_idx] = new_len;
            }
        }
    }

    /// Returns the accumulated text on the paper roll as a UTF-8 string.
    pub fn paper_roll_text(&self) -> String {
        let start_row = self.state.platen_row.saturating_sub((MAX_PAPER_LINES as u32) - 1);
        let total_lines = (self.state.platen_row - start_row) as usize + 1;

        let mut lines = Vec::with_capacity(total_lines);
        for r in start_row..=self.state.platen_row {
            let row_idx = (r as usize) % MAX_PAPER_LINES;
            let len = (self.line_lengths[row_idx] as usize).min(PLATEN_WIDTH);
            let line_slice = &self.paper[row_idx][..len];
            let line_str = std::str::from_utf8(line_slice).unwrap_or("");
            lines.push(line_str.trim_end());
        }

        lines.join("\n")
    }

    /// Converts current paper roll state and print carriage position into a declarative [`PaintScene`].
    pub fn to_paint_scene(&self) -> PaintScene {
        let start_row = self.state.platen_row.saturating_sub((MAX_PAPER_LINES as u32) - 1);
        let total_lines = (self.state.platen_row - start_row) as usize + 1;
        let height = total_lines.max(1) as f64;
        let mut scene = PaintScene::new(PLATEN_WIDTH as f64, height);
        scene.background = "#ffffff".to_string();

        let mut lines_text = Vec::with_capacity(total_lines);
        for r in start_row..=self.state.platen_row {
            let row_idx = (r as usize) % MAX_PAPER_LINES;
            let line_str = std::str::from_utf8(&self.paper[row_idx]).unwrap_or("");
            lines_text.push(line_str.to_string());
        }

        let full_text = lines_text.join("\n");
        scene.instructions.push(PaintInstruction::Text(PaintText {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            text: full_text,
            font_ref: None,
            font_size: 1.0,
            fill: Some("#000000".to_string()),
            text_align: Some(TextAlign::Left),
        }));

        // Render carriage print-head cursor as a filled 1x1 block
        let cursor_col = (self.state.platen_col.min(71)) as f64;
        let cursor_row = (self.state.platen_row - start_row) as f64;
        scene.instructions.push(PaintInstruction::Rect(PaintRect::filled(
            cursor_col,
            cursor_row,
            1.0,
            1.0,
            "#000000",
        )));

        scene
    }

    /// Renders the 72-column paper roll + print carriage cursor into a deterministic ASCII string.
    pub fn render_paper_roll_ascii(&self) -> String {
        let scene = self.to_paint_scene();
        render(
            &scene,
            AsciiOptions {
                scale_x: 1,
                scale_y: 1,
            },
        )
        .unwrap_or_else(|_| self.paper_roll_text())
    }
}

// ============================================================================
// Public Standalone Entry Points
// ============================================================================

/// Constructs a new functional ASR-33 simulator.
pub fn new_asr33() -> Asr33Functional {
    Asr33Functional::new()
}

/// Constructs a new gate-level ASR-33 simulator.
pub fn new_asr33_gate_level() -> Asr33GateLevel {
    Asr33GateLevel::new()
}

/// Translates an ASR-33 functional state into a declarative [`PaintScene`].
pub fn to_paint_scene(asr: &Asr33Functional) -> PaintScene {
    asr.to_paint_scene()
}

/// Renders an ASR-33 functional state to an ASCII paper-roll string.
pub fn render_paper_roll_ascii(asr: &Asr33Functional) -> String {
    asr.render_paper_roll_ascii()
}
