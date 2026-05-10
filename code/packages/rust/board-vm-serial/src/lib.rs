use std::io::{Read, Write};
use std::time::Duration;

use board_vm_client::{RawFrameTransport, TransportError};
use board_vm_stream::{StreamTransport, StreamTransportError};
pub use serialport::{
    ClearBuffer, DataBits, FlowControl, Parity, SerialPort, SerialPortInfo, StopBits,
};

pub const DEFAULT_BAUD_RATE: u32 = 115_200;
pub const DEFAULT_TIMEOUT_MS: u64 = 1_000;
pub const ARDUINO_BOOTLOADER_TOUCH_BAUD_RATE: u32 = 1_200;
pub const ARDUINO_BOOTLOADER_TOUCH_TIMEOUT_MS: u64 = 250;
pub const ARDUINO_BOOTLOADER_TOUCH_DTR_HIGH_MS: u64 = 50;
pub const ARDUINO_BOOTLOADER_TOUCH_SETTLE_MS: u64 = 1_500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialConfig {
    pub path: String,
    pub baud_rate: u32,
    pub timeout: Duration,
    pub data_bits: DataBits,
    pub flow_control: FlowControl,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub dtr_on_open: Option<bool>,
    pub clear_on_open: bool,
    pub settle_on_open: Duration,
}

impl SerialConfig {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            baud_rate: DEFAULT_BAUD_RATE,
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            dtr_on_open: None,
            clear_on_open: false,
            settle_on_open: Duration::ZERO,
        }
    }

    pub fn baud_rate(mut self, baud_rate: u32) -> Self {
        self.baud_rate = baud_rate;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn data_bits(mut self, data_bits: DataBits) -> Self {
        self.data_bits = data_bits;
        self
    }

    pub fn flow_control(mut self, flow_control: FlowControl) -> Self {
        self.flow_control = flow_control;
        self
    }

    pub fn parity(mut self, parity: Parity) -> Self {
        self.parity = parity;
        self
    }

    pub fn stop_bits(mut self, stop_bits: StopBits) -> Self {
        self.stop_bits = stop_bits;
        self
    }

    pub fn dtr_on_open(mut self, dtr_on_open: bool) -> Self {
        self.dtr_on_open = Some(dtr_on_open);
        self
    }

    pub fn preserve_dtr_on_open(mut self) -> Self {
        self.dtr_on_open = None;
        self
    }

    pub fn clear_on_open(mut self, clear_on_open: bool) -> Self {
        self.clear_on_open = clear_on_open;
        self
    }

    pub fn settle_on_open(mut self, settle_on_open: Duration) -> Self {
        self.settle_on_open = settle_on_open;
        self
    }
}

#[derive(Debug)]
pub enum SerialTransportError {
    Open(serialport::Error),
    Configure(serialport::Error),
    Stream(StreamTransportError),
}

impl From<StreamTransportError> for SerialTransportError {
    fn from(value: StreamTransportError) -> Self {
        Self::Stream(value)
    }
}

pub struct BoardSerialTransport<S, const WIRE_BYTES: usize = 1024> {
    inner: StreamTransport<S, WIRE_BYTES>,
}

impl<S, const WIRE_BYTES: usize> BoardSerialTransport<S, WIRE_BYTES> {
    pub fn from_stream(stream: S) -> Self {
        Self {
            inner: StreamTransport::new(stream),
        }
    }

    pub fn into_inner(self) -> StreamTransport<S, WIRE_BYTES> {
        self.inner
    }

    pub fn stream_transport(&self) -> &StreamTransport<S, WIRE_BYTES> {
        &self.inner
    }

    pub fn stream_transport_mut(&mut self) -> &mut StreamTransport<S, WIRE_BYTES> {
        &mut self.inner
    }

    pub fn send_raw_frame(&mut self, raw_frame: &[u8]) -> Result<usize, SerialTransportError>
    where
        S: Write,
    {
        Ok(self.inner.send_raw_frame(raw_frame)?)
    }

    pub fn receive_raw_frame(&mut self, raw_out: &mut [u8]) -> Result<usize, SerialTransportError>
    where
        S: Read,
    {
        Ok(self.inner.receive_raw_frame(raw_out)?)
    }

    pub fn exchange_raw_frame_checked(
        &mut self,
        raw_request: &[u8],
        raw_response_out: &mut [u8],
    ) -> Result<usize, SerialTransportError>
    where
        S: Read + Write,
    {
        Ok(self
            .inner
            .exchange_raw_frame_checked(raw_request, raw_response_out)?)
    }
}

impl<const WIRE_BYTES: usize> BoardSerialTransport<Box<dyn SerialPort>, WIRE_BYTES> {
    pub fn open(config: &SerialConfig) -> Result<Self, SerialTransportError> {
        let mut builder = serialport::new(&config.path, config.baud_rate)
            .timeout(config.timeout)
            .data_bits(config.data_bits)
            .flow_control(config.flow_control)
            .parity(config.parity)
            .stop_bits(config.stop_bits);

        if let Some(dtr_on_open) = config.dtr_on_open {
            builder = builder.dtr_on_open(dtr_on_open);
        }

        let port = builder.open().map_err(SerialTransportError::Open)?;
        if config.clear_on_open {
            port.clear(ClearBuffer::All)
                .map_err(SerialTransportError::Configure)?;
        }
        if !config.settle_on_open.is_zero() {
            std::thread::sleep(config.settle_on_open);
            if config.clear_on_open {
                port.clear(ClearBuffer::All)
                    .map_err(SerialTransportError::Configure)?;
            }
        }
        Ok(Self::from_stream(port))
    }
}

impl<S, const WIRE_BYTES: usize> RawFrameTransport for BoardSerialTransport<S, WIRE_BYTES>
where
    S: Read + Write,
{
    fn exchange_raw_frame(
        &mut self,
        request: &[u8],
        response_out: &mut [u8],
    ) -> Result<usize, TransportError> {
        self.inner.exchange_raw_frame(request, response_out)
    }
}

pub fn available_ports() -> Result<Vec<SerialPortInfo>, serialport::Error> {
    serialport::available_ports()
}

pub fn touch_arduino_bootloader(path: &str) -> Result<(), serialport::Error> {
    touch_arduino_bootloader_with_timing(
        path,
        Duration::from_millis(ARDUINO_BOOTLOADER_TOUCH_TIMEOUT_MS),
        Duration::from_millis(ARDUINO_BOOTLOADER_TOUCH_SETTLE_MS),
    )
}

pub fn touch_arduino_bootloader_with_timing(
    path: &str,
    timeout: Duration,
    settle: Duration,
) -> Result<(), serialport::Error> {
    let mut port = serialport::new(path, ARDUINO_BOOTLOADER_TOUCH_BAUD_RATE)
        .timeout(timeout)
        .dtr_on_open(true)
        .open()?;
    port.write_data_terminal_ready(true)?;
    std::thread::sleep(Duration::from_millis(ARDUINO_BOOTLOADER_TOUCH_DTR_HIGH_MS));
    port.write_data_terminal_ready(false)?;
    drop(port);

    if !settle.is_zero() {
        std::thread::sleep(settle);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io;

    use board_vm_protocol::{
        decode_wire_frame, encode_frame, encode_ping, encode_pong, encode_wire_frame, Frame,
        MessageType, Ping, Pong, FLAG_IS_RESPONSE, FLAG_RESPONSE_REQUIRED,
    };

    #[derive(Default)]
    struct ScriptedSerialStream {
        read: VecDeque<u8>,
        written: Vec<u8>,
    }

    impl ScriptedSerialStream {
        fn with_read(bytes: &[u8]) -> Self {
            Self {
                read: bytes.iter().copied().collect(),
                written: Vec::new(),
            }
        }
    }

    impl Read for ScriptedSerialStream {
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

    impl Write for ScriptedSerialStream {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
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

    fn wire_frame(raw: &[u8]) -> Vec<u8> {
        let mut wire = [0; 96];
        let len = encode_wire_frame(raw, &mut wire).unwrap();
        wire[..len].to_vec()
    }

    #[test]
    fn default_config_matches_usb_serial_smoke_test_defaults() {
        let config = SerialConfig::new("/dev/ttyACM0");

        assert_eq!(config.path, "/dev/ttyACM0");
        assert_eq!(config.baud_rate, 115_200);
        assert_eq!(config.timeout, Duration::from_millis(1_000));
        assert_eq!(config.data_bits, DataBits::Eight);
        assert_eq!(config.flow_control, FlowControl::None);
        assert_eq!(config.parity, Parity::None);
        assert_eq!(config.stop_bits, StopBits::One);
        assert_eq!(config.dtr_on_open, None);
        assert!(!config.clear_on_open);
        assert_eq!(config.settle_on_open, Duration::ZERO);
    }

    #[test]
    fn config_builder_sets_serial_options() {
        let config = SerialConfig::new("COM7")
            .baud_rate(230_400)
            .timeout(Duration::from_millis(250))
            .data_bits(DataBits::Seven)
            .flow_control(FlowControl::Software)
            .parity(Parity::Even)
            .stop_bits(StopBits::Two)
            .dtr_on_open(true)
            .clear_on_open(true)
            .settle_on_open(Duration::from_millis(50));

        assert_eq!(config.path, "COM7");
        assert_eq!(config.baud_rate, 230_400);
        assert_eq!(config.timeout, Duration::from_millis(250));
        assert_eq!(config.data_bits, DataBits::Seven);
        assert_eq!(config.flow_control, FlowControl::Software);
        assert_eq!(config.parity, Parity::Even);
        assert_eq!(config.stop_bits, StopBits::Two);
        assert_eq!(config.dtr_on_open, Some(true));
        assert!(config.clear_on_open);
        assert_eq!(config.settle_on_open, Duration::from_millis(50));
    }

    #[test]
    fn config_builder_can_preserve_dtr_after_override() {
        let config = SerialConfig::new("COM7")
            .dtr_on_open(true)
            .preserve_dtr_on_open();

        assert_eq!(config.dtr_on_open, None);
    }

    #[test]
    fn arduino_bootloader_touch_uses_the_reset_baud_rate() {
        assert_eq!(ARDUINO_BOOTLOADER_TOUCH_BAUD_RATE, 1_200);
        assert_eq!(ARDUINO_BOOTLOADER_TOUCH_TIMEOUT_MS, 250);
        assert_eq!(ARDUINO_BOOTLOADER_TOUCH_DTR_HIGH_MS, 50);
        assert_eq!(ARDUINO_BOOTLOADER_TOUCH_SETTLE_MS, 1_500);
    }

    #[test]
    fn exchanges_raw_frames_over_serial_byte_stream() {
        let request = raw_ping(9, 0xDEAD_BEEF);
        let response = raw_pong(9, 0xDEAD_BEEF);
        let stream = ScriptedSerialStream::with_read(&wire_frame(&response));
        let mut transport: BoardSerialTransport<_, 96> = BoardSerialTransport::from_stream(stream);
        let mut response_out = [0; 64];

        let response_len = transport
            .exchange_raw_frame_checked(&request, &mut response_out)
            .unwrap();
        assert_eq!(&response_out[..response_len], &response);

        let stream = transport.into_inner().into_inner();
        let mut decoded_request = [0; 64];
        let request_len = decode_wire_frame(&stream.written, &mut decoded_request).unwrap();
        assert_eq!(&decoded_request[..request_len], &request);
    }
}
