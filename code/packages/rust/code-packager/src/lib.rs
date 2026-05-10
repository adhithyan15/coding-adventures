//! # code-packager
//!
//! Cross-platform binary packaging for compiled machine code (LANG10).
//!
//! This crate wraps raw native bytes in OS-specific binary container formats
//! so that operating systems, runtimes, and embedded programmers can load and
//! execute the code.
//!
//! ## Supported output formats
//!
//! ```text
//! Format      │ Platform          │ File ext │ Factory method
//! ────────────┼───────────────────┼──────────┼─────────────────────────
//! ELF64       │ Linux (x64/arm64) │ .elf     │ Target::linux_x64()
//! Mach-O 64   │ macOS (x64/arm64) │ .macho   │ Target::macos_arm64()
//! PE32+       │ Windows x64       │ .exe     │ Target::windows_x64()
//! WebAssembly │ Browser/Wasmtime  │ .wasm    │ Target::wasm()
//! Raw binary  │ Bare-metal/BIOS   │ .bin     │ Target::raw("x86_64")
//! Intel HEX   │ Microcontrollers  │ .hex     │ Target::intel_4004()
//! ```
//!
//! ## Quick start
//!
//! ```rust
//! use code_packager::{CodeArtifact, PackagerRegistry, Target};
//!
//! // 1. Describe the target platform.
//! let target = Target::linux_x64();
//!
//! // 2. Wrap the native bytes in a CodeArtifact.
//! //    `entry_point` is the byte offset of the first instruction.
//! let artifact = CodeArtifact::new(
//!     vec![0x48, 0x31, 0xC0, // xor rax, rax  (return 0)
//!          0xC3],            // ret
//!     0, // entry_point
//!     target,
//! );
//!
//! // 3. Pack into the target binary format.
//! let bytes: Vec<u8> = PackagerRegistry::pack(&artifact).unwrap();
//!
//! // 4. `bytes` is now a valid Linux ELF64 executable.
//! assert_eq!(&bytes[0..4], &[0x7F, b'E', b'L', b'F']);
//! ```
//!
//! ## Architecture
//!
//! ```text
//! CodeArtifact ──► PackagerRegistry::pack
//!                        │
//!          ┌─────────────┼──────────────────┐
//!          ▼             ▼                  ▼
//!       elf64::pack  macho64::pack      pe::pack
//!       raw::pack    intel_hex::pack    wasm::pack
//! ```
//!
//! Each packager module is independent: they only import `artifact`, `errors`,
//! and `target`. The registry dispatches by `artifact.target.binary_format`.
//!
//! ## Metadata hints
//!
//! Some packagers accept metadata for fine-grained control:
//!
//! ```text
//! Key            │ Type │ Packagers          │ Meaning
//! ───────────────┼──────┼────────────────────┼────────────────────────────
//! "load_address" │ Int  │ elf64, macho64     │ Virtual address of segment
//! "origin"       │ Int  │ intel_hex          │ Start address (16-bit)
//! "exports"      │ List │ wasm               │ WASM export function names
//! ```
//!
//! Use `CodeArtifact::with_metadata(...)` to pass these hints.
//!
//! ## No unsafe code
//!
//! All binary construction uses `.to_le_bytes()` and `extend_from_slice` on
//! `Vec<u8>`. There is no `unsafe` anywhere in this crate.

pub mod artifact;
pub mod elf64;
pub mod errors;
pub mod intel_hex;
pub mod macho64;
pub mod macho_object;
pub mod pe;
pub mod raw;
pub mod registry;
pub mod target;
pub mod wasm;

pub use artifact::{CodeArtifact, MetadataValue};
pub use errors::PackagerError;
pub use registry::PackagerRegistry;
pub use target::Target;
