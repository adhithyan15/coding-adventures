#![no_std]

use board_vm_device::DeviceByteStream;

pub const DEFAULT_UART_BAUD_RATE: u32 = 115_200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBits {
    Seven,
    Eight,
    Nine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    None,
    Even,
    Odd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopBits {
    One,
    Two,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UartConfig {
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
}

impl UartConfig {
    pub const fn new(baud_rate: u32) -> Self {
        Self {
            baud_rate,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
        }
    }

    pub const fn data_bits(mut self, data_bits: DataBits) -> Self {
        self.data_bits = data_bits;
        self
    }

    pub const fn parity(mut self, parity: Parity) -> Self {
        self.parity = parity;
        self
    }

    pub const fn stop_bits(mut self, stop_bits: StopBits) -> Self {
        self.stop_bits = stop_bits;
        self
    }

    pub const fn is_8n1(self) -> bool {
        matches!(
            (self.data_bits, self.parity, self.stop_bits),
            (DataBits::Eight, Parity::None, StopBits::One)
        )
    }
}

impl Default for UartConfig {
    fn default() -> Self {
        Self::new(DEFAULT_UART_BAUD_RATE)
    }
}

pub trait BlockingUart {
    type Error;

    fn read_byte(&mut self) -> Result<u8, Self::Error>;

    fn write_byte(&mut self, byte: u8) -> Result<(), Self::Error>;

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        for byte in bytes {
            self.write_byte(*byte)?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UartByteStream<U> {
    uart: U,
}

impl<U> UartByteStream<U> {
    pub const fn new(uart: U) -> Self {
        Self { uart }
    }

    pub fn into_inner(self) -> U {
        self.uart
    }

    pub fn uart(&self) -> &U {
        &self.uart
    }

    pub fn uart_mut(&mut self) -> &mut U {
        &mut self.uart
    }
}

impl<U> DeviceByteStream for UartByteStream<U>
where
    U: BlockingUart,
{
    type Error = U::Error;

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        self.uart.read_byte()
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.uart.write_all(bytes)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.uart.flush()
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

    struct FakeUart {
        read: [u8; 4],
        read_len: usize,
        read_offset: usize,
        written: [u8; 4],
        written_len: usize,
        flushes: usize,
    }

    impl FakeUart {
        fn with_read(bytes: &[u8]) -> Self {
            let mut read = [0; 4];
            read[..bytes.len()].copy_from_slice(bytes);
            Self {
                read,
                read_len: bytes.len(),
                read_offset: 0,
                written: [0; 4],
                written_len: 0,
                flushes: 0,
            }
        }
    }

    impl BlockingUart for FakeUart {
        type Error = FakeError;

        fn read_byte(&mut self) -> Result<u8, Self::Error> {
            if self.read_offset >= self.read_len {
                return Err(FakeError::EndOfInput);
            }
            let byte = self.read[self.read_offset];
            self.read_offset += 1;
            Ok(byte)
        }

        fn write_byte(&mut self, byte: u8) -> Result<(), Self::Error> {
            if self.written_len >= self.written.len() {
                return Err(FakeError::OutputFull);
            }
            self.written[self.written_len] = byte;
            self.written_len += 1;
            Ok(())
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            self.flushes += 1;
            Ok(())
        }
    }

    #[test]
    fn default_config_matches_host_serial_defaults() {
        assert_eq!(UartConfig::default(), UartConfig::new(115_200));
        assert!(UartConfig::default().is_8n1());
    }

    #[test]
    fn uart_byte_stream_delegates_to_blocking_uart() {
        let mut stream = UartByteStream::new(FakeUart::with_read(&[0x11, 0x22]));

        assert_eq!(stream.read_byte().unwrap(), 0x11);
        stream.write_all(&[0xA0, 0xB1]).unwrap();
        stream.flush().unwrap();

        let uart = stream.into_inner();
        assert_eq!(&uart.written[..uart.written_len], &[0xA0, 0xB1]);
        assert_eq!(uart.flushes, 1);
    }
}
