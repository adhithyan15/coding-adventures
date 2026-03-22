//! # D12 Device Driver Framework
//!
//! A device driver is a piece of software that knows how to talk to a specific
//! piece of hardware. Without drivers, every program that wanted to read from a
//! disk would need to know the exact protocol for that specific disk model —
//! the register addresses, the timing requirements, the error codes. If you
//! replaced the disk with a different model, every program would break.
//!
//! Device drivers solve this by providing a **uniform interface** over diverse
//! hardware. A program says "read 512 bytes from block 7" and the driver
//! translates that into whatever specific commands the hardware needs.
//!
//! **Analogy:** Think of a universal remote control. You press "Volume Up" and
//! it works on your Samsung TV, your Sony soundbar, and your LG projector.
//! Each device speaks a different infrared protocol, but the remote translates
//! your single button press into the right signal for each device. Device
//! drivers are the universal remote for your operating system.
//!
//! # Architecture
//!
//! ```text
//! User Programs
//!     |  sys_write(fd, buf, n)
//!     v
//! OS Kernel (S04)
//!     |  "Which device does fd=1 refer to?"
//!     v
//! Device Driver Framework (D12) <-- THIS CRATE
//!     |  CharacterDevice / BlockDevice / NetworkDevice
//!     v
//! Simulated Hardware (display, keyboard, disk, NIC)
//! ```
//!
//! # The Three Device Families
//!
//! Not all hardware behaves the same way:
//!
//! | Type      | Data Model         | Examples              | Analogy          |
//! |-----------|--------------------|-----------------------|------------------|
//! | Character | Stream of bytes    | Keyboard, display     | A pipe           |
//! | Block     | Fixed-size chunks  | Hard disk, SSD        | A filing cabinet |
//! | Network   | Variable packets   | Ethernet NIC, WiFi    | A mailbox        |

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

// =========================================================================
// Well-known interrupt numbers for devices
// =========================================================================

/// Timer interrupt — clock tick from timer chip (interrupt 32).
pub const INT_TIMER: usize = 32;
/// Keyboard interrupt — key pressed (interrupt 33).
pub const INT_KEYBOARD: usize = 33;
/// Disk interrupt — block I/O completed (interrupt 34).
pub const INT_DISK: usize = 34;
/// NIC interrupt — packet received (interrupt 35).
pub const INT_NIC: usize = 35;
/// System call interrupt — ecall instruction (interrupt 128).
pub const INT_SYSCALL: usize = 128;

// =========================================================================
// Well-known major numbers for device drivers
// =========================================================================

/// Major number for display devices (character).
pub const MAJOR_DISPLAY: u32 = 1;
/// Major number for keyboard devices (character).
pub const MAJOR_KEYBOARD: u32 = 2;
/// Major number for disk devices (block).
pub const MAJOR_DISK: u32 = 3;
/// Major number for NIC devices (network).
pub const MAJOR_NIC: u32 = 4;

// =========================================================================
// DeviceType — classifies hardware into three families
// =========================================================================

/// DeviceType classifies hardware into three families, each with a different
/// data model and access pattern.
///
/// - **Character** devices are sequential — bytes flow through like water in
///   a pipe. You cannot "seek" to byte 47 of a keyboard.
/// - **Block** devices are random-access — you can read any block in any
///   order, like pulling drawers from a filing cabinet.
/// - **Network** devices deal in packets — discrete messages with headers
///   and payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    /// Byte-stream devices: keyboard, serial port, display terminal.
    Character = 0,
    /// Fixed-size block devices: hard disk, SSD, USB drive.
    Block = 1,
    /// Packet-oriented devices: Ethernet NIC, WiFi adapter.
    Network = 2,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::Character => write!(f, "Character"),
            DeviceType::Block => write!(f, "Block"),
            DeviceType::Network => write!(f, "Network"),
        }
    }
}

// =========================================================================
// DeviceInfo — common fields for all devices
// =========================================================================

/// DeviceInfo holds the common metadata that every device has, regardless of
/// whether it is a keyboard, disk, or network card.
///
/// # Major and Minor Numbers
///
/// In Unix, every device is identified by two numbers:
/// - **Major number**: identifies the DRIVER (which software module handles this)
/// - **Minor number**: identifies the INSTANCE (which specific device of that type)
///
/// Example:
/// ```text
/// Major 3 = disk driver
/// Minor 0 = first disk (/dev/sda)
/// Minor 1 = second disk (/dev/sdb)
/// ```
///
/// This lets the kernel route I/O requests to the correct driver without
/// knowing anything about the hardware itself.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Human-readable name, e.g., "disk0". Used for lookup and logging.
    pub name: String,
    /// Character, Block, or Network — determines which protocol the device uses.
    pub device_type: DeviceType,
    /// Driver identifier. All devices handled by the same driver share a major number.
    pub major: u32,
    /// Instance identifier within the driver. First disk = 0, second = 1.
    pub minor: u32,
    /// Which interrupt this device raises when it needs attention.
    /// Use `None` if the device does not use interrupts (e.g., display).
    pub interrupt_number: Option<usize>,
    /// Has init() been called? Prevents double-initialization.
    pub initialized: bool,
}

impl DeviceInfo {
    /// Create a new DeviceInfo with the given parameters.
    pub fn new(name: &str, device_type: DeviceType, major: u32, minor: u32, interrupt_number: Option<usize>) -> Self {
        Self {
            name: name.to_string(),
            device_type,
            major,
            minor,
            interrupt_number,
            initialized: false,
        }
    }
}

impl std::fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let irq = match self.interrupt_number {
            Some(n) => n.to_string(),
            None => "-1".to_string(),
        };
        write!(
            f,
            "{} ({}, major={}, minor={}, irq={})",
            self.name, self.device_type, self.major, self.minor, irq
        )
    }
}

// =========================================================================
// CharacterDevice trait
// =========================================================================

/// CharacterDevice — a device that produces or consumes a stream of bytes.
///
/// Character devices are sequential: bytes flow through one at a time, like
/// water through a pipe. You read whatever is available and write whatever
/// you have. There is no concept of "seeking" to a position.
///
/// Examples: keyboard (produces bytes when keys are pressed), serial port
/// (sends/receives bytes over a wire), display terminal (consumes bytes and
/// renders them as characters on screen).
pub trait CharacterDevice {
    /// Return the device's metadata (name, type, major/minor, etc.).
    fn info(&self) -> &DeviceInfo;
    /// Return a mutable reference to the device's metadata.
    fn info_mut(&mut self) -> &mut DeviceInfo;

    /// Read up to `count` bytes from the device.
    ///
    /// Returns the bytes that were available (may be fewer than `count`).
    /// Returns an empty Vec if no data is available (non-blocking).
    fn read(&mut self, count: usize) -> Vec<u8>;

    /// Write bytes to the device.
    ///
    /// Returns the number of bytes actually written, or -1 on error.
    /// For a display, this renders characters to the screen.
    /// For a serial port, this sends bytes over the wire.
    fn write(&mut self, data: &[u8]) -> isize;

    /// Initialize the device. Called once at boot.
    ///
    /// For a keyboard: clear the input buffer.
    /// For a display: clear the screen, set cursor to (0,0).
    fn init(&mut self);
}

// =========================================================================
// BlockDevice trait
// =========================================================================

/// BlockDevice — a device that reads and writes fixed-size blocks.
///
/// Block devices are random-access: you can read any block in any order,
/// like pulling drawers out of a filing cabinet. Every block is the same
/// size (typically 512 bytes, the standard sector size since the IBM PC/AT
/// in 1984).
///
/// Why whole blocks? Physical disks read whole sectors at a time. Even if
/// you only want 1 byte, the disk reads 512. The OS caches the extra bytes.
/// This is why filesystems exist — to manage partial-block reads/writes
/// efficiently.
pub trait BlockDevice {
    /// Return the device's metadata.
    fn info(&self) -> &DeviceInfo;
    /// Return a mutable reference to the device's metadata.
    fn info_mut(&mut self) -> &mut DeviceInfo;

    /// Return the number of bytes per block (typically 512).
    fn block_size(&self) -> usize;

    /// Return the total number of blocks on this device.
    fn total_blocks(&self) -> usize;

    /// Read one block from the device.
    ///
    /// Returns exactly `block_size()` bytes.
    /// Returns Err if block_number is out of range.
    fn read_block(&self, block_number: usize) -> Result<Vec<u8>, String>;

    /// Write one block to the device.
    ///
    /// `data` must be exactly `block_size()` bytes.
    /// Returns Err if block_number is out of range or data has wrong size.
    fn write_block(&mut self, block_number: usize, data: &[u8]) -> Result<(), String>;

    /// Initialize the device. Called once at boot.
    fn init(&mut self);
}

// =========================================================================
// NetworkDevice trait
// =========================================================================

/// NetworkDevice — a device that sends and receives variable-length packets.
///
/// Network devices deal in packets — discrete messages with headers,
/// addresses, and payloads. Unlike character devices (continuous byte streams)
/// or block devices (fixed-size chunks), network packets can be any size up
/// to the maximum transmission unit (MTU).
///
/// Every network device has a MAC address — a 6-byte unique identifier, like
/// a mailing address for the network card. In real hardware, this is burned
/// into the NIC at the factory. In simulation, we assign it at creation time.
pub trait NetworkDevice {
    /// Return the device's metadata.
    fn info(&self) -> &DeviceInfo;
    /// Return a mutable reference to the device's metadata.
    fn info_mut(&mut self) -> &mut DeviceInfo;

    /// Return the 6-byte MAC address of this device.
    fn mac_address(&self) -> &[u8; 6];

    /// Send a packet over the network.
    ///
    /// Returns the number of bytes sent, or -1 on error.
    fn send_packet(&mut self, data: &[u8]) -> isize;

    /// Receive the next packet from the network.
    ///
    /// Non-blocking: returns None immediately if no packet is available.
    fn receive_packet(&mut self) -> Option<Vec<u8>>;

    /// Check whether a packet is waiting to be received.
    fn has_packet(&self) -> bool;

    /// Initialize the device. Called once at boot.
    fn init(&mut self);
}

// =========================================================================
// DeviceEntry — a type-erased wrapper for registered devices
// =========================================================================

/// DeviceEntry wraps a device of any type for storage in the registry.
///
/// Rust's trait system requires us to know the concrete type at compile time,
/// but the registry needs to store devices of different types together. We
/// solve this with an enum that holds each device family in a separate variant.
pub enum DeviceEntry {
    /// A character device (keyboard, display, serial port).
    Char(Box<dyn CharacterDevice>),
    /// A block device (disk, SSD).
    Block(Box<dyn BlockDevice>),
    /// A network device (NIC).
    Network(Box<dyn NetworkDevice>),
}

impl DeviceEntry {
    /// Get the DeviceInfo for the wrapped device.
    pub fn info(&self) -> &DeviceInfo {
        match self {
            DeviceEntry::Char(d) => d.info(),
            DeviceEntry::Block(d) => d.info(),
            DeviceEntry::Network(d) => d.info(),
        }
    }
}

// =========================================================================
// DeviceRegistry — the kernel's phonebook for devices
// =========================================================================

/// DeviceRegistry — the kernel's phonebook for devices.
///
/// When a driver initializes a device, it registers it here. When the kernel
/// needs to perform I/O, it looks up the device here. Think of it as a
/// telephone directory: you look up a name ("disk0") or a number pair
/// (major=3, minor=0), and get back a reference to the actual device.
///
/// The registry maintains mappings for fast lookup:
/// - By name: `"disk0"` -> device index
/// - By (major, minor): `(3, 0)` -> device index
///
/// # Example
///
/// ```
/// use device_driver_framework::*;
///
/// let mut registry = DeviceRegistry::new();
/// let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
/// disk.init();
/// registry.register(DeviceEntry::Block(Box::new(disk))).unwrap();
///
/// assert!(registry.lookup_by_name("disk0").is_some());
/// ```
pub struct DeviceRegistry {
    /// All registered devices, stored in registration order.
    devices: Vec<DeviceEntry>,
    /// Maps device name to index in `devices`.
    name_index: HashMap<String, usize>,
    /// Maps (major, minor) to index in `devices`.
    major_minor_index: HashMap<(u32, u32), usize>,
}

impl DeviceRegistry {
    /// Create a new, empty registry.
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            name_index: HashMap::new(),
            major_minor_index: HashMap::new(),
        }
    }

    /// Register a device in the registry.
    ///
    /// The device must already be initialized. Registration fails if a device
    /// with the same name or the same (major, minor) pair already exists.
    pub fn register(&mut self, entry: DeviceEntry) -> Result<(), String> {
        let info = entry.info();

        if !info.initialized {
            return Err(format!("Device '{}' must be initialized before registration", info.name));
        }

        if self.name_index.contains_key(&info.name) {
            return Err(format!("Device with name '{}' is already registered", info.name));
        }

        let key = (info.major, info.minor);
        if self.major_minor_index.contains_key(&key) {
            return Err(format!(
                "Device with major={}, minor={} is already registered",
                info.major, info.minor
            ));
        }

        let index = self.devices.len();
        let name = info.name.clone();
        self.name_index.insert(name, index);
        self.major_minor_index.insert(key, index);
        self.devices.push(entry);

        Ok(())
    }

    /// Look up a device by its human-readable name.
    pub fn lookup_by_name(&self, name: &str) -> Option<&DeviceEntry> {
        self.name_index.get(name).map(|&idx| &self.devices[idx])
    }

    /// Look up a device by its human-readable name (mutable).
    pub fn lookup_by_name_mut(&mut self, name: &str) -> Option<&mut DeviceEntry> {
        self.name_index.get(name).copied().map(move |idx| &mut self.devices[idx])
    }

    /// Look up a device by its (major, minor) number pair.
    pub fn lookup_by_major_minor(&self, major: u32, minor: u32) -> Option<&DeviceEntry> {
        self.major_minor_index.get(&(major, minor)).map(|&idx| &self.devices[idx])
    }

    /// Return a slice of all registered devices.
    pub fn list_all(&self) -> &[DeviceEntry] {
        &self.devices
    }

    /// Return all devices of a specific type.
    pub fn list_by_type(&self, device_type: DeviceType) -> Vec<&DeviceEntry> {
        self.devices
            .iter()
            .filter(|entry| entry.info().device_type == device_type)
            .collect()
    }

    /// Return the number of registered devices.
    pub fn size(&self) -> usize {
        self.devices.len()
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// =========================================================================
// SimulatedDisk — in-memory block storage
// =========================================================================

/// SimulatedDisk — an in-memory block storage device.
///
/// This is the "hard drive" for our simulated computer. Instead of magnetic
/// platters spinning at 7200 RPM, we use a plain Vec<u8>. The interface is
/// identical to what a real disk driver would provide.
///
/// How it works:
/// - The disk is divided into fixed-size blocks (default 512 bytes each).
/// - A 1 MB disk has 2048 blocks (2048 * 512 = 1,048,576 bytes).
/// - Each block can be read or written independently (random access).
/// - Block N starts at byte offset N * block_size in the storage array.
///
/// # Example
///
/// ```
/// use device_driver_framework::*;
///
/// let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
/// disk.init();
///
/// let data = vec![0x42u8; 512];
/// disk.write_block(5, &data).unwrap();
/// assert_eq!(disk.read_block(5).unwrap(), data);
/// ```
pub struct SimulatedDisk {
    info: DeviceInfo,
    block_size_val: usize,
    total_blocks_val: usize,
    /// The backing store — a flat byte array. Block N occupies bytes
    /// [N*block_size .. (N+1)*block_size].
    storage: Vec<u8>,
}

impl SimulatedDisk {
    /// Create a new simulated disk.
    ///
    /// # Arguments
    /// * `name` - Device name (e.g., "disk0")
    /// * `minor` - Minor number (e.g., 0 for first disk)
    /// * `total_blocks` - Number of blocks (2048 = 1 MB with 512-byte blocks)
    /// * `block_size` - Bytes per block (typically 512)
    pub fn new(name: &str, minor: u32, total_blocks: usize, block_size: usize) -> Self {
        Self {
            info: DeviceInfo::new(name, DeviceType::Block, MAJOR_DISK, minor, Some(INT_DISK)),
            block_size_val: block_size,
            total_blocks_val: total_blocks,
            storage: Vec::new(), // Allocated in init()
        }
    }

    /// Validate that a block number is within range.
    fn validate_block_number(&self, block_number: usize) -> Result<(), String> {
        if block_number >= self.total_blocks_val {
            Err(format!(
                "Block number {} out of range (0..{})",
                block_number,
                self.total_blocks_val - 1
            ))
        } else {
            Ok(())
        }
    }
}

impl BlockDevice for SimulatedDisk {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn info_mut(&mut self) -> &mut DeviceInfo {
        &mut self.info
    }

    fn block_size(&self) -> usize {
        self.block_size_val
    }

    fn total_blocks(&self) -> usize {
        self.total_blocks_val
    }

    fn read_block(&self, block_number: usize) -> Result<Vec<u8>, String> {
        self.validate_block_number(block_number)?;
        let offset = block_number * self.block_size_val;
        Ok(self.storage[offset..offset + self.block_size_val].to_vec())
    }

    fn write_block(&mut self, block_number: usize, data: &[u8]) -> Result<(), String> {
        self.validate_block_number(block_number)?;

        if data.len() != self.block_size_val {
            return Err(format!(
                "Data must be exactly {} bytes, got {}",
                self.block_size_val,
                data.len()
            ));
        }

        let offset = block_number * self.block_size_val;
        self.storage[offset..offset + self.block_size_val].copy_from_slice(data);
        Ok(())
    }

    fn init(&mut self) {
        self.storage = vec![0u8; self.block_size_val * self.total_blocks_val];
        self.info.initialized = true;
    }
}

// =========================================================================
// SimulatedKeyboard — character device with internal buffer
// =========================================================================

/// SimulatedKeyboard — a character device backed by an internal byte buffer.
///
/// In a real computer, pressing a key sends a scan code to the keyboard
/// controller, which raises interrupt 33. The keyboard ISR reads the scan
/// code, translates it to ASCII, and deposits it into a buffer. When a
/// program calls read(), it gets bytes from that buffer.
///
/// Our SimulatedKeyboard lets you push bytes directly into the buffer
/// (via `enqueue_bytes`), simulating what the ISR would do. The read()
/// method then pulls bytes out in FIFO order.
///
/// Why is write() not supported? A keyboard is an input-only device.
/// Calling write() returns -1 to indicate an error.
///
/// # Example
///
/// ```
/// use device_driver_framework::*;
///
/// let mut kb = SimulatedKeyboard::new("keyboard0", 0);
/// kb.init();
/// kb.enqueue_bytes(&[0x48, 0x69]); // Simulate typing "Hi"
/// let data = kb.read(2);
/// assert_eq!(data, vec![0x48, 0x69]);
/// ```
pub struct SimulatedKeyboard {
    info: DeviceInfo,
    /// FIFO buffer of keystrokes, filled by the keyboard ISR (interrupt 33).
    buffer: VecDeque<u8>,
}

impl SimulatedKeyboard {
    /// Create a new simulated keyboard.
    pub fn new(name: &str, minor: u32) -> Self {
        Self {
            info: DeviceInfo::new(name, DeviceType::Character, MAJOR_KEYBOARD, minor, Some(INT_KEYBOARD)),
            buffer: VecDeque::new(),
        }
    }

    /// Simulate keystrokes by pushing bytes into the buffer.
    ///
    /// In a real system, the keyboard ISR does this when interrupt 33 fires.
    pub fn enqueue_bytes(&mut self, bytes: &[u8]) {
        self.buffer.extend(bytes);
    }

    /// Return the number of bytes waiting in the buffer.
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }
}

impl CharacterDevice for SimulatedKeyboard {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn info_mut(&mut self) -> &mut DeviceInfo {
        &mut self.info
    }

    fn read(&mut self, count: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            match self.buffer.pop_front() {
                Some(byte) => result.push(byte),
                None => break,
            }
        }
        result
    }

    fn write(&mut self, _data: &[u8]) -> isize {
        // Keyboards are input-only devices. Cannot write to them.
        -1
    }

    fn init(&mut self) {
        self.buffer.clear();
        self.info.initialized = true;
    }
}

// =========================================================================
// SimulatedDisplay — character device with framebuffer
// =========================================================================

/// SimulatedDisplay — a character device that renders bytes to a framebuffer.
///
/// The classic text-mode display is 80 columns by 25 rows, with 2 bytes per
/// character cell: one for the ASCII character, one for the color attribute.
/// That is 80 * 25 * 2 = 4000 bytes total.
///
/// Framebuffer layout (VGA text mode convention):
/// ```text
/// Byte 0: character at row 0, col 0
/// Byte 1: color attribute at row 0, col 0
/// Byte 2: character at row 0, col 1
/// Byte 3: color attribute at row 0, col 1
/// ...
/// ```
///
/// Why is read() not supported? A display is output-only through the
/// character device interface. Writing returns -1 as an error indicator.
///
/// # Example
///
/// ```
/// use device_driver_framework::*;
///
/// let mut display = SimulatedDisplay::new("display0", 0);
/// display.init();
/// display.write(&[0x48, 0x69]); // Write "Hi"
/// assert_eq!(display.char_at(0, 0), 0x48); // 'H'
/// assert_eq!(display.char_at(0, 1), 0x69); // 'i'
/// ```
pub struct SimulatedDisplay {
    info: DeviceInfo,
    /// The framebuffer: 80 * 25 * 2 = 4000 bytes of video memory.
    pub framebuffer: Vec<u8>,
    /// Current cursor row (0-24).
    pub cursor_row: usize,
    /// Current cursor column (0-79).
    pub cursor_col: usize,
}

/// Display dimensions and constants.
impl SimulatedDisplay {
    /// Number of columns in text mode.
    pub const COLS: usize = 80;
    /// Number of rows in text mode.
    pub const ROWS: usize = 25;
    /// Bytes per character cell (character + color attribute).
    pub const BYTES_PER_CELL: usize = 2;
    /// Total framebuffer size in bytes.
    pub const FRAMEBUFFER_SIZE: usize = Self::COLS * Self::ROWS * Self::BYTES_PER_CELL;
    /// Default color attribute: light gray on black (VGA standard).
    pub const DEFAULT_COLOR: u8 = 0x07;

    /// Create a new simulated display.
    pub fn new(name: &str, minor: u32) -> Self {
        Self {
            info: DeviceInfo::new(name, DeviceType::Character, MAJOR_DISPLAY, minor, None),
            framebuffer: vec![0u8; Self::FRAMEBUFFER_SIZE],
            cursor_row: 0,
            cursor_col: 0,
        }
    }

    /// Place a single character at the current cursor position.
    pub fn put_char(&mut self, byte: u8) {
        let offset = (self.cursor_row * Self::COLS + self.cursor_col) * Self::BYTES_PER_CELL;
        self.framebuffer[offset] = byte;
        self.framebuffer[offset + 1] = Self::DEFAULT_COLOR;
        self.advance_cursor();
    }

    /// Clear the entire screen and reset cursor to (0, 0).
    pub fn clear_screen(&mut self) {
        self.framebuffer.fill(0);
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    /// Read the character at a specific (row, col) position.
    pub fn char_at(&self, row: usize, col: usize) -> u8 {
        let offset = (row * Self::COLS + col) * Self::BYTES_PER_CELL;
        self.framebuffer[offset]
    }

    /// Advance the cursor by one position, wrapping at end of row and screen.
    fn advance_cursor(&mut self) {
        self.cursor_col += 1;
        if self.cursor_col >= Self::COLS {
            self.cursor_col = 0;
            self.cursor_row += 1;
            if self.cursor_row >= Self::ROWS {
                self.cursor_row = 0; // Wrap to top
            }
        }
    }
}

impl CharacterDevice for SimulatedDisplay {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn info_mut(&mut self) -> &mut DeviceInfo {
        &mut self.info
    }

    fn read(&mut self, _count: usize) -> Vec<u8> {
        // Displays are output-only. We return an empty vec and the caller
        // should check the return. In a Unix-like system, this would set
        // errno to ENOSYS.
        Vec::new()
    }

    fn write(&mut self, data: &[u8]) -> isize {
        for &byte in data {
            self.put_char(byte);
        }
        data.len() as isize
    }

    fn init(&mut self) {
        self.clear_screen();
        self.info.initialized = true;
    }
}

// =========================================================================
// SharedWire — simulated network cable
// =========================================================================

/// SharedWire — a simulated network cable connecting multiple NICs.
///
/// In a real network, devices are connected by physical cables (Ethernet) or
/// radio waves (WiFi). When one device sends a packet, it travels along the
/// medium and is received by other devices on the same segment.
///
/// Our SharedWire simulates this. It maintains a set of receive queues (one
/// per connected NIC). When one NIC sends a packet, it is delivered to every
/// other NIC's receive queue (but not back to the sender).
///
/// This is a simplified model of a shared medium (like early Ethernet hubs).
/// In a real network, you would also need to handle collisions, addressing,
/// and routing.
///
/// The SharedWire uses `Arc<Mutex<...>>` so that multiple NICs can share
/// access to it safely. Each NIC holds a reference to its own receive queue
/// (identified by an index).
pub struct SharedWire {
    /// One receive queue per connected NIC.
    queues: Vec<Arc<Mutex<VecDeque<Vec<u8>>>>>,
}

impl SharedWire {
    /// Create a new shared wire with no connected NICs.
    pub fn new() -> Self {
        Self { queues: Vec::new() }
    }

    /// Connect a new NIC and return its queue index and a reference to its
    /// receive queue.
    pub fn connect(&mut self) -> (usize, Arc<Mutex<VecDeque<Vec<u8>>>>) {
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let index = self.queues.len();
        self.queues.push(queue.clone());
        (index, queue)
    }

    /// Broadcast a packet to all connected NICs except the sender.
    ///
    /// The sender is identified by its queue index. All other queues receive
    /// a copy of the packet.
    pub fn broadcast(&self, data: &[u8], sender_index: usize) {
        for (i, queue) in self.queues.iter().enumerate() {
            if i != sender_index {
                let mut q = queue.lock().unwrap();
                q.push_back(data.to_vec());
            }
        }
    }

    /// Return the number of connected NICs.
    pub fn connected_count(&self) -> usize {
        self.queues.len()
    }
}

impl Default for SharedWire {
    fn default() -> Self {
        Self::new()
    }
}

// =========================================================================
// SimulatedNIC — network device with packet queues
// =========================================================================

/// SimulatedNIC — a network interface card backed by in-memory packet queues.
///
/// A NIC (Network Interface Card) connects a computer to a network. It sends
/// and receives packets — discrete chunks of data with headers and payloads.
///
/// Our SimulatedNIC uses a SharedWire to exchange packets with other NICs.
/// When you call `send_packet()`, the data is broadcast to all other NICs on
/// the same wire. When another NIC sends a packet, it appears in this NIC's
/// receive queue.
///
/// # Example
///
/// ```
/// use device_driver_framework::*;
///
/// let mut wire = SharedWire::new();
/// let (idx_a, rx_a) = wire.connect();
/// let (idx_b, rx_b) = wire.connect();
///
/// let mut nic_a = SimulatedNIC::new("nic0", 0, [0xAA; 6], idx_a, rx_a);
/// let mut nic_b = SimulatedNIC::new("nic1", 1, [0xBB; 6], idx_b, rx_b);
/// nic_a.init();
/// nic_b.init();
///
/// // We need the wire reference to send
/// wire.broadcast(&[1, 2, 3], idx_a); // A sends
/// assert!(nic_b.has_packet());
/// assert_eq!(nic_b.receive_packet(), Some(vec![1, 2, 3]));
/// ```
pub struct SimulatedNIC {
    info: DeviceInfo,
    mac: [u8; 6],
    /// Index of this NIC in the SharedWire's queue list.
    wire_index: usize,
    /// Reference to this NIC's receive queue (shared with the wire).
    rx_queue: Arc<Mutex<VecDeque<Vec<u8>>>>,
}

impl SimulatedNIC {
    /// Create a new simulated NIC.
    ///
    /// # Arguments
    /// * `name` - Device name (e.g., "nic0")
    /// * `minor` - Minor number
    /// * `mac_address` - 6-byte MAC address
    /// * `wire_index` - This NIC's index in the SharedWire
    /// * `rx_queue` - Shared reference to this NIC's receive queue
    pub fn new(
        name: &str,
        minor: u32,
        mac_address: [u8; 6],
        wire_index: usize,
        rx_queue: Arc<Mutex<VecDeque<Vec<u8>>>>,
    ) -> Self {
        Self {
            info: DeviceInfo::new(name, DeviceType::Network, MAJOR_NIC, minor, Some(INT_NIC)),
            mac: mac_address,
            wire_index,
            rx_queue,
        }
    }

    /// Return this NIC's index on the shared wire.
    ///
    /// Needed by the caller when broadcasting packets through the wire.
    pub fn wire_index(&self) -> usize {
        self.wire_index
    }
}

impl NetworkDevice for SimulatedNIC {
    fn info(&self) -> &DeviceInfo {
        &self.info
    }

    fn info_mut(&mut self) -> &mut DeviceInfo {
        &mut self.info
    }

    fn mac_address(&self) -> &[u8; 6] {
        &self.mac
    }

    fn send_packet(&mut self, _data: &[u8]) -> isize {
        // Note: In this design, the caller must use wire.broadcast() directly
        // because the NIC does not hold a mutable reference to the wire (that
        // would create a circular ownership problem in Rust). The caller does:
        //   wire.broadcast(data, nic.wire_index());
        //
        // This method exists to satisfy the trait. For direct use without the
        // wire, we return the data length to indicate "would have sent".
        _data.len() as isize
    }

    fn receive_packet(&mut self) -> Option<Vec<u8>> {
        let mut queue = self.rx_queue.lock().unwrap();
        queue.pop_front()
    }

    fn has_packet(&self) -> bool {
        let queue = self.rx_queue.lock().unwrap();
        !queue.is_empty()
    }

    fn init(&mut self) {
        let mut queue = self.rx_queue.lock().unwrap();
        queue.clear();
        drop(queue);
        self.info.initialized = true;
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- DeviceType tests ---

    #[test]
    fn test_device_type_values() {
        assert_eq!(DeviceType::Character as u8, 0);
        assert_eq!(DeviceType::Block as u8, 1);
        assert_eq!(DeviceType::Network as u8, 2);
    }

    #[test]
    fn test_device_type_display() {
        assert_eq!(format!("{}", DeviceType::Character), "Character");
        assert_eq!(format!("{}", DeviceType::Block), "Block");
        assert_eq!(format!("{}", DeviceType::Network), "Network");
    }

    #[test]
    fn test_device_type_equality() {
        assert_eq!(DeviceType::Character, DeviceType::Character);
        assert_ne!(DeviceType::Character, DeviceType::Block);
    }

    // --- DeviceInfo tests ---

    #[test]
    fn test_device_info_stores_all_fields() {
        let info = DeviceInfo::new("test0", DeviceType::Character, 1, 0, Some(33));
        assert_eq!(info.name, "test0");
        assert_eq!(info.device_type, DeviceType::Character);
        assert_eq!(info.major, 1);
        assert_eq!(info.minor, 0);
        assert_eq!(info.interrupt_number, Some(33));
        assert!(!info.initialized);
    }

    #[test]
    fn test_device_info_no_interrupt() {
        let info = DeviceInfo::new("display0", DeviceType::Character, 1, 0, None);
        assert_eq!(info.interrupt_number, None);
    }

    #[test]
    fn test_device_info_display() {
        let info = DeviceInfo::new("disk0", DeviceType::Block, 3, 0, Some(34));
        let s = format!("{}", info);
        assert!(s.contains("disk0"));
        assert!(s.contains("Block"));
        assert!(s.contains("major=3"));
    }

    // --- SimulatedDisk tests ---

    #[test]
    fn test_disk_default_properties() {
        let disk = SimulatedDisk::new("disk0", 0, 2048, 512);
        assert_eq!(disk.info().name, "disk0");
        assert_eq!(disk.info().device_type, DeviceType::Block);
        assert_eq!(disk.info().major, MAJOR_DISK);
        assert_eq!(disk.info().interrupt_number, Some(INT_DISK));
        assert_eq!(disk.block_size(), 512);
        assert_eq!(disk.total_blocks(), 2048);
    }

    #[test]
    fn test_disk_fresh_reads_zeros() {
        let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
        disk.init();
        let data = disk.read_block(0).unwrap();
        assert_eq!(data.len(), 512);
        assert!(data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_disk_write_then_read() {
        let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
        disk.init();

        let test_data: Vec<u8> = (0..512).map(|i| (i % 256) as u8).collect();
        disk.write_block(5, &test_data).unwrap();

        let result = disk.read_block(5).unwrap();
        assert_eq!(result, test_data);
    }

    #[test]
    fn test_disk_write_does_not_affect_other_blocks() {
        let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
        disk.init();

        disk.write_block(5, &vec![0xFF; 512]).unwrap();

        // Adjacent blocks should still be zeros
        let block4 = disk.read_block(4).unwrap();
        assert!(block4.iter().all(|&b| b == 0));
        let block6 = disk.read_block(6).unwrap();
        assert!(block6.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_disk_overwrite_block() {
        let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
        disk.init();

        disk.write_block(3, &vec![0xAA; 512]).unwrap();
        disk.write_block(3, &vec![0xBB; 512]).unwrap();

        let result = disk.read_block(3).unwrap();
        assert!(result.iter().all(|&b| b == 0xBB));
    }

    #[test]
    fn test_disk_read_last_block() {
        let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
        disk.init();
        let data = disk.read_block(63).unwrap();
        assert_eq!(data.len(), 512);
    }

    #[test]
    fn test_disk_read_out_of_bounds() {
        let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
        disk.init();
        assert!(disk.read_block(64).is_err());
    }

    #[test]
    fn test_disk_write_out_of_bounds() {
        let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
        disk.init();
        assert!(disk.write_block(64, &vec![0; 512]).is_err());
    }

    #[test]
    fn test_disk_write_wrong_size() {
        let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
        disk.init();
        assert!(disk.write_block(0, &vec![0; 100]).is_err());
        assert!(disk.write_block(0, &vec![0; 600]).is_err());
    }

    #[test]
    fn test_disk_custom_block_size() {
        let mut disk = SimulatedDisk::new("disk0", 0, 10, 1024);
        disk.init();
        assert_eq!(disk.block_size(), 1024);
        let data = disk.read_block(0).unwrap();
        assert_eq!(data.len(), 1024);
    }

    // --- SimulatedKeyboard tests ---

    #[test]
    fn test_keyboard_default_properties() {
        let kb = SimulatedKeyboard::new("keyboard0", 0);
        assert_eq!(kb.info().name, "keyboard0");
        assert_eq!(kb.info().device_type, DeviceType::Character);
        assert_eq!(kb.info().major, MAJOR_KEYBOARD);
        assert_eq!(kb.info().interrupt_number, Some(INT_KEYBOARD));
    }

    #[test]
    fn test_keyboard_read_empty_buffer() {
        let mut kb = SimulatedKeyboard::new("keyboard0", 0);
        kb.init();
        let result = kb.read(10);
        assert!(result.is_empty());
    }

    #[test]
    fn test_keyboard_enqueue_then_read() {
        let mut kb = SimulatedKeyboard::new("keyboard0", 0);
        kb.init();
        kb.enqueue_bytes(&[0x48, 0x69]); // "Hi"
        let result = kb.read(2);
        assert_eq!(result, vec![0x48, 0x69]);
    }

    #[test]
    fn test_keyboard_fifo_order() {
        let mut kb = SimulatedKeyboard::new("keyboard0", 0);
        kb.init();
        kb.enqueue_bytes(&[1, 2, 3, 4, 5]);
        let result = kb.read(5);
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_keyboard_read_fewer_than_available() {
        let mut kb = SimulatedKeyboard::new("keyboard0", 0);
        kb.init();
        kb.enqueue_bytes(&[1, 2, 3, 4, 5]);
        let result = kb.read(3);
        assert_eq!(result, vec![1, 2, 3]);
        let result2 = kb.read(2);
        assert_eq!(result2, vec![4, 5]);
    }

    #[test]
    fn test_keyboard_read_more_than_available() {
        let mut kb = SimulatedKeyboard::new("keyboard0", 0);
        kb.init();
        kb.enqueue_bytes(&[0x41, 0x42, 0x43]);
        let result = kb.read(10);
        assert_eq!(result, vec![0x41, 0x42, 0x43]);
    }

    #[test]
    fn test_keyboard_write_returns_error() {
        let mut kb = SimulatedKeyboard::new("keyboard0", 0);
        kb.init();
        assert_eq!(kb.write(&[0x48, 0x69]), -1);
    }

    #[test]
    fn test_keyboard_buffer_size() {
        let mut kb = SimulatedKeyboard::new("keyboard0", 0);
        kb.init();
        assert_eq!(kb.buffer_size(), 0);
        kb.enqueue_bytes(&[1, 2, 3]);
        assert_eq!(kb.buffer_size(), 3);
        kb.read(1);
        assert_eq!(kb.buffer_size(), 2);
    }

    #[test]
    fn test_keyboard_init_clears_buffer() {
        let mut kb = SimulatedKeyboard::new("keyboard0", 0);
        kb.init();
        kb.enqueue_bytes(&[1, 2, 3]);
        kb.init();
        assert_eq!(kb.buffer_size(), 0);
    }

    // --- SimulatedDisplay tests ---

    #[test]
    fn test_display_default_properties() {
        let display = SimulatedDisplay::new("display0", 0);
        assert_eq!(display.info().name, "display0");
        assert_eq!(display.info().device_type, DeviceType::Character);
        assert_eq!(display.info().major, MAJOR_DISPLAY);
        assert_eq!(display.info().interrupt_number, None);
    }

    #[test]
    fn test_display_framebuffer_size() {
        let mut display = SimulatedDisplay::new("display0", 0);
        display.init();
        assert_eq!(display.framebuffer.len(), 4000);
    }

    #[test]
    fn test_display_framebuffer_starts_zeroed() {
        let mut display = SimulatedDisplay::new("display0", 0);
        display.init();
        assert!(display.framebuffer.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_display_write_single_character() {
        let mut display = SimulatedDisplay::new("display0", 0);
        display.init();
        display.write(&[0x48]); // 'H'
        assert_eq!(display.char_at(0, 0), 0x48);
        assert_eq!(display.framebuffer[1], SimulatedDisplay::DEFAULT_COLOR);
    }

    #[test]
    fn test_display_write_two_characters() {
        let mut display = SimulatedDisplay::new("display0", 0);
        display.init();
        display.write(&[0x48, 0x69]); // "Hi"
        assert_eq!(display.char_at(0, 0), 0x48);
        assert_eq!(display.char_at(0, 1), 0x69);
    }

    #[test]
    fn test_display_write_returns_count() {
        let mut display = SimulatedDisplay::new("display0", 0);
        display.init();
        let result = display.write(&[0x48, 0x69]);
        assert_eq!(result, 2);
    }

    #[test]
    fn test_display_read_returns_empty() {
        let mut display = SimulatedDisplay::new("display0", 0);
        display.init();
        let result = display.read(10);
        assert!(result.is_empty());
    }

    #[test]
    fn test_display_cursor_starts_at_origin() {
        let mut display = SimulatedDisplay::new("display0", 0);
        display.init();
        assert_eq!(display.cursor_row, 0);
        assert_eq!(display.cursor_col, 0);
    }

    #[test]
    fn test_display_cursor_advances() {
        let mut display = SimulatedDisplay::new("display0", 0);
        display.init();
        display.write(&[0x41]); // 'A'
        assert_eq!(display.cursor_row, 0);
        assert_eq!(display.cursor_col, 1);
    }

    #[test]
    fn test_display_cursor_wraps_at_end_of_row() {
        let mut display = SimulatedDisplay::new("display0", 0);
        display.init();
        display.write(&vec![0x41; 80]); // Fill row 0
        assert_eq!(display.cursor_row, 1);
        assert_eq!(display.cursor_col, 0);
    }

    #[test]
    fn test_display_cursor_wraps_at_bottom() {
        let mut display = SimulatedDisplay::new("display0", 0);
        display.init();
        display.write(&vec![0x41; 80 * 25]); // Fill entire screen
        assert_eq!(display.cursor_row, 0);
        assert_eq!(display.cursor_col, 0);
    }

    #[test]
    fn test_display_clear_screen() {
        let mut display = SimulatedDisplay::new("display0", 0);
        display.init();
        display.write(&[0x48, 0x69]);
        display.clear_screen();
        assert!(display.framebuffer.iter().all(|&b| b == 0));
        assert_eq!(display.cursor_row, 0);
        assert_eq!(display.cursor_col, 0);
    }

    #[test]
    fn test_display_init_clears_screen() {
        let mut display = SimulatedDisplay::new("display0", 0);
        display.put_char(0x41);
        display.init();
        assert_eq!(display.cursor_row, 0);
        assert_eq!(display.cursor_col, 0);
        assert_eq!(display.char_at(0, 0), 0);
    }

    // --- SharedWire tests ---

    #[test]
    fn test_shared_wire_connect() {
        let mut wire = SharedWire::new();
        assert_eq!(wire.connected_count(), 0);
        let (idx, _) = wire.connect();
        assert_eq!(idx, 0);
        assert_eq!(wire.connected_count(), 1);
    }

    #[test]
    fn test_shared_wire_broadcast_delivers_to_others() {
        let mut wire = SharedWire::new();
        let (idx_a, _rx_a) = wire.connect();
        let (_idx_b, rx_b) = wire.connect();

        wire.broadcast(&[1, 2, 3], idx_a);

        let queue_b = rx_b.lock().unwrap();
        assert_eq!(queue_b.len(), 1);
        assert_eq!(queue_b[0], vec![1, 2, 3]);

        // Sender should NOT have received
        let queue_a = _rx_a.lock().unwrap();
        assert!(queue_a.is_empty());
    }

    // --- SimulatedNIC tests ---

    #[test]
    fn test_nic_default_properties() {
        let mut wire = SharedWire::new();
        let (idx, rx) = wire.connect();
        let nic = SimulatedNIC::new("nic0", 0, [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01], idx, rx);
        assert_eq!(nic.info().name, "nic0");
        assert_eq!(nic.info().device_type, DeviceType::Network);
        assert_eq!(nic.info().major, MAJOR_NIC);
        assert_eq!(nic.info().interrupt_number, Some(INT_NIC));
        assert_eq!(nic.mac_address(), &[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01]);
    }

    #[test]
    fn test_nic_mac_address_is_6_bytes() {
        let mut wire = SharedWire::new();
        let (idx, rx) = wire.connect();
        let nic = SimulatedNIC::new("nic0", 0, [0xAA; 6], idx, rx);
        assert_eq!(nic.mac_address().len(), 6);
    }

    #[test]
    fn test_nic_receive_empty_returns_none() {
        let mut wire = SharedWire::new();
        let (idx, rx) = wire.connect();
        let mut nic = SimulatedNIC::new("nic0", 0, [0xAA; 6], idx, rx);
        nic.init();
        assert_eq!(nic.receive_packet(), None);
    }

    #[test]
    fn test_nic_has_packet_false_when_empty() {
        let mut wire = SharedWire::new();
        let (idx, rx) = wire.connect();
        let mut nic = SimulatedNIC::new("nic0", 0, [0xAA; 6], idx, rx);
        nic.init();
        assert!(!nic.has_packet());
    }

    #[test]
    fn test_nic_send_and_receive_via_wire() {
        let mut wire = SharedWire::new();
        let (idx_a, rx_a) = wire.connect();
        let (idx_b, rx_b) = wire.connect();

        let mut nic_a = SimulatedNIC::new("nic0", 0, [0xAA; 6], idx_a, rx_a);
        let mut nic_b = SimulatedNIC::new("nic1", 1, [0xBB; 6], idx_b, rx_b);
        nic_a.init();
        nic_b.init();

        // A sends via the wire
        wire.broadcast(&[1, 2, 3, 4, 5], nic_a.wire_index());

        // B should receive
        assert!(nic_b.has_packet());
        assert_eq!(nic_b.receive_packet(), Some(vec![1, 2, 3, 4, 5]));

        // A should NOT receive its own packet
        assert!(!nic_a.has_packet());
    }

    #[test]
    fn test_nic_fifo_ordering() {
        let mut wire = SharedWire::new();
        let (idx_a, _rx_a) = wire.connect();
        let (_idx_b, rx_b) = wire.connect();

        let mut nic_b = SimulatedNIC::new("nic1", 1, [0xBB; 6], _idx_b, rx_b);
        nic_b.init();

        wire.broadcast(&[1], idx_a);
        wire.broadcast(&[2], idx_a);
        wire.broadcast(&[3], idx_a);

        assert_eq!(nic_b.receive_packet(), Some(vec![1]));
        assert_eq!(nic_b.receive_packet(), Some(vec![2]));
        assert_eq!(nic_b.receive_packet(), Some(vec![3]));
    }

    #[test]
    fn test_nic_bidirectional() {
        let mut wire = SharedWire::new();
        let (idx_a, rx_a) = wire.connect();
        let (idx_b, rx_b) = wire.connect();

        let mut nic_a = SimulatedNIC::new("nic0", 0, [0xAA; 6], idx_a, rx_a);
        let mut nic_b = SimulatedNIC::new("nic1", 1, [0xBB; 6], idx_b, rx_b);
        nic_a.init();
        nic_b.init();

        wire.broadcast(&[0xAA], idx_a); // A sends
        wire.broadcast(&[0xBB], idx_b); // B sends

        assert_eq!(nic_a.receive_packet(), Some(vec![0xBB])); // A gets B's packet
        assert_eq!(nic_b.receive_packet(), Some(vec![0xAA])); // B gets A's packet
    }

    #[test]
    fn test_nic_broadcast_to_multiple() {
        let mut wire = SharedWire::new();
        let (idx_a, _rx_a) = wire.connect();
        let (_idx_b, rx_b) = wire.connect();
        let (_idx_c, rx_c) = wire.connect();

        let mut nic_b = SimulatedNIC::new("nic1", 1, [0xBB; 6], _idx_b, rx_b);
        let mut nic_c = SimulatedNIC::new("nic2", 2, [0xCC; 6], _idx_c, rx_c);
        nic_b.init();
        nic_c.init();

        wire.broadcast(&[42], idx_a);

        assert_eq!(nic_b.receive_packet(), Some(vec![42]));
        assert_eq!(nic_c.receive_packet(), Some(vec![42]));

        // Sender should not receive
        let mut nic_a = SimulatedNIC::new("nic0", 0, [0xAA; 6], idx_a, _rx_a);
        nic_a.init(); // This clears the queue
        assert!(!nic_a.has_packet());
    }

    #[test]
    fn test_nic_init_clears_queue() {
        let mut wire = SharedWire::new();
        let (idx_a, _rx_a) = wire.connect();
        let (idx_b, rx_b) = wire.connect();

        let mut nic_b = SimulatedNIC::new("nic1", 1, [0xBB; 6], idx_b, rx_b);
        nic_b.init();

        wire.broadcast(&[1, 2, 3], idx_a);
        assert!(nic_b.has_packet());

        nic_b.init(); // Should clear queue
        assert!(!nic_b.has_packet());
    }

    #[test]
    fn test_nic_has_packet_after_drain() {
        let mut wire = SharedWire::new();
        let (idx_a, _rx_a) = wire.connect();
        let (_idx_b, rx_b) = wire.connect();

        let mut nic_b = SimulatedNIC::new("nic1", 1, [0xBB; 6], _idx_b, rx_b);
        nic_b.init();

        wire.broadcast(&[1], idx_a);
        assert!(nic_b.has_packet());
        nic_b.receive_packet();
        assert!(!nic_b.has_packet());
    }

    #[test]
    fn test_nic_send_packet_returns_length() {
        let mut wire = SharedWire::new();
        let (idx, rx) = wire.connect();
        let mut nic = SimulatedNIC::new("nic0", 0, [0xAA; 6], idx, rx);
        nic.init();
        assert_eq!(nic.send_packet(&[1, 2, 3]), 3);
    }

    // --- DeviceRegistry tests ---

    #[test]
    fn test_registry_register_and_lookup_by_name() {
        let mut registry = DeviceRegistry::new();
        let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
        disk.init();
        registry.register(DeviceEntry::Block(Box::new(disk))).unwrap();

        let found = registry.lookup_by_name("disk0");
        assert!(found.is_some());
        assert_eq!(found.unwrap().info().name, "disk0");
    }

    #[test]
    fn test_registry_register_and_lookup_by_major_minor() {
        let mut registry = DeviceRegistry::new();
        let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
        disk.init();
        registry.register(DeviceEntry::Block(Box::new(disk))).unwrap();

        let found = registry.lookup_by_major_minor(MAJOR_DISK, 0);
        assert!(found.is_some());
        assert_eq!(found.unwrap().info().name, "disk0");
    }

    #[test]
    fn test_registry_requires_initialized() {
        let mut registry = DeviceRegistry::new();
        let disk = SimulatedDisk::new("disk0", 0, 64, 512);
        // Not initialized!
        let result = registry.register(DeviceEntry::Block(Box::new(disk)));
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_duplicate_name_error() {
        let mut registry = DeviceRegistry::new();
        let mut disk1 = SimulatedDisk::new("disk0", 0, 64, 512);
        disk1.init();
        registry.register(DeviceEntry::Block(Box::new(disk1))).unwrap();

        let mut disk2 = SimulatedDisk::new("disk0", 1, 64, 512);
        disk2.init();
        let result = registry.register(DeviceEntry::Block(Box::new(disk2)));
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_duplicate_major_minor_error() {
        let mut registry = DeviceRegistry::new();
        let mut disk1 = SimulatedDisk::new("disk0", 0, 64, 512);
        disk1.init();
        registry.register(DeviceEntry::Block(Box::new(disk1))).unwrap();

        let mut disk2 = SimulatedDisk::new("disk1", 0, 64, 512);
        disk2.init();
        let result = registry.register(DeviceEntry::Block(Box::new(disk2)));
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_lookup_missing_name() {
        let registry = DeviceRegistry::new();
        assert!(registry.lookup_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_registry_lookup_missing_major_minor() {
        let registry = DeviceRegistry::new();
        assert!(registry.lookup_by_major_minor(99, 99).is_none());
    }

    #[test]
    fn test_registry_list_all() {
        let mut registry = DeviceRegistry::new();

        let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
        disk.init();
        registry.register(DeviceEntry::Block(Box::new(disk))).unwrap();

        let mut kb = SimulatedKeyboard::new("keyboard0", 0);
        kb.init();
        registry.register(DeviceEntry::Char(Box::new(kb))).unwrap();

        assert_eq!(registry.list_all().len(), 2);
    }

    #[test]
    fn test_registry_list_by_type() {
        let mut registry = DeviceRegistry::new();

        let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
        disk.init();
        registry.register(DeviceEntry::Block(Box::new(disk))).unwrap();

        let mut kb = SimulatedKeyboard::new("keyboard0", 0);
        kb.init();
        registry.register(DeviceEntry::Char(Box::new(kb))).unwrap();

        let mut display = SimulatedDisplay::new("display0", 0);
        display.init();
        registry.register(DeviceEntry::Char(Box::new(display))).unwrap();

        assert_eq!(registry.list_by_type(DeviceType::Block).len(), 1);
        assert_eq!(registry.list_by_type(DeviceType::Character).len(), 2);
        assert_eq!(registry.list_by_type(DeviceType::Network).len(), 0);
    }

    #[test]
    fn test_registry_size() {
        let mut registry = DeviceRegistry::new();
        assert_eq!(registry.size(), 0);

        let mut disk = SimulatedDisk::new("disk0", 0, 64, 512);
        disk.init();
        registry.register(DeviceEntry::Block(Box::new(disk))).unwrap();
        assert_eq!(registry.size(), 1);
    }

    #[test]
    fn test_registry_lookup_by_name_mut() {
        let mut registry = DeviceRegistry::new();
        let mut kb = SimulatedKeyboard::new("keyboard0", 0);
        kb.init();
        registry.register(DeviceEntry::Char(Box::new(kb))).unwrap();

        let entry = registry.lookup_by_name_mut("keyboard0");
        assert!(entry.is_some());
    }

    #[test]
    fn test_registry_default() {
        let registry = DeviceRegistry::default();
        assert_eq!(registry.size(), 0);
    }

    #[test]
    fn test_shared_wire_default() {
        let wire = SharedWire::default();
        assert_eq!(wire.connected_count(), 0);
    }

    // --- Integration: full boot sequence ---

    #[test]
    fn test_full_boot_sequence() {
        let mut registry = DeviceRegistry::new();

        // 1. Display
        let mut display = SimulatedDisplay::new("display0", 0);
        display.init();
        registry.register(DeviceEntry::Char(Box::new(display))).unwrap();

        // 2. Keyboard
        let mut kb = SimulatedKeyboard::new("keyboard0", 0);
        kb.init();
        registry.register(DeviceEntry::Char(Box::new(kb))).unwrap();

        // 3. Disk
        let mut disk = SimulatedDisk::new("disk0", 0, 2048, 512);
        disk.init();
        registry.register(DeviceEntry::Block(Box::new(disk))).unwrap();

        // 4. NIC
        let mut wire = SharedWire::new();
        let (idx, rx) = wire.connect();
        let mut nic = SimulatedNIC::new("nic0", 0, [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01], idx, rx);
        nic.init();
        registry.register(DeviceEntry::Network(Box::new(nic))).unwrap();

        // Verify
        assert_eq!(registry.size(), 4);
        assert!(registry.lookup_by_name("display0").is_some());
        assert!(registry.lookup_by_name("keyboard0").is_some());
        assert!(registry.lookup_by_name("disk0").is_some());
        assert!(registry.lookup_by_name("nic0").is_some());

        assert_eq!(registry.list_by_type(DeviceType::Character).len(), 2);
        assert_eq!(registry.list_by_type(DeviceType::Block).len(), 1);
        assert_eq!(registry.list_by_type(DeviceType::Network).len(), 1);
    }
}
