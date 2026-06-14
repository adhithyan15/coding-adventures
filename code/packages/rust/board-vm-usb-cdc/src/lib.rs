#![no_std]

use board_vm_device::DeviceByteStream;

pub const USB_CDC_FULL_SPEED_MAX_PACKET_BYTES: usize = 64;
pub const DEFAULT_USB_CDC_BAUD_RATE: u32 = 115_200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbCdcStopBits {
    One,
    OnePointFive,
    Two,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbCdcParity {
    None,
    Odd,
    Even,
    Mark,
    Space,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbCdcLineCoding {
    pub baud_rate: u32,
    pub stop_bits: UsbCdcStopBits,
    pub parity: UsbCdcParity,
    pub data_bits: u8,
}

impl UsbCdcLineCoding {
    pub const fn new(baud_rate: u32) -> Self {
        Self {
            baud_rate,
            stop_bits: UsbCdcStopBits::One,
            parity: UsbCdcParity::None,
            data_bits: 8,
        }
    }

    pub const fn stop_bits(mut self, stop_bits: UsbCdcStopBits) -> Self {
        self.stop_bits = stop_bits;
        self
    }

    pub const fn parity(mut self, parity: UsbCdcParity) -> Self {
        self.parity = parity;
        self
    }

    pub const fn data_bits(mut self, data_bits: u8) -> Self {
        self.data_bits = data_bits;
        self
    }

    pub const fn is_8n1(self) -> bool {
        matches!(self.stop_bits, UsbCdcStopBits::One)
            && matches!(self.parity, UsbCdcParity::None)
            && self.data_bits == 8
    }
}

impl Default for UsbCdcLineCoding {
    fn default() -> Self {
        Self::new(DEFAULT_USB_CDC_BAUD_RATE)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbCdcControlLineState {
    pub dtr: bool,
    pub rts: bool,
}

impl UsbCdcControlLineState {
    pub const fn new(dtr: bool, rts: bool) -> Self {
        Self { dtr, rts }
    }

    pub const fn disconnected() -> Self {
        Self {
            dtr: false,
            rts: false,
        }
    }

    pub const fn connected(self) -> bool {
        self.dtr
    }
}

impl Default for UsbCdcControlLineState {
    fn default() -> Self {
        Self::disconnected()
    }
}

pub trait BlockingUsbCdc {
    type Error;

    fn read_byte(&mut self) -> Result<u8, Self::Error>;

    fn try_read_byte(&mut self) -> Result<Option<u8>, Self::Error> {
        self.read_byte().map(Some)
    }

    fn write_packet(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn line_coding(&self) -> UsbCdcLineCoding {
        UsbCdcLineCoding::default()
    }

    fn control_line_state(&self) -> UsbCdcControlLineState {
        UsbCdcControlLineState::default()
    }

    fn connected(&self) -> bool {
        self.control_line_state().connected()
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        for chunk in bytes.chunks(USB_CDC_FULL_SPEED_MAX_PACKET_BYTES) {
            self.write_packet(chunk)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbCdcByteStream<C> {
    cdc: C,
}

impl<C> UsbCdcByteStream<C> {
    pub const fn new(cdc: C) -> Self {
        Self { cdc }
    }

    pub fn into_inner(self) -> C {
        self.cdc
    }

    pub fn cdc(&self) -> &C {
        &self.cdc
    }

    pub fn cdc_mut(&mut self) -> &mut C {
        &mut self.cdc
    }
}

impl<C> DeviceByteStream for UsbCdcByteStream<C>
where
    C: BlockingUsbCdc,
{
    type Error = C::Error;

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        self.cdc.read_byte()
    }

    fn try_read_byte(&mut self) -> Result<Option<u8>, Self::Error> {
        self.cdc.try_read_byte()
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.cdc.write_all(bytes)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.cdc.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FakeError {
        EndOfInput,
        OutputFull,
    }

    struct FakeCdc {
        read: [u8; 4],
        read_len: usize,
        read_offset: usize,
        written: [u8; 96],
        written_len: usize,
        packets: usize,
        flushes: usize,
        state: UsbCdcControlLineState,
        line_coding: UsbCdcLineCoding,
    }

    impl FakeCdc {
        fn with_read(bytes: &[u8]) -> Self {
            let mut read = [0; 4];
            read[..bytes.len()].copy_from_slice(bytes);
            Self {
                read,
                read_len: bytes.len(),
                read_offset: 0,
                written: [0; 96],
                written_len: 0,
                packets: 0,
                flushes: 0,
                state: UsbCdcControlLineState::new(true, false),
                line_coding: UsbCdcLineCoding::default(),
            }
        }
    }

    impl BlockingUsbCdc for FakeCdc {
        type Error = FakeError;

        fn read_byte(&mut self) -> Result<u8, Self::Error> {
            if self.read_offset >= self.read_len {
                return Err(FakeError::EndOfInput);
            }
            let byte = self.read[self.read_offset];
            self.read_offset += 1;
            Ok(byte)
        }

        fn try_read_byte(&mut self) -> Result<Option<u8>, Self::Error> {
            if self.read_offset >= self.read_len {
                return Ok(None);
            }
            self.read_byte().map(Some)
        }

        fn write_packet(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
            self.packets += 1;
            let end = self
                .written_len
                .checked_add(bytes.len())
                .ok_or(FakeError::OutputFull)?;
            if end > self.written.len() {
                return Err(FakeError::OutputFull);
            }
            self.written[self.written_len..end].copy_from_slice(bytes);
            self.written_len = end;
            Ok(())
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            self.flushes += 1;
            Ok(())
        }

        fn line_coding(&self) -> UsbCdcLineCoding {
            self.line_coding
        }

        fn control_line_state(&self) -> UsbCdcControlLineState {
            self.state
        }
    }

    #[test]
    fn default_line_coding_matches_serial_smoke_defaults() {
        let line_coding = UsbCdcLineCoding::default();

        assert_eq!(line_coding, UsbCdcLineCoding::new(115_200));
        assert!(line_coding.is_8n1());
        assert!(!UsbCdcLineCoding::new(115_200).data_bits(7).is_8n1());
        assert!(!UsbCdcLineCoding::new(115_200)
            .parity(UsbCdcParity::Even)
            .is_8n1());
        assert!(!UsbCdcLineCoding::new(115_200)
            .stop_bits(UsbCdcStopBits::Two)
            .is_8n1());
    }

    #[test]
    fn usb_cdc_byte_stream_delegates_to_cdc_backend() {
        let mut stream = UsbCdcByteStream::new(FakeCdc::with_read(&[0x11, 0x22]));

        assert_eq!(stream.read_byte().unwrap(), 0x11);
        stream.write_all(&[0xA0, 0xB1]).unwrap();
        stream.flush().unwrap();

        let cdc = stream.into_inner();
        assert_eq!(&cdc.written[..cdc.written_len], &[0xA0, 0xB1]);
        assert_eq!(cdc.packets, 1);
        assert_eq!(cdc.flushes, 1);
        assert!(cdc.connected());
        assert_eq!(cdc.line_coding(), UsbCdcLineCoding::default());
    }

    #[test]
    fn usb_cdc_byte_stream_can_report_idle_without_error() {
        let mut stream = UsbCdcByteStream::new(FakeCdc::with_read(&[]));

        assert_eq!(stream.try_read_byte().unwrap(), None);
    }

    #[test]
    fn write_all_chunks_full_speed_packets() {
        let mut cdc = FakeCdc::with_read(&[]);
        let bytes = [0x5A; 70];

        cdc.write_all(&bytes).unwrap();

        assert_eq!(cdc.written_len, 70);
        assert_eq!(cdc.packets, 2);
        assert_eq!(cdc.written[0], 0x5A);
        assert_eq!(cdc.written[69], 0x5A);
    }
}
