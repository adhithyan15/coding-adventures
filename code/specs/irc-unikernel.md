# LibOS / Unikernel Framework

## Overview

A **Unikernel** (also called a **Library OS** or **LibOS**) is a single-address-space machine
image that bundles an application with only the OS components it actually needs. There is no
general-purpose kernel, no multi-user isolation, no shell, no `/proc`. There is just your
application and a minimal set of OS services linked directly into the same binary.

This document specifies a **modular LibOS framework** written in Rust for the
`x86_64-unknown-none` target. It is structured as a menu of composable modules: you include
only what your application requires. An HTTP server needs networking. A batch data processor
needs storage but not networking. A key-value store might need both. An IRC server needs
networking but not storage.

The framework is not IRC-specific. IRC is the application that first drove this work, but every
module is general-purpose. Swap the application, swap the modules you need, and you have a
different unikernel.

---

## The Nesting Doll in Full

```
┌─────────────────────────────────────────────────────────────────┐
│  Application  (IRC server, HTTP server, database, or anything)  │
├─────────────────────────────────────────────────────────────────┤
│  LibOS Module Layer  (include only what you need)               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │ Network  │  │ Storage  │  │  Timer   │  │  Serial  │       │
│  │ (smoltcp)│  │ (virtio  │  │ (HPET/  │  │  (UART)  │       │
│  │          │  │  blk)    │  │  APIC)   │  │          │       │
│  └────┬─────┘  └────┬─────┘  └──────────┘  └──────────┘       │
│       │              │                                          │
│  ┌────┴─────┐  ┌────┴─────┐                                    │
│  │  NIC     │  │  Disk    │                                    │
│  │  Driver  │  │  Driver  │                                    │
│  └──────────┘  └──────────┘                                    │
├─────────────────────────────────────────────────────────────────┤
│  Core Layer  (always required)                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Boot  →  Allocator  →  Panic Handler  →  Interrupts    │   │
│  └──────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  Hardware Interface                                             │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  virtio-net  │  virtio-blk  │  PCI bus  │  UART  │  APIC │  │
│  └───────────────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│  Hypervisor  (QEMU / KVM / Firecracker / Cloud Hypervisor)     │
├─────────────────────────────────────────────────────────────────┤
│  Physical hardware                                              │
└─────────────────────────────────────────────────────────────────┘
```

---

## Module Catalogue

Each module is a Rust crate (or feature flag within a crate). Modules declare their
dependencies. The application's `Cargo.toml` pulls in only what it needs.

### Core Modules (always required)

These are not optional. Every unikernel needs them.

| Module | Crate / feature | Provides |
|---|---|---|
| **boot** | `libos-boot` | x86-64 entry point, stack setup, BSS zeroing, Multiboot2 parsing |
| **allocator** | `libos-alloc` | Global heap allocator; implements `GlobalAlloc` |
| **panic** | `libos-panic` | `#[panic_handler]`; writes to serial if available, then halts |
| **interrupts** | `libos-interrupts` | IDT setup, basic exception handlers (page fault, double fault, etc.) |

### Optional Modules

Include only what your application uses.

| Module | Crate / feature | Provides | Depends on |
|---|---|---|---|
| **serial** | `libos-serial` | UART 16550 driver; `serial_print!` macro for debugging | boot |
| **timer** | `libos-timer` | HPET or APIC timer; `sleep_ms()`, `Instant::now()` | interrupts |
| **pci** | `libos-pci` | PCI bus enumeration; finds devices by vendor/device ID | boot |
| **network** | `libos-net` | smoltcp integration; `TcpListener`, `TcpStream` abstractions | pci, timer |
| **nic-virtio** | `libos-nic-virtio` | virtio-net driver; implements `smoltcp::phy::Device` | pci |
| **nic-e1000** | `libos-nic-e1000` | Intel e1000 driver (for QEMU `-net nic,model=e1000`) | pci |
| **storage** | `libos-storage` | Block device abstraction; `read_block()`, `write_block()` | pci |
| **blk-virtio** | `libos-blk-virtio` | virtio-blk driver; implements `Storage` | pci |
| **fs-fat** | `libos-fs-fat` | FAT32 filesystem on top of a block device | storage |

### Composing Modules

An application specifies its LibOS composition in `Cargo.toml`:

```toml
[dependencies]
# Core (always)
libos-boot = "0.1"
libos-alloc = "0.1"
libos-panic = "0.1"
libos-interrupts = "0.1"

# IRC server needs: serial (debug), timer (smoltcp), PCI, network, virtio-net NIC
libos-serial = "0.1"
libos-timer = "0.1"
libos-pci = "0.1"
libos-net = "0.1"
libos-nic-virtio = "0.1"

# IRC server does NOT need: storage, block driver, filesystem
# → Those crates are simply not listed
```

A batch data processor that reads from disk and writes results, but never touches the network:

```toml
[dependencies]
libos-boot = "0.1"
libos-alloc = "0.1"
libos-panic = "0.1"
libos-interrupts = "0.1"
libos-serial = "0.1"
libos-pci = "0.1"
libos-storage = "0.1"
libos-blk-virtio = "0.1"
libos-fs-fat = "0.1"
# No libos-net, libos-nic-virtio, libos-timer
```

---

## Core Module Specs

### boot

Responsibilities:
- Define `_start` (the ELF entry point, called directly by GRUB or the bootloader)
- Set up the stack (`mov rsp, STACK_TOP`)
- Zero the `.bss` segment (C guarantees this; we must do it ourselves)
- Parse the Multiboot2 info structure to find the memory map
- Call `libos_main(memory_map: &MemoryMap)` — the application's entry point

```rust
// The application provides this:
extern "C" fn libos_main(memory_map: &MemoryMap) -> !;

// The boot module calls it after setup:
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    // ... stack setup, BSS zero, multiboot parse ...
    libos_main(&memory_map);
}
```

### allocator

A pluggable global heap allocator. The `libos-alloc` crate provides a linked-list allocator
by default and a bump allocator as an alternative (faster, but cannot free).

```rust
use libos_alloc::LinkedListAllocator;

#[global_allocator]
static ALLOCATOR: LinkedListAllocator = LinkedListAllocator::new();

// Call this once from libos_main() after learning the heap region from the memory map:
pub fn init_heap(start: usize, size: usize) {
    unsafe { ALLOCATOR.init(start, size); }
}
```

The heap region is determined from the Multiboot2 memory map: find the largest contiguous
free region above the end of the kernel image.

### panic

```rust
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Attempt to write to serial port (may not be initialized yet)
    serial_println!("KERNEL PANIC: {}", info);
    loop {
        x86_64::instructions::hlt();
    }
}
```

### interrupts

Sets up the x86-64 Interrupt Descriptor Table (IDT) with handlers for:
- Breakpoint exception (`#BP`) — useful for debugging
- Page fault (`#PF`) — logs address and flags, then halts
- Double fault — catches stack overflow and other fatal errors; halts
- General protection fault (`#GP`) — logs and halts
- Hardware interrupts (IRQ) — delegated to registered handlers

```rust
pub fn init() {
    IDT.load();
    // Initialize the 8259 PIC or x2APIC
    unsafe { PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

// Modules register interrupt handlers:
pub fn register_irq(irq: u8, handler: fn()) {
    IRQ_HANDLERS.lock()[irq as usize] = Some(handler);
}
```

---

## Optional Module Specs

### serial

UART 16550 driver. The serial port at I/O address `0x3F8` (COM1) is always available in QEMU
and Firecracker. It is the primary debug output channel.

```rust
pub fn init() {
    // Configure UART: 115200 baud, 8N1
    unsafe {
        outb(COM1 + 1, 0x00);   // Disable interrupts
        outb(COM1 + 3, 0x80);   // Enable DLAB (baud rate divisor mode)
        outb(COM1 + 0, 0x01);   // Divisor low byte: 115200 baud
        outb(COM1 + 1, 0x00);   // Divisor high byte
        outb(COM1 + 3, 0x03);   // 8 bits, no parity, 1 stop bit
        outb(COM1 + 2, 0xC7);   // Enable FIFO, clear, 14-byte threshold
    }
}

pub fn write_byte(byte: u8) {
    while (unsafe { inb(COM1 + 5) } & 0x20) == 0 {}  // wait for empty
    unsafe { outb(COM1, byte); }
}

// Macro for formatted output:
macro_rules! serial_println {
    ($($arg:tt)*) => { /* write to serial */ };
}
```

### timer

Provides time measurement and sleeping. Two backends:

- **HPET** (High Precision Event Timer): preferred. Discovered via ACPI.
- **PIT** (Programmable Interval Timer, 8253): fallback. Always available.

```rust
pub trait Timer {
    fn now_ns(&self) -> u64;          // nanoseconds since boot
    fn sleep_ns(&self, ns: u64);      // block until elapsed
}

pub struct Instant(u64);  // nanoseconds since boot

impl Instant {
    pub fn now() -> Self { Self(TIMER.now_ns()) }
    pub fn elapsed(&self) -> Duration { Duration::from_nanos(TIMER.now_ns() - self.0) }
}
```

smoltcp requires an `Instant` type. The `libos-timer` module provides one that satisfies
smoltcp's `Instant` trait.

### pci

Enumerates the PCI bus and provides device discovery.

```rust
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub bar: [u32; 6],   // Base Address Registers
}

pub fn enumerate() -> Vec<PciDevice> { ... }

pub fn find(vendor_id: u16, device_id: u16) -> Option<PciDevice> {
    enumerate().into_iter().find(|d| d.vendor_id == vendor_id && d.device_id == device_id)
}
```

### network (libos-net)

A high-level networking API built on smoltcp. Hides the `Interface` / `SocketSet` complexity.

```rust
pub struct TcpListener { /* ... */ }
pub struct TcpStream { /* ... */ }

impl TcpListener {
    pub fn bind(addr: IpEndpoint) -> Result<Self, Error> { ... }
    pub fn accept(&mut self) -> Option<TcpStream> { ... }
}

impl TcpStream {
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> { ... }
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Error> { ... }
    pub fn close(self) { ... }
}

/// Must be called in the main loop to advance smoltcp state machines.
pub fn poll() {
    IFACE.lock().poll(Instant::now(), &mut *DEVICE.lock(), &mut *SOCKETS.lock());
}
```

The `libos-net` module's `TcpListener` and `TcpStream` implement the same `Listener` and
`Connection` protocols from `irc-net-stdlib`. This means `ircd` compiles unchanged — the
unikernel swap is purely at the module composition level.

### nic-virtio

virtio-net driver. Implements smoltcp's `Device` trait over virtio descriptor rings.

```rust
pub struct VirtioNet {
    rx_queue: Virtqueue,
    tx_queue: Virtqueue,
    mac: EthernetAddress,
}

impl smoltcp::phy::Device for VirtioNet {
    type RxToken<'a> = VirtioRxToken<'a>;
    type TxToken<'a> = VirtioTxToken<'a>;

    fn receive(&mut self, timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        // Check used ring for received frames
    }

    fn transmit(&mut self, timestamp: Instant) -> Option<Self::TxToken<'_>> {
        // Return a token that writes to the TX descriptor ring on consume()
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1514;
        caps.medium = Medium::Ethernet;
        caps
    }
}
```

### storage (libos-storage)

A block device abstraction:

```rust
pub trait BlockDevice {
    fn block_size(&self) -> usize;
    fn read_block(&mut self, block_idx: u64, buf: &mut [u8]) -> Result<(), Error>;
    fn write_block(&mut self, block_idx: u64, buf: &[u8]) -> Result<(), Error>;
}
```

### blk-virtio

Implements `BlockDevice` using virtio-blk descriptor rings. Structurally identical to
`nic-virtio` but for disk I/O.

### fs-fat

FAT32 filesystem on top of any `BlockDevice`. Provides file open/read/write/close:

```rust
pub struct FatFs<D: BlockDevice> { ... }

impl<D: BlockDevice> FatFs<D> {
    pub fn open(&mut self, path: &str) -> Result<File, Error> { ... }
    pub fn read(&mut self, file: &mut File, buf: &mut [u8]) -> Result<usize, Error> { ... }
    pub fn write(&mut self, file: &mut File, buf: &[u8]) -> Result<usize, Error> { ... }
}
```

---

## Boot Sequence (Generic)

```
VM start
    │
GRUB / rust-osdev/bootloader
    Load ELF into memory
    Set up protected / long mode
    Pass Multiboot2 info → jump to _start()
    │
_start() [libos-boot]
    Set RSP to STACK_TOP
    Zero BSS
    Parse memory map from Multiboot2
    Call libos_main()
    │
libos_main() [application-defined]
    init_heap(heap_start, heap_size)   [libos-alloc]
    serial::init()                     [libos-serial, if included]
    interrupts::init()                 [libos-interrupts]
    timer::init()                      [libos-timer, if included]
    pci::enumerate()                   [libos-pci, if included]
    nic = VirtioNet::init(pci_dev)     [libos-nic-virtio, if included]
    net::init(nic, ip_config)          [libos-net, if included]
    // ... any other modules ...
    application_main()                 [application code]
```

---

## Application Entry Points

### IRC server

```rust
fn application_main() -> ! {
    let mut server = IRCServer::new("irc.local", vec!["Welcome.".into()]);
    let listener = TcpListener::bind(IpEndpoint::new(IpAddress::v4(0, 0, 0, 0), 6667)).unwrap();
    loop {
        net::poll();  // advance smoltcp
        if let Some(stream) = listener.accept() {
            // handle IRC connection (single-threaded: handle one at a time, or maintain a list)
        }
    }
}
```

### Minimal "hello world" unikernel (no network, no storage)

```rust
// Cargo.toml: only libos-boot, libos-alloc, libos-panic, libos-serial

fn application_main() -> ! {
    serial_println!("Hello from bare metal!");
    loop {
        x86_64::instructions::hlt();
    }
}
```

### HTTP file server (network + storage + FAT)

```rust
// Cargo.toml: all core + libos-net, libos-nic-virtio, libos-storage, libos-blk-virtio, libos-fs-fat

fn application_main() -> ! {
    let disk = VirtioBlk::init(pci::find(VIRTIO_VENDOR, VIRTIO_BLK_DEVICE).unwrap());
    let fs = FatFs::new(disk);
    let listener = TcpListener::bind(/* :80 */);
    loop {
        net::poll();
        if let Some(stream) = listener.accept() {
            serve_http(stream, &mut fs);
        }
    }
}
```

---

## Build System

```toml
# .cargo/config.toml
[build]
target = "x86_64-unknown-none"

[unstable]
build-std = ["core", "alloc", "compiler_builtins"]
build-std-features = ["compiler-builtins-mem"]
```

```bash
# Build
cargo build --target x86_64-unknown-none --release

# Create bootable disk image (using rust-osdev/bootimage)
cargo bootimage

# Run in QEMU (with virtio-net for applications that need networking)
qemu-system-x86_64 \
    -kernel target/x86_64-unknown-none/release/myapp \
    -nographic \
    -serial stdio \
    -netdev tap,id=net0,ifname=tap0,script=no,downscript=no \
    -device virtio-net-pci,netdev=net0 \
    -m 128M

# Run without networking (pure compute / storage app)
qemu-system-x86_64 \
    -kernel target/x86_64-unknown-none/release/myapp \
    -nographic \
    -serial stdio \
    -m 32M
```

---

## Development Milestones

| Milestone | Modules | Test |
|---|---|---|
| M1: Bare metal boot | boot, panic, serial | "hello" appears on serial console |
| M2: Heap | + alloc | `Vec::new()` and `String::from()` work |
| M3: Interrupts | + interrupts | Timer ticks appear on serial; page faults caught |
| M4: PCI | + pci | PCI devices enumerated and logged |
| M5: NIC | + nic-virtio | virtio-net initialized; MAC address printed |
| M6: Network | + network | Ping from host succeeds; TCP connection established |
| M7: Application | application code | IRC / HTTP / etc. working over virtio-net |
| M8: Storage | + blk-virtio | Block reads/writes work |
| M9: Filesystem | + fs-fat | Files read from FAT32 disk image |

---

## Related Specs

- [irc-architecture.md](irc-architecture.md) — IRC system overview
- [irc-net-smoltcp.md](irc-net-smoltcp.md) — Userspace TCP (the `libos-net` module's engine)
