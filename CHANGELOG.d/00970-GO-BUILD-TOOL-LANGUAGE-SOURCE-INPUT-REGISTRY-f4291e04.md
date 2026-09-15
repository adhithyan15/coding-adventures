### Go build-tool language source-input registry

- Replaced the Go hasher's partial source and metadata maps with a generated,
  strictly decoded projection of the complete checked 23-language registry and
  all seven selector roles. The runtime no longer depends on neutral fixtures.
- Added fail-closed unknown-language precedence, exact canonical package-root
  binding, scoped/root selector preservation, universal capability metadata,
  exact generated-component pruning, canonical digest checks, four neutral
  production-path fixtures, and a deterministic generator round trip.

