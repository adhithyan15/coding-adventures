# IRC System Architecture

## Overview

This document describes the architecture for a full IRC (Internet Relay Chat) server and client
ecosystem built across all supported languages in this repository. IRC is a text-based chat
protocol defined in RFC 1459 (1993) and updated by RFCs 2810–2813. Its simplicity makes it an
ideal vehicle for learning deep systems concepts: protocol parsing, byte-stream framing, event
loops, OS syscalls, and eventually, unikernel operating system design.

The system is built as a **Russian Nesting Doll**: each layer exposes a stable interface to the
layer above, and can be replaced with a deeper implementation without touching any other layer.
The IRC application logic is written once. The network substrate beneath it is peeled back,
layer by layer, until — in Rust — the IRC server boots on bare metal with no operating system.

---

## The Russian Nesting Doll

```
┌─────────────────────────────────────────────────────────────────┐
│  irc-server                                                     │
│  Channel state, nick table, command dispatch (RFC 1459)         │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  irc-proto                                                │  │
│  │  Parse/serialize IRC messages. Pure function, no I/O.     │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  irc-framing                                              │  │
│  │  Byte stream → \r\n-delimited frames. Pure, buffered.     │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  irc-net  [THE DOLL — implementation swapped per phase]   │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │  Level 1: stdlib (socket + thread per connection)   │  │  │
│  │  │  ┌───────────────────────────────────────────────┐  │  │  │
│  │  │  │  Level 2: selectors / mio (event loop)        │  │  │  │
│  │  │  │  ┌─────────────────────────────────────────┐  │  │  │  │
│  │  │  │  │  Level 3: epoll / kqueue (raw syscalls) │  │  │  │  │
│  │  │  │  │  ┌───────────────────────────────────┐  │  │  │  │  │
│  │  │  │  │  │  Level 4: smoltcp (userspace TCP) │  │  │  │  │  │
│  │  │  │  │  │  ┌─────────────────────────────┐  │  │  │  │  │  │
│  │  │  │  │  │  │  Level 5: virtio-net / NIC  │  │  │  │  │  │  │
│  │  │  │  │  │  │  (Unikernel, no OS at all)  │  │  │  │  │  │  │
│  │  │  │  │  │  └─────────────────────────────┘  │  │  │  │  │  │
│  │  │  │  │  └───────────────────────────────────┘  │  │  │  │  │
│  │  │  │  └─────────────────────────────────────────┘  │  │  │  │
│  │  │  └───────────────────────────────────────────────┘  │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

The critical rule: **`irc-server` never imports any `irc-net` package**. The server sees only
`ConnId`, `Message`, and `Handler` — abstract types with no I/O. The `ircd` program is the only
place that wires the doll together and picks which `irc-net` implementation to use.

---

## Stable Interfaces (The Seams)

These types and protocols are defined once and shared across all `irc-net-*` specs. They never
change as implementations are swapped.

```python
from __future__ import annotations
from typing import Protocol, NewType, Iterator
from dataclasses import dataclass

# --- irc-proto boundary ---

@dataclass
class Message:
    prefix: str | None       # "nick!user@host" or "server.name" or None
    command: str             # "PRIVMSG", "001", "JOIN", etc.
    params: list[str]        # all params, including trailing

# --- irc-net boundary ---

ConnId = NewType('ConnId', int)

class Connection(Protocol):
    @property
    def id(self) -> ConnId: ...
    @property
    def peer_addr(self) -> tuple[str, int]: ...
    def read(self) -> bytes: ...           # b"" signals connection closed
    def write(self, data: bytes) -> None: ...
    def close(self) -> None: ...

class Listener(Protocol):
    def accept(self) -> Connection: ...
    def close(self) -> None: ...

class Handler(Protocol):
    def on_connect(self, conn_id: ConnId) -> None: ...
    def on_message(self, conn_id: ConnId, msg: Message) -> None: ...
    def on_disconnect(self, conn_id: ConnId) -> None: ...

class EventLoop(Protocol):
    def run(self, listener: Listener, handler: Handler) -> None: ...
```

---

## Package Map

| Package | Type | Purpose | Depends On |
|---|---|---|---|
| `irc-proto` | library | Parse/serialize IRC messages | nothing |
| `irc-framing` | library | Byte buffer → \r\n frames | nothing |
| `irc-server` | library | IRC state machine, command dispatch | `irc-proto` |
| `irc-net-stdlib` | library | Level 1 network impl: threads | nothing |
| `irc-net-selectors` | library | Level 2: OS selector / mio event loop | nothing |
| `irc-net-epoll` | library | Level 3: raw epoll/kqueue syscalls | nothing |
| `irc-net-smoltcp` | library | Level 4: userspace TCP (Rust only) | nothing |
| `ircd` | program | Wires all packages; owns CLI and config | all above |

Each `irc-net-*` package implements the same `Connection`, `Listener`, `EventLoop` interfaces.
The program selects which one to use at compile time (or via a flag).

---

## Language Rollout

### Phase 1 — Python prototype

Python first. The goal is a working IRC server you can connect to with WeeChat before touching
any other language. All packages implemented with typed Python (`from __future__ import
annotations`, `mypy --strict`). Network layer: `irc-net-stdlib` using `socket` and `threading`.

### Phase 2 — All primary languages

Port all packages to Go, TypeScript, Ruby, Elixir, Rust, Kotlin, and Swift. Each language uses
`irc-net-stdlib` as its starting network implementation. Focus: the IRC logic is identical across
all languages. The interfaces enforce this.

### Phase 3 — C# and F#

C# and F# join here. They get the full, clean architecture from day one — no legacy
implementation to refactor. F# is a natural fit for `irc-proto` (discriminated unions for
commands, pattern matching on message structure).

### Phase 4 — Peeling the doll

Language by language, start replacing `irc-net-stdlib` with deeper implementations. Python goes
to `irc-net-selectors` (Python `selectors` module). Go goes to `irc-net-epoll` (`x/sys/unix`).
Rust goes all the way.

---

## Doll Depth by Language

| Level | Python | Go | TypeScript | Ruby | Elixir | Rust | Kotlin | Swift | C# | F# |
|---|---|---|---|---|---|---|---|---|---|---|
| stdlib (threads) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| selectors / mio | ✓ | ✓ | ✓ | — | — | ✓ | — | — | — | — |
| epoll / kqueue | ✓ (Linux) | ✓ | — | — | — | ✓ | — | — | — | — |
| smoltcp (userspace TCP) | — | — | — | — | — | ✓ | — | — | — | — |
| unikernel / bare metal | — | — | — | — | — | ✓ | — | — | — | — |

---

## The Unikernel Vision (Long-Term Rust Path)

The end state for the Rust implementation is an IRC server that **is** the operating system.
No Linux. No Windows. No syscall boundary between application and kernel. One ELF binary that
boots directly on a hypervisor (QEMU, KVM, or Firecracker).

```
┌─────────────────────────────────────────────────────┐
│  ircd  (IRC application — unchanged from Phase 2)   │
├─────────────────────────────────────────────────────┤
│  irc-net-smoltcp  (userspace TCP/IP stack)          │
├─────────────────────────────────────────────────────┤
│  virtio-net driver  (talks directly to virtual NIC) │
├─────────────────────────────────────────────────────┤
│  MMIO / PCI  (hypervisor exposes these registers)   │
├─────────────────────────────────────────────────────┤
│  KVM / Firecracker / QEMU hypervisor                │
├─────────────────────────────────────────────────────┤
│  Physical hardware                                  │
└─────────────────────────────────────────────────────┘
```

The IRC application code never changes. Only the layers beneath `irc-net` are replaced.
This is the nesting doll principle taken to its logical conclusion.

Key Rust techniques required:
- `#![no_std]` and `#![no_main]` — no standard library, no OS entry point
- Custom global allocator (`linked_list_allocator` or `buddy_system_allocator`)
- Custom panic handler (log to serial port, halt)
- `x86_64-unknown-none` build target
- `smoltcp` for TCP/IP (designed for `no_std`)
- `virtio-net` descriptor ring implementation for NIC access
- Boot via Multiboot2 header + GRUB, or `rust-osdev/bootloader` crate

See `irc-unikernel.md` for the full spec.

---

## Swap-out Plan for irc-net

| Phase | irc-net implementation | What you learn |
|---|---|---|
| 1 | `irc-net-stdlib` (threads) | IRC protocol, framing, server state |
| 2 | `irc-net-selectors` (event loop) | Reactor pattern, readiness model, non-blocking I/O |
| 3 | `irc-net-epoll` (raw syscalls) | `epoll_create1`, `epoll_ctl`, `epoll_wait`, edge-triggered bugs |
| 4 | `irc-net-smoltcp` (userspace TCP) | What an OS TCP stack does; smoltcp Interface, SocketSet |
| 5 | Unikernel | Boot sequence, allocators, virtio-net, bare metal networking |

Because the `Connection` / `Listener` / `EventLoop` interfaces are stable, each phase is a
drop-in replacement. The IRC logic — thousands of lines across all languages — never changes.

---

## Related Specs

- [irc-proto.md](irc-proto.md) — Message parsing and serialization
- [irc-framing.md](irc-framing.md) — Byte stream framing
- [irc-server.md](irc-server.md) — IRC state machine and command dispatch
- [irc-net-stdlib.md](irc-net-stdlib.md) — Level 1: stdlib sockets + threads
- [irc-net-selectors.md](irc-net-selectors.md) — Level 2: event-driven I/O
- [irc-net-epoll.md](irc-net-epoll.md) — Level 3: raw epoll/kqueue syscalls
- [irc-net-smoltcp.md](irc-net-smoltcp.md) — Level 4: userspace TCP (Rust)
- [irc-unikernel.md](irc-unikernel.md) — Level 5: bare metal unikernel (Rust)
- [ircd.md](ircd.md) — The server program
