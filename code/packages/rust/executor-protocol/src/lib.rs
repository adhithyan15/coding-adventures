//! # `executor-protocol` — Wire format and transport between runtime and executors
//!
//! This is the implementation of spec **MX03**.  See:
//!
//! - [`code/specs/MX03-executor-protocol.md`] — the contract
//! - [`code/specs/MX00-matrix-execution-overview.md`] — architecture
//!
//! The unifying principle: **anything that crosses from runtime to
//! executor goes as bytes**.  Local executors run the same code path
//! as remote ones; in-process is just one transport implementation.
//!
//! ## What lives here
//!
//! - **Messages** ([`ExecutorRequest`], [`ExecutorResponse`],
//!   [`ExecutorEvent`]) — the fixed set of messages an executor can
//!   answer and produce.
//! - **Sub-types** ([`KernelSource`], [`BackendProfile`], [`OpTiming`],
//!   [`ErrorCode`]) — building blocks of the messages.
//! - **Frame** ([`MessageFrame`]) — the top-level versioned envelope
//!   that wraps every message.
//! - **Wire format** ([`Frame::to_bytes`], [`Frame::from_bytes`]) —
//!   hand-rolled binary encoding per spec MX03 §"Wire format
//!   primitives".
//! - **Transport trait** ([`Transport`]) — pluggable wire layer.
//! - **`LocalTransport`** — in-process transport that calls the
//!   executor handler directly.  In debug builds it round-trips
//!   through serialisation to enforce the discipline.
//! - **`block_on`** — hand-rolled minimal async runner so the local
//!   transport's async signatures resolve without a dependency on a
//!   real async runtime.
//! - **`KernelCacheKey`** — SipHash-based content key for the
//!   executor-side kernel cache.
//!
//! ## Zero dependencies
//!
//! Per the MX00 zero-dependency mandate, this crate uses only `core`,
//! `alloc`, `std`, and the upstream `matrix-ir` and `compute-ir`
//! (path-only, both zero-dep).  No `serde`, no `bincode`, no `tokio`,
//! no `futures`, no `async-trait`.

#![warn(rust_2018_idioms)]

mod frame;
mod messages;
mod wire;
mod transport;
mod local;
mod block_on;
mod kernel_cache;

pub use frame::{MessageFrame, MessageKind};
pub use messages::{
    BackendProfile, ErrorCode, ExecutorEvent, ExecutorRequest, ExecutorResponse,
    KernelSource, OpTiming,
};
pub use transport::{Transport, TransportError};
pub use local::LocalTransport;
pub use block_on::block_on;
pub use kernel_cache::KernelCacheKey;
pub use wire::WireError;

/// Protocol version.  Distinct from the wire-frame version; bump when
/// a message variant's payload layout changes incompatibly.
///
/// History:
///
/// - **v1**: initial release.  10 request variants (`Register` through
///   `Shutdown`), 11 response variants, 3 event variants.
/// - **v2** (this revision): adds `ExecutorRequest::DispatchSpecialised`
///   (tag 0x0A) for MX05 Phase 4.1 specialised-kernel dispatch.
///   Adds `ErrorCode::NOT_IMPLEMENTED` (0x0062) for backends that
///   recognise the request shape but haven't yet wired up execution.
///   Forward-compatible with v1 senders: every existing variant
///   still encodes/decodes byte-identically.
pub const PROTOCOL_VERSION: u32 = 2;
