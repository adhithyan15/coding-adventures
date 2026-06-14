//! `native-debug-info` — DWARF 4 and CodeView 4 debug-section emitter (LANG14).
//!
//! This crate converts source-location data from a [`DebugSidecarReader`] into
//! the native debug sections understood by gdb/lldb (DWARF 4) on Linux/macOS
//! and WinDbg/Visual Studio (CodeView 4) on Windows.
//!
//! # Public API
//!
//! - [`DwarfEmitter`] — builds `.debug_abbrev`, `.debug_info`, `.debug_line`,
//!   `.debug_str` and embeds them into an ELF64 or Mach-O 64-bit binary.
//!
//! - [`CodeViewEmitter`] — builds `.debug$S` and `.debug$T` and embeds them
//!   into a PE32+ binary.
//!
//! - [`embed_debug_info`] — convenience dispatcher that auto-detects the target
//!   platform and calls the correct emitter.
//!
//! - [`encode_uleb128`], [`encode_sleb128`], [`decode_uleb128`], [`decode_sleb128`]
//!   — ULEB128 / SLEB128 encode/decode used by the DWARF emitter (also exported
//!   for downstream packages that emit their own DWARF streams).
//!
//! # Pipeline position
//!
//! ```text
//! aot-core.compile(module) → .aot binary + sidecar bytes
//!   │
//!   ↓ DebugSidecarReader::new(sidecar_bytes)
//!   │
//!   ├── DwarfEmitter::embed_in_elf(elf_bytes)     → ELF + DWARF
//!   ├── DwarfEmitter::embed_in_macho(macho_bytes) → Mach-O + DWARF
//!   └── CodeViewEmitter::embed_in_pe(pe_bytes)    → PE + CodeView
//! ```
//!
//! # Quick start
//!
//! ```
//! use native_debug_info::{encode_uleb128, decode_uleb128};
//!
//! let encoded = encode_uleb128(624485);
//! let (decoded, _) = decode_uleb128(&encoded, 0);
//! assert_eq!(decoded, 624485);
//! ```

pub mod codeview;
pub mod dwarf;
pub mod embed;
pub mod leb128;

pub use codeview::CodeViewEmitter;
pub use dwarf::DwarfEmitter;
pub use embed::{embed_debug_info, ArtifactInfo};
pub use leb128::{decode_sleb128, decode_uleb128, encode_sleb128, encode_uleb128};
