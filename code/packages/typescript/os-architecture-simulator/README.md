# OS Architecture Simulator Probe (TypeScript)

This small HOS03 probe separates a deterministic event engine from guest
policy. The core owns logical ticks, queued JSON stimuli, bounded execution,
ordered effects, snapshots, and replay. A batch monitor and a synthetic modern
protected message scheduler run on the same engine. Their job and task rules
live in profiles, never in the core.

Profiles and callers exchange finite JSON payloads, including nested records,
arrays, numeric values, booleans, and null. The core copies and sorts records,
rejects accessors and unsupported JavaScript objects, and limits each JSON value
to 64 nested levels, 100,000 nodes, and one million code units per string or
key, and ten million code units across all strings and keys in one value.
`Machine.step()` advances exactly one transition and returns that
transition's effects; it has the same behavior as `Machine.run(1)`, including
the synthesized `clock.tick` when there is no queued input.
The reducer itself is trusted TypeScript code; the engine validates its JSON
boundary but is not a sandbox for profiles or JavaScript Proxy traps.

Run `bash BUILD` from this directory with Node.js 24 or newer. The shared profile
fixture is replayed by a build-only driver beside the fixture; package unit tests
remain self-contained and the library needs no host capabilities. No command-line
interface or host-output adapter is shipped. The engine is a semantic simulator.
It does not execute guest instructions or emulate an ISA, CPU, bus, device, board,
or wall clock; those boundaries are described in
[HOS03](../../../specs/HOS03-modular-os-simulation.md). The guest policies are
synthetic examples, not claims of historical or production fidelity.
