# chief-of-staff-agent-manifest

Pure, fail-closed codec for the D18 `manifest.json` contract. It is the shared
boundary between Level 1 manifest generation, signed agent packages, discovery,
and host registration.

`parse_manifest` accepts schema version 1 only. It rejects malformed JSON,
duplicate or unknown fields, invalid nested structures, unsupported capability
pairs, and agents that read and write the same channel. The parsed type can be
rendered back to deterministic schema-shaped JSON with `AgentManifest::to_json`.

This package does not scan directories, verify package signatures, register
agents, or perform any operating-system access. Those effects belong to the
discovery and host layers that consume this contract.

## Validation

```sh
sh chief-of-staff-agent-manifest/BUILD
```
