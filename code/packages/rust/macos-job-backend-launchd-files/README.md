# macos-job-backend-launchd-files

Pure Rust rendering of per-user `launchd` LaunchAgent plists.

## What it does

- Converts a portable `JobSpec` into a deterministic plist file
- Produces an `InstallPlan` that writes into `~/Library/LaunchAgents`
- Emits `launchctl` commands for loading or unloading the agent

## Supported triggers

- `interval`
- `daily`
- `weekly`
- `monthly`
- `at_login`

## Unsupported in this first pass

- `once`
- `at_boot`
- interval anchors
- stdin payloads

These are returned as explicit `JobError` values rather than being silently
approximated.

## Dependencies

- os-job-core

## Development

```bash
bash BUILD
```
