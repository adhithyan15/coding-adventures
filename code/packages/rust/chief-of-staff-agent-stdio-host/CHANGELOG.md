# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- `StdioAgentSession::spawn` isolates the agent's descriptors (D18S S-I2,
  S-I3; #13980 P2.2). stderr is `/dev/null`, where it used to be inherited.
  On Unix, nothing above fd 2 is inherited, the agent gets its own session,
  and a terminal on a standard descriptor refuses the spawn.

## [0.1.0] - 2026-08-03

- Add a shell-free, long-lived subprocess session for the Level 4 JSON-lines protocol.
- Add receive, response, publish, and acknowledge orchestration over injected channel endpoints.
- Kill and reap the owned child after protocol, framing, or process-I/O failure.
