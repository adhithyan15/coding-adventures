# Changelog

## 0.1.0

- Add a bounded, versioned JSON request/response protocol for Chief host lifecycle operations.
- Require connection-local authentication and per-operation authorization.
- Bind the protocol to `chief-of-staff-orchestrator-core` and the repository WebSocket runtime.
- Preserve separate durable and authoritative health evidence with precision-safe JSON encoding.
- Add a typed blocking WebSocket client with strict response-ID and envelope validation.
