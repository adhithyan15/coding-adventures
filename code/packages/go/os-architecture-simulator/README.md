# OS Architecture Simulator (Go)

This dependency-free HOS03 library provides a deterministic event machine for
OS policy profiles. The machine owns JSON-shaped stimuli, virtual ticks,
bounded `Step`/`Run`, ordered effects, snapshots, and history replay. Its
synthetic batch monitor and protected message scheduler reproduce the two
shared `os-architecture-v1` fixture traces.

The scope and the exact parity claim are recorded in
[HOS03d](../../../specs/HOS03d-go-semantic-simulator.md). This package does
not run a guest ISA or model hardware, host I/O, or Unix syscalls.

Run `BUILD` from this directory to execute the pure package tests, vet, and
the fixture-adjacent conformance verifier. The verifier imports this library,
reads the shared fixture outside the package tree, and checks both profile
traces plus snapshot/restore/replay continuation. A stdout demo is deferred to
a separate program so this reusable package has no host I/O capability.
