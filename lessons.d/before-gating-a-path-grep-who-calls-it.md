---
category: Security boundaries
---

# Before gating a path, grep who calls it

Three times in one arc I put an authorization check on code the shipping path never executes: `HostProfile::from_manifest` (7 call sites, all in-crate tests), `v1_agent_tool_catalog` (zero callers outside its own test), and `HostProfileRuntime` (the daemon imports no host-runtime type at all — its model surface is `SmartHomeToolBridge::invoke` over a bare `InMemoryToolRuntime`). Each took ten seconds to disprove with `grep -rn <symbol> --include='*.rs'`. A boundary on a dead path is worse than none, because the commit message says the hole is closed.
