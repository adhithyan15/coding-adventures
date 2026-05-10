//! `BufferStore` for matrix-metal — owns a `HashMap<BufferId, MetalBuffer>`.
//!
//! Mirrors the matrix-cpu `BufferStore` API so the dispatch code is
//! parallel-structured.  The difference is that buffers live on the
//! GPU (or in unified memory on Apple Silicon, which means they're
//! also CPU-visible — that's what makes `as_slice` / `as_slice_mut`
//! work).

#[cfg(target_vendor = "apple")]
use compute_ir::BufferId;
#[cfg(target_vendor = "apple")]
use metal_compute::{MetalBuffer, MetalDevice, MetalError};
#[cfg(target_vendor = "apple")]
use std::collections::HashMap;

#[cfg(target_vendor = "apple")]
pub struct BufferStore {
    buffers: HashMap<BufferId, MetalBuffer>,
}

#[cfg(target_vendor = "apple")]
impl BufferStore {
    pub fn new() -> Self {
        BufferStore {
            buffers: HashMap::new(),
        }
    }

    /// Allocate a fresh zero-initialised buffer at `id` of `bytes`
    /// bytes.  Replaces any existing buffer at that id (matches
    /// matrix-cpu's semantics).
    pub fn alloc(&mut self, device: &MetalDevice, id: BufferId, bytes: usize) -> Result<(), String> {
        let buf = device
            .alloc(bytes)
            .map_err(|e: MetalError| format!("alloc {} bytes: {:?}", bytes, e))?;
        self.buffers.insert(id, buf);
        Ok(())
    }

    /// Free a buffer.  Idempotent.
    pub fn free(&mut self, id: BufferId) {
        self.buffers.remove(&id);
    }

    /// Write bytes into a buffer at offset 0.  Errors if the buffer
    /// doesn't exist or if `data.len() > buf.len()`.
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
        let slice = buf.as_slice_mut();
        slice[offset..end].copy_from_slice(data);
        Ok(())
    }

    /// Read `len` bytes starting at `offset`.
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
        Ok(buf.as_slice()[offset..end].to_vec())
    }

    /// Borrow a buffer immutably by id.
    pub fn get(&self, id: BufferId) -> Result<&MetalBuffer, String> {
        self.buffers
            .get(&id)
            .ok_or_else(|| format!("buffer {} not found", id.0))
    }

    pub fn contains(&self, id: BufferId) -> bool {
        self.buffers.contains_key(&id)
    }
}

#[cfg(target_vendor = "apple")]
impl Default for BufferStore {
    fn default() -> Self {
        Self::new()
    }
}

// ── Non-Apple stub ────────────────────────────────────────────────

#[cfg(not(target_vendor = "apple"))]
pub struct BufferStore;

#[cfg(not(target_vendor = "apple"))]
impl BufferStore {
    pub fn new() -> Self {
        BufferStore
    }
}

#[cfg(not(target_vendor = "apple"))]
impl Default for BufferStore {
    fn default() -> Self {
        Self::new()
    }
}
