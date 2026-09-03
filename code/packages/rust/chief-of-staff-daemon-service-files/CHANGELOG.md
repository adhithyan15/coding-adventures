# Changelog

## Unreleased

- Raise the descriptor soft limit to 65536 in both the systemd unit
  (`LimitNOFILE`) and the launchd plist (`SoftResourceLimits/NumberOfFiles`).
  The supervisor holds three descriptors per agent, so the default soft
  `RLIMIT_NOFILE` of 1024 is exhausted at roughly 340 agents. D18S's capacity
  analysis puts this ahead of every limit except memory, and its failure mode
  is why it is worth raising up front: the spawn fails with a descriptor error
  naming neither the agent nor the cause, and looks like a bug in whatever was
  being launched.

## 0.1.0 - 2026-08-03

- Add deterministic launchd LaunchAgent, systemd user-service, and Windows Task
  Scheduler definitions for the Chief daemon.
- Validate normalized absolute executable and configuration paths before
  rendering platform files.
- Encode login startup, least-privilege user scope, cooperative Unix shutdown,
  single-instance execution, and crash restart policy without shell wrappers.
