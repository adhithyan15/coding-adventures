# Chief of Staff SDK

The TypeScript SDK is the capability-safe interface between an agent and its
Chief of Staff host. Agent code never needs filesystem, network, process,
clock, or random-number permissions. It sends typed requests to a host over an
injected transport, and the host enforces the sealed manifest.

The SDK supports Level 2 one-file handlers and Level 3 direct encrypted-channel
and Vault access. A Level 2 agent contains only its handler:

```typescript
import { defineAgent } from "@coding-adventures/chief-of-staff-sdk";

defineAgent(async (message) => {
  const { city } = JSON.parse(message.plaintext);
  return `The weather in ${city} is sunny.`;
});
```

The trusted wrapper binds that definition to a `SimpleAgentRuntime`. Each
`runOnce()` performs receive, strict UTF-8 decoding, handler execution,
publication, and acknowledgement in that order. Handler and publication
failures leave the input unacknowledged for host-driven retry.

Level 3 agents can operate channels directly:

```typescript
import {
  JsonRpcLineTransport,
  configureHostTransport,
  channel_read,
  channel_write,
  channel_ack,
} from "@coding-adventures/chief-of-staff-sdk";

configureHostTransport(new JsonRpcLineTransport(lines));

const message = await channel_read("finance-requests");
if (message !== null) {
  await channel_write(
    "finance-summaries",
    new TextEncoder().encode("complete"),
    "text/plain",
  );
  await channel_ack("finance-requests", message.id);
}
```

`lines` implements `JsonLineDuplex`. A Deno or subprocess wrapper owns the
actual stdin/stdout handles and injects them; the SDK itself has no ambient OS
access. Requests are serialized so one ordered stdio stream never has multiple
unmatched responses in flight.

Channel payloads use canonical base64 on the wire and `Uint8Array` in agent
code. Sequence numbers and nanosecond timestamps use decimal strings on the
wire and `bigint` in agent code, avoiding JavaScript's 53-bit integer limit.

Level 3 agents can request host-mediated Vault operations without receiving
secret bytes or cryptographic keys:

```typescript
import {
  vault_release_lease,
  vault_request_direct,
  vault_request_lease,
} from "@coding-adventures/chief-of-staff-sdk";

const lease = await vault_request_lease("bank-creds", 10_000);
await approvedHostOperation({ vault_ref: lease.vault_ref });
await vault_release_lease(lease.vault_ref);

await vault_request_direct("browser-session", "browser-agent");
```

The returned `vault_ref` is an opaque bearer capability. It remains directly
accessible for approved host calls but is non-enumerable, immutable, and
replaced with `<redacted>` by JSON serialization. Agent code must not log,
persist, derive meaning from, or send the reference outside approved host
operations.

## Security properties

- JSON-RPC version, response ID, result/error exclusivity, and result shapes
  are validated before data reaches agent code.
- Malformed base64, non-canonical decimal integers, oversized identifiers,
  content types, payloads, and protocol lines are rejected.
- Host errors retain their numeric code and structured data without exposing
  unchecked response objects as SDK values.
- Vault TTLs, receipts, and null acknowledgements are validated before use;
  opaque lease references are redacted from enumerable and JSON diagnostics.
- Level 2 input must be strict UTF-8, handler output must be non-empty text,
  and publication must succeed before acknowledgement.
