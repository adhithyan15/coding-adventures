/**
 * Device Driver Framework
 * =======================
 *
 * A device driver is a piece of software that knows how to talk to a specific
 * piece of hardware. Without drivers, every program that wanted to read from a
 * disk would need to know the exact protocol for that specific disk model —
 * the register addresses, the timing requirements, the error codes.
 *
 * Device drivers solve this by providing a **uniform interface** over diverse
 * hardware. A program says "read 512 bytes from block 7" and the driver
 * translates that into whatever specific commands the hardware needs.
 *
 * **Analogy:** Think of a universal remote control. You press "Volume Up" and
 * it works on your Samsung TV, your Sony soundbar, and your LG projector. Each
 * device speaks a different infrared protocol, but the remote translates your
 * single button press into the right signal for each device. Device drivers are
 * the universal remote for your operating system.
 *
 * This module implements three device families:
 *   1. CharacterDevice — byte streams (keyboard, serial, display)
 *   2. BlockDevice — fixed-size blocks (disk)
 *   3. NetworkDevice — packets (NIC)
 *
 * Plus a DeviceRegistry for registering and looking up devices by name or
 * by major/minor number, and concrete simulated implementations of each
 * device type.
 */

// ============================================================================
// DeviceType Enum
// ============================================================================
//
// Not all hardware behaves the same way. A keyboard produces one byte at a
// time, whenever the user presses a key. A disk reads and writes fixed-size
// chunks. A network card sends and receives variable-length packets. Rather
// than force all three into one interface, operating systems classify devices
// into families — each with an interface matching how the hardware naturally
// operates.
//
//   Device Type       Data Model            Examples
//   ─────────────────────────────────────────────────────────
//   Character         Stream of bytes       Keyboard, serial port, display
//   Block             Fixed-size chunks     Hard disk, SSD, USB drive
//   Network           Variable-length       Ethernet NIC, WiFi adapter
//                     packets

export enum DeviceType {
  /** Byte-stream devices like keyboards and displays. */
  CHARACTER = 0,

  /** Fixed-size block devices like disks. */
  BLOCK = 1,

  /** Packet-oriented devices like network cards. */
  NETWORK = 2,
}

// ============================================================================
// Device Interfaces
// ============================================================================
//
// Each device family gets its own interface. This is the contract that every
// driver must fulfill. The kernel never sees the hardware-specific details —
// it only sees these interfaces.

/**
 * Base fields common to every device, regardless of type.
 *
 * Every device has:
 *   - name:             Human-readable name (e.g., "disk0") for lookup/logging
 *   - deviceType:       Which family (Character, Block, Network)
 *   - major:            Driver identifier — all devices of the same driver share this
 *   - minor:            Instance number within the driver (first disk = 0, second = 1)
 *   - interruptNumber:  Which interrupt this device raises (-1 if none)
 *   - initialized:      Has init() been called? Guards against double-init.
 *
 * In Unix, major/minor numbers let the kernel route I/O requests:
 *   Major 3 = disk driver   → Minor 0 = first disk, Minor 1 = second disk
 *   Major 4 = NIC driver    → Minor 0 = first NIC, Minor 1 = second NIC
 */
export interface DeviceBase {
  readonly name: string;
  readonly deviceType: DeviceType;
  readonly major: number;
  readonly minor: number;
  readonly interruptNumber: number;
  initialized: boolean;
}

/**
 * CharacterDevice — devices that produce or consume a stream of bytes.
 *
 * Think of water flowing through a pipe: bytes come in order, one at a time.
 * You cannot "seek" to byte 47 of a keyboard — that makes no sense. Data
 * arrives when it arrives.
 *
 * Examples: keyboard (read bytes), display (write bytes), serial port (both).
 */
export interface CharacterDevice extends DeviceBase {
  /**
   * Read up to `count` bytes from the device.
   * Returns a Uint8Array of the bytes actually read.
   * Returns an empty array if no data is available (non-blocking).
   *
   * Why return fewer bytes? The device might have fewer bytes available
   * than you asked for. A keyboard might have only 3 keystrokes buffered
   * when you asked for 10.
   */
  read(count: number): Uint8Array;

  /**
   * Write bytes to the device.
   * Returns the number of bytes actually written, or -1 on error.
   *
   * For a display, this renders characters on screen.
   * For a serial port, this sends bytes over the wire.
   */
  write(data: Uint8Array): number;

  /** Initialize the device. Called once at boot. */
  init(): void;
}

/**
 * BlockDevice — devices that read and write fixed-size chunks.
 *
 * Think of a filing cabinet: each drawer is numbered, each holds exactly the
 * same amount of paper, and you can open any drawer in any order. You cannot
 * read half a drawer — the hardware always reads the entire block (sector).
 *
 * The standard block size is 512 bytes, dating back to the IBM PC/AT in 1984.
 * Modern disks use 4096, but 512 is simpler and traditional.
 */
export interface BlockDevice extends DeviceBase {
  /** Bytes per block. Traditionally 512. */
  readonly blockSize: number;

  /** Total number of blocks on this device. */
  readonly totalBlocks: number;

  /**
   * Read exactly `blockSize` bytes from the given block number.
   * Throws if blockNumber >= totalBlocks.
   *
   * Why whole blocks? Disks physically read whole sectors at a time.
   * Even if you only want 1 byte, the disk reads 512. The OS caches
   * the extra bytes for later.
   */
  readBlock(blockNumber: number): Uint8Array;

  /**
   * Write exactly `blockSize` bytes to the given block number.
   * The data must be exactly `blockSize` bytes long.
   * Throws if blockNumber >= totalBlocks or data length != blockSize.
   */
  writeBlock(blockNumber: number, data: Uint8Array): void;

  /** Initialize the device. Called once at boot. */
  init(): void;
}

/**
 * NetworkDevice — devices that send and receive variable-length packets.
 *
 * Think of a mailbox: you send and receive discrete envelopes (packets), each
 * with an address (MAC address) and contents (payload). You do not read
 * "byte 5 of the network" — you send and receive complete packets.
 *
 * Each NIC has a MAC address — a 6-byte unique identifier, like a mailing
 * address burned into the network card at the factory. In simulation, we
 * assign it at creation time.
 */
export interface NetworkDevice extends DeviceBase {
  /** 6-byte Media Access Control address. Unique per NIC. */
  readonly macAddress: Uint8Array;

  /**
   * Send a packet over the network.
   * Returns the number of bytes sent, or -1 on error.
   */
  sendPacket(data: Uint8Array): number;

  /**
   * Receive the next packet from the network.
   * Returns the packet data, or null if no packet is available.
   * This is non-blocking.
   */
  receivePacket(): Uint8Array | null;

  /** Returns true if there is at least one packet waiting. */
  hasPacket(): boolean;

  /** Initialize the device. Called once at boot. */
  init(): void;
}

// ============================================================================
// Type guard helpers
// ============================================================================
//
// TypeScript doesn't know at runtime which interface an object implements.
// These type guards let us safely narrow a Device to a specific sub-type.

/** A Device is any of the three device families. */
export type Device = CharacterDevice | BlockDevice | NetworkDevice;

/** Type guard: is this device a CharacterDevice? */
export function isCharacterDevice(d: Device): d is CharacterDevice {
  return d.deviceType === DeviceType.CHARACTER;
}

/** Type guard: is this device a BlockDevice? */
export function isBlockDevice(d: Device): d is BlockDevice {
  return d.deviceType === DeviceType.BLOCK;
}

/** Type guard: is this device a NetworkDevice? */
export function isNetworkDevice(d: Device): d is NetworkDevice {
  return d.deviceType === DeviceType.NETWORK;
}

// ============================================================================
// DeviceRegistry
// ============================================================================
//
// The registry is the kernel's phonebook for devices. When a driver initializes
// a device, it registers it here. When the kernel needs to perform I/O, it
// looks up the device here.
//
// Two lookup strategies:
//   1. By name: "disk0" → SimulatedDisk instance (for human use)
//   2. By major/minor: (3, 0) → SimulatedDisk instance (for kernel routing)
//
// Both must be unique — registering a duplicate name or duplicate (major,minor)
// is an error.

export class DeviceRegistry {
  /** Fast lookup by name: "disk0" → Device. */
  private devicesByName: Map<string, Device> = new Map();

  /** Fast lookup by major/minor: "3:0" → Device. */
  private devicesByMajorMinor: Map<string, Device> = new Map();

  /** Ordered list of all registered devices. */
  private allDevices: Device[] = [];

  /**
   * Register a device in the registry.
   *
   * The device must have been initialized (init() called) before registration.
   * Registration fails if:
   *   - A device with the same name already exists
   *   - A device with the same (major, minor) pair already exists
   */
  register(device: Device): void {
    if (!device.initialized) {
      throw new Error(
        `Device "${device.name}" must be initialized before registration`
      );
    }
    if (this.devicesByName.has(device.name)) {
      throw new Error(
        `Device with name "${device.name}" is already registered`
      );
    }
    const key = `${device.major}:${device.minor}`;
    if (this.devicesByMajorMinor.has(key)) {
      throw new Error(
        `Device with major=${device.major}, minor=${device.minor} is already registered`
      );
    }

    this.devicesByName.set(device.name, device);
    this.devicesByMajorMinor.set(key, device);
    this.allDevices.push(device);
  }

  /**
   * Remove a device from the registry by name.
   * Returns true if the device was found and removed, false otherwise.
   */
  unregister(name: string): boolean {
    const device = this.devicesByName.get(name);
    if (!device) {
      return false;
    }
    this.devicesByName.delete(name);
    this.devicesByMajorMinor.delete(`${device.major}:${device.minor}`);
    this.allDevices = this.allDevices.filter((d) => d.name !== name);
    return true;
  }

  /**
   * Look up a device by its human-readable name.
   * Returns null if not found.
   */
  lookupByName(name: string): Device | null {
    return this.devicesByName.get(name) ?? null;
  }

  /**
   * Look up a device by its major/minor number pair.
   * Returns null if not found.
   *
   * The kernel uses this when routing I/O: the file descriptor table maps
   * fd numbers to (major, minor) pairs, and the registry maps those pairs
   * to device instances.
   */
  lookupByMajorMinor(major: number, minor: number): Device | null {
    return this.devicesByMajorMinor.get(`${major}:${minor}`) ?? null;
  }

  /** Return all registered devices, in registration order. */
  listAll(): Device[] {
    return [...this.allDevices];
  }

  /**
   * Return all devices of a specific type.
   *
   * Useful for enumeration: "list all block devices" to show available disks,
   * or "list all network devices" to show available NICs.
   */
  listByType(deviceType: DeviceType): Device[] {
    return this.allDevices.filter((d) => d.deviceType === deviceType);
  }
}

// ============================================================================
// SimulatedDisk (BlockDevice)
// ============================================================================
//
// Wraps an in-memory byte array to simulate a block storage device. This is
// the "hard drive" for our simulated computer.
//
// How it works:
//   - The disk is a flat array of bytes, divided into fixed-size blocks.
//   - read_block(n) reads bytes from offset n*blockSize to n*blockSize+blockSize.
//   - write_block(n, data) writes exactly blockSize bytes at that offset.
//
// Default configuration:
//   - block_size = 512 bytes (standard sector size since IBM PC/AT, 1984)
//   - total_blocks = 2048 (giving a 1 MB disk: 2048 * 512 = 1,048,576 bytes)
//   - interrupt_number = 34 (disk I/O complete)
//
// In a real system, disk reads take milliseconds (the read head must
// physically move). In simulation, it's a simple array index — instantaneous.
// The interrupt mechanism is included for educational completeness.

export class SimulatedDisk implements BlockDevice {
  readonly name: string;
  readonly deviceType = DeviceType.BLOCK;
  readonly major: number;
  readonly minor: number;
  readonly interruptNumber: number;
  readonly blockSize: number;
  readonly totalBlocks: number;
  initialized: boolean;

  /** The backing store — a flat byte array representing the entire disk. */
  private storage: Uint8Array;

  constructor(
    options: {
      name?: string;
      major?: number;
      minor?: number;
      interruptNumber?: number;
      blockSize?: number;
      totalBlocks?: number;
    } = {}
  ) {
    this.name = options.name ?? "disk0";
    this.major = options.major ?? 3;
    this.minor = options.minor ?? 0;
    this.interruptNumber = options.interruptNumber ?? 34;
    this.blockSize = options.blockSize ?? 512;
    this.totalBlocks = options.totalBlocks ?? 2048;
    this.initialized = false;
    // Allocate the backing store — all zeros, like a freshly formatted disk.
    this.storage = new Uint8Array(this.blockSize * this.totalBlocks);
  }

  /**
   * Initialize the disk. In a real system, this would detect the hardware,
   * read the partition table, and configure DMA channels. For us, it just
   * marks the device as ready.
   */
  init(): void {
    this.initialized = true;
  }

  /**
   * Read exactly `blockSize` bytes from the given block number.
   *
   * The offset calculation is straightforward:
   *   offset = blockNumber * blockSize
   *   data   = storage[offset .. offset + blockSize]
   *
   * Returns a copy of the data (not a reference into the backing store),
   * just like a real disk controller would DMA data into a separate buffer.
   */
  readBlock(blockNumber: number): Uint8Array {
    if (blockNumber < 0 || blockNumber >= this.totalBlocks) {
      throw new Error(
        `Block number ${blockNumber} out of range [0, ${this.totalBlocks})`
      );
    }
    const offset = blockNumber * this.blockSize;
    return new Uint8Array(this.storage.slice(offset, offset + this.blockSize));
  }

  /**
   * Write exactly `blockSize` bytes to the given block number.
   *
   * The data must be exactly `blockSize` bytes — you cannot write a partial
   * block to a disk. The filesystem layer above us handles the bookkeeping
   * of reading a block, modifying part of it, and writing it back.
   */
  writeBlock(blockNumber: number, data: Uint8Array): void {
    if (blockNumber < 0 || blockNumber >= this.totalBlocks) {
      throw new Error(
        `Block number ${blockNumber} out of range [0, ${this.totalBlocks})`
      );
    }
    if (data.length !== this.blockSize) {
      throw new Error(
        `Data length ${data.length} does not match block size ${this.blockSize}`
      );
    }
    const offset = blockNumber * this.blockSize;
    this.storage.set(data, offset);
  }
}

// ============================================================================
// SimulatedKeyboard (CharacterDevice)
// ============================================================================
//
// Wraps a FIFO buffer to simulate a keyboard. In a real system, each keypress
// triggers interrupt 33, and the keyboard ISR reads the scancode from the
// keyboard controller and places it in a buffer. User programs then read from
// this buffer via the character device interface.
//
// Our simulation skips the ISR part — we provide an `enqueueKeys()` method
// to manually add keystrokes to the buffer, simulating what the ISR would do.
//
// Key properties:
//   - Read-only: write() returns -1 (you can't "write" to a keyboard)
//   - Non-blocking: read() returns 0 bytes if the buffer is empty
//   - FIFO ordering: keys come out in the order they were pressed

export class SimulatedKeyboard implements CharacterDevice {
  readonly name: string;
  readonly deviceType = DeviceType.CHARACTER;
  readonly major: number;
  readonly minor: number;
  readonly interruptNumber: number;
  initialized: boolean;

  /** Internal buffer of keystrokes, filled by the keyboard ISR. */
  private buffer: number[] = [];

  constructor(
    options: {
      name?: string;
      major?: number;
      minor?: number;
      interruptNumber?: number;
    } = {}
  ) {
    this.name = options.name ?? "keyboard0";
    this.major = options.major ?? 2;
    this.minor = options.minor ?? 0;
    this.interruptNumber = options.interruptNumber ?? 33;
    this.initialized = false;
  }

  /**
   * Initialize the keyboard. Clears any buffered keystrokes.
   */
  init(): void {
    this.buffer = [];
    this.initialized = true;
  }

  /**
   * Read up to `count` bytes from the keyboard buffer.
   *
   * Returns a Uint8Array containing the bytes actually read. If the buffer
   * is empty, returns an empty array (non-blocking behavior). If there are
   * fewer bytes than requested, returns only what's available.
   *
   * This mirrors how Unix character devices work: read() never blocks in
   * non-blocking mode, and always returns the lesser of (requested, available).
   */
  read(count: number): Uint8Array {
    const bytesToRead = Math.min(count, this.buffer.length);
    const result = new Uint8Array(bytesToRead);
    for (let i = 0; i < bytesToRead; i++) {
      result[i] = this.buffer.shift()!;
    }
    return result;
  }

  /**
   * Write to the keyboard — always fails.
   *
   * You cannot "write" to a keyboard. This would be like trying to push
   * letters back into the keyboard through the keys. The device is input-only.
   * Returns -1 to signal the error.
   */
  write(_data: Uint8Array): number {
    return -1;
  }

  /**
   * Enqueue keystrokes into the buffer.
   *
   * In a real system, the keyboard ISR (interrupt 33 handler) would call this
   * after reading the scancode from the keyboard controller's data port.
   * In our simulation, tests call this directly to simulate keypresses.
   */
  enqueueKeys(bytes: number[]): void {
    this.buffer.push(...bytes);
  }
}

// ============================================================================
// SimulatedDisplay (CharacterDevice)
// ============================================================================
//
// Simulates a text-mode display as a character device. In a real system, the
// display is memory-mapped: writing to specific memory addresses changes what
// appears on screen. Each character cell is 2 bytes: one for the character
// code, one for the color attribute (foreground + background).
//
// Our display uses a framebuffer of 80 columns * 25 rows * 2 bytes/cell =
// 4000 bytes. Writing a character advances the cursor position, wrapping to
// the next line at column 80.
//
// Key properties:
//   - Write-only (as a character device): read() returns empty (the S05
//     display driver has its own snapshot mechanism for reading the screen)
//   - No interrupts: displays don't generate interrupts (interrupt = -1)
//   - Cursor tracking: the display knows where the next character goes

export class SimulatedDisplay implements CharacterDevice {
  readonly name: string;
  readonly deviceType = DeviceType.CHARACTER;
  readonly major: number;
  readonly minor: number;
  readonly interruptNumber: number;
  initialized: boolean;

  /** Number of columns in the display. Standard VGA text mode = 80. */
  readonly columns: number;

  /** Number of rows in the display. Standard VGA text mode = 25. */
  readonly rows: number;

  /**
   * The framebuffer — a byte array representing the display memory.
   * Each character cell takes 2 bytes: [character_code, attribute].
   * Total size = columns * rows * 2.
   */
  readonly framebuffer: Uint8Array;

  /** Current cursor position. */
  cursorRow: number = 0;
  cursorCol: number = 0;

  /** Default attribute byte (0x07 = light gray on black). */
  readonly defaultAttribute: number;

  constructor(
    options: {
      name?: string;
      major?: number;
      minor?: number;
      columns?: number;
      rows?: number;
      defaultAttribute?: number;
    } = {}
  ) {
    this.name = options.name ?? "display0";
    this.major = options.major ?? 1;
    this.minor = options.minor ?? 0;
    this.interruptNumber = -1; // Displays don't generate interrupts
    this.columns = options.columns ?? 80;
    this.rows = options.rows ?? 25;
    this.defaultAttribute = options.defaultAttribute ?? 0x07;
    this.initialized = false;
    this.framebuffer = new Uint8Array(this.columns * this.rows * 2);
  }

  /**
   * Initialize the display by clearing the screen.
   * Sets every cell to (space, default_attribute) and resets the cursor to (0,0).
   */
  init(): void {
    this.clearScreen();
    this.initialized = true;
  }

  /**
   * Read from the display — returns empty array.
   *
   * Displays are write-only as character devices. You cannot "read" pixels
   * from the screen through the character device interface. (The S05 display
   * driver provides snapshot() for reading the framebuffer.)
   */
  read(_count: number): Uint8Array {
    return new Uint8Array(0);
  }

  /**
   * Write characters to the display at the current cursor position.
   *
   * Each byte is treated as an ASCII character code. The character is written
   * to the framebuffer at the current cursor position with the default
   * attribute, then the cursor advances. At the end of a line, the cursor
   * wraps to the next line.
   *
   * Returns the number of bytes written (always equals data.length).
   */
  write(data: Uint8Array): number {
    for (let i = 0; i < data.length; i++) {
      this.putChar(data[i]);
    }
    return data.length;
  }

  /**
   * Write a single character to the framebuffer at the current cursor position.
   *
   * The framebuffer layout:
   *   offset = (row * columns + col) * 2
   *   framebuffer[offset]     = character code
   *   framebuffer[offset + 1] = attribute byte
   *
   * After writing, the cursor advances. If it goes past the last column,
   * it wraps to column 0 of the next row. If it goes past the last row,
   * the cursor stays at the last row (scrolling is handled by the display
   * driver at a higher level).
   */
  private putChar(charCode: number): void {
    if (this.cursorRow >= this.rows) {
      return; // Screen is full — no scrolling in this simple simulation
    }
    const offset = (this.cursorRow * this.columns + this.cursorCol) * 2;
    this.framebuffer[offset] = charCode;
    this.framebuffer[offset + 1] = this.defaultAttribute;

    this.cursorCol++;
    if (this.cursorCol >= this.columns) {
      this.cursorCol = 0;
      this.cursorRow++;
    }
  }

  /**
   * Clear the entire screen.
   *
   * Sets every cell to (0x20 = space, default_attribute) and resets the
   * cursor to position (0, 0). This is what happens when you call `clear`
   * in a terminal — every cell gets a space character with the default colors.
   */
  clearScreen(): void {
    for (let i = 0; i < this.framebuffer.length; i += 2) {
      this.framebuffer[i] = 0x20;     // space character
      this.framebuffer[i + 1] = this.defaultAttribute;
    }
    this.cursorRow = 0;
    this.cursorCol = 0;
  }

  /**
   * Read the character at a given row and column.
   * Useful for testing — lets you verify what was written to the display.
   */
  getCharAt(row: number, col: number): number {
    const offset = (row * this.columns + col) * 2;
    return this.framebuffer[offset];
  }
}

// ============================================================================
// SharedWire
// ============================================================================
//
// A simulated network cable connecting multiple NICs. In a real Ethernet
// network, when one NIC sends a frame onto the wire, all other NICs on the
// same segment receive it (this is how hubs work — switches are smarter and
// only forward to the destination, but the basic model is broadcast).
//
// Our SharedWire models a hub: when one NIC sends a packet, every other NIC
// connected to the same wire receives a copy. The sender does NOT receive
// its own packet (just like a real NIC doesn't echo its own transmissions
// back to itself).
//
// Each NIC has its own receive queue. The wire pushes packets into these
// queues when broadcast() is called.

export class SharedWire {
  /** All NICs connected to this wire. */
  private connectedNics: SimulatedNIC[] = [];

  /** Connect a NIC to this wire. */
  connect(nic: SimulatedNIC): void {
    this.connectedNics.push(nic);
  }

  /**
   * Broadcast a packet to all NICs on the wire, except the sender.
   *
   * This models how Ethernet hubs work: a frame arriving on one port is
   * copied to all other ports. In simulation, we push the packet data
   * into each receiving NIC's rx_queue.
   */
  broadcast(data: Uint8Array, sender: SimulatedNIC): void {
    for (const nic of this.connectedNics) {
      if (nic !== sender) {
        nic.enqueuePacket(new Uint8Array(data));
      }
    }
  }
}

// ============================================================================
// SimulatedNIC (NetworkDevice)
// ============================================================================
//
// A network interface card backed by in-memory packet queues. Two SimulatedNICs
// connected to the same SharedWire can exchange packets, modeling a simple
// Ethernet network.
//
// How real NICs work:
//   1. To SEND: the OS writes packet data to a "transmit ring buffer" in memory,
//      then pokes a register on the NIC to say "new packet ready." The NIC's DMA
//      engine reads the data and puts it on the wire.
//   2. To RECEIVE: the NIC's DMA engine writes incoming packets into a "receive
//      ring buffer" in memory, then raises an interrupt to tell the OS "new
//      packet arrived."
//
// Our simulation skips the DMA and ring buffers — send_packet() directly
// broadcasts via the SharedWire, and receive_packet() dequeues from an
// internal array. The interrupt mechanism is included for completeness.
//
// MAC addresses:
//   Every NIC has a 6-byte MAC (Media Access Control) address, like a mailing
//   address. In real hardware, this is burned in at the factory. Example:
//   DE:AD:BE:EF:00:01. We assign MAC addresses at construction time.

export class SimulatedNIC implements NetworkDevice {
  readonly name: string;
  readonly deviceType = DeviceType.NETWORK;
  readonly major: number;
  readonly minor: number;
  readonly interruptNumber: number;
  readonly macAddress: Uint8Array;
  initialized: boolean;

  /** Receive queue — packets waiting to be read by the OS. */
  private rxQueue: Uint8Array[] = [];

  /** The shared wire this NIC is connected to. */
  private wire: SharedWire;

  constructor(
    wire: SharedWire,
    options: {
      name?: string;
      major?: number;
      minor?: number;
      interruptNumber?: number;
      macAddress?: Uint8Array;
    } = {}
  ) {
    this.name = options.name ?? "nic0";
    this.major = options.major ?? 4;
    this.minor = options.minor ?? 0;
    this.interruptNumber = options.interruptNumber ?? 35;
    this.macAddress =
      options.macAddress ??
      new Uint8Array([0xde, 0xad, 0xbe, 0xef, 0x00, 0x01]);
    this.initialized = false;
    this.wire = wire;
  }

  /**
   * Initialize the NIC. Connects it to the shared wire and clears the
   * receive queue.
   */
  init(): void {
    this.rxQueue = [];
    this.wire.connect(this);
    this.initialized = true;
  }

  /**
   * Send a packet over the network.
   *
   * The packet is broadcast to all other NICs on the same wire. Returns
   * the number of bytes sent (the packet length).
   *
   * In a real NIC, this would write the packet to the transmit ring buffer
   * and trigger the NIC's DMA engine. Our simulation is instantaneous.
   */
  sendPacket(data: Uint8Array): number {
    if (data.length === 0) {
      return -1;
    }
    this.wire.broadcast(data, this);
    return data.length;
  }

  /**
   * Receive the next packet from the receive queue.
   *
   * Returns the packet data, or null if no packets are waiting.
   * This is non-blocking — it returns immediately.
   *
   * In a real system, the NIC raises interrupt 35 when a packet arrives,
   * and the ISR calls this to retrieve the packet data.
   */
  receivePacket(): Uint8Array | null {
    if (this.rxQueue.length === 0) {
      return null;
    }
    return this.rxQueue.shift()!;
  }

  /**
   * Check whether there are packets waiting in the receive queue.
   *
   * This is useful for polling: the kernel can check hasPacket() before
   * calling receivePacket(), avoiding the null check. In interrupt-driven
   * mode, the ISR would call receivePacket() directly.
   */
  hasPacket(): boolean {
    return this.rxQueue.length > 0;
  }

  /**
   * Add a packet to the receive queue.
   *
   * Called by SharedWire.broadcast() when another NIC sends a packet.
   * In a real system, the NIC hardware writes the packet to the receive
   * ring buffer via DMA. In simulation, we just push it onto an array.
   */
  enqueuePacket(data: Uint8Array): void {
    this.rxQueue.push(data);
  }
}
