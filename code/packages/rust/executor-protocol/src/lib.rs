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
    BackendProfile, ErrorCode, ExecutorEvent, ExecutorRequest, ExecutorResponse, KernelSource,
    OpTiming, EXECUTOR_EVENT_VARIANTS, EXECUTOR_REQUEST_VARIANTS, EXECUTOR_RESPONSE_VARIANTS,
    KERNEL_SOURCE_VARIANTS,
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

/// Compact, payload-free description of the current protocol surface.
///
/// Hosts and catalogs can persist or compare this value without
/// constructing sample [`ExecutorRequest`] payloads or exposing kernel
/// source bytes in diagnostics.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ProtocolSurfaceSummary {
    /// Payload protocol version expected inside [`MessageFrame`] payloads.
    pub protocol_version: u32,
    /// Top-level frame layout version.
    pub frame_version: u8,
    /// Number of [`MessageKind`] variants accepted by this crate.
    pub message_kinds: usize,
    /// Number of [`ExecutorRequest`] variants.
    pub request_variants: usize,
    /// Number of [`ExecutorResponse`] variants.
    pub response_variants: usize,
    /// Number of [`ExecutorEvent`] variants.
    pub event_variants: usize,
    /// Number of [`KernelSource`] variants.
    pub kernel_source_variants: usize,
    /// Whether `DispatchSpecialised` is part of this protocol surface.
    pub supports_dispatch_specialised: bool,
    /// Soft-refusal code used when a known request shape is not wired up.
    pub not_implemented_code: ErrorCode,
}

impl ProtocolSurfaceSummary {
    /// Summary for the protocol surface compiled into this crate.
    pub const fn current() -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            frame_version: frame::FRAME_VERSION,
            message_kinds: 3,
            request_variants: EXECUTOR_REQUEST_VARIANTS,
            response_variants: EXECUTOR_RESPONSE_VARIANTS,
            event_variants: EXECUTOR_EVENT_VARIANTS,
            kernel_source_variants: KERNEL_SOURCE_VARIANTS,
            supports_dispatch_specialised: true,
            not_implemented_code: ErrorCode::NOT_IMPLEMENTED,
        }
    }
}

/// Return a compact, payload-free summary of the current executor
/// protocol surface.
pub const fn protocol_surface_summary() -> ProtocolSurfaceSummary {
    ProtocolSurfaceSummary::current()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_surface_summary_reports_current_contract() {
        let summary = protocol_surface_summary();
        assert_eq!(summary.protocol_version, PROTOCOL_VERSION);
        assert_eq!(summary.frame_version, frame::FRAME_VERSION);
        assert_eq!(summary.message_kinds, 3);
        assert_eq!(summary.request_variants, EXECUTOR_REQUEST_VARIANTS);
        assert_eq!(summary.response_variants, EXECUTOR_RESPONSE_VARIANTS);
        assert_eq!(summary.event_variants, EXECUTOR_EVENT_VARIANTS);
        assert_eq!(summary.kernel_source_variants, KERNEL_SOURCE_VARIANTS);
        assert!(summary.supports_dispatch_specialised);
        assert_eq!(summary.not_implemented_code, ErrorCode::NOT_IMPLEMENTED);
    }
}
