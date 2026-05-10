use std::io::{Read, Write};

use board_vm_client::{RawFrameTransport, TransportError};
use board_vm_protocol::{decode_wire_frame, encode_wire_frame, ProtocolError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamTransportError {
    Io,
    FrameTooLarge,
    Protocol(ProtocolError),
}

impl StreamTransportError {
    pub const fn as_transport_error(self) -> TransportError {
        match self {
            Self::Io => TransportError::Io,
            Self::FrameTooLarge => TransportError::ResponseTooLarge,
            Self::Protocol(_) => TransportError::Io,
        }
    }
}

pub struct StreamTransport<S, const WIRE_BYTES: usize = 1024> {
    stream: S,
    wire: [u8; WIRE_BYTES],
}

impl<S, const WIRE_BYTES: usize> StreamTransport<S, WIRE_BYTES> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            wire: [0; WIRE_BYTES],
        }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }

    pub fn inner(&self) -> &S {
        &self.stream
    }

    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    pub fn send_raw_frame(&mut self, raw_frame: &[u8]) -> Result<usize, StreamTransportError>
    where
        S: Write,
    {
        let wire_len = encode_wire_frame(raw_frame, &mut self.wire).map_err(map_protocol_error)?;
        self.stream
            .write_all(&self.wire[..wire_len])
            .map_err(|_| StreamTransportError::Io)?;
        self.stream.flush().map_err(|_| StreamTransportError::Io)?;
        Ok(wire_len)
    }

    pub fn receive_raw_frame(&mut self, raw_out: &mut [u8]) -> Result<usize, StreamTransportError>
    where
        S: Read,
    {
        let wire_len = self.read_wire_frame()?;
        decode_wire_frame(&self.wire[..wire_len], raw_out).map_err(map_protocol_error)
    }

    pub fn exchange_raw_frame_checked(
        &mut self,
        raw_request: &[u8],
        raw_response_out: &mut [u8],
    ) -> Result<usize, StreamTransportError>
    where
        S: Read + Write,
    {
        self.send_raw_frame(raw_request)?;
        self.receive_raw_frame(raw_response_out)
    }

    fn read_wire_frame(&mut self) -> Result<usize, StreamTransportError>
    where
        S: Read,
    {
        let mut len = 0;
        loop {
            if len >= self.wire.len() {
                return Err(StreamTransportError::FrameTooLarge);
            }

            let mut byte = [0u8; 1];
            self.stream
                .read_exact(&mut byte)
                .map_err(|_| StreamTransportError::Io)?;
            self.wire[len] = byte[0];
            len += 1;

            if byte[0] == 0 {
                return Ok(len);
            }
        }
    }
}

impl<S, const WIRE_BYTES: usize> RawFrameTransport for StreamTransport<S, WIRE_BYTES>
where
    S: Read + Write,
{
    fn exchange_raw_frame(
        &mut self,
        request: &[u8],
        response_out: &mut [u8],
    ) -> Result<usize, TransportError> {
        self.exchange_raw_frame_checked(request, response_out)
            .map_err(StreamTransportError::as_transport_error)
    }
}

fn map_protocol_error(error: ProtocolError) -> StreamTransportError {
    match error {
        ProtocolError::OutputTooSmall | ProtocolError::PayloadTooLarge => {
            StreamTransportError::FrameTooLarge
        }
        other => StreamTransportError::Protocol(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io;

    use board_vm_client::BoardVmClient;
    use board_vm_loopback::LoopbackBoard;
    use board_vm_protocol::{
        decode_wire_frame, encode_frame, encode_ping, encode_pong, encode_wire_frame, Frame,
        MessageType, Ping, Pong, RunStatus, FLAG_IS_RESPONSE, FLAG_RESPONSE_REQUIRED,
    };

    #[derive(Default)]
    struct ScriptedStream {
        read: VecDeque<u8>,
        written: Vec<u8>,
    }

    impl ScriptedStream {
        fn with_read(bytes: &[u8]) -> Self {
            Self {
                read: bytes.iter().copied().collect(),
                written: Vec::new(),
            }
        }
    }

    impl Read for ScriptedStream {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if self.read.is_empty() {
                return Ok(0);
            }
            let len = out.len().min(2).min(self.read.len());
            for slot in out.iter_mut().take(len) {
                *slot = self.read.pop_front().unwrap();
            }
            Ok(len)
        }
    }

    impl Write for ScriptedStream {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct LoopbackStream {
        board: LoopbackBoard<256, 8, 8>,
        pending_request_wire: Vec<u8>,
        read: VecDeque<u8>,
        request_raw: [u8; 512],
        response_payload: [u8; 256],
        response_raw: [u8; 512],
        response_wire: [u8; 768],
    }

    impl LoopbackStream {
        fn new() -> Self {
            Self {
                board: LoopbackBoard::new(),
                pending_request_wire: Vec::new(),
                read: VecDeque::new(),
                request_raw: [0; 512],
                response_payload: [0; 256],
                response_raw: [0; 512],
                response_wire: [0; 768],
            }
        }

        fn complete_request(&mut self) -> io::Result<()> {
            let request_len = decode_wire_frame(&self.pending_request_wire, &mut self.request_raw)
                .map_err(to_invalid_data)?;
            let response_len = self
                .board
                .handle_raw_frame(
                    &self.request_raw[..request_len],
                    &mut self.response_payload,
                    &mut self.response_raw,
                )
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
            let wire_len =
                encode_wire_frame(&self.response_raw[..response_len], &mut self.response_wire)
                    .map_err(to_invalid_data)?;
            self.read.extend(&self.response_wire[..wire_len]);
            self.pending_request_wire.clear();
            Ok(())
        }
    }

    impl Read for LoopbackStream {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if self.read.is_empty() {
                return Ok(0);
            }
            let len = out.len().min(3).min(self.read.len());
            for slot in out.iter_mut().take(len) {
                *slot = self.read.pop_front().unwrap();
            }
            Ok(len)
        }
    }

    impl Write for LoopbackStream {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            for byte in bytes {
                self.pending_request_wire.push(*byte);
                if *byte == 0 {
                    self.complete_request()?;
                }
            }
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn to_invalid_data(error: ProtocolError) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, format!("{error:?}"))
    }

    fn raw_ping(request_id: u16, nonce: u32) -> Vec<u8> {
        let mut payload = [0; 8];
        let payload_len = encode_ping(&Ping { nonce }, &mut payload).unwrap();
        raw_frame(
            FLAG_RESPONSE_REQUIRED,
            MessageType::PING,
            request_id,
            &payload[..payload_len],
        )
    }

    fn raw_pong(request_id: u16, nonce: u32) -> Vec<u8> {
        let mut payload = [0; 8];
        let payload_len = encode_pong(&Pong { nonce }, &mut payload).unwrap();
        raw_frame(
            FLAG_IS_RESPONSE,
            MessageType::PONG,
            request_id,
            &payload[..payload_len],
        )
    }

    fn raw_frame(flags: u8, message_type: MessageType, request_id: u16, payload: &[u8]) -> Vec<u8> {
        let mut raw = [0; 64];
        let len = encode_frame(
            &Frame {
                flags,
                message_type,
                request_id,
                payload,
            },
            &mut raw,
        )
        .unwrap();
        raw[..len].to_vec()
    }

    fn wire_frame(raw: &[u8]) -> Vec<u8> {
        let mut wire = [0; 96];
        let len = encode_wire_frame(raw, &mut wire).unwrap();
        wire[..len].to_vec()
    }

    #[test]
    fn exchanges_raw_frames_over_cobs_terminated_stream() {
        let request = raw_ping(7, 0xCAFE_BABE);
        let response = raw_pong(7, 0xCAFE_BABE);
        let response_wire = wire_frame(&response);
        let stream = ScriptedStream::with_read(&response_wire);
        let mut transport: StreamTransport<_, 96> = StreamTransport::new(stream);
        let mut response_out = [0; 64];

        let response_len = transport
            .exchange_raw_frame_checked(&request, &mut response_out)
            .unwrap();
        assert_eq!(&response_out[..response_len], &response);

        let stream = transport.into_inner();
        let mut decoded_request = [0; 64];
        let request_len = decode_wire_frame(&stream.written, &mut decoded_request).unwrap();
        assert_eq!(&decoded_request[..request_len], &request);
    }

    #[test]
    fn reports_oversized_incoming_wire_frame() {
        let stream = ScriptedStream::with_read(&[1, 1, 1, 1, 1]);
        let mut transport: StreamTransport<_, 4> = StreamTransport::new(stream);
        let mut raw_out = [0; 16];

        assert_eq!(
            transport.receive_raw_frame(&mut raw_out),
            Err(StreamTransportError::FrameTooLarge)
        );
    }

    #[test]
    fn board_client_can_drive_loopback_board_through_stream_transport() {
        let stream = LoopbackStream::new();
        let transport: StreamTransport<_, 1024> = StreamTransport::new(stream);
        let mut client: BoardVmClient<_, 256, 512, 512> = BoardVmClient::new(transport);

        let hello = client.hello(0x1234_5678).unwrap();
        assert_eq!(hello.host_nonce, 0x1234_5678);
        let descriptor = client.query_caps().unwrap();
        assert_eq!(descriptor.board_id, "loopback-uno-r4");

        let run = client.blink_onboard_led(1, 100).unwrap();
        assert_eq!(run.status, RunStatus::Running);
        assert_eq!(run.open_handles, 1);

        let stream = client.into_transport().into_inner();
        assert!(!stream.board.fake_hal().events().is_empty());
    }
}
