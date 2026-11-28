//! MemoryController -- serializes memory requests from multiple cores.
//!
//! # Why a Memory Controller?
//!
//! In a multi-core system, multiple cores may request memory access in the
//! same clock cycle. Real memory (DRAM) can only handle a limited number of
//! concurrent requests, so the memory controller queues and serializes them.
//!
//! The memory controller is like a librarian at a busy library: patrons
//! (cores) line up with their requests, and the librarian processes them
//! one at a time, delivering books (data) after a delay (latency).
//!
//! # Memory Model
//!
//! The underlying memory is a flat byte array. Word reads/writes use
//! little-endian byte ordering, matching modern ARM and x86 architectures.

use std::fmt;

/// A byte range supplied to the checked RAM image initializer.
#[derive(Debug, Clone, Copy)]
pub struct MemorySegment<'a> {
    /// Starting byte address within RAM.
    pub address: usize,
    /// Bytes to copy into the range.
    pub bytes: &'a [u8],
}

/// Bounds or layout error from checked RAM initialization and observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryImageError {
    /// A byte range extends past the end of RAM.
    OutOfBounds {
        address: usize,
        length: usize,
        memory_size: usize,
    },
    /// Two nonempty segments in one image overlap.
    OverlappingSegments { first: usize, second: usize },
}

impl fmt::Display for MemoryImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfBounds {
                address,
                length,
                memory_size,
            } => write!(
                f,
                "RAM byte range {address}..+{length} exceeds memory size {memory_size}"
            ),
            Self::OverlappingSegments { first, second } => {
                write!(f, "RAM image segments {first} and {second} overlap")
            }
        }
    }
}

impl std::error::Error for MemoryImageError {}

/// An in-flight read request.
#[derive(Debug, Clone)]
struct MemoryRequest {
    /// Starting byte address.
    address: usize,
    /// Number of bytes to read.
    num_bytes: usize,
    /// Which core submitted the request.
    requester_id: usize,
    /// Cycles remaining until data is ready.
    cycles_left: usize,
}

/// An in-flight write request.
#[derive(Debug, Clone)]
struct MemoryWriteRequest {
    /// Starting byte address.
    address: usize,
    /// Bytes to write.
    data: Vec<u8>,
    /// Which core submitted the request.
    #[allow(dead_code)]
    requester_id: usize,
    /// Cycles remaining until write completes.
    cycles_left: usize,
}

/// A completed read -- data delivered to a requester.
#[derive(Debug, Clone)]
pub struct MemoryReadResult {
    /// Which core receives this data.
    pub requester_id: usize,
    /// The address that was read.
    pub address: usize,
    /// The bytes that were read.
    pub data: Vec<u8>,
}

/// Manages access to shared main memory from multiple cores.
pub struct MemoryController {
    /// Raw byte storage (shared across all cores).
    memory: Vec<u8>,

    /// Number of cycles for a memory access to complete.
    latency: usize,

    /// In-flight read requests.
    pending_reads: Vec<MemoryRequest>,

    /// In-flight write requests.
    pending_writes: Vec<MemoryWriteRequest>,
}

impl MemoryController {
    /// Creates a memory controller with the given backing memory and access latency.
    ///
    /// The memory vector is owned by the controller. In a multi-core system,
    /// multiple cores share the same controller to model shared physical memory.
    pub fn new(memory: Vec<u8>, latency: usize) -> Self {
        Self {
            memory,
            latency,
            pending_reads: Vec::new(),
            pending_writes: Vec::new(),
        }
    }

    /// Submits a read request. The read will complete after `latency` cycles.
    pub fn request_read(&mut self, address: usize, num_bytes: usize, requester_id: usize) {
        self.pending_reads.push(MemoryRequest {
            address,
            num_bytes,
            requester_id,
            cycles_left: self.latency,
        });
    }

    /// Submits a write request. The write completes after `latency` cycles.
    pub fn request_write(&mut self, address: usize, data: &[u8], requester_id: usize) {
        self.pending_writes.push(MemoryWriteRequest {
            address,
            data: data.to_vec(),
            requester_id,
            cycles_left: self.latency,
        });
    }

    /// Advances the memory controller by one cycle.
    ///
    /// Decrements all pending request counters. When a request reaches zero
    /// remaining cycles, it is completed:
    ///   - Reads: data is copied from memory and returned in the result list
    ///   - Writes: data is committed to memory
    pub fn tick(&mut self) -> Vec<MemoryReadResult> {
        let mut completed = Vec::new();

        // Process pending reads: drain into a temporary vec to avoid
        // borrowing self while iterating.
        let reads: Vec<_> = std::mem::take(&mut self.pending_reads);
        for mut req in reads {
            req.cycles_left -= 1;
            if req.cycles_left == 0 {
                let data = self.read_memory(req.address, req.num_bytes);
                completed.push(MemoryReadResult {
                    requester_id: req.requester_id,
                    address: req.address,
                    data,
                });
            } else {
                self.pending_reads.push(req);
            }
        }

        // Process pending writes: same pattern.
        let writes: Vec<_> = std::mem::take(&mut self.pending_writes);
        for mut req in writes {
            req.cycles_left -= 1;
            if req.cycles_left == 0 {
                self.write_memory(req.address, &req.data);
            } else {
                self.pending_writes.push(req);
            }
        }

        completed
    }

    /// Reads a 32-bit word from memory at the given address. Little-endian.
    pub fn read_word(&self, address: i64) -> i64 {
        let addr = address as usize;
        if addr + 4 > self.memory.len() {
            return 0;
        }
        (self.memory[addr] as i64)
            | ((self.memory[addr + 1] as i64) << 8)
            | ((self.memory[addr + 2] as i64) << 16)
            | ((self.memory[addr + 3] as i64) << 24)
    }

    /// Writes a 32-bit word to memory at the given address. Little-endian.
    pub fn write_word(&mut self, address: i64, value: i64) {
        let addr = address as usize;
        if addr + 4 > self.memory.len() {
            return;
        }
        self.memory[addr] = (value & 0xFF) as u8;
        self.memory[addr + 1] = ((value >> 8) & 0xFF) as u8;
        self.memory[addr + 2] = ((value >> 16) & 0xFF) as u8;
        self.memory[addr + 3] = ((value >> 24) & 0xFF) as u8;
    }

    /// Copies program bytes into memory starting at the given address.
    pub fn load_program(&mut self, program: &[u8], start_address: usize) {
        if start_address + program.len() > self.memory.len() {
            return;
        }
        self.memory[start_address..start_address + program.len()].copy_from_slice(program);
    }

    /// Returns the total size of memory in bytes.
    pub fn memory_size(&self) -> usize {
        self.memory.len()
    }

    /// Initializes multiple nonoverlapping byte ranges atomically.
    ///
    /// Every range is checked before any bytes are written. Empty ranges are
    /// accepted at any address through `memory_size()`, including exactly at
    /// the end of RAM. Existing bytes outside the supplied ranges are kept.
    pub fn initialize_segments(
        &mut self,
        segments: &[MemorySegment<'_>],
    ) -> Result<(), MemoryImageError> {
        for segment in segments {
            self.validate_range(segment.address, segment.bytes.len())?;
        }

        for first in 0..segments.len() {
            let first_segment = segments[first];
            if first_segment.bytes.is_empty() {
                continue;
            }
            let first_end = first_segment.address + first_segment.bytes.len();
            for (second, second_segment) in segments.iter().enumerate().skip(first + 1) {
                if second_segment.bytes.is_empty() {
                    continue;
                }
                let second_end = second_segment.address + second_segment.bytes.len();
                if first_segment.address < second_end && second_segment.address < first_end {
                    return Err(MemoryImageError::OverlappingSegments { first, second });
                }
            }
        }

        for segment in segments {
            let end = segment.address + segment.bytes.len();
            self.memory[segment.address..end].copy_from_slice(segment.bytes);
        }
        Ok(())
    }

    /// Returns a copy of one bounded byte range from RAM.
    pub fn read_bytes(
        &self,
        address: usize,
        length: usize,
    ) -> Result<Vec<u8>, MemoryImageError> {
        self.validate_range(address, length)?;
        Ok(self.memory[address..address + length].to_vec())
    }

    /// Validates a range without adding untrusted address and length values.
    fn validate_range(&self, address: usize, length: usize) -> Result<(), MemoryImageError> {
        if address > self.memory.len() || length > self.memory.len() - address {
            return Err(MemoryImageError::OutOfBounds {
                address,
                length,
                memory_size: self.memory.len(),
            });
        }
        Ok(())
    }

    /// Returns the number of in-flight requests.
    pub fn pending_count(&self) -> usize {
        self.pending_reads.len() + self.pending_writes.len()
    }

    /// Reads bytes from the backing memory array.
    fn read_memory(&self, address: usize, num_bytes: usize) -> Vec<u8> {
        if address + num_bytes > self.memory.len() {
            return vec![0; num_bytes];
        }
        self.memory[address..address + num_bytes].to_vec()
    }

    /// Writes bytes to the backing memory array.
    fn write_memory(&mut self, address: usize, data: &[u8]) {
        if address + data.len() > self.memory.len() {
            return;
        }
        self.memory[address..address + data.len()].copy_from_slice(data);
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_memory_controller() {
        let mc = MemoryController::new(vec![0; 1024], 10);
        assert_eq!(mc.memory_size(), 1024);
        assert_eq!(mc.pending_count(), 0);
    }

    #[test]
    fn test_read_write_word() {
        let mut mc = MemoryController::new(vec![0; 1024], 10);
        mc.write_word(0, 0x12345678);
        assert_eq!(mc.read_word(0), 0x12345678);
    }

    #[test]
    fn test_read_word_out_of_bounds() {
        let mc = MemoryController::new(vec![0; 4], 10);
        assert_eq!(mc.read_word(4), 0);
    }

    #[test]
    fn test_write_word_out_of_bounds() {
        let mut mc = MemoryController::new(vec![0; 4], 10);
        mc.write_word(4, 42); // should not panic
    }

    #[test]
    fn test_load_program() {
        let mut mc = MemoryController::new(vec![0; 1024], 10);
        mc.load_program(&[0x01, 0x02, 0x03, 0x04], 0);
        assert_eq!(mc.read_word(0), 0x04030201);
    }

    #[test]
    fn test_load_program_out_of_bounds() {
        let mut mc = MemoryController::new(vec![0; 4], 10);
        mc.load_program(&[1, 2, 3, 4, 5], 0); // too big -- should be ignored
        assert_eq!(mc.read_word(0), 0);
    }

    #[test]
    fn test_request_read_and_tick() {
        let mut mc = MemoryController::new(vec![0; 1024], 2);
        mc.write_word(0, 42);
        mc.request_read(0, 4, 0);
        assert_eq!(mc.pending_count(), 1);

        // Tick 1 -- not yet complete.
        let results = mc.tick();
        assert!(results.is_empty());
        assert_eq!(mc.pending_count(), 1);

        // Tick 2 -- should complete.
        let results = mc.tick();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].requester_id, 0);
        assert_eq!(results[0].address, 0);
        assert_eq!(mc.pending_count(), 0);
    }

    #[test]
    fn test_request_write_and_tick() {
        let mut mc = MemoryController::new(vec![0; 1024], 2);
        mc.request_write(0, &[0x42, 0x00, 0x00, 0x00], 0);

        // Tick 1 -- not yet committed.
        mc.tick();
        assert_eq!(mc.read_word(0), 0);

        // Tick 2 -- committed.
        mc.tick();
        assert_eq!(mc.read_word(0), 0x42);
    }

    #[test]
    fn test_little_endian() {
        let mut mc = MemoryController::new(vec![0; 1024], 1);
        mc.write_word(0, 0x01020304);
        // Byte 0 = 0x04 (least significant), Byte 3 = 0x01 (most significant).
        assert_eq!(mc.read_word(0), 0x01020304);
    }

    #[test]
    fn checked_image_writes_disjoint_ranges_and_keeps_other_bytes() {
        let mut mc = MemoryController::new(vec![0xAA; 16], 1);
        mc.initialize_segments(&[
            MemorySegment {
                address: 2,
                bytes: &[1, 2],
            },
            MemorySegment {
                address: 12,
                bytes: &[3, 4],
            },
        ])
        .unwrap();

        assert_eq!(
            mc.read_bytes(0, 16).unwrap(),
            [
                0xAA, 0xAA, 1, 2, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 3, 4,
                0xAA, 0xAA
            ]
        );
    }

    #[test]
    fn checked_image_accepts_exact_end_and_empty_end_ranges() {
        let mut mc = MemoryController::new(vec![0; 256], 1);
        mc.initialize_segments(&[
            MemorySegment {
                address: 252,
                bytes: &[1, 2, 3, 4],
            },
            MemorySegment {
                address: 256,
                bytes: &[],
            },
        ])
        .unwrap();
        assert_eq!(mc.read_bytes(252, 4).unwrap(), [1, 2, 3, 4]);
        assert!(mc.read_bytes(256, 0).unwrap().is_empty());
    }

    #[test]
    fn checked_image_rejects_invalid_batch_without_partial_writes() {
        let original = vec![0xA5; 8];
        let mut mc = MemoryController::new(original.clone(), 1);
        let error = mc
            .initialize_segments(&[
                MemorySegment {
                    address: 1,
                    bytes: &[1, 2],
                },
                MemorySegment {
                    address: 7,
                    bytes: &[3, 4],
                },
            ])
            .unwrap_err();
        assert_eq!(
            error,
            MemoryImageError::OutOfBounds {
                address: 7,
                length: 2,
                memory_size: 8,
            }
        );
        assert_eq!(mc.read_bytes(0, 8).unwrap(), original);

        let overflow = mc.read_bytes(usize::MAX, 1).unwrap_err();
        assert_eq!(
            overflow,
            MemoryImageError::OutOfBounds {
                address: usize::MAX,
                length: 1,
                memory_size: 8,
            }
        );
    }

    #[test]
    fn checked_image_rejects_overlap_atomically() {
        let original = vec![0xA5; 8];
        let mut mc = MemoryController::new(original.clone(), 1);
        assert_eq!(
            mc.initialize_segments(&[
                MemorySegment {
                    address: 1,
                    bytes: &[1, 2, 3],
                },
                MemorySegment {
                    address: 3,
                    bytes: &[4, 5],
                },
            ]),
            Err(MemoryImageError::OverlappingSegments {
                first: 0,
                second: 1,
            })
        );
        assert_eq!(mc.read_bytes(0, 8).unwrap(), original);
    }

    #[test]
    fn checked_read_returns_an_independent_copy() {
        let mc = MemoryController::new(vec![1, 2, 3, 4], 1);
        let mut observed = mc.read_bytes(0, 4).unwrap();
        observed[0] = 99;
        assert_eq!(mc.read_bytes(0, 4).unwrap(), [1, 2, 3, 4]);
        assert!(mc.read_bytes(3, 2).is_err());
    }
}
