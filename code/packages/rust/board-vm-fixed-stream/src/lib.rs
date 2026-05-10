#![no_std]

use board_vm_device::DeviceByteStream;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedStreamError {
    BufferTooSmall,
    EndOfInput,
}

#[derive(Debug)]
pub struct FixedByteStream<const READ_BYTES: usize, const WRITE_BYTES: usize> {
    read: [u8; READ_BYTES],
    read_len: usize,
    read_offset: usize,
    written: [u8; WRITE_BYTES],
    written_len: usize,
    flushes: usize,
}

impl<const READ_BYTES: usize, const WRITE_BYTES: usize> FixedByteStream<READ_BYTES, WRITE_BYTES> {
    pub fn with_read(bytes: &[u8]) -> Result<Self, FixedStreamError> {
        if bytes.len() > READ_BYTES {
            return Err(FixedStreamError::BufferTooSmall);
        }
        let mut read = [0; READ_BYTES];
        read[..bytes.len()].copy_from_slice(bytes);
        Ok(Self {
            read,
            read_len: bytes.len(),
            read_offset: 0,
            written: [0; WRITE_BYTES],
            written_len: 0,
            flushes: 0,
        })
    }

    pub fn written(&self) -> &[u8] {
        &self.written[..self.written_len]
    }

    pub fn remaining_input(&self) -> usize {
        self.read_len.saturating_sub(self.read_offset)
    }

    pub fn flushes(&self) -> usize {
        self.flushes
    }
}

impl<const READ_BYTES: usize, const WRITE_BYTES: usize> DeviceByteStream
    for FixedByteStream<READ_BYTES, WRITE_BYTES>
{
    type Error = FixedStreamError;

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        if self.read_offset >= self.read_len {
            return Err(FixedStreamError::EndOfInput);
        }
        let byte = self.read[self.read_offset];
        self.read_offset += 1;
        Ok(byte)
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        let end = self
            .written_len
            .checked_add(bytes.len())
            .ok_or(FixedStreamError::BufferTooSmall)?;
        if end > WRITE_BYTES {
            return Err(FixedStreamError::BufferTooSmall);
        }
        self.written[self.written_len..end].copy_from_slice(bytes);
        self.written_len = end;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.flushes += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_scripted_bytes_and_records_writes() {
        let mut stream = FixedByteStream::<4, 4>::with_read(&[1, 2]).unwrap();

        assert_eq!(stream.read_byte().unwrap(), 1);
        assert_eq!(stream.remaining_input(), 1);
        stream.write_all(&[0xA0, 0xB0]).unwrap();
        stream.flush().unwrap();

        assert_eq!(stream.read_byte().unwrap(), 2);
        assert_eq!(stream.read_byte(), Err(FixedStreamError::EndOfInput));
        assert_eq!(stream.written(), &[0xA0, 0xB0]);
        assert_eq!(stream.flushes(), 1);
    }

    #[test]
    fn rejects_oversized_read_or_write_buffers() {
        assert_eq!(
            FixedByteStream::<1, 1>::with_read(&[1, 2]).unwrap_err(),
            FixedStreamError::BufferTooSmall
        );

        let mut stream = FixedByteStream::<1, 1>::with_read(&[1]).unwrap();
        assert_eq!(
            stream.write_all(&[1, 2]),
            Err(FixedStreamError::BufferTooSmall)
        );
    }
}
