#![no_std]

use board_vm_device::{
    DeviceByteStream, DeviceStreamEndpoint, DeviceStreamError, DeviceStreamPoll,
};
use board_vm_esp32::{devkit_v1_device, Esp32Backend, Esp32Device};
use board_vm_runtime::BoardHal;

pub const ESP32_DEFAULT_BOARD_NONCE: u32 = 0xE532_0001;

pub type Esp32ServerEndpoint<S> = DeviceStreamEndpoint<S, 1024, 512, 256>;

pub fn devkit_v1_server<B>(backend: B, board_nonce: u32) -> Esp32Device<B>
where
    B: Esp32Backend,
{
    devkit_v1_device(backend, board_nonce)
}

pub fn esp32_endpoint<S>(stream: S) -> Esp32ServerEndpoint<S> {
    DeviceStreamEndpoint::new(stream)
}

pub fn serve_esp32_once<S, B>(
    endpoint: &mut Esp32ServerEndpoint<S>,
    device: &mut Esp32Device<B>,
) -> Result<usize, DeviceStreamError<S::Error>>
where
    S: DeviceByteStream,
    B: Esp32Backend,
    board_vm_esp32::Esp32Board<B>: BoardHal,
{
    endpoint.serve_one(device)
}

pub fn serve_esp32_available<S, B>(
    endpoint: &mut Esp32ServerEndpoint<S>,
    device: &mut Esp32Device<B>,
) -> Result<DeviceStreamPoll, DeviceStreamError<S::Error>>
where
    S: DeviceByteStream,
    B: Esp32Backend,
    board_vm_esp32::Esp32Board<B>: BoardHal,
{
    endpoint.serve_available(device)
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_device::DeviceByteStream;
    use board_vm_esp32::Esp32Backend;
    use board_vm_protocol::{
        decode_frame, decode_hello_ack, decode_wire_frame, encode_frame, encode_hello,
        encode_wire_frame, Frame, Hello, MessageType, FLAG_RESPONSE_REQUIRED,
    };
    use board_vm_runtime::{GpioMode, HalError, Level};
    use std::vec::Vec;

    #[derive(Default)]
    struct FakeBackend;

    impl Esp32Backend for FakeBackend {
        fn configure_gpio(&mut self, _pin: u8, _mode: GpioMode) -> Result<(), HalError> {
            Ok(())
        }

        fn write_gpio(&mut self, _pin: u8, _level: Level) -> Result<(), HalError> {
            Ok(())
        }

        fn read_gpio(&mut self, _pin: u8) -> Result<Level, HalError> {
            Ok(Level::Low)
        }

        fn sleep_ms(&mut self, _duration_ms: u16) -> Result<(), HalError> {
            Ok(())
        }

        fn now_ms(&self) -> u32 {
            0
        }
    }

    #[derive(Default)]
    struct ScriptedStream {
        read: Vec<u8>,
        read_index: usize,
        written: Vec<u8>,
    }

    impl ScriptedStream {
        fn with_read(bytes: &[u8]) -> Self {
            Self {
                read: bytes.to_vec(),
                read_index: 0,
                written: Vec::new(),
            }
        }
    }

    impl DeviceByteStream for ScriptedStream {
        type Error = ();

        fn read_byte(&mut self) -> Result<u8, Self::Error> {
            let byte = self.read[self.read_index];
            self.read_index += 1;
            Ok(byte)
        }

        fn try_read_byte(&mut self) -> Result<Option<u8>, Self::Error> {
            if self.read_index >= self.read.len() {
                Ok(None)
            } else {
                self.read_byte().map(Some)
            }
        }

        fn write_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
            self.written.extend_from_slice(bytes);
            Ok(())
        }
    }

    #[test]
    fn esp32_endpoint_serves_hello_ack() {
        let request = hello_wire_request();
        let mut endpoint = esp32_endpoint(ScriptedStream::with_read(&request));
        let mut device = devkit_v1_server(FakeBackend, ESP32_DEFAULT_BOARD_NONCE);

        let written_len = serve_esp32_once(&mut endpoint, &mut device).unwrap();
        let stream = endpoint.into_inner();
        assert_eq!(written_len, stream.written.len());

        let mut raw = [0u8; 256];
        let raw_len = decode_wire_frame(&stream.written, &mut raw).unwrap();
        let frame = decode_frame(&raw[..raw_len]).unwrap();
        let hello = decode_hello_ack(frame.payload).unwrap();

        assert_eq!(frame.message_type, MessageType::HELLO_ACK);
        assert_eq!(hello.board_name, "esp32-devkit-v1");
        assert_eq!(hello.runtime_name, "board-vm-esp32");
        assert_eq!(hello.board_nonce, ESP32_DEFAULT_BOARD_NONCE);
    }

    #[test]
    fn esp32_endpoint_reports_idle_when_no_bytes_are_available() {
        let mut endpoint = esp32_endpoint(ScriptedStream::default());
        let mut device = devkit_v1_server(FakeBackend, ESP32_DEFAULT_BOARD_NONCE);

        assert_eq!(
            serve_esp32_available(&mut endpoint, &mut device),
            Ok(DeviceStreamPoll::Idle)
        );
    }

    fn hello_wire_request() -> Vec<u8> {
        let mut payload = [0u8; 64];
        let payload_len = encode_hello(
            &Hello {
                min_version: 1,
                max_version: 1,
                host_name: "firmware-test",
                host_nonce: 0xABCD_1234,
            },
            &mut payload,
        )
        .unwrap();
        let mut raw = [0u8; 96];
        let raw_len = encode_frame(
            &Frame {
                flags: FLAG_RESPONSE_REQUIRED,
                message_type: MessageType::HELLO,
                request_id: 7,
                payload: &payload[..payload_len],
            },
            &mut raw,
        )
        .unwrap();
        let mut wire = [0u8; 128];
        let wire_len = encode_wire_frame(&raw[..raw_len], &mut wire).unwrap();
        wire[..wire_len].to_vec()
    }
}
