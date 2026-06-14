//! `BufferStore` — owns the executor's `BufferId → Vec<u8>` map.
//!
//! On a real GPU executor, `Vec<u8>` would be replaced with a Metal
//! `MTLBuffer` or a CUDA `CUdeviceptr`.  On CPU, the buffer *is* the
//! host memory, so we just store bytes directly.
//!
//! ## Security
//!
//! - All offsets and lengths are checked against the buffer's size
//!   with `checked_add` to prevent integer overflow.
//! - `read` and `write` return `Result<_, String>` rather than
//!   panicking on invalid input — wrong-size uploads from a
//!   malicious or buggy runtime should fail cleanly.

use compute_ir::BufferId;
use std::collections::HashMap;

/// Map of [`BufferId`] → owned bytes.  V1 stores buffers as a
/// `HashMap<BufferId, Vec<u8>>` for simplicity; if profiling shows
/// hash overhead matters we can switch to a `Vec`-indexed slot map.
pub struct BufferStore {
    buffers: HashMap<BufferId, Vec<u8>>,
}

impl BufferStore {
    pub fn new() -> Self {
        BufferStore {
            buffers: HashMap::new(),
        }
    }

    /// Allocate a fresh zero-filled buffer of the given size.  If the
    /// buffer already exists at that id, replace it (this matches
    /// real GPU behaviour where the executor decides the BufferId).
    pub fn alloc(&mut self, id: BufferId, bytes: usize) {
        self.buffers.insert(id, vec![0u8; bytes]);
    }

    /// Free a buffer.  Idempotent — freeing an unknown id is a no-op.
    pub fn free(&mut self, id: BufferId) {
        self.buffers.remove(&id);
    }

    /// Write bytes into a buffer starting at `offset`.  Returns
    /// `Err` if the buffer doesn't exist or if `offset + data.len()`
    /// would exceed the buffer's size.
    pub fn write(&mut self, id: BufferId, offset: usize, data: &[u8]) -> Result<(), String> {
        let buf = self
            .buffers
            .get_mut(&id)
            .ok_or_else(|| format!("buffer {} not found", id.0))?;
        let end = offset
            .checked_add(data.len())
            .ok_or_else(|| "offset + len overflows usize".to_string())?;
        if end > buf.len() {
            return Err(format!(
                "write past end: offset {} + len {} > buffer size {}",
                offset,
                data.len(),
                buf.len()
            ));
        }
        buf[offset..end].copy_from_slice(data);
        Ok(())
    }

    /// Read `len` bytes from a buffer starting at `offset`.  Same
    /// bounds-check rules as [`write`].
    pub fn read(&self, id: BufferId, offset: usize, len: usize) -> Result<Vec<u8>, String> {
        let buf = self
            .buffers
            .get(&id)
            .ok_or_else(|| format!("buffer {} not found", id.0))?;
        let end = offset
            .checked_add(len)
            .ok_or_else(|| "offset + len overflows usize".to_string())?;
        if end > buf.len() {
            return Err(format!(
                "read past end: offset {} + len {} > buffer size {}",
                offset,
                len,
                buf.len()
            ));
        }
        Ok(buf[offset..end].to_vec())
    }

    /// Borrow a whole buffer immutably.  Returns `Err` if the id is
    /// unknown.
    pub fn get(&self, id: BufferId) -> Result<&[u8], String> {
        self.buffers
            .get(&id)
            .map(|v| v.as_slice())
            .ok_or_else(|| format!("buffer {} not found", id.0))
    }

    /// Borrow a whole buffer mutably.
    pub fn get_mut(&mut self, id: BufferId) -> Result<&mut [u8], String> {
        self.buffers
            .get_mut(&id)
            .map(|v| v.as_mut_slice())
            .ok_or_else(|| format!("buffer {} not found", id.0))
    }

    /// True iff the store has an entry for `id`.
    pub fn contains(&self, id: BufferId) -> bool {
        self.buffers.contains_key(&id)
    }

    /// Number of buffers currently held.
    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    /// Whether the store has no buffers.
    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }
}

impl Default for BufferStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_write_read_round_trip() {
        let mut s = BufferStore::new();
        s.alloc(BufferId(7), 16);
        s.write(BufferId(7), 0, &[1, 2, 3, 4]).unwrap();
        let read = s.read(BufferId(7), 0, 4).unwrap();
        assert_eq!(read, vec![1, 2, 3, 4]);
        // Unwritten bytes remain zero.
        let read2 = s.read(BufferId(7), 4, 4).unwrap();
        assert_eq!(read2, vec![0, 0, 0, 0]);
    }

    #[test]
    fn write_past_end_errors() {
        let mut s = BufferStore::new();
        s.alloc(BufferId(1), 4);
        assert!(s.write(BufferId(1), 2, &[1, 2, 3, 4]).is_err());
    }

    #[test]
    fn read_past_end_errors() {
        let mut s = BufferStore::new();
        s.alloc(BufferId(1), 4);
        assert!(s.read(BufferId(1), 2, 8).is_err());
    }

    #[test]
    fn missing_buffer_errors() {
        let s = BufferStore::new();
        assert!(s.read(BufferId(99), 0, 1).is_err());
        assert!(s.get(BufferId(99)).is_err());
    }

    #[test]
    fn free_idempotent() {
        let mut s = BufferStore::new();
        s.alloc(BufferId(1), 4);
        s.free(BufferId(1));
        s.free(BufferId(1));   // no panic
        assert!(!s.contains(BufferId(1)));
    }

    #[test]
    fn overflow_in_offset_plus_len() {
        let mut s = BufferStore::new();
        s.alloc(BufferId(1), 4);
        assert!(s.read(BufferId(1), usize::MAX, 1).is_err());
        assert!(s.write(BufferId(1), usize::MAX, &[0]).is_err());
    }
}
