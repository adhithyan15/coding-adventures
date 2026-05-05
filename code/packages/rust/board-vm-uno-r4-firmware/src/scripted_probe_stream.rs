use board_vm_device::DeviceByteStream;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeStreamError {
    BufferTooSmall,
    EndOfInput,
}

pub struct ProbeStream<const READ_BYTES: usize, const WRITE_BYTES: usize> {
    read: [u8; READ_BYTES],
    read_len: usize,
    read_offset: usize,
    written: [u8; WRITE_BYTES],
    written_len: usize,
}

impl<const READ_BYTES: usize, const WRITE_BYTES: usize> ProbeStream<READ_BYTES, WRITE_BYTES> {
    pub fn with_read(bytes: &[u8]) -> Result<Self, ProbeStreamError> {
        if bytes.len() > READ_BYTES {
            return Err(ProbeStreamError::BufferTooSmall);
        }
        let mut read = [0; READ_BYTES];
        read[..bytes.len()].copy_from_slice(bytes);
        Ok(Self {
            read,
            read_len: bytes.len(),
            read_offset: 0,
            written: [0; WRITE_BYTES],
            written_len: 0,
        })
    }

    pub fn written(&self) -> &[u8] {
        &self.written[..self.written_len]
    }
}

impl<const READ_BYTES: usize, const WRITE_BYTES: usize> DeviceByteStream
    for ProbeStream<READ_BYTES, WRITE_BYTES>
{
    type Error = ProbeStreamError;

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        if self.read_offset >= self.read_len {
            return Err(ProbeStreamError::EndOfInput);
        }
        let byte = self.read[self.read_offset];
        self.read_offset += 1;
        Ok(byte)
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        let end = self
            .written_len
            .checked_add(bytes.len())
            .ok_or(ProbeStreamError::BufferTooSmall)?;
        if end > WRITE_BYTES {
            return Err(ProbeStreamError::BufferTooSmall);
        }
        self.written[self.written_len..end].copy_from_slice(bytes);
        self.written_len = end;
        Ok(())
    }
}
