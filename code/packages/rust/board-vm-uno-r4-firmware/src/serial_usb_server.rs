use board_vm_device::{BoardVmDevice, DeviceStreamEndpoint, DeviceStreamError};
use board_vm_runtime::BoardHal;
use board_vm_usb_cdc::{BlockingUsbCdc, UsbCdcByteStream};

pub const SERIAL_USB_WIRE_BYTES: usize = 1024;
pub const SERIAL_USB_RAW_BYTES: usize = 512;
pub const SERIAL_USB_PAYLOAD_BYTES: usize = 256;

pub type SerialUsbEndpoint<C> = DeviceStreamEndpoint<
    UsbCdcByteStream<C>,
    SERIAL_USB_WIRE_BYTES,
    SERIAL_USB_RAW_BYTES,
    SERIAL_USB_PAYLOAD_BYTES,
>;

pub fn serial_usb_endpoint<C>(cdc: C) -> SerialUsbEndpoint<C>
where
    C: BlockingUsbCdc,
{
    DeviceStreamEndpoint::new(UsbCdcByteStream::new(cdc))
}

pub fn serve_serial_usb_once<
    'a,
    C,
    H,
    const MAX_PROGRAM_BYTES: usize,
    const MAX_STACK: usize,
    const MAX_HANDLES: usize,
>(
    endpoint: &mut SerialUsbEndpoint<C>,
    device: &mut BoardVmDevice<'a, H, MAX_PROGRAM_BYTES, MAX_STACK, MAX_HANDLES>,
) -> Result<usize, DeviceStreamError<C::Error>>
where
    C: BlockingUsbCdc,
    H: BoardHal,
{
    endpoint.serve_one(device)
}

#[cfg(test)]
mod tests {
    use super::*;
    use board_vm_host::HostSession;
    use board_vm_protocol::{decode_frame, decode_hello_ack, encode_wire_frame, MessageType};
    use board_vm_runtime::{GpioMode, HalError, Level};
    use board_vm_uno_r4::{minima_device, UnoR4Backend};
    use std::vec::Vec;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FakeCdcError {
        EndOfInput,
    }

    struct FakeCdc {
        read: Vec<u8>,
        read_offset: usize,
        written: Vec<u8>,
    }

    impl FakeCdc {
        fn new(read: Vec<u8>) -> Self {
            Self {
                read,
                read_offset: 0,
                written: Vec::new(),
            }
        }
    }

    impl BlockingUsbCdc for FakeCdc {
        type Error = FakeCdcError;

        fn read_byte(&mut self) -> Result<u8, Self::Error> {
            if self.read_offset >= self.read.len() {
                return Err(FakeCdcError::EndOfInput);
            }
            let byte = self.read[self.read_offset];
            self.read_offset += 1;
            Ok(byte)
        }

        fn write_packet(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
            self.written.extend_from_slice(bytes);
            Ok(())
        }
    }

    struct FakeBackend;

    impl UnoR4Backend for FakeBackend {
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

    #[test]
    fn serial_usb_endpoint_serves_a_board_vm_hello_frame() {
        let mut session = HostSession::new();
        let mut host_payload = [0u8; 128];
        let mut raw_request = [0u8; 256];
        let mut wire_request = [0u8; 512];
        let hello = session
            .hello_frame(
                "serialusb-host",
                0xCAFE_BABE,
                &mut host_payload,
                &mut raw_request,
            )
            .unwrap();
        let wire_len = encode_wire_frame(&raw_request[..hello.len], &mut wire_request).unwrap();

        let cdc = FakeCdc::new(wire_request[..wire_len].to_vec());
        let mut endpoint = serial_usb_endpoint(cdc);
        let mut device = minima_device(FakeBackend, 0xB04D_1005);

        let response_len = serve_serial_usb_once(&mut endpoint, &mut device).unwrap();
        let cdc = endpoint.into_inner().into_inner();
        assert_eq!(response_len, cdc.written.len());

        let mut raw_response = [0u8; 512];
        let raw_len =
            board_vm_protocol::decode_wire_frame(&cdc.written, &mut raw_response).unwrap();
        let frame = decode_frame(&raw_response[..raw_len]).unwrap();
        assert_eq!(frame.message_type, MessageType::HELLO_ACK);
        let ack = decode_hello_ack(frame.payload).unwrap();
        assert_eq!(ack.host_nonce, 0xCAFE_BABE);
        assert_eq!(ack.board_name, "arduino-uno-r4-minima");
        assert_eq!(ack.runtime_name, "board-vm-uno-r4");
    }
}
