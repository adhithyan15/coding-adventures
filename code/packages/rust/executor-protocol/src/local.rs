//! `LocalTransport` — in-process transport for executors that live in
//! the same process as the runtime.
//!
//! Implementation: holds an `Arc<dyn Fn(ExecutorRequest) -> ExecutorResponse>`
//! that the runtime invokes synchronously.  `request()` returns a
//! future that resolves immediately with the handler's output.
//!
//! ## Debug-build serialisation discipline
//!
//! In debug builds, `request()` round-trips the request and response
//! through the wire format before invoking the handler.  This catches
//! "I accidentally put a non-serializable type in the protocol"
//! bugs — every change to the message types is exercised by the
//! local-transport path on every CI run.
//!
//! In release builds, the round-trip is skipped for performance.
//! There is no functional difference between the two paths; the
//! discipline only enforces *that the protocol is serialisable at
//! all*.

use crate::frame::MessageFrame;
use crate::messages::{ExecutorRequest, ExecutorResponse};
use crate::transport::{Transport, TransportError};
use core::future::Future;
use std::sync::Arc;

/// Type alias for an in-process executor handler.  Returns the
/// response synchronously.
pub type LocalHandler = Arc<dyn Fn(ExecutorRequest) -> ExecutorResponse + Send + Sync>;

/// In-process transport.  Holds a closure that handles requests
/// directly.
#[derive(Clone)]
pub struct LocalTransport {
    handler: LocalHandler,
}

impl LocalTransport {
    /// Construct a `LocalTransport` from a handler closure.  The
    /// handler must be `Send + Sync` because the transport may be
    /// shared across threads.
    pub fn new<H>(handler: H) -> Self
    where
        H: Fn(ExecutorRequest) -> ExecutorResponse + Send + Sync + 'static,
    {
        LocalTransport {
            handler: Arc::new(handler),
        }
    }
}

impl Transport for LocalTransport {
    fn request(
        &self,
        req: ExecutorRequest,
    ) -> impl Future<Output = Result<ExecutorResponse, TransportError>> + Send {
        let handler = self.handler.clone();
        async move {
            // Debug-build discipline: round-trip through the wire format
            // to catch any non-serializable additions to the protocol.
            #[cfg(debug_assertions)]
            let req = {
                let frame = MessageFrame::request(0, &req);
                let bytes = frame.to_bytes();
                let decoded_frame = MessageFrame::from_bytes(&bytes)?;
                decoded_frame.as_request()?
            };

            let resp = handler(req);

            #[cfg(debug_assertions)]
            let resp = {
                let frame = MessageFrame::response(0, &resp);
                let bytes = frame.to_bytes();
                let decoded_frame = MessageFrame::from_bytes(&bytes)?;
                decoded_frame.as_response()?
            };

            Ok(resp)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_on;
    use crate::messages::ExecutorRequest;
    use compute_ir::BufferId;

    #[test]
    fn echo_handler_round_trips() {
        // A trivial handler that responds to AllocBuffer with a
        // BufferAllocated whose id is the requested byte count.
        let t = LocalTransport::new(|req| match req {
            ExecutorRequest::AllocBuffer { bytes } => ExecutorResponse::BufferAllocated {
                buffer: BufferId(bytes),
            },
            _ => ExecutorResponse::ShuttingDown,
        });

        let resp = block_on(t.request(ExecutorRequest::AllocBuffer { bytes: 42 })).unwrap();
        match resp {
            ExecutorResponse::BufferAllocated { buffer } => assert_eq!(buffer, BufferId(42)),
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[test]
    fn heartbeat_round_trips() {
        let t = LocalTransport::new(|_| ExecutorResponse::Alive {
            profile: stub_profile(),
        });
        let resp = block_on(t.request(ExecutorRequest::Heartbeat)).unwrap();
        assert!(matches!(resp, ExecutorResponse::Alive { .. }));
    }

    fn stub_profile() -> crate::BackendProfile {
        crate::BackendProfile {
            kind: "test".to_string(),
            supported_ops: 0,
            supported_dtypes: 0,
            gflops_f32: 0,
            gflops_u8: 0,
            gflops_i32: 0,
            host_to_device_bw: 0,
            device_to_host_bw: 0,
            device_internal_bw: 0,
            launch_overhead_ns: 0,
            transport_latency_ns: 0,
            on_device_mib: 0,
            max_tensor_rank: 0,
            max_dim: 0,
        }
    }
}
