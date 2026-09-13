---
category: Security boundaries
---

# Naming a function after the trust you want does not create it

`OrchestratorProfile::from_manifests` took `&AgentManifest` values from the caller and its doc claimed the surface came "from inside the integrity boundary". `AgentManifest`'s fields are all `pub` and `parse_manifest` accepts any text, so an operator-authored manifest passed there had exactly the trust level of the `from_json` path it was written to replace — under a name asserting the opposite. Make the trusted thing the only input: read the manifest from `package.manifest_bytes()` after verifying the package, and take no manifest argument at all. This repo already had two crates doing that (`agent-discovery`, `skill-package`); look for an existing verified-source pattern before inventing a parameter.
