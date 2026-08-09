# Changelog

## Unreleased

- Expose complete revalidated launch authority, including the pipeline's agent
  identity, for per-request host data-plane authorization and service dispatch.

## 0.1.0

- Add bounded versioned host-binding and immutable channel-claim records.
- Require exact durable registration and current one-way channel membership
  before wiring and again before every launch resolution.
- Add create-if-absent wiring, revision-CAS replacement and unwiring, strict
  corruption rejection, cross-pipeline isolation, and restart coverage.
