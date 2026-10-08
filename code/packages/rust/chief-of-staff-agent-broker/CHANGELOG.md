# Changelog

## Unreleased

### Added

- The agent broker (D18S P2.6d-1):
  - `LoadedKeys`: slot-table validation against the binding, then key
    loading with the old authority's rules;
  - `ChannelBroker`: Receive, Publish and Acknowledge, served with the
    agent's keys through `ChannelCallbacks`;
  - `FramedCallbacks`: those callbacks over stdin and stdout.
- The `chief-of-staff-agent-broker` binary:
  - core dumps suppressed first;
  - keys read from inherited descriptors, with a tripwire on the
    descriptor count;
  - `Ready` reports public halves;
  - requests served one at a time;
  - it exits non-zero, silently, on anything an honest supervisor never
    sends.
- Before encrypting, the broker checks every reserved header, and refuses
  to encrypt twice under one sequence (nonce reuse) within its lifetime.
- Delivery receipts are per broker, capped at 4096.
- Security review round 1:
  - a refused commit is followed by `AbandonAppend`;
  - the descriptor tripwire scans up to the open-file limit, not just the
    next descriptor;
  - the README says plainly that the daemon, which supplies definitions,
    is not defended against.
