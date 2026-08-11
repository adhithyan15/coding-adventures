# smart-home-discovery-service

Actor-owned lifecycle and durable journals for centrally scheduled smart-home
mDNS discovery.

The service receives the shared `smart-home-controller-runtime` owner and
composes its runtime scheduler with an injectable mDNS executor, a
transport-specific report adapter, and the shared `StorageBackend` contract.
All schedule registration and due-run mutation goes through one
revision-guarded controller transaction. The controller persists:

- selected-interface worker schedules and their retry/backoff state

The service backend separately persists:

- compact reports for every supervised discovery tick
- actor tick health and the latest runtime error

Reopening the controller restores worker cadence before the actor accepts
another tick. Existing service-owned schedule records are imported once when
the central record does not already contain that worker; central state wins on
conflict. Network I/O and transport-specific discovery record conversion
remain behind the existing injectable traits.

## Development

```bash
bash BUILD
```
