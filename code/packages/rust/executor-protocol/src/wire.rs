//! Hand-rolled binary wire format for the executor protocol.
//!
//! Same primitives as `matrix-ir` and `compute-ir` (varint,
//! length-prefixed bytes, tagged unions) — see spec MX03
//! §"Wire format primitives".
//!
//! ## Security
//!
//! Like the upstream wire codecs, this one is hardened against
//! malicious input:
//!
//! - Every length-prefixed allocation is bounded against remaining
//!   buffer bytes.
//! - `Reader::need` uses `checked_add`.
//! - `bytes()` rejects `u64` lengths exceeding `usize::MAX`.
//! - Varint is capped at 10 bytes.

use crate::frame::{MessageFrame, MessageKind, FRAME_VERSION};
use crate::messages::{
    BackendProfile, ErrorCode, ExecutorEvent, ExecutorRequest, ExecutorResponse, KernelSource,
    OpTiming,
};
use compute_ir::{BufferId, ComputeGraph, ExecutorId, KernelId};

// ─────────────────────── error type ───────────────────────

/// Errors produced by wire encoding/decoding.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WireError {
    /// Truncated input.
    UnexpectedEof,
    /// Frame version doesn't match this reader.
    UnsupportedFrameVersion(u8),
    /// Message kind byte is unknown.
    UnknownMessageKind(u8),
    /// Tagged-union variant tag is unknown.
    UnknownTag { what: &'static str, tag: u64 },
    /// Varint exceeds 10 bytes.
    OversizedVarint,
    /// String payload is not valid UTF-8.
    InvalidUtf8,
    /// Trailing bytes after the parsed payload.
    TrailingBytes,
    /// Underlying compute-ir wire error (when decoding embedded
    /// `ComputeGraph` payloads).
    ComputeIr(compute_ir::ComputeIrError),
}

impl From<compute_ir::ComputeIrError> for WireError {
    fn from(e: compute_ir::ComputeIrError) -> Self {
        WireError::ComputeIr(e)
    }
}

// ─────────────────────── primitives ───────────────────────

struct Writer<'a> {
    out: &'a mut Vec<u8>,
}

impl<'a> Writer<'a> {
    fn new(out: &'a mut Vec<u8>) -> Self {
        Writer { out }
    }
    fn u8(&mut self, v: u8) {
        self.out.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    fn uv64(&mut self, mut v: u64) {
        while v >= 0x80 {
            self.out.push(((v as u8) & 0x7F) | 0x80);
            v >>= 7;
        }
        self.out.push(v as u8);
    }
    fn bytes(&mut self, b: &[u8]) {
        self.uv64(b.len() as u64);
        self.out.extend_from_slice(b);
    }
    fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }
    fn need(&self, n: usize) -> Result<(), WireError> {
        let end = self.pos.checked_add(n).ok_or(WireError::UnexpectedEof)?;
        if end > self.buf.len() {
            Err(WireError::UnexpectedEof)
        } else {
            Ok(())
        }
    }
    fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }
    fn u8(&mut self) -> Result<u8, WireError> {
        self.need(1)?;
        let v = self.buf[self.pos];
        self.pos += 1;
        Ok(v)
    }
    fn u16(&mut self) -> Result<u16, WireError> {
        self.need(2)?;
        let v = u16::from_le_bytes(self.buf[self.pos..self.pos + 2].try_into().unwrap());
        self.pos += 2;
        Ok(v)
    }
    fn u32(&mut self) -> Result<u32, WireError> {
        self.need(4)?;
        let v = u32::from_le_bytes(self.buf[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        Ok(v)
    }
    fn u64(&mut self) -> Result<u64, WireError> {
        self.need(8)?;
        let v = u64::from_le_bytes(self.buf[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        Ok(v)
    }
    fn uv64(&mut self) -> Result<u64, WireError> {
        let mut acc: u64 = 0;
        let mut shift = 0u32;
        for _ in 0..10 {
            let b = self.u8()?;
            acc |= ((b & 0x7F) as u64) << shift;
            if b & 0x80 == 0 {
                return Ok(acc);
            }
            shift += 7;
        }
        Err(WireError::OversizedVarint)
    }
    fn bytes(&mut self) -> Result<Vec<u8>, WireError> {
        let len_u64 = self.uv64()?;
        if len_u64 > usize::MAX as u64 {
            return Err(WireError::UnexpectedEof);
        }
        let len = len_u64 as usize;
        self.need(len)?;
        let v = self.buf[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Ok(v)
    }
    fn str(&mut self) -> Result<String, WireError> {
        let bytes = self.bytes()?;
        String::from_utf8(bytes).map_err(|_| WireError::InvalidUtf8)
    }
    fn at_end(&self) -> bool {
        self.pos == self.buf.len()
    }
}

fn bounded_capacity(n: u64, min_elem_bytes: usize, remaining: usize) -> usize {
    let max_possible = remaining / min_elem_bytes.max(1);
    if n > max_possible as u64 {
        max_possible
    } else {
        n as usize
    }
}

// ─────────────────────── BackendProfile ───────────────────────

fn encode_profile(p: &BackendProfile, w: &mut Writer<'_>) {
    w.str(&p.kind);
    w.u32(p.supported_ops);
    w.u8(p.supported_dtypes);
    w.u32(p.gflops_f32);
    w.u32(p.gflops_u8);
    w.u32(p.gflops_i32);
    w.u32(p.host_to_device_bw);
    w.u32(p.device_to_host_bw);
    w.u32(p.device_internal_bw);
    w.u32(p.launch_overhead_ns);
    w.u32(p.transport_latency_ns);
    w.u32(p.on_device_mib);
    w.u8(p.max_tensor_rank);
    w.u32(p.max_dim);
}

fn decode_profile(r: &mut Reader<'_>) -> Result<BackendProfile, WireError> {
    Ok(BackendProfile {
        kind: r.str()?,
        supported_ops: r.u32()?,
        supported_dtypes: r.u8()?,
        gflops_f32: r.u32()?,
        gflops_u8: r.u32()?,
        gflops_i32: r.u32()?,
        host_to_device_bw: r.u32()?,
        device_to_host_bw: r.u32()?,
        device_internal_bw: r.u32()?,
        launch_overhead_ns: r.u32()?,
        transport_latency_ns: r.u32()?,
        on_device_mib: r.u32()?,
        max_tensor_rank: r.u8()?,
        max_dim: r.u32()?,
    })
}

// ─────────────────────── KernelSource ───────────────────────

fn encode_kernel_source(s: &KernelSource, w: &mut Writer<'_>) {
    w.u8(s.wire_tag());
    match s {
        KernelSource::Msl { code, entry }
        | KernelSource::CudaC { code, entry }
        | KernelSource::Glsl { code, entry }
        | KernelSource::Wgsl { code, entry }
        | KernelSource::OpenClC { code, entry } => {
            w.str(code);
            w.str(entry);
        }
        KernelSource::SpirV { bytes, entry } => {
            w.bytes(bytes);
            w.str(entry);
        }
        KernelSource::Native { backend, blob } => {
            w.str(backend);
            w.bytes(blob);
        }
    }
}

fn decode_kernel_source(r: &mut Reader<'_>) -> Result<KernelSource, WireError> {
    let tag = r.u8()?;
    match tag {
        0x00 => Ok(KernelSource::Msl {
            code: r.str()?,
            entry: r.str()?,
        }),
        0x01 => Ok(KernelSource::CudaC {
            code: r.str()?,
            entry: r.str()?,
        }),
        0x02 => Ok(KernelSource::Glsl {
            code: r.str()?,
            entry: r.str()?,
        }),
        0x03 => Ok(KernelSource::SpirV {
            bytes: r.bytes()?,
            entry: r.str()?,
        }),
        0x04 => Ok(KernelSource::Wgsl {
            code: r.str()?,
            entry: r.str()?,
        }),
        0x05 => Ok(KernelSource::OpenClC {
            code: r.str()?,
            entry: r.str()?,
        }),
        0xFF => Ok(KernelSource::Native {
            backend: r.str()?,
            blob: r.bytes()?,
        }),
        unknown => Err(WireError::UnknownTag {
            what: "KernelSource",
            tag: unknown as u64,
        }),
    }
}

// ─────────────────────── ExecutorRequest ───────────────────────

fn encode_request(req: &ExecutorRequest, w: &mut Writer<'_>) {
    w.u8(req.wire_tag());
    match req {
        ExecutorRequest::Register {
            protocol_version,
            executor_kind,
            profile,
        } => {
            w.u32(*protocol_version);
            w.str(executor_kind);
            encode_profile(profile, w);
        }
        ExecutorRequest::PrepareKernel { kernel_id, source } => {
            w.u64(kernel_id.0);
            encode_kernel_source(source, w);
        }
        ExecutorRequest::AllocBuffer { bytes } => {
            w.u64(*bytes);
        }
        ExecutorRequest::UploadBuffer {
            buffer,
            offset,
            data,
        } => {
            w.u64(buffer.0);
            w.u64(*offset);
            w.bytes(data);
        }
        ExecutorRequest::Dispatch { job_id, graph } => {
            w.u64(*job_id);
            // Embed the compute_ir-encoded graph as bytes — bounded
            // length so the reader can frame it.
            let g_bytes = graph.to_bytes();
            w.bytes(&g_bytes);
        }
        ExecutorRequest::DownloadBuffer {
            buffer,
            offset,
            len,
        } => {
            w.u64(buffer.0);
            w.u64(*offset);
            w.u64(*len);
        }
        ExecutorRequest::FreeBuffer { buffer } => {
            w.u64(buffer.0);
        }
        ExecutorRequest::CancelJob { job_id } => {
            w.u64(*job_id);
        }
        ExecutorRequest::Heartbeat => {}
        ExecutorRequest::Shutdown => {}
        ExecutorRequest::DispatchSpecialised {
            job_id,
            handle,
            inputs,
            outputs,
        } => {
            w.u64(*job_id);
            w.u64(*handle);
            w.u32(inputs.len() as u32);
            for b in inputs {
                w.u64(b.0);
            }
            w.u32(outputs.len() as u32);
            for b in outputs {
                w.u64(b.0);
            }
        }
    }
}

fn decode_request(r: &mut Reader<'_>) -> Result<ExecutorRequest, WireError> {
    let tag = r.u8()?;
    match tag {
        0x00 => Ok(ExecutorRequest::Register {
            protocol_version: r.u32()?,
            executor_kind: r.str()?,
            profile: decode_profile(r)?,
        }),
        0x01 => Ok(ExecutorRequest::PrepareKernel {
            kernel_id: KernelId(r.u64()?),
            source: decode_kernel_source(r)?,
        }),
        0x02 => Ok(ExecutorRequest::AllocBuffer { bytes: r.u64()? }),
        0x03 => Ok(ExecutorRequest::UploadBuffer {
            buffer: BufferId(r.u64()?),
            offset: r.u64()?,
            data: r.bytes()?,
        }),
        0x04 => {
            let job_id = r.u64()?;
            let g_bytes = r.bytes()?;
            let graph = ComputeGraph::from_bytes(&g_bytes)?;
            Ok(ExecutorRequest::Dispatch { job_id, graph })
        }
        0x05 => Ok(ExecutorRequest::DownloadBuffer {
            buffer: BufferId(r.u64()?),
            offset: r.u64()?,
            len: r.u64()?,
        }),
        0x06 => Ok(ExecutorRequest::FreeBuffer {
            buffer: BufferId(r.u64()?),
        }),
        0x07 => Ok(ExecutorRequest::CancelJob { job_id: r.u64()? }),
        0x08 => Ok(ExecutorRequest::Heartbeat),
        0x09 => Ok(ExecutorRequest::Shutdown),
        0x0A => {
            let job_id = r.u64()?;
            let handle = r.u64()?;
            // Each BufferId is 8 wire bytes; bound the pre-allocation
            // against the bytes actually remaining so an attacker cannot
            // request a multi-GB Vec via a maliciously large `n_in`.
            // Same pattern as `OpTiming` decoding above.  If the
            // attacker lies about the count, the loop below will hit
            // the truncated-read path and fail cleanly.
            let n_in = r.u32()? as u64;
            let cap_in = bounded_capacity(n_in, 8, r.remaining());
            let mut inputs = Vec::with_capacity(cap_in);
            for _ in 0..n_in {
                inputs.push(BufferId(r.u64()?));
            }
            let n_out = r.u32()? as u64;
            let cap_out = bounded_capacity(n_out, 8, r.remaining());
            let mut outputs = Vec::with_capacity(cap_out);
            for _ in 0..n_out {
                outputs.push(BufferId(r.u64()?));
            }
            Ok(ExecutorRequest::DispatchSpecialised {
                job_id,
                handle,
                inputs,
                outputs,
            })
        }
        unknown => Err(WireError::UnknownTag {
            what: "ExecutorRequest",
            tag: unknown as u64,
        }),
    }
}

// ─────────────────────── ExecutorResponse ───────────────────────

fn encode_response(resp: &ExecutorResponse, w: &mut Writer<'_>) {
    w.u8(resp.wire_tag());
    match resp {
        ExecutorResponse::Registered { executor_id } => {
            w.u32(executor_id.0);
        }
        ExecutorResponse::KernelReady { kernel_id } => {
            w.u64(kernel_id.0);
        }
        ExecutorResponse::BufferAllocated { buffer }
        | ExecutorResponse::BufferUploaded { buffer } => {
            w.u64(buffer.0);
        }
        ExecutorResponse::DispatchDone { job_id, timings } => {
            w.u64(*job_id);
            w.uv64(timings.len() as u64);
            for t in timings {
                w.u32(t.op_index);
                w.u64(t.ns);
            }
        }
        ExecutorResponse::BufferData { buffer, data } => {
            w.u64(buffer.0);
            w.bytes(data);
        }
        ExecutorResponse::BufferFreed => {}
        ExecutorResponse::Cancelled { job_id } => {
            w.u64(*job_id);
        }
        ExecutorResponse::Alive { profile } => {
            encode_profile(profile, w);
        }
        ExecutorResponse::ShuttingDown => {}
        ExecutorResponse::Error {
            code,
            message,
            job_id,
        } => {
            w.u16(code.0);
            w.str(message);
            // Option<u64>: presence byte then optional payload.
            match job_id {
                Some(id) => {
                    w.u8(1);
                    w.u64(*id);
                }
                None => {
                    w.u8(0);
                }
            }
        }
    }
}

fn decode_response(r: &mut Reader<'_>) -> Result<ExecutorResponse, WireError> {
    let tag = r.u8()?;
    match tag {
        0x00 => Ok(ExecutorResponse::Registered {
            executor_id: ExecutorId(r.u32()?),
        }),
        0x01 => Ok(ExecutorResponse::KernelReady {
            kernel_id: KernelId(r.u64()?),
        }),
        0x02 => Ok(ExecutorResponse::BufferAllocated {
            buffer: BufferId(r.u64()?),
        }),
        0x03 => Ok(ExecutorResponse::BufferUploaded {
            buffer: BufferId(r.u64()?),
        }),
        0x04 => {
            let job_id = r.u64()?;
            let n = r.uv64()?;
            // Each OpTiming is 12 bytes (u32 + u64).
            let cap = bounded_capacity(n, 12, r.remaining());
            let mut timings = Vec::with_capacity(cap);
            for _ in 0..n {
                let op_index = r.u32()?;
                let ns = r.u64()?;
                timings.push(OpTiming { op_index, ns });
            }
            Ok(ExecutorResponse::DispatchDone { job_id, timings })
        }
        0x05 => Ok(ExecutorResponse::BufferData {
            buffer: BufferId(r.u64()?),
            data: r.bytes()?,
        }),
        0x06 => Ok(ExecutorResponse::BufferFreed),
        0x07 => Ok(ExecutorResponse::Cancelled { job_id: r.u64()? }),
        0x08 => Ok(ExecutorResponse::Alive {
            profile: decode_profile(r)?,
        }),
        0x09 => Ok(ExecutorResponse::ShuttingDown),
        0xFE => {
            let code = ErrorCode(r.u16()?);
            let message = r.str()?;
            let presence = r.u8()?;
            let job_id = if presence == 0 {
                None
            } else {
                Some(r.u64()?)
            };
            Ok(ExecutorResponse::Error {
                code,
                message,
                job_id,
            })
        }
        unknown => Err(WireError::UnknownTag {
            what: "ExecutorResponse",
            tag: unknown as u64,
        }),
    }
}

// ─────────────────────── ExecutorEvent ───────────────────────

fn encode_event(ev: &ExecutorEvent, w: &mut Writer<'_>) {
    w.u8(ev.wire_tag());
    match ev {
        ExecutorEvent::BufferLost { buffer, reason } => {
            w.u64(buffer.0);
            w.str(reason);
        }
        ExecutorEvent::ProfileUpdated { profile } => {
            encode_profile(profile, w);
        }
        ExecutorEvent::ShuttingDown => {}
    }
}

fn decode_event(r: &mut Reader<'_>) -> Result<ExecutorEvent, WireError> {
    let tag = r.u8()?;
    match tag {
        0x00 => Ok(ExecutorEvent::BufferLost {
            buffer: BufferId(r.u64()?),
            reason: r.str()?,
        }),
        0x01 => Ok(ExecutorEvent::ProfileUpdated {
            profile: decode_profile(r)?,
        }),
        0x02 => Ok(ExecutorEvent::ShuttingDown),
        unknown => Err(WireError::UnknownTag {
            what: "ExecutorEvent",
            tag: unknown as u64,
        }),
    }
}

// ─────────────────────── frame + public API ───────────────────────

fn encode_frame_header(f: &MessageFrame, w: &mut Writer<'_>) {
    w.u8(f.format_version);
    w.u8(f.kind.wire_byte());
    w.u64(f.correlation_id);
    w.bytes(&f.payload);
}

fn decode_frame_header(r: &mut Reader<'_>) -> Result<MessageFrame, WireError> {
    let format_version = r.u8()?;
    if format_version != FRAME_VERSION {
        return Err(WireError::UnsupportedFrameVersion(format_version));
    }
    let kind_byte = r.u8()?;
    let kind = MessageKind::from_wire_byte(kind_byte)
        .ok_or(WireError::UnknownMessageKind(kind_byte))?;
    let correlation_id = r.u64()?;
    let payload = r.bytes()?;
    Ok(MessageFrame {
        format_version,
        kind,
        correlation_id,
        payload,
    })
}

impl MessageFrame {
    /// Encode this frame to bytes.  For typed message payloads, prefer
    /// the [`request`](Self::request), [`response`](Self::response),
    /// or [`event`](Self::event) constructors which encode the payload
    /// for you.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16 + self.payload.len());
        encode_frame_header(self, &mut Writer::new(&mut out));
        out
    }

    /// Decode bytes into a [`MessageFrame`].  The payload is left as
    /// raw bytes; use [`as_request`](Self::as_request),
    /// [`as_response`](Self::as_response), or [`as_event`](Self::as_event)
    /// to decode the payload further.
    pub fn from_bytes(buf: &[u8]) -> Result<MessageFrame, WireError> {
        let mut r = Reader::new(buf);
        let f = decode_frame_header(&mut r)?;
        if !r.at_end() {
            return Err(WireError::TrailingBytes);
        }
        Ok(f)
    }

    /// Build a request frame.  Encodes `req` into the payload.
    pub fn request(correlation_id: u64, req: &ExecutorRequest) -> MessageFrame {
        let mut payload = Vec::with_capacity(16);
        encode_request(req, &mut Writer::new(&mut payload));
        MessageFrame {
            format_version: FRAME_VERSION,
            kind: MessageKind::Request,
            correlation_id,
            payload,
        }
    }

    /// Build a response frame.  Encodes `resp` into the payload.
    pub fn response(correlation_id: u64, resp: &ExecutorResponse) -> MessageFrame {
        let mut payload = Vec::with_capacity(16);
        encode_response(resp, &mut Writer::new(&mut payload));
        MessageFrame {
            format_version: FRAME_VERSION,
            kind: MessageKind::Response,
            correlation_id,
            payload,
        }
    }

    /// Build an event frame.  Events use `correlation_id` 0.
    pub fn event(ev: &ExecutorEvent) -> MessageFrame {
        let mut payload = Vec::with_capacity(8);
        encode_event(ev, &mut Writer::new(&mut payload));
        MessageFrame {
            format_version: FRAME_VERSION,
            kind: MessageKind::Event,
            correlation_id: 0,
            payload,
        }
    }

    /// Decode this frame's payload as an [`ExecutorRequest`].  Errors
    /// if the frame's `kind` is not [`MessageKind::Request`].
    pub fn as_request(&self) -> Result<ExecutorRequest, WireError> {
        if self.kind != MessageKind::Request {
            return Err(WireError::UnknownMessageKind(self.kind.wire_byte()));
        }
        let mut r = Reader::new(&self.payload);
        let req = decode_request(&mut r)?;
        if !r.at_end() {
            return Err(WireError::TrailingBytes);
        }
        Ok(req)
    }

    /// Decode this frame's payload as an [`ExecutorResponse`].
    pub fn as_response(&self) -> Result<ExecutorResponse, WireError> {
        if self.kind != MessageKind::Response {
            return Err(WireError::UnknownMessageKind(self.kind.wire_byte()));
        }
        let mut r = Reader::new(&self.payload);
        let resp = decode_response(&mut r)?;
        if !r.at_end() {
            return Err(WireError::TrailingBytes);
        }
        Ok(resp)
    }

    /// Decode this frame's payload as an [`ExecutorEvent`].
    pub fn as_event(&self) -> Result<ExecutorEvent, WireError> {
        if self.kind != MessageKind::Event {
            return Err(WireError::UnknownMessageKind(self.kind.wire_byte()));
        }
        let mut r = Reader::new(&self.payload);
        let ev = decode_event(&mut r)?;
        if !r.at_end() {
            return Err(WireError::TrailingBytes);
        }
        Ok(ev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_round_trip() {
        for v in [0u64, 1, 127, 128, u64::MAX] {
            let mut buf = Vec::new();
            Writer::new(&mut buf).uv64(v);
            let mut r = Reader::new(&buf);
            assert_eq!(r.uv64().unwrap(), v);
        }
    }

    #[test]
    fn unsupported_frame_version_rejected() {
        let mut buf = Vec::new();
        Writer::new(&mut buf).u8(99);
        Writer::new(&mut buf).u8(0);
        Writer::new(&mut buf).u64(0);
        Writer::new(&mut buf).bytes(b"");
        assert!(matches!(
            MessageFrame::from_bytes(&buf),
            Err(WireError::UnsupportedFrameVersion(99))
        ));
    }

    #[test]
    fn dispatch_specialised_request_round_trips() {
        let original = ExecutorRequest::DispatchSpecialised {
            job_id: 0xABCD_EF12_3456_7890,
            handle: 0x1234_5678_9ABC_DEF0,
            inputs: vec![BufferId(7), BufferId(8), BufferId(9)],
            outputs: vec![BufferId(10), BufferId(11)],
        };

        let frame = MessageFrame::request(99, &original);
        let bytes = frame.to_bytes();
        let decoded_frame = MessageFrame::from_bytes(&bytes).expect("frame decodes");
        let decoded_req = decoded_frame.as_request().expect("payload decodes");

        assert_eq!(decoded_req, original);
    }

    #[test]
    fn dispatch_specialised_with_empty_buffer_lists_round_trips() {
        let original = ExecutorRequest::DispatchSpecialised {
            job_id: 0,
            handle: 0,
            inputs: vec![],
            outputs: vec![],
        };
        let frame = MessageFrame::request(0, &original);
        let bytes = frame.to_bytes();
        let decoded_frame = MessageFrame::from_bytes(&bytes).expect("frame decodes");
        assert_eq!(decoded_frame.as_request().unwrap(), original);
    }

    /// **Security regression test.**  An attacker-supplied `n_inputs` of
    /// `u32::MAX` would request ~34 GiB if `Vec::with_capacity` were
    /// passed the raw count.  The decoder must bound capacity against
    /// remaining wire bytes (each `BufferId` costs 8 bytes), so
    /// pre-allocation stays small and the truncated-read path catches
    /// the malformed message cleanly without panicking.
    #[test]
    fn dispatch_specialised_oversized_input_count_does_not_oom() {
        let mut payload = Vec::new();
        let mut w = Writer::new(&mut payload);
        // tag 0x0A, job_id, handle
        w.u8(0x0A);
        w.u64(0);
        w.u64(0);
        // attacker-controlled "n_inputs = u32::MAX" with no actual
        // buffer ids following.  A naive decoder would allocate
        // capacity for 4.29B BufferId entries.
        w.u32(u32::MAX);

        let mut frame_bytes = Vec::new();
        let mut fw = Writer::new(&mut frame_bytes);
        fw.u8(1); // format_version
        fw.u8(0); // kind = Request
        fw.u64(0); // correlation_id
        fw.bytes(&payload);

        // Decode must return Err(Truncated) — and crucially, must
        // return *cleanly* without panic or OOM-abort.
        let frame = MessageFrame::from_bytes(&frame_bytes).expect("outer frame ok");
        let res = frame.as_request();
        assert!(res.is_err(), "decoder must reject the malformed message");
    }
}
