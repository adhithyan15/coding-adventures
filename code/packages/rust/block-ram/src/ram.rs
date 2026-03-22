//! RAM Modules — synchronous memory with read/write ports.
//!
//! # From Array to Module
//!
//! An SRAM array ([`crate::sram`]) provides raw row-level read/write. A RAM module
//! adds the interface that digital circuits actually use:
//!
//! 1. **Address decoding** — binary address bits select a row
//! 2. **Synchronous operation** — reads and writes happen on clock edges
//! 3. **Read modes** — what the output shows during a write operation
//! 4. **Dual-port access** — two independent ports for simultaneous operations
//!
//! # Read Modes
//!
//! During a write operation, what should the data output show? There are
//! three valid answers, and different designs need different behaviors:
//!
//! 1. **ReadFirst**: Output shows the OLD value at the address being written.
//!    The read happens before the write within the same cycle. Useful when
//!    you need to know what was there before overwriting it.
//!
//! 2. **WriteFirst** (read-after-write): Output shows the NEW value being
//!    written. The write happens first, then the read sees the new value.
//!    Useful for pipeline forwarding.
//!
//! 3. **NoChange**: Output retains its previous value during writes. This
//!    saves power in FPGA Block RAMs because the read circuitry does not
//!    activate during writes.
//!
//! # Dual-Port RAM
//!
//! Two completely independent ports (A and B), each with its own address,
//! data, and write enable. Both can operate simultaneously:
//! - Read A + Read B at different addresses: both get their data
//! - Write A + Read B at different addresses: both succeed
//! - Write A + Write B at the SAME address: **collision** (undefined in
//!   hardware, we return an error)

use crate::sram::{validate_bit, SRAMArray};
use std::fmt;

// ===========================================================================
// ReadMode — controls what data_out shows during writes
// ===========================================================================

/// Controls what data_out shows during a write operation.
///
/// ```text
/// ReadFirst:  data_out = old value (read before write)
/// WriteFirst: data_out = new value (write before read)
/// NoChange:   data_out = previous read value (output unchanged)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadMode {
    /// Read the old value before writing the new one.
    ReadFirst,
    /// Write first, then read back the new value.
    WriteFirst,
    /// Output retains its previous value during writes.
    NoChange,
}

// ===========================================================================
// WriteCollisionError — dual-port write conflict
// ===========================================================================

/// Error returned when both ports of a dual-port RAM write to the same address.
///
/// In real hardware, simultaneous writes to the same address produce
/// undefined results (the cell may store either value, or a corrupted
/// value). We detect this and return an error to prevent silent bugs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteCollisionError {
    /// The conflicting address.
    pub address: usize,
}

impl fmt::Display for WriteCollisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Write collision: both ports writing to address {}",
            self.address
        )
    }
}

impl std::error::Error for WriteCollisionError {}

// ===========================================================================
// SinglePortRAM — one address port, one data bus
// ===========================================================================

/// Single-port synchronous RAM.
///
/// One address port, one data bus. Each clock cycle you can do ONE
/// operation: read OR write (controlled by write_enable).
///
/// ```text
///                 +----------------------------+
///   address ------+                            |
///                 |     Single-Port RAM        +---- data_out
///   data_in ------+                            |
///                 |     (depth x width)        |
///   write_en -----+                            |
///                 |                            |
///   clock --------+                            |
///                 +----------------------------+
/// ```
///
/// Operations happen on the rising edge of the clock (transition 0->1).
///
/// # Example
///
/// ```
/// use block_ram::ram::{SinglePortRAM, ReadMode};
/// let mut ram = SinglePortRAM::new(256, 8, ReadMode::ReadFirst);
///
/// // Write 0xFF to address 0 (rising edge: clock 0 -> 1)
/// ram.tick(0, 0, &[1,1,1,1,1,1,1,1], 1);
/// let out = ram.tick(1, 0, &[1,1,1,1,1,1,1,1], 1);
///
/// // Read from address 0 (rising edge)
/// ram.tick(0, 0, &[0,0,0,0,0,0,0,0], 0);
/// let out = ram.tick(1, 0, &[0,0,0,0,0,0,0,0], 0);
/// assert_eq!(out, vec![1,1,1,1,1,1,1,1]);
/// ```
#[derive(Debug, Clone)]
pub struct SinglePortRAM {
    depth: usize,
    width: usize,
    read_mode: ReadMode,
    array: SRAMArray,
    prev_clock: u8,
    last_read: Vec<u8>,
}

impl SinglePortRAM {
    /// Create a new single-port RAM.
    ///
    /// # Panics
    ///
    /// Panics if depth or width < 1.
    pub fn new(depth: usize, width: usize, read_mode: ReadMode) -> Self {
        assert!(depth >= 1, "depth must be >= 1, got {depth}");
        assert!(width >= 1, "width must be >= 1, got {width}");

        Self {
            depth,
            width,
            read_mode,
            array: SRAMArray::new(depth, width),
            prev_clock: 0,
            last_read: vec![0; width],
        }
    }

    /// Execute one half-cycle. Operations happen on rising edge (0->1).
    ///
    /// # Parameters
    ///
    /// - `clock`: Clock signal (0 or 1)
    /// - `address`: Word address (0 to depth-1)
    /// - `data_in`: Data to write (slice of width bits)
    /// - `write_enable`: 0 = read, 1 = write
    ///
    /// # Returns
    ///
    /// `data_out`: Vec of width bits read from the address.
    /// During writes, behavior depends on the read mode.
    ///
    /// # Panics
    ///
    /// Panics if address is out of range or data_in has the wrong length.
    pub fn tick(
        &mut self,
        clock: u8,
        address: usize,
        data_in: &[u8],
        write_enable: u8,
    ) -> Vec<u8> {
        validate_bit(clock, "clock");
        validate_bit(write_enable, "write_enable");
        assert!(
            address < self.depth,
            "address {address} out of range [0, {}]",
            self.depth - 1
        );
        assert!(
            data_in.len() == self.width,
            "data_in length {} does not match width {}",
            data_in.len(),
            self.width
        );

        // Detect rising edge: previous clock was 0, now it's 1
        let rising_edge = self.prev_clock == 0 && clock == 1;
        self.prev_clock = clock;

        if !rising_edge {
            return self.last_read.clone();
        }

        // Rising edge: perform the operation
        if write_enable == 0 {
            // Read operation
            self.last_read = self.array.read(address);
            return self.last_read.clone();
        }

        // Write operation — behavior depends on read mode
        match self.read_mode {
            ReadMode::ReadFirst => {
                // Read the old value first, then write
                self.last_read = self.array.read(address);
                self.array.write(address, data_in);
                self.last_read.clone()
            }
            ReadMode::WriteFirst => {
                // Write first, then read back the new value
                self.array.write(address, data_in);
                self.last_read = data_in.to_vec();
                self.last_read.clone()
            }
            ReadMode::NoChange => {
                // Write but don't update data_out
                self.array.write(address, data_in);
                self.last_read.clone()
            }
        }
    }

    /// Number of addressable words.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Bits per word.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Return all contents for inspection.
    pub fn dump(&self) -> Vec<Vec<u8>> {
        (0..self.depth).map(|row| self.array.read(row)).collect()
    }
}

// ===========================================================================
// DualPortRAM — two independent read/write ports
// ===========================================================================

/// True dual-port synchronous RAM.
///
/// Two independent ports (A and B), each with its own address, data,
/// and write enable. Both ports can operate simultaneously on different
/// addresses.
///
/// ```text
/// +--------------------------------------------+
/// |               Dual-Port RAM                |
/// |  Port A                      Port B        |
/// |  addr_a, din_a, we_a        addr_b, din_b  |
/// |  dout_a                      we_b, dout_b  |
/// +--------------------------------------------+
/// ```
///
/// Write collision: if both ports write to the same address in the
/// same cycle, a [`WriteCollisionError`] is returned.
#[derive(Debug, Clone)]
pub struct DualPortRAM {
    depth: usize,
    width: usize,
    read_mode_a: ReadMode,
    read_mode_b: ReadMode,
    array: SRAMArray,
    prev_clock: u8,
    last_read_a: Vec<u8>,
    last_read_b: Vec<u8>,
}

impl DualPortRAM {
    /// Create a new dual-port RAM.
    ///
    /// # Panics
    ///
    /// Panics if depth or width < 1.
    pub fn new(
        depth: usize,
        width: usize,
        read_mode_a: ReadMode,
        read_mode_b: ReadMode,
    ) -> Self {
        assert!(depth >= 1, "depth must be >= 1, got {depth}");
        assert!(width >= 1, "width must be >= 1, got {width}");

        Self {
            depth,
            width,
            read_mode_a,
            read_mode_b,
            array: SRAMArray::new(depth, width),
            prev_clock: 0,
            last_read_a: vec![0; width],
            last_read_b: vec![0; width],
        }
    }

    /// Execute one half-cycle on both ports.
    ///
    /// # Returns
    ///
    /// `Ok((data_out_a, data_out_b))` on success, or
    /// `Err(WriteCollisionError)` if both ports write to the same address.
    ///
    /// # Panics
    ///
    /// Panics if addresses are out of range or data lengths are wrong.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        clock: u8,
        address_a: usize,
        data_in_a: &[u8],
        write_enable_a: u8,
        address_b: usize,
        data_in_b: &[u8],
        write_enable_b: u8,
    ) -> Result<(Vec<u8>, Vec<u8>), WriteCollisionError> {
        validate_bit(clock, "clock");
        validate_bit(write_enable_a, "write_enable_a");
        validate_bit(write_enable_b, "write_enable_b");
        assert!(
            address_a < self.depth,
            "address_a {address_a} out of range [0, {}]",
            self.depth - 1
        );
        assert!(
            address_b < self.depth,
            "address_b {address_b} out of range [0, {}]",
            self.depth - 1
        );
        assert!(
            data_in_a.len() == self.width,
            "data_in_a length {} does not match width {}",
            data_in_a.len(),
            self.width
        );
        assert!(
            data_in_b.len() == self.width,
            "data_in_b length {} does not match width {}",
            data_in_b.len(),
            self.width
        );

        let rising_edge = self.prev_clock == 0 && clock == 1;
        self.prev_clock = clock;

        if !rising_edge {
            return Ok((self.last_read_a.clone(), self.last_read_b.clone()));
        }

        // Check for write collision
        if write_enable_a == 1 && write_enable_b == 1 && address_a == address_b {
            return Err(WriteCollisionError {
                address: address_a,
            });
        }

        // Process port A
        let out_a = self.process_port(
            address_a,
            data_in_a,
            write_enable_a,
            self.read_mode_a,
            self.last_read_a.clone(),
        );
        self.last_read_a = out_a.clone();

        // Process port B
        let out_b = self.process_port(
            address_b,
            data_in_b,
            write_enable_b,
            self.read_mode_b,
            self.last_read_b.clone(),
        );
        self.last_read_b = out_b.clone();

        Ok((out_a, out_b))
    }

    /// Process a single port operation.
    fn process_port(
        &mut self,
        address: usize,
        data_in: &[u8],
        write_enable: u8,
        read_mode: ReadMode,
        last_read: Vec<u8>,
    ) -> Vec<u8> {
        if write_enable == 0 {
            return self.array.read(address);
        }

        match read_mode {
            ReadMode::ReadFirst => {
                let result = self.array.read(address);
                self.array.write(address, data_in);
                result
            }
            ReadMode::WriteFirst => {
                self.array.write(address, data_in);
                data_in.to_vec()
            }
            ReadMode::NoChange => {
                self.array.write(address, data_in);
                last_read
            }
        }
    }

    /// Number of addressable words.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Bits per word.
    pub fn width(&self) -> usize {
        self.width
    }
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_port_write_and_read() {
        let mut ram = SinglePortRAM::new(4, 8, ReadMode::ReadFirst);

        // Write to address 0
        ram.tick(0, 0, &[1, 0, 1, 0, 1, 0, 1, 0], 1);
        ram.tick(1, 0, &[1, 0, 1, 0, 1, 0, 1, 0], 1);

        // Read from address 0
        ram.tick(0, 0, &[0; 8], 0);
        let out = ram.tick(1, 0, &[0; 8], 0);
        assert_eq!(out, vec![1, 0, 1, 0, 1, 0, 1, 0]);
    }

    #[test]
    fn test_single_port_read_first() {
        let mut ram = SinglePortRAM::new(4, 4, ReadMode::ReadFirst);

        // Write 1010 to addr 0
        ram.tick(0, 0, &[1, 0, 1, 0], 1);
        ram.tick(1, 0, &[1, 0, 1, 0], 1);

        // Write 0101 to addr 0 with ReadFirst — should return OLD value
        ram.tick(0, 0, &[0, 1, 0, 1], 1);
        let out = ram.tick(1, 0, &[0, 1, 0, 1], 1);
        assert_eq!(out, vec![1, 0, 1, 0]); // old value
    }

    #[test]
    fn test_single_port_write_first() {
        let mut ram = SinglePortRAM::new(4, 4, ReadMode::WriteFirst);

        // Write 1010 to addr 0
        ram.tick(0, 0, &[1, 0, 1, 0], 1);
        ram.tick(1, 0, &[1, 0, 1, 0], 1);

        // Write 0101 to addr 0 with WriteFirst — should return NEW value
        ram.tick(0, 0, &[0, 1, 0, 1], 1);
        let out = ram.tick(1, 0, &[0, 1, 0, 1], 1);
        assert_eq!(out, vec![0, 1, 0, 1]); // new value
    }

    #[test]
    fn test_dual_port_independent_read_write() {
        let mut ram = DualPortRAM::new(4, 4, ReadMode::ReadFirst, ReadMode::ReadFirst);

        // Write via port A to address 0
        ram.tick(0, 0, &[1, 1, 0, 0], 1, 1, &[0; 4], 0).unwrap();
        ram.tick(1, 0, &[1, 1, 0, 0], 1, 1, &[0; 4], 0).unwrap();

        // Read via port B from address 0
        ram.tick(0, 0, &[0; 4], 0, 0, &[0; 4], 0).unwrap();
        let (_, out_b) = ram.tick(1, 0, &[0; 4], 0, 0, &[0; 4], 0).unwrap();
        assert_eq!(out_b, vec![1, 1, 0, 0]);
    }

    #[test]
    fn test_dual_port_write_collision() {
        let mut ram = DualPortRAM::new(4, 4, ReadMode::ReadFirst, ReadMode::ReadFirst);

        // Both ports write to address 0 — should error
        ram.tick(0, 0, &[1; 4], 1, 0, &[0; 4], 1).unwrap();
        let result = ram.tick(1, 0, &[1; 4], 1, 0, &[0; 4], 1);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().address, 0);
    }
}
