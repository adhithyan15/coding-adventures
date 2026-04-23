# Event Loop Native Backends

## Overview

The existing `event-loop` packages in this repository teach the shape of an event
loop, but they do not yet wrap the native operating-system facilities that power
real interactive systems. That is the next step.

This document surveys the native OS mechanisms we can build on top of, compares
their semantics, and recommends a crate layout for future Rust implementations.
The goal is not to flatten every platform into the same lowest-common-denominator
API. The goal is to:

1. Wrap each native backend honestly, preserving its real semantics.
2. Build one layer above that which translates native events and completions into
   a repository-owned generic event substrate suitable for transports, protocols,
   native UIs, timers, wakeups, and other interactive systems.
3. Reuse those higher layers from Ruby, Python, and Perl via native extensions
   built on our existing bridge crates.

If we try to hide platform differences too early, we will end up with a leaky,
confusing abstraction. If we keep the layers crisp, we can teach the native
mechanisms clearly and still deliver a usable cross-platform event-loop runtime.

One key correction is important up front:

- native network readiness and completion APIs are only part of the story
- keyboard, mouse, touch, resize, focus, redraw, and window lifecycle events
  often arrive through window-system message pumps or protocol queues rather than
  through `epoll`, `kqueue`, or IOCP directly

So the repository should think in terms of a **general native event substrate**,
not a network-only poller.

That means the event loop itself should not be specialized to:

- TCP
- WebSockets
- GUI widgets
- one particular windowing system

Instead, those should all be consumers of the same lower event substrate.

---

## Repository Starting Point

Today the repository already contains:

- language-level educational `event-loop` packages in Rust, Go, Python, Ruby,
  TypeScript, Lua, Elixir, and Perl
- a lower-level networking spec, `irc-net-epoll.md`, that introduces raw
  `epoll`/`kqueue`
- bridge crates in Rust for Python, Ruby, Perl, Lua, Node, Erlang, and Objective-C

That means the missing layer is not "how do we expose Rust to dynamic languages?"
We already have that. The missing layer is "what are the native kernel-backed
polling, completion, message-pump, and input facilities we should wrap first,
and how should they stack?"

---

## Taxonomy

The first important distinction is semantic, not platform-specific.

### Readiness APIs

A readiness API answers:

> "Which handles are ready for me to try reading from or writing to right now?"

Examples:

- `select`
- `poll`
- `epoll`
- `kqueue`
- event ports (for file descriptors)
- `io_uring` poll operations when used as async poll

These APIs work well when your server owns nonblocking sockets and maintains its
own connection state machines.

### Completion APIs

A completion API answers:

> "Which previously submitted operations have finished?"

Examples:

- Windows IOCP
- Windows Registered I/O (RIO)
- Windows I/O rings
- Linux `io_uring` when used for submitted reads, writes, accepts, and sends

These APIs do not merely tell you "socket X is writable". They tell you "the
receive you posted for socket X completed" or "the accept operation finished".

### Window-System Dispatchers

A window-system dispatcher answers:

> "Which UI or application messages has the platform delivered to this thread,
> window, surface, or protocol connection?"

Examples:

- Win32 message queues (`GetMessage`, `DispatchMessage`)
- Cocoa / Core Foundation run loops on Apple platforms
- Wayland client event queues
- X11 event queues

These APIs are where keyboard, mouse, touch, resize, focus, close, and paint or
configure events typically enter a native UI application.

### Input Device Stacks

An input-device stack answers:

> "What raw or normalized device input has arrived from keyboards, pointers,
> touch devices, tablets, or other controllers?"

Examples:

- Linux evdev
- Linux `libinput`
- platform window-message translation layers that already normalize input

These matter most when we are building lower-level UI infrastructure such as a
Wayland compositor, a custom shell, a TTY UI with direct device access, or other
software that lives below the usual desktop widget toolkit layer.

### Auxiliary Event Sources

Real event loops need more than socket readiness:

- timers
- user-space wakeups
- signals
- process lifecycle events
- vnode or file-watch events
- window lifecycle events
- keyboard, mouse, touch, and gesture input
- redraw / present notifications

Some operating systems bundle these into one API. Others expose them as separate
facilities that must be integrated by the loop.

---

## Survey Summary

| Facility | OS family | Model | Strength | Main limitation | Recommendation |
|---|---|---|---|---|---|
| `select` / `pselect` | POSIX-ish | readiness | universal baseline | fixed-size fd sets, O(n) scan | fallback only |
| `poll` / `ppoll` | POSIX-ish | readiness | portable, simple | O(n) scan of all watched fds | fallback only |
| `epoll` | Linux | readiness | scalable interest/ready lists | Linux-only, ET/LT complexity | primary Linux backend |
| `eventfd` | Linux | auxiliary | cheap user wakeup | Linux-only, not a full poller | pair with `epoll` |
| `timerfd` | Linux | auxiliary | timer as fd | Linux-only | pair with `epoll` |
| `signalfd` | Linux | auxiliary | signals as fd | Linux-only, must manage signal mask | pair with `epoll` |
| `io_uring` | Linux | completion + async poll | very high ceiling | higher complexity, newer kernel surface | phase-two Linux backend |
| `kqueue` / `kevent` | BSD, macOS | readiness + filters | one API handles sockets, timers, signals, vnode, proc, user events | platform differences across BSD/macOS | primary BSD/macOS backend |
| event ports | illumos / Solaris / SmartOS | queue + mixed sources | one queue for fd, timer, aio, mq, user, file | niche platform target | optional backend |
| IOCP | Windows | completion | native Windows high-scale server model | fundamentally different from readiness APIs | primary Windows backend |
| Win32 message queue | Windows | window-system dispatch | native source for keyboard, mouse, touch, paint, focus, resize, close | separate from IOCP | required for Windows UI |
| Apple run loop / AppKit dispatch | macOS, iOS | window-system dispatch | native source for UI, timers, input sources, app lifecycle | UI sits above raw `kqueue` | required for Apple UI |
| Wayland client queue | Linux, BSD | protocol queue + pollable fd | modern Linux desktop UI path | protocol-specific and compositor-mediated | required for Wayland UI clients |
| X11 event queue | Unix desktops | window-system dispatch | mature queue-based desktop event model | legacy and display-server-specific | optional but practical |
| `libinput` / evdev | Linux | direct input stack | raw or normalized keyboard, pointer, touch input | aimed at compositors / low-level apps, not ordinary clients | use for compositor-grade work |
| RIO | Windows | completion, socket-specialized | high-throughput network path | Winsock-specific, registered-buffer complexity | phase-two Windows net backend |
| Windows I/O rings | Windows 11 / Server 2022+ | completion | ring-based async I/O | new, version-gated, less battle-tested for our use case | research backend |
| `WSAPoll`, `WSAEventSelect`, wait functions | Windows | readiness-ish / wait objects | available everywhere on Windows | poorer scaling and awkward integration | avoid as primary design |

---

## Facility-by-Facility Survey

### 1. `select` / `pselect`

This is the portable baseline. It is useful as a teaching tool and a fallback,
but not the foundation for a serious high-throughput or UI-heavy runtime.

Why it matters:

- every Unix programmer should understand it
- it gives us a tiny portability layer for tests and bootstrap work

Why it is not enough:

- fixed fd-set limits on many systems
- O(n) scan cost over the watched set
- rebuilding fd sets on every wait call

Use it for:

- reference implementations
- tiny fallback pollers
- tests

Do not use it as the main backend target.

### 2. `poll` / `ppoll`

`poll` improves on `select` by removing the bitset limit and using an array of
`pollfd` entries, but it still scans the whole watched set every time.

Use it for:

- portable fallback backends
- simple single-threaded servers
- debugger-friendly reference implementations
- integrating odd file-descriptor-driven sources into a small loop

Do not use it as the high-performance default.

### 3. Linux `epoll`

`epoll` is the first real production-grade Linux kernel event backend we should build.

What it gives us:

- a persistent in-kernel interest list
- a ready list populated by the kernel
- level-triggered and edge-triggered modes
- `EPOLLONESHOT` for explicit re-arming
- direct scalability to large numbers of mostly idle sockets

What it does not give us by itself:

- timers
- signals
- a built-in user wakeup primitive

Those come from companion Linux facilities:

- `eventfd` for wakeups and cross-thread nudges
- `timerfd` for timers
- `signalfd` for signals

This means the real Linux backend is not just an `epoll` crate. It is a small
family of Linux crates that fit together naturally.

Recommended first-class crates:

- `epoll`
- `eventfd`
- `timerfd`
- `signalfd`

These crates should stay honest and low-level. They should expose file-descriptor
ownership, registration flags, one-shot versus level-triggered semantics, and the
need for nonblocking I/O.

Important limitation for UI work:

- ordinary desktop keyboard, mouse, and touch events usually do not come from
  `epoll` directly
- they usually arrive from Wayland, X11, or some higher platform UI facility

So `epoll` is necessary for Linux networking and fd-driven sources, but it is
not the entire Linux UI story.

### 4. Linux `io_uring`

`io_uring` is not merely "better epoll". It is a different model.

What it gives us:

- shared submission and completion queues between user space and kernel space
- async reads, writes, accepts, sends, receives, and many other operations
- async poll requests, including multishot poll
- a path toward fewer syscalls and fewer context switches

Why we should not start here:

- substantially more moving parts than `epoll`
- completion semantics rather than simple readiness semantics
- version-sensitive features and a larger correctness surface
- the temptation to redesign the whole runtime around ring submissions before we
  have a stable repository-owned event model

Recommendation:

- absolutely survey and eventually support it
- do not make it the first backend we implement
- treat it as the Linux phase-two performance backend after the `epoll` stack is
  solid and after we have a clear cross-platform event model

### 5. BSD / macOS `kqueue`

`kqueue` is the natural peer to Linux `epoll`, but it is broader in scope.

The important feature of `kqueue` is that it is not only a socket poller. It is
an event filter hub. In one interface it can watch:

- read readiness
- write readiness
- vnode changes
- process lifecycle events
- signals
- timers
- user-triggered events

This is elegant, and it means the BSD/macOS story is often simpler than the
Linux story because fewer companion facilities are needed.

Important caveat:

- "kqueue" is a family, not one perfectly uniform behavior across every BSD and
  Apple release
- our common subset should target the features that are stable across macOS and
  modern BSDs

Recommended crate:

- `kqueue`

This crate should expose:

- raw `kevent` registration and retrieval
- filter-specific flags
- one-shot, clear, dispatch, and delete semantics
- user-event triggering
- timer filters where available in our supported targets

Important limitation for Apple UI work:

- Cocoa and AppKit applications do not stop at raw `kqueue`
- they run inside the platform run loop and higher UI dispatch machinery

So `kqueue` is an important kernel backend, but native macOS UI work will also
need an Apple run-loop or AppKit integration layer.

### 6. illumos / Solaris event ports

Event ports are less commonly discussed than `epoll` or `kqueue`, but they are a
real and capable native backend.

Interesting properties:

- one queue can multiplex events from disjoint sources
- sources include file descriptors, AIO, message queues, timers, user events,
  alerts, and file objects
- retrieved events carry both source type and user data

Important semantic difference:

- file-descriptor associations are one-shot and must be reassociated after the
  event is retrieved

That means event ports behave more like explicit re-arming than like default
level-triggered `epoll`.

Recommendation:

- support only if Solaris / illumos is a real repository target
- if we support it, give it its own crate and do not try to pretend it behaves
  exactly like `epoll`

### 7. Windows IOCP

IOCP is the primary Windows backend we should plan around.

This is the key Windows shift:

- Unix pollers tell you when an operation would not block
- IOCP tells you when a previously issued overlapped operation completed

That difference shapes everything above it.

What IOCP gives us:

- a completion port object shared across many overlapped handles
- queued completion packets for finished async operations
- native integration with sockets, pipes, files, and other overlapped endpoints
- a threading model designed for high concurrency
- `PostQueuedCompletionStatus` for user-space wakeups

What IOCP requires from us:

- explicit `OVERLAPPED` allocation and ownership
- pre-posted accepts, receives, and sends
- per-operation state tracking rather than mere "interest masks"

This is why Windows should not be forced into a fake readiness abstraction at the
raw layer. The raw crate should expose IOCP honestly. The higher layer can then
translate completions into repository-owned events like:

- listener accepted connection
- stream read completed
- stream write completed
- timer fired
- wakeup requested

Recommended crate:

- `iocp`

This crate should include wrappers for:

- `CreateIoCompletionPort`
- `GetQueuedCompletionStatus` and `GetQueuedCompletionStatusEx`
- `PostQueuedCompletionStatus`
- overlapped handle association

Important limitation for UI work:

- Win32 keyboard, mouse, touch, paint, resize, and close events do not arrive
  through IOCP
- they arrive through the thread's window message queue

That means Windows needs two different first-class event families:

- IOCP for high-performance overlapped I/O
- the Win32 message loop for native interactive UI

### 8. Win32 message queue

If we want native Windows UI, the Win32 message loop is not optional.

This is where Windows delivers events such as:

- key down / key up
- mouse movement and button changes
- touch and pointer messages
- paint requests
- resize and move notifications
- focus and activation changes
- close and quit requests

The core loop shape is:

- `GetMessage`
- `TranslateMessage`
- `DispatchMessage`

This is a different native event source from IOCP and should not be collapsed
into the same raw crate.

Recommended crate:

- `win32-message-loop`

This crate should expose:

- thread message retrieval
- dispatch to a Rust-owned window procedure shim
- wakeup or posted-message support
- translation into a repository-owned UI event vocabulary

### 9. Apple run loop and UI dispatch

On Apple platforms, native UI work lives in the platform run-loop machinery and
the windowing frameworks above it, not in raw `kqueue` alone.

Why it matters:

- `CFRunLoop` and `RunLoop` manage input sources and timers
- AppKit/UIKit sit on top of that machinery and deliver UI events through their
  own lifecycle and responder chains
- for this repository, the existing `objc-bridge` crate gives us a plausible path
  to native Objective-C framework integration later

Recommended crates:

- `cf-run-loop`
- later, for real Cocoa UI work, an AppKit-focused crate built on `objc-bridge`

### 10. Wayland client event queues

For modern Linux desktop UI clients, input and window lifecycle events often
arrive through a Wayland protocol connection rather than directly from kernel
input devices.

Important properties:

- a `wl_display` owns the connection to the compositor
- events are queued and then dispatched
- the connection exposes a file descriptor, which means a custom poller can
  integrate Wayland traffic with timers, sockets, and other fd-driven sources

This is exactly the kind of integration point we want: a UI protocol source that
can plug into a broader event loop without pretending to be raw keyboard hardware.

Recommended crate:

- `wayland-client-loop`

### 11. X11 event queues

X11 remains worth mentioning because it is still present in Unix desktop stacks
and because its event model is historically important.

It gives us:

- a queue of window-system events
- keyboard and mouse input selected by event masks
- synchronous or queued event retrieval

Recommendation:

- optional backend for portability and educational value
- not the centerpiece of a future-facing Linux UI stack, but still useful

Recommended crate:

- `x11-event-loop`

### 12. Linux direct input stacks: evdev and `libinput`

If we are building below the ordinary desktop-client level, especially if we are
building a compositor or shell, we should care about direct input stacks too.

Important distinction:

- ordinary GUI applications on Wayland typically receive input from the compositor
  via protocol events
- compositors and lower-level input managers use evdev or `libinput` to speak to
  actual devices

`libinput` is especially relevant because it normalizes and processes:

- keyboard events
- pointer motion
- touch coordinates
- touchpad gestures
- tablet input

Recommendation:

- use `libinput` / evdev for compositor-grade or direct-device work
- do not force ordinary application UIs to depend on these lower layers

Recommended crates:

- `evdev-input`
- `libinput-bridge`

### 13. Windows Registered I/O (RIO)

RIO is not a general-purpose event loop API. It is a socket-specialized,
performance-oriented extension to Winsock.

What makes it interesting:

- request queues and completion queues specialized for networking
- registered buffers
- explicit send and receive queue sizing
- completion via either event notification or IOCP notification

What makes it risky as a first step:

- socket-only surface
- buffer registration complexity
- less educational as the initial cross-platform baseline

Recommendation:

- do not start with RIO
- keep it in the survey because it may become our Windows "extreme throughput"
  networking backend later
- model it as a separate crate, not as part of `iocp`

Recommended crate:

- `rio`

### 14. Windows I/O rings

Windows now has I/O rings, which are conceptually closer to Linux `io_uring`
than to classic IOCP.

What makes them interesting:

- explicit submission and completion queues
- modern ring-based design
- completion event integration

Why they are not the first answer for us:

- minimum supported client is Windows build 22000
- still a young surface compared with IOCP
- we need a stable Windows backend that works on broader targets first

Recommendation:

- research and keep on the roadmap
- do not make it the primary Windows backend

Recommended crate:

- `win-ioring`

### 15. Windows readiness-style fallbacks

Windows also exposes things like:

- `WSAPoll`
- `WSAEventSelect`
- `WaitForMultipleObjects`

These are real APIs, but they are not the foundation we want for a scalable
server stack. They are fallback tools, not primary backend targets.

---

## Recommended Crate Stack

The stack should be layered.

### Layer 1: Native backend crates

One crate per native facility or tightly related family:

- `epoll`
- `eventfd`
- `timerfd`
- `signalfd`
- `kqueue`
- `iocp`
- `win32-message-loop`
- `cf-run-loop`
- `wayland-client-loop`
- optional later: `x11-event-loop`, `evdev-input`, `libinput-bridge`, `io-uring`,
  `event-port`, `rio`, `win-ioring`

These crates should:

- expose native handles and native errors
- preserve native semantics honestly
- avoid premature cross-platform trait unification
- be small, explicit, and greppable in the same spirit as the bridge crates

### Layer 2: Repository-owned backend translation layer

A new crate should sit above the raw backend crates and translate native events
into repository-owned models suitable for both networking and interactive UI.

Suggested crate:

- `native-event-core`

Responsibilities:

- registration tokens
- source identity
- readiness or completion normalization
- UI event normalization where the native facility naturally supplies it
- wakeup support
- timer support
- backend capability reporting

This layer should *not* erase every difference. It should translate platform
mechanisms into a common set of repository-relevant outcomes while preserving
important distinctions such as:

- readiness versus completion
- UI-dispatch events versus socket events
- compositor-delivered input versus direct-device input

Just as importantly, `native-event-core` should stop at the event-substrate
boundary. It should not know about:

- TCP connection state machines
- HTTP or WebSocket framing
- widget hierarchies
- paint or layout policies above raw redraw scheduling

### Layer 3: Domain runtimes and protocol stacks

Suggested crates:

- `stream-reactor`
- `tcp-reactor`
- `websocket-runtime`
- `ui-event-core`

Responsibilities:

For `stream-reactor`:

- generic readable and writable stream handling
- registration of stream-like sources without committing to one protocol
- buffer progression, backpressure, and shutdown semantics
- a neutral base for protocols built on byte streams

For `tcp-reactor`:

- listener accept loop
- connection state machines
- read buffers
- write queues
- half-close and shutdown handling
- backpressure
- timer-driven connection housekeeping

For `websocket-runtime`:

- handshake progression above HTTP or an existing upgraded stream
- frame parsing and serialization
- ping / pong and close semantics
- text and binary message delivery
- dependence on the generic event substrate and stream layer rather than on a
  bespoke WebSocket-specific loop

For `ui-event-core`:

- keyboard, mouse, touch, focus, resize, and close event vocabulary
- redraw or invalidate requests
- surface or window identity
- integration points for frame or present scheduling
- coexistence with timers, wakeups, and optional networking

These are the layers the language bridges should target.

### Layer 4: Native extensions for Ruby, Python, and Perl

For dynamic languages, the native extension should usually wrap a domain runtime
such as `stream-reactor`, `tcp-reactor`, `websocket-runtime`, or `ui-event-core`,
not the raw backend crates.

That keeps the FFI boundary stable and small.

Good FFI exports look like:

- opaque server handle creation and destruction
- opaque stream handle creation and destruction
- bind / listen / connect
- tick or run-until APIs
- callback registration
- event draining into language objects
- WebSocket session creation on top of an existing stream or listener
- message send / receive APIs built above the generic loop rather than inside it
- opaque window or application loop handles
- callback registration for keyboard, mouse, touch, resize, and redraw events

Bad FFI exports look like:

- exposing `epoll_event`, `kevent`, or `OVERLAPPED` directly to Ruby or Python
- exposing raw Win32 `MSG` or Objective-C objects unless the package is explicitly
  a platform-binding crate
- requiring dynamic-language code to reason about re-arming, kernel queue flags,
  or buffer registration details

---

## What We Should Build First

### Phase 1: Honest primary backends

Build these first:

1. `epoll`
2. `eventfd`
3. `timerfd`
4. `signalfd`
5. `kqueue`
6. `iocp`
7. `win32-message-loop`
8. `cf-run-loop`
9. `wayland-client-loop`

Why this set:

- it covers Linux, BSD/macOS, and Windows with their native first-choice
  mechanisms for both I/O and UI dispatch
- it gives Linux the missing companion fds needed for a real event loop
- it acknowledges that native UI input does not enter through the same doors as
  network readiness or overlapped I/O
- it postpones the highest-complexity backends until the repository-owned
  abstraction is tested by real application and server code

### Phase 2: Cross-platform translation and domain runtimes

Build:

- `native-event-core`
- `stream-reactor`
- `tcp-reactor`
- `websocket-runtime`
- `ui-event-core`

Use this phase to prove:

- generic stream progression
- listener registration
- readable and writable stream progression
- protocol layering above streams
- keyboard, mouse, touch, focus, resize, and close delivery
- timer integration
- wakeup integration
- clean shutdown
- language-agnostic event representation

### Phase 3: Performance-specialized backends

Only after the above is solid, add:

- Linux `io-uring`
- Windows `rio`
- Windows `win-ioring`
- illumos `event-port` if that platform matters

---

## Design Rules

### Rule 1: Native crates must stay native

Do not give the `iocp` crate fake `register_readable()` methods that merely mimic
Unix naming. It should expose completion-port semantics directly.

### Rule 2: The cross-platform layer owns the translation

The translation from:

- readiness events on Unix
- completion packets on Windows
- window messages on Windows
- run-loop or framework callbacks on Apple platforms
- compositor or display-protocol events on Unix desktops

into repository-owned event vocabularies should happen in `native-event-core`,
not in the raw crates.

Protocol logic should then happen one layer higher again, in crates such as
`tcp-reactor` or `websocket-runtime`, not in `native-event-core`.

### Rule 3: Dynamic-language bridges bind above the translation layer

Ruby, Python, and Perl should talk to a stable Rust server runtime, not to raw
OS backends.

### Rule 4: Timers and wakeups are first-class, not afterthoughts

Many event-loop designs get sockets right and then bolt timers or UI lifecycle on
later. We should design timers, wakeups, shutdown signals, and UI lifecycle
events into the core from day one.

### Rule 5: Keep the public surface small

Each raw backend crate should feel like a direct, readable wrapper over the
native facility, much like our existing bridge crates wrap runtime C APIs.

---

## Recommended Initial Package List

If we start implementation work, the first package specs to add should be:

- `epoll`
- `eventfd`
- `timerfd`
- `signalfd`
- `kqueue`
- `iocp`
- `win32-message-loop`
- `cf-run-loop`
- `wayland-client-loop`
- `native-event-core`
- `stream-reactor`
- `ui-event-core`
- `tcp-reactor`
- `websocket-runtime`

That is enough to start building serious TCP servers and native interactive event
loops without prematurely betting the architecture on `io_uring`, RIO, or a
single UI stack.

---

## Bottom Line

If the goal is "real event loops that can eventually power high-performance
TCP/IP servers and native interactive UIs", the repository should treat the
native backends like this:

- `epoll` is the primary Linux readiness backend.
- `eventfd`, `timerfd`, and `signalfd` are part of the real Linux event-loop story.
- `kqueue` is the primary BSD/macOS backend and covers more event kinds in one API.
- IOCP is the primary Windows backend and must be modeled as completion-based, not
  as fake readiness.
- Win32 message queues are the primary Windows UI event source.
- Apple run loops and framework dispatch are the primary Apple UI event source.
- Wayland and X11 are where desktop Linux client UI events enter the process.
- direct input stacks such as evdev and `libinput` matter for compositor-grade or
  lower-level input work.
- `io_uring`, RIO, and Windows I/O rings are important phase-two performance
  paths, not phase-one foundations.

That sequencing gives us the clearest educational story, the most stable
cross-platform design, and the best path toward native extensions for Ruby,
Python, and Perl.

---

## Primary Sources

- Linux `epoll(7)`: <https://man7.org/linux/man-pages/man7/epoll.7.html>
- Linux `poll(2)`: <https://man7.org/linux/man-pages/man2/poll.2.html>
- Linux `select(2)`: <https://man7.org/linux/man-pages/man2/select.2.html>
- Linux `eventfd(2)`: <https://man7.org/linux/man-pages/man2/eventfd.2.html>
- Linux `timerfd_create(2)`: <https://man7.org/linux/man-pages/man2/timerfd_create.2.html>
- Linux `signalfd(2)`: <https://man7.org/linux/man-pages/man2/signalfd.2.html>
- Linux `io_uring(7)`: <https://man7.org/linux/man-pages/man7/io_uring.7.html>
- Linux `io_uring_prep_poll_add(3)`: <https://man7.org/linux/man-pages/man3/io_uring_prep_poll_add.3.html>
- Apple `kqueue(2)` man page archive: <https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/kqueue.2.html>
- FreeBSD `kevent(2)` current manual: <https://man.freebsd.org/cgi/man.cgi?apropos=0&arch=default&format=html&manpath=FreeBSD+16.0-CURRENT&query=kevent&sektion=0>
- Windows IOCP overview: <https://learn.microsoft.com/en-us/windows/win32/fileio/i-o-completion-ports>
- Windows `CreateIoCompletionPort`: <https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-createiocompletionport>
- Windows `GetQueuedCompletionStatus`: <https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-getqueuedcompletionstatus>
- Windows `PostQueuedCompletionStatus`: <https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-postqueuedcompletionstatus>
- Windows message loop overview: <https://learn.microsoft.com/en-us/windows/win32/learnwin32/window-messages>
- Windows Winsock overlapped I/O: <https://learn.microsoft.com/en-us/windows/win32/winsock/overlapped-i-o-2>
- Windows RIO completion queue: <https://learn.microsoft.com/en-us/windows/win32/api/mswsock/nc-mswsock-lpfn_riocreatecompletionqueue>
- Windows RIO request queue: <https://learn.microsoft.com/en-us/windows/win32/api/mswsock/nc-mswsock-lpfn_riocreaterequestqueue>
- Windows I/O rings header overview: <https://learn.microsoft.com/en-us/windows/win32/api/ioringapi/>
- Windows `CreateIoRing`: <https://learn.microsoft.com/en-us/windows/win32/api/ioringapi/nf-ioringapi-createioring>
- Windows `SubmitIoRing`: <https://learn.microsoft.com/en-us/windows/win32/api/ioringapi/nf-ioringapi-submitioring>
- Apple `CFRunLoopRun()`: <https://developer.apple.com/documentation/corefoundation/cfrunlooprun%28%29>
- Apple `RunLoop`: <https://developer.apple.com/documentation/Foundation/RunLoop>
- Wayland client API appendix: <https://wayland.freedesktop.org/docs/html/apb.html>
- X11 `XNextEvent(3X11)`: <https://www.x.org/archive/X11R6.8.0/doc/XNextEvent.3.html>
- libinput overview: <https://wayland.freedesktop.org/libinput/doc/latest/index.html>
- illumos `port_create(3C)`: <https://www.smartos.org/man/3c/port_create>
- illumos `port_associate(3C)`: <https://www.smartos.org/man/3c/port_associate>
- illumos `port_get(3C)`: <https://www.smartos.org/man/3c/port_get>
