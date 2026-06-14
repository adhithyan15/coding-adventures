//! # GDSII Stream Format Writer
//!
//! Encodes a digital layout as a Calma GDSII binary stream (1978 format).
//! Used by every silicon foundry to describe cell geometries.
//!
//! ## Record format
//!
//! Every record is: `[2-byte length] [1-byte record_type] [1-byte data_type] [payload]`
//!
//! Length includes the 4-byte header. Payload encoding depends on data_type:
//! - 0x00 → no data
//! - 0x01 → bit array (2 bytes per entry)
//! - 0x02 → 2-byte signed integer array
//! - 0x03 → 4-byte signed integer array (XY coordinates)
//! - 0x05 → 8-byte GDSII real
//! - 0x06 → ASCII string (padded to even length)
//!
//! ## GDSII real (8-byte)
//!
//! A signed fraction with 7-bit base-16 exponent (excess-64 bias) and
//! 56-bit mantissa. Very different from IEEE 754.
//!
//! ## Record types implemented
//!
//! HEADER, BGNLIB, LIBNAME, UNITS, ENDLIB,
//! BGNSTR, STRNAME, ENDSTR,
//! BOUNDARY, PATH, SREF, TEXT,
//! LAYER, DATATYPE, XY, WIDTH, SNAME, STRING, ENDEL.
//!
//! ## Usage
//!
//! ```rust
//! use gdsii_writer::{GdsWriter, GdsCell, GdsBoundary};
//!
//! let mut writer = GdsWriter::new("mylib");
//! let mut cell = GdsCell::new("top");
//! cell.boundaries.push(GdsBoundary {
//!     layer: 68, datatype: 20,
//!     xy: vec![(0, 0), (100, 0), (100, 272), (0, 272), (0, 0)],
//! });
//! writer.cells.push(cell);
//! let bytes = writer.encode();
//! // GDS magic: starts with HEADER record.
//! assert_eq!(&bytes[0..2], &[0x00, 0x06]);  // length = 6 bytes
//! ```

pub mod stream;

pub use stream::{GdsBoundary, GdsCell, GdsPath, GdsSref, GdsText, GdsWriter};
