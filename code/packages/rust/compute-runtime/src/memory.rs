//! Memory management -- typed allocations, mapping, staging.
//!
//! # Memory Types on a GPU
//!
//! Unlike a CPU where all RAM is equally accessible, GPUs have distinct memory
//! pools with different performance characteristics:
//!
//! ```text
//! +-----------------------------------------------------------+
//! |                Discrete GPU (NVIDIA, AMD)                  |
//! |                                                           |
//! |   CPU side (system RAM)              GPU side (VRAM)      |
//! |   +------------------+               +-----------------+  |
//! |   |   HOST_VISIBLE   |<---- PCIe --->|   DEVICE_LOCAL  |  |
//! |   |   HOST_COHERENT  |   ~32 GB/s    |   (HBM / GDDR6) |  |
//! |   |   (staging pool) |               |   1-3 TB/s       |  |
//! |   +------------------+               +-----------------+  |
//! +-----------------------------------------------------------+
//!
//! +-----------------------------------------------------------+
//! |              Unified Memory (Apple M-series)              |
//! |                                                           |
//! |   +---------------------------------------------------+  |
//! |   |    DEVICE_LOCAL + HOST_VISIBLE + HOST_COHERENT     |  |
//! |   |    (shared physical RAM)                           |  |
//! |   |    Both CPU and GPU see the same bytes             |  |
//! |   |    No copy needed!                                 |  |
//! |   +---------------------------------------------------+  |
//! +-----------------------------------------------------------+
//! ```
//!
//! # The Staging Buffer Pattern
//!
//! On discrete GPUs, the standard way to get data onto the GPU is:
//!
//! 1. Allocate a HOST_VISIBLE staging buffer (CPU can write to it)
//! 2. Map it, write your data, unmap it
//! 3. Record a cmd_copy_buffer from staging -> DEVICE_LOCAL
//! 4. Submit and wait

use std::collections::HashMap;

use device_simulator::AcceleratorDevice;

use crate::protocols::{
    BufferUsage, MemoryProperties, MemoryType, RuntimeEventType, RuntimeStats, RuntimeTrace,
};

// =========================================================================
// Buffer -- a typed allocation on the device
// =========================================================================

/// A memory allocation on the device.
///
/// # Buffer Lifecycle
///
/// ```text
/// allocate() -> Buffer (with device_address)
/// map()      -> MappedMemory (CPU can read/write)
/// unmap()    -> buffer is GPU-only again
/// free()     -> memory returned to pool
/// ```
#[derive(Debug, Clone)]
pub struct Buffer {
    pub buffer_id: usize,
    pub size: usize,
    pub memory_type: MemoryType,
    pub usage: BufferUsage,
    pub device_address: u64,
    pub mapped: bool,
    pub freed: bool,
}

// =========================================================================
// MappedMemory -- CPU-accessible view of a buffer
// =========================================================================

/// CPU-accessible view of a mapped GPU buffer.
///
/// # What is Memory Mapping?
///
/// Mapping makes device memory accessible to the CPU. On discrete GPUs,
/// this only works for HOST_VISIBLE memory (system RAM accessible via PCIe).
/// On unified memory, any buffer can be mapped.
///
/// After mapping, you can `read()` and `write()` bytes. After `unmap()`, the
/// CPU can no longer access this memory.
pub struct MappedMemory<'a> {
    buffer_id: usize,
    data: &'a mut Vec<u8>,
    dirty: bool,
}

impl<'a> MappedMemory<'a> {
    pub fn buffer_id(&self) -> usize {
        self.buffer_id
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Read bytes from the mapped buffer.
    pub fn read(&self, offset: usize, size: usize) -> Result<Vec<u8>, String> {
        if offset + size > self.data.len() {
            return Err(format!(
                "Read out of bounds: offset={}, size={}, buffer_size={}",
                offset,
                size,
                self.data.len()
            ));
        }
        Ok(self.data[offset..offset + size].to_vec())
    }

    /// Write bytes to the mapped buffer.
    pub fn write(&mut self, offset: usize, data: &[u8]) -> Result<(), String> {
        if offset + data.len() > self.data.len() {
            return Err(format!(
                "Write out of bounds: offset={}, data_size={}, buffer_size={}",
                offset,
                data.len(),
                self.data.len()
            ));
        }
        self.data[offset..offset + data.len()].copy_from_slice(data);
        self.dirty = true;
        Ok(())
    }

    /// Get the full contents of the mapped buffer.
    pub fn get_data(&self) -> &[u8] {
        self.data
    }
}

// =========================================================================
// MemoryManager -- allocates, maps, frees device memory
// =========================================================================

/// Manages typed memory allocations on a device.
///
/// # How It Works
///
/// The MemoryManager wraps Layer 6's raw malloc/free with type information.
/// Each allocation is tagged with a MemoryType and BufferUsage, which the
/// runtime uses for validation and optimization.
///
/// For HOST_VISIBLE allocations, the manager supports mapping -- making the
/// buffer accessible to the CPU. For DEVICE_LOCAL-only allocations, mapping
/// is not allowed (you must use a staging buffer + copy).
pub struct MemoryManager {
    device: Box<dyn AcceleratorDevice>,
    properties: MemoryProperties,
    stats: *mut RuntimeStats,
    buffers: HashMap<usize, Buffer>,
    buffer_data: HashMap<usize, Vec<u8>>,
    next_id: usize,
    current_bytes: usize,
}

// Safety: MemoryManager is used single-threaded in our simulation.
unsafe impl Send for MemoryManager {}
unsafe impl Sync for MemoryManager {}

impl MemoryManager {
    /// Create a new memory manager for a device.
    ///
    /// # Safety
    ///
    /// The `stats` pointer must remain valid for the lifetime of this manager.
    /// This is guaranteed because the LogicalDevice owns both the stats and
    /// the memory manager.
    pub(crate) fn new(
        device: Box<dyn AcceleratorDevice>,
        properties: MemoryProperties,
        stats: *mut RuntimeStats,
    ) -> Self {
        Self {
            device,
            properties,
            stats,
            buffers: HashMap::new(),
            buffer_data: HashMap::new(),
            next_id: 0,
            current_bytes: 0,
        }
    }

    pub fn memory_properties(&self) -> &MemoryProperties {
        &self.properties
    }

    /// Allocate a buffer on the device.
    ///
    /// # The Allocation Flow
    ///
    /// ```text
    /// MemoryManager.allocate(1024, DEVICE_LOCAL)
    ///     |
    ///     +-> Validate: size > 0, memory type supported
    ///     +-> Layer 6: device.malloc(1024) -> device_address
    ///     +-> Create Buffer object with metadata
    ///     +-> Track in buffers map
    ///     +-> Log RuntimeTrace event
    /// ```
    pub fn allocate(
        &mut self,
        size: usize,
        memory_type: MemoryType,
        usage: BufferUsage,
    ) -> Result<usize, String> {
        if size == 0 {
            return Err("Allocation size must be positive".to_string());
        }

        let device_address = self.device.malloc(size);
        let buf_id = self.next_id;
        self.next_id += 1;

        let buf = Buffer {
            buffer_id: buf_id,
            size,
            memory_type,
            usage,
            device_address,
            mapped: false,
            freed: false,
        };
        self.buffers.insert(buf_id, buf);
        self.buffer_data.insert(buf_id, vec![0u8; size]);

        self.current_bytes += size;

        // Update stats through raw pointer (safe: single-threaded, pointer valid)
        unsafe {
            let stats = &mut *self.stats;
            stats.total_allocated_bytes += size;
            stats.total_allocations += 1;
            if self.current_bytes > stats.peak_allocated_bytes {
                stats.peak_allocated_bytes = self.current_bytes;
            }
            stats.traces.push(RuntimeTrace::new(
                RuntimeEventType::MemoryAlloc,
                &format!("Allocated {} bytes (buf#{}, {:?})", size, buf_id, memory_type),
            ));
        }

        Ok(buf_id)
    }

    /// Free a device memory allocation.
    pub fn free(&mut self, buffer_id: usize) -> Result<(), String> {
        let buf = self
            .buffers
            .get(&buffer_id)
            .ok_or_else(|| format!("Buffer {} not found", buffer_id))?;

        if buf.freed {
            return Err(format!("Buffer {} already freed", buffer_id));
        }
        if buf.mapped {
            return Err(format!(
                "Buffer {} is still mapped -- unmap before freeing",
                buffer_id
            ));
        }

        let address = buf.device_address;
        let size = buf.size;

        self.device.free(address);
        self.current_bytes -= size;

        // Mark as freed and remove from tracking
        self.buffers.remove(&buffer_id);
        self.buffer_data.remove(&buffer_id);

        unsafe {
            let stats = &mut *self.stats;
            stats.total_frees += 1;
            stats.traces.push(RuntimeTrace::new(
                RuntimeEventType::MemoryFree,
                &format!("Freed buf#{} ({} bytes)", buffer_id, size),
            ));
        }

        Ok(())
    }

    /// Map a buffer for CPU access.
    ///
    /// Only HOST_VISIBLE buffers can be mapped. Returns a MappedMemory
    /// for reading/writing.
    pub fn map(&mut self, buffer_id: usize) -> Result<MappedMemory<'_>, String> {
        // Validate first (without borrowing buffer_data)
        {
            let buf = self
                .buffers
                .get(&buffer_id)
                .ok_or_else(|| format!("Buffer {} not found", buffer_id))?;

            if buf.freed {
                return Err(format!("Cannot map freed buffer {}", buffer_id));
            }
            if buf.mapped {
                return Err(format!("Buffer {} is already mapped", buffer_id));
            }
            if !buf.memory_type.contains(MemoryType::HOST_VISIBLE) {
                return Err(format!(
                    "Cannot map buffer {}: not HOST_VISIBLE (type={:?})",
                    buffer_id, buf.memory_type
                ));
            }
        }

        // Mark as mapped
        self.buffers.get_mut(&buffer_id).unwrap().mapped = true;

        unsafe {
            let stats = &mut *self.stats;
            stats.total_maps += 1;
            stats.traces.push(RuntimeTrace::new(
                RuntimeEventType::MemoryMap,
                &format!("Mapped buf#{}", buffer_id),
            ));
        }

        let data = self.buffer_data.get_mut(&buffer_id).unwrap();
        Ok(MappedMemory {
            buffer_id,
            data,
            dirty: false,
        })
    }

    /// Unmap a buffer, ending CPU access.
    ///
    /// If the buffer has HOST_COHERENT, the data is automatically synced
    /// to the device.
    pub fn unmap(&mut self, buffer_id: usize) -> Result<(), String> {
        let buf = self
            .buffers
            .get(&buffer_id)
            .ok_or_else(|| format!("Buffer {} not found", buffer_id))?;

        if !buf.mapped {
            return Err(format!("Buffer {} is not mapped", buffer_id));
        }

        // If HOST_COHERENT, sync to device
        if buf.memory_type.contains(MemoryType::HOST_COHERENT) {
            let address = buf.device_address;
            let data = self.buffer_data.get(&buffer_id).unwrap().clone();
            self.device.memcpy_host_to_device(address, &data);
        }

        self.buffers.get_mut(&buffer_id).unwrap().mapped = false;
        Ok(())
    }

    /// Flush CPU writes to make them visible to GPU.
    ///
    /// Only needed for HOST_VISIBLE buffers without HOST_COHERENT.
    pub fn flush(&mut self, buffer_id: usize, offset: usize, size: usize) -> Result<(), String> {
        let buf = self
            .buffers
            .get(&buffer_id)
            .ok_or_else(|| format!("Buffer {} not found", buffer_id))?;

        if buf.freed {
            return Err(format!("Cannot flush freed buffer {}", buffer_id));
        }

        let actual_size = if size > 0 { size } else { buf.size };
        let address = buf.device_address;
        let data = self.buffer_data.get(&buffer_id).unwrap();
        let slice = data[offset..offset + actual_size].to_vec();
        self.device
            .memcpy_host_to_device(address + offset as u64, &slice);
        Ok(())
    }

    /// Invalidate CPU cache so GPU writes become visible to CPU.
    pub fn invalidate(
        &mut self,
        buffer_id: usize,
        offset: usize,
        size: usize,
    ) -> Result<(), String> {
        let buf = self
            .buffers
            .get(&buffer_id)
            .ok_or_else(|| format!("Buffer {} not found", buffer_id))?;

        if buf.freed {
            return Err(format!("Cannot invalidate freed buffer {}", buffer_id));
        }

        let actual_size = if size > 0 { size } else { buf.size };
        let address = buf.device_address;
        let (data, _cycles) = self
            .device
            .memcpy_device_to_host(address + offset as u64, actual_size);
        let buf_data = self.buffer_data.get_mut(&buffer_id).unwrap();
        buf_data[offset..offset + actual_size].copy_from_slice(&data);
        Ok(())
    }

    /// Look up a buffer by ID.
    pub fn get_buffer(&self, buffer_id: usize) -> Result<&Buffer, String> {
        self.buffers
            .get(&buffer_id)
            .ok_or_else(|| format!("Buffer {} not found", buffer_id))
    }

    /// Number of currently allocated buffers.
    pub fn allocated_buffer_count(&self) -> usize {
        self.buffers.len()
    }

    /// Current total bytes allocated.
    pub fn current_allocated_bytes(&self) -> usize {
        self.current_bytes
    }

    /// Internal: get raw data for a buffer.
    pub(crate) fn get_buffer_data(&self, buffer_id: usize) -> &Vec<u8> {
        self.buffer_data.get(&buffer_id).expect("Buffer not found")
    }

    /// Internal: get mutable raw data for a buffer.
    pub(crate) fn get_buffer_data_mut(&mut self, buffer_id: usize) -> &mut Vec<u8> {
        self.buffer_data
            .get_mut(&buffer_id)
            .expect("Buffer not found")
    }

    /// Internal: push buffer data to device. Returns cycles consumed.
    pub(crate) fn sync_buffer_to_device(&mut self, buffer_id: usize) -> u64 {
        let buf = self.buffers.get(&buffer_id).expect("Buffer not found");
        let address = buf.device_address;
        let data = self.buffer_data.get(&buffer_id).unwrap().clone();
        self.device.memcpy_host_to_device(address, &data)
    }

    /// Internal: pull buffer data from device. Returns cycles consumed.
    pub(crate) fn sync_buffer_from_device(&mut self, buffer_id: usize) -> u64 {
        let buf = self.buffers.get(&buffer_id).expect("Buffer not found");
        let address = buf.device_address;
        let size = buf.size;
        let (data, cycles) = self.device.memcpy_device_to_host(address, size);
        let buf_data = self.buffer_data.get_mut(&buffer_id).unwrap();
        buf_data[..].copy_from_slice(&data);
        cycles
    }

    /// Internal: get mutable reference to the underlying device.
    pub(crate) fn device_mut(&mut self) -> &mut dyn AcceleratorDevice {
        &mut *self.device
    }

    /// Internal: borrow the stats pointer.
    pub(crate) fn stats_ptr(&self) -> *mut RuntimeStats {
        self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use device_simulator::NvidiaGPU;

    fn make_manager() -> MemoryManager {
        let device = Box::new(NvidiaGPU::new(None, 2));
        let props = MemoryProperties {
            heaps: vec![],
            is_unified: true,
        };
        let stats = Box::into_raw(Box::new(RuntimeStats::default()));
        MemoryManager::new(device, props, stats)
    }

    #[test]
    fn test_allocate_and_free() {
        let mut mm = make_manager();
        let buf_id = mm
            .allocate(1024, MemoryType::DEVICE_LOCAL | MemoryType::HOST_VISIBLE, BufferUsage::STORAGE)
            .unwrap();
        assert_eq!(mm.allocated_buffer_count(), 1);
        assert_eq!(mm.current_allocated_bytes(), 1024);

        mm.free(buf_id).unwrap();
        assert_eq!(mm.allocated_buffer_count(), 0);
        assert_eq!(mm.current_allocated_bytes(), 0);
    }

    #[test]
    fn test_allocate_zero_fails() {
        let mut mm = make_manager();
        assert!(mm.allocate(0, MemoryType::DEVICE_LOCAL, BufferUsage::STORAGE).is_err());
    }

    #[test]
    fn test_double_free_fails() {
        let mut mm = make_manager();
        let buf_id = mm
            .allocate(64, MemoryType::DEVICE_LOCAL | MemoryType::HOST_VISIBLE, BufferUsage::STORAGE)
            .unwrap();
        mm.free(buf_id).unwrap();
        assert!(mm.free(buf_id).is_err());
    }

    #[test]
    fn test_map_and_unmap() {
        let mut mm = make_manager();
        let buf_id = mm
            .allocate(
                256,
                MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT,
                BufferUsage::STORAGE,
            )
            .unwrap();

        {
            let mut mapped = mm.map(buf_id).unwrap();
            mapped.write(0, &[1, 2, 3, 4]).unwrap();
            assert!(mapped.dirty());
            let read_back = mapped.read(0, 4).unwrap();
            assert_eq!(read_back, vec![1, 2, 3, 4]);
        }

        mm.unmap(buf_id).unwrap();
    }

    #[test]
    fn test_map_device_local_only_fails() {
        let mut mm = make_manager();
        let buf_id = mm
            .allocate(64, MemoryType::DEVICE_LOCAL, BufferUsage::STORAGE)
            .unwrap();
        assert!(mm.map(buf_id).is_err());
    }

    #[test]
    fn test_mapped_read_out_of_bounds() {
        let mut mm = make_manager();
        let buf_id = mm
            .allocate(16, MemoryType::HOST_VISIBLE, BufferUsage::STORAGE)
            .unwrap();
        let mapped = mm.map(buf_id).unwrap();
        assert!(mapped.read(10, 10).is_err());
    }

    #[test]
    fn test_mapped_write_out_of_bounds() {
        let mut mm = make_manager();
        let buf_id = mm
            .allocate(16, MemoryType::HOST_VISIBLE, BufferUsage::STORAGE)
            .unwrap();
        let mut mapped = mm.map(buf_id).unwrap();
        assert!(mapped.write(10, &[0u8; 10]).is_err());
    }

    #[test]
    fn test_free_while_mapped_fails() {
        let mut mm = make_manager();
        let buf_id = mm
            .allocate(64, MemoryType::HOST_VISIBLE, BufferUsage::STORAGE)
            .unwrap();
        // We need to map and then drop the mapped memory before trying to free
        // But the test is: free while still mapped
        // We mark mapped manually for this test
        mm.buffers.get_mut(&buf_id).unwrap().mapped = true;
        assert!(mm.free(buf_id).is_err());
    }
}
