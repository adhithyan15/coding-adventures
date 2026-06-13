//! # fpga-bitstream
//!
//! Emits iCE40 bitstreams in the Project IceStorm record-stream format.
//!
//! ## Background: what is an FPGA bitstream?
//!
//! An FPGA holds thousands of tiny programmable blocks — LUTs, flip-flops,
//! routing muxes.  Each block has a set of *configuration bits* stored in
//! CRAM (Configuration RAM).  A *bitstream* is the binary blob that programs
//! all those bits at power-on (or via USB with `iceprog`).
//!
//! ## IceStorm record-stream format
//!
//! The iCE40 bitstream is a sequence of variable-length records, each with the
//! structure:
//!
//! ```text
//! offset  size   field
//! ------  ----   -----
//! 0       1      total record length (including this byte and the command byte)
//! 1       1      command code
//! 2..n    n-2    payload
//! ```
//!
//! The preamble is two magic bytes: `0xFF 0x00`.
//! The stream terminates with the end marker `0xFFFF`.
//!
//! ## v0.1.0 limitations
//!
//! This implementation emits a *structurally correct* record stream with a
//! stub CRAM image (zeros).  For a bitstream loadable on real iCE40 hardware
//! you need Project IceStorm's chip database to map per-tile LUT truth-table
//! bits to the correct (row, col, bit-offset) positions in the CRAM image.
//! For that, use the `real-fpga-export` package's `icepack` shell-out path.
//!
//! ## Quick start
//!
//! ```rust
//! use fpga_bitstream::{FpgaConfig, ClbConfig, Ice40Part, emit_bitstream, write_bin};
//! use std::collections::HashMap;
//!
//! let mut config = FpgaConfig::new(Ice40Part::Hx1k);
//! config.clbs.insert((0, 0), ClbConfig::default());
//! let (bytes, report) = emit_bitstream(&config);
//! assert_eq!(bytes[0], 0xFF);  // preamble
//! assert_eq!(bytes[1], 0x00);
//! assert_eq!(bytes[bytes.len()-2..], [0xFF, 0xFF]); // end marker
//! ```

pub mod bitstream;

pub use bitstream::{
    emit_bitstream, part_specs, write_bin,
    BitstreamReport, ClbConfig, FpgaConfig, Ice40Part, PART_SPECS,
};
