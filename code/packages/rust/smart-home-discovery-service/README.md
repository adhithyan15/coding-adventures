# smart-home-discovery-service

Actor-owned lifecycle and durable journals for scheduled smart-home mDNS
discovery.

The service composes the existing `smart-home-runtime` scheduler with an
injectable mDNS executor, a transport-specific report adapter, and the shared
`StorageBackend` contract. It persists:

- selected-interface worker schedules and their retry/backoff state
- compact reports for every supervised discovery tick
- actor tick health and the latest runtime error

Reopening the service against the same backend restores worker cadence before
the actor accepts another tick. Network I/O and transport-specific discovery
record conversion remain behind the existing injectable traits.

## Development

```bash
bash BUILD
```
