# HOS00 — Historical operating system simulator

## Goal

Build original, inspectable models of historical operating systems. Each
milestone is a small runnable scenario whose trace explains an architectural
change. Start with documented First Edition Unix behavior in a TypeScript
browser program. Keep the existing S-series and Verified OS schedules separate.
Shared component contracts and fixtures can later support ports and, after a
separate evidence review, candidate Rust components for VOS.

## Historical scope

Unix existed on the PDP-7 before the first published Programmer's Manual.
That manual describes the PDP-11/20 system in November 1971. The first
milestone models a file read through a directory entry, inode, descriptor,
offset, and end-of-file. It uses an original in-memory image, a modern teaching
shell, and a semantic kernel model. It does not execute PDP-11 instructions or
recreate a disk format, historical shell, syscall ABI, or exact utility output.
The shell maps `ls` to a teaching directory-list request, not a historical
`ls` syscall. A modeled `read` returning zero bytes marks EOF.

## Components and contracts

- The read-only file system resolves paths from root. Empty paths and paths
  containing NUL are invalid. Repeated slash, dot, and dot-dot are supported.
  Traversal through a regular file, including a trailing slash, fails. Directory
  listings preserve image entry order. Files expose byte ranges, including
  short reads and EOF.
- A descriptor table owns slots numbered from zero. Open allocates the lowest
  free slot and starts its offset at zero. Read advances that slot by the bytes
  returned. Close frees just that slot. Concurrent opens of the same inode have
  independent offsets. Failed operations leave the table unchanged.
- The kernel sequences filesystem and descriptor operations. Its entry points
  for the demo are open, read, close and list. The shell translates command
  text into operations. The browser only renders results and supplies inputs.
- The portable fixture format is JSON under `fixtures/historical-unix-v1/`. A case supplies a shell command and
  expected ordered syscall events and console output. No host time, files, or
  randomness influence the simulated transitions.

## First demo

Run or step through `cat /hello`, `ls /`, or any supported command against a
small built-in image. The initial image includes a nested directory and an
empty file. A browser visitor can see a descriptor's allocation, byte offset,
EOF, and close. A reset returns to a fresh image and descriptor table.

## Language parity

Each milestone targets every implementation lane in the repository inventory:
C, C++, C#, Dart, Elixir, F#, Go, Haskell, Java, Kotlin, Lua, OCaml, Perl,
Python, Ruby, Rust, Swift, TypeScript, and WebAssembly. Mosaic, Twig and
Starlark are tracked separately because they serve UI, language-toolchain
and build-configuration roles in this repository. Document the runner and
its limitations for each lane. Shared fixtures are inputs and observations.
Each lane implements the component transitions in its own language.

Place reusable filesystem, descriptor and kernel interfaces in each
language's package tree. Callers can reuse a component without a shell, or
assemble the kernel to make a simulator. Browser and command-line adapters
depend on the library. A lane is complete for a milestone after its own
implementation replays the shared cases on its supported runtime.

## Hosted ports

The TypeScript browser and the Python and Ruby command-line ports use the
same image and JSON syscall schema. The hosted ports can emit one JSON object
containing `output` and `events` for a command. Shell words are separated
by ASCII whitespace. Replay the same fixtures in each implementation, and
keep command parsing, kernel dispatch, and component transitions separate.

## Evidence

Check unit cases for lookup and descriptor transitions, replay the portable
fixtures, enumerate bounded descriptor histories, and compare longer seeded
histories with an independent model. Fuzz shell and kernel integration with
deterministic bytes and assert termination, reproducibility, and descriptor
cleanup. Use the repository SAT solver for a bounded counterexample check of
lowest-free allocation. Include a deliberately faulty selection as a control.
These checks cover the named domains. They establish neither full Unix
compatibility nor a machine-code proof.

## Evolution

1. Extend the Unix model with process IDs, fork, exec and wait. Show what the
   child inherits, what exec replaces and what wait observes.
2. Trace descriptor inheritance and redirection before introducing pipes as a
   later Unix milestone. Add a machine-backed runner when the guest CPU and
   device model can support it.
3. Build a GM-NAA-style batch monitor and a time-sharing milestone. Compare
   how the monitor sequences jobs with how a scheduler switches tasks.
4. Add DOS-style console and file operations to explore application, OS and BIOS
   boundaries. Use each working milestone as the reference for Python, Ruby,
   Rust and other language ports.

## Sources

- [Dennis Ritchie, First Edition Unix Programmer's Manual](https://www.nokia.com/bell-labs/about/dennis-m-ritchie/1stEdman.html)
- [First Edition source and manual archive](https://www.tuhs.org/cgi-bin/utree.pl?file=V1)
- [Robert Patrick, GM-NAA I/O](https://softwarepreservation.computerhistory.org/os/gm.html)
- [VOS component catalogue](VOS04-component-catalogue.md)
