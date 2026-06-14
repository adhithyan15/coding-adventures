//! `BufferStore` for matrix-cuda — owns a `HashMap<BufferId, CudaBuffer>`.
//!
//! Mirrors `matrix-metal::buffers::BufferStore` and `matrix-cpu`'s buffer
//! store in shape, so the dispatch code in Phase 3+ can be written
//! parallel to its Metal counterpart.
//!
//! Differences from the Metal store:
//!
//! - `MetalBuffer` lives in unified memory on Apple Silicon, so the
//!   Metal store exposes `as_slice` / `as_slice_mut` and writes are
//!   in-place memcpys.  CUDA device memory lives behind PCIe — we
//!   round-trip through `cuMemcpyHtoD` (`CudaDevice::upload`) and
//!   `cuMemcpyDtoH` (`CudaDevice::download`).
//!
//! - Phase 2 supports **offset = 0 only**.  Every caller in
//!   `matrix-metal::dispatch` uses `offset = 0` today (confirmed via
//!   grep), so this isn't a functional limitation for the upcoming
//!   Phase 3 dispatch code.  When a real need for offset writes
//!   appears, we'll either add an offset variant to `cuda-compute`'s
//!   API or do the download / patch / upload dance here.
//!
//! - Cross-platform: `cuda-compute` already loads `libcuda.so` /
//!   `nvcuda.dll` at runtime via `dlopen`/`LoadLibrary` and returns
//!   `Err(CudaError::NotAvailable)` when the driver is absent.  We
//!   thread that error through transparently — no `#[cfg]` split.

use compute_ir::BufferId;
use cuda_compute::{CudaBuffer, CudaDevice};
use std::collections::HashMap;

/// Process-local map of `BufferId → CudaBuffer`.
///
/// Owned by `CudaExecutor::State` (Phase 3 onwards) and accessed
/// only under the executor's `Mutex`, so the store itself can stay
/// `!Sync` without extra wrapping.
pub struct BufferStore {
    buffers: HashMap<BufferId, CudaBuffer>,
}

impl BufferStore {
    /// Construct an empty store.
    pub fn new() -> Self {
        BufferStore {
            buffers: HashMap::new(),
        }
    }

    /// Number of currently-resident buffers.  Useful for tests and
    /// for leak detection — the executor's drop path should leave
    /// this at zero on shutdown.
    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    /// Is the store empty?
    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    /// Allocate `bytes` of device memory at `id`.
    ///
    /// Replaces any existing buffer at that id — matches matrix-cpu /
    /// matrix-metal semantics.  The dropped buffer's destructor
    /// (provided by `cuda-compute::CudaBuffer`) will run, freeing the
    /// device allocation.
    ///
    /// `cuda-compute` rejects zero-length allocations with a typed
    /// error; we forward that decision so call sites get a clear
    /// message instead of a confusing silent zero-byte handle.
    pub fn alloc(
        &mut self,
        device: &CudaDevice,
        id: BufferId,
        bytes: usize,
    ) -> Result<(), String> {
        let buf = device
            .alloc(bytes)
            .map_err(|e| format!("alloc {} bytes: {:?}", bytes, e))?;
        self.buffers.insert(id, buf);
        Ok(())
    }

    /// Free a buffer.  Idempotent — freeing an unknown id is a no-op,
    /// matching the matrix-metal and matrix-cpu contract.
    pub fn free(&mut self, id: BufferId) {
        self.buffers.remove(&id);
    }

    /// Write `data` into the buffer at `id`, starting at byte
    /// `offset`.
    ///
    /// **Phase 2 limitation**: `offset` must be `0`.  See the module
    /// doc-comment for why.
    ///
    /// Errors if:
    /// - The buffer doesn't exist.
    /// - `offset != 0` (Phase 2 restriction).
    /// - `data.len() > buffer.len()` — `cuda-compute` enforces this
    ///   itself and returns a typed error that we propagate.
    pub fn write(
        &mut self,
        device: &CudaDevice,
        id: BufferId,
        offset: usize,
        data: &[u8],
    ) -> Result<(), String> {
        if offset != 0 {
            return Err(format!(
                "matrix-cuda Phase 2: write offset must be 0 (got {}); \
                 partial writes land in a later phase",
                offset
            ));
        }
        let buf = self
            .buffers
            .get(&id)
            .ok_or_else(|| format!("buffer {} not found", id.0))?;
        device
            .upload(buf, data)
            .map_err(|e| format!("upload to buffer {}: {:?}", id.0, e))
    }

    /// Read `len` bytes from the buffer at `id`, starting at byte
    /// `offset`.
    ///
    /// Phase 2 supports arbitrary `offset` / `len` ranges by
    /// downloading the whole buffer (single device→host transfer)
    /// and slicing on the host.  This is wasteful for large buffers
    /// but is the minimum-viable read implementation — Phase 3+ can
    /// introduce a partial-download path when a real workload
    /// justifies it.
    ///
    /// Errors if the buffer doesn't exist, or if `offset + len`
    /// extends past the buffer.
    pub fn read(
        &self,
        device: &CudaDevice,
        id: BufferId,
        offset: usize,
        len: usize,
    ) -> Result<Vec<u8>, String> {
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
        let host = device
            .download(buf)
            .map_err(|e| format!("download buffer {}: {:?}", id.0, e))?;
        Ok(host[offset..end].to_vec())
    }

    /// Borrow a buffer immutably by id.  Returns `Err` if the buffer
    /// doesn't exist.  Dispatch code (Phase 3+) calls this to look
    /// up `CUdeviceptr`s for `cuLaunchKernel`.
    pub fn get(&self, id: BufferId) -> Result<&CudaBuffer, String> {
        self.buffers
            .get(&id)
            .ok_or_else(|| format!("buffer {} not found", id.0))
    }

    /// Does the store hold a buffer at `id`?
    pub fn contains(&self, id: BufferId) -> bool {
        self.buffers.contains_key(&id)
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

    fn device_or_skip() -> Option<CudaDevice> {
        // The tests below need a live CUDA device.  On hosts without
        // an NVIDIA driver (macOS, NVIDIA-less Linux, NVIDIA-less
        // Windows), `CudaDevice::new(0)` fails and the test is a
        // silent pass — matches matrix-metal's "skip if no device"
        // convention.
        CudaDevice::new(0).ok()
    }

    #[test]
    fn new_store_is_empty() {
        let s = BufferStore::new();
        assert_eq!(s.len(), 0);
        assert!(s.is_empty());
        assert!(!s.contains(BufferId(0)));
    }

    #[test]
    fn default_matches_new() {
        let a = BufferStore::new();
        let b = BufferStore::default();
        assert_eq!(a.len(), b.len());
        assert_eq!(a.is_empty(), b.is_empty());
    }

    #[test]
    fn alloc_then_free_round_trips() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let mut s = BufferStore::new();
        let id = BufferId(7);
        s.alloc(&device, id, 64).expect("alloc 64 bytes");
        assert_eq!(s.len(), 1);
        assert!(s.contains(id));
        s.free(id);
        assert_eq!(s.len(), 0);
        assert!(!s.contains(id));
    }

    #[test]
    fn free_unknown_id_is_idempotent_noop() {
        let mut s = BufferStore::new();
        s.free(BufferId(42)); // would panic if not idempotent
        s.free(BufferId(42));
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn alloc_replaces_existing_buffer_at_same_id() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let mut s = BufferStore::new();
        let id = BufferId(3);
        s.alloc(&device, id, 16).unwrap();
        s.alloc(&device, id, 32).unwrap();
        // Still one buffer at `id`; the prior allocation was dropped.
        assert_eq!(s.len(), 1);
        let buf = s.get(id).unwrap();
        assert_eq!(buf.len(), 32);
    }

    #[test]
    fn round_trip_write_then_read_matches_input() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let mut s = BufferStore::new();
        let id = BufferId(1);
        let payload: Vec<u8> = (0..32).collect();
        s.alloc(&device, id, payload.len()).unwrap();
        s.write(&device, id, 0, &payload).unwrap();
        let out = s.read(&device, id, 0, payload.len()).unwrap();
        assert_eq!(out, payload);
    }

    #[test]
    fn read_with_offset_returns_correct_slice() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let mut s = BufferStore::new();
        let id = BufferId(2);
        let payload: Vec<u8> = (0..32).collect();
        s.alloc(&device, id, payload.len()).unwrap();
        s.write(&device, id, 0, &payload).unwrap();
        // Read bytes 4..12 inclusive of 4, exclusive of 12 → 8 bytes.
        let out = s.read(&device, id, 4, 8).unwrap();
        assert_eq!(out, vec![4, 5, 6, 7, 8, 9, 10, 11]);
    }

    #[test]
    fn write_unknown_id_errors() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let mut s = BufferStore::new();
        let err = s
            .write(&device, BufferId(99), 0, &[0u8; 4])
            .unwrap_err();
        assert!(err.contains("not found"), "{}", err);
    }

    #[test]
    fn write_nonzero_offset_errors_in_phase_2() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let mut s = BufferStore::new();
        let id = BufferId(5);
        s.alloc(&device, id, 8).unwrap();
        let err = s.write(&device, id, 1, &[0u8; 4]).unwrap_err();
        assert!(err.contains("offset must be 0"), "{}", err);
    }

    #[test]
    fn read_unknown_id_errors() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let s = BufferStore::new();
        let err = s.read(&device, BufferId(123), 0, 4).unwrap_err();
        assert!(err.contains("not found"), "{}", err);
    }

    #[test]
    fn read_past_end_errors() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let mut s = BufferStore::new();
        let id = BufferId(6);
        s.alloc(&device, id, 8).unwrap();
        let err = s.read(&device, id, 4, 100).unwrap_err();
        assert!(err.contains("read past end"), "{}", err);
    }

    #[test]
    fn read_offset_plus_len_overflow_errors() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let mut s = BufferStore::new();
        let id = BufferId(7);
        s.alloc(&device, id, 16).unwrap();
        let err = s
            .read(&device, id, usize::MAX, 1)
            .unwrap_err();
        assert!(err.contains("overflow"), "{}", err);
    }

    #[test]
    fn alloc_zero_bytes_errors() {
        let Some(device) = device_or_skip() else {
            return;
        };
        // cuda-compute rejects zero-length allocations.  We surface
        // its typed error transparently.
        let mut s = BufferStore::new();
        let err = s.alloc(&device, BufferId(8), 0).unwrap_err();
        assert!(
            err.contains("alloc 0 bytes") || err.contains("zero-length"),
            "{}",
            err
        );
    }

    #[test]
    fn get_unknown_id_errors() {
        let s = BufferStore::new();
        // CudaBuffer doesn't implement Debug, so .unwrap_err()
        // doesn't compile.  Pattern-match instead — same coverage,
        // doesn't add a Debug bound to cuda-compute.
        match s.get(BufferId(0)) {
            Ok(_) => panic!("expected Err on unknown id"),
            Err(msg) => assert!(msg.contains("not found"), "{}", msg),
        }
    }
}
