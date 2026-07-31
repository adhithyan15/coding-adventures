# Changelog

## 0.1.0

- Add authenticated Home Assistant WebSocket export collection.
- Collect area, device, and entity registries plus current state.
- Add deterministic normalization, synthetic entities for unregistered state,
  atomic output, and an environment-token CLI.
- Add protocol and process-level tests backed by a real local WebSocket server.
