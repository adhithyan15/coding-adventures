# First-Party Parity

## Overview

**No agent we ship gets special treatment.** Every first-party
agent — the weather agent, future Gmail agents, smart-home hosts,
coding agents, anything we author and distribute — uses the
identical APIs, manifests, registry workflow, signature
verification, hash pinning, capability cage, sandbox enforcement,
and host protocol that a third-party agent uses.

This is the eat-your-own-dogfood principle, made enforceable. If
the weather agent needs a side door — an internal-only host API,
an implicit capability the manifest doesn't declare, a special
case in the orchestrator that bypasses some check — that side
door is a bug in the public API, not a feature of the platform.
We close the bug rather than walk through it.

The principle is the strongest forcing function we have for good
public-API design. If we cannot build the agents we want using
only the APIs we publish, then those APIs are missing something a
third-party developer will also be missing. Better to discover
the gap by trying to build through it than to ship a platform
nobody can extend.

This spec is short, but it is the principle that constrains every
other spec in the system. When in doubt about a design decision —
"should we add a fast-path for first-party agents here?" — the
answer is no.

---

## Where It Fits

```
   Every other spec
        │
        │  written subject to this constraint
        ▼
   first-party-parity (this spec)
        │
        ├── orchestrator           — no first-party fast path
        ├── agent-registry         — first-party agents register the same way
        ├── capability-cage-rust   — first-party manifests are checked the same
        ├── host-protocol          — no internal-only methods
        ├── secure-host-channel    — first-party hosts handshake the same
        ├── host-runtime-rust      — same SDK for everyone
        ├── read-write-separation  — first-party agents are RWS-checked the same
        ├── tls-platform           — same trust roots
        └── ...
```

**Depends on:**
- Every spec in the system. This is a meta-constraint.

**Used by:**
- Every contributor. Every reviewer. Every code change.

---

## The Rule

For every API surface, mechanism, or check in the system, the
following must be true:

> **A third-party developer could replicate any first-party
> agent's behavior using only documented public APIs and the
> same manifest format, signature workflow, registry workflow,
> CLI commands, and SDK that the first-party agent uses.**

Equivalently: if you remove the first-party agents from the
codebase and ask a third-party developer to rewrite them from
scratch using only the public surface, they should succeed.
Anything they cannot do without source-level access to the
substrate is a violation of this rule.

---

## What This Rules Out

The rule, stated as the rule, is short. The list of things it
rules out is what makes it actionable. None of the following are
permitted in any first-party agent:

### 1. Well-known agent names with implicit capabilities

There is no list inside the orchestrator like "if the agent name
is `weather-fetcher`, grant it `net:connect:api.weather.gov:443`
implicitly." Every agent — first-party or third — declares all
its capabilities in its `required_capabilities.json`, period.
The cage performs no name-based exceptions.

### 2. Implicit vault namespaces

A first-party agent does not get to read `vault://orchestrator/...`
without declaring it in its manifest. If an agent needs vault
data, the manifest declares the exact path. The vault's policy
gates access by the manifest, not by the agent's identity.

### 3. Internal-only host.* methods

Every method published in the host runtime's dispatcher must be
in the public `host.*` namespace and documented in
`host-protocol.md`. There is no `host._internal_special_thing` a
first-party agent can call. If a method exists, a third party
can call it (subject to its own manifest's capabilities).

### 4. First-party signature exceptions

The agent registry checks the hash of every package the same way.
A first-party signed package goes through the same Tier 3
challenge to register, the same hash pin on launch, the same
signature verification. The orchestrator does not have a
hard-coded "first-party fingerprint allowlist" that bypasses
registration.

### 5. First-party SDK extensions

The host runtime's SDK (`host::network`, `host::fs`,
`host::vault`, `host::channel`, `host::system`) is the same set
of modules with the same signatures whether you are writing a
first-party agent in this repository or a third-party agent in
your own. There is no `host::internal::*` for our own use.

### 6. First-party capability flavors

The RWS classification (`flavor: actuation | ingestion | internal`,
`trust: trusted | untrusted`) defaults the same way for every
manifest. A first-party `net:connect` is treated as
`actuation`+`untrusted` by default exactly like a third-party
`net:connect`. We override with explicit annotations and
justifications, and reviewers see those overrides in PRs the
same way external contributors would.

### 7. First-party orchestrator policies

There is no per-package configuration option that says "this is a
first-party package, relax the panic-broadcast threshold for it."
Panic thresholds are global; first-party agents must behave well
or face the same quarantine.

### 8. First-party CLI commands

Every CLI command (`orchestrator agent register`,
`orchestrator agent revoke`, `orchestrator launch`, etc.) takes
the same arguments and produces the same outcomes regardless of
whether the package is first-party or third-party. There is no
`orchestrator launch --first-party` shortcut.

### 9. First-party in-process linking

A first-party Tier 1 agent is statically linked to the host
runtime, but a third-party agent that builds against the same
public crate can do the same. The linking model is documented
public API; it is not a private build trick.

### 10. First-party transports

The HTTPS, SMTP, OAuth, vault, and channel transports are the
same for everyone. There is no internal "fast path" channel a
first-party agent can use that a third-party cannot configure.

---

## What This Permits

The rule does not require us to be helpless. The following are
permitted:

- **Bundled agents.** First-party agents can ship in this
  repository under `code/programs/rust/<agent>/`. Their source is
  open and reviewable.
- **Reference implementations.** First-party agents serve as
  worked examples for third-party developers. The
  `weather-agent` PoC is exactly this.
- **Faster developer signing keys.** The dev-key path that
  permits Tier 0-1 agents without Tier 3 challenges is available
  to any developer (first-party or third). The path is the same;
  who uses it differs.
- **Shipping with default configurations.** A first-party agent
  ships with a sensible default `agent.toml` or sample
  `required_capabilities.json`. The user can use it as-is, edit
  it, or replace it. The substrate enforces what the manifest
  says, not what we shipped.
- **CI-signed production builds.** A CI environment with a
  hardware-backed signing key produces production-tier signed
  packages. Any organization (first-party or third) that operates
  such a CI gets the same outcome.

The principle is **uniformity of enforcement**, not uniformity of
existence. We can ship agents; we can sign them; we cannot
exempt them from any check the substrate applies.

---

## How We Enforce It in Code Review

Every PR that touches the substrate is reviewed against this
question:

> **"If a third-party developer wanted to do exactly what this
> change does for a first-party agent, could they do it using
> only the public APIs?"**

Concretely:

- A new `host.*` method must be in `host-protocol.md` before any
  agent (first-party or third) can call it.
- A new vault namespace must be declared in some spec before any
  agent can read or write it.
- A new orchestrator policy must apply uniformly to every agent.
- A new SDK module must be in the published `host_runtime_rust`
  crate; not in some internal sibling crate that only first-party
  agents can depend on.
- A new manifest field must be in the
  `required_capabilities.json` schema; not in some "extended"
  schema only first-party agents use.

When a reviewer answers "no, the third party couldn't do this
without source access," the change is rejected (not merged
behind a flag, not merged with a TODO — rejected) and either:

- The change is reworked to expose the needed API publicly, or
- The first-party agent's design changes so it doesn't need the
  shortcut.

---

## Worked Example: The Weather Agent

Walking through every aspect of the v1 PoC weather agent
(`weather-agent.md`) and confirming first-party parity:

| Aspect                                  | First-party parity check |
|-----------------------------------------|--------------------------|
| Agent name `weather-fetcher`            | No special meaning to the orchestrator. Just a string in `required_capabilities.json`. |
| Manifest `net:connect:api.weather.gov:443` | Declared in JSON. Cage checks it. Same as any third-party agent. |
| Manifest annotation `flavor: ingestion` | Public capability flavor; any agent can use it. |
| Channel `weather-snapshots`             | Declared in `orchestrator.toml`. Same TOML schema any user writes. |
| Schema-pinned `weather-recommendations` | Public schema mechanism; any pipeline can use `trust_laundering: true`. |
| Signed by dev-key                       | Dev-key path is documented; any developer signs the same way. |
| Registered via `agent register`         | Same CLI command third-party uses. Tier 3 challenge fires the same way. |
| Hash-pinned in vault registry           | Same registry path, same record format. |
| Spawned via `spawn_host_process`         | Same code path the orchestrator uses for any host. |
| Channel bootstrap (X3DH per-spawn)      | Identical to every other host's bootstrap. |
| Panic-broadcast subscriber              | Subscribed automatically because every host is. |
| `host.network.fetch` call               | Public SDK method. |
| `host.channel.write` call                | Public SDK method. |
| Audit log entries                       | Same record format as every other agent. |
| `os-job-runtime` schedule                | Public job spec. Any agent can register one. |

The weather agent contains **zero shortcut, zero special-case,
zero internal-only API call**. A third-party developer who wanted
to write their own "fetch some forecast → write some file" agent
in the same shape would write the exact same kind of code, with
the exact same kind of manifest, registered the exact same way.

---

## What Happens When the Rule Is Violated

A violation is a substrate bug. The fix is to expose the missing
public API and update the first-party agent to use it.

Example workflow:

1. Author writes a first-party agent and discovers they can't
   accomplish a goal with the published `host.*` methods.
2. Tempting fix: add a special method only the first-party agent
   uses.
3. **Rejected at review.** This violates first-party parity.
4. Correct fix: open a PR that adds the missing method to
   `host-protocol.md` (with capability gating), implement it in
   `host-runtime-rust`, document it in the SDK, and **then** the
   first-party agent uses it just like a third-party agent
   would.

The friction is the point. Each violation discovered and fixed
this way improves the public API by exactly the amount that the
first-party agent needed.

---

## Test Strategy

This spec is a constraint, not a runtime check. Its enforcement
is via:

1. **Code review.** Every substrate-touching PR answers the
   first-party-parity question.

2. **A periodic audit.** Once per release cycle, a sweep of the
   first-party agents looks for any of the 10 forbidden patterns
   above. Any found becomes a tracked issue with priority equal
   to a substrate bug.

3. **The "rebuild from public APIs" thought experiment.** A
   reviewer occasionally walks through one first-party agent's
   source and asks: "could I write this from scratch using only
   public APIs?" Any "no" is a finding.

4. **A future first-party-parity-checker tool** that statically
   analyzes first-party agent source for use of internal crates,
   undocumented manifest fields, or unpublished SDK methods.
   The tool would report any cross-boundary usage as a violation.
   v1 of the substrate uses the manual review process; the tool
   is future work.

---

## Trade-Offs

**Friction for the substrate authors.** When we want a feature
for a first-party agent and the public API doesn't have it, we
must do the slower, harder work of designing and exposing it
publicly. We accept this; the result is a better platform for
everyone.

**Public-API design must be ahead of agent-feature need.** If
the agent we want to ship needs a feature we haven't designed,
we cannot ship the agent until we have designed the feature
publicly. This will sometimes delay first-party agents. We
accept the cost.

**No internal optimizations through private APIs.** A
first-party agent cannot, e.g., use a fast in-process channel
that bypasses the secure-host-channel ratchet. If we discover
performance limits, we improve the public channel for everyone,
not the private alternative for ourselves.

**Audits require discipline.** Code review is the primary
enforcement, and reviewers must remember the rule consistently.
The future automated tool will help; until it ships, this is a
process discipline.

**Some "bundled defaults" look like special-casing.** A
first-party agent shipped with a default config in this
repository will look — to a casual reader — like a special path.
The line is clear: defaults are public; they can be overridden;
no orchestrator code branches on whether the agent came with us.

---

## Future Extensions

- **Static checker.** A `cargo` lint that scans first-party
  agent source for any use of an undocumented API surface.
- **API parity test suite.** A set of automated tests that
  exercise every public API and confirm the first-party agents
  use only those.
- **A "third-party rewrite" exercise.** Once a year, rewrite a
  first-party agent in a separate repo using only public APIs to
  verify nothing has crept in.

These are deliberately out of scope for v1.
