# Changelog

## 0.1.0

- Add bounded versioned host-binding and immutable channel-claim records.
- Require exact durable registration and current one-way channel membership
  before wiring and again before every launch resolution.
- Add create-if-absent wiring, revision-CAS replacement and unwiring, strict
  corruption rejection, cross-pipeline isolation, and restart coverage.
