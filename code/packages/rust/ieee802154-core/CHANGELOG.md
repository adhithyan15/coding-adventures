# Changelog

## Unreleased

- Added body-free MAC frame summaries for Zigbee/Thread diagnostics, exposing
  frame shape, addressing/security flags, sequence/FCS presence, and payload
  length without copying payload bytes or address values.

## 0.1.0

- Initial MAC frame-control, addressing, payload, and optional FCS parsing.
- Added auxiliary security header parsing and encoding.
- Added security level, frame counter, and key identifier models.
- Added beacon payload parsing and scan-facing PAN descriptors for Zigbee and
  Thread discovery work.
- Added PAN scan summaries with channel filters and association-candidate
  ranking by link quality.
