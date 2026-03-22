/**
 * Tests for the Device Driver Framework.
 *
 * These tests verify all three device families (Character, Block, Network),
 * the DeviceRegistry, and the concrete simulated implementations. They follow
 * the testing strategy outlined in the D12 specification.
 */

import { describe, it, expect, beforeEach } from "vitest";
import {
  DeviceType,
  DeviceRegistry,
  SimulatedDisk,
  SimulatedKeyboard,
  SimulatedDisplay,
  SimulatedNIC,
  SharedWire,
  isCharacterDevice,
  isBlockDevice,
  isNetworkDevice,
  type Device,
} from "../src/index.js";

// ============================================================================
// DeviceType Enum Tests
// ============================================================================

describe("DeviceType", () => {
  it("has three distinct values", () => {
    expect(DeviceType.CHARACTER).toBe(0);
    expect(DeviceType.BLOCK).toBe(1);
    expect(DeviceType.NETWORK).toBe(2);
  });

  it("values are all distinct from each other", () => {
    const values = new Set([
      DeviceType.CHARACTER,
      DeviceType.BLOCK,
      DeviceType.NETWORK,
    ]);
    expect(values.size).toBe(3);
  });
});

// ============================================================================
// Type Guard Tests
// ============================================================================

describe("type guards", () => {
  it("isCharacterDevice identifies character devices", () => {
    const kb = new SimulatedKeyboard();
    kb.init();
    expect(isCharacterDevice(kb)).toBe(true);
    expect(isBlockDevice(kb)).toBe(false);
    expect(isNetworkDevice(kb)).toBe(false);
  });

  it("isBlockDevice identifies block devices", () => {
    const disk = new SimulatedDisk();
    disk.init();
    expect(isBlockDevice(disk)).toBe(true);
    expect(isCharacterDevice(disk)).toBe(false);
    expect(isNetworkDevice(disk)).toBe(false);
  });

  it("isNetworkDevice identifies network devices", () => {
    const wire = new SharedWire();
    const nic = new SimulatedNIC(wire);
    nic.init();
    expect(isNetworkDevice(nic)).toBe(true);
    expect(isCharacterDevice(nic)).toBe(false);
    expect(isBlockDevice(nic)).toBe(false);
  });
});

// ============================================================================
// SimulatedDisk Tests
// ============================================================================

describe("SimulatedDisk", () => {
  let disk: SimulatedDisk;

  beforeEach(() => {
    // Create a small disk for testing: 8 blocks of 512 bytes = 4 KB
    disk = new SimulatedDisk({ totalBlocks: 8 });
    disk.init();
  });

  it("has correct default properties", () => {
    expect(disk.name).toBe("disk0");
    expect(disk.deviceType).toBe(DeviceType.BLOCK);
    expect(disk.major).toBe(3);
    expect(disk.minor).toBe(0);
    expect(disk.interruptNumber).toBe(34);
    expect(disk.blockSize).toBe(512);
    expect(disk.totalBlocks).toBe(8);
    expect(disk.initialized).toBe(true);
  });

  it("allows custom properties", () => {
    const custom = new SimulatedDisk({
      name: "disk1",
      major: 3,
      minor: 1,
      interruptNumber: 34,
      blockSize: 1024,
      totalBlocks: 16,
    });
    custom.init();
    expect(custom.name).toBe("disk1");
    expect(custom.minor).toBe(1);
    expect(custom.blockSize).toBe(1024);
    expect(custom.totalBlocks).toBe(16);
  });

  it("reads all zeros from a fresh disk", () => {
    // A freshly initialized disk should be all zeros, like a formatted drive.
    const data = disk.readBlock(0);
    expect(data.length).toBe(512);
    expect(data.every((b) => b === 0)).toBe(true);
  });

  it("round-trips data through write and read", () => {
    // Write a pattern to block 5, then read it back.
    const pattern = new Uint8Array(512);
    for (let i = 0; i < 512; i++) {
      pattern[i] = i & 0xff; // Repeating 0-255 pattern
    }
    disk.writeBlock(5, pattern);

    const readBack = disk.readBlock(5);
    expect(readBack).toEqual(pattern);
  });

  it("does not affect adjacent blocks when writing", () => {
    // Writing to block 3 should not change block 2 or block 4.
    const data = new Uint8Array(512).fill(0xaa);
    disk.writeBlock(3, data);

    const block2 = disk.readBlock(2);
    const block4 = disk.readBlock(4);
    expect(block2.every((b) => b === 0)).toBe(true);
    expect(block4.every((b) => b === 0)).toBe(true);
  });

  it("throws on read of out-of-bounds block number", () => {
    expect(() => disk.readBlock(8)).toThrow("out of range");
    expect(() => disk.readBlock(-1)).toThrow("out of range");
    expect(() => disk.readBlock(100)).toThrow("out of range");
  });

  it("throws on write of out-of-bounds block number", () => {
    const data = new Uint8Array(512);
    expect(() => disk.writeBlock(8, data)).toThrow("out of range");
    expect(() => disk.writeBlock(-1, data)).toThrow("out of range");
  });

  it("throws when write data does not match block size", () => {
    const shortData = new Uint8Array(256);
    expect(() => disk.writeBlock(0, shortData)).toThrow(
      "does not match block size"
    );

    const longData = new Uint8Array(1024);
    expect(() => disk.writeBlock(0, longData)).toThrow(
      "does not match block size"
    );
  });

  it("returns a copy from readBlock, not a reference", () => {
    // Modifying the returned data should not change the disk.
    const data = new Uint8Array(512).fill(0xff);
    disk.writeBlock(0, data);

    const read1 = disk.readBlock(0);
    read1[0] = 0x00; // Modify the returned copy

    const read2 = disk.readBlock(0);
    expect(read2[0]).toBe(0xff); // Original data should be unchanged
  });

  it("can read and write the last block", () => {
    const data = new Uint8Array(512).fill(0xbb);
    disk.writeBlock(7, data); // Block 7 is the last block (0-indexed)
    const readBack = disk.readBlock(7);
    expect(readBack).toEqual(data);
  });
});

// ============================================================================
// SimulatedKeyboard Tests
// ============================================================================

describe("SimulatedKeyboard", () => {
  let keyboard: SimulatedKeyboard;

  beforeEach(() => {
    keyboard = new SimulatedKeyboard();
    keyboard.init();
  });

  it("has correct default properties", () => {
    expect(keyboard.name).toBe("keyboard0");
    expect(keyboard.deviceType).toBe(DeviceType.CHARACTER);
    expect(keyboard.major).toBe(2);
    expect(keyboard.minor).toBe(0);
    expect(keyboard.interruptNumber).toBe(33);
    expect(keyboard.initialized).toBe(true);
  });

  it("returns empty array when buffer is empty", () => {
    // No keys pressed — read should return nothing (non-blocking).
    const data = keyboard.read(10);
    expect(data.length).toBe(0);
  });

  it("returns enqueued bytes in FIFO order", () => {
    // Simulate pressing 'H', 'i', '!' (ASCII 72, 105, 33)
    keyboard.enqueueKeys([72, 105, 33]);

    const data = keyboard.read(3);
    expect(data.length).toBe(3);
    expect(data[0]).toBe(72); // 'H'
    expect(data[1]).toBe(105); // 'i'
    expect(data[2]).toBe(33); // '!'
  });

  it("returns only available bytes when count exceeds buffer size", () => {
    // Only 2 keys pressed, but asking for 10
    keyboard.enqueueKeys([65, 66]);

    const data = keyboard.read(10);
    expect(data.length).toBe(2); // Only got what was available
    expect(data[0]).toBe(65); // 'A'
    expect(data[1]).toBe(66); // 'B'
  });

  it("drains the buffer progressively", () => {
    keyboard.enqueueKeys([1, 2, 3, 4, 5]);

    // Read 2, then read 2 more, then read the rest
    const first = keyboard.read(2);
    expect(first).toEqual(new Uint8Array([1, 2]));

    const second = keyboard.read(2);
    expect(second).toEqual(new Uint8Array([3, 4]));

    const third = keyboard.read(10);
    expect(third).toEqual(new Uint8Array([5]));

    // Buffer should now be empty
    const empty = keyboard.read(1);
    expect(empty.length).toBe(0);
  });

  it("write returns -1 (keyboards are read-only)", () => {
    // You cannot write to a keyboard — it's an input device.
    const data = new Uint8Array([72, 105]);
    expect(keyboard.write(data)).toBe(-1);
  });

  it("init clears the buffer", () => {
    keyboard.enqueueKeys([1, 2, 3]);
    keyboard.init(); // Re-initialize should clear buffer
    const data = keyboard.read(10);
    expect(data.length).toBe(0);
  });
});

// ============================================================================
// SimulatedDisplay Tests
// ============================================================================

describe("SimulatedDisplay", () => {
  let display: SimulatedDisplay;

  beforeEach(() => {
    display = new SimulatedDisplay({ columns: 80, rows: 25 });
    display.init();
  });

  it("has correct default properties", () => {
    expect(display.name).toBe("display0");
    expect(display.deviceType).toBe(DeviceType.CHARACTER);
    expect(display.major).toBe(1);
    expect(display.minor).toBe(0);
    expect(display.interruptNumber).toBe(-1); // No interrupts for displays
    expect(display.initialized).toBe(true);
    expect(display.columns).toBe(80);
    expect(display.rows).toBe(25);
  });

  it("framebuffer size is columns * rows * 2", () => {
    // Each cell is 2 bytes: character + attribute
    expect(display.framebuffer.length).toBe(80 * 25 * 2);
  });

  it("init clears screen to spaces", () => {
    // After init, every cell should be (space=0x20, attribute=0x07)
    for (let i = 0; i < display.framebuffer.length; i += 2) {
      expect(display.framebuffer[i]).toBe(0x20);
      expect(display.framebuffer[i + 1]).toBe(0x07);
    }
  });

  it("init resets cursor to (0, 0)", () => {
    display.cursorRow = 5;
    display.cursorCol = 10;
    display.init();
    expect(display.cursorRow).toBe(0);
    expect(display.cursorCol).toBe(0);
  });

  it("writes characters to framebuffer", () => {
    // Write 'H' (0x48) and 'i' (0x69)
    const data = new Uint8Array([0x48, 0x69]);
    const written = display.write(data);

    expect(written).toBe(2);
    expect(display.getCharAt(0, 0)).toBe(0x48); // 'H'
    expect(display.getCharAt(0, 1)).toBe(0x69); // 'i'
  });

  it("advances cursor after writing", () => {
    display.write(new Uint8Array([0x48, 0x69]));
    expect(display.cursorRow).toBe(0);
    expect(display.cursorCol).toBe(2);
  });

  it("wraps to next line when reaching end of column", () => {
    // Write exactly 80 characters to fill the first line
    const line = new Uint8Array(80).fill(0x41); // 80 'A's
    display.write(line);

    // Cursor should now be at the start of line 1
    expect(display.cursorRow).toBe(1);
    expect(display.cursorCol).toBe(0);
  });

  it("read returns empty array (displays are write-only)", () => {
    // You cannot "read" a display through the character device interface.
    const data = display.read(10);
    expect(data.length).toBe(0);
  });

  it("clearScreen resets all cells and cursor", () => {
    // Write something, then clear
    display.write(new Uint8Array([0x48, 0x65, 0x6c, 0x6c, 0x6f]));
    display.clearScreen();

    expect(display.cursorRow).toBe(0);
    expect(display.cursorCol).toBe(0);
    expect(display.getCharAt(0, 0)).toBe(0x20); // space
  });

  it("writes with correct attribute byte", () => {
    display.write(new Uint8Array([0x41])); // 'A'
    // Character at (0,0) should have attribute 0x07
    const offset = 0;
    expect(display.framebuffer[offset]).toBe(0x41);
    expect(display.framebuffer[offset + 1]).toBe(0x07);
  });

  it("handles custom display configuration", () => {
    const small = new SimulatedDisplay({ columns: 40, rows: 12 });
    small.init();
    expect(small.columns).toBe(40);
    expect(small.rows).toBe(12);
    expect(small.framebuffer.length).toBe(40 * 12 * 2);
  });

  it("stops writing when screen is full", () => {
    const small = new SimulatedDisplay({ columns: 4, rows: 2 });
    small.init();

    // Write 10 characters — only 8 fit (4 cols * 2 rows)
    const data = new Uint8Array(10).fill(0x41);
    const written = small.write(data);

    // All 10 are "written" from the interface's perspective
    expect(written).toBe(10);
    // But only 8 actually make it to the framebuffer
    expect(small.cursorRow).toBe(2); // Past the last row
  });
});

// ============================================================================
// SharedWire & SimulatedNIC Tests
// ============================================================================

describe("SharedWire and SimulatedNIC", () => {
  let wire: SharedWire;
  let nicA: SimulatedNIC;
  let nicB: SimulatedNIC;

  beforeEach(() => {
    wire = new SharedWire();
    nicA = new SimulatedNIC(wire, {
      name: "nic0",
      macAddress: new Uint8Array([0xde, 0xad, 0xbe, 0xef, 0x00, 0x01]),
    });
    nicB = new SimulatedNIC(wire, {
      name: "nic1",
      minor: 1,
      macAddress: new Uint8Array([0xde, 0xad, 0xbe, 0xef, 0x00, 0x02]),
    });
    nicA.init();
    nicB.init();
  });

  it("has correct default properties", () => {
    expect(nicA.name).toBe("nic0");
    expect(nicA.deviceType).toBe(DeviceType.NETWORK);
    expect(nicA.major).toBe(4);
    expect(nicA.minor).toBe(0);
    expect(nicA.interruptNumber).toBe(35);
    expect(nicA.initialized).toBe(true);
  });

  it("mac address is exactly 6 bytes", () => {
    expect(nicA.macAddress.length).toBe(6);
    expect(nicB.macAddress.length).toBe(6);
  });

  it("receivePacket returns null when queue is empty", () => {
    expect(nicA.receivePacket()).toBeNull();
    expect(nicB.receivePacket()).toBeNull();
  });

  it("hasPacket returns false when queue is empty", () => {
    expect(nicA.hasPacket()).toBe(false);
  });

  it("sends packet from NIC A that appears in NIC B", () => {
    const packet = new Uint8Array([0x01, 0x02, 0x03, 0x04]);
    const sent = nicA.sendPacket(packet);

    expect(sent).toBe(4);
    expect(nicB.hasPacket()).toBe(true);

    const received = nicB.receivePacket();
    expect(received).toEqual(packet);
  });

  it("sender does NOT receive its own packet", () => {
    // This models real Ethernet: a NIC doesn't echo its own transmissions.
    const packet = new Uint8Array([0x01, 0x02, 0x03]);
    nicA.sendPacket(packet);

    expect(nicA.hasPacket()).toBe(false);
    expect(nicA.receivePacket()).toBeNull();
  });

  it("broadcasts to all NICs on the wire", () => {
    // Add a third NIC to the wire
    const nicC = new SimulatedNIC(wire, {
      name: "nic2",
      minor: 2,
      macAddress: new Uint8Array([0xde, 0xad, 0xbe, 0xef, 0x00, 0x03]),
    });
    nicC.init();

    const packet = new Uint8Array([0xaa, 0xbb]);
    nicA.sendPacket(packet);

    // Both B and C should receive it
    expect(nicB.receivePacket()).toEqual(packet);
    expect(nicC.receivePacket()).toEqual(packet);
    // A should not
    expect(nicA.receivePacket()).toBeNull();
  });

  it("preserves packet ordering (FIFO)", () => {
    nicA.sendPacket(new Uint8Array([0x01]));
    nicA.sendPacket(new Uint8Array([0x02]));
    nicA.sendPacket(new Uint8Array([0x03]));

    expect(nicB.receivePacket()).toEqual(new Uint8Array([0x01]));
    expect(nicB.receivePacket()).toEqual(new Uint8Array([0x02]));
    expect(nicB.receivePacket()).toEqual(new Uint8Array([0x03]));
    expect(nicB.receivePacket()).toBeNull();
  });

  it("sendPacket returns -1 for empty packet", () => {
    const result = nicA.sendPacket(new Uint8Array(0));
    expect(result).toBe(-1);
  });

  it("bidirectional communication works", () => {
    // A sends to B, then B sends to A
    nicA.sendPacket(new Uint8Array([0x01]));
    nicB.sendPacket(new Uint8Array([0x02]));

    expect(nicB.receivePacket()).toEqual(new Uint8Array([0x01]));
    expect(nicA.receivePacket()).toEqual(new Uint8Array([0x02]));
  });

  it("received packet is a copy, not a reference", () => {
    const packet = new Uint8Array([0x01, 0x02, 0x03]);
    nicA.sendPacket(packet);

    const received = nicB.receivePacket()!;
    received[0] = 0xff; // Modify the received copy

    // Send the same packet again
    nicA.sendPacket(packet);
    const received2 = nicB.receivePacket()!;
    expect(received2[0]).toBe(0x01); // Should be original data
  });

  it("init clears the receive queue", () => {
    nicA.sendPacket(new Uint8Array([0x01]));
    expect(nicB.hasPacket()).toBe(true);

    nicB.init(); // Re-initialize should clear the queue
    expect(nicB.hasPacket()).toBe(false);
  });

  it("uses default mac address when none provided", () => {
    const wire2 = new SharedWire();
    const nic = new SimulatedNIC(wire2);
    nic.init();
    expect(nic.macAddress).toEqual(
      new Uint8Array([0xde, 0xad, 0xbe, 0xef, 0x00, 0x01])
    );
  });
});

// ============================================================================
// DeviceRegistry Tests
// ============================================================================

describe("DeviceRegistry", () => {
  let registry: DeviceRegistry;
  let disk: SimulatedDisk;
  let keyboard: SimulatedKeyboard;
  let display: SimulatedDisplay;

  beforeEach(() => {
    registry = new DeviceRegistry();
    disk = new SimulatedDisk({ totalBlocks: 8 });
    disk.init();
    keyboard = new SimulatedKeyboard();
    keyboard.init();
    display = new SimulatedDisplay();
    display.init();
  });

  it("registers and looks up by name", () => {
    registry.register(disk);
    const found = registry.lookupByName("disk0");
    expect(found).toBe(disk);
  });

  it("registers and looks up by major/minor", () => {
    registry.register(disk);
    const found = registry.lookupByMajorMinor(3, 0);
    expect(found).toBe(disk);
  });

  it("returns null for unknown name", () => {
    expect(registry.lookupByName("nonexistent")).toBeNull();
  });

  it("returns null for unknown major/minor", () => {
    expect(registry.lookupByMajorMinor(99, 99)).toBeNull();
  });

  it("throws on duplicate name", () => {
    registry.register(disk);
    const disk2 = new SimulatedDisk({ name: "disk0", minor: 1 });
    disk2.init();
    expect(() => registry.register(disk2)).toThrow("already registered");
  });

  it("throws on duplicate major/minor", () => {
    registry.register(disk);
    const disk2 = new SimulatedDisk({ name: "disk1", major: 3, minor: 0 });
    disk2.init();
    expect(() => registry.register(disk2)).toThrow("already registered");
  });

  it("throws when registering uninitialized device", () => {
    const uninit = new SimulatedDisk();
    // Do NOT call uninit.init()
    expect(() => registry.register(uninit)).toThrow("must be initialized");
  });

  it("lists all registered devices", () => {
    registry.register(disk);
    registry.register(keyboard);
    registry.register(display);

    const all = registry.listAll();
    expect(all.length).toBe(3);
    expect(all).toContain(disk);
    expect(all).toContain(keyboard);
    expect(all).toContain(display);
  });

  it("listAll returns a copy, not the internal array", () => {
    registry.register(disk);
    const all = registry.listAll();
    all.pop(); // Modify the returned copy
    expect(registry.listAll().length).toBe(1); // Internal list unchanged
  });

  it("lists devices by type", () => {
    registry.register(disk);
    registry.register(keyboard);
    registry.register(display);

    const blocks = registry.listByType(DeviceType.BLOCK);
    expect(blocks.length).toBe(1);
    expect(blocks[0]).toBe(disk);

    const chars = registry.listByType(DeviceType.CHARACTER);
    expect(chars.length).toBe(2);
    expect(chars).toContain(keyboard);
    expect(chars).toContain(display);

    const nets = registry.listByType(DeviceType.NETWORK);
    expect(nets.length).toBe(0);
  });

  it("unregisters a device by name", () => {
    registry.register(disk);
    expect(registry.unregister("disk0")).toBe(true);
    expect(registry.lookupByName("disk0")).toBeNull();
    expect(registry.lookupByMajorMinor(3, 0)).toBeNull();
    expect(registry.listAll().length).toBe(0);
  });

  it("unregister returns false for unknown name", () => {
    expect(registry.unregister("nonexistent")).toBe(false);
  });

  it("handles multiple devices of different types", () => {
    const wire = new SharedWire();
    const nic = new SimulatedNIC(wire);
    nic.init();

    registry.register(disk);
    registry.register(keyboard);
    registry.register(display);
    registry.register(nic);

    expect(registry.listAll().length).toBe(4);
    expect(registry.listByType(DeviceType.CHARACTER).length).toBe(2);
    expect(registry.listByType(DeviceType.BLOCK).length).toBe(1);
    expect(registry.listByType(DeviceType.NETWORK).length).toBe(1);
  });
});

// ============================================================================
// Integration Tests
// ============================================================================

describe("integration: full I/O path through registry", () => {
  it("writes through registry to display and verifies framebuffer", () => {
    // This simulates the kernel's sys_write path:
    //   1. Create and register a display
    //   2. Look it up via the registry
    //   3. Write characters through the character device interface
    //   4. Verify the characters appear in the framebuffer
    const registry = new DeviceRegistry();
    const display = new SimulatedDisplay();
    display.init();
    registry.register(display);

    // Kernel looks up the device
    const device = registry.lookupByName("display0")!;
    expect(isCharacterDevice(device)).toBe(true);

    // Kernel writes "Hi" through the device
    const charDevice = device as SimulatedDisplay;
    charDevice.write(new Uint8Array([0x48, 0x69])); // "Hi"

    // Verify characters in framebuffer
    expect(charDevice.getCharAt(0, 0)).toBe(0x48); // 'H'
    expect(charDevice.getCharAt(0, 1)).toBe(0x69); // 'i'
  });

  it("keyboard ISR → buffer → read through registry", () => {
    // Simulates the interrupt-driven keyboard path:
    //   1. Register keyboard
    //   2. ISR deposits keystrokes (enqueueKeys)
    //   3. Kernel reads via character device interface
    const registry = new DeviceRegistry();
    const keyboard = new SimulatedKeyboard();
    keyboard.init();
    registry.register(keyboard);

    // ISR deposits a keystroke
    keyboard.enqueueKeys([0x41]); // 'A'

    // Kernel reads through registry
    const device = registry.lookupByName("keyboard0")!;
    expect(isCharacterDevice(device)).toBe(true);

    const kb = device as SimulatedKeyboard;
    const data = kb.read(1);
    expect(data[0]).toBe(0x41);
  });

  it("network roundtrip: NIC A sends, NIC B receives via registry", () => {
    // Simulates two machines on the same network:
    //   1. Create a wire and two NICs
    //   2. Register both
    //   3. Send from A, receive on B
    const registry = new DeviceRegistry();
    const wire = new SharedWire();

    const nicA = new SimulatedNIC(wire, { name: "nic0" });
    nicA.init();
    registry.register(nicA);

    const nicB = new SimulatedNIC(wire, {
      name: "nic1",
      minor: 1,
      macAddress: new Uint8Array([0xde, 0xad, 0xbe, 0xef, 0x00, 0x02]),
    });
    nicB.init();
    registry.register(nicB);

    // Send from NIC A
    const sender = registry.lookupByName("nic0") as SimulatedNIC;
    sender.sendPacket(new Uint8Array([0xca, 0xfe]));

    // Receive on NIC B
    const receiver = registry.lookupByName("nic1") as SimulatedNIC;
    expect(receiver.hasPacket()).toBe(true);
    const packet = receiver.receivePacket()!;
    expect(packet).toEqual(new Uint8Array([0xca, 0xfe]));

    // A should not have received its own packet
    expect(sender.hasPacket()).toBe(false);
  });

  it("disk block I/O through registry", () => {
    // Simulates reading and writing disk blocks through the registry
    const registry = new DeviceRegistry();
    const disk = new SimulatedDisk({ totalBlocks: 8 });
    disk.init();
    registry.register(disk);

    // Kernel looks up the disk
    const device = registry.lookupByMajorMinor(3, 0)!;
    expect(isBlockDevice(device)).toBe(true);

    const blockDev = device as SimulatedDisk;

    // Write a block
    const data = new Uint8Array(512);
    data[0] = 0xfe;
    data[511] = 0xed;
    blockDev.writeBlock(0, data);

    // Read it back
    const readBack = blockDev.readBlock(0);
    expect(readBack[0]).toBe(0xfe);
    expect(readBack[511]).toBe(0xed);
  });
});
