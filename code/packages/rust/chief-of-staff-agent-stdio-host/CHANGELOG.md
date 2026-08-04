# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-08-03

- Add a shell-free, long-lived subprocess session for the Level 4 JSON-lines protocol.
- Add receive, response, publish, and acknowledge orchestration over injected channel endpoints.
- Kill and reap the owned child after protocol, framing, or process-I/O failure.
