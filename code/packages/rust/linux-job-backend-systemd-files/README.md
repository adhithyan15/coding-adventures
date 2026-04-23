# linux-job-backend-systemd-files

Pure Rust rendering of `systemd --user` service and timer units.

## What it does

- Renders `.service` units for job actions
- Renders `.timer` units for recurring or calendar-based schedules
- Produces an `InstallPlan` that targets `~/.config/systemd/user`

## Supported triggers

- `once`
- `interval` without anchor
- `daily`
- `weekly`
- `monthly`
- `at_login`

## Unsupported in this first pass

- `at_boot` for the user manager
- interval anchors
- stdin payloads

The backend returns structured `JobError` values for these cases.

## Dependencies

- os-job-core

## Development

```bash
bash BUILD
```
