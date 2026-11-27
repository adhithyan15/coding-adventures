# OS Architecture Simulator Probe (TypeScript)

This HOS03 probe provides two independent layers: a deterministic semantic
engine for guest policy, and a small synthetic CPU/bus/board probe. The core
owns logical ticks, queued JSON stimuli, bounded execution, ordered effects,
snapshots, and replay. A batch monitor and a synthetic protected message
scheduler run on that engine. The separate bytecode stepper exercises explicit
fetch, data access, device, and memory-map boundaries on two board configs.

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
and board fixtures are replayed by build-only verifiers beside those fixtures;
package unit tests remain self-contained and the library needs no host
capabilities. No command-line interface or host-output adapter is shipped. The
machine probe uses only its declared toy instruction set; it is not an ISA
implementation, cycle-accurate model, or claim of real hardware compatibility.
The bus uses bounded non-overlapping regions and explicit permissions, while
devices keep their effects in simulated state. See
[HOS03](../../../specs/HOS03-modular-os-simulation.md) and
[HOS03a](../../../specs/HOS03a-machine-layer.md). All profiles and board maps
are synthetic examples, not claims of historical or production fidelity.
