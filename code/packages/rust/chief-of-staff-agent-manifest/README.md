# chief-of-staff-agent-manifest

Pure, fail-closed codec for the D18 `manifest.json` contract. It is the shared
boundary between Level 1 manifest generation, signed agent packages, discovery,
and host registration.

`parse_manifest` accepts installed schema-v1 and schema-v2 packages and current
schema-v3 packages. V2 maps every declared read/write channel name to a positive
payload-schema version. Versions are scoped by channel name, so a
writer and reader are compatible only when both declare the same version for
that channel. `require_channel_compatibility` provides that fail-closed wiring
check; legacy manifests remain discoverable but cannot be treated as
schema-compatible without an explicit upgrade.

V3 adds `allowed_tools`: the D18D tool identifiers this agent may call. Before
v3 the signed manifest declared operating-system `capabilities` and
`vault_access` but named **no tools at all**, so a profile-backed supervisor had
no signed source for its tool surface — the one thing `HostProfile` needs that
the manifest could not supply.

The field is **required** at v3, so an agent that calls no tools declares `[]`
rather than omitting it. Identifiers are stored sorted and deduplicated, and
must be namespaced (`artifact.write`, not `artifact`): a bare namespace names no
tool and would invite prefix matching, which is how one declared tool becomes a
whole namespace. Earlier schema versions may not carry the field at all — a v1
or v2 manifest declaring `allowed_tools` is rejected, so a consumer that trusts
`version` is never told something false about what the signed bytes authorize.

```json
{
  "version": 3,
  "agent": "weather-agent",
  "description": "Produces a concise local weather forecast.",
  "privilege_tier": 0,
  "channels": {
    "reads": {"weather-requests": 1},
    "writes": {"weather-reports": 2}
  },
  "capabilities": [],
  "allowed_tools": ["artifact.write", "context.append_entry"],
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
