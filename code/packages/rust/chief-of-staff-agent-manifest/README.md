# chief-of-staff-agent-manifest

Pure, fail-closed codec for the D18 `manifest.json` contract. It is the shared
boundary between Level 1 manifest generation, signed agent packages, discovery,
and host registration.

`parse_manifest` accepts installed legacy schema-v1 packages and current
schema-v2 packages. V2 maps every declared read/write channel name to a positive
payload-schema version. Versions are scoped by channel name, so a
writer and reader are compatible only when both declare the same version for
that channel. `require_channel_compatibility` provides that fail-closed wiring
check; legacy manifests remain discoverable but cannot be treated as
schema-compatible without an explicit upgrade.

```json
{
  "version": 2,
  "agent": "weather-agent",
  "description": "Produces a concise local weather forecast.",
  "privilege_tier": 0,
  "channels": {
    "reads": {"weather-requests": 1},
    "writes": {"weather-reports": 2}
  },
  "capabilities": [],
  "justification": "Uses only the declared encrypted weather channels."
}
```

The codec rejects malformed JSON, duplicate or unknown fields, invalid nested
structures, unsupported capability pairs, incomplete schema-version maps, and
agents that read and write the same channel. Parsed manifests render back to
deterministic schema-shaped JSON with `AgentManifest::to_json`.

This package does not scan directories, verify package signatures, register
agents, or perform any operating-system access. Those effects belong to the
discovery and host layers that consume this contract.

## Validation

```sh
sh chief-of-staff-agent-manifest/BUILD
```
