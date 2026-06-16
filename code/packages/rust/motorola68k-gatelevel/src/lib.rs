//! # Motorola 68000 Gate-Level Simulator
//!
//! Every arithmetic and logical operation routes through real gate functions:
//! `AND → OR → XOR → NOT → full_adder → ripple-carry adder → ALU`.
//! Registers are modelled as 32-bit D flip-flop arrays.
//!
//! ## Why gate-level?
//!
//! The real Motorola 68000 had ~68,000 transistors (NMOS, 3.5-micron, 1979).
//! By simulating at gate level, we can count exactly how many gates each
//! operation uses and trace a bit through the 32-bit ripple-carry adder
//! (32 full-adder stages ≈ 160 gate outputs).
//!
//! ## Architecture
//!
//! ```text
//! bits.rs      — integer ↔ bit-vector conversion (8/16/32-bit)
//!                ripple-carry adders: add_8bit_full, add_16bit_full, add_32bit_full
//!                flag helpers: compute_n, compute_z, compute_v_from_carries
//!                bitwise NOT helpers: not_8bit, not_16bit, not_32bit
//! alu.rs       — AluResult68K: all ALU operations through gate primitives
//!                add/sub/neg/negx (8/16/32-bit); and/or/xor/not (8/16/32-bit)
//!                cmp, shift_op (ASL/LSL/ASR/LSR/ROXL/ROXR/ROL/ROR)
//! registers.rs — RegisterFile68K: D0–D7, A0–A7, PC, SR
//!                read/write Dn with size (byte/word/long, upper bits preserved)
//!                write_an with word sign-extension
//!                set_ccr / set_nz_clear_vc / negx_z for flag updates
//!                test_cc: all 16 condition codes (T/F/HI/LS/CC/CS/…)
//! cpu.rs       — Cpu68K: full fetch-decode-execute loop
//!                big-endian memory helpers (16 MB flat)
//!                EA resolution (all 14 addressing modes)
//!                ~100 opcodes across all instruction groups
//! ```
//!
//! ## Design constraints
//!
//! | Area           | Constraint |
//! |----------------|-----------|
//! | Data path      | Every +/–/AND/OR/XOR goes through `full_adder` / gate functions |
//! | MUL/DIV        | Host arithmetic — gate-level ×16 multiplier is out of scope |
//! | Address space  | 24-bit flat (16 MB) via `& 0x00FF_FFFF` |
//! | Memory         | 16 MB flat `Box<[u8; 0x100_0000]>`, big-endian byte order |
//!
//! ## Example
//!
//! ```rust
//! use coding_adventures_motorola68k_gatelevel::cpu::Cpu68K;
//!
//! let mut cpu = Cpu68K::new();
//! // MOVEQ #5, D0; MOVEQ #3, D1; ADD.L D1, D0; STOP #0x2700
//! let steps = cpu.execute(&[
//!     0x70, 0x05,              // MOVEQ #5, D0
//!     0x72, 0x03,              // MOVEQ #3, D1
//!     0xD0, 0x81,              // ADD.L D1, D0
//!     0x4E, 0x72, 0x27, 0x00, // STOP #0x2700
//! ], 1000);
//! assert_eq!(cpu.rf.d[0], 8);
//! assert_eq!(cpu.rf.flag_c(), 0);
//! assert!(cpu.halted);
//! ```

pub mod alu;
pub mod bits;
pub mod cpu;
pub mod registers;

pub use cpu::Cpu68K;
